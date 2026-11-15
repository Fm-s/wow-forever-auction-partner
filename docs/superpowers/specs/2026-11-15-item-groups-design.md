# Item groups — short design

Approved 2026-11-15.

## Intent

Named groups of item variants (e.g. "Flips", "Herbs") used to filter the Market table, so a goldmaker
can jump straight to the items they work with in any house.

## Decisions (the user's)

- **Membership is per variant**: item id + bonus-list key. Notched Shortsword `12920` and `12718` are
  separate members; items without variants use the empty key.
- **Groups are shared** across Alliance, Horde and Blackwater.
- **Adding/removing** is a right-click context menu on a Market row: groups listed with a check for
  membership (click toggles), and "New group…" with an inline name field that creates the group and
  adds the row.
- **Filtering** selects several groups at once and shows the union.

## Decisions (my calls)

- The Market toolbar gets a "Groups" control next to class/quality: a popover checklist with member
  counts and "Clear". No ticks = all items. The selection lives in App state, so it survives house
  switches and visiting an Item page; ids of deleted groups drop out of it.
- The context menu stays open after a toggle (several groups can be ticked in one go); Esc, a click
  outside, or scrolling closes it. Creating a group closes it.
- Rename and delete live in a "Groups" section in Settings (member counts; delete asks inline for
  confirmation). Deleting a group removes only the group and its membership rows.
- Names are trimmed, 1–60 characters, unique ignoring case: "A group named Flips already exists".
- Members not listed in the current house's snapshot just don't appear when filtering.

## Approaches considered

1. **SQLite tables + client-side filter (chosen)**: groups live with the rest of the data; the Market
   screen already holds every row, so filtering by a set of `item:variant` keys is instant.
2. Server-side filter in `get_market`: no benefit at ~14k rows and couples the market query to groups.
3. Browser localStorage: per-webview, not in the app database, lost with site data. Rejected.

## Storage

- `item_groups(id, name, created_at)` with a case-insensitive unique index on `name`.
- `item_group_members(group_id → item_groups ON DELETE CASCADE, item_id, variant_key, added_at)`,
  primary key `(group_id, item_id, variant_key)`.

Commands: `get_groups`, `create_group(name)`, `rename_group(id, name)`, `delete_group(id)`,
`set_group_member(groupId, itemId, variantKey, member)`.

## Non-goals

A group column in the table, bulk/multi-row add, per-group watchlist alerts (Plan 2 may build on
groups), import/export.
