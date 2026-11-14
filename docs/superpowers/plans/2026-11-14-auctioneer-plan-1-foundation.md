# Auctioneer Plan 1: Foundation (collector, storage, market, health)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Tauri 2 tray app that collects the three WoW Forever PvP US auction houses from Blizzard every snapshot, plus AHledger price tables. It stores price and volume history in SQLite, and shows a Market browser, Item detail charts, a Health/Sources view and Settings.

**Architecture:**
- **Rust side** (`src-tauri/`): an async collector (tokio, reqwest) fetches feeds with conditional requests and parses them into `Listing`s. A blocking ingest step writes snapshots, aggregates, volume diffs and daily rollups to one SQLite file (rusqlite).
- **React + TS side** (`src/`): reads everything through typed Tauri commands, and refreshes on a `collector://snapshot` event.
- **Packaging:** the app lives in the menu bar, starts at login, and keeps collecting with the window closed.

**Tech Stack:**
- Tauri 2 (`tauri = "2"`, `@tauri-apps/cli@2`, `@tauri-apps/api@2`), with the plugins `tauri-plugin-autostart = "2"` and `tauri-plugin-opener = "2"`.
- Rust 1.95: `tokio`, `reqwest` (features `json`, `gzip`), `serde`/`serde_json`, `rusqlite` (feature `bundled`) with `rusqlite_migration` (pick the rusqlite version it depends on), `keyring = "4"`, `thiserror = "2"`, `chrono`, and `httpmock` (dev).
- Frontend: React 19 + TypeScript + Vite, `@tanstack/react-table@8`, `echarts@6` + `echarts-for-react@3`, `vitest`.
- Package manager: pnpm.

**Spec:** `docs/superpowers/specs/2026-11-14-auctioneer-design.md`. Read it before starting; this plan argues from it.

## Global Constraints

- **Scope:** WoW Forever, PvP ruleset, US only. Houses: Alliance = 110, Horde = 113, Blackwater = 116.
- **Blizzard endpoints:**
  - Feeds: `https://us.api.blizzard.com/data/wow/ruleset/1/auctions/{houseId}` (items) and `…/{houseId}/commodities`, with `namespace=dynamic-forever-us&locale=en_US`.
  - Static data: `static-forever-us`.
- **Blizzard auth:** OAuth client credentials at `https://oauth.battle.net/token`.
- **AHledger:**
  - Endpoint: `https://api.ahledger.com/v1/pricetable/{market}`, with markets `forever.pvp.alliance.us` and `forever.pvp.horde.us`. Blackwater has no AHledger market.
  - Send the optional key as `Authorization: Bearer ahlk_…`.
  - Any screen showing AHledger numbers shows the credit link "Prices from AHledger" → `https://ahledger.com`.
- **Money:** integer copper everywhere, displayed as `Xg Ys Zc` (zero units omitted, `0c` for zero, `—` for unknown).
- **Price key:** `(house, item_id, variant_key)`, where `variant_key` = bonus lists sorted ascending and joined with `:`, or `""` when there are none.
- **Polling:**
  - Blizzard: conditional poll every 300 s.
  - AHledger: fetch when a new Blizzard snapshot lands, and otherwise at most every 900 s.
  - Never poll faster than that.
- **Gaps:**
  - A gap is a Δt between consecutive feed snapshots greater than 1.5 × `cadence_secs`. The default `cadence_secs` is 3600; Task 1 measures the real value.
  - Gaps are never interpolated. Charts show them as breaks.
- **Volume:**
  - Trade volume is labelled "est." everywhere in the UI.
  - A vanished listing counts as est. sold only if its `time_left` minimum remaining time is greater than Δt. Minimums: SHORT 0, MEDIUM 1800, LONG 7200, VERY_LONG 43200 s.
- **Retention:** per-snapshot aggregates and volume are kept 30 days, daily rollups forever, and raw listings for the last 2 snapshots per (house, feed) only.
- **Freshness:** a feed is stale when now − its `last_modified` > 2 × `cadence_secs`. A house is partially stale when exactly one of its two feeds is stale.
- **House cuts:** stored as basis points in settings, default Alliance 500, Horde 500, Blackwater 1500, editable.
- **Credentials:**
  - Stored in the macOS Keychain under service `app.auctioneer.desktop`, accounts `blizzard_client_id`, `blizzard_client_secret`, `ahledger_key`.
  - Debug builds fall back to the repo-root `.env` (`BLIZZARD_CLIENT_ID`, `BLIZZARD_CLIENT_SECRET`, optional `AHLEDGER_KEY`). Secrets are never sent to the UI.
- **Commits:** Conventional Commits, with no `Co-Authored-By` or other AI attribution trailer.
- **Bundle identifier:** `app.auctioneer.desktop`. Product name: `Auctioneer`.

## Review Focus

1. **Mac slept for hours.**
   - Expected: the next snapshot is marked gap, its diff is stored as unattributed and excluded from est. sold/day, and the chart shows a break, not a straight line.
   - Pinned in Task 9 (`ingest_marks_gap_after_long_sleep`) and Task 12 (`history_inserts_break_at_gap`).
2. **Blizzard returns 200 with the same `Last-Modified` as the last ingest** (ignores `If-Modified-Since`).
   - Expected: no duplicate snapshot and no zero-volume diff.
   - Pinned in Task 9 (`ingest_same_last_modified_is_duplicate`).
3. **First launch, or an item with less than 7 days of history.**
   - Expected: 7d median, trend % and est. sold/day show `—`, not `0`, `NaN` or a crash.
   - Pinned in Task 12 (`market_row_without_history_has_none_fields`).
4. **Missing or invalid Blizzard credentials** (token 401).
   - Expected: the collector records a clear error in Health, backs off, doesn't panic or spin, and resumes when credentials are saved.
   - Pinned in Task 11 (`cycle_with_bad_credentials_records_error`).
5. **AHledger down or rate-limiting (429/503)** while Blizzard works.
   - Expected: Blizzard ingestion still happens, and AHledger shows an error in Health.
   - Pinned in Task 11 (`ahledger_failure_does_not_block_blizzard`).

## File Structure

```
tools/cadence_probe.py            # Task 1: 24h Last-Modified logger (stdlib only)
tools/cadence_report.py           # Task 1: summarizes the probe CSV
src-tauri/src/lib.rs              # Tauri builder: plugins, state, commands, tray, collector start
src-tauri/src/domain.rs           # House, Feed, TimeLeft, Listing, constants
src-tauri/src/blizzard/parse.rs   # feed JSON -> Vec<Listing>
src-tauri/src/blizzard/client.rs  # token + conditional feed fetch
src-tauri/src/blizzard/static_data.rs # item + media metadata fetch
src-tauri/src/ahledger/parse.rs   # pricetable text -> PriceTable
src-tauri/src/ahledger/client.rs  # conditional pricetable fetch
src-tauri/src/db/mod.rs           # Db handle, open(), migrations
src-tauri/src/db/schema.sql       # migration 1
src-tauri/src/processing/aggregate.rs # per-item stats for one snapshot
src-tauri/src/processing/volume.rs    # snapshot diff -> est sold / expired
src-tauri/src/processing/ingest.rs    # write snapshot, aggregates, volume, raw, rollups
src-tauri/src/processing/retention.rs # 30-day prune
src-tauri/src/settings.rs         # app settings in the settings table
src-tauri/src/credentials.rs      # Keychain + .env fallback
src-tauri/src/collector.rs        # polling loop, status, events
src-tauri/src/queries/market.rs   # MarketRow, cross-house
src-tauri/src/queries/history.rs  # ItemHistory
src-tauri/src/queries/health.rs   # Health
src-tauri/src/commands.rs         # #[tauri::command] wrappers
src-tauri/src/tray.rs             # tray menu, hide-on-close
src-tauri/tests/fixtures/         # trimmed real snapshots
src/api.ts  src/types.ts  src/money.ts  src/App.tsx
src/views/{MarketView,ItemView,HealthView,SettingsView}.tsx
src/components/{HouseSelect,Money,PriceChart,AhledgerCredit}.tsx
```

