//! End-to-end tests against a real ESPHome device.
//!
//! These run when `EAM_LIVE_DEVICE` (address `host:port`) is set; otherwise
//! each test exits early so CI stays green without hardware.
//!
//! ```sh
//! EAM_LIVE_DEVICE=10.60.109.61:6053 cargo test -p eam-esphome-api --test live
//! ```

use eam_esphome_api::{ApiClient, Error};
use uuid::Uuid;

/// Address from `EAM_LIVE_DEVICE`, e.g. `10.60.109.61:6053`.
/// Returns `None` when the variable is unset/empty.
fn live_addr() -> Option<String> {
    std::env::var("EAM_LIVE_DEVICE")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

/// Skip helper: prints a notice and returns early when no device is configured.
macro_rules! live {
    () => {
        match live_addr() {
            Some(addr) => addr,
            None => {
                eprintln!("SKIP: set EAM_LIVE_DEVICE=host:port to run against a real device");
                return;
            }
        }
    };
}

/// Fetch the entity type of the first entity matching `predicate`.
fn find_key(entities: &[eam_domain::Entity], entity_type: eam_domain::EntityType) -> u32 {
    entities
        .iter()
        .find(|e| e.entity_type == entity_type)
        .unwrap_or_else(|| panic!("test firmware declares a {entity_type:?}"))
        .native_entity_key
        .parse()
        .expect("entity key must be numeric")
}

/// Read state updates until `key` reports `expected`, or give up.
async fn wait_for_state(
    client: &mut ApiClient,
    key: u32,
    expected: eam_domain::EntityValue,
) -> bool {
    for _ in 0..40 {
        match client.next_state().await {
            Ok(update) if update.key == key => {
                if update.value == expected {
                    return true;
                }
            }
            Ok(_) => {}
            Err(e) if e.is_recoverable() => return false,
            Err(e) => panic!("unexpected error reading state: {e}"),
        }
    }
    false
}

#[tokio::test]
async fn handshake_and_device_info() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();

    let (major, minor) = client.api_version();
    assert!(major >= 1, "unexpected major version {major}.{minor}");

    let info = client.device_info().await.unwrap();
    assert_eq!(info.name, "eam-test-device");
    assert!(
        info.model.to_lowercase().contains("esp32"),
        "unexpected board model: {}",
        info.model
    );
    assert!(!info.esphome_version.is_empty());

    // Repeating the request must give the same identity.
    let info2 = client.device_info().await.unwrap();
    assert_eq!(info.name, info2.name, "device identity must be stable");
    assert_eq!(info.mac_address, info2.mac_address);

    client.ping().await.unwrap();
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn entity_list_has_expected_types() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();
    let device_id = Uuid::new_v4();
    let entities = client.list_entities(device_id).await.unwrap();

    // The test firmware declares exactly seven entities.
    let mut kinds: Vec<String> = entities
        .iter()
        .map(|e| format!("{:?}", e.entity_type))
        .collect();
    kinds.sort();
    assert_eq!(
        kinds,
        [
            "BinarySensor",
            "Light",
            "Number",
            "Sensor",
            "Sensor",
            "Switch",
            "TextSensor"
        ],
        "entity list did not match the test firmware"
    );

    assert!(
        entities
            .iter()
            .all(|e| e.native_entity_key.parse::<u32>().is_ok()),
        "every entity key must parse as the protocol's numeric key"
    );

    // The switch and number from the firmware must be present with capabilities.
    let number = entities
        .iter()
        .find(|e| e.entity_type == eam_domain::EntityType::Number)
        .unwrap();
    assert_eq!(number.capabilities["min_value"], 0.0);
    assert_eq!(number.capabilities["max_value"], 100.0);
    assert_eq!(number.capabilities["step"], 1.0);
}

#[tokio::test]
async fn states_stream_after_subscribe() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();
    let device_id = Uuid::new_v4();
    let entities = client.list_entities(device_id).await.unwrap();

    client.subscribe_states().await.unwrap();
    client.set_frame_timeout(std::time::Duration::from_secs(5));

    // Devices publish a burst of states shortly after subscribing.
    let mut seen = 0;
    let mut keys = std::collections::HashSet::new();
    while seen < 3 {
        let update = client.next_state().await.unwrap();
        assert!(update.key != 0, "state update must carry a key");
        keys.insert(update.key);
        seen += 1;
    }

    // Every reported key must belong to an entity we listed.
    let known: std::collections::HashSet<u32> = entities
        .iter()
        .map(|e| e.native_entity_key.parse().unwrap())
        .collect();
    assert!(
        keys.iter().all(|k| known.contains(k)),
        "device sent states for entities it never listed"
    );
}

#[tokio::test]
async fn switch_command_toggles_state() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();
    let device_id = Uuid::new_v4();
    let entities = client.list_entities(device_id).await.unwrap();
    client.subscribe_states().await.unwrap();
    client.set_frame_timeout(std::time::Duration::from_secs(5));

    let key = find_key(&entities, eam_domain::EntityType::Switch);

    // Command ON, then confirm the device reports ON back.
    client.switch_command(key, true).await.unwrap();
    assert!(
        wait_for_state(&mut client, key, eam_domain::EntityValue::bool(true)).await,
        "device never reported switch ON after command"
    );

    // Command OFF, then confirm OFF.
    client.switch_command(key, false).await.unwrap();
    assert!(
        wait_for_state(&mut client, key, eam_domain::EntityValue::bool(false)).await,
        "device never reported switch OFF after command"
    );
}

#[tokio::test]
async fn number_command_updates_value() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();
    let device_id = Uuid::new_v4();
    let entities = client.list_entities(device_id).await.unwrap();
    client.subscribe_states().await.unwrap();
    client.set_frame_timeout(std::time::Duration::from_secs(5));

    let key = find_key(&entities, eam_domain::EntityType::Number);

    client.number_command(key, 77.0).await.unwrap();

    // The template number is optimistic: the device echoes the new value.
    assert!(
        wait_for_state(&mut client, key, eam_domain::EntityValue::number(77.0)).await,
        "device never reported number = 77 after command"
    );
}

#[tokio::test]
async fn reconnect_after_connection_drop() {
    let addr = live!();
    let device_id = Uuid::new_v4();

    // First connection: list entities, then drop abruptly (no graceful close).
    {
        let mut client = ApiClient::connect(&addr).await.unwrap();
        let first = client.list_entities(device_id).await.unwrap();
        assert!(!first.is_empty());
    }

    // Reconnecting must succeed with identical entity keys.
    let mut client = ApiClient::connect(&addr).await.unwrap();
    let again = client.list_entities(device_id).await.unwrap();
    assert_eq!(
        again.len(),
        7,
        "entity list must be identical after reconnect"
    );
    client.ping().await.unwrap();
}

#[tokio::test]
async fn unknown_switch_key_is_rejected() {
    let addr = live!();
    let mut client = ApiClient::connect(&addr).await.unwrap();
    // Key 4_000_000 is not in the entity list, so the command must fail locally.
    let err = client.switch_command(4_000_000, true).await.unwrap_err();
    assert!(matches!(err, Error::UnknownKey(4_000_000)));
}
