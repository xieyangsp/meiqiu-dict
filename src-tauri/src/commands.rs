use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State,
    async_runtime,
};
use tauri_plugin_autostart::ManagerExt;

use crate::config::{self, AppConfig};
use crate::dict::{self, DictEntry};
use crate::error::{AppError, AppResult};
use crate::events;
use crate::hotkey;
use crate::state::AppState;
use crate::window::{self, FLOATER_LABEL, POPUP_LABEL};
use crate::tts;

const POPUP_DROP: i32 = 30;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsAccent {
    EnUs,
    EnGb,
}

#[derive(Clone, Serialize)]
struct LookupPayload<'a> {
    text: &'a str,
}

#[tauri::command]
pub async fn dict_lookup(
    word: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Option<DictEntry>> {
    let pool = state
        .dict()
        .ok_or_else(|| AppError::Dict("dictionary not initialized".into()))?;
    async_runtime::spawn_blocking(move || dict::lookup(&pool, &word))
        .await
        .map_err(|e| AppError::Dict(format!("join: {e}")))?
}

/// Hide the floater, show the popup at the captured cursor, request the lookup.
#[tauri::command]
pub fn request_lookup<R: Runtime>(
    text: String,
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {

    // Transition first so any mouseup arriving mid-flight is suppressed.
    state.enter_popup();
    if let Some(floater) = app.get_webview_window(FLOATER_LABEL) {
        if let Err(e) = floater.hide() {
            log::warn!("floater hide: {e}");
        }
    }
    let popup = app
        .get_webview_window(POPUP_LABEL)
        .ok_or_else(|| AppError::Other("popup window not found".into()))?;
    if let Some((x, y)) = state.last_cursor() {
        let anchor = PhysicalPosition::new(x, y + POPUP_DROP);
        window::position_at(&app, &popup, anchor)?;
    }
    popup.show()?;

    // Refocus-less popups sink under the taskbar unless topmost is re-asserted post-show.
    popup.set_always_on_top(true)?;
    popup.set_focus()?;
    app.emit_to(POPUP_LABEL, "lookup-request", LookupPayload { text: &text })?;
    Ok(())
}

#[tauri::command]
pub fn notify_floater_hidden(state: State<'_, Arc<AppState>>) {
    state.floater_hidden();
}

#[tauri::command]
pub fn notify_popup_hidden(state: State<'_, Arc<AppState>>) {
    state.popup_hidden();
}

#[tauri::command]
pub fn set_autostart<R: Runtime>(
    enabled: bool,
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| AppError::Other(format!("autostart enable: {e}")))?;
    } else {
        manager
            .disable()
            .map_err(|e| AppError::Other(format!("autostart disable: {e}")))?;
    }
    let mut cfg = state.config();
    cfg.autostart = enabled;
    state.set_config(cfg.clone());
    config::save(&app, &cfg)?;
    log::info!("autostart set to {enabled}");
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> AppConfig {
    state.config()
}

#[tauri::command]
pub fn set_config<R: Runtime>(
    cfg: AppConfig,
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    let old = state.config();
    state.set_config(cfg.clone());
    config::save(&app, &cfg)?;
    if cfg.hotkey != old.hotkey {
        hotkey::register(&app, &cfg.hotkey)?;
        log::info!("hotkey re-registered: {}", cfg.hotkey);
    }
    if cfg.skin != old.skin {
        if let Err(e) = app.emit(events::SKIN_CHANGED, cfg.skin.clone()) {
            log::warn!("emit {}: {e}", events::SKIN_CHANGED);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn speak_text(text: String, accent: TtsAccent) -> AppResult<()> {
    async_runtime::spawn_blocking(move || {
        let accent = match accent {
            TtsAccent::EnUs => tts::Accent::EnUs,
            TtsAccent::EnGb => tts::Accent::EnGb,
        };
        tts::speak_with_accent(&text, accent)
    })
    .await
    .map_err(|e| AppError::Other(format!("tts join: {e}")))?
}
