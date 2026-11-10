//! Errors for the entity registry and command service.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Entity and command failures.
#[derive(Debug, Error)]
pub enum Error {
    /// The device owning the entity is not connected.
    #[error("device {0} is not connected")]
    DeviceNotConnected(String),

    /// No entity with this id is registered.
    #[error("entity {0} is not registered")]
    UnknownEntity(String),

    /// Local validation rejected the command.
    #[error("command rejected by local validation: {0}")]
    Validation(String),

    /// The device accepted the command but never echoed the resulting state.
    #[error("command was sent but no state change was observed")]
    NoStateChangeObserved,

    /// Command expired before the device reacted.
    #[error("command timed out after {0} ms")]
    Timeout(u64),

    /// The command was rejected by the device/transport.
    #[error("command failed: {0}")]
    CommandFailed(String),
}
