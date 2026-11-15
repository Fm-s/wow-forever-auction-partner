use crate::domain::{Faction, House, Realm, RealmType, Region};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Replaced by the measured cadence once tools/cadence_report.py has 24h of data.
pub const DEFAULT_CADENCE_SECS: i64 = 3600;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub cadence_secs: i64,
    pub cut_bps_alliance: i64,
    pub cut_bps_horde: i64,
    pub cut_bps_blackwater: i64,
    pub launch_at_login: bool,
    pub collection_paused: bool,
    pub region: Region,
    pub realm_type: RealmType,
    /// First-launch setup (realm + credentials) finished; nothing is collected before.
    pub setup_done: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            cadence_secs: DEFAULT_CADENCE_SECS,
            cut_bps_alliance: 500,
            cut_bps_horde: 500,
            cut_bps_blackwater: 1500,
            launch_at_login: true,
            collection_paused: false,
            region: Region::Us,
            realm_type: RealmType::Pvp,
            setup_done: false,
        }
    }
}

impl AppSettings {
    pub fn cut_bps(&self, h: House) -> i64 {
        match h.faction {
            Faction::Alliance => self.cut_bps_alliance,
            Faction::Horde => self.cut_bps_horde,
            Faction::Blackwater => self.cut_bps_blackwater,
        }
    }

    /// The realm being tracked.
    pub fn realm(&self) -> Realm {
        Realm { region: self.region, kind: self.realm_type }
    }
}

pub fn get_kv(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional().ok().flatten()
}

pub fn set_kv(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

/// Settings stored as one JSON value under key `app`; missing or unreadable values fall back to defaults.
pub fn load_settings(conn: &Connection) -> AppSettings {
    get_kv(conn, "app").and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

pub fn save_settings(conn: &Connection, s: &AppSettings) -> rusqlite::Result<()> {
    set_kv(conn, "app", &serde_json::to_string(s).expect("settings serialize"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn settings_defaults() {
        let s = load_settings(&open_in_memory());
        assert_eq!((s.cut_bps_alliance, s.cut_bps_horde, s.cut_bps_blackwater), (500, 500, 1500));
        assert_eq!(s.cadence_secs, DEFAULT_CADENCE_SECS);
        assert!(s.launch_at_login);
        assert!(!s.collection_paused);
    }

    #[test]
    fn settings_roundtrip() {
        let c = open_in_memory();
        let mut s = load_settings(&c);
        s.cadence_secs = 1800;
        s.cut_bps_blackwater = 1200;
        s.collection_paused = true;
        save_settings(&c, &s).unwrap();
        assert_eq!(load_settings(&c), s);
    }

    #[test]
    fn settings_missing_fields_keep_stored_values() {
        let c = open_in_memory();
        set_kv(&c, "app", r#"{"cadenceSecs":1800,"cutBpsHorde":700}"#).unwrap();
        let s = load_settings(&c);
        assert_eq!((s.cadence_secs, s.cut_bps_horde, s.cut_bps_blackwater), (1800, 700, 1500));
    }

    #[test]
    fn cut_bps_per_house() {
        let s = load_settings(&open_in_memory());
        assert_eq!(s.cut_bps(House::Alliance), 500);
        assert_eq!(s.cut_bps(House::Blackwater), 1500);
    }

    #[test]
    fn kv_roundtrip() {
        let c = open_in_memory();
        assert_eq!(get_kv(&c, "static_ns"), None);
        set_kv(&c, "static_ns", "static-1.60.1_70399-forever-us").unwrap();
        assert_eq!(get_kv(&c, "static_ns").as_deref(), Some("static-1.60.1_70399-forever-us"));
    }
}
