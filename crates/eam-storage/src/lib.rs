//! # eam-storage
//!
//! SQLite persistence for EAM. All SQL lives behind the repository
//! abstractions in [`repo`]; no other crate talks to the database directly.
//!
//! - [`db`] — pool setup (WAL, foreign keys) and migration runner
//! - [`repo`] — device and entity persistence with current-state storage
//!
//! Migrations are versioned SQL files in `migrations/` that are embedded at
//! compile time, so a deployed app carries its schema. Add a new numbered file
//! for each change; never edit an applied migration (AGENTS.md rule 8).

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod db;
pub mod error;
pub mod repo;

pub use db::{open, open_in_memory, run_migrations, schema_version};
pub use error::{Result, StorageError};
pub use repo::{DeviceRepository, EntityRepository, NewDevice};
