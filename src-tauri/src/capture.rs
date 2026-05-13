// Mouse listener + clipboard cycle. When capture is enabled and the user
// releases the left mouse button, simulate Ctrl+C, read the selection,
// position the floater near the cursor, show it, and emit the text payload.
//
// rdev::listen blocks for the process lifetime and runs on its own OS thread.
// Its callback must stay non-blocking, so we offload the clipboard cycle to
// a worker thread via a single-slot channel.

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rdev::{Button, Event, EventType, listen};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime};

use crate::error::{AppError, AppResult};
use crate::selection::is_acceptable_selection;
use crate::state::AppState;
use crate::window::clamp_to_monitor;

/// Minimum gap between two capture candidates. Guards against double-click
/// noise and rapid drag-release sequences.
const THROTTLE: Duration = Duration::from_millis(200);

/// How long to wait after Ctrl+C before reading the clipboard. The source
/// application needs time to populate CF_UNICODETEXT.
const COPY_SETTLE: Duration = Duration::from_millis(80);

/// Floater is shown slightly offset from the cursor so it does not sit under
/// the pointer.
const FLOATER_OFFSET: i32 = 12;

const FLOATER_LABEL: &str = "floater";

/// Declared floater size in tauri.conf.json; kept in sync manually.
const FLOATER_W: u32 = 88;
const FLOATER_H: u32 = 36;

#[derive(Clone, Serialize)]
struct SelectionPayload<'a> {
    text: &'a str,
}

/// Spawn the rdev listener thread and the clipboard-cycle worker thread.
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
        // Drain extra pending signals so we only do one cycle per burst.
        // Keep the most recent cursor position.
        while let Ok(next) = rx.try_recv() {
            pos = next;
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(acquire_selection));
        match outcome {
            Ok(Ok(Some(text))) => {
                let trimmed = text.trim();
                if is_acceptable_selection(trimmed) {
                    log::info!("capture acquired: {trimmed:?}");
                    state.set_last_cursor(pos);
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

/// Reposition the floater near the cursor and emit the selection text.
fn show_floater<R: Runtime>(app: &AppHandle<R>, text: &str, (x, y): (i32, i32)) {
    let Some(win) = app.get_webview_window(FLOATER_LABEL) else {
        log::warn!("floater window not found");
        return;
    };
    let anchor = PhysicalPosition::new(x + FLOATER_OFFSET, y + FLOATER_OFFSET);
    // outer_size() returns physical pixels and accounts for DPI scaling;
    // fall back to the declared logical size if the query fails.
    let size = win
        .outer_size()
        .unwrap_or(PhysicalSize::new(FLOATER_W, FLOATER_H));
    let target = clamp_to_monitor(app, anchor, size);
    if let Err(e) = win.set_position(tauri::Position::Physical(target)) {
        log::warn!("floater set_position: {e}");
    }
    if let Err(e) = win.show() {
        log::warn!("floater show: {e}");
    }
    // Re-assert topmost so we sit above the Windows taskbar; the config
    // value alone is not enough when the window never takes focus.
    if let Err(e) = win.set_always_on_top(true) {
        log::warn!("floater set_always_on_top: {e}");
    }
    if let Err(e) = app.emit_to(FLOATER_LABEL, "selection-acquired", SelectionPayload { text }) {
        log::warn!("floater emit: {e}");
    }
}

/// Backup clipboard, simulate Ctrl+C, read selection, restore clipboard.
/// Returns `None` if the clipboard did not change (no active selection).
fn acquire_selection() -> AppResult<Option<String>> {
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

    // Detect "nothing happened": empty read, or read equals backup => no
    // active selection. We deliberately do not clear the clipboard first,
    // so as not to disturb non-text clipboard payloads (images, etc.) and
    // to avoid racing the OS clipboard chain against IME hooks.
    let candidate = match (new_text, backup) {
        (Some(t), _) if t.is_empty() => None,
        (Some(t), Some(prev)) if t == prev => None,
        (Some(t), _) => Some(t),
        (None, _) => None,
    };
    Ok(candidate)
}

fn simulate_copy() -> AppResult<()> {
    use enigo::{
        Direction::{Click, Press, Release},
        Enigo, Key, Keyboard, Settings,
    };
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::Capture(format!("enigo init: {e}")))?;
    enigo
        .key(Key::Control, Press)
        .map_err(|e| AppError::Capture(format!("enigo press ctrl: {e}")))?;
    let click_result = enigo
        .key(Key::Unicode('c'), Click)
        .map_err(|e| AppError::Capture(format!("enigo click c: {e}")));
    let release_result = enigo
        .key(Key::Control, Release)
        .map_err(|e| AppError::Capture(format!("enigo release ctrl: {e}")));
    click_result?;
    release_result?;
    Ok(())
}
