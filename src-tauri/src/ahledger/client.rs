use super::parse::{parse_pricetable, PriceTable};
pub use crate::blizzard::client::FetchError;
use crate::blizzard::client::{http_client, retry_after};
use reqwest::{header, StatusCode};

pub const AHLEDGER_BASE: &str = "https://api.ahledger.com";

pub enum TableFetch {
    NotModified,
    Fresh { etag: Option<String>, table: PriceTable },
}

pub struct AhledgerClient {
    http: reqwest::Client,
    base: String,
    key: Option<String>,
}

impl AhledgerClient {
    pub fn new(base: String, key: Option<String>) -> Self {
        AhledgerClient { http: http_client(), base, key }
    }

    pub async fn fetch_pricetable(&self, market: &str, if_none_match: Option<&str>) -> Result<TableFetch, FetchError> {
        let mut req = self.http.get(format!("{}/v1/pricetable/{}", self.base, market));
        if let Some(k) = &self.key {
            req = req.bearer_auth(k);
        }
        if let Some(etag) = if_none_match {
            req = req.header(header::IF_NONE_MATCH, etag);
        }
        let resp = req.send().await?;
        match resp.status() {
            StatusCode::NOT_MODIFIED => return Ok(TableFetch::NotModified),
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => return Err(FetchError::RateLimited(retry_after(&resp))),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(FetchError::Unauthorized),
            s if !s.is_success() => return Err(FetchError::Status(s.as_u16())),
            _ => {}
        }
        let etag = resp.headers().get(header::ETAG).and_then(|v| v.to_str().ok()).map(str::to_string);
        let text = resp.text().await?;
        let table = parse_pricetable(&text).map_err(|e| FetchError::Parse(e.to_string()))?;
        Ok(TableFetch::Fresh { etag, table })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    const FIX: &str = include_str!("../../tests/fixtures/pricetable_pvp_alliance.txt");

    #[tokio::test]
    async fn ahledger_sends_key_and_etag() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path("/v1/pricetable/forever.pvp.alliance.us").header("authorization", "Bearer ahlk_x").header("if-none-match", "\"abc\"");
                then.status(304);
            })
            .await;
        let c = AhledgerClient::new(server.base_url(), Some("ahlk_x".into()));
        assert!(matches!(c.fetch_pricetable("forever.pvp.alliance.us", Some("\"abc\"")).await.unwrap(), TableFetch::NotModified));
        assert_eq!(m.hits_async().await, 1);
    }

    #[tokio::test]
    async fn ahledger_fresh_returns_etag_and_table() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/v1/pricetable/forever.pvp.alliance.us");
                then.status(200).header("ETag", "\"v2\"").body(FIX);
            })
            .await;
        let c = AhledgerClient::new(server.base_url(), None);
        match c.fetch_pricetable("forever.pvp.alliance.us", None).await.unwrap() {
            TableFetch::Fresh { etag, table } => {
                assert_eq!(etag.as_deref(), Some("\"v2\""));
                assert_eq!(table.rows.len(), 300);
            }
            TableFetch::NotModified => panic!("expected Fresh"),
        }
    }

    #[tokio::test]
    async fn ahledger_without_key_sends_no_authorization() {
        let server = MockServer::start_async().await;
        let with_auth = server
            .mock_async(|when, then| {
                when.method(GET).header_exists("authorization");
                then.status(500);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET).path("/v1/pricetable/forever.pvp.horde.us");
                then.status(200).body(FIX);
            })
            .await;
        let c = AhledgerClient::new(server.base_url(), None);
        assert!(c.fetch_pricetable("forever.pvp.horde.us", None).await.is_ok());
        assert_eq!(with_auth.hits_async().await, 0);
    }

    #[tokio::test]
    async fn ahledger_503_is_rate_limited() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET);
                then.status(503).header("Retry-After", "12");
            })
            .await;
        let c = AhledgerClient::new(server.base_url(), None);
        assert!(matches!(c.fetch_pricetable("forever.pvp.horde.us", None).await, Err(FetchError::RateLimited(12))));
    }
}
