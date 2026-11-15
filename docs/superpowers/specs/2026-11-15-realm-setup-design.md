# Realm type, region and first-launch setup — design

Approved 2026-11-15.

## Intent

The repo goes public, so the app can no longer assume PvP US. A new install asks where the user
plays (region + realm type) and for Blizzard API credentials, then tracks that realm's three
auction houses. The realm can be changed later in Settings.

## Decisions (the user's)

- One realm type at a time.
- Switching keeps the previous realm's history (aged out after 30 days like everything else).
- Regions: US and EU.
- First launch asks realm type and Blizzard credentials.

## Decisions (my calls)

- Setup is a two-step screen replacing the app until finished: (1) region + realm type,
  (2) Blizzard client ID/secret (+ optional AHledger key). "Check and start tracking" requests a
  Blizzard token with the credentials before saving them; a rejection says so. Saved credentials can
  be kept. Existing installs see setup once (`setupDone` defaults to false).
- Settings gains a Realm section (region, realm type); saving a change switches collection at once
  and says the old realm's history is kept.
- The top bar shows the active realm ("PvP · US"). House tabs stay Alliance / Horde / Blackwater.
- Groups, item metadata and tooltips are shared by all realms. Static item data always comes from
  the US API (Forever game data is the same in both regions), so switching region never refetches it.
- Collection is idle until setup is finished (status "Finish setup to start collecting").

## Verified facts (2026-11-15)

Same auction house ids in US and EU (`{region}.api.blizzard.com`, `dynamic-forever-{region}`),
verified by matching min buyouts against AHledger (50-81% exact vs <11% for other markets):

| Realm type | Alliance | Horde | Blackwater | AHledger market |
|---|---|---|---|---|
| PvP | 110 | 113 | 116 | `forever.pvp.{faction}.{region}` |
| Normal | 111 | 114 | 117 | `forever.normal…` |
| Roleplay | 119 | 120 | 121 | `forever.rp…` |
| Roleplay PvP | 123 | 124 | 125 | `forever.rppvp…` |

Blackwater ids by pattern (AHledger has no neutral markets). 112/115/118 empty.

## Approach

1. **House = realm + faction, stored as a region-qualified key (chosen).** `House` becomes a value
   (`{realm, faction}`) whose DB key is `{region}-{auction house id}` (e.g. `us-110`). Existing rows
   are migrated (`alliance` → `us-110`, `horde` → `us-113`, `blackwater` → `us-116`). Only four
   tables store the house (feed_snapshots, daily_rollups, daily_coverage, source_status); the rest
   hang off snapshot ids. Commands take a faction from the UI and resolve it with the current realm.
2. One database file per realm: simple switching, but groups/items/tooltips would split or need a
   second shared database. Rejected.
3. Wipe on switch: rejected by the user's "kept".

## Non-goals

Several realms at once, KR/CN, non-English item names.