---

### Task 1: Cadence probe

Measures the real refresh cadence and checks the `time_left` assumption. Start it first: it runs for 24 h in the background while the other tasks proceed.

**Files:**
- Create: `tools/cadence_probe.py`, `tools/cadence_report.py`, `.gitignore`

**Interfaces:**
- Produces: `tools/cadence.csv` (gitignored) with columns `polled_at_utc,house,feed,status,last_modified_utc`, and a report that prints, per feed, the median, p90 and max interval between distinct `Last-Modified` values in seconds.

- [ ] **Step 1: Init repo and ignore file**

```bash
cd /Users/felipesouza/Projetos/Felipe/auctioneer && git init
```
`.gitignore` lines: `.env`, `node_modules/`, `dist/`, `src-tauri/target/`, `tools/cadence.csv`, `.DS_Store`.

- [ ] **Step 2: Write `tools/cadence_probe.py`**

Stdlib only (`urllib`, `json`, `csv`, `time`):
- Read `.env` from the repo root and get a token.
- Every 300 s, for houses 110, 113 and 116 and both feeds, send a `HEAD` request. If `HEAD` isn't allowed, send a `GET` and discard the body.
- Append one CSV row per feed per poll.
- Accept `--hours 24`.
- Refresh the token on 401.

- [ ] **Step 3: Write `tools/cadence_report.py`**

Group by (house, feed), keep only rows where `last_modified` changed, and print the median, p90 and max of the intervals. Also print whether any `If-Modified-Since` request returned 304: the probe sends `If-Modified-Since` on every poll after the first and logs the status.

- [ ] **Step 4: Start the probe in the background and confirm rows appear**

Run: `python3 tools/cadence_probe.py --hours 24 &` and, after 1 minute, `tail -3 tools/cadence.csv`.
Expected: 6 rows per poll with status 200 or 304 and a `last_modified_utc` value.

- [ ] **Step 5: Commit**

```bash
git add .gitignore tools/cadence_probe.py tools/cadence_report.py
git commit -m "chore: add Blizzard snapshot cadence probe"
```

- [ ] **Step 6 (after 24 h, before Task 13): Record the result**

Run `python3 tools/cadence_report.py`, then:
- Set the `cadence_secs` default in Task 7's settings (`DEFAULT_CADENCE_SECS`) to the median interval, rounded to the minute.
- Note in the spec's Constraints section whether 304 is honored.
- Commit: `chore: set measured snapshot cadence`.

---

### Task 2: Scaffold the Tauri 2 + React app

**Files:**
- Create: the project skeleton via `pnpm create tauri-app`, `src-tauri/tauri.conf.json`, and `vitest` config in `vite.config.ts`.

**Interfaces:**
- Produces: `pnpm tauri dev` opens a window titled "Auctioneer". `cargo test` (in `src-tauri`) and `pnpm test` both run.

- [ ] **Step 1: Scaffold**

Run `pnpm create tauri-app@4 . --template react-ts --manager pnpm -y` in a temporary directory and move the result into the repo root; it refuses non-empty dirs. Then:
- Check that `src-tauri/Cargo.toml` has `tauri = { version = "2", … }`, and `package.json` has `@tauri-apps/cli` `^2` and `@tauri-apps/api` `^2`. Downgrade any 3.x alpha to 2.
- Set `productName` to `Auctioneer` and `identifier` to `app.auctioneer.desktop`.
- Add the dev dependency `vitest` and the script `"test": "vitest run"`.

- [ ] **Step 2: Add Rust dependencies**

In `src-tauri/Cargo.toml`, add the crates listed under Tech Stack, with `httpmock` under `[dev-dependencies]`. Create empty module files from the File Structure and declare them in `lib.rs`, so later tasks only fill them in.

- [ ] **Step 3: Verify**

Run: `cd src-tauri && cargo test` → `test result: ok. 0 passed`. Run `pnpm test` → vitest reports no test files (exit code 0; add `--passWithNoTests` to the script). Run `pnpm tauri dev` → a window opens.

- [ ] **Step 4: Commit** `chore: scaffold Tauri 2 + React app`

---

### Task 3: Domain types and fixtures

