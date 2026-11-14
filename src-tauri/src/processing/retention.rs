use rusqlite::{params, Connection};

pub const RETENTION_SECS: i64 = 30 * 86400;

/// Drops per-snapshot aggregates, volume and AHledger rows older than 30 days.
/// Daily rollups and the snapshot timeline are kept.
pub fn prune(conn: &mut Connection, now: i64) -> rusqlite::Result<()> {
    let cutoff = now - RETENTION_SECS;
    let tx = conn.transaction()?;
    let old = "SELECT id FROM feed_snapshots WHERE last_modified < ?1";
    tx.execute(&format!("DELETE FROM item_aggregates WHERE snapshot_id IN ({old})"), params![cutoff])?;
    tx.execute(&format!("DELETE FROM volume WHERE snapshot_id IN ({old})"), params![cutoff])?;
    tx.execute("DELETE FROM ahledger_rows WHERE as_of < ?1", params![cutoff])?;
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::{Feed, House, TimeLeft};
    use crate::processing::aggregate::tests::l;
    use crate::processing::ingest::ingest_feed;

    #[test]
    fn prune_drops_old_aggregates_keeps_rollups() {
        let mut c = open_in_memory();
        let now = 1794614400;
        let old = now - 31 * 86400;
        let mut x = l(1, 5, "", Some(100), 2);
        x.time_left = TimeLeft::Long;
        ingest_feed(&mut c, House::Horde, Feed::Items, old, old, &[x], 3600).unwrap();
        ingest_feed(&mut c, House::Horde, Feed::Items, old + 3600, old + 3600, &[], 3600).unwrap();
        ingest_feed(&mut c, House::Horde, Feed::Items, now, now, &[l(2, 6, "", Some(9), 1)], 3600).unwrap();
        prune(&mut c, now).unwrap();
        let q = |sql: &str| -> i64 { c.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(q("SELECT COUNT(*) FROM item_aggregates WHERE item_id = 5"), 0);
        assert_eq!(q("SELECT COUNT(*) FROM volume"), 0);
        assert_eq!(q("SELECT COUNT(*) FROM item_aggregates WHERE item_id = 6"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM daily_rollups WHERE item_id = 5"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM feed_snapshots"), 3);
    }

    #[test]
    fn prune_drops_old_ahledger_rows() {
        let mut c = open_in_memory();
        let now = 1794614400;
        c.execute("INSERT INTO ahledger_rows(market, as_of, item_id) VALUES ('m', ?1, 1), ('m', ?2, 1)", [now - 31 * 86400, now]).unwrap();
        prune(&mut c, now).unwrap();
        assert_eq!(c.query_row("SELECT COUNT(*) FROM ahledger_rows", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }
}
