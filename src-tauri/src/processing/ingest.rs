use crate::ahledger::parse::PriceTable;
use crate::domain::{Feed, House, Listing, TimeLeft};
use crate::processing::aggregate::aggregate;
use crate::processing::volume::{diff, is_gap};
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq)]
pub enum IngestOutcome {
    Duplicate,
    Ingested { snapshot_id: i64, gap: bool },
}

/// UTC day string (YYYY-MM-DD) and its start in unix seconds.
pub fn utc_day(t: i64) -> (String, i64) {
    let d = Utc.timestamp_opt(t, 0).single().expect("valid timestamp").date_naive();
    let start = d.and_hms_opt(0, 0, 0).expect("midnight").and_utc().timestamp();
    (d.format("%Y-%m-%d").to_string(), start)
}

fn load_raw(tx: &Transaction, snapshot_id: i64) -> rusqlite::Result<Vec<Listing>> {
    let mut st = tx.prepare_cached("SELECT auction_id, item_id, variant_key, unit_price, quantity, time_left FROM raw_listings WHERE snapshot_id = ?1")?;
    let rows = st.query_map([snapshot_id], |r| {
        let tl: String = r.get(5)?;
        Ok(Listing {
            auction_id: r.get(0)?,
            item_id: r.get(1)?,
            variant_key: r.get(2)?,
            unit_price: r.get(3)?,
            quantity: r.get(4)?,
            time_left: TimeLeft::from_str(&tl).unwrap_or(TimeLeft::Short),
        })
    })?;
    rows.collect()
}

