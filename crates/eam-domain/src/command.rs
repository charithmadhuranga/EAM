//! Entity command types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EntityId, EntityValue};

/// Unique identifier for a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub Uuid);

impl CommandId {
    /// Create a new random command ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CommandId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Command status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// Command was rejected by local validation
    Rejected,
    
    /// Command is queued for execution
    Queued,
    
    /// Command was sent to device
    Sent,
    
    /// Command was acknowledged by transport
    Acknowledged,
    
    /// State change observed
    Observed,
    
    /// Command timed out
    TimedOut,
    
    /// Device unavailable
    DeviceUnavailable,
    
    /// Command failed
    Failed,
}

impl std::fmt::Display for CommandStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected => write!(f, "Rejected"),
            Self::Queued => write!(f, "Queued"),
            Self::Sent => write!(f, "Sent"),
            Self::Acknowledged => write!(f, "Acknowledged"),
            Self::Observed => write!(f, "Observed"),
            Self::TimedOut => write!(f, "Timed Out"),
            Self::DeviceUnavailable => write!(f, "Device Unavailable"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Command origin - who initiated the command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOrigin {
    /// User initiated from UI
    User {
        user_id: Option<String>,
    },
    
    /// Automation initiated
    Automation {
        automation_id: Uuid,
        run_id: Uuid,
    },
    
    /// System initiated
    System,
}

/// Entity command kind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum EntityCommand {
    /// Turn switch on
    SwitchOn,
    
    /// Turn switch off
    SwitchOff,
    
    /// Toggle switch
    SwitchToggle,
    
    /// Set numeric value
    SetNumber {
        value: f64,
    },
    
    /// Set select option
    SelectOption {
        option: String,
    },
    
    /// Press button
    ButtonPress,
    
    /// Set light state
    SetLight {
        state: LightState,
    },
    
    /// Set fan state
    SetFan {
        state: FanState,
    },
    
    /// Set climate state
    SetClimate {
        state: ClimateState,
    },
    
    /// Lock
    Lock,
    
    /// Unlock
    Unlock,
    
    /// Open cover
    CoverOpen,
    
    /// Close cover
    CoverClose,
    
    /// Stop cover
    CoverStop,
    
    /// Set cover position
    CoverSetPosition {
        position: f64, // 0-100%
    },
}

/// Light state command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightState {
    /// On/off state
    pub state: bool,
    
    /// Brightness (0-255)
    pub brightness: Option<u8>,
    
    /// Color temperature in mireds
    pub color_temp: Option<u16>,
    
    /// RGB color
    pub rgb: Option<(u8, u8, u8)>,
    
    /// Effect name
    pub effect: Option<String>,
}

/// Fan state command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanState {
    /// On/off state
    pub state: bool,
    
    /// Speed (0-100%)
    pub speed: Option<u8>,
    
    /// Oscillation
    pub oscillating: Option<bool>,
}

/// Climate state command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateState {
    /// Mode (off, auto, cool, heat, etc.)
    pub mode: Option<String>,
    
    /// Target temperature
    pub target_temperature: Option<f64>,
    
    /// Fan mode
    pub fan_mode: Option<String>,
    
    /// Swing mode
    pub swing_mode: Option<String>,
}

/// Command to be executed on an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// Unique command identifier
    pub id: CommandId,
    
    /// Target entity
    pub entity_id: EntityId,
    
    /// Command to execute
    pub command: EntityCommand,
    
    /// When command was requested
    pub requested_at: DateTime<Utc>,
    
    /// Who requested the command
    pub requested_by: CommandOrigin,
    
    /// Command timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Current status
    pub status: CommandStatus,
}

impl Command {
    /// Create a new command
    pub fn new(entity_id: EntityId, command: EntityCommand) -> Self {
        Self {
            id: CommandId::new(),
            entity_id,
            command,
            requested_at: Utc::now(),
            requested_by: CommandOrigin::System,
            timeout_ms: 5000,
            status: CommandStatus::Queued,
        }
    }
    
    /// Set command origin to user
    pub fn by_user(mut self, user_id: Option<String>) -> Self {
        self.requested_by = CommandOrigin::User { user_id };
        self
    }
    
    /// Set command timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let entity_id = EntityId::new();
        let command = Command::new(entity_id, EntityCommand::SwitchOn);
        
        assert_eq!(command.status, CommandStatus::Queued);
        assert_eq!(command.timeout_ms, 5000);
    }

    #[test]
    fn test_command_with_options() {
        let entity_id = EntityId::new();
        let command = Command::new(entity_id, EntityCommand::SwitchOn)
            .by_user(Some("user-123".to_string()))
            .with_timeout(10000);
        
        assert_eq!(command.timeout_ms, 10000);
        assert!(matches!(command.requested_by, CommandOrigin::User { .. }));
    }
}
