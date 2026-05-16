use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rdev::{Button, Event, EventType, listen};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime};

use crate::config::CaptureMethod;
use crate::error::{AppError, AppResult};
use crate::selection::{SelectionOutcome, is_acceptable_selection};
use crate::state::{AppState, CaptureState};
use crate::uia;
use crate::window::clamp_to_monitor;

const THROTTLE: Duration = Duration::from_millis(200);

// Source apps need time to populate CF_UNICODETEXT after Ctrl+C.
const COPY_SETTLE: Duration = Duration::from_millis(80);

const FLOATER_OFFSET: i32 = 12;
const FLOATER_LABEL: &str = "floater";

// Clicks inside our own webviews must not start a capture cycle.
const OWN_WINDOW_LABELS: &[&str] = &["floater", "popup", "main"];

#[derive(Clone, Serialize)]
struct SelectionPayload<'a> {
    text: &'a str,
}

pub fn start_listener<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) -> AppResult<()> {
    let cursor: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((0, 0)));
    let (tx, rx) = mpsc::channel::<(i32, i32)>();

    let app_worker = app.clone();
    let state_worker = state.clone();
    thread::Builder::new()
        .name("capture-worker".into())
        .spawn(move || worker_loop(rx, app_worker, state_worker))
        .map_err(|e| AppError::Capture(format!("spawn worker: {e}")))?;

    let last_emit: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let cursor_listener = cursor.clone();
    thread::Builder::new()
        .name("capture-listener".into())
        .spawn(move || {
            if let Err(e) = listen(move |event| {
                handle(event, &state, &last_emit, &tx, &cursor_listener)
            }) {
                log::error!("rdev listen failed: {e:?}");
            }
        })
        .map_err(|e| AppError::Capture(format!("spawn listener: {e}")))?;
    Ok(())
}

fn handle(
    event: Event,
    state: &AppState,
    last_emit: &Mutex<Option<Instant>>,
    tx: &mpsc::Sender<(i32, i32)>,
    cursor: &Mutex<(i32, i32)>,
) {
    match event.event_type {
        EventType::MouseMove { x, y } => {
            *cursor.lock() = (x as i32, y as i32);
        }
        EventType::ButtonRelease(Button::Left) => {
            if !state.capture_enabled() {
                return;
            }
            let now = Instant::now();
            let mut guard = last_emit.lock();
            if let Some(prev) = *guard {
                if now.duration_since(prev) < THROTTLE {
                    return;
                }
            }
            *guard = Some(now);
            drop(guard);
            let pos = *cursor.lock();
            if tx.send(pos).is_err() {
                log::warn!("capture worker channel closed");
            }
        }
        _ => {}
    }
}

fn worker_loop<R: Runtime>(
    rx: mpsc::Receiver<(i32, i32)>,
    app: AppHandle<R>,
    state: Arc<AppState>,
) {
    while let Ok(mut pos) = rx.recv() {

        // Collapse a burst of mouseups into one capture at the latest cursor.
        while let Ok(next) = rx.try_recv() {
            pos = next;
        }

        // Popup owns the foreground; our own webview clicks are not selections.
        let current = state.capture_state();
        if current == CaptureState::Popup {
            log::debug!("capture skipped: state=Popup at {pos:?}");
            continue;
        }
        if click_on_own_window(&app, pos) {
            log::debug!("capture skipped: click on our own window at {pos:?}");
            continue;
        }
        let state_ref = state.as_ref();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            acquire_selection(state_ref)
        }));
        match outcome {
            Ok(Ok(Some(text))) => {
                let trimmed = text.trim();
                if is_acceptable_selection(trimmed) {
                    log::info!("capture acquired: {trimmed:?}");
                    state.set_last_cursor(pos);
                    state.enter_floater();
                    show_floater(&app, trimmed, pos);
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(e)) => log::warn!("capture failed: {e}"),
            Err(payload) => {
                let msg = panic_message(payload.as_ref());
                log::error!("capture panicked: {msg}");
            }
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<unknown panic payload>".to_string()
    }
}

fn click_on_own_window<R: Runtime>(app: &AppHandle<R>, (x, y): (i32, i32)) -> bool {
    for label in OWN_WINDOW_LABELS {
        let Some(win) = app.get_webview_window(label) else {
            continue;
        };
        if !win.is_visible().unwrap_or(false) {
            continue;
        }
        let Ok(origin) = win.outer_position() else {
            continue;
        };
        let Ok(size) = win.outer_size() else {
            continue;
        };
        let w = size.width as i32;
        let h = size.height as i32;
        if x >= origin.x && x < origin.x + w && y >= origin.y && y < origin.y + h {
            return true;
        }
    }
    false
}