**Files:**
- Create: `src-tauri/src/domain.rs`, `src-tauri/tests/fixtures/{items_110.json,commodities_110.json,pricetable_pvp_alliance.txt}`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "lowercase")]
  pub enum House { Alliance, Horde, Blackwater }
  impl House { pub const ALL: [House; 3]; pub fn auction_house_id(self) -> u32; pub fn ahledger_market(self) -> Option<&'static str>; pub fn as_str(self) -> &'static str; pub fn from_str(s: &str) -> Option<House>; }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "lowercase")]
  pub enum Feed { Items, Commodities }   // as_str(): "items" | "commodities"; Feed::ALL
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
  pub enum TimeLeft { Short, Medium, Long, VeryLong }
  impl TimeLeft { pub fn min_remaining_secs(self) -> i64; pub fn as_str(self) -> &'static str; pub fn from_str(s:&str)->Option<Self>; }
  #[derive(Clone, Debug, PartialEq)]
  pub struct Listing { pub auction_id: i64, pub item_id: i64, pub variant_key: String, pub unit_price: Option<i64>, pub quantity: i64, pub time_left: TimeLeft }
  pub const GAP_FACTOR: f64 = 1.5;
  pub const STALE_FACTOR: i64 = 2;
  ```

- [ ] **Step 1: Write failing tests in `domain.rs`**

```rust
#[test] fn house_ids() { assert_eq!(House::Alliance.auction_house_id(), 110); assert_eq!(House::Horde.auction_house_id(), 113); assert_eq!(House::Blackwater.auction_house_id(), 116); }
#[test] fn ahledger_markets() { assert_eq!(House::Alliance.ahledger_market(), Some("forever.pvp.alliance.us")); assert_eq!(House::Horde.ahledger_market(), Some("forever.pvp.horde.us")); assert_eq!(House::Blackwater.ahledger_market(), None); }
#[test] fn time_left_minimums() { assert_eq!(TimeLeft::Short.min_remaining_secs(), 0); assert_eq!(TimeLeft::Medium.min_remaining_secs(), 1800); assert_eq!(TimeLeft::Long.min_remaining_secs(), 7200); assert_eq!(TimeLeft::VeryLong.min_remaining_secs(), 43200); }
#[test] fn time_left_parses_api_strings() { assert_eq!(TimeLeft::from_str("VERY_LONG"), Some(TimeLeft::VeryLong)); assert_eq!(TimeLeft::from_str("bogus"), None); }
```

- [ ] **Step 2: Run** `cargo test domain` → FAIL (not defined). **Step 3: Implement.** **Step 4: Run** → PASS.

- [ ] **Step 5: Create fixtures from the spike snapshots**

Spike data is at `/private/tmp/claude-501/-Users-felipesouza-Projetos-Felipe-auctioneer/d742f889-9706-4876-9f5d-e67de1ac462d/scratchpad/spike/`. If that's gone, re-download with the Blizzard token. Fixtures:
- `items_110.json`: `{"auctions": [first 300 auctions of ah_110] + [every auction of item 727]}`, top-level `id` and `name` kept. Build it with `jq`.
- `commodities_110.json`: `{"auctions": first 300 of com_110}`.
- `pricetable_pvp_alliance.txt`: the header line plus the first 300 rows of `ahl/pt_forever.pvp.alliance.us.txt`. Rewrite the header's row count to `300`.

- [ ] **Step 6: Commit** `feat: add domain types and API fixtures`

---

### Task 4: Blizzard feed parsing

**Files:**
- Create: `src-tauri/src/blizzard/parse.rs`

**Interfaces:**
- Consumes: `Listing` and `TimeLeft` (Task 3).
- Produces:
  - `pub fn parse_items_feed(body: &[u8]) -> Result<Vec<Listing>, ParseError>`
  - `pub fn parse_commodities_feed(body: &[u8]) -> Result<Vec<Listing>, ParseError>`
  - `pub fn variant_key(bonus_lists: Option<&[i64]>) -> String`

- [ ] **Step 1: Failing tests** (read the fixtures with `include_bytes!("../../tests/fixtures/…")`)

```rust
#[test] fn variant_key_sorts_and_joins() { assert_eq!(variant_key(Some(&[12761, 5, 900])), "5:900:12761"); assert_eq!(variant_key(None), ""); assert_eq!(variant_key(Some(&[])), ""); }
#[test] fn items_feed_unit_price_is_buyout_over_quantity() { /* listing with buyout 600, quantity 3 in a tiny inline JSON → unit_price Some(200) */ }
#[test] fn items_feed_bid_only_has_no_price() { /* inline JSON {"auctions":[{"id":1,"item":{"id":5},"bid":100,"quantity":1,"time_left":"SHORT"}]} → unit_price None, quantity 1 */ }
#[test] fn items_fixture_parses_with_variants() { let l = parse_items_feed(FIX_ITEMS).unwrap(); assert!(l.len() >= 300); assert!(l.iter().filter(|x| x.item_id == 727).map(|x| &x.variant_key).collect::<std::collections::HashSet<_>>().len() > 1); }
#[test] fn commodities_fixture_uses_unit_price() { let l = parse_commodities_feed(FIX_COMM).unwrap(); assert_eq!(l.len(), 300); assert!(l.iter().all(|x| x.unit_price.is_some() && x.variant_key.is_empty())); }
#[test] fn unknown_time_left_is_error() { /* "time_left":"FOREVER" → Err */ }
```

- [ ] **Step 2: Run** `cargo test blizzard::parse` → FAIL. **Step 3: Implement** with serde structs mirroring the JSON. Integer division is fine (copper, truncated). **Step 4: Run** → PASS. **Step 5: Commit** `feat: parse Blizzard auction feeds`

---

### Task 5: AHledger price table parsing

**Files:**
- Create: `src-tauri/src/ahledger/parse.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct PriceTable { pub market: String /* dotted, e.g. forever.pvp.alliance.us */, pub as_of: i64, pub rows: Vec<AhlRow> }
  pub struct AhlRow { pub item_id: i64, pub median: Option<i64>, pub min_buyout: Option<i64>, pub quantity: Option<i64>, pub median_7d: Option<i64>, pub median_30d: Option<i64>, pub low_30d: Option<i64>, pub high_30d: Option<i64>, pub sales_per_day_x10: Option<i64> }
  pub fn parse_pricetable(text: &str) -> Result<PriceTable, ParseError>
  ```

- [ ] **Step 1: Failing tests**

```rust
#[test] fn header_market_uses_dots() { let t = parse_pricetable("AHL1|forever/pvp/horde/us|1793000000|1\n2589:20:18:3420:22:21:18:25:3600\n").unwrap(); assert_eq!(t.market, "forever.pvp.horde.us"); assert_eq!(t.as_of, 1793000000); assert_eq!(t.rows[0].sales_per_day_x10, Some(3600)); assert_eq!(t.rows[0].min_buyout, Some(18)); }
#[test] fn empty_field_is_none() { let t = parse_pricetable("AHL1|forever/pvp/horde/us|1|1\n12784:1500000::2:::::\n").unwrap(); assert_eq!(t.rows[0].min_buyout, None); assert_eq!(t.rows[0].quantity, Some(2)); }
#[test] fn wrong_format_tag_is_error() { assert!(parse_pricetable("XYZ|a|1|0\n").is_err()); }
#[test] fn fixture_has_300_rows() { assert_eq!(parse_pricetable(include_str!("../../tests/fixtures/pricetable_pvp_alliance.txt")).unwrap().rows.len(), 300); }
```

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. **Step 5: Commit** `feat: parse AHledger price tables`

---

### Task 6: HTTP clients (Blizzard + AHledger)

**Files:**
- Create: `src-tauri/src/blizzard/client.rs`, `src-tauri/src/ahledger/client.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct BlizzardConfig { pub api_base: String /* https://us.api.blizzard.com */, pub token_url: String /* https://oauth.battle.net/token */, pub client_id: String, pub client_secret: String }
  pub enum FeedFetch { NotModified, Fresh { last_modified: i64 /* unix secs from header */, body: bytes::Bytes } }
  #[derive(thiserror::Error, Debug)] pub enum FetchError { #[error("unauthorized")] Unauthorized, #[error("rate limited, retry after {0}s")] RateLimited(u64), #[error("http {0}")] Status(u16), #[error("missing Last-Modified")] NoLastModified, #[error(transparent)] Net(#[from] reqwest::Error) }
  pub struct BlizzardClient { /* reqwest::Client, config, cached token + expiry behind tokio::sync::Mutex */ }
  impl BlizzardClient {
      pub fn new(config: BlizzardConfig) -> Self;
      pub async fn fetch_feed(&self, house: House, feed: Feed, if_modified_since: Option<i64>) -> Result<FeedFetch, FetchError>;
      pub async fn get_json(&self, path_and_query: &str) -> Result<serde_json::Value, FetchError>; // used by static_data
  }
  pub enum TableFetch { NotModified, Fresh { etag: Option<String>, table: PriceTable } }
  pub struct AhledgerClient { /* base: String (https://api.ahledger.com), key: Option<String> */ }
  impl AhledgerClient { pub fn new(base: String, key: Option<String>) -> Self; pub async fn fetch_pricetable(&self, market: &str, if_none_match: Option<&str>) -> Result<TableFetch, FetchError>; }
  ```
- Token: POST form `grant_type=client_credentials` with basic auth. Cache until `expires_in − 60` s. On a 401 from an API call, drop the token and retry once; a second 401, or a 401 from the token endpoint, gives `Unauthorized`. A 429 or 503 reads `Retry-After` (default 60) into `RateLimited`.

- [ ] **Step 1: Failing tests** (in `#[cfg(test)]` with `httpmock::MockServer`, both URLs pointed at the mock)

```rust
#[tokio::test] async fn fetch_feed_fresh_parses_last_modified() { /* mock token + GET /data/wow/ruleset/1/auctions/110 with Last-Modified "Sat, 14 Nov 2026 17:17:47 GMT" → Fresh{last_modified: 1794676667, ..} */ }
#[tokio::test] async fn fetch_feed_sends_if_modified_since_and_handles_304() { /* expect header If-Modified-Since: "Sat, 14 Nov 2026 17:17:47 GMT"; respond 304 → NotModified */ }
#[tokio::test] async fn commodities_url_has_suffix() { /* Feed::Commodities hits /data/wow/ruleset/1/auctions/113/commodities */ }
#[tokio::test] async fn api_401_refreshes_token_once() { /* first GET 401, token endpoint hit twice, second GET 200 → Fresh; token mock hits == 2 */ }
#[tokio::test] async fn token_401_is_unauthorized() { /* token endpoint 401 → Err(Unauthorized) */ }
#[tokio::test] async fn rate_limited_reads_retry_after() { /* 429 Retry-After: 30 → RateLimited(30) */ }
#[tokio::test] async fn ahledger_sends_key_and_etag() { /* expect Authorization: Bearer ahlk_x and If-None-Match: "abc"; respond 304 → NotModified */ }
#[tokio::test] async fn ahledger_fresh_returns_etag_and_table() { /* 200 body = fixture, ETag "v2" → Fresh{etag: Some("\"v2\""), table.rows.len()==300} */ }
```

- [ ] **Steps 2–4:** run `cargo test client` → FAIL, implement, run → PASS. Parse HTTP dates with `chrono::DateTime::parse_from_rfc2822`, and format `If-Modified-Since` as an RFC 7231 GMT string. **Step 5: Commit** `feat: add Blizzard and AHledger HTTP clients`

---

### Task 7: Database schema and settings

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/schema.sql`, `src-tauri/src/settings.rs`

**Interfaces:**
- Produces:
  - `pub fn open(path: &Path) -> rusqlite::Result<Connection>` (WAL mode, migrations applied) and `pub fn open_in_memory() -> Connection` (for tests).
  - `pub type Db = std::sync::Arc<std::sync::Mutex<Connection>>`.
  - The schema in `schema.sql` (migration 1). Later tasks rely on these exact names:
  ```sql
  CREATE TABLE feed_snapshots (id INTEGER PRIMARY KEY, house TEXT NOT NULL, feed TEXT NOT NULL, last_modified INTEGER NOT NULL, fetched_at INTEGER NOT NULL, listing_count INTEGER NOT NULL, dt_secs INTEGER, gap INTEGER NOT NULL DEFAULT 0, UNIQUE(house, feed, last_modified));
  CREATE TABLE item_aggregates (snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id), item_id INTEGER NOT NULL, variant_key TEXT NOT NULL, min_price INTEGER, median INTEGER, p25 INTEGER, p75 INTEGER, quantity INTEGER NOT NULL, listings INTEGER NOT NULL, PRIMARY KEY(snapshot_id, item_id, variant_key));
  CREATE TABLE volume (snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id), item_id INTEGER NOT NULL, variant_key TEXT NOT NULL, est_sold INTEGER NOT NULL, expired_unknown INTEGER NOT NULL, PRIMARY KEY(snapshot_id, item_id, variant_key));
  CREATE TABLE raw_listings (snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id), auction_id INTEGER NOT NULL, item_id INTEGER NOT NULL, variant_key TEXT NOT NULL, unit_price INTEGER, quantity INTEGER NOT NULL, time_left TEXT NOT NULL, PRIMARY KEY(snapshot_id, auction_id));
  CREATE TABLE daily_rollups (house TEXT NOT NULL, item_id INTEGER NOT NULL, variant_key TEXT NOT NULL, day TEXT NOT NULL /* YYYY-MM-DD UTC */, min_price INTEGER, median INTEGER, avg_quantity INTEGER, est_sold INTEGER NOT NULL, PRIMARY KEY(house, item_id, variant_key, day));
  CREATE TABLE ahledger_rows (market TEXT NOT NULL, as_of INTEGER NOT NULL, item_id INTEGER NOT NULL, median INTEGER, min_buyout INTEGER, quantity INTEGER, median_7d INTEGER, median_30d INTEGER, low_30d INTEGER, high_30d INTEGER, sales_per_day_x10 INTEGER, PRIMARY KEY(market, as_of, item_id));
  CREATE TABLE items (item_id INTEGER PRIMARY KEY, name TEXT NOT NULL, quality TEXT, item_class TEXT, item_subclass TEXT, level INTEGER, icon_url TEXT, purchase_price INTEGER, purchase_quantity INTEGER, sell_price INTEGER, static_ns TEXT NOT NULL, fetched_at INTEGER NOT NULL);
  CREATE TABLE source_status (source TEXT NOT NULL /* 'blizzard' | 'ahledger' */, house TEXT NOT NULL, feed TEXT NOT NULL /* 'items'|'commodities'|'pricetable' */, last_success INTEGER, last_error_at INTEGER, last_error TEXT, last_modified INTEGER, etag TEXT, PRIMARY KEY(source, house, feed));
  CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
  CREATE INDEX idx_snap_house_feed_time ON feed_snapshots(house, feed, last_modified);
  CREATE INDEX idx_agg_item ON item_aggregates(item_id, variant_key);
  ```
  ```rust
  pub const DEFAULT_CADENCE_SECS: i64 = 3600; // replaced by Task 1 Step 6
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
  pub struct AppSettings { pub cadence_secs: i64, pub cut_bps_alliance: i64, pub cut_bps_horde: i64, pub cut_bps_blackwater: i64, pub launch_at_login: bool, pub collection_paused: bool }
  pub fn load_settings(conn: &Connection) -> AppSettings;          // defaults: DEFAULT_CADENCE_SECS, 500, 500, 1500, true, false
  pub fn save_settings(conn: &Connection, s: &AppSettings) -> rusqlite::Result<()>;
  impl AppSettings { pub fn cut_bps(&self, h: House) -> i64; }
  ```

- [ ] **Step 1: Failing tests**

```rust
#[test] fn migrations_create_tables() { let c = open_in_memory(); for t in ["feed_snapshots","item_aggregates","volume","raw_listings","daily_rollups","ahledger_rows","items","source_status","settings"] { assert!(c.query_row("SELECT 1 FROM sqlite_master WHERE name=?1",[t],|_|Ok(())).is_ok(), "{t}"); } }
#[test] fn settings_defaults() { let s = load_settings(&open_in_memory()); assert_eq!((s.cut_bps_alliance, s.cut_bps_horde, s.cut_bps_blackwater), (500,500,1500)); assert_eq!(s.cadence_secs, DEFAULT_CADENCE_SECS); assert!(!s.collection_paused); }
#[test] fn settings_roundtrip() { /* save modified, load → equal */ }
```

- [ ] **Steps 2–4:** run → FAIL, implement (settings stored as one JSON value under key `app`), run → PASS. **Step 5: Commit** `feat: add SQLite schema and settings`

---

### Task 8: Aggregation and volume diff

**Files:**
- Create: `src-tauri/src/processing/aggregate.rs`, `src-tauri/src/processing/volume.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct ItemAggregate { pub item_id: i64, pub variant_key: String, pub min_price: Option<i64>, pub median: Option<i64>, pub p25: Option<i64>, pub p75: Option<i64>, pub quantity: i64, pub listings: i64 }
  pub fn aggregate(listings: &[Listing]) -> Vec<ItemAggregate>;   // sorted by (item_id, variant_key)
  pub struct VolumeRow { pub item_id: i64, pub variant_key: String, pub est_sold: i64, pub expired_unknown: i64 }
  pub fn diff(prev: &[Listing], next: &[Listing], dt_secs: i64) -> Vec<VolumeRow>; // only rows with a non-zero value
  pub fn is_gap(dt_secs: i64, cadence_secs: i64) -> bool;         // dt > GAP_FACTOR * cadence
  ```
- Percentiles are quantity-weighted, nearest-rank over priced units: expand each priced listing to `quantity` units and sort ascending; the value at percentile p is the unit at index `ceil(p × N) − 1`. Bid-only listings count toward `quantity` and `listings`, but not toward prices.
- Diff rules, matching listings by `auction_id`:
  - In prev but not in next: if `time_left.min_remaining_secs() > dt_secs`, add its quantity to `est_sold`, else to `expired_unknown`.
  - In both, with quantity decreased: add the decrease to `est_sold`.
  - New listings and quantity increases are ignored.

- [ ] **Step 1: Failing tests**

```rust
#[test] fn weighted_percentiles() { // prices 100×1, 200×3
    let a = aggregate(&[l(1,5,"",Some(100),1), l(2,5,"",Some(200),3)]); assert_eq!((a[0].min_price,a[0].p25,a[0].median,a[0].p75), (Some(100),Some(100),Some(200),Some(200))); assert_eq!((a[0].quantity,a[0].listings),(4,2)); }
