use crate::ahledger::client::{AhledgerClient, TableFetch, AHLEDGER_BASE};
use crate::blizzard::client::{BlizzardClient, BlizzardConfig, FeedFetch, FetchError};
use crate::blizzard::parse::{parse_commodities_feed, parse_items_feed};
use crate::blizzard::static_data::{fetch_items, missing_item_ids, store_items};
use crate::credentials::{load_credentials, Credentials};
use crate::db::Db;
use crate::domain::{Feed, House};
use crate::processing::ingest::{ingest_feed, ingest_pricetable, IngestOutcome};
use crate::processing::retention::prune;
use crate::settings::load_settings;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

pub const POLL_SECS: i64 = 300;
pub const AHLEDGER_MIN_SECS: i64 = 900;
const METADATA_BATCH: usize = 2000;
const WAKE_JUMP_SECS: i64 = 600;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorStatus {
    /// "running" | "paused" | "error" | "no_credentials" | "starting"
    pub state: String,
    pub last_cycle_at: Option<i64>,
    pub last_new_snapshot_at: Option<i64>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEvent {
    pub house: House,
    pub feed: Feed,
    pub snapshot_id: i64,
}

pub enum CollectorEvent {
    Status(CollectorStatus),
    Snapshot(SnapshotEvent),
}

#[derive(Clone, Debug)]
pub struct Endpoints {
    pub blizzard_api: String,
    pub blizzard_token: String,
    pub ahledger: String,
}

impl Endpoints {
    /// Production URLs; debug builds let `AUCTIONEER_AHLEDGER_BASE` override the AHledger base.
    pub fn production() -> Self {
        let ahledger = if cfg!(debug_assertions) {
            std::env::var("AUCTIONEER_AHLEDGER_BASE").unwrap_or_else(|_| AHLEDGER_BASE.to_string())
        } else {
            AHLEDGER_BASE.to_string()
        };
        Endpoints {
            blizzard_api: "https://us.api.blizzard.com".into(),
            blizzard_token: "https://oauth.battle.net/token".into(),
            ahledger,
        }
    }
}

#[derive(Debug, Default)]
pub struct CycleReport {
    pub new_snapshots: usize,
    pub errors: Vec<String>,
}

/// 300 s normally, doubling per consecutive failed cycle, capped at 1800 s.
pub fn next_delay_secs(consecutive_errors: u32) -> i64 {
    (POLL_SECS << consecutive_errors.min(6)).min(1800)
}

pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

enum CredSource {
    Keychain,
    Fixed(Option<Credentials>),
}

struct Clients {
    creds: Credentials,
    blizzard: Arc<BlizzardClient>,
    ahledger: Arc<AhledgerClient>,
}

#[derive(Default)]
struct RunState {
    clients: Option<Clients>,
    last_ahledger_at: i64,
    last_prune_at: i64,
    consecutive_errors: u32,
    retry_at: i64,
}

pub struct Collector {
    db: Db,
    endpoints: Endpoints,
    creds: CredSource,
    paused: AtomicBool,
    status: StdMutex<CollectorStatus>,
    emit: Box<dyn Fn(CollectorEvent) + Send + Sync>,
    state: Mutex<RunState>,
    wake: Notify,
}

type Emit = Box<dyn Fn(CollectorEvent) + Send + Sync>;

impl Collector {
    pub fn new(db: Db, endpoints: Endpoints, emit: Emit) -> Arc<Self> {
        Self::build(db, endpoints, emit, CredSource::Keychain)
    }

    /// Test constructor: fixed credentials instead of the Keychain / .env.
    pub fn with_credentials(db: Db, endpoints: Endpoints, emit: Emit, creds: Option<Credentials>) -> Arc<Self> {
        Self::build(db, endpoints, emit, CredSource::Fixed(creds))
    }

    fn build(db: Db, endpoints: Endpoints, emit: Emit, creds: CredSource) -> Arc<Self> {
        let paused = load_settings(&db.lock().expect("db")).collection_paused;
        Arc::new(Collector {
            db,
            endpoints,
            creds,
            paused: AtomicBool::new(paused),
            status: StdMutex::new(CollectorStatus { state: "starting".into(), last_cycle_at: None, last_new_snapshot_at: None, message: None }),
            emit,
            state: Mutex::new(RunState::default()),
            wake: Notify::new(),
        })
    }

    pub fn status(&self) -> CollectorStatus {
        self.status.lock().expect("status").clone()
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
        if !paused {
            self.wake.notify_one();
        }
    }

    /// Wakes the loop to run a cycle now (Settings saved, "refresh" pressed).
    pub fn trigger_now(&self) {
        self.wake.notify_one();
    }

    fn set_status(&self, state: &str, message: Option<String>, now: i64, new_snapshot: bool) {
        let s = {
            let mut st = self.status.lock().expect("status");
            st.state = state.into();
            st.message = message;
            st.last_cycle_at = Some(now);
            if new_snapshot {
                st.last_new_snapshot_at = Some(now);
            }
            st.clone()
        };
        (self.emit)(CollectorEvent::Status(s));
    }

    fn load_creds(&self) -> Option<Credentials> {
        match &self.creds {
            CredSource::Keychain => load_credentials(),
            CredSource::Fixed(c) => c.clone(),
        }
    }

    fn clients<'a>(&self, st: &'a mut RunState, creds: Credentials) -> &'a Clients {
        if st.clients.as_ref().is_none_or(|c| c.creds != creds) {
            let blizzard = Arc::new(BlizzardClient::new(BlizzardConfig {
                api_base: self.endpoints.blizzard_api.clone(),
                token_url: self.endpoints.blizzard_token.clone(),
                client_id: creds.blizzard_client_id.clone(),
                client_secret: creds.blizzard_client_secret.clone(),
            }));
            let ahledger = Arc::new(AhledgerClient::new(self.endpoints.ahledger.clone(), creds.ahledger_key.clone()));
            st.clients = Some(Clients { creds, blizzard, ahledger });
        }
        st.clients.as_ref().expect("clients")
    }

    fn stored(&self, source: &str, house: House, feed: &str) -> (Option<i64>, Option<String>) {
        let conn = self.db.lock().expect("db");
        conn.query_row(
            "SELECT last_modified, etag FROM source_status WHERE source = ?1 AND house = ?2 AND feed = ?3",
            params![source, house.as_str(), feed],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()
        .flatten()
        .unwrap_or((None, None))
    }

    fn record_ok(&self, source: &str, house: House, feed: &str, now: i64, last_modified: Option<i64>, etag: Option<&str>) {
        let conn = self.db.lock().expect("db");
        let _ = conn.execute(
            "INSERT INTO source_status(source, house, feed, last_success, last_modified, etag) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(source, house, feed) DO UPDATE SET last_success = excluded.last_success,
             last_modified = COALESCE(excluded.last_modified, last_modified), etag = COALESCE(excluded.etag, etag)",
            params![source, house.as_str(), feed, now, last_modified, etag],
        );
    }

    fn record_err(&self, source: &str, house: House, feed: &str, now: i64, err: &str) {
        let conn = self.db.lock().expect("db");
        let _ = conn.execute(
            "INSERT INTO source_status(source, house, feed, last_error_at, last_error) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(source, house, feed) DO UPDATE SET last_error_at = excluded.last_error_at, last_error = excluded.last_error",
            params![source, house.as_str(), feed, now, err],
        );
    }

    async fn ingest(&self, house: House, feed: Feed, last_modified: i64, now: i64, body: bytes::Bytes, cadence: i64) -> Result<IngestOutcome, String> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let listings = match feed {
                Feed::Items => parse_items_feed(&body),
                Feed::Commodities => parse_commodities_feed(&body),
            }
            .map_err(|e| e.to_string())?;
            let mut conn = db.lock().map_err(|e| e.to_string())?;
            ingest_feed(&mut conn, house, feed, last_modified, now, &listings, cadence).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// One collection pass: Blizzard feeds, then AHledger, then item metadata and pruning.
    pub async fn run_cycle(&self, now: i64) -> CycleReport {
        let mut report = CycleReport::default();
        if self.paused.load(Ordering::SeqCst) {
            self.set_status("paused", None, now, false);
            return report;
        }
        let Some(creds) = self.load_creds() else {
            self.set_status("no_credentials", Some("Add Blizzard API credentials in Settings".into()), now, false);
            return report;
        };
        let cadence = load_settings(&self.db.lock().expect("db")).cadence_secs;
        let mut st = self.state.lock().await;
        let (blizzard, ahledger) = {
            let c = self.clients(&mut st, creds);
            (c.blizzard.clone(), c.ahledger.clone())
        };

        let mut fatal: Option<String> = None;
        let mut blizzard_ok = 0usize;
        'feeds: for house in House::ALL {
            for feed in Feed::ALL {
                let (stored_lm, _) = self.stored("blizzard", house, feed.as_str());
                match blizzard.fetch_feed(house, feed, stored_lm).await {
                    Ok(FeedFetch::NotModified) => {
                        blizzard_ok += 1;
                        self.record_ok("blizzard", house, feed.as_str(), now, None, None)
                    }
                    Ok(FeedFetch::Fresh { last_modified, .. }) if Some(last_modified) == stored_lm => {
                        blizzard_ok += 1;
                        self.record_ok("blizzard", house, feed.as_str(), now, None, None)
                    }
                    Ok(FeedFetch::Fresh { last_modified, body }) => match self.ingest(house, feed, last_modified, now, body, cadence).await {
                        Ok(outcome) => {
                            blizzard_ok += 1;
                            self.record_ok("blizzard", house, feed.as_str(), now, Some(last_modified), None);
                            if let IngestOutcome::Ingested { snapshot_id, .. } = outcome {
                                report.new_snapshots += 1;
                                (self.emit)(CollectorEvent::Snapshot(SnapshotEvent { house, feed, snapshot_id }));
                            }
                        }
                        Err(e) => {
                            let msg = format!("ingest failed: {e}");
                            self.record_err("blizzard", house, feed.as_str(), now, &msg);
                            report.errors.push(msg);
                        }
                    },
                    Err(FetchError::Unauthorized) => {
                        let msg = "Blizzard credentials rejected".to_string();
                        self.record_err("blizzard", house, feed.as_str(), now, &msg);
                        report.errors.push(msg.clone());
                        fatal = Some(msg);
                        break 'feeds;
                    }
                    Err(FetchError::RateLimited(secs)) => {
                        let msg = format!("Blizzard rate limited, retrying in {secs}s");
                        self.record_err("blizzard", house, feed.as_str(), now, &msg);
                        report.errors.push(msg.clone());
                        st.retry_at = now + secs as i64;
                        fatal = Some(msg);
                        break 'feeds;
                    }
                    Err(e) => {
                        let msg = format!("Blizzard fetch failed: {e}");
                        self.record_err("blizzard", house, feed.as_str(), now, &msg);
                        report.errors.push(msg);
                    }
                }
            }
        }

        if fatal.is_none() && blizzard_ok == 0 {
            let last = report.errors.last().cloned().unwrap_or_default();
            fatal = Some(format!("All Blizzard feeds failing ({last})"));
        }

        if report.new_snapshots > 0 || now - st.last_ahledger_at >= AHLEDGER_MIN_SECS {
            st.last_ahledger_at = now;
            for house in House::ALL {
                let Some(market) = house.ahledger_market() else { continue };
                let (_, etag) = self.stored("ahledger", house, "pricetable");
                match ahledger.fetch_pricetable(market, etag.as_deref()).await {
                    Ok(TableFetch::NotModified) => self.record_ok("ahledger", house, "pricetable", now, None, None),
                    Ok(TableFetch::Fresh { etag, table }) => {
                        let as_of = table.as_of;
                        let db = self.db.clone();
                        let res = tokio::task::spawn_blocking(move || ingest_pricetable(&mut db.lock().expect("db"), &table)).await;
                        match res {
                            Ok(Ok(_)) => self.record_ok("ahledger", house, "pricetable", now, Some(as_of), etag.as_deref()),
                            Ok(Err(e)) => self.record_err("ahledger", house, "pricetable", now, &format!("store failed: {e}")),
                            Err(e) => self.record_err("ahledger", house, "pricetable", now, &format!("store failed: {e}")),
                        }
                    }
                    Err(e) => {
                        let msg = format!("AHledger fetch failed: {e}");
                        self.record_err("ahledger", house, "pricetable", now, &msg);
                        report.errors.push(msg);
                    }
                }
            }
        }

        if fatal.is_none() {
            let ids = missing_item_ids(&self.db.lock().expect("db"), METADATA_BATCH).unwrap_or_default();
            if !ids.is_empty() {
                let metas = fetch_items(&blizzard, &ids).await;
                let _ = store_items(&mut self.db.lock().expect("db"), &metas, now);
            }
        }

        if now - st.last_prune_at >= 86400 {
            st.last_prune_at = now;
            let db = self.db.clone();
            let _ = tokio::task::spawn_blocking(move || prune(&mut db.lock().expect("db"), now)).await;
        }

        match &fatal {
            Some(msg) => {
                st.consecutive_errors += 1;
                self.set_status("error", Some(msg.clone()), now, report.new_snapshots > 0);
            }
            None => {
                st.consecutive_errors = 0;
                self.set_status("running", None, now, report.new_snapshots > 0);
            }
        }
        report
    }

    /// Runs a cycle at start, every POLL_SECS (with backoff after failures), right after a wall-clock
    /// jump (wake from sleep), and whenever `trigger_now` is called.
    pub fn spawn_loop(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            let mut next_run = 0i64;
            let mut last_tick = now_unix();
            loop {
                let now = now_unix();
                if now - last_tick > WAKE_JUMP_SECS {
                    next_run = now;
                }
                last_tick = now;
                if now >= next_run {
                    self.run_cycle(now).await;
                    let st = self.state.lock().await;
                    next_run = (now + next_delay_secs(st.consecutive_errors)).max(st.retry_at);
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(30)) => {}
                    _ = self.wake.notified() => { next_run = 0; }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use httpmock::prelude::*;
    use std::sync::Mutex as StdMutex;

    const LM: &str = "Sat, 14 Nov 2026 17:17:47 GMT";
    const NOW: i64 = 1794677000;
    const ITEMS: &str = include_str!("../tests/fixtures/items_110.json");
    const COMM: &str = include_str!("../tests/fixtures/commodities_110.json");
    const TABLE: &str = include_str!("../tests/fixtures/pricetable_pvp_alliance.txt");

    struct Harness {
        server: MockServer,
        collector: Arc<Collector>,
        events: Arc<StdMutex<Vec<String>>>,
        db: Db,
    }

    fn creds() -> Credentials {
        Credentials { blizzard_client_id: "id".into(), blizzard_client_secret: "secret".into(), ahledger_key: None }
    }

    async fn harness(token_ok: bool) -> Harness {
        let server = MockServer::start_async().await;
        if token_ok {
            server.mock_async(|w, t| { w.method(POST).path("/token"); t.status(200).json_body(serde_json::json!({"access_token":"t","expires_in":86399})); }).await;
        } else {
            server.mock_async(|w, t| { w.method(POST).path("/token"); t.status(401); }).await;
        }
        server.mock_async(|w, t| { w.method(GET).path_prefix("/data/wow/item/"); t.status(404); }).await;
        server.mock_async(|w, t| { w.method(GET).path_prefix("/data/wow/media/"); t.status(404); }).await;
        let db: Db = Arc::new(StdMutex::new(open_in_memory()));
        let events = Arc::new(StdMutex::new(Vec::new()));
        let ev = events.clone();
        let endpoints = Endpoints { blizzard_api: server.base_url(), blizzard_token: server.url("/token"), ahledger: server.url("/ahl") };
        let collector = Collector::with_credentials(
            db.clone(),
            endpoints,
            Box::new(move |e| {
                ev.lock().unwrap().push(match e {
                    CollectorEvent::Status(s) => format!("status:{}", s.state),
                    CollectorEvent::Snapshot(s) => format!("snapshot:{}:{}", s.house.as_str(), s.feed.as_str()),
                })
            }),
            Some(creds()),
        );
        Harness { server, collector, events, db }
    }

    async fn mock_feeds_fresh(server: &MockServer) -> (httpmock::Mock<'_>, httpmock::Mock<'_>) {
        let items = server.mock_async(|w, t| { w.method(GET).path_matches(r"^/data/wow/ruleset/1/auctions/\d+$"); t.status(200).header("Last-Modified", LM).body(ITEMS); }).await;
        let comm = server.mock_async(|w, t| { w.method(GET).path_matches(r"^/data/wow/ruleset/1/auctions/\d+/commodities$"); t.status(200).header("Last-Modified", LM).body(COMM); }).await;
        (items, comm)
    }

    async fn mock_ahledger_ok(server: &MockServer) -> httpmock::Mock<'_> {
        server.mock_async(|w, t| { w.method(GET).path_prefix("/ahl/v1/pricetable/"); t.status(200).header("ETag", "\"e1\"").body(TABLE); }).await
    }

    fn q(db: &Db, sql: &str) -> i64 {
        db.lock().unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[tokio::test]
    async fn cycle_ingests_all_six_feeds() {
        let h = harness(true).await;
        mock_feeds_fresh(&h.server).await;
        let ahl = mock_ahledger_ok(&h.server).await;
        let report = h.collector.run_cycle(NOW).await;
        assert_eq!(report.new_snapshots, 6, "{:?}", report.errors);
        assert_eq!(h.events.lock().unwrap().iter().filter(|e| e.starts_with("snapshot:")).count(), 6);
        assert_eq!(q(&h.db, "SELECT COUNT(*) FROM feed_snapshots"), 6);
        assert_eq!(ahl.calls_async().await, 2, "alliance + horde tables, no blackwater");
        assert_eq!(q(&h.db, "SELECT COUNT(DISTINCT market) FROM ahledger_rows"), 1, "fixture table names one market");
        assert_eq!(h.collector.status().state, "running");
    }

    #[tokio::test]
    async fn second_cycle_not_modified_adds_nothing() {
        let h = harness(true).await;
        let (items, comm) = mock_feeds_fresh(&h.server).await;
        mock_ahledger_ok(&h.server).await;
        h.collector.run_cycle(NOW).await;
        items.delete_async().await;
        comm.delete_async().await;
        let nm = h.server.mock_async(|w, t| { w.method(GET).path_prefix("/data/wow/ruleset/").header("if-modified-since", LM); t.status(304); }).await;
        let report = h.collector.run_cycle(NOW + 300).await;
        assert_eq!(report.new_snapshots, 0);
        assert_eq!(nm.calls_async().await, 6);
        assert_eq!(q(&h.db, "SELECT COUNT(*) FROM feed_snapshots"), 6);
    }

    #[tokio::test]
    async fn same_last_modified_with_200_is_not_reingested() {
        let h = harness(true).await;
        mock_feeds_fresh(&h.server).await;
        mock_ahledger_ok(&h.server).await;
        h.collector.run_cycle(NOW).await;
        let report = h.collector.run_cycle(NOW + 300).await;
        assert_eq!(report.new_snapshots, 0);
        assert_eq!(q(&h.db, "SELECT COUNT(*) FROM feed_snapshots"), 6);
    }

    #[tokio::test]
    async fn cycle_with_bad_credentials_records_error() {
        let h = harness(false).await;
        mock_feeds_fresh(&h.server).await;
        mock_ahledger_ok(&h.server).await;
        let report = h.collector.run_cycle(NOW).await;
        assert_eq!(report.new_snapshots, 0);
        assert_eq!(h.collector.status().state, "error");
        assert!(h.collector.status().message.unwrap().contains("credentials"));
        let err: String = h.db.lock().unwrap().query_row("SELECT last_error FROM source_status WHERE source='blizzard' LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(err.contains("credentials"));
    }

    #[tokio::test]
    async fn ahledger_failure_does_not_block_blizzard() {
        let h = harness(true).await;
        mock_feeds_fresh(&h.server).await;
        h.server.mock_async(|w, t| { w.method(GET).path_prefix("/ahl/"); t.status(503).header("Retry-After", "5"); }).await;
        let report = h.collector.run_cycle(NOW).await;
        assert_eq!(report.new_snapshots, 6);
        assert_eq!(q(&h.db, "SELECT COUNT(*) FROM source_status WHERE source='ahledger' AND last_error IS NOT NULL"), 2);
        assert_eq!(h.collector.status().state, "running");
    }

    #[tokio::test]
    async fn all_feeds_failing_is_error_state() {
        let h = harness(true).await;
        h.server.mock_async(|w, t| { w.method(GET).path_prefix("/data/wow/ruleset/"); t.status(500); }).await;
        mock_ahledger_ok(&h.server).await;
        h.collector.run_cycle(NOW).await;
        let s = h.collector.status();
        assert_eq!(s.state, "error");
        assert!(s.message.unwrap().contains("Blizzard"));
    }

    #[tokio::test]
    async fn paused_cycle_does_nothing() {
        let h = harness(true).await;
        let (items, comm) = mock_feeds_fresh(&h.server).await;
        h.collector.set_paused(true);
        let report = h.collector.run_cycle(NOW).await;
        assert_eq!(report.new_snapshots, 0);
        assert_eq!(items.calls_async().await + comm.calls_async().await, 0);
        assert_eq!(h.collector.status().state, "paused");
    }

    #[tokio::test]
    async fn missing_credentials_state() {
        let server = MockServer::start_async().await;
        let db: Db = Arc::new(StdMutex::new(open_in_memory()));
        let endpoints = Endpoints { blizzard_api: server.base_url(), blizzard_token: server.url("/token"), ahledger: server.url("/ahl") };
        let c = Collector::with_credentials(db, endpoints, Box::new(|_| {}), None);
        c.run_cycle(NOW).await;
        assert_eq!(c.status().state, "no_credentials");
    }

    #[test]
    fn backoff_schedule() {
        assert_eq!(next_delay_secs(0), 300);
        assert_eq!(next_delay_secs(1), 600);
        assert_eq!(next_delay_secs(2), 1200);
        assert_eq!(next_delay_secs(3), 1800);
        assert_eq!(next_delay_secs(9), 1800);
    }
}
