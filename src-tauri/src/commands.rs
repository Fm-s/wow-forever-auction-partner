//! Tauri commands. Database and Keychain work runs on the blocking pool, never on the main
//! (UI/tray) thread, and only holds the DB lock for the query itself.
use crate::collector::{now_unix, CollectorStatus};
use crate::credentials::{credentials_status, save_credentials as store_credentials, Credentials, CredentialsStatus};
use crate::db::Db;
use crate::domain::House;
use crate::queries::health::{health, Health};
use crate::queries::history::{item_history, HistoryRange, ItemHistory};
use crate::queries::market::{cross_house, item_row, market, CrossHouseRow, MarketRow, MarketView};
use crate::settings::{load_settings, AppSettings};
use crate::AppState;
use rusqlite::Connection;
use tauri::{AppHandle, State};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(err)?
}

async fn with_db<T: Send + 'static>(db: Db, f: impl FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static) -> Result<T, String> {
    blocking(move || {
        let conn = db.lock().map_err(err)?;
        f(&conn).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn get_market(state: State<'_, AppState>, house: House) -> Result<MarketView, String> {
    with_db(state.db.clone(), move |c| market(c, house, &load_settings(c), now_unix())).await
}

#[tauri::command]
pub async fn get_item_row(state: State<'_, AppState>, house: House, item_id: i64, variant_key: String) -> Result<Option<MarketRow>, String> {
    with_db(state.db.clone(), move |c| item_row(c, house, item_id, &variant_key, &load_settings(c), now_unix())).await
}

#[tauri::command]
pub async fn get_item_history(state: State<'_, AppState>, house: House, item_id: i64, variant_key: String, range: HistoryRange) -> Result<ItemHistory, String> {
    with_db(state.db.clone(), move |c| item_history(c, house, item_id, &variant_key, range, now_unix())).await
}

#[tauri::command]
pub async fn get_cross_house(state: State<'_, AppState>, item_id: i64, variant_key: String) -> Result<Vec<CrossHouseRow>, String> {
    with_db(state.db.clone(), move |c| cross_house(c, item_id, &variant_key, &load_settings(c))).await
}

#[tauri::command]
pub async fn get_health(state: State<'_, AppState>) -> Result<Health, String> {
    with_db(state.db.clone(), move |c| health(c, &load_settings(c), now_unix())).await
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    with_db(state.db.clone(), |c| Ok(load_settings(c))).await
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    state.apply_settings(&app, settings)
}

#[tauri::command]
pub async fn get_credentials_status() -> Result<CredentialsStatus, String> {
    blocking(|| Ok(credentials_status())).await
}

#[tauri::command]
pub async fn save_credentials(state: State<'_, AppState>, blizzard_client_id: String, blizzard_client_secret: String, ahledger_key: Option<String>) -> Result<(), String> {
    let c = Credentials {
        blizzard_client_id: blizzard_client_id.trim().to_string(),
        blizzard_client_secret: blizzard_client_secret.trim().to_string(),
        ahledger_key: ahledger_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()),
    };
    if c.blizzard_client_id.is_empty() || c.blizzard_client_secret.is_empty() {
        return Err("Blizzard client ID and secret are required".into());
    }
    blocking(move || store_credentials(&c).map_err(err)).await?;
    state.collector.trigger_now();
    Ok(())
}

#[tauri::command]
pub fn get_collector_status(state: State<AppState>) -> CollectorStatus {
    state.collector.status()
}

#[tauri::command]
pub fn run_collection_now(state: State<AppState>) {
    state.collector.trigger_now();
}
