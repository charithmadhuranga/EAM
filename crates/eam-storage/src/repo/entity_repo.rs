//! Entity repository: persistence for `eam_domain::Entity` and current state.

use eam_domain::{Entity, EntityId, EntityState, StateQuality};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// CRUD access to the `entities` and `entity_current_states` tables.
pub struct EntityRepository {
    pool: SqlitePool,
}

impl EntityRepository {
    /// Create a repository bound to `pool`.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert one entity.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure (including duplicate
    /// `device_id` + `native_entity_key` pairs).
    pub async fn create(&self, entity: &Entity) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO entities
                (id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                 device_class, state_class, capabilities_json, raw_metadata_json,
                 availability, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entity.id.0.to_string())
        .bind(entity.device_id.to_string())
        .bind(&entity.native_entity_key)
        .bind(&entity.object_id)
        .bind(&entity.name)
        .bind(serde_json::to_string(&entity.entity_type).expect("entity type serializes"))
        .bind(&entity.unit)
        .bind(&entity.icon)
        .bind(&entity.device_class)
        .bind(&entity.state_class)
        .bind(entity.capabilities.to_string())
        .bind(entity.raw_metadata.to_string())
        .bind(serde_json::to_string(&entity.availability).expect("availability serializes"))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        Ok(())
    }

    /// Insert every entity discovered for a device in one transaction.
    ///
    /// Existing rows for the same `(device_id, native_entity_key)` pair are
    /// refreshed, keeping their domain id so states and automations stay valid.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn replace_for_device(
        &self,
        device_id: Uuid,
        entities: &[Entity],
    ) -> Result<Vec<Entity>> {
        let mut tx = self.pool.begin().await.map_err(StorageError::Database)?;
        let mut stored = Vec::with_capacity(entities.len());
        let now = chrono::Utc::now().to_rfc3339();

        // Keys the device no longer reports must not linger (they were removed
        // from the firmware, or disabled).
        let kept: Vec<String> = entities
            .iter()
            .map(|e| e.native_entity_key.clone())
            .collect();
        let existing: Vec<String> =
            sqlx::query_scalar("SELECT native_entity_key FROM entities WHERE device_id = ?")
                .bind(device_id.to_string())
                .fetch_all(&mut *tx)
                .await
                .map_err(StorageError::Database)?;
        for stale in existing.iter().filter(|k| !kept.contains(k)) {
            sqlx::query("DELETE FROM entities WHERE device_id = ? AND native_entity_key = ?")
                .bind(device_id.to_string())
                .bind(stale)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::Database)?;
        }

        for entity in entities {
            // Keep the existing id when the entity was already known.
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT id FROM entities WHERE device_id = ? AND native_entity_key = ?",
            )
            .bind(device_id.to_string())
            .bind(&entity.native_entity_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(StorageError::Database)?;

            let persisted = match existing {
                Some(id) => {
                    let id = EntityId(
                        Uuid::parse_str(&id)
                            .map_err(|e| StorageError::decode_str(e.to_string()))?,
                    );
                    Entity {
                        id,
                        ..entity.clone()
                    }
                }
                None => entity.clone(),
            };

            sqlx::query(
                "INSERT INTO entities
                    (id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                     device_class, state_class, capabilities_json, raw_metadata_json,
                     availability, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(device_id, native_entity_key) DO UPDATE SET
                    object_id = excluded.object_id,
                    name = excluded.name,
                    entity_type = excluded.entity_type,
                    unit = excluded.unit,
                    icon = excluded.icon,
                    device_class = excluded.device_class,
                    state_class = excluded.state_class,
                    capabilities_json = excluded.capabilities_json,
                    raw_metadata_json = excluded.raw_metadata_json,
                    availability = excluded.availability,
                    updated_at = excluded.updated_at",
            )
            .bind(persisted.id.0.to_string())
            .bind(device_id.to_string())
            .bind(&persisted.native_entity_key)
            .bind(&persisted.object_id)
            .bind(&persisted.name)
            .bind(serde_json::to_string(&persisted.entity_type).expect("serializes"))
            .bind(&persisted.unit)
            .bind(&persisted.icon)
            .bind(&persisted.device_class)
            .bind(&persisted.state_class)
            .bind(persisted.capabilities.to_string())
            .bind(persisted.raw_metadata.to_string())
            .bind(serde_json::to_string(&persisted.availability).expect("serializes"))
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(StorageError::Database)?;

            stored.push(persisted);
        }

        tx.commit().await.map_err(StorageError::Database)?;
        Ok(stored)
    }

    /// Fetch an entity by id.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn get(&self, id: EntityId) -> Result<Option<Entity>> {
        let row: Option<EntityRow> = sqlx::query_as(
            "SELECT id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                    device_class, state_class, capabilities_json, raw_metadata_json,
                    availability, created_at, updated_at
             FROM entities WHERE id = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        row.map(entity_from_row).transpose()
    }

    /// Fetch the entity for a protocol key within a device.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn get_by_key(
        &self,
        device_id: Uuid,
        native_entity_key: &str,
    ) -> Result<Option<Entity>> {
        let row: Option<EntityRow> = sqlx::query_as(
            "SELECT id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                    device_class, state_class, capabilities_json, raw_metadata_json,
                    availability, created_at, updated_at
             FROM entities WHERE device_id = ? AND native_entity_key = ?",
        )
        .bind(device_id.to_string())
        .bind(native_entity_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        row.map(entity_from_row).transpose()
    }

    /// List entities for a device.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn list_by_device(&self, device_id: Uuid) -> Result<Vec<Entity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                    device_class, state_class, capabilities_json, raw_metadata_json,
                    availability, created_at, updated_at
             FROM entities WHERE device_id = ? ORDER BY name ASC",
        )
        .bind(device_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        rows.into_iter().map(entity_from_row).collect()
    }

    /// List every entity across all devices.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn list(&self) -> Result<Vec<Entity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT id, device_id, native_entity_key, object_id, name, entity_type, unit, icon,
                    device_class, state_class, capabilities_json, raw_metadata_json,
                    availability, created_at, updated_at
             FROM entities ORDER BY device_id, name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        rows.into_iter().map(entity_from_row).collect()
    }

    /// Upsert the current state of an entity.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn set_current_state(&self, state: &EntityState) -> Result<()> {
        let value_json = serde_json::to_string(&state.value)
            .map_err(|e| StorageError::decode_str(e.to_string()))?;
        sqlx::query(
            "INSERT INTO entity_current_states
                (entity_id, value_type, value_json, source_timestamp, received_at, quality)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(entity_id) DO UPDATE SET
                value_type = excluded.value_type,
                value_json = excluded.value_json,
                source_timestamp = excluded.source_timestamp,
                received_at = excluded.received_at,
                quality = excluded.quality",
        )
        .bind(state.entity_id.0.to_string())
        .bind(crate::repo::value_tag::value_tag(&state.value))
        .bind(&value_json)
        .bind(
            state
                .source_timestamp
                .as_ref()
                .map(chrono::DateTime::to_rfc3339),
        )
        .bind(state.received_at.to_rfc3339())
        .bind(
            serde_json::to_string(&state.quality)
                .map_err(|e| StorageError::decode_str(e.to_string()))?,
        )
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        Ok(())
    }

    /// Read the current state of an entity.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn get_current_state(&self, id: EntityId) -> Result<Option<EntityState>> {
        let row: Option<(String, String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT value_type, value_json, source_timestamp, received_at, quality
             FROM entity_current_states WHERE entity_id = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;

        let Some((_value_type, value_json, source_timestamp, received_at, quality)) = row else {
            return Ok(None);
        };

        let value: eam_domain::EntityValue = serde_json::from_str(&value_json)
            .map_err(|e| StorageError::decode_str(e.to_string()))?;
        let received_at = chrono::DateTime::parse_from_rfc3339(&received_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| StorageError::decode_str(e.to_string()))?;
        let quality: StateQuality =
            serde_json::from_str(&quality).map_err(|e| StorageError::decode_str(e.to_string()))?;

        Ok(Some(EntityState {
            entity_id: id,
            value,
            raw_value: None,
            source_timestamp: source_timestamp
                .as_deref()
                .map(chrono::DateTime::parse_from_rfc3339)
                .transpose()
                .map(|d| d.map(|d| d.with_timezone(&chrono::Utc)))
                .map_err(|e| StorageError::decode_str(e.to_string()))?,
            received_at,
            quality,
        }))
    }

    /// Delete every entity of a device (cascades to states).
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn delete_for_device(&self, device_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM entities WHERE device_id = ?")
            .bind(device_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(StorageError::Database)?;
        Ok(())
    }
}

