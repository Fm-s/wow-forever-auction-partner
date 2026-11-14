//! Shared lookups for the UI queries.
use crate::domain::{Feed, House, STALE_FACTOR};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

pub const WEEK: i64 = 7 * 86400;
/// AHledger figures are only compared with Blizzard ones this close in time.
pub const COMPARE_WINDOW_SECS: i64 = 3600;
/// est. sold/day needs at least this much non-gap coverage.
pub const MIN_COVERAGE_SECS: i64 = 6 * 3600;

pub type Key = (i64, String);
/// (per-snapshot medians, first snapshot time, last snapshot time)
pub type MedianWindow = (Vec<i64>, i64, i64);

pub fn latest_snapshot(conn: &Connection, house: House, feed: Feed) -> rusqlite::Result<Option<(i64, i64)>> {
    conn.query_row(
        "SELECT id, last_modified FROM feed_snapshots WHERE house = ?1 AND feed = ?2 ORDER BY last_modified DESC LIMIT 1",
        params![house.as_str(), feed.as_str()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

pub fn is_stale(last_modified: Option<i64>, now: i64, cadence_secs: i64) -> bool {
    last_modified.is_none_or(|lm| now - lm > STALE_FACTOR * cadence_secs)
}

/// Lower-middle median (same rule as daily rollups).
pub fn median_lower(v: &mut [i64]) -> Option<i64> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some(v[(v.len() - 1) / 2])
}

#[derive(Clone, Debug, Default)]
pub struct ItemInfo {
    pub name: Option<String>,
    pub quality: Option<String>,
    pub item_class: Option<String>,
    pub item_subclass: Option<String>,
    pub icon_url: Option<String>,
}

pub fn items_info(conn: &Connection) -> rusqlite::Result<HashMap<i64, ItemInfo>> {
    let mut st = conn.prepare("SELECT item_id, name, quality, item_class, item_subclass, icon_url FROM items")?;
    let rows = st.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, ItemInfo { name: r.get(1)?, quality: r.get(2)?, item_class: r.get(3)?, item_subclass: r.get(4)?, icon_url: r.get(5)? }))
    })?;
    rows.collect()
}

pub fn display_name(info: Option<&ItemInfo>, item_id: i64) -> String {
    info.and_then(|i| i.name.clone()).unwrap_or_else(|| format!("Item #{item_id}"))
}

pub struct LatestAgg {
    pub item_id: i64,
    pub variant_key: String,
    pub min_price: Option<i64>,
    pub median: Option<i64>,
    pub quantity: i64,
}

pub fn snapshot_aggregates(conn: &Connection, snapshot_id: i64) -> rusqlite::Result<Vec<LatestAgg>> {
    let mut st = conn.prepare("SELECT item_id, variant_key, min_price, median, quantity FROM item_aggregates WHERE snapshot_id = ?1")?;
    let rows = st.query_map([snapshot_id], |r| Ok(LatestAgg { item_id: r.get(0)?, variant_key: r.get(1)?, min_price: r.get(2)?, median: r.get(3)?, quantity: r.get(4)? }))?;
    rows.collect()
}

/// Per (item, variant): per-snapshot medians since `from`, with the first and last snapshot time.
pub fn window_medians(conn: &Connection, house: House, feed: Feed, from: i64) -> rusqlite::Result<HashMap<Key, MedianWindow>> {
    let mut st = conn.prepare(
        "SELECT a.item_id, a.variant_key, a.median, s.last_modified FROM item_aggregates a JOIN feed_snapshots s ON s.id = a.snapshot_id
         WHERE s.house = ?1 AND s.feed = ?2 AND s.last_modified >= ?3 AND a.median IS NOT NULL",
    )?;
    let mut out: HashMap<Key, MedianWindow> = HashMap::new();
    let mut rows = st.query(params![house.as_str(), feed.as_str(), from])?;
    while let Some(r) = rows.next()? {
        let key: Key = (r.get(0)?, r.get(1)?);
        let (m, t): (i64, i64) = (r.get(2)?, r.get(3)?);
        let e = out.entry(key).or_insert_with(|| (Vec::new(), t, t));
        e.0.push(m);
        e.1 = e.1.min(t);
        e.2 = e.2.max(t);
    }
    Ok(out)
}