/// Writes one feed snapshot: aggregates, volume vs the previous snapshot, raw listings (last 2 kept)
/// and the daily rollups touched by it. A `last_modified` not newer than the latest stored one is a Duplicate.
pub fn ingest_feed(
    conn: &mut Connection,
    house: House,
    feed: Feed,
    last_modified: i64,
    fetched_at: i64,
    listings: &[Listing],
    cadence_secs: i64,
) -> rusqlite::Result<IngestOutcome> {
    let tx = conn.transaction()?;
    let prev: Option<(i64, i64)> = tx
        .query_row(
            "SELECT id, last_modified FROM feed_snapshots WHERE house = ?1 AND feed = ?2 ORDER BY last_modified DESC LIMIT 1",
            params![house.as_str(), feed.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((_, prev_lm)) = prev {
        if last_modified <= prev_lm {
            return Ok(IngestOutcome::Duplicate);
        }
    }
    let dt = prev.map(|(_, lm)| last_modified - lm);
    let gap = dt.is_some_and(|d| is_gap(d, cadence_secs));
    tx.execute(
        "INSERT INTO feed_snapshots(house, feed, last_modified, fetched_at, listing_count, dt_secs, gap) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![house.as_str(), feed.as_str(), last_modified, fetched_at, listings.len() as i64, dt, gap],
    )?;
    let snapshot_id = tx.last_insert_rowid();

    let mut touched: BTreeSet<(i64, String)> = BTreeSet::new();
    {
        let mut st = tx.prepare_cached(
            "INSERT INTO item_aggregates(snapshot_id, item_id, variant_key, min_price, median, p25, p75, quantity, listings) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for a in aggregate(listings) {
            st.execute(params![snapshot_id, a.item_id, a.variant_key, a.min_price, a.median, a.p25, a.p75, a.quantity, a.listings])?;
            touched.insert((a.item_id, a.variant_key));
        }
    }
    if let (Some((prev_id, _)), Some(dt)) = (prev, dt) {
        let prev_listings = load_raw(&tx, prev_id)?;
        let mut st = tx.prepare_cached("INSERT INTO volume(snapshot_id, item_id, variant_key, est_sold, expired_unknown) VALUES (?1, ?2, ?3, ?4, ?5)")?;
        for v in diff(&prev_listings, listings, dt) {
            st.execute(params![snapshot_id, v.item_id, v.variant_key, v.est_sold, v.expired_unknown])?;
            touched.insert((v.item_id, v.variant_key));
        }
    }
    {
        let mut st = tx.prepare_cached(
            "INSERT INTO raw_listings(snapshot_id, auction_id, item_id, variant_key, unit_price, quantity, time_left) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for l in listings {
            st.execute(params![snapshot_id, l.auction_id, l.item_id, l.variant_key, l.unit_price, l.quantity, l.time_left.as_str()])?;
        }
    }
    tx.execute(
        "DELETE FROM raw_listings WHERE snapshot_id IN (SELECT id FROM feed_snapshots WHERE house = ?1 AND feed = ?2 ORDER BY last_modified DESC LIMIT -1 OFFSET 2)",
        params![house.as_str(), feed.as_str()],
    )?;
    update_rollups(&tx, house, feed, last_modified, &touched)?;
    tx.commit()?;
    Ok(IngestOutcome::Ingested { snapshot_id, gap })
}

fn update_rollups(tx: &Transaction, house: House, feed: Feed, t: i64, touched: &BTreeSet<(i64, String)>) -> rusqlite::Result<()> {
    if touched.is_empty() {
        return Ok(());
    }
    let (day, start) = utc_day(t);
    let end = start + 86400;
    // per (item, variant): (mins, medians, quantities, est_sold)
    let mut acc: BTreeMap<(i64, String), (Vec<i64>, Vec<i64>, Vec<i64>, i64)> = BTreeMap::new();
    {
        let mut st = tx.prepare_cached(
            "SELECT a.item_id, a.variant_key, a.min_price, a.median, a.quantity FROM item_aggregates a JOIN feed_snapshots s ON s.id = a.snapshot_id
             WHERE s.house = ?1 AND s.feed = ?2 AND s.last_modified >= ?3 AND s.last_modified < ?4",
        )?;
        let mut rows = st.query(params![house.as_str(), feed.as_str(), start, end])?;
        while let Some(r) = rows.next()? {
            let key: (i64, String) = (r.get(0)?, r.get(1)?);
            if !touched.contains(&key) {
                continue;
            }
            let e = acc.entry(key).or_default();
            if let Some(m) = r.get::<_, Option<i64>>(2)? {
                e.0.push(m);
            }
            if let Some(m) = r.get::<_, Option<i64>>(3)? {
                e.1.push(m);
            }
            e.2.push(r.get(4)?);
        }
    }
    {
        let mut st = tx.prepare_cached(
            "SELECT v.item_id, v.variant_key, SUM(v.est_sold) FROM volume v JOIN feed_snapshots s ON s.id = v.snapshot_id
             WHERE s.house = ?1 AND s.feed = ?2 AND s.gap = 0 AND s.last_modified >= ?3 AND s.last_modified < ?4 GROUP BY v.item_id, v.variant_key",
        )?;
        let mut rows = st.query(params![house.as_str(), feed.as_str(), start, end])?;
        while let Some(r) = rows.next()? {
            let key: (i64, String) = (r.get(0)?, r.get(1)?);
            if touched.contains(&key) {
                acc.entry(key).or_default().3 = r.get(2)?;
            }
        }
    }
    for key in touched {
        acc.entry(key.clone()).or_default();
    }
    let mut st = tx.prepare_cached(
        "INSERT INTO daily_rollups(house, item_id, variant_key, day, min_price, median, avg_quantity, est_sold) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(house, item_id, variant_key, day) DO UPDATE SET min_price = excluded.min_price, median = excluded.median,
         avg_quantity = excluded.avg_quantity, est_sold = excluded.est_sold",
    )?;
    for ((item_id, variant), (mins, mut medians, qtys, sold)) in acc {
        medians.sort_unstable();
        let median = if medians.is_empty() { None } else { Some(medians[(medians.len() - 1) / 2]) };
        let avg_q = if qtys.is_empty() { None } else { Some((qtys.iter().sum::<i64>() as f64 / qtys.len() as f64).round() as i64) };
        st.execute(params![house.as_str(), item_id, variant, day, mins.iter().min().copied(), median, avg_q, sold])?;
    }
    Ok(())
}

/// Stores an AHledger table once per (market, as_of); returns false when already stored.
pub fn ingest_pricetable(conn: &mut Connection, table: &PriceTable) -> rusqlite::Result<bool> {
    let tx = conn.transaction()?;
    let exists: bool = tx
        .query_row("SELECT 1 FROM ahledger_rows WHERE market = ?1 AND as_of = ?2 LIMIT 1", params![table.market, table.as_of], |_| Ok(()))
        .optional()?
        .is_some();
    if exists {
        return Ok(false);
    }
    {
        let mut st = tx.prepare_cached(
            "INSERT OR REPLACE INTO ahledger_rows(market, as_of, item_id, median, min_buyout, quantity, median_7d, median_30d, low_30d, high_30d, sales_per_day_x10)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;
        for r in &table.rows {
            st.execute(params![table.market, table.as_of, r.item_id, r.median, r.min_buyout, r.quantity, r.median_7d, r.median_30d, r.low_30d, r.high_30d, r.sales_per_day_x10])?;
        }
    }
    tx.commit()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahledger::parse::{AhlRow, PriceTable};
    use crate::db::open_in_memory;
    use crate::domain::{Feed, House, Listing, TimeLeft};
    use crate::processing::aggregate::tests::l;

    const T0: i64 = 1794614400; // 2026-11-14 00:00:00 UTC
    const H: i64 = 3600;

    fn ing(c: &mut Connection, t: i64, ls: &[Listing]) -> IngestOutcome {
        ingest_feed(c, House::Alliance, Feed::Items, t, t + 60, ls, H).unwrap()
    }

    fn count(c: &Connection, sql: &str) -> i64 {
        c.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn long(auction_id: i64, item_id: i64, price: i64, qty: i64) -> Listing {
        let mut x = l(auction_id, item_id, "", Some(price), qty);
        x.time_left = TimeLeft::Long;
        x
    }

    #[test]
    fn first_ingest_has_no_volume() {
        let mut c = open_in_memory();
        assert!(matches!(ing(&mut c, T0, &[long(1, 5, 100, 1)]), IngestOutcome::Ingested { gap: false, .. }));
        assert_eq!(count(&c, "SELECT COUNT(*) FROM volume"), 0);
        assert_eq!(count(&c, "SELECT COUNT(*) FROM item_aggregates"), 1);
        assert_eq!(count(&c, "SELECT COUNT(*) FROM feed_snapshots WHERE dt_secs IS NULL"), 1);
    }

    #[test]
    fn second_ingest_writes_volume() {
        let mut c = open_in_memory();
        ing(&mut c, T0, &[long(1, 5, 100, 3), long(2, 5, 120, 1)]);
        ing(&mut c, T0 + H, &[long(2, 5, 120, 1)]);
        assert_eq!(count(&c, "SELECT est_sold FROM volume"), 3);
        assert_eq!(count(&c, "SELECT dt_secs FROM feed_snapshots ORDER BY last_modified DESC LIMIT 1"), H);
    }

    #[test]
    fn ingest_same_last_modified_is_duplicate() {
        let mut c = open_in_memory();
        ing(&mut c, T0, &[long(1, 5, 100, 3)]);
        assert!(matches!(ing(&mut c, T0, &[]), IngestOutcome::Duplicate));
        assert_eq!(count(&c, "SELECT COUNT(*) FROM feed_snapshots"), 1);
        assert_eq!(count(&c, "SELECT COUNT(*) FROM volume"), 0);
    }

    #[test]
    fn older_last_modified_is_duplicate() {
        let mut c = open_in_memory();
        ing(&mut c, T0 + H, &[long(1, 5, 100, 3)]);
        assert!(matches!(ing(&mut c, T0, &[]), IngestOutcome::Duplicate));
    }

    #[test]
    fn ingest_marks_gap_after_long_sleep() {
        let mut c = open_in_memory();
        ing(&mut c, T0, &[long(1, 5, 100, 3)]);
        assert!(matches!(ing(&mut c, T0 + 3 * H, &[]), IngestOutcome::Ingested { gap: true, .. }));
        assert_eq!(count(&c, "SELECT gap FROM feed_snapshots ORDER BY last_modified DESC LIMIT 1"), 1);
        assert_eq!(count(&c, "SELECT COUNT(*) FROM volume"), 1);
        assert_eq!(count(&c, "SELECT est_sold FROM daily_rollups WHERE item_id = 5"), 0);
    }

    #[test]
    fn raw_listings_keep_last_two_snapshots() {
        let mut c = open_in_memory();
        for i in 0..3 {
            ing(&mut c, T0 + i * H, &[long(10 + i, 5, 100, 1)]);
        }
        assert_eq!(count(&c, "SELECT COUNT(DISTINCT snapshot_id) FROM raw_listings"), 2);
    }

    #[test]
    fn feeds_are_independent() {
        let mut c = open_in_memory();
        let a = ingest_feed(&mut c, House::Alliance, Feed::Items, T0, T0, &[long(1, 5, 1, 1)], H).unwrap();
        let b = ingest_feed(&mut c, House::Alliance, Feed::Commodities, T0, T0, &[long(2, 6, 1, 1)], H).unwrap();
        assert!(matches!(a, IngestOutcome::Ingested { .. }) && matches!(b, IngestOutcome::Ingested { .. }));
        assert_eq!(count(&c, "SELECT COUNT(*) FROM feed_snapshots WHERE dt_secs IS NULL"), 2);
    }

    #[test]
    fn daily_rollup_median_of_medians() {
        let mut c = open_in_memory();
        ing(&mut c, T0, &[long(1, 5, 100, 1)]);
        ing(&mut c, T0 + H, &[long(1, 5, 100, 1), long(2, 5, 300, 5)]);
        ing(&mut c, T0 + 2 * H, &[long(3, 5, 200, 1)]);
        let (min, med, avg_q, sold): (i64, i64, i64, i64) = c
            .query_row("SELECT min_price, median, avg_quantity, est_sold FROM daily_rollups WHERE item_id = 5 AND day = '2026-11-14'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap();
        // snapshot medians 100, 300, 200 -> 200; mins 100, 100, 200 -> 100; quantities 1, 6, 1 -> avg 3 (rounded)
        // sold: snapshot 3 vanished auctions 1 (qty 1) and 2 (qty 5), both LONG -> 6
        assert_eq!((min, med, avg_q, sold), (100, 200, 3, 6));
    }

    #[test]
    fn pricetable_ingest_dedupes_as_of() {
        let mut c = open_in_memory();
        let t = PriceTable {
            market: "forever.pvp.alliance.us".into(),
            as_of: T0,
            rows: vec![AhlRow { item_id: 5, median: Some(20), min_buyout: Some(18), quantity: Some(3), median_7d: None, median_30d: None, low_30d: None, high_30d: None, sales_per_day_x10: Some(40) }],
        };
        assert!(ingest_pricetable(&mut c, &t).unwrap());
        assert!(!ingest_pricetable(&mut c, &t).unwrap());
        assert_eq!(count(&c, "SELECT COUNT(*) FROM ahledger_rows"), 1);
    }
}

#[cfg(test)]
mod bench {
    /// Manual timing check against a full snapshot: AUCTIONEER_BENCH_FILE=/path/ah_110.json cargo test --release bench -- --ignored --nocapture
    #[test]
    #[ignore]
    fn ingest_full_snapshot_timing() {
        let path = std::env::var("AUCTIONEER_BENCH_FILE").expect("AUCTIONEER_BENCH_FILE");
        let body = std::fs::read(path).unwrap();
        let listings = crate::blizzard::parse::parse_items_feed(&body).unwrap();
        let mut c = crate::db::open_in_memory();
        let t = std::time::Instant::now();
        super::ingest_feed(&mut c, crate::domain::House::Alliance, crate::domain::Feed::Items, 1_000_000, 0, &listings, 3600).unwrap();
        let t1 = t.elapsed();
        let half = &listings[..listings.len() / 2];
        super::ingest_feed(&mut c, crate::domain::House::Alliance, crate::domain::Feed::Items, 1_003_600, 0, half, 3600).unwrap();
        println!("listings={} first={:?} second(with diff)={:?}", listings.len(), t1, t.elapsed() - t1);
    }
}
