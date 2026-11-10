//! Errors for the discovery adapter.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Discovery failures.
#[derive(Debug, Error)]
pub enum Error {
    /// Multicast socket could not be created or joined.
    #[error("network error: {0}")]
    Network(String),

    /// A response could not be parsed.
    #[error("dns parse error: {0}")]
    Dns(String),
}
