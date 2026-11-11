//! Runtime status commands.

use eam_domain::{RuntimeHealth, WorkspaceInfo};
use tauri::State;

use crate::app::AppState;

/// Get runtime health (uptime, device/entity counts).
#[tauri::command]
pub async fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeHealth, String> {
    let devices = state
        .manager
        .list_devices()
        .await
        .map_err(|e| e.to_string())?;
    let connected = devices
        .iter()
        .filter(|d| state.manager.status(d.id) == eam_domain::DeviceConnectionStatus::Online)
        .count();
    let entities = state.registry.lock().await.list().len();

    Ok(state.health.lock().await.snapshot(connected, entities))
}

/// Get workspace information.
#[tauri::command]
pub async fn get_workspace_info(state: State<'_, AppState>) -> Result<WorkspaceInfo, String> {
    Ok(WorkspaceInfo::new(
        "Default Workspace",
        state.manager.data_dir(),
    ))
}
