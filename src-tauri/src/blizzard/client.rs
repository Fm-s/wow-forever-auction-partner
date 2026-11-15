use crate::domain::{Feed, House, Region};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::{header, StatusCode};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;


#[derive(Clone, Debug)]
pub struct BlizzardConfig {
    /// e.g. `https://{region}.api.blizzard.com`; `{region}` is filled per request.
    pub api_base: String,
    /// e.g. `https://oauth.battle.net/token`
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
}

pub enum FeedFetch {
    NotModified,
    Fresh { last_modified: i64, body: bytes::Bytes },
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited, retry after {0}s")]
    RateLimited(u64),
    #[error("http {0}")]
    Status(u16),
    #[error("missing Last-Modified")]
    NoLastModified,
    #[error("parse: {0}")]
    Parse(String),
    #[error(transparent)]
    Net(#[from] reqwest::Error),
}

pub fn parse_http_date(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc2822(s).ok().map(|d| d.timestamp())
}

pub fn format_http_date(unix: i64) -> String {
    Utc.timestamp_opt(unix, 0)
        .single()
        .map(|d| d.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .unwrap_or_default()
}

/// Reads `Retry-After` seconds (default 60) for 429/503 responses.
pub(crate) fn retry_after(resp: &reqwest::Response) -> u64 {
    resp.headers()
        .get(header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(60)
}

pub(crate) fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Auctioneer/0.1")
        .timeout(Duration::from_secs(120))
        .build()
        .expect("reqwest client")
}

pub struct BlizzardClient {
    http: reqwest::Client,
    config: BlizzardConfig,
    token: Mutex<Option<(String, Instant)>>,
}

impl BlizzardClient {
    pub fn new(config: BlizzardConfig) -> Self {
        BlizzardClient { http: http_client(), config, token: Mutex::new(None) }
    }

    async fn token(&self, force_refresh: bool) -> Result<String, FetchError> {
        let mut guard = self.token.lock().await;
        if !force_refresh
            && let Some((tok, expires)) = guard.as_ref()
                && Instant::now() < *expires {
                    return Ok(tok.clone());
                }
        #[derive(serde::Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: u64,
        }
        let resp = self
            .http
            .post(&self.config.token_url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await?;
        match resp.status() {
            s if s.is_success() => {}
            StatusCode::UNAUTHORIZED | StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN => return Err(FetchError::Unauthorized),
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => return Err(FetchError::RateLimited(retry_after(&resp))),
            s => return Err(FetchError::Status(s.as_u16())),
        }
        let t: TokenResp = resp.json().await?;
        let ttl = Duration::from_secs(t.expires_in.saturating_sub(60));
        *guard = Some((t.access_token.clone(), Instant::now() + ttl));
        Ok(t.access_token)
    }

    /// GET with bearer auth; on 401 refreshes the token and retries once.
    async fn get(&self, url: &str, if_modified_since: Option<i64>) -> Result<reqwest::Response, FetchError> {
        for attempt in 0..2 {
            let tok = self.token(attempt > 0).await?;
            // reqwest adds Accept-Encoding: gzip itself (gzip feature) and only then decompresses.
            let mut req = self.http.get(url).bearer_auth(tok);
            if let Some(ims) = if_modified_since {
                req = req.header(header::IF_MODIFIED_SINCE, format_http_date(ims));
            }
            let resp = req.send().await?;
            match resp.status() {
                StatusCode::UNAUTHORIZED => continue,
                StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => return Err(FetchError::RateLimited(retry_after(&resp))),
                s if s.is_success() || s == StatusCode::NOT_MODIFIED => return Ok(resp),
                s => return Err(FetchError::Status(s.as_u16())),
            }
        }
        Err(FetchError::Unauthorized)
    }

    /// URL on the US API (static item data is the same in every region).
    pub fn static_url(&self, path_and_query: &str) -> String {
        format!("{}{}", self.base(Region::Us), path_and_query)
    }

    /// API base for a region (`{region}` in the configured base is replaced; test bases have none).
    fn base(&self, region: Region) -> String {
        self.config.api_base.replace("{region}", region.as_str())
    }

    /// Requests a fresh token: `Err(Unauthorized)` when Blizzard rejects the credentials.
    pub async fn check_credentials(&self) -> Result<(), FetchError> {
        self.token(true).await.map(|_| ())
    }

    pub fn feed_url(&self, house: House, feed: Feed) -> String {
        let suffix = match feed {
            Feed::Items => "",
            Feed::Commodities => "/commodities",
        };
        let region = house.realm.region;
        format!(
            "{}/data/wow/ruleset/1/auctions/{}{}?namespace=dynamic-forever-{}&locale=en_US",
            self.base(region),
            house.auction_house_id(),
            suffix,
            region.as_str()
        )
    }

    pub async fn fetch_feed(&self, house: House, feed: Feed, if_modified_since: Option<i64>) -> Result<FeedFetch, FetchError> {
        let resp = self.get(&self.feed_url(house, feed), if_modified_since).await?;
        if resp.status() == StatusCode::NOT_MODIFIED {
            return Ok(FeedFetch::NotModified);
        }
        let last_modified = resp
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_http_date)
            .ok_or(FetchError::NoLastModified)?;
        let body = resp.bytes().await?;
        Ok(FeedFetch::Fresh { last_modified, body })
    }

    /// GET `{api_base}{path_and_query}` as JSON (used for static item data).
    /// GET JSON from the US API (used for static item data).
    pub async fn get_json(&self, path_and_query: &str) -> Result<serde_json::Value, FetchError> {
        let url = self.static_url(path_and_query);
        Ok(self.get(&url, None).await?.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    const LM: &str = "Sat, 14 Nov 2026 17:17:47 GMT";
    const LM_UNIX: i64 = 1794676667;

    async fn setup() -> (MockServer, BlizzardClient) {
        let server = MockServer::start_async().await;
        let client = BlizzardClient::new(BlizzardConfig {
            api_base: server.base_url(),
            token_url: server.url("/token"),
            client_id: "id".into(),
            client_secret: "secret".into(),
        });
        (server, client)
    }

    async fn mock_token(server: &MockServer) -> httpmock::Mock<'_> {
        server
            .mock_async(|when, then| {
                when.method(POST).path("/token").header_exists("authorization");
                then.status(200).json_body(serde_json::json!({"access_token":"tok","token_type":"bearer","expires_in":86399}));
            })
            .await
    }

    #[tokio::test]
    async fn fetch_feed_fresh_parses_last_modified() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/110").query_param("namespace", "dynamic-forever-us").header("authorization", "Bearer tok");
                then.status(200).header("Last-Modified", LM).body(r#"{"auctions":[]}"#);
            })
            .await;
        match client.fetch_feed(House::Alliance, Feed::Items, None).await.unwrap() {
            FeedFetch::Fresh { last_modified, body } => {
                assert_eq!(last_modified, LM_UNIX);
                assert_eq!(&body[..], br#"{"auctions":[]}"#);
            }
            FeedFetch::NotModified => panic!("expected Fresh"),
        }
    }

    #[tokio::test]
    async fn fetch_feed_sends_if_modified_since_and_handles_304() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/110").header("if-modified-since", LM);
                then.status(304);
            })
            .await;
        assert!(matches!(client.fetch_feed(House::Alliance, Feed::Items, Some(LM_UNIX)).await.unwrap(), FeedFetch::NotModified));
        assert_eq!(m.calls_async().await, 1);
    }

    #[tokio::test]
    async fn commodities_url_has_suffix() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/113/commodities");
                then.status(200).header("Last-Modified", LM).body("{}");
            })
            .await;
        client.fetch_feed(House::Horde, Feed::Commodities, None).await.unwrap();
        assert_eq!(m.calls_async().await, 1);
    }

    #[tokio::test]
    async fn missing_last_modified_is_error() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/116");
                then.status(200).body("{}");
            })
            .await;
        assert!(matches!(client.fetch_feed(House::Blackwater, Feed::Items, None).await, Err(FetchError::NoLastModified)));
    }

    #[test]
    fn feed_urls_use_the_house_region_and_static_data_us() {
        use crate::domain::{Faction, Realm, RealmType, Region};
        let client = BlizzardClient::new(BlizzardConfig { api_base: "https://{region}.api.blizzard.com".into(), token_url: "t".into(), client_id: "a".into(), client_secret: "b".into() });
        let eu = Realm { region: Region::Eu, kind: RealmType::Rppvp }.house(Faction::Horde);
        assert_eq!(client.feed_url(eu, Feed::Commodities), "https://eu.api.blizzard.com/data/wow/ruleset/1/auctions/124/commodities?namespace=dynamic-forever-eu&locale=en_US");
        assert_eq!(client.static_url("/data/wow/item/1?namespace=static-forever-us"), "https://us.api.blizzard.com/data/wow/item/1?namespace=static-forever-us");
    }

    #[tokio::test]
    async fn check_credentials_reports_rejection() {
        let (server, client) = setup().await;
        server.mock_async(|when, then| { when.method(POST).path("/token"); then.status(401); }).await;
        assert!(matches!(client.check_credentials().await, Err(FetchError::Unauthorized)));
    }

    #[tokio::test]
    async fn check_credentials_accepts_valid_ones() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        assert!(client.check_credentials().await.is_ok());
    }

    #[tokio::test]
    async fn token_is_cached_between_calls() {
        let (server, client) = setup().await;
        let tok = mock_token(&server).await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/110");
                then.status(304);
            })
            .await;
        client.fetch_feed(House::Alliance, Feed::Items, Some(LM_UNIX)).await.unwrap();
        client.fetch_feed(House::Alliance, Feed::Items, Some(LM_UNIX)).await.unwrap();
        assert_eq!(tok.calls_async().await, 1);
    }

    #[tokio::test]
    async fn api_401_refreshes_token_once() {
        let (server, client) = setup().await;
        let tok = mock_token(&server).await;
        let api = server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/110");
                then.status(401);
            })
            .await;
        assert!(matches!(client.fetch_feed(House::Alliance, Feed::Items, None).await, Err(FetchError::Unauthorized)));
        assert_eq!(tok.calls_async().await, 2);
        assert_eq!(api.calls_async().await, 2);
    }

    #[tokio::test]
    async fn token_401_is_unauthorized() {
        let (server, client) = setup().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/token");
                then.status(401);
            })
            .await;
        assert!(matches!(client.fetch_feed(House::Alliance, Feed::Items, None).await, Err(FetchError::Unauthorized)));
    }

    #[tokio::test]
    async fn rate_limited_reads_retry_after() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/ruleset/1/auctions/110");
                then.status(429).header("Retry-After", "30");
            })
            .await;
        assert!(matches!(client.fetch_feed(House::Alliance, Feed::Items, None).await, Err(FetchError::RateLimited(30))));
    }

    #[tokio::test]
    async fn get_json_appends_to_api_base() {
        let (server, client) = setup().await;
        mock_token(&server).await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/data/wow/item/2589").query_param("namespace", "static-forever-us");
                then.status(200).json_body(serde_json::json!({"id":2589}));
            })
            .await;
        let v = client.get_json("/data/wow/item/2589?namespace=static-forever-us&locale=en_US").await.unwrap();
        assert_eq!(v["id"], 2589);
    }

    #[test]
    fn http_date_round_trip() {
        assert_eq!(parse_http_date(LM), Some(LM_UNIX));
        assert_eq!(format_http_date(LM_UNIX), LM);
    }
}
