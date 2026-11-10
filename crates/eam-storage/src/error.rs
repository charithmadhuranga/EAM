//! Errors for the storage layer.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Persistence failures.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Could not create the database directory.
    #[error("storage io error: {0}")]
    Io(String),

    /// Pool/connection/query failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// A migration failed to apply. The version identifies the migration.
    #[error("migration {version} failed: {source}")]
    Migration {
        /// Migration version that was being applied.
        version: i64,
        /// Underlying sqlx error.
        source: sqlx::Error,
    },

    /// Row could not be decoded into the domain type.
    #[error("decode error: {0}")]
    Decode(String),
}

impl StorageError {
    /// Build an [`StorageError::Io`] from an I/O failure.
    pub(crate) fn io(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }

    /// Build an [`StorageError::Migration`] for the given version.
    pub(crate) fn migration(version: i64, source: sqlx::Error) -> Self {
        Self::Migration { version, source }
    }

    /// Build an [`StorageError::Decode`] from a serde failure.
    pub(crate) fn decode_str(msg: impl Into<String>) -> Self {
        Self::Decode(msg.into())
    }
}
