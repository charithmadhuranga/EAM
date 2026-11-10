//! ESPHome Native API client.
//!
//! Owns all protocol-level types; converts them into `eam-domain` types at the
//! boundary so no other crate needs to know about the wire protocol.

use std::time::Duration;

use eam_domain::{Device, DiscoveredDevice, Entity, EntityId};
use prost::Message as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, instrument};

use crate::api_proto as proto;
use crate::error::{Error, Result};
use crate::framing::{Frame, FrameDecoder};
use crate::message::MessageType;

/// Minimum API major version we speak.
pub const MIN_API_MAJOR: u32 = 1;
/// Minimum API minor version we speak (entity keys, switch/number commands).
pub const MIN_API_MINOR: u32 = 9;

/// Default ESPHome Native API port.
pub const DEFAULT_PORT: u16 = 6053;

/// Default TCP connect timeout.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Default frame read timeout.
pub const DEFAULT_FRAME_TIMEOUT: Duration = Duration::from_secs(10);

/// Client name sent in the hello handshake.
pub const CLIENT_INFO: &str = "ESPHome Automation Manager";

/// State of a device as understood by EAM.
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Node name (hostname-derived identifier).
    pub name: String,
    /// MAC address, when reported.
    pub mac_address: String,
    /// ESPHome firmware version string.
    pub esphome_version: String,
    /// Compile timestamp string.
    pub compilation_time: String,
    /// Board model (e.g. `esp32dev`).
    pub model: String,
    /// Project name, when the firmware was built from a project.
    pub project_name: String,
    /// Project version, when applicable.
    pub project_version: String,
    /// Board manufacturer.
    pub manufacturer: String,
    /// User-facing name.
    pub friendly_name: String,
    /// Whether the device can receive an encryption key over Noise.
    pub api_encryption_supported: bool,
}

impl DeviceInfo {
    /// Convert protocol info into a domain [`Device`].
    #[must_use]
    pub fn to_domain(&self) -> Device {
        let mut device = Device::new(if self.friendly_name.is_empty() {
            self.name.clone()
        } else {
            self.friendly_name.clone()
        });
        device.node_name = self.name.clone();
        device.preferred_host = None;
        device.preferred_port = DEFAULT_PORT;
        device.connection_status = eam_domain::DeviceConnectionStatus::Online;
        device.last_seen_at = Some(chrono::Utc::now());
        device.firmware_version = Some(self.esphome_version.clone());
        device.mac_address = (!self.mac_address.is_empty()).then(|| self.mac_address.clone());
        device.api_encryption_secret_ref = None;
        device
    }
}

/// A state update for one entity, keyed by its protocol key.
#[derive(Debug, Clone)]
pub struct StateUpdate {
    /// Protocol entity key.
    pub key: u32,
    /// Decoded value in domain terms.
    pub value: eam_domain::EntityValue,
    /// True when the device reported "no valid state yet".
    pub missing_state: bool,
}

/// An ESPHome Native API connection.
pub struct ApiClient {
    stream: TcpStream,
    decoder: FrameDecoder,
    api_major: u32,
    api_minor: u32,
    server_name: String,
    /// Entities discovered by the last [`ApiClient::list_entities`] call.
    entities: Vec<Entity>,
    /// Device id discovered by [`ApiClient::device_info`].
    device_id: Option<uuid::Uuid>,
    /// Frame read timeout.
    frame_timeout: Duration,
}

impl ApiClient {
    /// Connect to a device and complete the plaintext hello handshake.
    ///
    /// # Errors
    /// Returns [`Error::Connection`] on socket failure, [`Error::HandshakeTimeout`]
    /// if the device stays silent, or [`Error::UnsupportedVersion`] for an
    /// incompatible API version.
    pub async fn connect(addr: impl Into<String>) -> Result<Self> {
        let addr = addr.into();
        let stream =
            tokio::time::timeout(DEFAULT_CONNECT_TIMEOUT, TcpStream::connect(addr.as_str()))
                .await
                .map_err(|_| Error::HandshakeTimeout)?
                .map_err(|e| Error::Connection(format!("{addr}: {e}")))?;
        stream.set_nodelay(true).ok();

        let mut client = Self {
            stream,
            decoder: FrameDecoder::new(),
            api_major: 0,
            api_minor: 0,
            server_name: String::new(),
            entities: Vec::new(),
            device_id: None,
            frame_timeout: DEFAULT_FRAME_TIMEOUT,
        };
        client.hello().await?;
        Ok(client)
    }

