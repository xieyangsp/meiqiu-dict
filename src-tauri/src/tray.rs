// System tray with two-state icon (active/idle) and a menu (toggle / quit).
// Owns no business state; talks to AppState via Tauri's managed state.

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const TRAY_ID: &str = "main-tray";

const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");

const MENU_TOGGLE: &str = "toggle_capture";
const MENU_QUIT: &str = "quit";

const LABEL_ENABLE: &str = "启用划词监听";
const LABEL_DISABLE: &str = "关闭划词监听";
const TIP_ON: &str = "煤球词典 — 划词监听：开";
const TIP_OFF: &str = "煤球词典 — 划词监听：关";

/// Managed handle to the toggle menu item so `sync()` can update its label.
struct ToggleMenuItem<R: Runtime>(MenuItem<R>);

pub fn build<R: Runtime>(app: &AppHandle<R>) -> AppResult<TrayIcon<R>> {
    let toggle = MenuItem::with_id(app, MENU_TOGGLE, LABEL_ENABLE, true, None::<&str>)
        .map_err(|e| AppError::Tray(e.to_string()))?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)
        .map_err(|e| AppError::Tray(e.to_string()))?;
    let menu = Menu::with_items(app, &[&toggle, &quit]).map_err(|e| AppError::Tray(e.to_string()))?;

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(load_icon(false)?)
        .icon_as_template(false)
        .tooltip(TIP_OFF)
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .build(app)
        .map_err(|e| AppError::Tray(e.to_string()))?;
    app.manage(ToggleMenuItem(toggle));
    Ok(tray)
}

/// Sync the tray icon, tooltip, and toggle-item label with the latest state.
pub fn sync<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> AppResult<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Err(AppError::Tray("tray not found".into()));
    };
    tray.set_icon(Some(load_icon(enabled)?))
        .map_err(|e| AppError::Tray(e.to_string()))?;
    tray.set_tooltip(Some(if enabled { TIP_ON } else { TIP_OFF }))
        .map_err(|e| AppError::Tray(e.to_string()))?;
    if let Some(item) = app.try_state::<ToggleMenuItem<R>>() {
        item.0
            .set_text(if enabled { LABEL_DISABLE } else { LABEL_ENABLE })
            .map_err(|e| AppError::Tray(e.to_string()))?;
    }
    Ok(())
}

fn load_icon(active: bool) -> AppResult<Image<'static>> {
    let bytes = if active { ICON_ACTIVE } else { ICON_IDLE };
    Image::from_bytes(bytes).map_err(|e| AppError::Tray(format!("failed to load icon: {e}")))
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_TOGGLE => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let enabled = state.toggle_capture();
                log::info!("tray toggled capture: enabled={enabled}");
                if let Err(e) = sync(app, enabled) {
                    log::warn!("tray sync failed: {e}");
                }
            }
        }
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}
