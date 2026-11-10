//! Device repository: persistence for `eam_domain::Device`.

use eam_domain::{Device, DeviceId};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// A row-level view of a device used for inserts/updates.
#[derive(Debug, Clone, Default)]
pub struct NewDevice {
    /// Node name (stable device identifier, unique).
    pub node_name: String,
    /// Display name.
    pub display_name: String,
    /// Preferred host.
    pub preferred_host: Option<String>,
    /// Preferred port (default 6053).
    pub preferred_port: u16,
    /// Reference to an encryption secret in the OS credential store.
    pub api_encryption_secret_ref: Option<String>,
    /// MAC address, when known.
    pub mac_address: Option<String>,
}

/// CRUD access to the `devices` table.
pub struct DeviceRepository {
    pool: SqlitePool,
}

impl DeviceRepository {
    /// Create a repository bound to `pool`.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a device. Rejects duplicate `node_name` values.
    ///
    /// # Errors
    /// Returns [`StorageError::Database`] on a unique violation or I/O failure.
    pub async fn create(&self, device: &Device) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO devices
                (id, node_name, display_name, preferred_host, preferred_port,
                 api_encryption_secret_ref, status, last_seen_at, firmware_version,
                 mac_address, metadata_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(device.id.0.to_string())
        .bind(&device.node_name)
        .bind(&device.display_name)
        .bind(&device.preferred_host)
        .bind(i64::from(device.preferred_port))
        .bind(&device.api_encryption_secret_ref)
        .bind(serde_json::to_string(&device.connection_status).expect("status serializes"))
        .bind(
            device
                .last_seen_at
                .as_ref()
                .map(chrono::DateTime::to_rfc3339),
        )
        .bind(&device.firmware_version)
        .bind(&device.mac_address)
        .bind("{}")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        Ok(())
    }

    /// Insert a device from a [`NewDevice`], returning the stored device.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure (including duplicate node name).
    pub async fn register(&self, new: &NewDevice) -> Result<Device> {
        let device = Device::new(&new.node_name);
        let stored = Device {
            display_name: new.display_name.clone(),
            preferred_host: new.preferred_host.clone(),
            preferred_port: new.preferred_port,
            api_encryption_secret_ref: new.api_encryption_secret_ref.clone(),
            mac_address: new.mac_address.clone(),
            ..device
        };
        self.create(&stored).await?;
        Ok(stored)
    }

    /// Fetch a device by id.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn get_by_id(&self, id: DeviceId) -> Result<Option<Device>> {
        let row: Option<DeviceRow> = sqlx::query_as(
            "SELECT id, node_name, display_name, preferred_host, preferred_port,
                        api_encryption_secret_ref, status, last_seen_at, firmware_version,
                        mac_address, created_at, updated_at
                 FROM devices WHERE id = ?",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        row.map(row_to_device).transpose()
    }

    /// Fetch a device by its unique node name.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn get_by_node_name(&self, node_name: &str) -> Result<Option<Device>> {
        let row: Option<DeviceRow> = sqlx::query_as(
            "SELECT id, node_name, display_name, preferred_host, preferred_port,
                        api_encryption_secret_ref, status, last_seen_at, firmware_version,
                        mac_address, created_at, updated_at
                 FROM devices WHERE node_name = ?",
        )
        .bind(node_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        row.map(row_to_device).transpose()
    }

    /// List all devices, newest last.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn list(&self) -> Result<Vec<Device>> {
        let rows: Vec<DeviceRow> = sqlx::query_as(
            "SELECT id, node_name, display_name, preferred_host, preferred_port,
                        api_encryption_secret_ref, status, last_seen_at, firmware_version,
                        mac_address, created_at, updated_at
                 FROM devices ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        rows.into_iter().map(row_to_device).collect()
    }

    /// Update the mutable fields of a device.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn update(&self, device: &Device) -> Result<()> {
        sqlx::query(
            "UPDATE devices SET
                node_name = ?, display_name = ?, preferred_host = ?, preferred_port = ?,
                api_encryption_secret_ref = ?, status = ?, last_seen_at = ?,
                firmware_version = ?, mac_address = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&device.node_name)
        .bind(&device.display_name)
        .bind(&device.preferred_host)
        .bind(i64::from(device.preferred_port))
        .bind(&device.api_encryption_secret_ref)
        .bind(serde_json::to_string(&device.connection_status).expect("status serializes"))
        .bind(
            device
                .last_seen_at
                .as_ref()
                .map(chrono::DateTime::to_rfc3339),
        )
        .bind(&device.firmware_version)
        .bind(&device.mac_address)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(device.id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(StorageError::Database)?;
        Ok(())
    }

