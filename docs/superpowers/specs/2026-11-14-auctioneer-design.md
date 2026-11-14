# Auctioneer — design

Date: 2026-11-14
Status: draft, pending approval

## Intent

A personal Mac app for professional goldmaking on **WoW: Forever, PvP ruleset, US region**. It tracks all three PvP auction houses (Alliance, Horde, Blackwater/neutral) from Blizzard's official API, builds its own price and volume history, and surfaces money-making opportunities: flips, crafting margins, cross-faction gaps, and price alerts.

AHledger (the source behind wow.gg) is used as an independent second opinion to validate that the app's numbers are right.

It is a single-user tool. Not distributed, not monetized.

## Goals (v1)

1. **Collection:** pull every new Blizzard snapshot for the three PvP houses, plus AHledger's PvP price tables, while the app is running.
2. **Market browser:** every item per house with min buyout, median, quantity, 7-day and 30-day medians, estimated units sold per day, and the AHledger figure beside it.
3. **Flipping:** individual listings priced well below their usual market value, ranked by expected profit after the house cut, filtered by how fast the item sells.
4. **Crafting margins:** every recipe's reagent cost vs. its output's sale value after the house cut, ranked by margin and by profit per day (margin × estimated sales per day).
5. **Watchlist and alerts:** per-item, per-house price rules that fire a macOS notification.
6. **Market trends:** per-item charts of price and quantity over time, daily averages, and volatility.
7. **Trade volume:** estimated units sold per hour and day, inferred from listings that disappear between snapshots.
8. **Cross-faction gaps:** the same item compared across Alliance, Horde and Blackwater, net of the house cuts.
9. **Source comparison:** Blizzard vs AHledger side by side, with disagreements flagged, plus a data-health view of last fetch times, gaps and errors.

## Non-goals

- Other rulesets (Normal, RP, RP-PvP), other regions, Retail or Classic Era. The house IDs are config, so adding these later is cheap, but v1 ships PvP US only.
- An in-game addon, or reading the user's own characters, bags, mail or sales.
- Automating anything in game (buying, posting, cancelling).
- A server or cloud collector. History has gaps when the Mac is off, asleep, or the app is quit, and that's accepted.
- Windows or Linux builds.
- wow.gg. Its backend refuses direct requests (403), and AHledger is the source behind it anyway.

## Anti-goals (failure even if it "works")

- **Silently wrong prices.** Stale or partial data must be visible as stale. A gap in history must show as a gap, not be interpolated.
- **Estimates presented as facts.** Trade volume is inferred, because Blizzard doesn't separate sold from cancelled. It is always labelled "est."
- **Abusing the data sources.** No polling faster than the data changes, and AHledger's credit link and terms are respected.
- **Republishing data.** Everything stays on the machine.

## Constraints and verified facts (spike, 2026-11-14)

### Blizzard API (verified with real calls)

- **Auth:** OAuth client credentials at `https://oauth.battle.net/token`. Credentials are currently in the project `.env` as `BLIZZARD_CLIENT_ID` and `BLIZZARD_CLIENT_SECRET`.
- **Namespaces:** `dynamic-forever-us` for auctions and rulesets, `static-forever-us` for items, media, professions and recipes. Forever has no realms or connected realms; those endpoints return 404.
- **Rulesets:** `/data/wow/ruleset/index` returns 1 PvP, 2 Normal, 3 Roleplay, 15 Roleplay PvP.
- **Houses:** `/data/wow/ruleset/{rid}/auctions/` lists house IDs. The same IDs and data come back under any ruleset in the path, so a house ID alone identifies a market.
- **PvP house IDs:**

  | ID | House | How it was identified |
  |---|---|---|
  | 110 | Alliance | 92% of min buyouts match AHledger exactly |
  | 113 | Horde | 89% exact match |
  | 116 | Blackwater (neutral) | by the ID pattern; AHledger has no neutral market to check against |

