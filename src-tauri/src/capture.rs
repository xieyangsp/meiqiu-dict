// Global mouse listener. When capture is enabled, log left-button release events
// as selection candidates. Position acquisition and clipboard cycle land later.
//
// rdev::listen blocks for the process lifetime; we run it on a dedicated thread.
// Callbacks must be Send + 'static; we keep them tiny and only touch AppState
// through its existing thread-safe API.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rdev::{Button, Event, EventType, listen};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Minimum gap between two capture candidates. Guards against double-click
/// noise and rapid drag-release sequences.
const THROTTLE: Duration = Duration::from_millis(200);

/// Spawn the background listener. Returns once the thread is up.
pub fn start_listener(state: Arc<AppState>) -> AppResult<()> {
    let last_emit: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    thread::Builder::new()
        .name("capture-listener".into())
        .spawn(move || {
            if let Err(e) = listen(move |event| handle(event, &state, &last_emit)) {
                log::error!("rdev listen failed: {e:?}");
            }
        })
        .map_err(|e| AppError::Capture(format!("spawn listener: {e}")))?;
    Ok(())
}

fn handle(event: Event, state: &AppState, last_emit: &Mutex<Option<Instant>>) {
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
    log::info!("capture candidate: mouseup");
}
