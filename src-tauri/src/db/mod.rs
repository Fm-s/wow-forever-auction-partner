use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("schema.sql")), M::up(include_str!("migration_2.sql")), M::up(include_str!("migration_3.sql")), M::up(include_str!("migration_4.sql")), M::up(include_str!("migration_5.sql"))])
}

fn prepare(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations()
        .to_latest(conn)
        .map_err(|e| rusqlite::Error::InvalidParameterName(format!("migration failed: {e}")))
}

/// Opens (creating if needed) the database file in WAL mode with migrations applied.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    prepare(&mut conn)?;
    Ok(conn)
}

/// In-memory database with migrations applied (tests).
pub fn open_in_memory() -> Connection {
    let mut conn = Connection::open_in_memory().expect("in-memory sqlite");
    prepare(&mut conn).expect("migrations");
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_tables() {
        let c = open_in_memory();
        for t in ["feed_snapshots", "item_aggregates", "volume", "raw_listings", "daily_rollups", "ahledger_rows", "items", "source_status", "settings"] {
            assert!(c.query_row("SELECT 1 FROM sqlite_master WHERE name=?1", [t], |_| Ok(())).is_ok(), "{t}");
        }
    }

    #[test]
    fn migration_2_adds_coverage_and_ahledger_item_index() {
        let c = open_in_memory();
        assert!(c.query_row("SELECT 1 FROM sqlite_master WHERE name='daily_coverage'", [], |_| Ok(())).is_ok());
        assert!(c.query_row("SELECT 1 FROM sqlite_master WHERE name='idx_ahl_item'", [], |_| Ok(())).is_ok());
    }

    #[test]
    fn migration_5_moves_existing_data_to_pvp_us_keys() {
        let mut c = Connection::open_in_memory().unwrap();
        migrations().to_version(&mut c, 4).unwrap();
        c.execute_batch(
            "INSERT INTO feed_snapshots(house, feed, last_modified, fetched_at, listing_count) VALUES ('alliance','items',1,1,0), ('horde','items',1,1,0), ('blackwater','items',1,1,0);
             INSERT INTO daily_rollups(house, item_id, variant_key, day, avg_quantity, est_sold) VALUES ('horde', 1, '', '2026-11-14', 1, 0);
             INSERT INTO daily_coverage(house, feed, day, covered_secs) VALUES ('blackwater', 'items', '2026-11-14', 60);
             INSERT INTO source_status(source, house, feed) VALUES ('ahledger', 'alliance', 'pricetable');",
        )
        .unwrap();
        migrations().to_latest(&mut c).unwrap();
        let col = |sql: &str| -> Vec<String> { c.prepare(sql).unwrap().query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect() };
        assert_eq!(col("SELECT house FROM feed_snapshots ORDER BY id"), vec!["us-110", "us-113", "us-116"]);
        assert_eq!(col("SELECT house FROM daily_rollups"), vec!["us-113"]);
        assert_eq!(col("SELECT house FROM daily_coverage"), vec!["us-116"]);
        assert_eq!(col("SELECT house FROM source_status"), vec!["us-110"]);
    }

    #[test]
    fn open_file_db_is_wal_and_reopenable() {
        let dir = std::env::temp_dir().join(format!("auctioneer-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sqlite3");
        let c = open(&path).unwrap();
        let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        drop(c);
        assert!(open(&path).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }
}
