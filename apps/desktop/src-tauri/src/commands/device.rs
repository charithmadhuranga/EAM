//! Device management commands.
//!
//! Thin IPC layer: every command delegates to the application layer
//! (`ConnectionManager`/repositories). No business logic here (AGENTS.md).

use eam_domain::{Device, DiscoveredDevice};
use tauri::State;
use uuid::Uuid;

use crate::app::AppState;

/// List registered devices with their current connection status.
#[tauri::command]
pub async fn get_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    state
        .manager
        .list_devices()
        .await
        .map(|mut devices| {
            for device in &mut devices {
                device.connection_status = state.manager.status(device.id);
            }
            devices
        })
        .map_err(|e| e.to_string())
}

/// Register a device manually (by address).
#[tauri::command]
pub async fn register_device(
    state: State<'_, AppState>,
    host: String,
    node_name: Option<String>,
    port: Option<u16>,
) -> Result<Device, String> {
    // The adapter verifies the device speaks ESPHome before registering it.
    let probe = state
        .manager
        .discovery()
        .probe(&host)
        .await
        .map_err(|e| e.to_string())?;

    let (name, resolved_port, mac) = match probe {
        Some(found) => (found.name, found.port, found.mac_address),
        None => (
            node_name.unwrap_or_else(|| host.clone()),
            port.unwrap_or(6053),
            None,
        ),
    };

    state
        .manager
        .register(name, host, resolved_port, mac)
        .await
        .map_err(|e| e.to_string())
}

/// Register a device from a discovery result.
#[tauri::command]
pub async fn register_discovered(
    state: State<'_, AppState>,
    discovered: DiscoveredDevice,
) -> Result<Device, String> {
    state
        .manager
        .register_discovered(&discovered)
        .await
        .map_err(|e| e.to_string())
}

/// Forget a device and its entities.
#[tauri::command]
pub async fn unregister_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    state
        .manager
        .unregister(eam_domain::DeviceId(id))
        .await
        .map_err(|e| e.to_string())
}

/// Connect a device and start streaming its states.
#[tauri::command]
pub async fn connect_device(state: State<'_, AppState>, device_id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    state
        .manager
        .connect(eam_domain::DeviceId(id))
        .await
        .map_err(|e| e.to_string())
}

/// Disconnect a device (keeps its registration and history).
#[tauri::command]
pub async fn disconnect_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    state
        .manager
        .disconnect(eam_domain::DeviceId(id))
        .await
        .map_err(|e| e.to_string())
}

/// Scan the local subnet for ESPHome devices.
#[tauri::command]
pub async fn scan_devices(state: State<'_, AppState>) -> Result<Vec<DiscoveredDevice>, String> {
    let local = eam_discovery::local_ipv4().map_err(|e| e.to_string())?;
    let service = state.manager.discovery();
    let mut rx = service
        .scan_subnet(local)
        .await
        .map_err(|e| e.to_string())?;

    let mut found = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while let Ok(Some(device)) = tokio::time::timeout_at(deadline.into(), rx.recv()).await {
        found.push(device);
    }
    Ok(found)
}

/// Probe a single address for an ESPHome device (used by the add form).
#[tauri::command]
pub async fn probe_device(
    state: State<'_, AppState>,
    host: String,
) -> Result<DiscoveredDevice, String> {
    state
        .manager
        .discovery()
        .probe(&host)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no ESPHome device at {host}"))
}

/// Entities of a device.
#[tauri::command]
pub async fn get_entities(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<Vec<eam_domain::Entity>, String> {
    let id = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    let registry = state.registry.lock().await;
    Ok(registry.list_by_device(id))
}

/// Current states of every known entity (the UI polls this on load).
#[tauri::command]
pub async fn get_entity_states(
    state: State<'_, AppState>,
) -> Result<Vec<eam_domain::EntityState>, String> {
    let registry = state.registry.lock().await;
    Ok(registry
        .list()
        .into_iter()
        .filter_map(|entity| registry.state(entity.id).cloned())
        .collect())
}

/// Turn a switch on or off.
#[tauri::command]
pub async fn control_switch(
    state: State<'_, AppState>,
    entity_id: String,
    on: bool,
) -> Result<(), String> {
    let id = Uuid::parse_str(&entity_id).map_err(|e| e.to_string())?;
    let commands = eam_entity_registry::CommandService::new(state.manager.as_ref());
    commands
        .execute(
            &state.registry,
            &eam_entity_registry::CommandInput {
                entity_id: eam_domain::EntityId(id),
                command: if on {
                    eam_domain::EntityCommand::SwitchOn
                } else {
                    eam_domain::EntityCommand::SwitchOff
                },
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Set a number entity.
#[tauri::command]
pub async fn set_number(
    state: State<'_, AppState>,
    entity_id: String,
    value: f64,
) -> Result<(), String> {
    let id = Uuid::parse_str(&entity_id).map_err(|e| e.to_string())?;
    let commands = eam_entity_registry::CommandService::new(state.manager.as_ref());
    commands
        .execute(
            &state.registry,
            &eam_entity_registry::CommandInput {
                entity_id: eam_domain::EntityId(id),
                command: eam_domain::EntityCommand::SetNumber { value },
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Insert a device row directly (used by discovery flows that skip probing).
#[tauri::command]
pub async fn register_manual(
    state: State<'_, AppState>,
    node_name: String,
    host: String,
    port: u16,
) -> Result<Device, String> {
    state
        .manager
        .register(node_name, host, port, None)
        .await
        .map_err(|e| e.to_string())
}
