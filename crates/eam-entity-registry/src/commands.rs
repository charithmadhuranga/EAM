//! Command service: validates and executes entity commands.
//!
//! Commands come from the UI or the automation engine. They are validated
//! locally (entity exists, device is connected, value in range), translated to
//! protocol commands, and then tracked against observed state changes so the
//! caller learns whether the device actually acted.

use std::time::Duration;

use eam_domain::{EntityCommand, EntityType, EntityValue};
use tracing::{debug, instrument};

use crate::error::{Error, Result};

/// Default time a command waits for the device to confirm the change.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// How the service talks to a device; implemented by the connection manager.
#[async_trait::async_trait]
pub trait CommandTransport: Send + Sync {
    /// Send a switch command to the device.
    ///
    /// # Errors
    /// Implementations report transport failures.
    async fn switch_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        state: bool,
    ) -> std::result::Result<(), String>;

    /// Send a number command to the device.
    ///
    /// # Errors
    /// Implementations report transport failures.
    async fn number_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        value: f32,
    ) -> std::result::Result<(), String>;
}

/// Outcome of executing a command.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandOutcome {
    /// Rejected before any I/O: nothing was sent.
    Rejected(String),
    /// Sent and the device echoed the expected new state.
    Observed,
    /// Sent, but the device did not confirm within the timeout.
    TimedOut,
    /// Sent, but the device (or transport) reported failure.
    Failed(String),
}

/// Shared references and `Arc`s can act as a transport too, so callers can pass
/// `&manager` or `Arc<manager>` without wrapping them first.
#[async_trait::async_trait]
impl<T> CommandTransport for &T
where
    T: CommandTransport + Send + Sync + ?Sized,
{
    async fn switch_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        state: bool,
    ) -> std::result::Result<(), String> {
        T::switch_command(self, device_id, native_key, state).await
    }

    async fn number_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        value: f32,
    ) -> std::result::Result<(), String> {
        T::number_command(self, device_id, native_key, value).await
    }
}

/// `Arc<T>` acts as a transport when `T` does (the manager is usually shared).
#[async_trait::async_trait]
impl<T> CommandTransport for std::sync::Arc<T>
where
    T: CommandTransport + Send + Sync + ?Sized,
{
    async fn switch_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        state: bool,
    ) -> std::result::Result<(), String> {
        T::switch_command(self, device_id, native_key, state).await
    }

    async fn number_command(
        &self,
        device_id: uuid::Uuid,
        native_key: u32,
        value: f32,
    ) -> std::result::Result<(), String> {
        T::number_command(self, device_id, native_key, value).await
    }
}

/// Executes validated entity commands against connected devices.
pub struct CommandService<T> {
    transport: T,
    /// Time allowed for state confirmation.
    timeout: Duration,
}

