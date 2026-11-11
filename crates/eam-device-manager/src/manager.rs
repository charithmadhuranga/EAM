//! Connection manager: owns one live API connection per registered device.
//!
//! Responsibilities (PLAN.md §1.4):
//! - register devices (manual or discovered) and persist them
//! - connect / disconnect / report status
//! - feed the entity registry with listed entities and live states
//! - implement [`CommandTransport`] so commands reach the right device
//! - reconnect with exponential backoff and jitter (see [`backoff`])
//!
//! Credentials are never stored here: only a reference to the OS credential
//! store (`api_encryption_secret_ref`) is kept on the domain device.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use eam_discovery::DiscoveryService;
use eam_domain::{Device, DeviceConnectionStatus, DeviceId, DiscoveredDevice};
use eam_entity_registry::{registry::SharedRegistry, CommandTransport};
use eam_esphome_api::ApiClient;
use eam_storage::DeviceRepository;
use parking_lot::Mutex;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::backoff::{Backoff, BackoffConfig};
use crate::error::{Error, Result};

/// Capacity of the event channel per subscriber.
pub const EVENT_CAPACITY: usize = 256;

/// A live connection to one device.
struct Connection {
    /// The live client handle; `None` while disconnected.
    client: Option<Arc<tokio::sync::Mutex<ApiClient>>>,
    /// Backoff state for the next reconnect attempt.
    backoff: Backoff,
}

/// Events emitted by the manager.
#[derive(Debug, Clone, PartialEq)]
pub enum ManagerEvent {
    /// A device changed connection status.
    DeviceStatus {
        /// Device id.
        device_id: Uuid,
        /// New status.
        status: DeviceConnectionStatus,
    },
    /// A device was registered.
    DeviceRegistered {
        /// Device id.
        device_id: Uuid,
    },
    /// A device was removed.
    DeviceRemoved {
        /// Device id.
        device_id: Uuid,
    },
}

/// Entry point for device connectivity.
pub struct ConnectionManager {
    /// Storage pool for device and entity persistence.
    pool: sqlx::SqlitePool,
    /// Where the database lives (shown in the UI workspace info).
    data_dir: String,
    /// Live connections by device id.
    connections: Mutex<HashMap<Uuid, Connection>>,
    /// Shared in-memory registry (states are broadcast from here).
    registry: SharedRegistry,
    /// Reconnect configuration.
    backoff: BackoffConfig,
    /// Publishes manager events.
    events: broadcast::Sender<ManagerEvent>,
    /// Cancels background reconnect loops.
    shutdown: CancellationToken,
}

