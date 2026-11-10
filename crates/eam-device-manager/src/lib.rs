//! # eam-device-manager
//!
//! Owns live ESPHome connections for every registered device.
//!
//! - [`ConnectionManager`] — registration, connect/disconnect, status,
//!   automatic reconnect with exponential backoff, state streaming into the
//!   entity registry, and the [`CommandTransport`] used to send commands
//! - [`backoff`] — reconnect delay computation with jitter
//!
//! Devices come from [`eam_discovery`] or manual registration and are persisted
//! through [`eam_storage`]; encryption secrets are referenced (OS credential
//! store) but never handled here.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod backoff;
pub mod error;
pub mod manager;

pub use crate::backoff::{Backoff, BackoffConfig};
pub use crate::error::{is_retryable, Error, Result};
pub use crate::manager::{ConnectionManager, ManagerEvent};
