//! Error types for the runtime.

use thiserror::Error;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Runtime lifecycle failures.
#[derive(Debug, Error)]
pub enum Error {
    /// A supervised task failed (either panicked or returned an error).
    #[error("supervised task '{name}' failed: {source}")]
    TaskFailed {
        /// Name the task was registered under.
        name: String,
        /// The error the task reported.
        source: Box<Error>,
    },

    /// A supervised task panicked.
    #[error("supervised task '{name}' panicked")]
    TaskPanicked {
        /// Name the task was registered under.
        name: String,
    },

    /// Shutdown did not finish within the configured deadline.
    #[error("shutdown timed out after {seconds}s with {running} tasks still running")]
    ShutdownTimeout {
        /// Configured shutdown deadline in seconds.
        seconds: u64,
        /// Tasks still running when the deadline hit.
        running: usize,
    },
}