- **Per house, two feeds:**
  - `/data/wow/ruleset/1/auctions/{id}` (non-commodities):
    - Fields: `id`, `item{id, context?, bonus_lists?}`, `bid?`, `buyout?`, `quantity`, `time_left`.
    - `buyout` can be absent for bid-only auctions.
    - About 90k listings and 9 MB for a faction house; about 7k for Blackwater.
  - `/data/wow/ruleset/1/auctions/{id}/commodities`:
    - Fields: `id`, `item{id}`, `quantity`, `unit_price`, `time_left`.
    - About 72k listings and 6 MB for a faction house.
- **Freshness:** responses carry `Last-Modified`. The snapshot seen was about 17 minutes old, and the refresh cadence is still to be measured; hourly is expected. **Measuring it is the first task of the build:** log `Last-Modified` per feed for about 24 hours with a throwaway poller, before the gap and volume rules are fixed.
- **`time_left` buckets** (assumed from Classic; check during the measurement): `SHORT` under 30 minutes, `MEDIUM` 30 minutes to 2 hours, `LONG` 2 to 12 hours, `VERY_LONG` over 12 hours.
- **Random-suffix variants matter.** Around 21k listings in house 110 carry `bonus_lists`, and the same item ID varies hugely by variant: item 727 sells for 257c with one bonus list and 7,300c with another. Prices are keyed by **item ID + sorted bonus lists**, not by item ID alone.
- **Item data:** `/data/wow/item/{id}` and `/data/wow/media/item/{id}` in `static-forever-us` provide name, quality, class, level and icon. The resolved static namespace is versioned (`static-1.60.1_70399-forever-us`), so the cache is refreshed when the version changes.
- **Vendor prices:** items carry `purchase_price`, `purchase_quantity` and `sell_price`, but **`purchase_price` is set even for items no vendor sells**. Linen Cloth shows `purchase_price: 55`. So it can't be used on its own to decide whether a reagent is vendor-bought.
- **Recipes:** `/data/wow/profession/index` → profession → `skill_tiers[]` → `/profession/{id}/skill-tier/{tier}` → `categories[].recipes[]` → `/data/wow/recipe/{id}`. A recipe gives `crafted_item`, `reagents[{reagent, quantity}]` and `crafted_quantity.value`.

### AHledger API (official, documented at ahledger.com/developers)

- **Base:** `https://api.ahledger.com`. Markets are `forever.pvp.alliance.us` and `forever.pvp.horde.us`; there's no Blackwater market.
- **Price table:** `GET /v1/pricetable/{market}` returns plain text.
  - Header line: `AHL1|market|unixTime|rowCount`.
  - Each row: `itemId:median:minBuyout:quantity:median7d:median30d:low30d:high30d:salesPerDay×10`. An empty field means unknown.
  - Supports ETag (an unchanged table answers 304) and is cached for 60 seconds.
  - Prices come from player addon scans, so they lag Blizzard by minutes to hours.
- **Terms:**
  - Free for personal use, provided the app shows a "Prices from AHledger" credit link.
  - Without a key: 30 requests a minute from 2026-12-10.
  - With a free key (`Authorization: Bearer ahlk_…`): 300 a minute.
  - An optional key field goes in Settings.
- **Other endpoints** (`/prices/{market}/{item}?range=7d`, `/crafting/{market}`, `/deals/{market}`) exist but aren't needed in v1. The app computes its own figures from Blizzard data, and AHledger is only for comparison.

### Platform

- The app only collects while it's running. It runs as a menu-bar (tray) app that starts at login.
- Sleep and shutdown cause gaps.

## Approaches considered

| | Approach | Trade-off |
|---|---|---|
| A | Native Swift: SwiftUI app plus a login-item collector, SQLite, Swift Charts | Smallest and most native; Swift-only data crunching |
| B | Python collector run by launchd, plus a SwiftUI app reading the same database | Fastest analytics iteration; two languages and a Python environment to maintain on the Mac |
| **C** | **Tauri 2: Rust backend, React + TypeScript UI** | **Rich web table and chart libraries, and could go cross-platform later; two languages, and tray/background behaviour is less native** |

