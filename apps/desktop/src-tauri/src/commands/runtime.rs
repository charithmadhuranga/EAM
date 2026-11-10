//! Runtime status commands

use crate::app::AppState;
use eam_domain::{RuntimeHealth, WorkspaceInfo};
use tauri::State;

/// Get runtime health status
#[tauri::command]
pub async fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeHealth, String> {
    let health = state.health.read().await;
    Ok(health.clone())
}

/// Get workspace information
#[tauri::command]
pub async fn get_workspace_info() -> Result<WorkspaceInfo, String> {
    // TODO: Fetch actual workspace info
    Ok(WorkspaceInfo::new(
        "Default Workspace",
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
    ))
}
