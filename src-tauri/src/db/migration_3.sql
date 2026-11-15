-- Item tooltips (from the item endpoint's preview_item), fetched once and kept locally.
-- tooltip_json NULL = the item exists in neither static namespace (not refetched).
CREATE TABLE item_tooltips (
    item_id INTEGER PRIMARY KEY,
    tooltip_json TEXT,
    fetched_at INTEGER NOT NULL
);