**Chosen: C, Tauri 2, with React + TypeScript.** It was the user's pick, over A, which was recommended for native background and menu-bar behaviour.

## Decided design

### Shape

- **One Tauri 2 app:**
  - **Rust side:** collector, processing, SQLite access and alert evaluation.
  - **React + TypeScript side:** all UI.
  - The two talk through Tauri commands, and events for "new snapshot processed".
- **Tray app at login:**
  - Starts at login into the menu bar, via `tauri-plugin-autostart`.
  - Closing the window hides it; the collector keeps running as a tokio task in the Rust process. Only "Quit" from the tray stops collection.
  - The tray menu shows last update time, collector status, and Open / Pause collection / Quit.
- **Notifications:** `tauri-plugin-notification`.
- **Storage:** a single SQLite database in the app data directory.
- **Credentials:**
  - Entered in Settings and stored in the macOS Keychain.
  - In dev builds, the `.env` file seeds them.
  - The `.env` file is gitignored.

### Collector

- **Polling:**
  - Every ~5 minutes, send conditional requests (`If-Modified-Since` with the last `Last-Modified`) for the 6 Blizzard feeds (3 houses × 2 feeds).
  - Only a changed feed is downloaded and processed.
  - **Feeds are tracked independently.** A *feed snapshot* is one download of one feed, stamped with its `Last-Modified`.
  - Aggregates, diffs, staleness and gaps are all per feed. An item's data comes from whichever feed it lives in: commodities or non-commodities, never both.
  - A house is **partially stale** when one feed is current and the other isn't. The UI marks that house, and alerts still run on the fresh feed.
  - *(Builder-check call.)*
- **AHledger:** fetch both price tables (with `If-None-Match`) whenever a new Blizzard snapshot lands, and at most every 15 minutes otherwise.
- **Token:** OAuth tokens are cached and refreshed on expiry or a 401.
- **Errors:** retry with backoff and respect `Retry-After`. Errors are recorded per source for the data-health view.
- **Catch-up:** on wake from sleep or at startup, fetch immediately.

### Processing per snapshot

1. **Variants:** normalize every listing to `(house, item_id, variant_key)`, where `variant_key` is the sorted bonus lists ("" when there are none), with a unit price. Use `buyout / quantity` for non-commodities and `unit_price` for commodities. Bid-only listings are kept for counts but excluded from prices.
2. **Aggregates:** per `(house, item, variant)` per snapshot, store min unit price, median, p25 and p75 (quantity-weighted), total quantity and listing count.
3. **Volume:**
   - Keep the previous snapshot's raw listings per house, by auction ID, plus quantity and time left.
   - Diff against the new snapshot, with Δt = the time between the two `Last-Modified` stamps.
   - A vanished listing counts as **est. sold** only if its previous `time_left` bucket's *minimum* remaining time is longer than Δt, so it couldn't have expired.
     - Otherwise it counts as **expired/unknown**.
     - With hourly snapshots, that makes `SHORT` and `MEDIUM` expired/unknown, and `LONG` and `VERY_LONG` est. sold.
   - A commodity listing whose quantity dropped counts the difference as sold.
   - **Gap:** Δt is more than 1.5× the measured normal cadence. The diff is still recorded but marked **unattributed** and excluded from per-hour rates.
   - **Calibration:** the Health view compares est. sold/day with AHledger's salesPerDay for the same items, so a systematically biased rule shows up. *(Builder-check call.)*
4. **Raw retention:** only the last two raw snapshots per house are kept; everything else is aggregates.
5. **Rollups:**
   - Per-snapshot aggregates are kept for 30 days.
   - Daily rollups (min, median, average quantity, est. sold) are kept indefinitely.
   - 7-day and 30-day medians are derived from these.
