// Library entry: assembles plugins, state, tray, and the hotkey.
// Tauri 2 idiom: startup lives in lib; main only calls run().

mod config;
mod error;
mod hotkey;
mod state;
mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle();

            let cfg = config::load(handle).unwrap_or_else(|e| {
                log::warn!("failed to load config, using defaults: {e}");
                config::AppConfig::default()
            });
            let hotkey = cfg.hotkey.clone();
            let state = AppState::new(cfg);
            app.manage::<Arc<AppState>>(state);

            tray::build(handle)?;
            hotkey::register(handle, &hotkey)?;

            log::info!("meiqiu-dict started, hotkey={hotkey}");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start tauri application");
}