    /// Send the hello handshake and validate the API version.
    async fn hello(&mut self) -> Result<()> {
        let hello = proto::HelloRequest {
            client_info: CLIENT_INFO.to_string(),
            api_version_major: MIN_API_MAJOR,
            api_version_minor: MIN_API_MINOR + 7,
        };
        self.send_message(MessageType::HelloRequest, &hello).await?;

        let frame = self.read_frame().await?;
        if frame.message_type != MessageType::HelloResponse.id() {
            return Err(Error::UnexpectedResponse(format!(
                "hello expected message {}, got {}",
                MessageType::HelloResponse.id(),
                frame.message_type
            )));
        }

        let response = proto::HelloResponse::decode(&frame.payload[..]).map_err(Error::decode)?;
        self.api_major = response.api_version_major;
        self.api_minor = response.api_version_minor;
        self.server_name = response.server_info;

        if self.api_major < MIN_API_MAJOR
            || (self.api_major == MIN_API_MAJOR && self.api_minor < MIN_API_MINOR)
        {
            return Err(Error::UnsupportedVersion {
                major: self.api_major,
                minor: self.api_minor,
                required: MIN_API_MAJOR,
                minor_required: MIN_API_MINOR,
            });
        }

        debug!(
            api = format!("{}.{}", self.api_major, self.api_minor),
            server = %self.server_name,
            name = %response.name,
            "hello accepted"
        );
        self.server_name = response.name;
        Ok(())
    }

    /// API major/minor negotiated with the device.
    #[must_use]
    pub const fn api_version(&self) -> (u32, u32) {
        (self.api_major, self.api_minor)
    }

    /// Node name reported by the device.
    #[must_use]
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Override the default frame read timeout.
    pub const fn set_frame_timeout(&mut self, timeout: Duration) {
        self.frame_timeout = timeout;
    }

    /// Fetch device information.
    #[instrument(skip_all)]
    pub async fn device_info(&mut self) -> Result<DeviceInfo> {
        self.send_message(MessageType::DeviceInfoRequest, &proto::Void {})
            .await?;

        let frame = self.read_frame().await?;
        if frame.message_type != MessageType::DeviceInfoResponse.id() {
            return Err(Error::UnexpectedResponse(format!(
                "device_info expected message {}, got {}",
                MessageType::DeviceInfoResponse.id(),
                frame.message_type
            )));
        }
        let msg = proto::DeviceInfoResponse::decode(&frame.payload[..]).map_err(Error::decode)?;

        Ok(DeviceInfo {
            name: msg.name,
            mac_address: msg.mac_address,
            esphome_version: msg.esphome_version,
            compilation_time: msg.compilation_time,
            model: msg.model,
            project_name: msg.project_name,
            project_version: msg.project_version,
            manufacturer: msg.manufacturer,
            friendly_name: msg.friendly_name,
            api_encryption_supported: msg.api_encryption_supported,
        })
    }

