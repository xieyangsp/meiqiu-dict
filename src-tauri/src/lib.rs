// Library entry: assembles plugins, state, tray, and the hotkey.
// Tauri 2 idiom: startup lives in lib; main only calls run().

mod capture;
mod commands;
mod config;
mod dict;
mod error;
mod hotkey;
mod selection;
mod state;
mod tray;
mod uia;
mod window;

use std::sync::Arc;

use tauri::Manager;
use tauri::path::BaseDirectory;
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
        .invoke_handler(tauri::generate_handler![
            commands::dict_lookup,
            commands::request_lookup
        ])
        .setup(|app| {
            let handle = app.handle();

            let cfg = config::load(handle).unwrap_or_else(|e| {
                log::warn!("failed to load config, using defaults: {e}");
                config::AppConfig::default()
            });
            let hotkey = cfg.hotkey.clone();
            let state = AppState::new(cfg);
            app.manage::<Arc<AppState>>(state.clone());

            // Bundled dictionary lives at <resource>/resources/ecdict.db.
            match handle
                .path()
                .resolve("resources/ecdict.db", BaseDirectory::Resource)
            {
                Ok(db_path) => match dict::open(&db_path) {
                    Ok(pool) => {
                        state.set_dict(pool);
                        log::info!("dict pool open: {}", db_path.display());
                    }
                    Err(e) => log::warn!("dict pool open failed: {e}"),
                },
                Err(e) => log::warn!("resolve ecdict.db path failed: {e}"),
            }

            tray::build(handle)?;
            hotkey::register(handle, &hotkey)?;
            if let Err(e) = capture::start_listener(handle.clone(), state.clone()) {
                log::warn!("capture listener start failed: {e}");
            }

            log::info!("meiqiu-dict started, hotkey={hotkey}");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start tauri application");
}
