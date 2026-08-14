use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct ModelManager {
    gate: Arc<Mutex<()>>,
}

impl ModelManager {
    pub fn gate(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.gate)
    }
}
