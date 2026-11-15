//! Tauri commands. Database and Keychain work runs on the blocking pool, never on the main
//! (UI/tray) thread, and only holds the DB lock for the query itself.
use crate::collector::{now_unix, CollectorStatus};
use crate::credentials::{credentials_status, save_credentials as store_credentials, Credentials, CredentialsStatus};
use crate::blizzard::static_data::{cached_tooltip, fetch_tooltip, store_tooltip};
use crate::blizzard::tooltip::ItemTooltip;
use crate::db::Db;
use crate::credentials::load_credentials;
use crate::domain::{Faction, RealmType, Region};
use crate::groups::{self, Group};
use crate::queries::health::{health, Health};
use crate::queries::history::{item_history, HistoryRange, ItemHistory};
use crate::queries::market::{cross_house, item_row, market, CrossHouseRow, MarketRow, MarketView};
use crate::settings::{load_settings, AppSettings};
use crate::AppState;
use rusqlite::Connection;
use serde::Serialize;
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
pub async fn get_market(state: State<'_, AppState>, house: Faction) -> Result<MarketView, String> {
    with_db(state.db.clone(), move |c| {
        let s = load_settings(c);
        market(c, s.realm().house(house), &s, now_unix())
    })
    .await
}

#[tauri::command]
pub async fn get_item_row(state: State<'_, AppState>, house: Faction, item_id: i64, variant_key: String) -> Result<Option<MarketRow>, String> {
    with_db(state.db.clone(), move |c| {
        let s = load_settings(c);
        item_row(c, s.realm().house(house), item_id, &variant_key, &s, now_unix())
    })
    .await
}

#[tauri::command]
pub async fn get_item_history(state: State<'_, AppState>, house: Faction, item_id: i64, variant_key: String, range: HistoryRange) -> Result<ItemHistory, String> {
    with_db(state.db.clone(), move |c| item_history(c, load_settings(c).realm().house(house), item_id, &variant_key, range, now_unix())).await
}

#[tauri::command]
pub async fn get_cross_house(state: State<'_, AppState>, item_id: i64, variant_key: String) -> Result<Vec<CrossHouseRow>, String> {
    with_db(state.db.clone(), move |c| cross_house(c, item_id, &variant_key, &load_settings(c))).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipLookup {
    /// false = not stored locally yet; call `fetch_item_tooltip`.
    pub cached: bool,
    pub tooltip: Option<ItemTooltip>,
}

/// Local lookup only, never touches the network.
#[tauri::command]
pub async fn get_cached_item_tooltip(state: State<'_, AppState>, item_id: i64) -> Result<TooltipLookup, String> {
    let found = with_db(state.db.clone(), move |c| cached_tooltip(c, item_id)).await?;
    Ok(TooltipLookup { cached: found.is_some(), tooltip: found.flatten() })
}

/// Fetches an item's tooltip from Blizzard and stores it, so later lookups are local.
#[tauri::command]
pub async fn fetch_item_tooltip(state: State<'_, AppState>, item_id: i64) -> Result<Option<ItemTooltip>, String> {
    let collector = state.collector.clone();
    let client = blocking(move || collector.lookup_client().ok_or_else(|| "Add Blizzard API credentials in Settings".to_string())).await?;
    let tooltip = fetch_tooltip(&client, item_id).await.map_err(err)?;
    let stored = tooltip.clone();
    with_db(state.db.clone(), move |c| store_tooltip(c, item_id, stored.as_ref(), now_unix())).await?;
    Ok(tooltip)
}

#[tauri::command]
pub async fn get_groups(state: State<'_, AppState>) -> Result<Vec<Group>, String> {
    with_db(state.db.clone(), groups::list_groups).await
}

#[tauri::command]
pub async fn create_group(state: State<'_, AppState>, name: String) -> Result<Group, String> {
    let db = state.db.clone();
    blocking(move || groups::create_group(&*db.lock().map_err(err)?, &name, now_unix()).map_err(err)).await
}

#[tauri::command]
pub async fn rename_group(state: State<'_, AppState>, id: i64, name: String) -> Result<(), String> {
    let db = state.db.clone();
    blocking(move || groups::rename_group(&*db.lock().map_err(err)?, id, &name).map_err(err)).await
}

#[tauri::command]
pub async fn delete_group(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    with_db(state.db.clone(), move |c| groups::delete_group(c, id)).await
}

#[tauri::command]
pub async fn set_group_member(state: State<'_, AppState>, group_id: i64, item_id: i64, variant_key: String, member: bool) -> Result<(), String> {
    with_db(state.db.clone(), move |c| groups::set_member(c, group_id, item_id, &variant_key, member, now_unix())).await
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

fn new_credentials(id: Option<String>, secret: Option<String>, ahledger_key: Option<String>) -> Option<Credentials> {
    let id = id.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())?;
    let secret = secret.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())?;
    Some(Credentials { blizzard_client_id: id, blizzard_client_secret: secret, ahledger_key: ahledger_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) })
}

/// Checks the credentials with Blizzard, then stores them in the Keychain.
async fn check_and_store(state: &AppState, c: Credentials) -> Result<(), String> {
    state.collector.check_credentials(&c).await?;
    blocking(move || store_credentials(&c).map_err(err)).await
}

#[tauri::command]
pub async fn save_credentials(state: State<'_, AppState>, blizzard_client_id: String, blizzard_client_secret: String, ahledger_key: Option<String>) -> Result<(), String> {
    let c = new_credentials(Some(blizzard_client_id), Some(blizzard_client_secret), ahledger_key).ok_or("Blizzard client ID and secret are required")?;
    check_and_store(&state, c).await?;
    state.collector.trigger_now();
    Ok(())
}

/// First-launch setup: realm + credentials. New credentials are checked with Blizzard before they are
/// saved; without them, already-saved credentials are kept (an AHledger key can still be added).
#[tauri::command]
pub async fn complete_setup(
    app: AppHandle,
    state: State<'_, AppState>,
    region: Region,
    realm_type: RealmType,
    blizzard_client_id: Option<String>,
    blizzard_client_secret: Option<String>,
    ahledger_key: Option<String>,
) -> Result<(), String> {
    match new_credentials(blizzard_client_id, blizzard_client_secret, ahledger_key.clone()) {
        Some(c) => check_and_store(&state, c).await?,
        None => {
            let mut saved = blocking(|| Ok(load_credentials())).await?.ok_or("Enter your Blizzard client ID and secret")?;
            if let Some(k) = ahledger_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                saved.ahledger_key = Some(k);
                blocking(move || store_credentials(&saved).map_err(err)).await?;
            }
        }
    }
    let mut s = with_db(state.db.clone(), |c| Ok(load_settings(c))).await?;
    s.region = region;
    s.realm_type = realm_type;
    s.setup_done = true;
    state.apply_settings(&app, s)
}

#[tauri::command]
pub fn get_collector_status(state: State<AppState>) -> CollectorStatus {
    state.collector.status()
}

#[tauri::command]
pub fn run_collection_now(state: State<AppState>) {
    state.collector.trigger_now();
}
