//! Additional domain types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Area (room, zone) for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Unique area identifier
    pub id: Uuid,
    
    /// Area name
    pub name: String,
    
    /// Parent area (for nested areas)
    pub parent_id: Option<Uuid>,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Area {
    /// Create a new area
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            parent_id: None,
            created_at: Utc::now(),
        }
    }
}

/// Device group for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    /// Unique group identifier
    pub id: Uuid,
    
    /// Group name
    pub name: String,
    
    /// Group description
    pub description: Option<String>,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl DeviceGroup {
    /// Create a new device group
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            created_at: Utc::now(),
        }
    }
}

/// Discovered device (from mDNS or manual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// Device name
    pub name: String,
    
    /// Host address
    pub host: String,
    
    /// Port number
    pub port: u16,
    
    /// MAC address (if available)
    pub mac_address: Option<String>,
    
    /// Friendly name (if advertised)
    pub friendly_name: Option<String>,
    
    /// Discovery source
    pub source: DiscoverySource,
}

/// Discovery source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    /// mDNS discovery
    Mdns,
    
    /// Manual registration
    Manual,
    
    /// Network scan
    NetworkScan,
}

/// Runtime health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealth {
    /// Runtime is healthy
    pub healthy: bool,
    
    /// Number of connected devices
    pub connected_devices: usize,
    
    /// Number of registered entities
    pub total_entities: usize,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// Uptime in seconds
    pub uptime_secs: u64,
    
    /// Last health check timestamp
    pub checked_at: DateTime<Utc>,
}

impl RuntimeHealth {
    /// Create a new health status
    pub fn new() -> Self {
        Self {
            healthy: true,
            connected_devices: 0,
            total_entities: 0,
            memory_usage: 0,
            uptime_secs: 0,
            checked_at: Utc::now(),
        }
    }
}

impl Default for RuntimeHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Workspace ID
    pub id: Uuid,
    
    /// Workspace name
    pub name: String,
    
    /// Workspace path
    pub path: String,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl WorkspaceInfo {
    /// Create workspace info
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            path: path.into(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_creation() {
        let area = Area::new("Living Room");
        
        assert_eq!(area.name, "Living Room");
        assert!(area.parent_id.is_none());
    }

    #[test]
    fn test_device_group_creation() {
        let group = DeviceGroup::new("First Floor");
        
        assert_eq!(group.name, "First Floor");
        assert!(group.description.is_none());
    }

    #[test]
    fn test_runtime_health() {
        let health = RuntimeHealth::new();
        
        assert!(health.healthy);
        assert_eq!(health.connected_devices, 0);
    }
}
