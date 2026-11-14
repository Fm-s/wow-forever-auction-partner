use super::common::*;
use crate::domain::House;
use crate::processing::ingest::utc_day;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryRange {
    #[serde(rename = "24h")]
    H24,
    #[serde(rename = "7d")]
    D7,
    #[serde(rename = "30d")]
    D30,
    #[serde(rename = "all")]
    All,
}

impl HistoryRange {
    fn start_ts(self, now: i64) -> Option<i64> {
        match self {
            HistoryRange::H24 => Some(now - 86400),
            HistoryRange::D7 => Some(now - WEEK),
            HistoryRange::D30 => Some(now - 30 * 86400),
            HistoryRange::All => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub t: i64,
    pub min_price: Option<i64>,
    pub median: Option<i64>,
    pub quantity: Option<i64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemHistory {
    pub points: Vec<HistoryPoint>,
    /// (UTC day, est. units sold that day)
    pub est_sold_daily: Vec<(String, i64)>,
    pub volatility_7d: Option<f64>,
    /// (AHledger as_of, min buyout)
    pub ahledger: Vec<(i64, Option<i64>)>,
}

/// Price/quantity series for one item variant in one house. Snapshots where it isn't listed show
/// quantity 0 and no price; a gap snapshot is preceded by an all-None point so charts break there.
pub fn item_history(conn: &Connection, house: House, item_id: i64, variant_key: &str, range: HistoryRange, now: i64) -> rusqlite::Result<ItemHistory> {
    let from = range.start_ts(now);
    let feed: Option<String> = conn
        .query_row(
            "SELECT s.feed FROM item_aggregates a JOIN feed_snapshots s ON s.id = a.snapshot_id
             WHERE s.house = ?1 AND a.item_id = ?2 AND a.variant_key = ?3 ORDER BY s.last_modified DESC LIMIT 1",
            params![house.as_str(), item_id, variant_key],
            |r| r.get(0),
        )
        .optional()?;

    let mut points = Vec::new();
    match (from, &feed) {
        (Some(from), Some(feed)) => {
            let mut st = conn.prepare(
                "SELECT s.last_modified, s.gap, a.min_price, a.median, a.quantity FROM feed_snapshots s
                 LEFT JOIN item_aggregates a ON a.snapshot_id = s.id AND a.item_id = ?3 AND a.variant_key = ?4
                 WHERE s.house = ?1 AND s.feed = ?2 AND s.last_modified >= ?5 ORDER BY s.last_modified",
            )?;
            let mut rows = st.query(params![house.as_str(), feed, item_id, variant_key, from])?;
            while let Some(r) = rows.next()? {
                let t: i64 = r.get(0)?;
                if r.get::<_, bool>(1)? {
                    points.push(HistoryPoint { t: t - 1, min_price: None, median: None, quantity: None });
                }
                let qty: Option<i64> = r.get(4)?;
                points.push(HistoryPoint { t, min_price: r.get(2)?, median: r.get(3)?, quantity: Some(qty.unwrap_or(0)) });
            }
        }
        (None, Some(_)) => {
            let mut st = conn.prepare("SELECT day, min_price, median, avg_quantity FROM daily_rollups WHERE house = ?1 AND item_id = ?2 AND variant_key = ?3 ORDER BY day")?;
            let rows = st.query_map(params![house.as_str(), item_id, variant_key], |r| {
                let day: String = r.get(0)?;
                let t = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).expect("midnight").and_utc().timestamp()).unwrap_or(0);
                Ok(HistoryPoint { t, min_price: r.get(1)?, median: r.get(2)?, quantity: r.get(3)? })
            })?;
            points = rows.collect::<rusqlite::Result<_>>()?;
        }
        _ => {}
    }

    let from_day = from.map(|f| utc_day(f).0).unwrap_or_default();
    let mut st = conn.prepare("SELECT day, est_sold FROM daily_rollups WHERE house = ?1 AND item_id = ?2 AND variant_key = ?3 AND day >= ?4 ORDER BY day")?;
    let est_sold_daily = st.query_map(params![house.as_str(), item_id, variant_key, from_day], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;

    let mut st = conn.prepare(
        "SELECT a.median FROM item_aggregates a JOIN feed_snapshots s ON s.id = a.snapshot_id
         WHERE s.house = ?1 AND a.item_id = ?2 AND a.variant_key = ?3 AND s.last_modified >= ?4 AND a.median IS NOT NULL",
    )?;
    let medians: Vec<f64> = st.query_map(params![house.as_str(), item_id, variant_key, now - WEEK], |r| r.get::<_, i64>(0).map(|m| m as f64))?.collect::<rusqlite::Result<_>>()?;
    let volatility_7d = (medians.len() >= 24).then(|| {
        let n = medians.len() as f64;
        let mean = medians.iter().sum::<f64>() / n;
        let var = medians.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / n;
        if mean > 0.0 { Some(var.sqrt() / mean) } else { None }
    }).flatten();

    let ahledger = match house.ahledger_market() {
        Some(market) => {
            let mut st = conn.prepare("SELECT as_of, min_buyout FROM ahledger_rows WHERE market = ?1 AND item_id = ?2 AND as_of >= ?3 ORDER BY as_of")?;
            st.query_map(params![market, item_id, from.unwrap_or(0)], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?
        }
        None => Vec::new(),
    };
    Ok(ItemHistory { points, est_sold_daily, volatility_7d, ahledger })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::{Feed, TimeLeft};
    use crate::queries::testutil::*;

    #[test]
    fn history_inserts_break_at_gap() {
        let mut c = open_in_memory();
        let t0 = NOW - 10 * H;
        for t in [t0, t0 + H, t0 + 4 * H] {
            seed(&mut c, House::Alliance, Feed::Items, t, &[li(t, 5, "", 100, 1, TimeLeft::VeryLong)]);
        }
        let h = item_history(&c, House::Alliance, 5, "", HistoryRange::H24, NOW).unwrap();
        let ts: Vec<_> = h.points.iter().map(|p| (p.t, p.median)).collect();
        assert_eq!(ts, vec![(t0, Some(100)), (t0 + H, Some(100)), (t0 + 4 * H - 1, None), (t0 + 4 * H, Some(100))]);
    }

    #[test]
    fn unlisted_snapshot_has_zero_quantity_and_no_price() {
        let mut c = open_in_memory();
        let t0 = NOW - 5 * H;
        seed(&mut c, House::Alliance, Feed::Items, t0, &[li(1, 5, "", 100, 2, TimeLeft::VeryLong)]);
        seed(&mut c, House::Alliance, Feed::Items, t0 + H, &[li(2, 6, "", 1, 1, TimeLeft::VeryLong)]);
        let h = item_history(&c, House::Alliance, 5, "", HistoryRange::H24, NOW).unwrap();
        assert_eq!(h.points.len(), 2);
        assert_eq!((h.points[1].min_price, h.points[1].quantity), (None, Some(0)));
    }

    #[test]
    fn range_filters_points() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 3 * 86400, &[li(1, 5, "", 100, 1, TimeLeft::VeryLong)]);
        seed(&mut c, House::Alliance, Feed::Items, NOW - H, &[li(2, 5, "", 100, 1, TimeLeft::VeryLong)]);
        // the 3-day jump is a gap, so a break point precedes the second snapshot; count real points only
        let real = |r| item_history(&c, House::Alliance, 5, "", r, NOW).unwrap().points.iter().filter(|p| p.quantity.is_some()).count();
        assert_eq!(real(HistoryRange::H24), 1);
        assert_eq!(real(HistoryRange::D7), 2);
    }

    #[test]
    fn all_range_uses_daily_rollups() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 3 * 86400, &[li(1, 5, "", 100, 1, TimeLeft::VeryLong)]);
        seed(&mut c, House::Alliance, Feed::Items, NOW - H, &[li(2, 5, "", 300, 1, TimeLeft::VeryLong)]);
        let h = item_history(&c, House::Alliance, 5, "", HistoryRange::All, NOW).unwrap();
        assert_eq!(h.points.iter().map(|p| p.median).collect::<Vec<_>>(), vec![Some(100), Some(300)]);
        assert_eq!(h.est_sold_daily.len(), 2);
    }

    #[test]
    fn volatility_needs_24_points() {
        let mut c = open_in_memory();
        for k in 0..23i64 {
            seed(&mut c, House::Alliance, Feed::Items, NOW - (30 - k) * H, &[li(k, 5, "", 100, 1, TimeLeft::VeryLong)]);
        }
        assert_eq!(item_history(&c, House::Alliance, 5, "", HistoryRange::D7, NOW).unwrap().volatility_7d, None);
        seed(&mut c, House::Alliance, Feed::Items, NOW - 2 * H, &[li(99, 5, "", 124, 1, TimeLeft::VeryLong)]);
        // 23 x 100 and 1 x 124: mean 101, population stddev sqrt(23*1 + 23^2)/sqrt(24)... computed below
        let medians: Vec<f64> = std::iter::repeat_n(100.0, 23).chain([124.0]).collect();
        let mean = medians.iter().sum::<f64>() / 24.0;
        let sd = (medians.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / 24.0).sqrt();
        let v = item_history(&c, House::Alliance, 5, "", HistoryRange::D7, NOW).unwrap().volatility_7d.unwrap();
        assert!((v - sd / mean).abs() < 1e-12);
    }

    #[test]
    fn ahledger_overlay_matches_item_and_market() {
        let mut c = open_in_memory();
        seed(&mut c, House::Horde, Feed::Items, NOW - H, &[li(1, 5, "", 100, 1, TimeLeft::VeryLong)]);
        ahl(&mut c, "forever.pvp.horde.us", NOW - H, &[(5, 90, None), (6, 1, None)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - H, &[(5, 1, None)]);
        let h = item_history(&c, House::Horde, 5, "", HistoryRange::H24, NOW).unwrap();
        assert_eq!(h.ahledger, vec![(NOW - H, Some(90))]);
    }
}
