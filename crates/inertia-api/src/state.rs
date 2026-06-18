use std::sync::Arc;

use inertia_core::Engine;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Mutex<Engine>>,
    pub shutdown: Arc<Notify>,
}
