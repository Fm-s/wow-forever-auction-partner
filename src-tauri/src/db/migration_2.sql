-- Per (house, feed, UTC day): seconds covered by non-gap snapshot intervals. Normalises daily sales
-- and gates 7-day figures without scanning per-snapshot aggregates.
CREATE TABLE daily_coverage (
    house TEXT NOT NULL,
    feed TEXT NOT NULL,
    day TEXT NOT NULL,
    covered_secs INTEGER NOT NULL,
    PRIMARY KEY(house, feed, day)
);
INSERT INTO daily_coverage(house, feed, day, covered_secs)
    SELECT house, feed, strftime('%Y-%m-%d', last_modified, 'unixepoch'), COALESCE(SUM(CASE WHEN gap = 0 THEN dt_secs END), 0)
    FROM feed_snapshots GROUP BY 1, 2, 3;
-- AHledger overlay looks rows up by item.
CREATE INDEX idx_ahl_item ON ahledger_rows(market, item_id, as_of);