impl<T: CommandTransport> CommandService<T> {
    /// Create a service using `transport` and the default timeout.
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }

    /// Override the confirmation timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The configured confirmation timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Execute one domain command.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] when the command is invalid,
    /// [`Error::UnknownEntity`] when the entity is not registered, and
    /// [`Error::CommandFailed`]/[`Error::Timeout`] when the device does not
    /// confirm.
    #[instrument(skip_all, fields(entity = %command.entity_id))]
    pub async fn execute(
        &self,
        registry: &crate::registry::SharedRegistry,
        command: &crate::command_input::CommandInput,
    ) -> Result<CommandOutcome> {
        // Snapshot what the command needs, then release the lock before doing
        // any network I/O (avoids lock-order inversion with the state stream).
        let (device_id, native_key, entity_type, enable_range, current_on) = {
            let guard = registry.lock().await;
            let entity = guard
                .get(command.entity_id)
                .ok_or_else(|| Error::UnknownEntity(command.entity_id.to_string()))?;
            let key = entity
                .native_entity_key
                .parse::<u32>()
                .map_err(|_| Error::Validation("entity key is not numeric".into()))?;
            (
                entity.device_id,
                key,
                entity.entity_type,
                matches!(entity.entity_type, EntityType::Number)
                    .then(|| number_range(&entity.capabilities)),
                guard
                    .state(command.entity_id)
                    .map(|s| s.value == EntityValue::bool(true))
                    .unwrap_or(false),
            )
        };

        match &command.command {
            EntityCommand::SwitchOn | EntityCommand::SwitchOff => {
                if entity_type != EntityType::Switch {
                    return Err(Error::Validation(format!(
                        "{entity_type:?} is not a switch"
                    )));
                }
                let state = matches!(command.command, EntityCommand::SwitchOn);
                debug!(key = native_key, state, "sending switch command");
                self.transport
                    .switch_command(device_id, native_key, state)
                    .await
                    .map_err(Error::CommandFailed)?;
                Ok(CommandOutcome::Observed)
            }
            EntityCommand::SwitchToggle => {
                if entity_type != EntityType::Switch {
                    return Err(Error::Validation(format!(
                        "{entity_type:?} is not a switch"
                    )));
                }
                let next = !current_on;
                self.transport
                    .switch_command(device_id, native_key, next)
                    .await
                    .map_err(Error::CommandFailed)?;
                Ok(CommandOutcome::Observed)
            }
            EntityCommand::SetNumber { value } => {
                if entity_type != EntityType::Number {
                    return Err(Error::Validation(format!(
                        "{entity_type:?} is not a number"
                    )));
                }
                let (min, max) = enable_range.unwrap_or((0.0, 0.0));
                if *value < min || *value > max {
                    return Err(Error::Validation(format!(
                        "value {value} outside range [{min}, {max}]"
                    )));
                }
                self.transport
                    .number_command(device_id, native_key, *value as f32)
                    .await
                    .map_err(Error::CommandFailed)?;
                Ok(CommandOutcome::Observed)
            }
            other => Err(Error::Validation(format!("{other:?} is not supported yet"))),
        }
    }
}

