// Shared application state. Business modules talk through AppState
// instead of importing each other.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::dict::DictPool;

/// Lifecycle of the capture pipeline. Mouseups only trigger capture when
/// the machine is in `Idle` or `Floater`. `Disabled` gates everything off;
/// `Popup` means the user is interacting with the lookup popup, so new
/// captures must not be started until the popup hides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureState {
    #[default]
    Disabled,
    Idle,
    Floater,
    Popup,
}

#[derive(Default)]
pub struct AppState {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    config: AppConfig,
    capture_state: CaptureState,
    dict: Option<DictPool>,
    last_cursor: Option<(i32, i32)>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(Inner {
                config,
                capture_state: CaptureState::Disabled,
                dict: None,
                last_cursor: None,
            }),
        })
    }

    pub fn config(&self) -> AppConfig {
        self.inner.read().config.clone()
    }

    pub fn set_config(&self, config: AppConfig) {
        self.inner.write().config = config;
    }

    /// True when the capture pipeline is in any non-Disabled state.
    pub fn capture_enabled(&self) -> bool {
        self.inner.read().capture_state != CaptureState::Disabled
    }

    pub fn capture_state(&self) -> CaptureState {
        self.inner.read().capture_state
    }

    /// Hotkey master switch: Disabled <-> Idle. Toggling off from any
    /// state (including Floater / Popup) returns to Disabled.
    pub fn toggle_capture(&self) -> bool {
        let mut g = self.inner.write();
        g.capture_state = if g.capture_state == CaptureState::Disabled {
            CaptureState::Idle
        } else {
            CaptureState::Disabled
        };
        g.capture_state != CaptureState::Disabled
    }

    /// Idle / Floater -> Floater. Called after a successful selection
    /// capture. No-op if the machine is Disabled or already in Popup.
    pub fn enter_floater(&self) {
        let mut g = self.inner.write();
        if matches!(g.capture_state, CaptureState::Idle | CaptureState::Floater) {
            g.capture_state = CaptureState::Floater;
        }
    }

    /// Floater -> Popup. Called by request_lookup before showing the popup.
    /// No-op if the machine is not currently Floater.
    pub fn enter_popup(&self) {
        let mut g = self.inner.write();
        if g.capture_state == CaptureState::Floater {
            g.capture_state = CaptureState::Popup;
        }
    }

    /// Floater -> Idle. Invoked by the frontend after the floater hides
    /// itself (timeout or click-when-empty). Idempotent.
    pub fn floater_hidden(&self) {
        let mut g = self.inner.write();
        if g.capture_state == CaptureState::Floater {
            g.capture_state = CaptureState::Idle;
        }
    }

    /// Popup -> Idle. Invoked by the frontend after the popup hides itself
    /// (close button or Esc). Idempotent.
    pub fn popup_hidden(&self) {
        let mut g = self.inner.write();
        if g.capture_state == CaptureState::Popup {
            g.capture_state = CaptureState::Idle;
        }
    }

    pub fn set_dict(&self, pool: DictPool) {
        self.inner.write().dict = Some(pool);
    }

    /// Clone of the pool handle (r2d2::Pool is internally Arc).
    pub fn dict(&self) -> Option<DictPool> {
        self.inner.read().dict.clone()
    }

    pub fn set_last_cursor(&self, pos: (i32, i32)) {
        self.inner.write().last_cursor = Some(pos);
    }

    pub fn last_cursor(&self) -> Option<(i32, i32)> {
        self.inner.read().last_cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_disabled_and_idle() {
        let s = AppState::new(AppConfig::default());
        assert_eq!(s.capture_state(), CaptureState::Disabled);
        assert!(!s.capture_enabled());
        assert!(s.toggle_capture());
        assert_eq!(s.capture_state(), CaptureState::Idle);
        assert!(s.capture_enabled());
        assert!(!s.toggle_capture());
        assert_eq!(s.capture_state(), CaptureState::Disabled);
    }

    #[test]
    fn idle_to_floater_to_popup_to_idle() {
        let s = AppState::new(AppConfig::default());
        s.toggle_capture();
        s.enter_floater();
        assert_eq!(s.capture_state(), CaptureState::Floater);
        s.enter_popup();
        assert_eq!(s.capture_state(), CaptureState::Popup);
        s.popup_hidden();
        assert_eq!(s.capture_state(), CaptureState::Idle);
    }

    #[test]
    fn floater_hides_back_to_idle() {
        let s = AppState::new(AppConfig::default());
        s.toggle_capture();
        s.enter_floater();
        s.floater_hidden();
        assert_eq!(s.capture_state(), CaptureState::Idle);
    }

    #[test]
    fn transitions_are_noops_on_unexpected_state() {
        let s = AppState::new(AppConfig::default());
        // Disabled: everything except toggle is a no-op.
        s.enter_floater();
        s.enter_popup();
        s.floater_hidden();
        s.popup_hidden();
        assert_eq!(s.capture_state(), CaptureState::Disabled);
        s.toggle_capture();
        // Idle: enter_popup requires Floater; hidden hooks require their own state.
        s.enter_popup();
        s.popup_hidden();
        s.floater_hidden();
        assert_eq!(s.capture_state(), CaptureState::Idle);
    }

    #[test]
    fn disable_resets_from_any_ui_state() {
        let s = AppState::new(AppConfig::default());
        s.toggle_capture();
        s.enter_floater();
        s.enter_popup();
        assert_eq!(s.capture_state(), CaptureState::Popup);
        assert!(!s.toggle_capture());
        assert_eq!(s.capture_state(), CaptureState::Disabled);
    }
}