impl ConnectionManager {
    /// Create a manager over `pool` and a shared `registry`.
    #[must_use]
    pub fn new(pool: sqlx::SqlitePool, registry: SharedRegistry) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            pool,
            data_dir: ".".to_string(),
            connections: Mutex::new(HashMap::new()),
            registry,
            backoff: BackoffConfig::default(),
            events,
            shutdown: CancellationToken::new(),
        }
    }

    /// Device repository bound to this manager's pool.
    #[must_use]
    pub fn devices(&self) -> DeviceRepository {
        DeviceRepository::new(self.pool.clone())
    }

    /// Discovery service used by [`ConnectionManager::register_discovered`].
    #[must_use]
    pub fn discovery(&self) -> DiscoveryService {
        DiscoveryService::new()
    }

    /// Record the database location for the UI.
    #[must_use]
    pub fn with_data_dir(mut self, dir: impl Into<String>) -> Self {
        self.data_dir = dir.into();
        self
    }

    /// Database location as shown in the UI.
    #[must_use]
    pub fn data_dir(&self) -> String {
        self.data_dir.clone()
    }

    /// Override the reconnect backoff configuration.
    #[must_use]
    pub fn with_backoff(mut self, config: BackoffConfig) -> Self {
        self.backoff = config;
        self
    }

    /// Subscribe to manager events (status changes, registrations).
    #[must_use]
    pub fn subscribe_events(&self) -> broadcast::Receiver<ManagerEvent> {
        self.events.subscribe()
    }

    /// Subscribe to state updates (same feed the UI consumes).
    #[must_use]
    pub async fn subscribe_states(
        &self,
    ) -> tokio::sync::broadcast::Receiver<eam_entity_registry::StateUpdate> {
        self.registry.lock().await.subscribe_states()
    }

    fn emit(&self, event: ManagerEvent) {
        let _ = self.events.send(event);
    }

    /// All registered devices.
    ///
    /// # Errors
    /// Returns [`Error`] on storage failure.
    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        Ok(self.devices().list().await?)
    }

    /// Fetch a device by id.
    ///
    /// # Errors
    /// Returns [`Error`] on storage failure.
    pub async fn device(&self, id: DeviceId) -> Result<Option<Device>> {
        Ok(self.devices().get_by_id(id).await?)
    }

    /// Register a discovered device, returning the stored device.
    ///
    /// Re-discovering a known node name returns the existing device instead of
    /// creating a duplicate (PLAN.md §1.3: duplicate detection).
    ///
    /// # Errors
    /// Returns [`Error`] on storage failure.
    pub async fn register_discovered(&self, found: &DiscoveredDevice) -> Result<Device> {
        self.register(
            found.name.clone(),
            found.host.clone(),
            found.port,
            found.mac_address.clone(),
        )
        .await
    }

    /// Register a device by address, returning the stored device.
    ///
    /// # Errors
    /// Returns [`Error`] on storage failure.
    pub async fn register(
        &self,
        node_name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        mac_address: Option<String>,
    ) -> Result<Device> {
        let node_name = node_name.into();
        if let Some(existing) = self.devices().get_by_node_name(&node_name).await? {
            debug!(%node_name, "device already registered");
            return Ok(existing);
        }

        let device = self
            .devices()
            .register(&eam_storage::NewDevice {
                node_name: node_name.clone(),
                display_name: node_name,
                preferred_host: Some(host.into()),
                preferred_port: port,
                api_encryption_secret_ref: None,
                mac_address,
            })
            .await?;
        info!(id = %device.id, node = %device.node_name, "device registered");

        self.connections.lock().insert(
            device.id.0,
            Connection {
                client: None,
                backoff: Backoff::new(self.backoff),
            },
        );
        self.emit(ManagerEvent::DeviceRegistered {
            device_id: device.id.0,
        });
        Ok(device)
    }

    /// Remove a device (disconnecting first) together with its entities.
    ///
    /// # Errors
    /// Returns [`Error`] on storage failure.
    pub async fn unregister(&self, id: DeviceId) -> Result<()> {
        // Take the client out first so the streaming task sees the drop, then
        // close it without holding the connections lock across await points.
        let client = {
            let mut connections = self.connections.lock();
            connections
                .get_mut(&id.0)
                .and_then(|conn| conn.client.take())
        };
        if let Some(client) = client {
            client.lock().await.disconnect().await.ok();
        }
        self.registry.lock().await.remove_device(id.0);
        self.connections.lock().remove(&id.0);
        self.devices().delete(id).await?;
        self.emit(ManagerEvent::DeviceRemoved { device_id: id.0 });
        Ok(())
    }

    /// Connection status of a device.
    #[must_use]
    pub fn status(&self, id: DeviceId) -> DeviceConnectionStatus {
        self.connections
            .lock()
            .get(&id.0)
            .map(|c| {
                if c.client.is_some() {
                    DeviceConnectionStatus::Online
                } else {
                    DeviceConnectionStatus::Offline
                }
            })
            .unwrap_or(DeviceConnectionStatus::Unknown)
    }

    /// Connect a device: handshake, list entities, persist, subscribe states.
    ///
    /// Safe to call repeatedly; a connected device is left untouched. Starts a
    /// background task that streams states into the registry.
    ///
    /// # Errors
    /// Returns [`Error::ConnectFailed`] when the device cannot be reached, and
    /// [`Error::Storage`] when persistence fails.
    pub async fn connect(self: &Arc<Self>, id: DeviceId) -> Result<()> {
        let device = self
            .devices()
            .get_by_id(id)
            .await?
            .ok_or_else(|| Error::UnknownDevice(id.to_string()))?;

        if self.status(id) == DeviceConnectionStatus::Online {
            return Ok(());
        }

        let host = device
            .preferred_host
            .clone()
            .unwrap_or_else(|| format!("{}.local", device.node_name));
        let addr = format!("{}:{}", host, device.preferred_port);

        debug!(%id, %addr, "connecting to device");
        let mut client = ApiClient::connect(&addr)
            .await
            .map_err(|e| Error::ConnectFailed {
                node: device.node_name.clone(),
                reason: e.to_string(),
            })?;

        let info = client
            .device_info()
            .await
            .map_err(|e| Error::ConnectFailed {
                node: device.node_name.clone(),
                reason: e.to_string(),
            })?;
        let entities = client
            .list_entities(id.0)
            .await
            .map_err(|e| Error::ConnectFailed {
                node: device.node_name.clone(),
                reason: e.to_string(),
            })?;
        client
            .subscribe_states()
            .await
            .map_err(|e| Error::ConnectFailed {
                node: device.node_name.clone(),
                reason: e.to_string(),
            })?;

        self.persist_device_and_entities(&device, &info, &entities)
            .await?;

        let shared = Arc::new(tokio::sync::Mutex::new(client));
        {
            let mut connections = self.connections.lock();
            let entry = connections.entry(id.0).or_insert_with(|| Connection {
                client: None,
                backoff: Backoff::new(self.backoff),
            });
            entry.client = Some(shared.clone());
            entry.backoff.reset();
        }

        info!(%id, node = %device.node_name, entities = entities.len(), "device connected");
        self.emit(ManagerEvent::DeviceStatus {
            device_id: id.0,
            status: DeviceConnectionStatus::Online,
        });

        self.spawn_state_stream(id.0, shared);
        Ok(())
    }

    /// Persist device identity and the entity list.
    async fn persist_device_and_entities(
        &self,
        device: &Device,
        info: &eam_esphome_api::DeviceInfo,
        entities: &[eam_domain::Entity],
    ) -> Result<()> {
        let mut stored = device.clone();
        stored.firmware_version = Some(info.esphome_version.clone());
        stored.mac_address = (!info.mac_address.is_empty())
            .then(|| info.mac_address.clone())
            .or_else(|| device.mac_address.clone());
        stored.last_seen_at = Some(chrono::Utc::now());
        stored.connection_status = DeviceConnectionStatus::Online;
        self.devices().update(&stored).await?;

        let persisted = eam_storage::EntityRepository::new(self.pool.clone())
            .replace_for_device(device.id.0, entities)
            .await?;
        self.registry
            .lock()
            .await
            .refresh_entities(device.id.0, persisted);
        Ok(())
    }

    /// Spawn the task that reads states from a device into the registry.
    ///
    /// The task exits when the client stops responding (the connection is then
    /// marked offline so the reconnect loop can take over) or on shutdown.
    fn spawn_state_stream(
        self: &Arc<Self>,
        device_id: Uuid,
        client: Arc<tokio::sync::Mutex<ApiClient>>,
    ) {
        let manager = Arc::clone(self);
        let registry = Arc::clone(&self.registry);
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            loop {
                if shutdown.is_cancelled() {
                    return;
                }
                let next = tokio::time::timeout(
                    Duration::from_secs(120),
                    client.lock().await.next_state(),
                )
                .await;
                match next {
                    Ok(Ok(update)) => {
                        // Map protocol key → domain entity id.
                        let entity_id = {
                            let guard = registry.lock().await;
                            guard
                                .get_by_key(device_id, &update.key.to_string())
                                .map(|e| e.id)
                        };
                        if let Some(entity_id) = entity_id {
                            registry.lock().await.update_state(
                                entity_id,
                                update.value,
                                update.missing_state,
                            );
                        } else {
                            debug!(key = update.key, %device_id, "state for unknown entity key");
                        }
                    }
                    Ok(Err(e)) => {
                        warn!(%device_id, "state stream ended: {e}");
                        break;
                    }
                    Err(_) => {
                        debug!(%device_id, "state read timed out; keep-alive ping");
                        if client.lock().await.ping().await.is_err() {
                            warn!(%device_id, "ping failed; closing stream");
                            break;
                        }
                    }
                }
            }

            // Mark the device offline so a reconnect can re-establish it.
            manager.mark_offline(device_id);
            manager.emit(ManagerEvent::DeviceStatus {
                device_id,
                status: DeviceConnectionStatus::Offline,
            });
        });
    }

    /// Disconnect a device and stop streaming.
    ///
    /// # Errors
    /// Returns [`Error`] when the device is unknown.
    pub async fn disconnect(&self, id: DeviceId) -> Result<()> {
        let client = {
            let mut connections = self.connections.lock();
            let conn = connections
                .get_mut(&id.0)
                .ok_or_else(|| Error::UnknownDevice(id.to_string()))?;
            conn.client.take()
        };
        if let Some(client) = client {
            client.lock().await.disconnect().await.ok();
        }
        self.emit(ManagerEvent::DeviceStatus {
            device_id: id.0,
            status: DeviceConnectionStatus::Offline,
        });
        Ok(())
    }

    /// Reconnect every disconnected device once.
    ///
    /// # Errors
    /// Returns [`Error::ConnectFailed`] when any device could not be reached.
    pub async fn reconnect_all(self: &Arc<Self>) -> Result<()> {
        let devices = self.devices().list().await?;
        let mut failures = 0usize;
        for device in devices {
            if self.status(device.id) == DeviceConnectionStatus::Online {
                continue;
            }
            if let Err(e) = self.connect(device.id).await {
                warn!(id = %device.id, "reconnect failed: {e}");
                failures += 1;
            }
        }
        if failures == 0 {
            Ok(())
        } else {
            Err(Error::ConnectFailed {
                node: "multiple".into(),
                reason: format!("{failures} devices still unreachable"),
            })
        }
    }

    /// Clear a device's connection so `status` reports Offline.
    pub fn mark_offline(&self, device_id: Uuid) {
        if let Some(conn) = self.connections.lock().get_mut(&device_id) {
            conn.client.take();
        }
    }

    /// Stop background streaming/reconnect tasks.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    /// Whether the runtime has been asked to shut down.
    #[must_use]
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.is_cancelled()
    }
}

#[async_trait::async_trait]
impl CommandTransport for ConnectionManager {
    async fn switch_command(
        &self,
        device_id: Uuid,
        native_key: u32,
        state: bool,
    ) -> std::result::Result<(), String> {
        let client = self
            .connections
            .lock()
            .get(&device_id)
            .and_then(|c| c.client.clone())
            .ok_or_else(|| format!("device {device_id} is not connected"))?;
        let mut guard = client.lock().await;
        guard
            .switch_command(native_key, state)
            .await
            .map_err(|e| e.to_string())
    }

    async fn number_command(
        &self,
        device_id: Uuid,
        native_key: u32,
        value: f32,
    ) -> std::result::Result<(), String> {
        let client = self
            .connections
            .lock()
            .get(&device_id)
            .and_then(|c| c.client.clone())
            .ok_or_else(|| format!("device {device_id} is not connected"))?;
        let mut guard = client.lock().await;
        guard
            .number_command(native_key, value)
            .await
            .map_err(|e| e.to_string())
    }
}
