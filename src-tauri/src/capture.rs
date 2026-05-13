// Mouse listener + clipboard cycle. When capture is enabled and the user
// releases the left mouse button, simulate Ctrl+C, read the selection,
// restore the previous clipboard, and log accepted candidates.
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

use crate::error::{AppError, AppResult};
use crate::selection::is_acceptable_selection;
use crate::state::AppState;

/// Minimum gap between two capture candidates. Guards against double-click
/// noise and rapid drag-release sequences.
const THROTTLE: Duration = Duration::from_millis(200);

/// How long to wait after Ctrl+C before reading the clipboard. The source
/// application needs time to populate CF_UNICODETEXT.
const COPY_SETTLE: Duration = Duration::from_millis(80);

/// Spawn the rdev listener thread and the clipboard-cycle worker thread.
pub fn start_listener(state: Arc<AppState>) -> AppResult<()> {
    let (tx, rx) = mpsc::channel::<()>();

    thread::Builder::new()
        .name("capture-worker".into())
        .spawn(move || worker_loop(rx))
        .map_err(|e| AppError::Capture(format!("spawn worker: {e}")))?;

    let last_emit: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    thread::Builder::new()
        .name("capture-listener".into())
        .spawn(move || {
            if let Err(e) = listen(move |event| handle(event, &state, &last_emit, &tx)) {
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
    tx: &mpsc::Sender<()>,
) {
    let EventType::ButtonRelease(Button::Left) = event.event_type else {
        return;
    };
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
    if tx.send(()).is_err() {
        log::warn!("capture worker channel closed");
    }
}

fn worker_loop(rx: mpsc::Receiver<()>) {
    while rx.recv().is_ok() {
        // Drain extra pending signals so we only do one cycle per burst.
        while rx.try_recv().is_ok() {}
        match acquire_selection() {
            Ok(Some(text)) => {
                if is_acceptable_selection(&text) {
                    log::info!("capture acquired: {:?}", text.trim());
                } else {
                    log::debug!("capture rejected: {:?}", text.trim());
                }
            }
            Ok(None) => log::debug!("capture: no selection"),
            Err(e) => log::warn!("capture failed: {e}"),
        }
    }
}

/// Backup clipboard, simulate Ctrl+C, read selection, restore clipboard.
/// Returns `None` if the clipboard did not change (no active selection).
fn acquire_selection() -> AppResult<Option<String>> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| AppError::Capture(format!("clipboard open: {e}")))?;
    let backup = cb.get_text().ok();

    // Sentinel so we can tell "Ctrl+C did nothing" apart from "user selected
    // the same text that was already on the clipboard".
    cb.set_text(String::new())
        .map_err(|e| AppError::Capture(format!("clipboard clear: {e}")))?;

    let copy_result = simulate_copy();

    thread::sleep(COPY_SETTLE);

    let new_text = cb.get_text().ok();

    // Always restore, regardless of read/copy outcomes.
    if let Some(prev) = backup {
        let _ = cb.set_text(prev);
    } else {
        let _ = cb.clear();
    }

    copy_result?;
    Ok(new_text.filter(|s| !s.is_empty()))
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