/// Row shape for the `entities` table.
type EntityRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
);

fn entity_from_row(row: EntityRow) -> Result<Entity> {
    let (
        id,
        device_id,
        native_entity_key,
        object_id,
        name,
        entity_type,
        unit,
        icon,
        device_class,
        state_class,
        capabilities_json,
        raw_metadata_json,
        availability,
        created_at,
        updated_at,
    ) = row;

    Ok(Entity {
        id: EntityId(Uuid::parse_str(&id).map_err(|e| StorageError::decode_str(e.to_string()))?),
        device_id: Uuid::parse_str(&device_id)
            .map_err(|e| StorageError::decode_str(e.to_string()))?,
        native_entity_key,
        object_id,
        name,
        entity_type: serde_json::from_str(&entity_type)
            .map_err(|e| StorageError::decode_str(e.to_string()))?,
        unit,
        icon,
        state_class,
        device_class,
        capabilities: serde_json::from_str(&capabilities_json)
            .map_err(|e| StorageError::decode_str(e.to_string()))?,
        raw_metadata: serde_json::from_str(&raw_metadata_json)
            .map_err(|e| StorageError::decode_str(e.to_string()))?,
        availability: serde_json::from_str(&availability)
            .map_err(|e| StorageError::decode_str(e.to_string()))?,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

/// Parse an RFC 3339 timestamp.
fn parse_ts(value: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| StorageError::decode_str(format!("bad timestamp {value}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::repo::device_repo::DeviceRepository;
    use crate::repo::value_tag::value_tag;
    use eam_domain::{Device, DeviceConnectionStatus, EntityState};
    use uuid::Uuid;

    async fn fixture() -> (DeviceRepository, EntityRepository, Uuid) {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let entities = EntityRepository::new(pool);
        let device = Device::new("entity-fixture");
        devices.create(&device).await.unwrap();
        let device_id = device.id.0;
        (devices, entities, device_id)
    }

    fn entity(device_id: Uuid, key: &str, name: &str) -> Entity {
        Entity::new(device_id, key, name, eam_domain::EntityType::Switch)
    }

    #[tokio::test]
    async fn create_and_get_by_id() {
        let (_d, repo, device_id) = fixture().await;
        let e = entity(device_id, "1", "Relay");
        repo.create(&e).await.unwrap();
        let fetched = repo.get(e.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Relay");
        assert_eq!(fetched.entity_type, eam_domain::EntityType::Switch);
        assert_eq!(fetched.native_entity_key, "1");
    }

    #[tokio::test]
    async fn duplicate_device_and_key_rejected() {
        let (_d, repo, device_id) = fixture().await;
        repo.create(&entity(device_id, "1", "A")).await.unwrap();
        let err = repo.create(&entity(device_id, "1", "B")).await.unwrap_err();
        assert!(matches!(err, StorageError::Database(_)));
    }

    #[tokio::test]
    async fn same_key_on_different_devices_allowed() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let repo = EntityRepository::new(pool);
        let d1 = Device::new("dev-1");
        let d2 = Device::new("dev-2");
        devices.create(&d1).await.unwrap();
        devices.create(&d2).await.unwrap();

        repo.create(&entity(d1.id.0, "1", "A")).await.unwrap();
        repo.create(&entity(d2.id.0, "1", "B")).await.unwrap();
        assert_eq!(repo.list_by_device(d2.id.0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_by_key_lookup() {
        let (_d, repo, device_id) = fixture().await;
        repo.create(&entity(device_id, "1040761112", "Relay"))
            .await
            .unwrap();
        let found = repo
            .get_by_key(device_id, "1040761112")
            .await
            .unwrap()
            .expect("found");
        assert_eq!(found.name, "Relay");
        assert!(repo.get_by_key(device_id, "nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_by_device_and_list_all() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let repo = EntityRepository::new(pool);
        let d = Device::new("dev");
        devices.create(&d).await.unwrap();
        repo.create(&entity(d.id.0, "1", "A")).await.unwrap();
        repo.create(&entity(d.id.0, "2", "B")).await.unwrap();
        assert_eq!(repo.list_by_device(d.id.0).await.unwrap().len(), 2);
        assert_eq!(repo.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn replace_for_device_is_idempotent_and_keeps_ids() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let repo = EntityRepository::new(pool);
        let d = Device::new("replace");
        devices.create(&d).await.unwrap();

        let first = repo
            .replace_for_device(d.id.0, &[entity(d.id.0, "1", "A")])
            .await
            .unwrap();
        assert_eq!(first.len(), 1);

        // Same key again: the stored entity keeps its id, but is refreshed.
        let mut renamed = entity(d.id.0, "1", "A renamed");
        renamed.capabilities = serde_json::json!({"max_value": 100});
        let second = repo.replace_for_device(d.id.0, &[renamed]).await.unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, first[0].id, "id must survive a refresh");
        let stored = repo.get_by_key(d.id.0, "1").await.unwrap().unwrap();
        assert_eq!(stored.name, "A renamed");
        assert_eq!(stored.capabilities["max_value"], 100);
    }

    #[tokio::test]
    async fn replace_for_device_drops_missing_entities() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let repo = EntityRepository::new(pool);
        let d = Device::new("prune");
        devices.create(&d).await.unwrap();
        repo.create(&entity(d.id.0, "1", "A")).await.unwrap();
        repo.create(&entity(d.id.0, "2", "B")).await.unwrap();

        // Only key "1" is reported now: key "2" was removed from the device.
        repo.replace_for_device(d.id.0, &[entity(d.id.0, "1", "A")])
            .await
            .unwrap();
        let remaining = repo.list_by_device(d.id.0).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].native_entity_key, "1");
    }

    #[tokio::test]
    async fn current_state_round_trip_for_each_value_type() {
        let (_d, repo, device_id) = fixture().await;
        let e = entity(device_id, "1", "State");
        repo.create(&e).await.unwrap();

        for value in [
            eam_domain::EntityValue::number(21.5),
            eam_domain::EntityValue::bool(false),
            eam_domain::EntityValue::text("on"),
            eam_domain::EntityValue::Json(serde_json::json!({"a": 1})),
            eam_domain::EntityValue::Unknown,
        ] {
            let tag = value_tag(&value);
            repo.set_current_state(&EntityState::new(e.id, value.clone()))
                .await
                .unwrap();
            let stored = repo.get_current_state(e.id).await.unwrap().unwrap();
            assert_eq!(stored.value, value);
            let row: (String,) =
                sqlx::query_as("SELECT value_type FROM entity_current_states WHERE entity_id = ?")
                    .bind(e.id.0.to_string())
                    .fetch_one(&repo.pool)
                    .await
                    .unwrap();
            assert_eq!(row.0, tag);
        }
    }

    #[tokio::test]
    async fn current_state_overwrites_previous_value() {
        let (_d, repo, device_id) = fixture().await;
        let e = entity(device_id, "1", "Overwrite");
        repo.create(&e).await.unwrap();
        repo.set_current_state(&EntityState::new(
            e.id,
            eam_domain::EntityValue::number(1.0),
        ))
        .await
        .unwrap();
        repo.set_current_state(&EntityState::new(
            e.id,
            eam_domain::EntityValue::number(2.0),
        ))
        .await
        .unwrap();

        let stored = repo.get_current_state(e.id).await.unwrap().unwrap();
        assert_eq!(stored.value.as_number(), Some(2.0));
    }

    #[tokio::test]
    async fn unknown_entity_has_no_state() {
        let (_d, repo, _device_id) = fixture().await;
        let missing = EntityId(Uuid::new_v4());
        assert!(repo.get_current_state(missing).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_for_device_removes_entities_and_states() {
        let (_d, repo, device_id) = fixture().await;
        let e = entity(device_id, "1", "Gone");
        repo.create(&e).await.unwrap();
        repo.set_current_state(&EntityState::new(e.id, eam_domain::EntityValue::bool(true)))
            .await
            .unwrap();

        repo.delete_for_device(device_id).await.unwrap();

        assert!(repo.list_by_device(device_id).await.unwrap().is_empty());
        assert!(repo.get_current_state(e.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn persisted_device_carries_connection_status() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool);
        let mut d = Device::new("status");
        d.connection_status = DeviceConnectionStatus::Connecting;
        devices.create(&d).await.unwrap();
        let fetched = devices.get_by_id(d.id).await.unwrap().unwrap();
        assert_eq!(
            fetched.connection_status,
            DeviceConnectionStatus::Connecting
        );
    }
}
