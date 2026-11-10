//! # eam-discovery
//!
//! Finds ESPHome devices on the local network.
//!
//! Two paths because real networks differ (validated in Phase 0):
//!
//! - **multicast** DNS-SD to `224.0.0.251:5353` — works on open networks
//! - **unicast probe** of a host on port 5353 — the test ESP32 only answered
//!   this way, and it is also what manual registration uses
//!
//! Results are converted to `eam_domain::DiscoveredDevice`; protocol parsing
//! stays in [`dns`].

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod dns;
pub mod error;
pub mod fixtures;
pub mod mdns;

pub use crate::error::{Error, Result};
pub use crate::mdns::{local_ipv4, DiscoveryConfig, DiscoveryService};
