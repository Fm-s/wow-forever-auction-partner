use crate::collector::CollectorStatus;
use crate::credentials::{credentials_status, save_credentials as store_credentials, Credentials, CredentialsStatus};
use crate::domain::House;
use crate::queries::health::{health, Health};
use crate::queries::history::{item_history, HistoryRange, ItemHistory};
use crate::queries::market::{cross_house, market, CrossHouseRow, MarketView};
use crate::settings::{load_settings, AppSettings};
use crate::collector::now_unix;
use crate::AppState;
use tauri::{AppHandle, State};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_market(state: State<AppState>, house: House) -> Result<MarketView, String> {
    let conn = state.db.lock().map_err(err)?;
    let settings = load_settings(&conn);
    market(&conn, house, &settings, now_unix()).map_err(err)
}

#[tauri::command]
pub fn get_item_history(state: State<AppState>, house: House, item_id: i64, variant_key: String, range: HistoryRange) -> Result<ItemHistory, String> {
    let conn = state.db.lock().map_err(err)?;
    item_history(&conn, house, item_id, &variant_key, range, now_unix()).map_err(err)
}

#[tauri::command]
pub fn get_cross_house(state: State<AppState>, item_id: i64, variant_key: String) -> Result<Vec<CrossHouseRow>, String> {
    let conn = state.db.lock().map_err(err)?;
    let settings = load_settings(&conn);
    cross_house(&conn, item_id, &variant_key, &settings).map_err(err)
}

#[tauri::command]
pub fn get_health(state: State<AppState>) -> Result<Health, String> {
    let conn = state.db.lock().map_err(err)?;
    let settings = load_settings(&conn);
    health(&conn, &settings, now_unix()).map_err(err)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    Ok(load_settings(&*state.db.lock().map_err(err)?))
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: AppSettings) -> Result<(), String> {
    state.apply_settings(&app, settings).map_err(err)
}

#[tauri::command]
pub fn get_credentials_status() -> CredentialsStatus {
    credentials_status()
}

#[tauri::command]
pub fn save_credentials(state: State<AppState>, blizzard_client_id: String, blizzard_client_secret: String, ahledger_key: Option<String>) -> Result<(), String> {
    let c = Credentials {
        blizzard_client_id: blizzard_client_id.trim().to_string(),
        blizzard_client_secret: blizzard_client_secret.trim().to_string(),
        ahledger_key: ahledger_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()),
    };
    if c.blizzard_client_id.is_empty() || c.blizzard_client_secret.is_empty() {
        return Err("Blizzard client ID and secret are required".into());
    }
    store_credentials(&c).map_err(err)?;
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
