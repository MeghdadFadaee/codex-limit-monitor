use crate::{
    codex_rpc, local_usage,
    models::{LiveStatus, MonitorSnapshot, RateLimitBucket},
    settings::AppSettings,
    tray,
};
use chrono::Utc;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{Mutex, RwLock};

#[derive(Default)]
struct NotificationMemory {
    seen: HashMap<String, String>,
}

pub struct MonitorService {
    settings: RwLock<AppSettings>,
    snapshot: RwLock<Option<MonitorSnapshot>>,
    notifications: Mutex<NotificationMemory>,
}

impl MonitorService {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: RwLock::new(settings),
            snapshot: RwLock::new(None),
            notifications: Mutex::new(NotificationMemory::default()),
        }
    }

    pub async fn settings(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    pub async fn replace_settings(&self, settings: AppSettings) {
        *self.settings.write().await = settings;
    }

    pub async fn snapshot_or_refresh(
        &self,
        app: Option<AppHandle>,
    ) -> Result<MonitorSnapshot, String> {
        if let Some(snapshot) = self.snapshot.read().await.clone() {
            return Ok(snapshot);
        }
        self.refresh(app).await
    }

    pub async fn refresh(&self, app: Option<AppHandle>) -> Result<MonitorSnapshot, String> {
        let settings = self.settings().await;
        if let Some(app) = app.as_ref() {
            let _ = app.emit("refresh_started", ());
        }
        let usage_summary = local_usage::read_usage_summary(&settings);
        let live = codex_rpc::fetch_rate_limits(&settings).await;
        let fetched_at = Utc::now().timestamp();

        let (source, live_status, buckets, message) = match live {
            Ok(buckets) => (
                "codex-app-server".to_string(),
                LiveStatus::online(),
                buckets,
                None,
            ),
            Err(error) if usage_summary.has_data() => (
                "local-sqlite-fallback".to_string(),
                LiveStatus::fallback(error.to_string()),
                Vec::new(),
                Some("Live Codex limits are unavailable; showing local usage history.".to_string()),
            ),
            Err(error) => (
                "offline".to_string(),
                LiveStatus::offline(error.to_string()),
                Vec::new(),
                Some("No live Codex limits or local usage data could be read.".to_string()),
            ),
        };

        let current_thread = usage_summary.current_thread.clone();
        let snapshot = MonitorSnapshot {
            source,
            fetched_at,
            live_status,
            buckets,
            usage_summary,
            current_thread,
            message,
        };

        *self.snapshot.write().await = Some(snapshot.clone());

        if let Some(app) = app {
            let _ = app.emit("rate_limit_updated", &snapshot);
            let _ = app.emit("usage_updated", &snapshot.usage_summary);
            tray::update_tray(&app, &snapshot);
            self.maybe_notify(&app, &settings, &snapshot.buckets).await;
        }

        Ok(snapshot)
    }

    pub async fn run(self: Arc<Self>, app: AppHandle) {
        loop {
            let settings = self.settings().await;
            let interval = settings.refresh_interval_secs.max(15);
            let _ = self.refresh(Some(app.clone())).await;
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    }

    async fn maybe_notify(
        &self,
        app: &AppHandle,
        settings: &AppSettings,
        buckets: &[RateLimitBucket],
    ) {
        if settings
            .notifications_muted_until
            .is_some_and(|until| until > Utc::now().timestamp())
        {
            return;
        }

        let mut memory = self.notifications.lock().await;
        for bucket in buckets {
            let Some(level) = threshold_level(settings, bucket.used_percent) else {
                continue;
            };
            let reset = bucket.resets_at.unwrap_or_default();
            let key = format!("{}:{reset}", bucket.limit_id);
            if memory.seen.get(&key).is_some_and(|seen| seen == level) {
                continue;
            }

            memory.seen.insert(key, level.to_string());
            let title = match level {
                "exhausted" => "Codex limit reached",
                "critical" => "Codex limit nearly reached",
                _ => "Codex limit warning",
            };
            let body = format!(
                "{} is at {:.0}% and resets {}.",
                bucket
                    .limit_name
                    .as_deref()
                    .unwrap_or(bucket.limit_id.as_str()),
                bucket.used_percent,
                bucket
                    .resets_at
                    .map(format_reset)
                    .unwrap_or_else(|| "soon".to_string())
            );

            let _ = app.notification().builder().title(title).body(body).show();
        }
    }
}

fn threshold_level(settings: &AppSettings, percent: f64) -> Option<&'static str> {
    if percent >= settings.thresholds.exhausted {
        Some("exhausted")
    } else if percent >= settings.thresholds.critical {
        Some("critical")
    } else if percent >= settings.thresholds.warning {
        Some("warning")
    } else {
        None
    }
}

fn format_reset(timestamp: i64) -> String {
    let now = Utc::now().timestamp();
    if timestamp <= now {
        return "now".to_string();
    }

    let seconds = timestamp - now;
    if seconds < 60 {
        format!("in {seconds}s")
    } else {
        format!("in {}m", seconds / 60)
    }
}
