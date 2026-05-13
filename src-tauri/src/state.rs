// Shared application state. Business modules talk through AppState
// instead of importing each other.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Default)]
pub struct AppState {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    config: AppConfig,
    capture_enabled: bool,
}

impl AppState {
    pub fn new(config: AppConfig) -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(Inner {
                config,
                capture_enabled: false,
            }),
        })
    }

    pub fn config(&self) -> AppConfig {
        self.inner.read().config.clone()
    }

    pub fn set_config(&self, config: AppConfig) {
        self.inner.write().config = config;
    }

    pub fn capture_enabled(&self) -> bool {
        self.inner.read().capture_enabled
    }

    /// Toggle the capture flag and return the new value.
    pub fn toggle_capture(&self) -> bool {
        let mut g = self.inner.write();
        g.capture_enabled = !g.capture_enabled;
        g.capture_enabled
    }
}
