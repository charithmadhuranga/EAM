//! In-memory registry of known entities, with their current state.
//!
//! The registry is the write path for live states: the connection manager
//! feeds it from device subscriptions, the command service reads it to verify
//! commands, and the UI subscribes to [`StateUpdate`]s.

use std::collections::HashMap;

use eam_domain::{Entity, EntityId, EntityState, EntityType, EntityValue, StateQuality};
use tokio::sync::broadcast;
use tracing::debug;

/// Shared, async-aware handle to a registry.
///
/// Commands and streaming both lock the registry, so it must be a
/// `tokio::sync::Mutex` — holding a `parking_lot` guard across an `await`
/// deadlocks against the state stream.
pub type SharedRegistry = std::sync::Arc<tokio::sync::Mutex<EntityRegistry>>;

/// Capacity of the state-update channel per subscriber.
pub const STATE_CHANNEL_CAPACITY: usize = 512;

/// A state change broadcast to subscribers.
#[derive(Debug, Clone, PartialEq)]
pub struct StateUpdate {
    /// Entity that changed.
    pub entity_id: EntityId,
    /// New value.
    pub value: EntityValue,
    /// When the state was received.
    pub received_at: chrono::DateTime<chrono::Utc>,
}

/// Entities and their current state for one device.
#[derive(Debug, Default)]
struct DeviceEntities {
    /// Entities by domain id.
    entities: HashMap<EntityId, Entity>,
    /// Current state by entity id.
    states: HashMap<EntityId, EntityState>,
}

/// Registry of all known entities and their live state.
pub struct EntityRegistry {
    /// Per-device entity and state maps.
    devices: HashMap<uuid::Uuid, DeviceEntities>,
    /// Broadcasts state changes to subscribers.
    updates: broadcast::Sender<StateUpdate>,
}

