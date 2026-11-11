//! Connection manager tests: registration/dedup offline, full lifecycle live.

use std::sync::Arc;

use eam_device_manager::{ConnectionManager, ManagerEvent};
use eam_domain::{DeviceConnectionStatus, DeviceId, EntityCommand};
use eam_entity_registry::registry::SharedRegistry;
use eam_entity_registry::{
    CommandInput, CommandOutcome, CommandService, CommandTransport, EntityRegistry,
};
use eam_storage::open_in_memory;
use uuid::Uuid;

/// Address of the real test device, or `None` to skip live tests.
fn live_addr() -> Option<String> {
    std::env::var("EAM_LIVE_DEVICE")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

macro_rules! live {
    () => {
        match live_addr() {
            Some(addr) => addr,
            None => {
                eprintln!("SKIP: set EAM_LIVE_DEVICE=host:port for a real device");
                return;
            }
        }
    };
}

async fn manager() -> (Arc<ConnectionManager>, SharedRegistry) {
    let pool = open_in_memory().await.unwrap();
    let registry: SharedRegistry = Arc::new(tokio::sync::Mutex::new(EntityRegistry::new()));
    (
        Arc::new(ConnectionManager::new(pool, Arc::clone(&registry))),
        registry,
    )
}

#[tokio::test]
async fn register_is_idempotent_on_node_name() {
    let (manager, _registry) = manager().await;
    let first = manager
        .register("eam-test-device", "10.60.109.61", 6053, None)
        .await
        .unwrap();
    let second = manager
        .register("eam-test-device", "10.60.109.61", 6053, None)
        .await
        .unwrap();

    assert_eq!(first.id, second.id, "same node name must not duplicate");
    assert_eq!(manager.list_devices().await.unwrap().len(), 1);
}

#[tokio::test]
async fn register_emits_event_and_tracks_status() {
    let (manager, _registry) = manager().await;
    let mut events = manager.subscribe_events();
    let device = manager
        .register("node-a", "127.0.0.1", 6053, None)
        .await
        .unwrap();

    let event = events.try_recv().expect("registration event");
    assert_eq!(
        event,
        ManagerEvent::DeviceRegistered {
            device_id: device.id.0
        }
    );
    // A freshly registered device has no connection yet.
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Offline);
    assert_eq!(
        manager.status(DeviceId(Uuid::new_v4())),
        DeviceConnectionStatus::Unknown
    );
}

#[tokio::test]
async fn unregister_removes_device_entities_and_state() {
    let (manager, registry) = manager().await;
    let device = manager
        .register("node-b", "127.0.0.1", 6053, None)
        .await
        .unwrap();

    // Inject an entity directly to verify cascade behaviour.
    let entity = eam_domain::Entity::new(device.id.0, "1", "Relay", eam_domain::EntityType::Switch);
    {
        let mut guard = registry.lock().await;
        guard.insert(entity.clone());
        guard.update_state(entity.id, eam_domain::EntityValue::bool(true), false);
    }

    manager.unregister(device.id).await.unwrap();

    assert!(manager.device(device.id).await.unwrap().is_none());
    {
        let guard = registry.lock().await;
        assert!(guard.get(entity.id).is_none());
        assert!(guard.state(entity.id).is_none());
    }
}

#[tokio::test]
async fn unknown_device_connect_fails() {
    let (manager, _registry) = manager().await;
    let err = manager.connect(DeviceId(Uuid::new_v4())).await.unwrap_err();
    assert!(err.to_string().contains("not registered"));
}

#[tokio::test]
async fn connect_to_unreachable_host_fails_retryably() {
    let (manager, _registry) = manager().await;
    // Port 1 refuses connections immediately.
    let device = manager
        .register("dead", "127.0.0.1", 1, None)
        .await
        .unwrap();
    let err = manager.connect(device.id).await.unwrap_err();
    assert!(
        eam_device_manager::is_retryable(&err),
        "unreachable devices must be retryable: {err}"
    );
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Offline);
}

