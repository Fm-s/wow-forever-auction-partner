use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("schema.sql")), M::up(include_str!("migration_2.sql"))])
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