#[test] fn bid_only_counts_but_no_price() { let a = aggregate(&[l(1,5,"",None,2)]); assert_eq!((a[0].min_price,a[0].quantity,a[0].listings),(None,2,1)); }
#[test] fn variants_aggregate_separately() { let a = aggregate(&[l(1,727,"12718",Some(257),1), l(2,727,"12721",Some(7300),1)]); assert_eq!(a.len(), 2); }
#[test] fn hourly_diff_classifies_by_time_left() { // dt 3600: vanished LONG→sold, VERY_LONG→sold, MEDIUM→expired, SHORT→expired
    /* assert est_sold == qty(LONG)+qty(VERY_LONG), expired_unknown == qty(MEDIUM)+qty(SHORT) */ }
#[test] fn short_dt_counts_medium_as_sold() { /* dt 1200: vanished MEDIUM (1800 > 1200) → est_sold */ }
#[test] fn commodity_partial_buy_counts_decrease() { /* same auction_id qty 10 → 4 → est_sold 6, expired 0 */ }
#[test] fn gap_threshold() { assert!(!is_gap(5400, 3600)); assert!(is_gap(5401, 3600)); }
```
(`l(auction_id, item_id, variant, unit_price, qty)` is a test helper with `time_left: VeryLong`; the time_left tests set it explicitly.)

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. **Step 5: Commit** `feat: per-snapshot aggregation and volume diff`

---

### Task 9: Ingest, rollups and retention

**Files:**
- Create: `src-tauri/src/processing/ingest.rs`, `src-tauri/src/processing/retention.rs`

**Interfaces:**
- Consumes: `aggregate`, `diff`, `is_gap` (Task 8); schema (Task 7).
- Produces:
  ```rust
  pub enum IngestOutcome { Duplicate, Ingested { snapshot_id: i64, gap: bool } }
  pub fn ingest_feed(conn: &mut Connection, house: House, feed: Feed, last_modified: i64, fetched_at: i64, listings: &[Listing], cadence_secs: i64) -> rusqlite::Result<IngestOutcome>;
  pub fn ingest_pricetable(conn: &mut Connection, table: &PriceTable) -> rusqlite::Result<bool>; // false if (market, as_of) already stored
  pub fn prune(conn: &mut Connection, now: i64) -> rusqlite::Result<()>; // 30-day aggregates/volume/ahledger_rows
  ```
- `ingest_feed` runs in one transaction:
  1. If `(house, feed, last_modified)` exists, return `Duplicate`.
  2. Insert the snapshot, with `dt_secs` = last_modified − previous snapshot's last_modified (NULL for the first) and `gap = is_gap(dt, cadence)`.
  3. Insert the aggregates.
  4. If there's a previous snapshot, load its raw listings, `diff`, and insert the volume rows. Rows are stored even for gaps; queries exclude them.
  5. Insert the new raw listings and delete raw listings of all but the newest 2 snapshots for this (house, feed).
  6. Recompute `daily_rollups` for each (item, variant) in this snapshot for its UTC day:
     - `min_price`: min of the day's per-snapshot mins.
     - `median`: median of the day's per-snapshot medians.
     - `avg_quantity`: rounded average quantity.
     - `est_sold`: sum of the day's volume `est_sold` over non-gap snapshots.

- [ ] **Step 1: Failing tests** (`open_in_memory`, listings built with the Task 8 helper)

```rust
#[test] fn first_ingest_has_no_volume() { /* Ingested{gap:false}; volume count 0; aggregates count > 0 */ }
#[test] fn second_ingest_writes_volume() { /* t0 then t0+3600 with one LONG listing vanished (qty 3) → volume est_sold 3 */ }
#[test] fn ingest_same_last_modified_is_duplicate() { /* ingest twice with the same last_modified → second returns Duplicate; feed_snapshots count 1 */ }
#[test] fn ingest_marks_gap_after_long_sleep() { /* t0 then t0+3*3600, cadence 3600 → Ingested{gap:true}; feed_snapshots.gap=1; volume row exists; daily_rollups.est_sold excludes it (0) */ }
#[test] fn raw_listings_keep_last_two_snapshots() { /* three ingests → SELECT COUNT(DISTINCT snapshot_id) FROM raw_listings == 2 */ }
#[test] fn feeds_are_independent() { /* ingest Items then Commodities for the same house and time → both Ingested, neither has dt_secs */ }
#[test] fn daily_rollup_median_of_medians() { /* three snapshots same day, medians 100, 300, 200 → rollup median 200, min = min of mins */ }
#[test] fn pricetable_ingest_dedupes_as_of() { /* same table twice → true then false */ }
#[test] fn prune_drops_old_aggregates_keeps_rollups() { /* snapshot at now-31d → prune → aggregates/volume gone, daily_rollups row still present, feed_snapshots row still present */ }
```

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. **Step 5: Commit** `feat: ingest snapshots with rollups and retention`

---

### Task 10: Item metadata and credentials

**Files:**
- Create: `src-tauri/src/blizzard/static_data.rs`, `src-tauri/src/credentials.rs`

**Interfaces:**
- Consumes: `BlizzardClient::get_json` (Task 6), `items` table (Task 7).
- Produces:
  ```rust
  pub fn missing_item_ids(conn: &Connection, limit: usize) -> rusqlite::Result<Vec<i64>>; // item_ids in item_aggregates not in items, plus rows whose static_ns != current setting 'static_ns'
  pub async fn fetch_items(client: &BlizzardClient, ids: &[i64]) -> Vec<ItemMeta>; // concurrency 10, per-item errors skipped
  pub struct ItemMeta { pub item_id: i64, pub name: String, pub quality: Option<String>, pub item_class: Option<String>, pub item_subclass: Option<String>, pub level: Option<i64>, pub icon_url: Option<String>, pub purchase_price: Option<i64>, pub purchase_quantity: Option<i64>, pub sell_price: Option<i64>, pub static_ns: String /* from _links.self.href namespace param, e.g. static-1.60.1_70399-forever-us */ }
  pub fn store_items(conn: &mut Connection, items: &[ItemMeta], now: i64) -> rusqlite::Result<()>; // upsert; also sets settings key 'static_ns' to the newest seen
  pub struct Credentials { pub blizzard_client_id: String, pub blizzard_client_secret: String, pub ahledger_key: Option<String> }
  pub fn load_credentials() -> Option<Credentials>;          // Keychain; in debug builds falls back to parse_env_file(repo .env)
  pub fn save_credentials(c: &Credentials) -> Result<(), keyring::Error>;
  pub fn credentials_status() -> CredentialsStatus;          // { blizzard_set: bool, ahledger_set: bool } — never the secrets
  pub fn parse_env_file(text: &str) -> std::collections::HashMap<String, String>;
  ```
- Item URL: `/data/wow/item/{id}?namespace=static-forever-us&locale=en_US`. Media URL: `/data/wow/media/item/{id}?…`, taking the asset whose `key == "icon"`. Use `quality.name`, `item_class.name`, `item_subclass.name`.
- In debug builds, locate the repo `.env` via `env!("CARGO_MANIFEST_DIR")/../.env`.

- [ ] **Step 1: Failing tests**

```rust
#[tokio::test] async fn fetch_item_reads_fields_and_namespace() { /* httpmock item 2589 JSON from spike (purchase_price 55, sell_price 13, _links.self.href ...namespace=static-1.60.1_70399-forever-us) + media → ItemMeta{name:"Linen Cloth", purchase_price:Some(55), static_ns:"static-1.60.1_70399-forever-us", icon_url:Some(..)} */ }
#[test] fn missing_ids_lists_unknown_items() { /* aggregates for 2589, 727; items has 2589 → [727] */ }
#[test] fn env_parsing() { let m = parse_env_file("BLIZZARD_CLIENT_ID=abc\n# c\nBLIZZARD_CLIENT_SECRET = \"s3\"\n"); assert_eq!(m["BLIZZARD_CLIENT_ID"], "abc"); assert_eq!(m["BLIZZARD_CLIENT_SECRET"], "s3"); }
```

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. Keychain functions aren't unit-tested; they're checked manually in Task 14. **Step 5: Commit** `feat: item metadata cache and credentials store`

---

### Task 11: Collector

**Files:**
- Create: `src-tauri/src/collector.rs`

**Interfaces:**
- Consumes: Tasks 6, 7, 9, 10.
- Produces:
  ```rust
  #[derive(Clone, Serialize)] pub struct CollectorStatus { pub state: String /* "running"|"paused"|"error"|"no_credentials" */, pub last_cycle_at: Option<i64>, pub last_new_snapshot_at: Option<i64>, pub message: Option<String> }
  #[derive(Clone, Serialize)] pub struct SnapshotEvent { pub house: House, pub feed: Feed, pub snapshot_id: i64 }
  pub struct Endpoints { pub blizzard_api: String, pub blizzard_token: String, pub ahledger: String } // Endpoints::production(); debug builds let env AUCTIONEER_AHLEDGER_BASE override `ahledger`
  pub struct Collector { /* db: Db, endpoints, paused: AtomicBool, status: Mutex<CollectorStatus>, last_ahledger_at, emit: Box<dyn Fn(CollectorEvent) + Send + Sync> */ }
  pub enum CollectorEvent { Status(CollectorStatus), Snapshot(SnapshotEvent) }
  impl Collector {
      pub fn new(db: Db, endpoints: Endpoints, emit: Box<dyn Fn(CollectorEvent) + Send + Sync>) -> Arc<Self>;
      pub async fn run_cycle(&self, now: i64) -> CycleReport;   // one pass; used by the loop and tests
      pub fn spawn_loop(self: Arc<Self>);                          // every 300 s, plus immediately at start and after a wall-clock jump > 600 s (wake)
      pub fn set_paused(&self, paused: bool); pub fn status(&self) -> CollectorStatus;
  }
  pub struct CycleReport { pub new_snapshots: usize, pub errors: Vec<String> }
  ```
- `run_cycle` steps:
  1. Load credentials; if missing, set state `no_credentials` and return.
  2. For each house × feed:
     - Call `fetch_feed`, sending `If-Modified-Since` = `source_status.last_modified`.
     - On `Fresh`, parse and run `ingest_feed` inside `tokio::task::spawn_blocking`.
     - Update `source_status` on success or error, and emit `Snapshot` for each `Ingested`.
  3. Fetch AHledger tables, using the stored `etag`, if any snapshot was new or ≥ 900 s have passed since the last AHledger fetch. Run `ingest_pricetable`.
  4. Fetch up to 500 `missing_item_ids` and store them.
  5. Run `prune` once a day.
- Error handling:
  - `Unauthorized` stops the Blizzard part of the cycle with state `error` and message "Blizzard credentials rejected".
  - `RateLimited(n)` makes the loop skip cycles for `n` seconds.
  - An AHledger error never prevents the Blizzard steps; they run first.
  - On an error state, back off 300 → 600 → 1200 s (cap 1800).
- The collector loads credentials on every cycle, so saving them in Settings takes effect on the next cycle.

- [ ] **Step 1: Failing tests** (httpmock serves the fixtures; `Collector` is built with `open_in_memory`; credentials are injected through a test-only `Collector::with_credentials`)

```rust
#[tokio::test] async fn cycle_ingests_all_six_feeds() { /* all feeds 200 with fixtures + Last-Modified → new_snapshots == 6; 6 Snapshot events emitted */ }
#[tokio::test] async fn second_cycle_not_modified_adds_nothing() { /* second cycle mocks return 304 → new_snapshots == 0; feed_snapshots count still 6 */ }
#[tokio::test] async fn cycle_with_bad_credentials_records_error() { /* token 401 → status.state == "error"; source_status.last_error contains "credentials"; no panic */ }
#[tokio::test] async fn ahledger_failure_does_not_block_blizzard() { /* ahledger 503 → Blizzard snapshots == 6; source_status for ahledger has last_error */ }
#[tokio::test] async fn paused_cycle_does_nothing() { /* set_paused(true) → no HTTP hits (mock hits == 0), state "paused" */ }
```

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. **Step 5: Commit** `feat: background collector`

---

### Task 12: Queries for the UI

**Files:**
- Create: `src-tauri/src/queries/market.rs`, `src-tauri/src/queries/history.rs`, `src-tauri/src/queries/health.rs`

**Interfaces:**
- Consumes: the schema and settings.
- Produces (all `#[derive(Serialize)]`, camelCase via `#[serde(rename_all = "camelCase")]`):
  ```rust
  pub struct MarketRow { item_id: i64, variant_key: String, name: String /* "Item #<id>" when unknown */, quality: Option<String>, item_class: Option<String>, item_subclass: Option<String>, icon_url: Option<String>, min_price: Option<i64>, median: Option<i64>, quantity: i64, median_7d: Option<i64>, est_sold_per_day: Option<f64>, trend_pct: Option<f64>, ahl_min: Option<i64>, ahl_delta_pct: Option<f64>, feed: Feed }
  pub struct MarketView { house: House, as_of: Option<i64>, partially_stale: bool, stale: bool, rows: Vec<MarketRow> }
  pub fn market(conn: &Connection, house: House, settings: &AppSettings, now: i64) -> rusqlite::Result<MarketView>;
  pub struct CrossHouseRow { house: House, min_price: Option<i64>, median: Option<i64>, quantity: i64, cut_bps: i64, as_of: i64 }
  pub fn cross_house(conn: &Connection, item_id: i64, variant_key: &str, settings: &AppSettings) -> rusqlite::Result<Vec<CrossHouseRow>>;
  pub enum HistoryRange { H24, D7, D30, All }   // serde: "24h" | "7d" | "30d" | "all"
  pub struct HistoryPoint { t: i64, min_price: Option<i64>, median: Option<i64>, quantity: Option<i64> }
  pub struct ItemHistory { points: Vec<HistoryPoint>, est_sold_daily: Vec<(String, i64)>, volatility_7d: Option<f64>, ahledger: Vec<(i64, Option<i64>)> }
  pub fn item_history(conn: &Connection, house: House, item_id: i64, variant_key: &str, range: HistoryRange, now: i64) -> rusqlite::Result<ItemHistory>;
  pub struct FeedHealth { source: String, house: House, feed: String, last_success: Option<i64>, last_error_at: Option<i64>, last_error: Option<String>, last_modified: Option<i64>, stale: bool }
  pub struct TimelinePoint { house: House, feed: Feed, t: i64, gap: bool }
  pub struct Disagreement { house: House, item_id: i64, name: String, blizzard_min: i64, ahledger_min: i64, delta_pct: f64 }
  pub struct Calibration { house: House, item_id: i64, name: String, est_sold_per_day: Option<f64>, ahl_sales_per_day: f64 }
  pub struct Health { feeds: Vec<FeedHealth>, timeline: Vec<TimelinePoint> /* last 7 days */, disagreements: Vec<Disagreement>, calibration: Vec<Calibration> /* top 50 by ahl_sales_per_day */ }
  pub fn health(conn: &Connection, settings: &AppSettings, now: i64) -> rusqlite::Result<Health>;
  ```
