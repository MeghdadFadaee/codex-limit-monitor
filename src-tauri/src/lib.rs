pub mod codex_rpc;
pub mod local_usage;
pub mod models;
#[cfg(not(test))]
pub mod monitor;
pub mod settings;
#[cfg(not(test))]
pub mod tray;

#[cfg(not(test))]
use monitor::MonitorService;
#[cfg(not(test))]
use settings::{AppSettings, BadgePosition};
#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};

#[cfg(not(test))]
#[derive(Clone)]
pub struct AppState {
    monitor: Arc<MonitorService>,
}

#[cfg(not(test))]
#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<models::MonitorSnapshot, String> {
    state.monitor.snapshot_or_refresh(None).await
}

#[cfg(not(test))]
#[tauri::command]
async fn refresh_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<models::MonitorSnapshot, String> {
    state.monitor.refresh(Some(app)).await
}

#[cfg(not(test))]
#[tauri::command]
async fn get_usage_history(
    range: String,
    state: State<'_, AppState>,
) -> Result<models::UsageHistory, String> {
    let settings = state.monitor.settings().await;
    local_usage::read_usage_history(&settings, &range).map_err(|err| err.to_string())
}

#[cfg(not(test))]
#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.monitor.settings().await)
}

#[cfg(not(test))]
#[tauri::command]
async fn update_settings(
    app: AppHandle,
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    settings.save().map_err(|err| err.to_string())?;
    state.monitor.replace_settings(settings.clone()).await;
    apply_badge_visibility(&app, settings.badge_visible)?;
    apply_badge_position(&app, &settings.badge_position)?;
    let snapshot = state.monitor.refresh(Some(app.clone())).await?;
    let _ = app.emit("settings_updated", &settings);
    let _ = app.emit("rate_limit_updated", &snapshot);
    Ok(settings)
}

#[cfg(not(test))]
#[tauri::command]
async fn toggle_badge(
    app: AppHandle,
    visible: bool,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let mut settings = state.monitor.settings().await;
    settings.badge_visible = visible;
    settings.save().map_err(|err| err.to_string())?;
    state.monitor.replace_settings(settings.clone()).await;
    apply_badge_visibility(&app, visible)?;
    let _ = app.emit("settings_updated", &settings);
    Ok(settings)
}

#[cfg(not(test))]
#[tauri::command]
async fn set_badge_position(
    app: AppHandle,
    position: BadgePosition,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let mut settings = state.monitor.settings().await;
    settings.badge_position = position;
    settings.save().map_err(|err| err.to_string())?;
    state.monitor.replace_settings(settings.clone()).await;
    apply_badge_position(&app, &settings.badge_position)?;
    let _ = app.emit("settings_updated", &settings);
    Ok(settings)
}

#[cfg(not(test))]
#[tauri::command]
async fn move_badge(app: AppHandle, position: BadgePosition) -> Result<(), String> {
    apply_badge_position(&app, &position)
}

#[cfg(not(test))]
#[tauri::command]
async fn pause_notifications(
    until: Option<i64>,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let mut settings = state.monitor.settings().await;
    settings.notifications_muted_until = until;
    settings.save().map_err(|err| err.to_string())?;
    state.monitor.replace_settings(settings.clone()).await;
    Ok(settings)
}

#[cfg(not(test))]
#[tauri::command]
async fn open_dashboard(app: AppHandle) -> Result<(), String> {
    show_dashboard(&app)
}

#[cfg(not(test))]
pub(crate) fn show_dashboard(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[cfg(not(test))]
pub(crate) fn apply_badge_visibility(app: &AppHandle, visible: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("badge") {
        if visible {
            window.show().map_err(|err| err.to_string())?;
        } else {
            window.hide().map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn apply_badge_position(app: &AppHandle, position: &BadgePosition) -> Result<(), String> {
    let Some(window) = app.get_webview_window("badge") else {
        return Ok(());
    };

    window
        .set_position(PhysicalPosition::new(position.x, position.y))
        .map_err(|err| err.to_string())
}

#[cfg(not(test))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let settings = AppSettings::load().unwrap_or_default();
            let monitor = Arc::new(MonitorService::new(settings.clone()));
            app.manage(AppState {
                monitor: monitor.clone(),
            });

            tray::setup_tray(app.handle())?;
            apply_badge_visibility(app.handle(), settings.badge_visible)?;
            apply_badge_position(app.handle(), &settings.badge_position)?;
            if let Some(window) = app.get_webview_window("badge") {
                let _ = window.set_shadow(false);
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                monitor.run(app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            refresh_now,
            get_usage_history,
            get_settings,
            update_settings,
            toggle_badge,
            set_badge_position,
            move_badge,
            pause_notifications,
            open_dashboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Limit Monitor");
}
