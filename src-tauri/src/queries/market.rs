use super::common::*;
use crate::domain::{Feed, House};
use crate::settings::AppSettings;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketRow {
    pub item_id: i64,
    pub variant_key: String,
    pub name: String,
    pub quality: Option<String>,
    pub item_class: Option<String>,
    pub item_subclass: Option<String>,
    pub icon_url: Option<String>,
    pub min_price: Option<i64>,
    pub median: Option<i64>,
    pub quantity: i64,
    pub median_7d: Option<i64>,
    pub est_sold_per_day: Option<f64>,
    pub trend_pct: Option<f64>,
    pub ahl_min: Option<i64>,
    pub ahl_delta_pct: Option<f64>,
    pub feed: Feed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketView {
    pub house: House,
    pub as_of: Option<i64>,
    pub partially_stale: bool,
    pub stale: bool,
    pub rows: Vec<MarketRow>,
}

/// Every item/variant in the house's latest snapshots, with 7-day context and the AHledger comparison.
pub fn market(conn: &Connection, house: House, settings: &AppSettings, now: i64) -> rusqlite::Result<MarketView> {
    let info = items_info(conn)?;
    let ahl = latest_ahledger(conn, house)?;
    let item_mins = latest_item_mins(conn, house)?;
    let from = now - WEEK;
    let mut rows = Vec::new();
    let mut stale_flags = Vec::new();
    let mut as_of: Option<i64> = None;
    for feed in Feed::ALL {
        let latest = latest_snapshot(conn, house, feed)?;
        stale_flags.push(is_stale(latest.map(|x| x.1), now, settings.cadence_secs));
        let Some((snap_id, lm)) = latest else { continue };
        as_of = Some(as_of.map_or(lm, |a: i64| a.max(lm)));
        let medians = window_medians(conn, house, feed, from)?;
        let (sold, covered) = sold_in_window(conn, house, feed, from)?;
        for a in snapshot_aggregates(conn, snap_id)? {
            let key: Key = (a.item_id, a.variant_key.clone());
            let median_7d = medians.get(&key).and_then(|(ms, first, last)| {
                let mut ms = ms.clone();
                (last - first >= 86400).then(|| median_lower(&mut ms)).flatten()
            });
            let trend_pct = match (a.median, median_7d) {
                (Some(m), Some(m7)) if m7 > 0 => Some((m - m7) as f64 / m7 as f64 * 100.0),
                _ => None,
            };
            let (ahl_min, ahl_delta_pct) = match &ahl {
                Some(t) => {
                    let ahl_min = t.rows.get(&a.item_id).and_then(|x| x.0);
                    let blizz = item_mins.get(&a.item_id).map(|x| x.0);
                    (ahl_min, delta_pct(blizz, lm, ahl_min, t.as_of))
                }
                None => (None, None),
            };
            let i = info.get(&a.item_id);
            rows.push(MarketRow {
                item_id: a.item_id,
                name: display_name(i, a.item_id),
                quality: i.and_then(|x| x.quality.clone()),
                item_class: i.and_then(|x| x.item_class.clone()),
                item_subclass: i.and_then(|x| x.item_subclass.clone()),
                icon_url: i.and_then(|x| x.icon_url.clone()),
                min_price: a.min_price,
                median: a.median,
                quantity: a.quantity,
                median_7d,
                est_sold_per_day: per_day(sold.get(&key).copied().unwrap_or(0), covered),
                trend_pct,
                ahl_min,
                ahl_delta_pct,
                feed,
                variant_key: a.variant_key,
            });
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.variant_key.cmp(&b.variant_key)));
    let stale_count = stale_flags.iter().filter(|s| **s).count();
    Ok(MarketView { house, as_of, partially_stale: stale_count == 1, stale: stale_count == 2, rows })
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CrossHouseRow {
    pub house: House,
    pub min_price: Option<i64>,
    pub median: Option<i64>,
    pub quantity: i64,
    pub cut_bps: i64,
    pub as_of: i64,
}

/// The item/variant in each house's latest snapshot (houses where it isn't listed are omitted).
pub fn cross_house(conn: &Connection, item_id: i64, variant_key: &str, settings: &AppSettings) -> rusqlite::Result<Vec<CrossHouseRow>> {
    let mut out = Vec::new();
    for house in House::ALL {
        let mut found: HashMap<i64, CrossHouseRow> = HashMap::new();
        for feed in Feed::ALL {
            let Some((id, lm)) = latest_snapshot(conn, house, feed)? else { continue };
            let row = conn
                .query_row(
                    "SELECT min_price, median, quantity FROM item_aggregates WHERE snapshot_id = ?1 AND item_id = ?2 AND variant_key = ?3",
                    rusqlite::params![id, item_id, variant_key],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok();
            if let Some((min_price, median, quantity)) = row {
                found.insert(lm, CrossHouseRow { house, min_price, median, quantity, cut_bps: settings.cut_bps(house), as_of: lm });
            }
        }
        if let Some(newest) = found.keys().max().copied() {
            out.extend(found.remove(&newest));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::{Feed, TimeLeft};
    use crate::queries::testutil::*;
    use crate::settings::AppSettings;

    fn row<'a>(v: &'a MarketView, item: i64, variant: &str) -> &'a MarketRow {
        v.rows.iter().find(|r| r.item_id == item && r.variant_key == variant).expect("row")
    }

    #[test]
    fn market_row_without_history_has_none_fields() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 727, "", 500, 2, TimeLeft::Long)]);
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        let r = row(&v, 727, "");
        assert_eq!(r.name, "Item #727");
        assert_eq!((r.min_price, r.median, r.quantity), (Some(500), Some(500), 2));
        assert_eq!((r.median_7d, r.trend_pct, r.est_sold_per_day, r.ahl_min), (None, None, None, None));
        assert_eq!(r.feed, Feed::Items);
        assert_eq!(v.as_of, Some(NOW - 60));
    }

    #[test]
    fn known_item_uses_metadata_name() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Commodities, NOW - 60, &[li(1, 2589, "", 20, 5, TimeLeft::Long)]);
        name_item(&c, 2589, "Linen Cloth");
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        assert_eq!(row(&v, 2589, "").name, "Linen Cloth");
    }

    #[test]
    fn est_sold_per_day_excludes_gaps() {
        let mut c = open_in_memory();
        let t0 = NOW - 20 * H;
        for k in 0..8i64 {
            let mut ls: Vec<_> = (k..k + 10).map(|a| li(a, 5, "", 100, 1, TimeLeft::Long)).collect();
            if k == 7 {
                ls.extend((1000..1050).map(|a| li(a, 5, "", 100, 1, TimeLeft::VeryLong)));
            }
            seed(&mut c, House::Horde, Feed::Items, t0 + k * H, &ls);
        }
        // gap: 4h later, the 50 VERY_LONG listings vanished (counted as sold, but in a gap)
        let ls: Vec<_> = (8..18).map(|a| li(a, 5, "", 100, 1, TimeLeft::Long)).collect();
        seed(&mut c, House::Horde, Feed::Items, t0 + 11 * H, &ls);
        let v = market(&c, House::Horde, &AppSettings::default(), NOW).unwrap();
        let est = row(&v, 5, "").est_sold_per_day.unwrap();
        assert!((est - 24.0).abs() < 1e-9, "{est}");
    }

    #[test]
    fn median_7d_and_trend_after_a_day_of_history() {
        let mut c = open_in_memory();
        let t0 = NOW - 30 * H;
        for k in 0..30i64 {
            let price = if k == 29 { 150 } else { 100 };
            seed(&mut c, House::Alliance, Feed::Items, t0 + k * H, &[li(k, 5, "", price, 1, TimeLeft::Long)]);
        }
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        let r = row(&v, 5, "");
        assert_eq!(r.median_7d, Some(100));
        assert!((r.trend_pct.unwrap() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn ahl_delta_only_when_close_in_time() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - 60 - 1800, &[(5, 80, None)]);
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        assert_eq!(row(&v, 5, "").ahl_min, Some(80));
        assert!((row(&v, 5, "").ahl_delta_pct.unwrap() - 25.0).abs() < 1e-9);

        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - 60 - 2 * H, &[(5, 80, None)]);
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        assert_eq!(row(&v, 5, "").ahl_delta_pct, None);
    }

    #[test]
    fn ahl_compares_min_across_variants() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 727, "12718", 257, 1, TimeLeft::Long), li(2, 727, "12721", 7300, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - 60, &[(727, 260, None)]);
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        for variant in ["12718", "12721"] {
            let d = row(&v, 727, variant).ahl_delta_pct.unwrap();
            assert!((d - (-300.0 / 260.0)).abs() < 1e-9, "{d}");
        }
    }

    #[test]
    fn blackwater_has_no_ahledger() {
        let mut c = open_in_memory();
        seed(&mut c, House::Blackwater, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - 60, &[(5, 80, None)]);
        let v = market(&c, House::Blackwater, &AppSettings::default(), NOW).unwrap();
        assert_eq!(row(&v, 5, "").ahl_min, None);
    }

    #[test]
    fn partially_stale_when_one_feed_old() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long)]);
        seed(&mut c, House::Alliance, Feed::Commodities, NOW - 3 * H, &[li(2, 6, "", 10, 1, TimeLeft::Long)]);
        let v = market(&c, House::Alliance, &AppSettings::default(), NOW).unwrap();
        assert!(v.partially_stale);
        assert!(!v.stale);
        assert_eq!(v.rows.len(), 2);
    }

    #[test]
    fn empty_house_is_stale_with_no_rows() {
        let c = open_in_memory();
        let v = market(&c, House::Horde, &AppSettings::default(), NOW).unwrap();
        assert!(v.stale && !v.partially_stale && v.rows.is_empty() && v.as_of.is_none());
    }

    #[test]
    fn cross_house_includes_cut() {
        let mut c = open_in_memory();
        for (i, h) in House::ALL.into_iter().enumerate() {
            seed(&mut c, h, Feed::Items, NOW - 60, &[li(i as i64, 5, "", 100 + i as i64, 1, TimeLeft::Long)]);
        }
        let rows = cross_house(&c, 5, "", &AppSettings::default()).unwrap();
        let cuts: Vec<_> = rows.iter().map(|r| (r.house, r.cut_bps, r.min_price)).collect();
        assert_eq!(cuts, vec![(House::Alliance, 500, Some(100)), (House::Horde, 500, Some(101)), (House::Blackwater, 1500, Some(102))]);
    }
}