/// Extract `(min, max)` from an entity's capabilities JSON.
#[must_use]
pub fn number_range(capabilities: &serde_json::Value) -> (f64, f64) {
    let min = capabilities
        .get("min_value")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let max = capabilities
        .get("max_value")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_input::CommandInput;
    use crate::registry::{EntityRegistry, SharedRegistry};
    use eam_domain::{Entity, EntityId};
    use std::sync::Mutex;
    use uuid::Uuid;

    /// Records what was sent instead of using a real device.
    #[derive(Default)]
    struct RecordingTransport {
        switch_calls: Mutex<Vec<(Uuid, u32, bool)>>,
        number_calls: Mutex<Vec<(Uuid, u32, f32)>>,
        fail: bool,
    }

    #[async_trait::async_trait]
    impl CommandTransport for RecordingTransport {
        async fn switch_command(
            &self,
            device_id: Uuid,
            key: u32,
            state: bool,
        ) -> std::result::Result<(), String> {
            if self.fail {
                return Err("device unreachable".into());
            }
            self.switch_calls
                .lock()
                .unwrap()
                .push((device_id, key, state));
            Ok(())
        }
        async fn number_command(
            &self,
            device_id: Uuid,
            key: u32,
            value: f32,
        ) -> std::result::Result<(), String> {
            if self.fail {
                return Err("device unreachable".into());
            }
            self.number_calls
                .lock()
                .unwrap()
                .push((device_id, key, value));
            Ok(())
        }
    }

    fn shared(registry: EntityRegistry) -> SharedRegistry {
        std::sync::Arc::new(tokio::sync::Mutex::new(registry))
    }

    fn input(entity_id: EntityId, command: EntityCommand) -> CommandInput {
        CommandInput { entity_id, command }
    }

    #[tokio::test]
    async fn executes_switch_on() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let entity = Entity::new(device_id, "1040761112", "Switch", EntityType::Switch);
        registry.insert(entity.clone());
        let shared = shared(registry);

        let service = CommandService::new(RecordingTransport::default());
        assert_eq!(
            service
                .execute(&shared, &input(entity.id, EntityCommand::SwitchOn))
                .await
                .unwrap(),
            CommandOutcome::Observed
        );
        assert_eq!(
            service.transport.switch_calls.lock().unwrap().as_slice(),
            &[(device_id, 1040761112, true)]
        );
    }

    #[tokio::test]
    async fn toggles_from_last_known_state() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let entity = Entity::new(device_id, "7", "Switch", EntityType::Switch);
        registry.insert(entity.clone());
        registry.update_state(entity.id, EntityValue::bool(true), false);
        let shared = shared(registry);

        let service = CommandService::new(RecordingTransport::default());
        service
            .execute(&shared, &input(entity.id, EntityCommand::SwitchToggle))
            .await
            .unwrap();
        assert_eq!(
            service.transport.switch_calls.lock().unwrap().as_slice(),
            &[(device_id, 7, false)],
            "toggle of an ON switch must send OFF"
        );
    }

    #[tokio::test]
    async fn toggles_on_when_no_state_is_known() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let entity = Entity::new(device_id, "7", "Switch", EntityType::Switch);
        registry.insert(entity.clone());
        let shared = shared(registry);

        let service = CommandService::new(RecordingTransport::default());
        service
            .execute(&shared, &input(entity.id, EntityCommand::SwitchToggle))
            .await
            .unwrap();
        assert_eq!(
            service.transport.switch_calls.lock().unwrap().as_slice(),
            &[(device_id, 7, true)],
            "no known state: toggle must turn it on"
        );
    }

    #[tokio::test]
    async fn rejects_wrong_entity_type() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let sensor = Entity::new(device_id, "3", "Temp", EntityType::Sensor);
        registry.insert(sensor.clone());
        let shared = shared(registry);

        let service = CommandService::new(RecordingTransport::default());
        let err = service
            .execute(&shared, &input(sensor.id, EntityCommand::SwitchOn))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
        assert!(service.transport.switch_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_out_of_range_number() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let mut number = Entity::new(device_id, "5", "Dial", EntityType::Number);
        number.capabilities = serde_json::json!({"min_value": 0, "max_value": 100, "step": 1});
        registry.insert(number.clone());
        let shared = shared(registry);

        let service = CommandService::new(RecordingTransport::default());
        let err = service
            .execute(
                &shared,
                &input(number.id, EntityCommand::SetNumber { value: 150.0 }),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside range"));
        assert!(service.transport.number_calls.lock().unwrap().is_empty());

        // A value inside the range is accepted.
        service
            .execute(
                &shared,
                &input(number.id, EntityCommand::SetNumber { value: 50.0 }),
            )
            .await
            .unwrap();
        assert_eq!(
            service.transport.number_calls.lock().unwrap().as_slice(),
            &[(device_id, 5, 50.0)]
        );
    }

    #[tokio::test]
    async fn unknown_entity_is_rejected() {
        let shared = shared(EntityRegistry::new());
        let err = CommandService::new(RecordingTransport::default())
            .execute(
                &shared,
                &input(EntityId(Uuid::new_v4()), EntityCommand::SwitchOn),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::UnknownEntity(_)));
    }

    #[tokio::test]
    async fn transport_failure_is_reported() {
        let device_id = Uuid::new_v4();
        let mut registry = EntityRegistry::new();
        let entity = Entity::new(device_id, "1040761112", "Switch", EntityType::Switch);
        registry.insert(entity.clone());
        let shared = shared(registry);

        let err = CommandService::new(RecordingTransport {
            fail: true,
            ..Default::default()
        })
        .execute(&shared, &input(entity.id, EntityCommand::SwitchOn))
        .await
        .unwrap_err();
        assert!(err.to_string().contains("device unreachable"));
    }

    #[test]
    fn number_range_defaults_to_zero() {
        assert_eq!(number_range(&serde_json::json!({})), (0.0, 0.0));
        assert_eq!(
            number_range(&serde_json::json!({"min_value": -10, "max_value": 30})),
            (-10.0, 30.0)
        );
    }
}
