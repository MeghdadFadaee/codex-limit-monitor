use crate::{models::MonitorSnapshot, show_dashboard};
use chrono::{Local, TimeZone};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

const TRAY_ID: &str = "codex-limit-monitor";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh Now", true, None::<&str>)?;
    let badge = MenuItem::with_id(app, "badge", "Toggle Badge", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &refresh, &badge, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app_tray_icon())
        .tooltip("Codex Limit Monitor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                let _ = show_dashboard(app);
            }
            "refresh" => {
                let app = app.clone();
                let monitor = app
                    .try_state::<crate::AppState>()
                    .map(|state| state.monitor.clone());
                tauri::async_runtime::spawn(async move {
                    if let Some(monitor) = monitor {
                        let _ = monitor.refresh(Some(app.clone())).await;
                    }
                });
            }
            "badge" => {
                let app = app.clone();
                let monitor = app
                    .try_state::<crate::AppState>()
                    .map(|state| state.monitor.clone());
                tauri::async_runtime::spawn(async move {
                    if let Some(monitor) = monitor {
                        let mut settings = monitor.settings().await;
                        settings.badge_visible = !settings.badge_visible;
                        if settings.save().is_ok() {
                            monitor.replace_settings(settings.clone()).await;
                            let _ = crate::apply_badge_visibility(&app, settings.badge_visible);
                            let _ = app.emit("settings_updated", &settings);
                        }
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_dashboard(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray(app: &AppHandle, snapshot: &MonitorSnapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let _ = tray.set_tooltip(Some(&tooltip(snapshot)));
}

fn app_tray_icon() -> Image<'static> {
    Image::from_bytes(include_bytes!("../icons/icon.png")).expect("valid embedded tray icon")
}

fn tooltip(snapshot: &MonitorSnapshot) -> String {
    if let Some(bucket) = snapshot.buckets.first() {
        let reset = bucket
            .resets_at
            .map(format_reset_datetime)
            .map(|value| format!(", resets {value}"))
            .unwrap_or_default();
        let weekly_reset = bucket
            .secondary_resets_at
            .map(format_reset_datetime)
            .map(|value| format!(", weekly resets {value}"))
            .unwrap_or_default();
        let weekly = bucket
            .secondary_used_percent
            .map(|percent| format!("{percent:.0}% weekly"))
            .unwrap_or_else(|| "weekly unavailable".to_string());
        format!(
            "Codex: {:.0}% current, {}{}{}",
            bucket.used_percent, weekly, reset, weekly_reset
        )
    } else {
        format!(
            "Codex usage: {} tokens today ({})",
            snapshot.usage_summary.today_tokens, snapshot.live_status.state
        )
    }
}

fn format_reset_datetime(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|date| date.format("%b %-d, %I:%M %p").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