impl EntityRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        let (updates, _) = broadcast::channel(STATE_CHANNEL_CAPACITY);
        Self {
            devices: HashMap::new(),
            updates,
        }
    }

    /// Subscribe to state updates.
    #[must_use]
    pub fn subscribe_states(&self) -> broadcast::Receiver<StateUpdate> {
        self.updates.subscribe()
    }

    /// Replace the entity list of a device with `entities`.
    ///
    /// Existing state is preserved for entities that remain, and dropped for
    /// entities that disappeared from the device.
    pub fn refresh_entities(&mut self, device_id: uuid::Uuid, entities: Vec<Entity>) {
        let entry = self.devices.entry(device_id).or_default();
        let mut kept = HashMap::with_capacity(entities.len());
        for entity in &entities {
            if let Some(previous) = entry.states.get(&entity.id) {
                debug!(
                    entity = %entity.id,
                    name = %entity.name,
                    "entity state preserved across refresh"
                );
                kept.insert(entity.id, previous.clone());
            }
        }
        entry.states = kept;
        entry.entities = entities.into_iter().map(|e| (e.id, e)).collect();
    }

    /// Register one entity.
    pub fn insert(&mut self, entity: Entity) {
        self.devices
            .entry(entity.device_id)
            .or_default()
            .entities
            .insert(entity.id, entity);
    }

    /// Look up an entity.
    #[must_use]
    pub fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.devices
            .values()
            .flat_map(|d| d.entities.get(&entity_id))
            .next()
    }

    /// Look up the entity matching a protocol key on a device.
    #[must_use]
    pub fn get_by_key(&self, device_id: uuid::Uuid, native_entity_key: &str) -> Option<&Entity> {
        self.devices.get(&device_id).and_then(|d| {
            d.entities
                .values()
                .find(|e| e.native_entity_key == native_entity_key)
        })
    }

    /// All entities of a device.
    #[must_use]
    pub fn list_by_device(&self, device_id: uuid::Uuid) -> Vec<Entity> {
        self.devices
            .get(&device_id)
            .map(|d| d.entities.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Every known entity.
    #[must_use]
    pub fn list(&self) -> Vec<Entity> {
        self.devices
            .values()
            .flat_map(|d| d.entities.values().cloned())
            .collect()
    }

    /// Every entity of a given type (e.g. all switches).
    #[must_use]
    pub fn list_by_type(&self, entity_type: EntityType) -> Vec<Entity> {
        self.list()
            .into_iter()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Current state of an entity.
    #[must_use]
    pub fn state(&self, entity_id: EntityId) -> Option<&EntityState> {
        self.devices
            .values()
            .flat_map(|d| d.states.get(&entity_id))
            .next()
    }

    /// Record a new state and notify subscribers.
    ///
    /// The value is stamped with `received_at`; quality is derived from the
    /// `missing_state` flag the protocol sends.
    pub fn update_state(
        &mut self,
        entity_id: EntityId,
        value: EntityValue,
        missing_state: bool,
    ) -> StateUpdate {
        self.update_state_at(entity_id, value, missing_state, chrono::Utc::now())
    }

    /// Record a new state with an explicit receive time (for deterministic tests).
    pub fn update_state_at(
        &mut self,
        entity_id: EntityId,
        value: EntityValue,
        missing_state: bool,
        received_at: chrono::DateTime<chrono::Utc>,
    ) -> StateUpdate {
        let quality = if missing_state {
            StateQuality::Unknown
        } else {
            StateQuality::Good
        };

        if let Some(device) = self
            .devices
            .values_mut()
            .find(|d| d.entities.contains_key(&entity_id))
        {
            device.states.insert(
                entity_id,
                EntityState {
                    entity_id,
                    value: value.clone(),
                    raw_value: None,
                    source_timestamp: None,
                    received_at,
                    quality,
                },
            );
        }

        let update = StateUpdate {
            entity_id,
            value,
            received_at,
        };
        // Lagging subscribers drop old updates rather than stalling the writer.
        let _ = self.updates.send(update.clone());
        update
    }

    /// Drop all entities and states of a device.
    pub fn remove_device(&mut self, device_id: uuid::Uuid) {
        self.devices.remove(&device_id);
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn registry_with_switch(device_id: Uuid) -> (EntityRegistry, Entity) {
        let mut registry = EntityRegistry::new();
        let entity = Entity::new(device_id, "1040761112", "Test Switch", EntityType::Switch);
        registry.insert(entity.clone());
        (registry, entity)
    }

    #[test]
    fn refresh_entities_keeps_state_of_surviving_entities() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        registry.update_state(entity.id, EntityValue::bool(true), false);

        // A refresh with the same key must not lose the state.
        registry.refresh_entities(device_id, vec![entity.clone()]);
        let state = registry.state(entity.id).expect("state kept");
        assert_eq!(state.value, EntityValue::bool(true));
    }

    #[test]
    fn refresh_entities_drops_state_of_removed_entities() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        registry.update_state(entity.id, EntityValue::bool(true), false);

        // Device no longer reports the switch.
        registry.refresh_entities(device_id, vec![]);
        assert!(registry.state(entity.id).is_none());
        assert!(registry.get(entity.id).is_none());
    }

    #[test]
    fn state_updates_reach_subscribers() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        let mut rx = registry.subscribe_states();

        registry.update_state(entity.id, EntityValue::bool(true), false);
        let update = rx.try_recv().expect("update delivered");
        assert_eq!(update.entity_id, entity.id);
        assert_eq!(update.value, EntityValue::bool(true));
    }

    #[test]
    fn missing_state_marks_quality_unknown() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        registry.update_state(entity.id, EntityValue::number(f64::NAN), true);
        assert_eq!(
            registry.state(entity.id).unwrap().quality,
            StateQuality::Unknown
        );
    }

    #[test]
    fn lookup_by_key_and_type() {
        let device_id = Uuid::new_v4();
        let (registry, _entity) = registry_with_switch(device_id);
        assert!(registry.get_by_key(device_id, "1040761112").is_some());
        assert!(registry.get_by_key(device_id, "nope").is_none());
        assert_eq!(registry.list_by_type(EntityType::Switch).len(), 1);
        assert_eq!(registry.list_by_type(EntityType::Number).len(), 0);
        assert_eq!(registry.list_by_device(device_id).len(), 1);
    }

    #[test]
    fn remove_device_clears_entities_and_state() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        registry.update_state(entity.id, EntityValue::bool(true), false);
        registry.remove_device(device_id);
        assert!(registry.get(entity.id).is_none());
        assert!(registry.state(entity.id).is_none());
    }

    #[test]
    fn update_state_at_is_deterministic() {
        let device_id = Uuid::new_v4();
        let (mut registry, entity) = registry_with_switch(device_id);
        let at = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        registry.update_state_at(entity.id, EntityValue::number(7.0), false, at);
        assert_eq!(registry.state(entity.id).unwrap().received_at, at);
    }
}