- Definitions (exact):
  - **Latest:** each row uses the latest snapshot of the feed the item lives in.
  - **`median_7d`:** the median of per-snapshot medians over the last 7 days. `None` if fewer than 24 h of snapshots exist for that item.
  - **`est_sold_per_day`:** the sum of `est_sold` over **non-gap** snapshots in the last 7 days ÷ the sum of those snapshots' `dt_secs` × 86400. `None` when the covered seconds are under 21600 (6 h).
  - **`trend_pct`:** (latest median − median_7d) ÷ median_7d × 100. `None` if either is `None`.
  - **`ahl_min`, `ahl_delta_pct`:**
    - Taken from the latest `ahledger_rows` for the house's market, matching `item_id` and using the min over all variants of the item. Every variant row of that item shows the same AHledger figure.
    - `ahl_delta_pct` = (blizzard_min_across_variants − ahl_min) ÷ ahl_min × 100.
    - Only set when |snapshot last_modified − ahledger as_of| ≤ 3600; otherwise `None`. Blackwater is always `None`.
  - **Disagreements:** items with |delta_pct| > 10 under the same time-window rule, sorted by |delta_pct| descending, limit 200.
  - **History:**
    - Ranges 24h, 7d and 30d come from `item_aggregates`; `All` comes from `daily_rollups`, with t = day start.
    - Wherever a snapshot is marked `gap`, insert a point with all-`None` values at t − 1 before it, so the chart breaks.
    - `volatility_7d` = population stddev ÷ mean of per-snapshot medians over 7 days. `None` with fewer than 24 points.
  - **Staleness:**
    - A feed is stale when now − last_modified > `STALE_FACTOR` × cadence_secs.
    - `MarketView.stale`: both feeds of the house are stale. `partially_stale`: exactly one is.

