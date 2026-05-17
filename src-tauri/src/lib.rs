mod capture;
mod commands;
mod config;
mod dict;
mod error;
mod events;
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

use crate::error::AppResult;
use crate::state::{AppState, DictPool};
use crate::window::MAIN_LABEL;

fn try_open_dict<R: tauri::Runtime>(handle: &tauri::AppHandle<R>) -> AppResult<DictPool> {
    let db_path = handle
        .path()
        .resolve("resources/ecdict.db", BaseDirectory::Resource)?;
    let pool = dict::open(&db_path)?;
    log::info!("dict pool open: {}", db_path.display());
    Ok(pool)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            log::info!("second instance ignored (args={args:?}, cwd={cwd:?})");
            if let Some(win) = app.get_webview_window(MAIN_LABEL) {
                if let Err(e) = win.show() {
                    log::warn!("single-instance show main: {e}");
                }
                if let Err(e) = win.set_focus() {
                    log::warn!("single-instance focus main: {e}");
                }
            }
        }))
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
            commands::request_lookup,
            commands::notify_floater_hidden,
            commands::notify_popup_hidden
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

            match try_open_dict(handle) {
                Ok(pool) => state.set_dict(pool),
                Err(e) => log::warn!("dict pool init failed: {e}"),
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
