use std::str::FromStr;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, AppResult};
use crate::events;
use crate::state::AppState;

pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> AppResult<()> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Hotkey(format!("failed to parse accelerator {accelerator}: {e}")))?;
    let gs = app.global_shortcut();
    gs.unregister_all()
        .map_err(|e| AppError::Hotkey(e.to_string()))?;
    gs.on_shortcut(shortcut, on_triggered)
        .map_err(|e| AppError::Hotkey(e.to_string()))?;
    Ok(())
}

fn on_triggered<R: Runtime>(app: &AppHandle<R>, _shortcut: &Shortcut, event: tauri_plugin_global_shortcut::ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    let enabled = state.toggle_capture();
    log::info!("hotkey toggled capture: enabled={enabled}");
    if let Err(e) = app.emit(events::CAPTURE_TOGGLED, enabled) {
        log::warn!("hotkey emit {}: {e}", events::CAPTURE_TOGGLED);
    }
}