    /// List all entities on the device until `ListEntitiesDoneResponse`.
    ///
    /// # Errors
    /// Returns [`Error::ProtocolViolation`] if the device never terminates the
    /// list, or [`Error::Connection`] if the stream drops mid-list.
    #[instrument(skip_all)]
    pub async fn list_entities(&mut self, device_id: uuid::Uuid) -> Result<Vec<Entity>> {
        self.device_id = Some(device_id);
        self.entities.clear();
        self.send_message(MessageType::ListEntitiesRequest, &proto::Void {})
            .await?;

        // Keep a copy in the client so `entity_for_key` stays valid, while the
        // caller owns the returned list.
        loop {
            let frame = self.read_frame().await?;
            match frame.message_type {
                id if id == MessageType::ListEntitiesDoneResponse.id() => {
                    proto::ListEntitiesDoneResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    debug!(count = self.entities.len(), "entity list complete");
                    // Clone so the caller owns the list while the client keeps
                    // it for `entity_for_key` lookups.
                    return Ok(self.entities.clone());
                }
                id if id == MessageType::ListEntitiesSwitchResponse.id() => {
                    let msg = proto::ListEntitiesSwitchResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::Switch,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.device_class.is_empty() {
                        entity.device_class = Some(msg.device_class);
                    }
                    if !msg.icon.is_empty() {
                        entity.icon = Some(msg.icon);
                    }
                    entity.raw_metadata = serde_json::json!({"assumed_state": msg.assumed_state});
                    self.entities.push(entity);
                }
                id if id == MessageType::ListEntitiesNumberResponse.id() => {
                    let msg = proto::ListEntitiesNumberResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::Number,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.unit_of_measurement.is_empty() {
                        entity.unit = Some(msg.unit_of_measurement);
                    }
                    if !msg.device_class.is_empty() {
                        entity.device_class = Some(msg.device_class);
                    }
                    entity.capabilities = serde_json::json!({
                        "min_value": msg.min_value,
                        "max_value": msg.max_value,
                        "step": msg.step,
                    });
                    self.entities.push(entity);
                }
                id if id == MessageType::ListEntitiesSensorResponse.id() => {
                    let msg = proto::ListEntitiesSensorResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::Sensor,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.unit_of_measurement.is_empty() {
                        entity.unit = Some(msg.unit_of_measurement);
                    }
                    if !msg.device_class.is_empty() {
                        entity.device_class = Some(msg.device_class);
                    }
                    if !msg.icon.is_empty() {
                        entity.icon = Some(msg.icon);
                    }
                    self.entities.push(entity);
                }
                id if id == MessageType::ListEntitiesBinarySensorResponse.id() => {
                    let msg = proto::ListEntitiesBinarySensorResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::BinarySensor,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.device_class.is_empty() {
                        entity.device_class = Some(msg.device_class);
                    }
                    if !msg.icon.is_empty() {
                        entity.icon = Some(msg.icon);
                    }
                    self.entities.push(entity);
                }
                id if id == MessageType::ListEntitiesTextSensorResponse.id() => {
                    let msg = proto::ListEntitiesTextSensorResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::TextSensor,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.device_class.is_empty() {
                        entity.device_class = Some(msg.device_class);
                    }
                    if !msg.icon.is_empty() {
                        entity.icon = Some(msg.icon);
                    }
                    self.entities.push(entity);
                }
                id if id == MessageType::ListEntitiesLightResponse.id() => {
                    let msg = proto::ListEntitiesLightResponse::decode(&frame.payload[..])
                        .map_err(Error::decode)?;
                    let mut entity = Entity::new(
                        device_id,
                        msg.key.to_string(),
                        &msg.name,
                        eam_domain::EntityType::Light,
                    );
                    entity.object_id = (!msg.object_id.is_empty()).then_some(msg.object_id);
                    if !msg.icon.is_empty() {
                        entity.icon = Some(msg.icon);
                    }
                    entity.capabilities = serde_json::json!({
                        "min_mireds": msg.min_mireds,
                        "max_mireds": msg.max_mireds,
                        "effects": msg.effects,
                    });
                    self.entities.push(entity);
                }
                id => {
                    return Err(Error::UnexpectedResponse(format!(
                        "entity list: unknown message id {id}"
                    )));
                }
            }
        }
    }

    /// Subscribe to state updates. After this, [`ApiClient::next_state`] yields
    /// [`StateUpdate`]s as the device publishes them.
    #[instrument(skip_all)]
    pub async fn subscribe_states(&mut self) -> Result<()> {
        self.send_message(MessageType::SubscribeStatesRequest, &proto::Void {})
            .await
    }

    /// Read the next state update.
    ///
    /// # Errors
    /// Returns [`Error::Connection`] when the peer disconnects (used as the
    /// reconnect trigger), or [`Error::UnexpectedResponse`] for non-state
    /// messages.
    #[instrument(skip_all)]
    pub async fn next_state(&mut self) -> Result<StateUpdate> {
        let frame = self.read_frame().await?;
        Self::decode_state(&frame)
    }

