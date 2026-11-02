//! eam-domain - Pure domain types for ESPHome Automation Manager
//!
//! This crate contains domain types with no external dependencies except serde.
//! It defines the core domain model: Device, Entity, Value, Command, Error.

#![warn(missing_docs)]

pub mod command;
pub mod device;
pub mod entity;
pub mod error;
pub mod types;
pub mod value;

pub use command::*;
pub use device::*;
pub use entity::*;
pub use error::*;
pub use types::*;
pub use value::*;
