use crate::collector::CollectorStatus;
use crate::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

/// Handles to tray menu items whose text changes at runtime.
pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub pause: MenuItem<Wry>,
}

/// One-line tray status; `fmt_time` renders a unix timestamp (local HH:MM in the app).
pub fn status_text(s: &CollectorStatus, fmt_time: impl Fn(i64) -> String) -> String {
    let updated = s.last_new_snapshot_at.map(&fmt_time);
    match (s.state.as_str(), updated) {
        ("running", Some(t)) => format!("Updated {t}"),
        ("running", None) => "Waiting for first snapshot".into(),
        ("paused", Some(t)) => format!("Paused · updated {t}"),
        ("paused", None) => "Paused".into(),
        ("starting", _) => "Starting…".into(),
        (_, _) => s.message.clone().unwrap_or_else(|| s.state.clone()),
    }
}

pub fn local_hhmm(t: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(t, 0).single().map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn pause_label(paused: bool) -> &'static str {
    if paused { "Resume collection" } else { "Pause collection" }
}

/// Builds the menu-bar icon: status line, Open, Pause/Resume, Quit.
pub fn build(app: &AppHandle, paused: bool) -> tauri::Result<TrayItems> {
    let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Auctioneer", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", pause_label(paused), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status, &sep, &open, &pause, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Auctioneer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "pause" => {
                let state = app.state::<AppState>();
                let _ = state.toggle_paused(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(TrayItems { status, pause })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::CollectorStatus;

    fn st(state: &str, snap: Option<i64>, msg: Option<&str>) -> CollectorStatus {
        CollectorStatus { state: state.into(), last_cycle_at: None, last_new_snapshot_at: snap, message: msg.map(Into::into) }
    }

    #[test]
    fn status_text_variants() {
        let fmt = |t: i64| format!("T{t}");
        assert_eq!(status_text(&st("running", Some(5), None), fmt), "Updated T5");
        assert_eq!(status_text(&st("running", None, None), fmt), "Waiting for first snapshot");
        assert_eq!(status_text(&st("paused", Some(5), None), fmt), "Paused · updated T5");
        assert_eq!(status_text(&st("error", Some(5), Some("Blizzard credentials rejected")), fmt), "Blizzard credentials rejected");
        assert_eq!(status_text(&st("no_credentials", None, Some("Add Blizzard API credentials in Settings")), fmt), "Add Blizzard API credentials in Settings");
        assert_eq!(status_text(&st("starting", None, None), fmt), "Starting…");
    }
}
