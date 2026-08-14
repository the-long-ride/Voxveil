use std::sync::{Mutex, MutexGuard};

use super::dto::AppViewState;

pub struct AppState {
    inner: Mutex<AppViewState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self { inner: Mutex::new(AppViewState::default()) }
    }
}

impl AppState {
    pub fn lock(&self) -> Result<MutexGuard<'_, AppViewState>, String> {
        self.inner.lock().map_err(|_| "application state lock is poisoned".to_string())
    }
}
