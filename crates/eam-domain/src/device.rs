//! Device domain types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    /// Create a new random device ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Device connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConnectionStatus {
    /// Device is online and connected
    Online,
    /// Currently connecting to device
    Connecting,
    /// Device is offline or unreachable
    Offline,
    /// Authentication failed
    AuthenticationFailed,
    /// Connection error
    Error,
    /// Device is disabled
    Disabled,
}

impl Default for DeviceConnectionStatus {
    fn default() -> Self {
        Self::Offline
    }
}

impl std::fmt::Display for DeviceConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "Online"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Offline => write!(f, "Offline"),
            Self::AuthenticationFailed => write!(f, "Authentication Failed"),
            Self::Error => write!(f, "Error"),
            Self::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier
    pub id: DeviceId,
    
    /// ESPHome node name (stable identifier from device)
    pub node_name: String,
    
    /// User-friendly display name
    pub display_name: String,
    
    /// Area ID (optional, for organization)
    pub area_id: Option<Uuid>,
    
    /// Group IDs (optional, for organization)
    pub group_ids: Vec<Uuid>,
    
    /// Preferred connection host (IP or hostname)
    pub preferred_host: Option<String>,
    
    /// Preferred connection port (default: 6053)
    pub preferred_port: u16,
    
    /// Reference to API encryption secret in OS credential store
    pub api_encryption_secret_ref: Option<String>,
    
    /// Current connection status
    pub connection_status: DeviceConnectionStatus,
    
    /// Last time device was seen
    pub last_seen_at: Option<DateTime<Utc>>,
    
    /// Device firmware version
    pub firmware_version: Option<String>,
    
    /// Device MAC address
    pub mac_address: Option<String>,
    
    /// Device creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Device {
    /// Create a new device with the given node name
    pub fn new(node_name: impl Into<String>) -> Self {
        let now = Utc::now();
        let name = node_name.into();
        
        Self {
            id: DeviceId::new(),
            node_name: name.clone(),
            display_name: name,
            area_id: None,
            group_ids: Vec::new(),
            preferred_host: None,
            preferred_port: 6053,
            api_encryption_secret_ref: None,
            connection_status: DeviceConnectionStatus::Offline,
            last_seen_at: None,
            firmware_version: None,
            mac_address: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Set the display name
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self.updated_at = Utc::now();
        self
    }
    
    /// Set the preferred connection endpoint
    pub fn with_endpoint(mut self, host: impl Into<String>, port: u16) -> Self {
        self.preferred_host = Some(host.into());
        self.preferred_port = port;
        self.updated_at = Utc::now();
        self
    }
    
    /// Mark device as online
    pub fn set_online(&mut self) {
        self.connection_status = DeviceConnectionStatus::Online;
        self.last_seen_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
    
    /// Mark device as offline
    pub fn set_offline(&mut self) {
        self.connection_status = DeviceConnectionStatus::Offline;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device::new("living-room-sensor");
        
        assert_eq!(device.node_name, "living-room-sensor");
        assert_eq!(device.display_name, "living-room-sensor");
        assert_eq!(device.preferred_port, 6053);
        assert_eq!(device.connection_status, DeviceConnectionStatus::Offline);
    }

    #[test]
    fn test_device_with_display_name() {
        let device = Device::new("living-room-sensor")
            .with_display_name("Living Room Temperature");
        
        assert_eq!(device.node_name, "living-room-sensor");
        assert_eq!(device.display_name, "Living Room Temperature");
    }

    #[test]
    fn test_device_connection_status() {
        let mut device = Device::new("test-device");
        assert_eq!(device.connection_status, DeviceConnectionStatus::Offline);
        
        device.set_online();
        assert_eq!(device.connection_status, DeviceConnectionStatus::Online);
        assert!(device.last_seen_at.is_some());
        
        device.set_offline();
        assert_eq!(device.connection_status, DeviceConnectionStatus::Offline);
    }
}
