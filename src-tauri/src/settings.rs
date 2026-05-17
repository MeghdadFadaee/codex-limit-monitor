use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BadgePosition {
    pub x: i32,
    pub y: i32,
}

impl Default for BadgePosition {
    fn default() -> Self {
        Self { x: 1530, y: 18 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationThresholds {
    pub warning: f64,
    pub critical: f64,
    pub exhausted: f64,
}

impl Default for NotificationThresholds {
    fn default() -> Self {
        Self {
            warning: 70.0,
            critical: 90.0,
            exhausted: 100.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub codex_executable: Option<String>,
    pub codex_home: String,
    pub refresh_interval_secs: u64,
    pub badge_visible: bool,
    pub badge_position: BadgePosition,
    pub privacy_mode: bool,
    pub thresholds: NotificationThresholds,
    pub notifications_muted_until: Option<i64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            codex_executable: None,
            codex_home: default_codex_home().to_string_lossy().to_string(),
            refresh_interval_secs: 60,
            badge_visible: true,
            badge_position: BadgePosition::default(),
            privacy_mode: false,
            thresholds: NotificationThresholds::default(),
            notifications_muted_until: None,
        }
    }
}

impl AppSettings {
    pub fn load() -> io::Result<Self> {
        let path = settings_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let body = fs::read_to_string(path)?;
        let mut settings = serde_json::from_str::<Self>(&body).unwrap_or_default();
        settings.normalize();
        Ok(settings)
    }

    pub fn save(&self) -> io::Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, body)
    }

    pub fn codex_home_path(&self) -> PathBuf {
        PathBuf::from(&self.codex_home)
    }

    pub fn resolve_codex_executable(&self) -> String {
        if let Some(path) = self
            .codex_executable
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            return path.clone();
        }

        for candidate in codex_executable_candidates() {
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }

        "codex".to_string()
    }

    fn normalize(&mut self) {
        if self.refresh_interval_secs < 15 {
            self.refresh_interval_secs = 15;
        }
        if self.codex_home.trim().is_empty() {
            self.codex_home = default_codex_home().to_string_lossy().to_string();
        }
    }
}

pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("CodexLimitMonitor")
        .join("settings.json")
}

fn default_codex_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
}

fn codex_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(app_data) = std::env::var_os("APPDATA") {
        candidates.push(Path::new(&app_data).join("npm").join("codex.cmd"));
    }

    candidates.push(
        Path::new("C:\\Program Files")
            .join("nodejs")
            .join("codex.cmd"),
    );
    candidates.push(Path::new("C:\\Program Files").join("nodejs").join("codex"));

    if let Some(home) = dirs::home_dir() {
        candidates.push(
            home.join(".config")
                .join("herd")
                .join("bin")
                .join("nvm")
                .join("v25.2.1")
                .join("node_modules")
                .join(".bin")
                .join("codex.cmd"),
        );
        candidates.push(
            home.join(".config")
                .join("herd")
                .join("bin")
                .join("nvm")
                .join("v25.2.1")
                .join("node_modules")
                .join("@openai")
                .join("codex")
                .join("bin")
                .join("codex.js"),
        );
    }

    candidates
}