6. **Comparison:**
   - Store AHledger rows per market per fetch.
   - Compare at **item-ID level**, since AHledger has no variants, using the Blizzard min across variants.
   - Only compare when the two snapshots are within 60 minutes of each other.
7. **Alerts:** evaluated after each processed snapshot.

### Features (UI)

- **Market** (default view):
  - A sortable, filterable table for the selected house.
  - Columns: item, variant, min buyout, median, quantity, 7d median, est. sold/day, trend %, AHledger min, and the Blizzard vs AHledger delta.
  - Filters: item class/subclass, quality, text search.
- **Item detail:**
  - Charts of min, median and quantity over 24h / 7d / 30d / all, with gaps shown as breaks.
  - Est. sold per day chart, daily averages, volatility (coefficient of variation of hourly medians over 7 days).
  - A cross-house panel for Alliance, Horde and Blackwater.
  - AHledger overlay on the price chart.
- **Flips:**
  - Current listings with unit price ≤ X% of the 7d median (default 70%) and est. sold/day ≥ Y.
  - Ranked by profit: (7d median × (1 − cut) − price) × quantity.
- **Crafting:**
  - All recipes from Blizzard for the selected house.
  - Reagent cost uses the cheaper of AH min and vendor price, for items on the **vendor-reagent list**.
    - The vendor price is `purchase_price / purchase_quantity` from Blizzard.
  - **Vendor-reagent list:**
    - The app ships with a curated list of vendor-sold reagent item IDs: threads, vials, dyes, flux, salts, spices, and similar.
    - The user can toggle "vendor-sold" on any item in its detail view.
    - Items not on the list use AH prices only, and a reagent with no AH price marks the recipe "unpriced". *(Builder-check call.)*
  - Output value is the median × crafted quantity × (1 − cut).
  - Shows margin, margin %, est. sold/day and profit/day, filterable by profession.
- **Watchlist:**
  - Items or variants pinned per house.
  - Rule types: "min buyout below X", "min buyout above X", "a listing below X% of the 7d median".
  - A notification fires when a rule starts matching, not on every snapshot while it stays matched.
  - Clicking a notification opens the item if that's achievable without native bridging; it's a nice-to-have.
    - `tauri-plugin-notification` may not report clicks on desktop. If it can't, a click just brings the app forward on an **Alerts** view listing recent fired alerts.
    - That Alerts view is part of v1 either way. *(Builder-check call.)*
- **Cross-faction:**
  - Items listed in at least two houses, sorted by the spread between the cheapest buy and the best sell net of each house's cut.
  - Actually moving goods between houses is the user's own business.
- **Sources / Health:**
  - Last success and last error per feed, snapshot timeline with gaps.
  - Blizzard vs AHledger disagreement list: items whose min buyouts differ by more than 10% when the snapshots are close in time.
  - The AHledger credit link is shown here and anywhere AHledger numbers appear.
- **Settings:**
  - Blizzard credentials and AHledger key.
  - Flip and crafting thresholds, house cuts, launch at login.

### Money and display

- Everything is stored as integer copper and shown as `Xg Ys Zc`.
- **House cut:** 5% for Alliance and Horde, 15% for Blackwater. These are the Classic values, assumed for Forever and to be confirmed in game, so they're editable in Settings.

## Left to the builder

- Exact SQLite schema, indexes and migration tooling (rusqlite vs sqlx); the table and chart libraries (TanStack Table plus a chart library such as ECharts or Recharts are suggested); the visual layout and styling.
- JSON parsing strategy for the 15 MB payloads (e.g. serde streaming); worker threading.
- Default thresholds beyond the ones stated, and the exact definition of "trend %".
- Item and recipe cache refresh mechanics, and how icons are cached on disk.
- Test strategy. Real snapshots captured during the spike make good fixtures for the parsing, aggregation and diff logic.
