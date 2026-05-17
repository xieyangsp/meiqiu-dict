use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult};

// Evaluated in the order listed by `AppConfig::capture_methods`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMethod {
    Uia,
    Clipboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
    pub autostart: bool,
    pub tts_voice: Option<String>,
    #[serde(default = "default_true")]
    pub uia_enabled: bool,
    #[serde(default = "default_true")]
    pub clipboard_enabled: bool,
    // Methods run in this order, each gated by its `*_enabled` flag.
    #[serde(default = "default_capture_methods")]
    pub capture_methods: Vec<CaptureMethod>,
}

fn default_true() -> bool {
    true
}

fn default_capture_methods() -> Vec<CaptureMethod> {
    vec![CaptureMethod::Uia, CaptureMethod::Clipboard]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "CommandOrControl+Alt+T".into(),
            autostart: false,
            tts_voice: None,
            uia_enabled: default_true(),
            clipboard_enabled: default_true(),
            capture_methods: default_capture_methods(),
        }
    }
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Config(format!("failed to resolve app_config_dir: {e}")))?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("config.json"))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        let cfg = AppConfig::default();
        save(app, &cfg)?;
        return Ok(cfg);
    }
    let text = fs::read_to_string(&path)?;
    let cfg = serde_json::from_str(&text)
        .map_err(|e| AppError::Config(format!("failed to parse config.json: {e}")))?;
    Ok(cfg)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, cfg: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    let text = serde_json::to_string_pretty(cfg)?;
    fs::write(path, text)?;
    Ok(())
}
