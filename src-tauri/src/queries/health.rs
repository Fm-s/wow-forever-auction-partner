use super::common::*;
use crate::domain::{Feed, House};
use crate::settings::AppSettings;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeedHealth {
    pub source: String,
    pub house: House,
    pub feed: String,
    pub last_success: Option<i64>,
    pub last_error_at: Option<i64>,
    pub last_error: Option<String>,
    pub last_modified: Option<i64>,
    pub stale: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePoint {
    pub house: House,
    pub feed: Feed,
    pub t: i64,
    pub gap: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Disagreement {
    pub house: House,
    pub item_id: i64,
    pub name: String,
    pub blizzard_min: i64,
    pub ahledger_min: i64,
    pub delta_pct: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub house: House,
    pub item_id: i64,
    pub name: String,
    pub est_sold_per_day: Option<f64>,
    pub ahl_sales_per_day: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub feeds: Vec<FeedHealth>,
    pub timeline: Vec<TimelinePoint>,
    pub disagreements: Vec<Disagreement>,
    pub calibration: Vec<Calibration>,
}

const DISAGREE_PCT: f64 = 10.0;
const DISAGREE_LIMIT: usize = 200;
const CALIBRATION_LIMIT: usize = 50;

fn feed_health(conn: &Connection, source: &str, house: House, feed: &str, now: i64, cadence: i64) -> rusqlite::Result<FeedHealth> {
    let row: Option<(Option<i64>, Option<i64>, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT last_success, last_error_at, last_error, last_modified FROM source_status WHERE source = ?1 AND house = ?2 AND feed = ?3",
            params![source, house.as_str(), feed],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let (last_success, last_error_at, last_error, last_modified) = row.unwrap_or((None, None, None, None));
    Ok(FeedHealth { source: source.into(), house, feed: feed.into(), last_success, last_error_at, last_error, last_modified, stale: is_stale(last_modified, now, cadence) })
}

/// Est. sold/day per item (summed over variants), using the coverage of the feed the item lives in.
fn est_per_item(conn: &Connection, house: House, now: i64) -> rusqlite::Result<HashMap<i64, Option<f64>>> {
    let mut out: HashMap<i64, Option<f64>> = HashMap::new();
    for feed in Feed::ALL {
        let Some((snap, _)) = latest_snapshot(conn, house, feed)? else { continue };
        let (sold, covered) = sold_in_window(conn, house, feed, now - WEEK)?;
        let mut by_item: HashMap<i64, i64> = HashMap::new();
        for a in snapshot_aggregates(conn, snap)? {
            by_item.entry(a.item_id).or_insert(0);
        }
        for ((item, _), s) in sold {
            *by_item.entry(item).or_insert(0) += s;
        }
        for (item, s) in by_item {
            out.insert(item, per_day(s, covered));
        }
    }
    Ok(out)
}

pub fn health(conn: &Connection, settings: &AppSettings, now: i64) -> rusqlite::Result<Health> {
    let cadence = settings.cadence_secs;
    let mut feeds = Vec::new();
    for house in House::ALL {
        for feed in Feed::ALL {
            feeds.push(feed_health(conn, "blizzard", house, feed.as_str(), now, cadence)?);
        }
    }
    for house in House::ALL.into_iter().filter(|h| h.ahledger_market().is_some()) {
        feeds.push(feed_health(conn, "ahledger", house, "pricetable", now, cadence)?);
    }

    let mut st = conn.prepare("SELECT house, feed, last_modified, gap FROM feed_snapshots WHERE last_modified >= ?1 ORDER BY last_modified, id")?;
    let timeline = st
        .query_map([now - WEEK], |r| {
            let h: String = r.get(0)?;
            let f: String = r.get(1)?;
            Ok((h, f, r.get::<_, i64>(2)?, r.get::<_, bool>(3)?))
        })?
        .filter_map(|x| x.ok())
        .filter_map(|(h, f, t, gap)| Some(TimelinePoint { house: House::from_str(&h)?, feed: Feed::from_str(&f)?, t, gap }))
        .collect();

    let info = items_info(conn)?;
    let mut disagreements = Vec::new();
    let mut calibration = Vec::new();
    for house in House::ALL {
        let Some(ahl) = latest_ahledger(conn, house)? else { continue };
        let mins = latest_item_mins(conn, house)?;
        for (item, &(min, t)) in &mins {
            let a = ahl.rows.get(item).and_then(|x| x.0);
            if let (Some(d), Some(a)) = (delta_pct(Some(min), t, a, ahl.as_of), a) {
                if d.abs() > DISAGREE_PCT {
                    disagreements.push(Disagreement { house, item_id: *item, name: display_name(info.get(item), *item), blizzard_min: min, ahledger_min: a, delta_pct: d });
                }
            }
        }
        let est = est_per_item(conn, house, now)?;
        for (item, (_, sales)) in &ahl.rows {
            if let Some(s) = sales {
                calibration.push(Calibration {
                    house,
                    item_id: *item,
                    name: display_name(info.get(item), *item),
                    est_sold_per_day: est.get(item).copied().flatten(),
                    ahl_sales_per_day: *s as f64 / 10.0,
                });
            }
        }
    }
    disagreements.sort_by(|a, b| b.delta_pct.abs().total_cmp(&a.delta_pct.abs()));
    disagreements.truncate(DISAGREE_LIMIT);
    calibration.sort_by(|a, b| b.ahl_sales_per_day.total_cmp(&a.ahl_sales_per_day));
    calibration.truncate(CALIBRATION_LIMIT);
    Ok(Health { feeds, timeline, disagreements, calibration })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::{Feed, TimeLeft};
    use crate::queries::testutil::*;
    use crate::settings::AppSettings;

    #[test]
    fn health_lists_disagreements_over_10pct() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long), li(2, 6, "", 100, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.alliance.us", NOW - 60, &[(5, 80, None), (6, 95, None)]);
        let h = health(&c, &AppSettings::default(), NOW).unwrap();
        assert_eq!(h.disagreements.len(), 1);
        assert_eq!((h.disagreements[0].item_id, h.disagreements[0].blizzard_min, h.disagreements[0].ahledger_min), (5, 100, 80));
        assert!((h.disagreements[0].delta_pct - 25.0).abs() < 1e-9);
    }

    #[test]
    fn feeds_list_all_expected_sources_even_without_status() {
        let c = open_in_memory();
        let h = health(&c, &AppSettings::default(), NOW).unwrap();
        assert_eq!(h.feeds.iter().filter(|f| f.source == "blizzard").count(), 6);
        assert_eq!(h.feeds.iter().filter(|f| f.source == "ahledger").count(), 2);
        assert!(h.feeds.iter().all(|f| f.stale));
    }

    #[test]
    fn feed_health_reads_status_and_staleness() {
        let c = open_in_memory();
        c.execute(
            "INSERT INTO source_status(source, house, feed, last_success, last_error_at, last_error, last_modified) VALUES ('blizzard','horde','items', ?1, ?2, 'boom', ?3)",
            rusqlite::params![NOW, NOW - 10, NOW - 600],
        )
        .unwrap();
        let h = health(&c, &AppSettings::default(), NOW).unwrap();
        let f = h.feeds.iter().find(|f| f.source == "blizzard" && f.house == House::Horde && f.feed == "items").unwrap();
        assert_eq!((f.last_success, f.last_error.as_deref(), f.stale), (Some(NOW), Some("boom"), false));
    }

    #[test]
    fn timeline_marks_gaps() {
        let mut c = open_in_memory();
        seed(&mut c, House::Alliance, Feed::Items, NOW - 10 * H, &[]);
        seed(&mut c, House::Alliance, Feed::Items, NOW - 6 * H, &[]);
        let h = health(&c, &AppSettings::default(), NOW).unwrap();
        assert_eq!(h.timeline.iter().map(|p| p.gap).collect::<Vec<_>>(), vec![false, true]);
    }

    #[test]
    fn calibration_ranks_by_ahledger_sales() {
        let mut c = open_in_memory();
        seed(&mut c, House::Horde, Feed::Items, NOW - 60, &[li(1, 5, "", 100, 1, TimeLeft::Long), li(2, 6, "", 100, 1, TimeLeft::Long)]);
        ahl(&mut c, "forever.pvp.horde.us", NOW - 60, &[(5, 100, Some(40)), (6, 100, Some(3600))]);
        name_item(&c, 6, "Six");
        let h = health(&c, &AppSettings::default(), NOW).unwrap();
        assert_eq!(h.calibration.iter().map(|x| x.item_id).collect::<Vec<_>>(), vec![6, 5]);
        assert_eq!(h.calibration[0].name, "Six");
        assert!((h.calibration[0].ahl_sales_per_day - 360.0).abs() < 1e-9);
        assert_eq!(h.calibration[0].est_sold_per_day, None);
    }
}
