//! Device management commands

use eam_domain::{Device, DeviceId, DiscoveredDevice};
use tauri::State;
use crate::app::AppState;

/// Get all registered devices
#[tauri::command]
pub async fn get_devices(
    state: State<'_, AppState>,
) -> Result<Vec<Device>, String> {
    let health = state.health.read().await;
    
    // TODO: Fetch actual devices from device manager
    let devices = Vec::new();
    
    Ok(devices)
}

/// Discover devices on the network
#[tauri::command]
pub async fn discover_devices(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredDevice>, String> {
    // TODO: Implement mDNS discovery
    let discovered = Vec::new();
    
    Ok(discovered)
}
