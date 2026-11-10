//! Errors for the device manager.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Connection management failures.
#[derive(Debug, Error)]
pub enum Error {
    /// The device is not known to the manager.
    #[error("device {0} is not registered")]
    UnknownDevice(String),

    /// A device with this node name already exists.
    #[error("device node name {0} is already registered")]
    DuplicateNodeName(String),

    /// Connecting to the device failed after exhausting retries.
    #[error("could not connect to {node}: {reason}")]
    ConnectFailed {
        /// Node name or address.
        node: String,
        /// Last failure reason.
        reason: String,
    },

    /// The device is not currently connected.
    #[error("device {0} is not connected")]
    NotConnected(String),

    /// Persistence failure.
    #[error("storage error: {0}")]
    Storage(#[from] eam_storage::StorageError),

    /// Discovery failure.
    #[error("discovery error: {0}")]
    Discovery(#[from] eam_discovery::Error),
}

/// Whether an error is worth retrying (network-level) or is permanent.
#[must_use]
pub fn is_retryable(err: &Error) -> bool {
    match err {
        Error::ConnectFailed { .. } => true,
        Error::NotConnected(_) => true,
        other => !matches!(other, Error::UnknownDevice(_) | Error::DuplicateNodeName(_)),
    }
}
