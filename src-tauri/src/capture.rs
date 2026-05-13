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
        // Trap panics from arboard/enigo so a single bad cycle never tears
        // down the worker thread (and through it, the whole app).
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(acquire_selection));
        match outcome {
            Ok(Ok(Some(text))) => {
                if is_acceptable_selection(&text) {
                    log::info!("capture acquired: {:?}", text.trim());
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