fn show_floater<R: Runtime>(app: &AppHandle<R>, text: &str, (x, y): (i32, i32)) {
    let Some(win) = app.get_webview_window(FLOATER_LABEL) else {
        log::warn!("floater window not found");
        return;
    };
    let anchor = PhysicalPosition::new(x + FLOATER_OFFSET, y + FLOATER_OFFSET);
    match win.outer_size() {
        Ok(size) => {
            let target = clamp_to_monitor(app, anchor, size);
            if let Err(e) = win.set_position(tauri::Position::Physical(target)) {
                log::warn!("floater set_position: {e}");
            }
        }
        Err(e) => log::warn!("floater outer_size: {e}; skipping reposition"),
    }
    if let Err(e) = win.show() {
        log::warn!("floater show: {e}");
    }

    // Refocus-less windows sink under the taskbar unless topmost is re-asserted post-show.
    if let Err(e) = win.set_always_on_top(true) {
        log::warn!("floater set_always_on_top: {e}");
    }
    if let Err(e) = app.emit_to(FLOATER_LABEL, "selection-acquired", SelectionPayload { text }) {
        log::warn!("floater emit: {e}");
    }
}

// Text / NoSelection are terminal; Unsupported falls through to the next method.
fn acquire_selection(state: &AppState) -> AppResult<Option<String>> {
    let cfg = state.config();
    for method in &cfg.capture_methods {
        let enabled = match method {
            CaptureMethod::Uia => cfg.uia_enabled,
            CaptureMethod::Clipboard => cfg.clipboard_enabled,
        };
        if !enabled {
            continue;
        }
        let outcome = match method {
            CaptureMethod::Uia => uia::try_get_selection(),
            CaptureMethod::Clipboard => try_clipboard(),
        };
        match outcome {
            SelectionOutcome::Text(t) => return Ok(Some(t)),
            SelectionOutcome::NoSelection => return Ok(None),
            SelectionOutcome::Unsupported => continue,
        }
    }
    Ok(None)
}

fn try_clipboard() -> SelectionOutcome {
    match clipboard_cycle() {
        Ok(Some(t)) => SelectionOutcome::Text(t),
        Ok(None) => SelectionOutcome::NoSelection,
        Err(e) => {
            log::warn!("clipboard capture failed: {e}");
            SelectionOutcome::Unsupported
        }
    }
}

fn clipboard_cycle() -> AppResult<Option<String>> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| AppError::Capture(format!("clipboard open: {e}")))?;

    let backup = cb.get_text().ok();

    let copy_result = simulate_copy();

    thread::sleep(COPY_SETTLE);

    let new_text = cb.get_text().ok();

    if let Some(prev) = &backup {
        let _ = cb.set_text(prev.clone());
    }

    copy_result?;

    // Empty read or read == backup means nothing was selected; pre-clearing would
    // disturb non-text payloads and race the clipboard chain against IME hooks.
    let candidate = match (new_text, backup) {
        (Some(t), _) if t.is_empty() => None,
        (Some(t), Some(prev)) if t == prev => None,
        (Some(t), _) => Some(t),
        (None, _) => None,
    };
    Ok(candidate)
}

// One SendInput batch is uninterruptible, so IME hooks and real keys cannot
// break the modifier state mid-sequence and leak a stray 'c'.
fn simulate_copy() -> AppResult<()> {
    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAP_VIRTUAL_KEY_TYPE, MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_CONTROL,
    };

    const VK_C: VIRTUAL_KEY = VIRTUAL_KEY(0x43);
    const MAPVK_VK_TO_VSC: MAP_VIRTUAL_KEY_TYPE = MAP_VIRTUAL_KEY_TYPE(0);

    // Games and remote desktops read scancodes, not just virtual keys.
    let ctrl_scan = unsafe { MapVirtualKeyW(VK_CONTROL.0 as u32, MAPVK_VK_TO_VSC) } as u16;
    let c_scan = unsafe { MapVirtualKeyW(VK_C.0 as u32, MAPVK_VK_TO_VSC) } as u16;

    let event = |vk: VIRTUAL_KEY, scan: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let inputs: [INPUT; 4] = [
        event(VK_CONTROL, ctrl_scan, false),
        event(VK_C, c_scan, false),
        event(VK_C, c_scan, true),
        event(VK_CONTROL, ctrl_scan, true),
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        let err = unsafe { GetLastError() };
        return Err(AppError::Capture(format!(
            "SendInput sent {sent}/{} inputs (last error {err:?})",
            inputs.len()
        )));
    }
    Ok(())
}
