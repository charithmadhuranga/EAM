//! Device management commands

use crate::app::AppState;
use eam_domain::{Device, DiscoveredDevice};
use tauri::State;

/// Get all registered devices
#[tauri::command]
pub async fn get_devices(_state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    // TODO: Fetch actual devices from device manager (Phase 1)
    Ok(Vec::new())
}

/// Discover devices on the network
#[tauri::command]
pub async fn discover_devices(
    _state: State<'_, AppState>,
) -> Result<Vec<DiscoveredDevice>, String> {
    // TODO: Implement mDNS discovery (Phase 1)
    Ok(Vec::new())
}
