pub mod ahledger;
pub mod blizzard;
pub mod collector;
pub mod commands;
pub mod credentials;
pub mod db;
pub mod domain;
pub mod processing;
pub mod queries;
pub mod settings;
pub mod tray;

use collector::{Collector, CollectorEvent, Endpoints};
use db::Db;
use settings::{load_settings, save_settings, AppSettings};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub const EVENT_STATUS: &str = "collector://status";
pub const EVENT_SNAPSHOT: &str = "collector://snapshot";
const HIDDEN_ARG: &str = "--hidden";

pub struct AppState {
    pub db: Db,
    pub collector: Arc<Collector>,
}

impl AppState {
    /// Persists settings and applies their side effects (launch at login, pause, tray label).
    pub fn apply_settings(&self, app: &AppHandle, s: AppSettings) -> Result<(), String> {
        {
            let conn = self.db.lock().map_err(|e| e.to_string())?;
            save_settings(&conn, &s).map_err(|e| e.to_string())?;
        }
        if !cfg!(debug_assertions) {
            let autolaunch = app.autolaunch();
            let res = if s.launch_at_login { autolaunch.enable() } else { autolaunch.disable() };
            if let Err(e) = res {
                eprintln!("autostart: {e}");
            }
        }
        self.collector.set_paused(s.collection_paused);
        if let Some(items) = app.try_state::<tray::TrayItems>() {
            let _ = items.pause.set_text(tray::pause_label(s.collection_paused));
        }
        self.collector.trigger_now();
        Ok(())
    }

    pub fn toggle_paused(&self, app: &AppHandle) -> Result<(), String> {
        let mut s = {
            let conn = self.db.lock().map_err(|e| e.to_string())?;
            load_settings(&conn)
        };
        s.collection_paused = !s.collection_paused;
        self.apply_settings(app, s)
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let db: Db = Arc::new(Mutex::new(db::open(&dir.join("auctioneer.sqlite3"))?));
    let settings = load_settings(&db.lock().expect("db"));

    let handle = app.handle().clone();
    let emit = Box::new(move |e: CollectorEvent| match e {
        CollectorEvent::Status(s) => {
            if let Some(items) = handle.try_state::<tray::TrayItems>() {
                let _ = items.status.set_text(tray::status_text(&s, tray::local_hhmm));
            }
            let _ = handle.emit(EVENT_STATUS, s);
        }
        CollectorEvent::Snapshot(s) => {
            let _ = handle.emit(EVENT_SNAPSHOT, s);
        }
    });
    let collector = Collector::new(db.clone(), Endpoints::production(), emit);
    app.manage(AppState { db: db.clone(), collector: collector.clone() });
    let items = tray::build(app.handle(), settings.collection_paused)?;
    app.manage(items);

    // Never register a debug build as the login item: it would point at target/debug.
    if !cfg!(debug_assertions)
        && settings.launch_at_login
        && !app.autolaunch().is_enabled().unwrap_or(false)
        && let Err(e) = app.autolaunch().enable() {
            eprintln!("autostart: {e}");
        }
    if std::env::args().any(|a| a == HIDDEN_ARG)
        && let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    collector.spawn_loop();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // One collector per database: a second launch just shows the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main_window(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_ARG])))
        .setup(setup)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_market,
            commands::get_item_row,
            commands::get_item_history,
            commands::get_cross_house,
            commands::get_cached_item_tooltip,
            commands::fetch_item_tooltip,
            commands::get_health,
            commands::get_settings,
            commands::save_settings,
            commands::get_credentials_status,
            commands::save_credentials,
            commands::get_collector_status,
            commands::run_collection_now,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Clicking the Dock icon while the window is hidden. macOS-only event.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                tray::show_main_window(_app);
            }
        });
}