#[tokio::test]
async fn disconnect_unknown_device_fails() {
    let (manager, _registry) = manager().await;
    let err = manager
        .disconnect(DeviceId(Uuid::new_v4()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not registered"));
}

// ---- live tests ----

#[tokio::test]
async fn live_full_lifecycle() {
    let addr = live!();
    let (manager, registry) = manager().await;

    // Register by hand (mirrors manual registration in the UI).
    let host = addr.split(':').next().unwrap().to_string();
    let device = manager
        .register("eam-test-device", &host, 6053, None)
        .await
        .unwrap();

    manager.connect(device.id).await.unwrap();
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Online);

    // Identity and entities must be persisted, not just held in memory.
    let stored = manager.device(device.id).await.unwrap().unwrap();
    assert_eq!(stored.firmware_version.as_deref(), Some("2026.10.2"));
    assert_eq!(stored.connection_status, DeviceConnectionStatus::Online);
    assert_eq!(registry.lock().await.list_by_device(device.id.0).len(), 7);

    // States must stream in from the device background task.
    let mut states = registry.lock().await.subscribe_states();

    let mut received = None;
    for _ in 0..25 {
        if let Ok(Ok(update)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), states.recv()).await
        {
            received = Some(update.entity_id);
            break;
        }
    }
    assert!(
        received.is_some(),
        "no state update streamed from the device"
    );

    // Commands flow through the registry + transport.
    let switch = registry
        .lock()
        .await
        .list_by_type(eam_domain::EntityType::Switch)
        .pop()
        .expect("switch present");
    let commands = CommandService::new(&manager);
    let outcome = commands
        .execute(
            &registry,
            &CommandInput {
                entity_id: switch.id,
                command: EntityCommand::SwitchOn,
            },
        )
        .await
        .unwrap();
    assert_eq!(outcome, CommandOutcome::Observed);

    // Commands are rejected while the device is offline.
    manager.disconnect(device.id).await.unwrap();
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Offline);
    let err = commands
        .execute(
            &registry,
            &CommandInput {
                entity_id: switch.id,
                command: EntityCommand::SwitchOn,
            },
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not connected"), "err: {err}");

    // Reconnect brings everything back.
    manager.connect(device.id).await.unwrap();
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Online);
}

#[tokio::test]
async fn live_state_updates_reach_subscriber() {
    let addr = live!();
    let (manager, registry) = manager().await;
    let host = addr.split(':').next().unwrap().to_string();
    let device = manager
        .register("eam-test-device", &host, 6053, None)
        .await
        .unwrap();
    manager.connect(device.id).await.unwrap();

    let mut rx = registry.lock().await.subscribe_states();
    // Toggle the switch through the command transport; the device echoes.
    let switch = registry
        .lock()
        .await
        .list_by_type(eam_domain::EntityType::Switch)
        .pop()
        .unwrap();
    let transport = &manager;
    transport
        .switch_command(device.id.0, switch.native_entity_key.parse().unwrap(), true)
        .await
        .unwrap();

    let mut seen = false;
    for _ in 0..30 {
        if let Ok(Ok(update)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await
        {
            if update.entity_id == switch.id {
                seen = true;
                break;
            }
        }
    }
    assert!(seen, "no state update for the toggled switch");
    manager.disconnect(device.id).await.unwrap();
}

/// Ensures the state map is keyed by device so multiple devices stay isolated.
#[tokio::test]
async fn registry_isolates_devices() {
    let d1 = Uuid::new_v4();
    let d2 = Uuid::new_v4();
    let mut registry = EntityRegistry::new();
    let e1 = eam_domain::Entity::new(d1, "1", "A", eam_domain::EntityType::Switch);
    let e2 = eam_domain::Entity::new(d2, "1", "A", eam_domain::EntityType::Switch);
    registry.insert(e1.clone());
    registry.insert(e2.clone());
    assert_eq!(registry.list_by_device(d1).len(), 1);
    assert_eq!(registry.list_by_device(d2).len(), 1);
    assert_eq!(registry.get_by_key(d1, "1").unwrap().id, e1.id);
    assert_eq!(registry.get_by_key(d2, "1").unwrap().id, e2.id);
    assert!(registry.get(e1.id).is_some());
}

#[tokio::test]
async fn live_reconnect_all_restores_disconnected_device() {
    let addr = live!();
    let host = addr.split(':').next().unwrap().to_string();
    let (manager, registry) = manager().await;
    let device = manager
        .register("eam-test-device", &host, 6053, None)
        .await
        .unwrap();

    manager.connect(device.id).await.unwrap();
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Online);

    // Network interruption: disconnect abruptly.
    manager.disconnect(device.id).await.unwrap();
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Offline);

    // The reconnect loop (and reconnect_all in particular) must restore it.
    manager
        .reconnect_all()
        .await
        .expect("reconnect_all succeeded");
    assert_eq!(manager.status(device.id), DeviceConnectionStatus::Online);
    assert_eq!(registry.lock().await.list_by_device(device.id.0).len(), 7);
    manager.disconnect(device.id).await.unwrap();
}
