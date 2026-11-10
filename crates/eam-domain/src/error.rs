//! Domain error types

use serde::{Deserialize, Serialize};
use std::fmt;

/// Domain error type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Additional details
    pub details: Option<serde_json::Value>,
}

impl Error {
    /// Create a new error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", format!("{} not found", resource.into()))
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::new("CONNECTION_ERROR", message)
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new("TIMEOUT_ERROR", message)
    }

    /// Add details to error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = Error::new("TEST_ERROR", "This is a test error");

        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "This is a test error");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_error_variants() {
        let validation = Error::validation("Invalid value");
        assert_eq!(validation.code, "VALIDATION_ERROR");

        let not_found = Error::not_found("Device");
        assert_eq!(not_found.code, "NOT_FOUND");

        let connection = Error::connection("Failed to connect");
        assert_eq!(connection.code, "CONNECTION_ERROR");

        let timeout = Error::timeout("Command timed out");
        assert_eq!(timeout.code, "TIMEOUT_ERROR");
    }
}
