// Tauri command handlers. Thin wrappers over business modules.
// Commands propagate AppResult to the frontend; AppError serializes as a string.

use std::sync::Arc;

use tauri::{State, async_runtime};

use crate::dict::{self, DictEntry};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

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
