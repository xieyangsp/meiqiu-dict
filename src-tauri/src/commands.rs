// Tauri command handlers. Thin wrappers over business modules.
// Commands propagate AppResult to the frontend; AppError serializes as a string.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, Runtime, State, async_runtime};

use crate::dict::{self, DictEntry};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const FLOATER_LABEL: &str = "floater";
const POPUP_LABEL: &str = "popup";

/// Vertical offset of the popup below the captured cursor.
const POPUP_DROP: i32 = 30;

#[derive(Clone, Serialize)]
struct LookupPayload<'a> {
    text: &'a str,
}

/// Look up a word in the bundled dictionary.
/// Returns Ok(None) if the word is not found or the query is empty.
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

/// Hide the floater, position and show the popup, and tell it which word to look up.
#[tauri::command]
pub fn request_lookup<R: Runtime>(
    text: String,
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if let Some(floater) = app.get_webview_window(FLOATER_LABEL) {
        let _ = floater.hide();
    }
    let popup = app
        .get_webview_window(POPUP_LABEL)
        .ok_or_else(|| AppError::Other("popup window not found".into()))?;
    if let Some((x, y)) = state.last_cursor() {
        let target = PhysicalPosition::new(x, y + POPUP_DROP);
        popup.set_position(Position::Physical(target))?;
    }
    popup.show()?;
    popup.set_focus()?;
    app.emit_to(POPUP_LABEL, "lookup-request", LookupPayload { text: &text })?;
    Ok(())
}
