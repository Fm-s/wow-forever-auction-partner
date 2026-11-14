CREATE TABLE feed_snapshots (
    id INTEGER PRIMARY KEY,
    house TEXT NOT NULL,
    feed TEXT NOT NULL,
    last_modified INTEGER NOT NULL,
    fetched_at INTEGER NOT NULL,
    listing_count INTEGER NOT NULL,
    dt_secs INTEGER,
    gap INTEGER NOT NULL DEFAULT 0,
    UNIQUE(house, feed, last_modified)
);
CREATE TABLE item_aggregates (
    snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id),
    item_id INTEGER NOT NULL,
    variant_key TEXT NOT NULL,
    min_price INTEGER,
    median INTEGER,
    p25 INTEGER,
    p75 INTEGER,
    quantity INTEGER NOT NULL,
    listings INTEGER NOT NULL,
    PRIMARY KEY(snapshot_id, item_id, variant_key)
);
CREATE TABLE volume (
    snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id),
    item_id INTEGER NOT NULL,
    variant_key TEXT NOT NULL,
    est_sold INTEGER NOT NULL,
    expired_unknown INTEGER NOT NULL,
    PRIMARY KEY(snapshot_id, item_id, variant_key)
);
CREATE TABLE raw_listings (
    snapshot_id INTEGER NOT NULL REFERENCES feed_snapshots(id),
    auction_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    variant_key TEXT NOT NULL,
    unit_price INTEGER,
    quantity INTEGER NOT NULL,
    time_left TEXT NOT NULL,
    PRIMARY KEY(snapshot_id, auction_id)
);
CREATE TABLE daily_rollups (
    house TEXT NOT NULL,
    item_id INTEGER NOT NULL,
    variant_key TEXT NOT NULL,
    day TEXT NOT NULL,
    min_price INTEGER,
    median INTEGER,
    avg_quantity INTEGER,
    est_sold INTEGER NOT NULL,
    PRIMARY KEY(house, item_id, variant_key, day)
);
CREATE TABLE ahledger_rows (
    market TEXT NOT NULL,
    as_of INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    median INTEGER,
    min_buyout INTEGER,
    quantity INTEGER,
    median_7d INTEGER,
    median_30d INTEGER,
    low_30d INTEGER,
    high_30d INTEGER,
    sales_per_day_x10 INTEGER,
    PRIMARY KEY(market, as_of, item_id)
);
CREATE TABLE items (
    item_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    quality TEXT,
    item_class TEXT,
    item_subclass TEXT,
    level INTEGER,
    icon_url TEXT,
    purchase_price INTEGER,
    purchase_quantity INTEGER,
    sell_price INTEGER,
    static_ns TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);
CREATE TABLE source_status (
    source TEXT NOT NULL,
    house TEXT NOT NULL,
    feed TEXT NOT NULL,
    last_success INTEGER,
    last_error_at INTEGER,
    last_error TEXT,
    last_modified INTEGER,
    etag TEXT,
    PRIMARY KEY(source, house, feed)
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE INDEX idx_snap_house_feed_time ON feed_snapshots(house, feed, last_modified);
CREATE INDEX idx_agg_item ON item_aggregates(item_id, variant_key);
