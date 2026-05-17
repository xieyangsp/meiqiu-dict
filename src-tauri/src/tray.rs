use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};

use crate::error::{AppError, AppResult};
use crate::events;
use crate::state::AppState;
use crate::window::MAIN_LABEL;

pub const TRAY_ID: &str = "main-tray";

const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");

const MENU_SETTINGS: &str = "open_settings";
const MENU_TOGGLE: &str = "toggle_capture";
const MENU_QUIT: &str = "quit";

const LABEL_SETTINGS: &str = "打开设置";
const LABEL_ENABLE: &str = "启用划词监听";
const LABEL_DISABLE: &str = "关闭划词监听";
const TIP_ON: &str = "煤球词典 — 划词监听：开";
const TIP_OFF: &str = "煤球词典 — 划词监听：关";

// Managed handle to the toggle menu item so `sync` can relabel it.
struct ToggleMenuItem<R: Runtime>(MenuItem<R>);

pub fn build<R: Runtime>(app: &AppHandle<R>) -> AppResult<TrayIcon<R>> {
    let settings = MenuItem::with_id(app, MENU_SETTINGS, LABEL_SETTINGS, true, None::<&str>)
        .map_err(AppError::tray)?;
    let toggle = MenuItem::with_id(app, MENU_TOGGLE, LABEL_ENABLE, true, None::<&str>)
        .map_err(AppError::tray)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)
        .map_err(AppError::tray)?;
    let menu = Menu::with_items(app, &[&settings, &toggle, &quit]).map_err(AppError::tray)?;

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(load_icon(false)?)
        .icon_as_template(false)
        .tooltip(TIP_OFF)
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)
        .map_err(AppError::tray)?;
    app.manage(ToggleMenuItem(toggle));

    let listen_handle = app.clone();
    app.listen(events::CAPTURE_TOGGLED, move |event| {
        match serde_json::from_str::<bool>(event.payload()) {
            Ok(enabled) => {
                if let Err(e) = sync(&listen_handle, enabled) {
                    log::warn!("tray sync via event failed: {e}");
                }
            }
            Err(e) => log::warn!("tray event payload parse failed: {e}"),
        }
    });
    Ok(tray)
}

fn sync<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> AppResult<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Err(AppError::Tray("tray not found".into()));
    };
    tray.set_icon(Some(load_icon(enabled)?))
        .map_err(AppError::tray)?;
    tray.set_tooltip(Some(if enabled { TIP_ON } else { TIP_OFF }))
        .map_err(AppError::tray)?;
    if let Some(item) = app.try_state::<ToggleMenuItem<R>>() {
        item.0
            .set_text(if enabled { LABEL_DISABLE } else { LABEL_ENABLE })
            .map_err(AppError::tray)?;
    }
    Ok(())
}

fn load_icon(active: bool) -> AppResult<Image<'static>> {
    let bytes = if active { ICON_ACTIVE } else { ICON_IDLE };
    Image::from_bytes(bytes).map_err(|e| AppError::Tray(format!("failed to load icon: {e}")))
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_SETTINGS => show_main_window(app),
        MENU_TOGGLE => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let enabled = state.toggle_capture();
                log::info!("tray toggled capture: enabled={enabled}");
                if let Err(e) = app.emit(events::CAPTURE_TOGGLED, enabled) {
                    log::warn!("tray emit {}: {e}", events::CAPTURE_TOGGLED);
                }
            }
        }
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

fn handle_tray_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(MAIN_LABEL) else {
        log::warn!("main window not found");
        return;
    };
    if let Err(e) = win.show() {
        log::warn!("main show: {e}");
    }
    if let Err(e) = win.set_focus() {
        log::warn!("main set_focus: {e}");
    }
}