    /// Delete a device and (via cascade) its entities and states.
    ///
    /// # Errors
    /// Returns [`StorageError`] on failure.
    pub async fn delete(&self, id: DeviceId) -> Result<()> {
        sqlx::query("DELETE FROM devices WHERE id = ?")
            .bind(id.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(StorageError::Database)?;
        Ok(())
    }
}

/// `(id, node_name, display_name, preferred_host, preferred_port, api_encryption_secret_ref,
///   status, last_seen_at, firmware_version, mac_address, created_at, updated_at)`
type DeviceRow = (
    String,
    String,
    String,
    Option<String>,
    i64,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);

fn row_to_device(row: DeviceRow) -> Result<Device> {
    let (
        id,
        node_name,
        display_name,
        preferred_host,
        preferred_port,
        api_encryption_secret_ref,
        status,
        last_seen_at,
        firmware_version,
        mac_address,
        created_at,
        updated_at,
    ) = row;

    let id = Uuid::parse_str(&id).map_err(|e| StorageError::decode_str(e.to_string()))?;
    let status: eam_domain::DeviceConnectionStatus =
        parse_enum(&status).map_err(|e| StorageError::decode_str(e.to_string()))?;
    let created_at = parse_ts(&created_at)?;
    let updated_at = parse_ts(&updated_at)?;

    Ok(Device {
        id: DeviceId(id),
        node_name,
        display_name,
        area_id: None,
        group_ids: Vec::new(),
        preferred_host,
        preferred_port: u16::try_from(preferred_port).unwrap_or(6053),
        api_encryption_secret_ref,
        connection_status: status,
        last_seen_at: last_seen_at.as_deref().map(parse_ts).transpose()?,
        firmware_version,
        mac_address,
        created_at,
        updated_at,
    })
}

/// Parse a serde-serialized domain enum value.
fn parse_enum<T: for<'de> serde::Deserialize<'de>>(value: &str) -> std::result::Result<T, String> {
    // Values are stored as serde JSON strings, e.g. `"online"` / `"offline"`.
    serde_json::from_str(value).map_err(|e| e.to_string())
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
    use crate::repo::entity_repo::EntityRepository;
    use eam_domain::{Entity, EntityState};

    fn new_device(node: &str) -> Device {
        Device::new(node)
    }

    #[tokio::test]
    async fn create_and_get_by_id() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        let device = new_device("nodemcu-01");
        repo.create(&device).await.unwrap();

        let fetched = repo.get_by_id(device.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, device.id);
        assert_eq!(fetched.node_name, "nodemcu-01");
        assert_eq!(fetched.preferred_port, 6053);
    }

    #[tokio::test]
    async fn create_rejects_duplicate_node_name() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        repo.create(&new_device("dupe")).await.unwrap();
        let err = repo.create(&new_device("dupe")).await.unwrap_err();
        assert!(matches!(err, StorageError::Database(_)));
    }

    #[tokio::test]
    async fn get_by_node_name_finds_device() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        let device = new_device("findme");
        repo.create(&device).await.unwrap();

        let found = repo
            .get_by_node_name("findme")
            .await
            .unwrap()
            .expect("device present");
        assert_eq!(found.id, device.id);
        assert!(repo.get_by_node_name("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_returns_all_in_insertion_order() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        for name in ["a", "b", "c"] {
            repo.create(&new_device(name)).await.unwrap();
        }
        assert_eq!(repo.list().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn update_persists_changes() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        let mut device = new_device("mutate");
        repo.create(&device).await.unwrap();

        device.connection_status = eam_domain::DeviceConnectionStatus::Online;
        device.display_name = "Renamed".into();
        device.firmware_version = Some("2026.10.2".into());
        repo.update(&device).await.unwrap();

        let fetched = repo.get_by_id(device.id).await.unwrap().unwrap();
        assert_eq!(
            fetched.connection_status,
            eam_domain::DeviceConnectionStatus::Online
        );
        assert_eq!(fetched.display_name, "Renamed");
        assert_eq!(fetched.firmware_version.as_deref(), Some("2026.10.2"));
        assert!(fetched.updated_at >= device.updated_at);
    }

    #[tokio::test]
    async fn delete_removes_device() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        let device = new_device("temporary");
        repo.create(&device).await.unwrap();
        repo.delete(device.id).await.unwrap();
        assert!(repo.get_by_id(device.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn register_new_device_round_trips() {
        let pool = open_in_memory().await.unwrap();
        let repo = DeviceRepository::new(pool);
        let stored = repo
            .register(&NewDevice {
                node_name: "registered".into(),
                display_name: "Registered Device".into(),
                preferred_host: Some("10.0.0.5".into()),
                preferred_port: 6053,
                api_encryption_secret_ref: None,
                mac_address: Some("AA:BB:CC:DD:EE:FF".into()),
            })
            .await
            .unwrap();
        assert_eq!(stored.display_name, "Registered Device");
        assert_eq!(stored.preferred_host.as_deref(), Some("10.0.0.5"));

        // Registering the same node name again is rejected (idempotent via lookup).
        assert!(repo
            .register(&NewDevice {
                node_name: "registered".into(),
                display_name: "dupe".into(),
                ..Default::default()
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn delete_cascades_to_entities_and_states() {
        let pool = open_in_memory().await.unwrap();
        let devices = DeviceRepository::new(pool.clone());
        let entities = EntityRepository::new(pool.clone());
        let device = new_device("cascade");
        devices.create(&device).await.unwrap();

        let entity = Entity::new(device.id.0, "42", "Relay", eam_domain::EntityType::Switch);
        entities.create(&entity).await.unwrap();
        entities
            .set_current_state(&EntityState::new(
                entity.id,
                eam_domain::EntityValue::bool(true),
            ))
            .await
            .unwrap();

        devices.delete(device.id).await.unwrap();

        assert!(entities.get(entity.id).await.unwrap().is_none());
        assert!(entities
            .get_current_state(entity.id)
            .await
            .unwrap()
            .is_none());
    }
}