/// est. units sold per (item, variant) over non-gap snapshots since `from`, and the seconds they cover.
pub fn sold_in_window(conn: &Connection, house: House, feed: Feed, from: i64) -> rusqlite::Result<(HashMap<Key, i64>, i64)> {
    let covered: i64 = conn.query_row(
        "SELECT COALESCE(SUM(dt_secs), 0) FROM feed_snapshots WHERE house = ?1 AND feed = ?2 AND gap = 0 AND dt_secs IS NOT NULL AND last_modified >= ?3",
        params![house.as_str(), feed.as_str(), from],
        |r| r.get(0),
    )?;
    let mut st = conn.prepare(
        "SELECT v.item_id, v.variant_key, SUM(v.est_sold) FROM volume v JOIN feed_snapshots s ON s.id = v.snapshot_id
         WHERE s.house = ?1 AND s.feed = ?2 AND s.gap = 0 AND s.last_modified >= ?3 GROUP BY v.item_id, v.variant_key",
    )?;
    let rows = st.query_map(params![house.as_str(), feed.as_str(), from], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?;
    Ok((rows.collect::<rusqlite::Result<_>>()?, covered))
}

pub fn per_day(sold: i64, covered_secs: i64) -> Option<f64> {
    (covered_secs >= MIN_COVERAGE_SECS).then(|| sold as f64 / covered_secs as f64 * 86400.0)
}

pub struct AhlLatest {
    pub as_of: i64,
    /// item_id -> (min_buyout, sales_per_day_x10)
    pub rows: HashMap<i64, (Option<i64>, Option<i64>)>,
}

pub fn latest_ahledger(conn: &Connection, house: House) -> rusqlite::Result<Option<AhlLatest>> {
    let Some(market) = house.ahledger_market() else { return Ok(None) };
    let as_of: Option<i64> = conn.query_row("SELECT MAX(as_of) FROM ahledger_rows WHERE market = ?1", [market], |r| r.get(0))?;
    let Some(as_of) = as_of else { return Ok(None) };
    let mut st = conn.prepare("SELECT item_id, min_buyout, sales_per_day_x10 FROM ahledger_rows WHERE market = ?1 AND as_of = ?2")?;
    let rows = st.query_map(params![market, as_of], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?;
    Ok(Some(AhlLatest { as_of, rows: rows.collect::<rusqlite::Result<_>>()? }))
}

/// (b - a) / a * 100 when both exist, a > 0 and the two observations are within COMPARE_WINDOW_SECS.
pub fn delta_pct(blizzard: Option<i64>, blizzard_t: i64, ahl: Option<i64>, ahl_t: i64) -> Option<f64> {
    match (blizzard, ahl) {
        (Some(b), Some(a)) if a > 0 && (blizzard_t - ahl_t).abs() <= COMPARE_WINDOW_SECS => Some((b - a) as f64 / a as f64 * 100.0),
        _ => None,
    }
}

/// Blizzard min unit price per item across variants and both feeds' latest snapshots, with that snapshot's time.
pub fn latest_item_mins(conn: &Connection, house: House) -> rusqlite::Result<HashMap<i64, (i64, i64)>> {
    let mut out: HashMap<i64, (i64, i64)> = HashMap::new();
    for feed in Feed::ALL {
        let Some((id, lm)) = latest_snapshot(conn, house, feed)? else { continue };
        for a in snapshot_aggregates(conn, id)? {
            if let Some(m) = a.min_price {
                let e = out.entry(a.item_id).or_insert((m, lm));
                if m < e.0 {
                    *e = (m, lm);
                }
            }
        }
    }
    Ok(out)
}