    /// Decode a state response frame into a [`StateUpdate`].
    fn decode_state(frame: &Frame) -> Result<StateUpdate> {
        let missing = |msg: bool| msg;
        let update = |key, value: eam_domain::EntityValue, missing: bool| StateUpdate {
            key,
            value,
            missing_state: missing,
        };

        match frame.message_type {
            id if id == MessageType::SwitchStateResponse.id() => {
                let msg = proto::SwitchStateResponse::decode(&frame.payload[..])
                    .map_err(Error::decode)?;
                Ok(update(
                    msg.key,
                    eam_domain::EntityValue::bool(msg.state),
                    false,
                ))
            }
            id if id == MessageType::NumberStateResponse.id() => {
                let msg = proto::NumberStateResponse::decode(&frame.payload[..])
                    .map_err(Error::decode)?;
                Ok(update(
                    msg.key,
                    eam_domain::EntityValue::number(f64::from(msg.state)),
                    missing(msg.missing_state),
                ))
            }
            id if id == MessageType::SensorStateResponse.id() => {
                let msg = proto::SensorStateResponse::decode(&frame.payload[..])
                    .map_err(Error::decode)?;
                Ok(update(
                    msg.key,
                    eam_domain::EntityValue::number(f64::from(msg.state)),
                    missing(msg.missing_state),
                ))
            }
            id if id == MessageType::BinarySensorStateResponse.id() => {
                let msg = proto::BinarySensorStateResponse::decode(&frame.payload[..])
                    .map_err(Error::decode)?;
                Ok(update(
                    msg.key,
                    eam_domain::EntityValue::bool(msg.state),
                    missing(msg.missing_state),
                ))
            }
            id if id == MessageType::TextSensorStateResponse.id() => {
                let msg = proto::TextSensorStateResponse::decode(&frame.payload[..])
                    .map_err(Error::decode)?;
                Ok(update(
                    msg.key,
                    eam_domain::EntityValue::text(msg.state),
                    missing(msg.missing_state),
                ))
            }
            id if id == MessageType::LightStateResponse.id() => {
                let msg =
                    proto::LightStateResponse::decode(&frame.payload[..]).map_err(Error::decode)?;
                let value = eam_domain::EntityValue::Json(serde_json::json!({
                    "state": msg.state,
                    "brightness": msg.brightness,
                    "color_mode": msg.color_mode,
                    "red": msg.red,
                    "green": msg.green,
                    "blue": msg.blue,
                }));
                Ok(update(msg.key, value, false))
            }
            id => Err(Error::UnexpectedResponse(format!(
                "state update: unknown message id {id}"
            ))),
        }
    }

    /// Find the domain entity id for a protocol key.
    ///
    /// # Errors
    /// Returns [`Error::UnknownKey`] when no entity with that key was listed.
    pub fn entity_for_key(&self, key: u32) -> Result<EntityId> {
        self.entities
            .iter()
            .find(|e| e.native_entity_key == key.to_string())
            .map(|e| e.id)
            .ok_or(Error::UnknownKey(key))
    }

    /// Turn a switch on or off.
    #[instrument(skip_all, fields(key, state))]
    pub async fn switch_command(&mut self, key: u32, state: bool) -> Result<()> {
        // Fail fast (and locally) if the key was never listed.
        self.entity_for_key(key)?;
        let msg = proto::SwitchCommandRequest {
            key,
            state,
            device_id: 0,
        };
        self.send_message(MessageType::SwitchCommandRequest, &msg)
            .await
    }

    /// Set a number entity.
    #[instrument(skip_all, fields(key, value))]
    pub async fn number_command(&mut self, key: u32, value: f32) -> Result<()> {
        self.entity_for_key(key)?;
        let msg = proto::NumberCommandRequest {
            key,
            state: value,
            device_id: 0,
        };
        self.send_message(MessageType::NumberCommandRequest, &msg)
            .await
    }

    /// Send a ping and wait for the pong.
    #[instrument(skip_all)]
    pub async fn ping(&mut self) -> Result<()> {
        self.send_message(MessageType::PingRequest, &proto::Void {})
            .await?;
        let frame = self.read_frame().await?;
        if frame.message_type != MessageType::PingResponse.id() {
            return Err(Error::UnexpectedResponse(format!(
                "ping expected message {}, got {}",
                MessageType::PingResponse.id(),
                frame.message_type
            )));
        }
        Ok(())
    }

    /// Graceful disconnect.
    #[instrument(skip_all)]
    pub async fn disconnect(&mut self) -> Result<()> {
        let msg = proto::DisconnectRequest { reason: 0 };
        if self
            .send_message(MessageType::DisconnectRequest, &msg)
            .await
            .is_ok()
        {
            // Best effort: read the ack but don't fail on it.
            match self.read_frame().await {
                Ok(_) => {}
                Err(e) => debug!("no disconnect ack: {e}"),
            }
        }
        self.stream.shutdown().await.ok();
        Ok(())
    }

