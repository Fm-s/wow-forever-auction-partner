# Realm Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Track any Forever realm type in US or EU, chosen at first launch (with Blizzard credentials) and changeable in Settings, keeping each realm's history.

**Architecture:** `House` becomes `{realm: Realm{region, kind}, faction}` with DB key `{region}-{id}`; migration 5 rewrites existing house keys; collector and queries take the realm from settings; UI keeps faction tabs and gains a setup screen and a Realm settings section.

**Tech Stack:** Rust (rusqlite, reqwest, httpmock), React 19 + TS, vitest (+happy-dom).

**Spec:** `docs/superpowers/specs/2026-11-15-realm-setup-design.md`

## Global Constraints

- House ids and AHledger market ids exactly as in the spec table; same ids in US and EU.
- Dynamic data: `https://{region}.api.blizzard.com`, namespace `dynamic-forever-{region}`. Static item data: always US.
- Settings JSON adds `region` ("us"|"eu", default "us"), `realmType` ("pvp"|"normal"|"rp"|"rppvp", default "pvp"), `setupDone` (default false).
- Production code never assumes PvP US; test-only shorthands (`House::Alliance` etc. = PvP US) are `#[cfg(test)]`.

## Review Focus

- Existing data must survive the migration and show up as PvP US.
- Switching realm must not mix houses: Sources timeline, cross-house and health show only the current realm.
- After a switch the first new snapshot of a returning realm must be marked a gap (no fake sales).
- Setup must not lose already-saved credentials when "keep" is chosen; rejected credentials are not saved.
- Collector must not collect before setup is done, and must pick up a realm change without restart.

---

### Task 1: Domain + migration
`domain.rs`: `Region`, `RealmType`, `Faction`, `Realm`, `House`; `House::key()`, `House::from_key()`, `realm.houses()`, `house.auction_house_id()`, `house.ahledger_market()`; serde of `House` = faction string. `migration_5.sql`. Tests: id/market table, key round-trip, migration rewrites the four tables.

### Task 2: Backend wiring
Client URLs per region; settings fields + `realm()`; collector reads realm per cycle, idles until setup; queries/health/commands resolve faction → house; `complete_setup` command validates credentials (token request) and saves credentials + settings. Tests: feed URL for EU RP-PvP, collector polls the configured realm's ids, idle before setup, health/timeline filter to realm, setup rejects bad credentials without saving.

### Task 3: UI
Setup screen, Realm section in Settings, realm label in top bar. Tests: settings form round-trip of realm fields; headless screenshots of setup and settings.
