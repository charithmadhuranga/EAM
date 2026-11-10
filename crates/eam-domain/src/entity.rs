//! Entity domain types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random entity ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Numeric sensor (temperature, humidity, etc.)
    Sensor,
    /// Binary sensor (motion, door, etc.)
    BinarySensor,
    /// Switch entity
    Switch,
    /// Light entity
    Light,
    /// Number entity (controllable numeric value)
    Number,
    /// Select entity (choose from options)
    Select,
    /// Button entity
    Button,
    /// Text sensor
    TextSensor,
    /// Text entity (controllable text)
    Text,
    /// Fan entity
    Fan,
    /// Cover entity (blinds, garage door, etc.)
    Cover,
    /// Climate entity (thermostat, AC, etc.)
    Climate,
    /// Lock entity
    Lock,
    /// Media player entity
    MediaPlayer,
    /// Unknown or unsupported entity type
    Unknown,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sensor => write!(f, "Sensor"),
            Self::BinarySensor => write!(f, "Binary Sensor"),
            Self::Switch => write!(f, "Switch"),
            Self::Light => write!(f, "Light"),
            Self::Number => write!(f, "Number"),
            Self::Select => write!(f, "Select"),
            Self::Button => write!(f, "Button"),
            Self::TextSensor => write!(f, "Text Sensor"),
            Self::Text => write!(f, "Text"),
            Self::Fan => write!(f, "Fan"),
            Self::Cover => write!(f, "Cover"),
            Self::Climate => write!(f, "Climate"),
            Self::Lock => write!(f, "Lock"),
            Self::MediaPlayer => write!(f, "Media Player"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Entity availability status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityAvailability {
    /// Entity is available
    #[default]
    Available,
    /// Entity is unavailable
    Unavailable,
    /// Entity state is stale (not updated recently)
    Stale,
}

/// Entity state quality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateQuality {
    /// State is current and reliable
    #[default]
    Good,
    /// State may be outdated or unreliable
    Poor,
    /// State is unknown
    Unknown,
}

/// Entity metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity identifier
    pub id: EntityId,

    /// Device this entity belongs to
    pub device_id: Uuid,

    /// Native entity key from ESPHome protocol (stable identifier)
    pub native_entity_key: String,

    /// Object ID (optional, used in Home Assistant integration)
    pub object_id: Option<String>,

    /// Entity name
    pub name: String,

    /// Entity type
    pub entity_type: EntityType,

    /// Unit of measurement (for sensors)
    pub unit: Option<String>,

    /// Icon name
    pub icon: Option<String>,

    /// State class (for sensors: measurement, total, etc.)
    pub state_class: Option<String>,

    /// Device class (e.g., temperature, humidity)
    pub device_class: Option<String>,

    /// Entity capabilities (entity-type specific)
    pub capabilities: JsonValue,

    /// Raw metadata from protocol (for forward compatibility)
    pub raw_metadata: JsonValue,

    /// Availability status
    pub availability: EntityAvailability,

    /// Entity creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Entity {
    /// Create a new entity
    pub fn new(
        device_id: Uuid,
        native_entity_key: impl Into<String>,
        name: impl Into<String>,
        entity_type: EntityType,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: EntityId::new(),
            device_id,
            native_entity_key: native_entity_key.into(),
            object_id: None,
            name: name.into(),
            entity_type,
            unit: None,
            icon: None,
            state_class: None,
            device_class: None,
            capabilities: JsonValue::Object(serde_json::Map::new()),
            raw_metadata: JsonValue::Object(serde_json::Map::new()),
            availability: EntityAvailability::Available,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if entity is a sensor
    pub fn is_sensor(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::Sensor | EntityType::BinarySensor | EntityType::TextSensor
        )
    }

    /// Check if entity is controllable
    pub fn is_controllable(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::Switch
                | EntityType::Light
                | EntityType::Number
                | EntityType::Select
                | EntityType::Button
                | EntityType::Text
                | EntityType::Fan
                | EntityType::Cover
                | EntityType::Climate
                | EntityType::Lock
        )
    }
}

/// Current state of an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// Entity ID
    pub entity_id: EntityId,

    /// Current value
    pub value: crate::EntityValue,

    /// Raw value from protocol
    pub raw_value: Option<JsonValue>,

    /// Timestamp from device (if available)
    pub source_timestamp: Option<DateTime<Utc>>,

    /// Timestamp when state was received
    pub received_at: DateTime<Utc>,

    /// State quality
    pub quality: StateQuality,
}

impl EntityState {
    /// Create a new entity state
    pub fn new(entity_id: EntityId, value: crate::EntityValue) -> Self {
        Self {
            entity_id,
            value,
            raw_value: None,
            source_timestamp: None,
            received_at: Utc::now(),
            quality: StateQuality::Good,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let device_id = Uuid::new_v4();
        let entity = Entity::new(
            device_id,
            "temperature",
            "Living Room Temperature",
            EntityType::Sensor,
        );

        assert_eq!(entity.native_entity_key, "temperature");
        assert_eq!(entity.name, "Living Room Temperature");
        assert_eq!(entity.entity_type, EntityType::Sensor);
        assert!(entity.is_sensor());
        assert!(!entity.is_controllable());
    }

    #[test]
    fn test_controllable_entity() {
        let device_id = Uuid::new_v4();
        let entity = Entity::new(
            device_id,
            "living_room_light",
            "Living Room Light",
            EntityType::Switch,
        );

        assert!(!entity.is_sensor());
        assert!(entity.is_controllable());
    }
}