- [ ] **Step 1: Failing tests** (seed with `ingest_feed` / `ingest_pricetable`)

```rust
#[test] fn market_row_without_history_has_none_fields() { /* one snapshot → median_7d None, trend_pct None, est_sold_per_day None, name "Item #727" when items empty */ }
#[test] fn est_sold_per_day_excludes_gaps() { /* 8 hourly snapshots each selling 1 unit + one gap snapshot selling 50 → est ≈ 24.0 */ }
#[test] fn ahl_delta_only_when_close_in_time() { /* ahledger as_of within 3600 → Some(delta); 2 h apart → None */ }
#[test] fn ahl_compares_min_across_variants() { /* item 727 variants 257 and 7300, ahl min 260 → delta_pct ≈ -1.15 on both rows */ }
#[test] fn partially_stale_when_one_feed_old() { /* items feed fresh, commodities last_modified 3×cadence ago → partially_stale true, stale false */ }
#[test] fn history_inserts_break_at_gap() { /* snapshots t0, t0+3600, gap at t0+4*3600 → a None point at t0+4*3600-1 */ }
#[test] fn health_lists_disagreements_over_10pct() { /* blizz 100 vs ahl 80 (25%) listed; 100 vs 95 not */ }
#[test] fn cross_house_includes_cut() { /* rows for alliance/horde/blackwater with cut_bps 500/500/1500 */ }
```

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS. **Step 5: Commit** `feat: market, history and health queries`

