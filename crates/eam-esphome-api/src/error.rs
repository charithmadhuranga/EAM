//! Errors for the ESPHome Native API client.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything the ESPHome adapter can report.
#[derive(Debug, Error)]
pub enum Error {
    /// Socket-level failure.
    #[error("connection error: {0}")]
    Connection(String),

    /// Could not complete the TLS/plaintext handshake in time.
    #[error("handshake timed out")]
    HandshakeTimeout,

    /// Device speaks an incompatible API version.
    #[error("unsupported API version {major}.{minor} (need >= {required}.{minor_required})")]
    UnsupportedVersion {
        /// Major version reported by the device.
        major: u32,
        /// Minor version reported by the device.
        minor: u32,
        /// Minimum supported major version.
        required: u32,
        /// Minimum supported minor version.
        minor_required: u32,
    },

    /// Wire framing did not match the protocol.
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    /// Message type ID is not in this protocol version's table.
    #[error("unknown message id {0}")]
    UnknownMessage(u16),

    /// Protobuf decode/encode failure.
    #[error("message decode error: {0}")]
    Decode(String),

    /// No matching entity found for a command key.
    #[error("no entity with key {0}")]
    UnknownKey(u32),

    /// Device refused the command.
    #[error("device rejected command: {0}")]
    CommandRejected(String),

    /// Expected a specific response but the device failed to send it.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}

impl Error {
    /// Build a [`Error::ProtocolViolation`] from any displayable cause.
    pub(crate) fn protocol(msg: impl Into<String>) -> Self {
        Self::ProtocolViolation(msg.into())
    }

    /// Build a [`Error::ProtocolViolation`] for an oversized frame.
    pub(crate) fn frame_too_large(declared: u64) -> Self {
        Self::ProtocolViolation(format!("frame of {declared} bytes exceeds maximum"))
    }

    /// Build a [`Error::Decode`] from a protobuf failure.
    pub(crate) fn decode(err: impl std::error::Error) -> Self {
        Self::Decode(err.to_string())
    }

    /// True when the connection should be retried/re-established.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Connection(_) | Self::HandshakeTimeout | Self::ProtocolViolation(_)
        )
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Connection(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoverable_errors_classified() {
        assert!(Error::Connection("reset".into()).is_recoverable());
        assert!(Error::HandshakeTimeout.is_recoverable());
        assert!(!Error::UnsupportedVersion {
            major: 2,
            minor: 0,
            required: 1,
            minor_required: 16,
        }
        .is_recoverable());
    }

    #[test]
    fn displays_useful_message() {
        let err = Error::UnsupportedVersion {
            major: 2,
            minor: 0,
            required: 1,
            minor_required: 16,
        };
        assert!(err.to_string().contains("2.0"));
    }
}
