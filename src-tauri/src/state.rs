// Shared application state. Business modules talk through AppState
// instead of importing each other.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::dict::DictPool;

#[derive(Default)]
pub struct AppState {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    config: AppConfig,
    capture_enabled: bool,
    dict: Option<DictPool>,
    last_cursor: Option<(i32, i32)>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(Inner {
                config,
                capture_enabled: false,
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

    pub fn capture_enabled(&self) -> bool {
        self.inner.read().capture_enabled
    }

    /// Toggle the capture flag and return the new value.
    pub fn toggle_capture(&self) -> bool {
        let mut g = self.inner.write();
        g.capture_enabled = !g.capture_enabled;
        g.capture_enabled
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