    /// Encode and send one protocol message.
    async fn send_message<M: prost::Message>(
        &mut self,
        msg_type: MessageType,
        msg: &M,
    ) -> Result<()> {
        let payload = msg.encode_to_vec();
        let frame = Frame::with_payload(msg_type.id(), payload);
        self.stream
            .write_all(&frame.encode())
            .await
            .map_err(Error::from)?;
        self.stream.flush().await.map_err(Error::from)?;
        Ok(())
    }

    /// Read the next frame, applying the frame timeout.
    async fn read_frame(&mut self) -> Result<Frame> {
        let deadline = tokio::time::Instant::now() + self.frame_timeout;
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame);
            }
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or_default();
            if remaining.is_zero() {
                return Err(Error::HandshakeTimeout);
            }

            let mut buf = [0u8; 4096];
            let n = tokio::time::timeout(remaining, self.stream.read(&mut buf))
                .await
                .map_err(|_| Error::HandshakeTimeout)?
                .map_err(Error::from)?;
            if n == 0 {
                return Err(Error::Connection("peer closed connection".into()));
            }
            self.decoder.extend(&buf[..n]);
        }
    }
}

impl Drop for ApiClient {
    fn drop(&mut self) {
        // The socket is closed with the client; no async work in Drop.
        let peer = self
            .stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "<closed>".into());
        debug!(%peer, "api client dropped");
    }
}

/// Convenience: discover, connect, and fetch device info in one call.
///
/// Used by tests and the discovery layer until mDNS lands in `eam-discovery`.
///
/// # Errors
/// See [`ApiClient::connect`].
pub async fn probe(
    addr: impl Into<String>,
    device_id: uuid::Uuid,
) -> Result<(DeviceInfo, Vec<Entity>, DiscoveredDevice)> {
    let addr = addr.into();
    let mut client = ApiClient::connect(addr.clone()).await?;
    let info = client.device_info().await?;
    let entities = client.list_entities(device_id).await?;
    let discovered = DiscoveredDevice {
        name: info.name.clone(),
        host: addr,
        port: DEFAULT_PORT,
        mac_address: (!info.mac_address.is_empty()).then(|| info.mac_address.clone()),
        friendly_name: (!info.friendly_name.is_empty()).then(|| info.friendly_name.clone()),
        source: eam_domain::DiscoverySource::Manual,
    };
    Ok((info, entities, discovered))
}

#[allow(dead_code)]
fn assert_sync() {
    fn is_send<T: Send>() {}
    is_send::<ApiClient>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_fails_on_closed_port() {
        // Port 1 on localhost is never open; verifies error type, not the network.
        let err = ApiClient::connect("127.0.0.1:1")
            .await
            .err()
            .expect("must fail");
        assert!(matches!(err, Error::Connection(_)));
        assert!(err.is_recoverable());
    }

    #[tokio::test]
    async fn connect_fails_on_bad_address() {
        let err = ApiClient::connect("not-a-host:6053")
            .await
            .err()
            .expect("must fail");
        assert!(matches!(
            err,
            Error::Connection(_) | Error::HandshakeTimeout
        ));
    }

    #[test]
    fn device_info_to_domain_uses_friendly_name() {
        let info = DeviceInfo {
            name: "eam-test-device".into(),
            friendly_name: "EAM Test Device".into(),
            esphome_version: "2026.10.2".into(),
            ..Default::default()
        };
        let device = info.to_domain();
        assert_eq!(device.display_name, "EAM Test Device");
        assert_eq!(device.node_name, "eam-test-device");
        assert_eq!(device.preferred_port, DEFAULT_PORT);
    }

    #[test]
    fn device_info_to_domain_falls_back_to_node_name() {
        let info = DeviceInfo {
            name: "bare-device".into(),
            ..Default::default()
        };
        assert_eq!(info.to_domain().display_name, "bare-device");
    }

    #[tokio::test]
    async fn read_frame_times_out_on_silent_peer() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local = listener.local_addr().unwrap();
        // One end connects and stays silent; we read from the accepted end.
        let _silent = tokio::net::TcpStream::connect(local).await.unwrap();
        let (stream, _peer) = listener.accept().await.unwrap();

        let mut client = ApiClient {
            stream,
            decoder: FrameDecoder::new(),
            api_major: 1,
            api_minor: 16,
            server_name: String::new(),
            entities: Vec::new(),
            device_id: None,
            frame_timeout: Duration::from_millis(50),
        };
        let err = client.read_frame().await.unwrap_err();
        assert!(matches!(err, Error::HandshakeTimeout));
        assert!(err.is_recoverable());
    }
}
