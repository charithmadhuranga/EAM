//! # eam-esphome-api
//!
//! ESPHome Native API client. This crate is the **only** place that knows about
//! the ESPHome wire protocol (AGENTS.md: "the ESPHome adapter owns protocol
//! types; keep everything versioned inside it").
//!
//! It exposes protocol types and domain conversions at the boundary; every
//! other crate works with `eam-domain` types.
//!
//! ## Modules
//!
//! - [`client`] — connection, handshake, entity listing, states, commands
//! - [`framing`] — wire framing (`[0x00][len][type][payload]` + varints)
//! - [`message`] — protocol message-ID table (generated from `api.proto`)
//! - [`api_proto`] — prost-generated protobuf types (checked in; regenerate
//!   with `cargo run -p eam-esphome-api --example regen_proto`)
//!
//! ## Phase 0 scope
//!
//! Plaintext handshake, device info, entity listing, live state subscriptions,
//! and switch/number commands. Encryption (Noise) is tracked as DEV-004 and
//! implemented in a later phase.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// Prost-generated protocol types. See `proto/api.proto`; regenerate with
/// `cargo run -p eam-esphome-api --example regen_proto`.
pub mod api_proto;
pub mod client;
pub mod error;
pub mod framing;
pub mod message;

/// Commonly used re-exports.
pub mod prelude {
    pub use crate::client::{ApiClient, DeviceInfo, StateUpdate, DEFAULT_PORT};
    pub use crate::error::{Error, Result};
    pub use crate::message::MessageType;
}

pub use crate::client::{
    ApiClient, DeviceInfo, StateUpdate, DEFAULT_CONNECT_TIMEOUT, DEFAULT_FRAME_TIMEOUT,
    DEFAULT_PORT,
};
pub use crate::error::{Error, Result};
pub use crate::message::MessageType;
