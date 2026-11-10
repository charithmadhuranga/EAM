//! Application state management

use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

/// Application state
pub struct AppState {
    /// Runtime health status
    pub health: Arc<RwLock<eam_domain::RuntimeHealth>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            health: Arc::new(RwLock::new(eam_domain::RuntimeHealth::new())),
        }
    }
}

/// Initialize application state
pub fn initialize(handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Create and store app state
    let state = AppState::default();
    handle.manage(state);

    tracing::info!("Application state initialized");

    Ok(())
}
