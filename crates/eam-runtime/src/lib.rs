//! # eam-runtime
//!
//! Application lifecycle: a task supervisor that owns every long-running
//! service, a health snapshot, and an in-process event bus.
//!
//! The runtime is independent of the UI (AGENTS.md: "Automations run
//! independently of UI navigation state"): it starts when the app starts and
//! keeps running while the window navigates.
//!
//! - [`TaskSupervisor`] — spawn, track, and stop background services
//! - [`HealthMonitor`] — uptime/health snapshot for the UI and diagnostics
//! - [`event_bus`] — broadcast channel for state updates and domain events

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod event_bus;
pub mod health;
pub mod task;

pub use error::{Error, Result};
pub use health::HealthMonitor;
pub use task::TaskSupervisor;