---

### Task 13: Tauri wiring: commands, tray, autostart

**Files:**
- Create: `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: Tasks 7, 10, 11, 12.
- Produces these Tauri commands (frontend names in snake_case; args camelCase from the JS side):
  - `get_market(house) -> MarketView`
  - `get_item_history(house, itemId, variantKey, range) -> ItemHistory`
  - `get_cross_house(itemId, variantKey) -> Vec<CrossHouseRow>`
  - `get_health() -> Health`
  - `get_settings() -> AppSettings`
  - `save_settings(settings: AppSettings)`: also toggles autostart and the collector pause state.
  - `get_credentials_status() -> CredentialsStatus`
  - `save_credentials(blizzardClientId, blizzardClientSecret, ahledgerKey: Option<String>)`
  - `get_collector_status() -> CollectorStatus`
  - `run_collection_now()`
- Produces these events: `collector://status` (payload `CollectorStatus`) and `collector://snapshot` (payload `SnapshotEvent`).
- Database path: `app.path().app_data_dir()/auctioneer.sqlite3`.
- Tray:
  - Menu items: disabled status line "Updated HH:MM" or the collector message; "Open Auctioneer"; "Pause collection" / "Resume collection"; "Quit".
  - Left-click opens the window.
  - The window's `CloseRequested` is prevented and the window hidden.
- Autostart: `tauri-plugin-autostart` with `MacosLauncher::LaunchAgent`, enabled on first run (`launch_at_login` default true).
- Start `Collector::spawn_loop` in `setup`.

- [ ] **Step 1: Implement commands and tray.** Commands are thin; they lock `Db`, call the query, and map errors to `String`.

- [ ] **Step 2: Verify the build** with `cd src-tauri && cargo test && cargo clippy -- -D warnings`. Expected: all prior tests pass and there are no warnings.

