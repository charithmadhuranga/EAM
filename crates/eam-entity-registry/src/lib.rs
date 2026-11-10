//! # eam-entity-registry
//!
//! Registry of known entities plus their live state, and the service that
//! executes commands against them.
//!
//! - [`registry`] — in-memory entity list, current states, state broadcast
//! - [`commands`] — local validation and execution of entity commands
//!
//! The registry never talks to devices itself; the connection manager feeds
//! states in and provides the [`CommandTransport`] implementation.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod command_input;
pub mod commands;
pub mod error;
pub mod registry;

pub use crate::command_input::CommandInput;
pub use crate::commands::{number_range, CommandOutcome, CommandService, CommandTransport};
pub use crate::error::{Error, Result};
pub use crate::registry::{EntityRegistry, StateUpdate};