- [ ] **Step 3: Manual check**
  - Run `pnpm tauri dev`. The menu-bar icon appears, and closing the window keeps the app alive in the tray.
  - Within ~1 minute, `sqlite3 ~/Library/Application\ Support/app.auctioneer.desktop/auctioneer.sqlite3 "select house,feed,listing_count from feed_snapshots"` shows 6 rows.
  - "Quit" exits.

- [ ] **Step 4: Commit** `feat: wire commands, tray and autostart`

---

### Task 14: Frontend shell, money formatting, Settings

**Files:**
- Create: `src/types.ts`, `src/api.ts`, `src/money.ts`, `src/money.test.ts`, `src/components/{Money,HouseSelect,AhledgerCredit}.tsx`, `src/views/SettingsView.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: Task 13 commands and events.
- Produces:
  - `types.ts` mirrors every Rust struct from Tasks 7, 11 and 12 in camelCase. `House = "alliance" | "horde" | "blackwater"`, `Feed = "items" | "commodities"`, `HistoryRange = "24h" | "7d" | "30d" | "all"`.
  - `api.ts`: one typed async function per command, e.g. `getMarket(house: House): Promise<MarketView>`, plus `onSnapshot(cb)` and `onStatus(cb)` wrapping `listen`.
  - `money.ts`: `formatCopper(c: number | null | undefined): string`.
  - `App.tsx`:
    - Holds `{ view: "market" | "item" | "health" | "settings", house: House, item?: { itemId: number; variantKey: string } }` in React state, with no router.
    - Has a top bar with the house selector and nav, and a collector status pill fed by `onStatus`.
    - Nav entries for Flips, Crafting, Watchlist and Cross-faction are not added; Plan 2 adds them.

- [ ] **Step 1: Failing test `src/money.test.ts`**

```ts
import { formatCopper } from "./money";
test.each([[0,"0c"],[50,"50c"],[10000,"1g"],[123456,"12g 34s 56c"],[10050,"1g 50c"],[-1250,"-12s 50c"],[null,"—"],[undefined,"—"]])("formatCopper(%p)", (c, s) => expect(formatCopper(c as any)).toBe(s));
```

- [ ] **Steps 2–4:** run `pnpm test` → FAIL, implement, run → PASS.

- [ ] **Step 5: Build `SettingsView`**
  - Blizzard client ID and secret fields, which show "saved" status from `getCredentialsStatus` and never pre-fill secrets.
  - AHledger key (optional), with a link to ahledger.com/developers.
  - Cadence (minutes), house cuts as percentages (stored as bps), launch at login, pause collection.
  - Save calls `saveCredentials` and `saveSettings`, then `runCollectionNow`.

- [ ] **Step 6: Manual check**
  - Remove the dev `.env` fallback temporarily by renaming `.env`, enter the credentials in Settings, and check `security find-generic-password -s app.auctioneer.desktop -a blizzard_client_id` finds the item.
  - The collector status goes to "running".
  - Restore `.env`.

- [ ] **Step 7: Commit** `feat: app shell, money formatting and settings`

---

### Task 15: Market and Item views

**Files:**
- Create: `src/views/MarketView.tsx`, `src/views/ItemView.tsx`, `src/components/PriceChart.tsx`, `src/marketFilter.ts`, `src/marketFilter.test.ts`

**Interfaces:**
- Consumes: `getMarket`, `getItemHistory`, `getCrossHouse`, `onSnapshot`, `formatCopper`.
- Produces: `filterRows(rows: MarketRow[], f: { search: string; itemClass: string | null; quality: string | null }): MarketRow[]`. Search is case-insensitive on the name, and is also an exact match on the item ID when the search is numeric.

- [ ] **Step 1: Failing test `src/marketFilter.test.ts`**: `filterRows` matches "linen" against "Linen Cloth", "2589" against itemId 2589, and the class filter excludes other classes.

- [ ] **Steps 2–4:** run → FAIL, implement, run → PASS.

- [ ] **Step 5: Build `MarketView`**
  - A TanStack Table, virtualized or paged at 100 rows.
  - Columns: icon+name (quality colour), variant (show `variantKey` or blank), min, median, qty, 7d median, "Est. sold/day", trend %, AHledger min, Δ vs AHledger.
  - Sortable columns. Class and quality filters built from the row data, plus a search box.
  - A banner when `stale` ("Data is stale — last snapshot HH:MM") or `partiallyStale` ("Commodities/Items feed is stale").
  - `<AhledgerCredit/>` under the table whenever any row has `ahlMin`.
  - Refetch on `onSnapshot` for the current house.
  - A row click opens ItemView.

- [ ] **Step 6: Build `ItemView` and `PriceChart`**
  - **Price chart:** ECharts line chart of min and median with a quantity bar on a second axis. `connectNulls: false`, so gaps break. An AHledger min overlay as a dashed series. Range tabs 24h / 7d / 30d / All.
  - **Est. sold:** a bar chart labelled "Est. sold per day".
  - **Other panels:** volatility (shown as %), a cross-house table (house, min, median, qty, cut %), and `<AhledgerCredit/>`.

- [ ] **Step 7: Manual check.** With real collected data, Market lists about 6k rows for Alliance. Clicking an item shows its chart, and switching house updates the table.

- [ ] **Step 8: Commit** `feat: market browser and item detail`

---

### Task 16: Health / Sources view

**Files:**
- Create: `src/views/HealthView.tsx`

**Interfaces:**
- Consumes: `getHealth`, `onSnapshot`, `onStatus`.

- [ ] **Step 1: Build `HealthView`**
  - **Feed table:** source, house, feed, last success, last error (time + message), last modified, and a stale badge.
  - **Timeline:** per house × feed, the last 7 days of snapshots as ticks, with gaps in a warning colour.
  - **Disagreements:** item, house, Blizzard min, AHledger min, Δ%. Clicking a row opens ItemView.
  - **Calibration:** item, est. sold/day (ours), AHledger sales/day, ratio.
  - `<AhledgerCredit/>`.
  - Refresh on events.

- [ ] **Step 2: Manual check**
  - Temporarily set a wrong AHledger base URL via a debug env var `AUCTIONEER_AHLEDGER_BASE`, read by `Endpoints::production()` in debug builds only. AHledger rows show an error while Blizzard rows stay green.
  - Revert.

- [ ] **Step 3: Full verification.** Run `cd src-tauri && cargo test && cargo clippy -- -D warnings && cd .. && pnpm test && pnpm tauri build`. Expected: all pass, and `src-tauri/target/release/bundle/macos/Auctioneer.app` exists.

- [ ] **Step 4: Commit** `feat: health and sources view`

---

## Plan 2 (written after Plan 1 ships)

Flips, Crafting, Watchlist + Alerts, and Cross-faction, building on `MarketRow`, `item_aggregates`, `raw_listings` and `AppSettings` cuts:
- **Crafting:** recipe sync from the Blizzard profession → skill-tier → recipe endpoints, plus the bundled, user-editable vendor-reagent list.
- **Alerts:** a watchlist rule engine evaluated after `ingest_feed`, using `tauri-plugin-notification`, with an Alerts view.
- **Flips:** the query over `raw_listings` of the latest snapshot vs `median_7d`.
- **Cross-faction:** the spread view.
