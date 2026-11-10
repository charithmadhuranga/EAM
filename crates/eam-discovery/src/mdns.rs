//! Device discovery for ESPHome devices.
//!
//! Two paths, because real networks differ:
//!
//! 1. **Multicast** — the standard DNS-SD query to `224.0.0.251:5353`. Many
//!    networks allow this; some (and some APs) filter it.
//! 2. **Unicast probe** — a direct DNS-SD query to a host's port 5353. The
//!    Phase 0 ESP32 answered only this way, so it is the reliable path.
//!
//! [`DiscoveryService`] combines a multicast listen with a bounded unicast
//! scan of the local subnet, plus explicit `probe` for manual registration.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use eam_domain::DiscoveredDevice;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::dns::{self, RecordData, RecordType, ESPHOME_SERVICE_TYPE, MDNS_ADDR, MDNS_PORT};
use crate::error::{Error, Result};

/// Default per-host timeout for unicast probes.
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_millis(1_000);
/// Default number of hosts probed concurrently.
///
/// Deliberately small: measured against a real ESP32, an mDNS burst wider than
/// ~16 in-flight queries gets dropped by the device responder (17/32 replies
/// at concurrency 32), which makes discovery miss the device entirely.
/// Bounded here per AGENTS.md rule 9 as well.
pub const DEFAULT_CONCURRENCY: usize = 16;
/// How long multicast responses are collected before reporting.
pub const DEFAULT_LISTEN_WINDOW: Duration = Duration::from_secs(2);

/// Discovery configuration.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Per-host probe timeout.
    pub probe_timeout: Duration,
    /// Concurrent probe limit.
    pub concurrency: usize,
    /// How long to listen for multicast answers.
    pub listen_window: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            probe_timeout: DEFAULT_PROBE_TIMEOUT,
            concurrency: DEFAULT_CONCURRENCY,
            listen_window: DEFAULT_LISTEN_WINDOW,
        }
    }
}

/// Finds ESPHome devices on the network.
pub struct DiscoveryService {
    config: DiscoveryConfig,
}

impl DiscoveryService {
    /// Create a service with the default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
        }
    }

    /// Create a service with a custom configuration.
    #[must_use]
    pub const fn with_config(config: DiscoveryConfig) -> Self {
        Self { config }
    }

    /// Probe one host directly with a DNS-SD query on port 5353.
    ///
    /// Used for manual registration and by the subnet scan. Resolves the
    /// device's real service port from the SRV record.
    ///
    /// # Errors
    /// Returns [`Error::Network`] on socket failures and [`Error::Dns`] when
    /// the reply cannot be parsed. A host that simply does not answer yields
    /// `Ok(None)`.
    pub async fn probe(&self, host: impl Into<String>) -> Result<Option<DiscoveredDevice>> {
        let host = host.into();
        let target: SocketAddr = (
            host.parse::<Ipv4Addr>()
                .map_err(|e| Error::Network(format!("{host} is not an IPv4 address: {e}")))?,
            MDNS_PORT,
        )
            .into();

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        let query = dns::encode_ptr_query(ESPHOME_SERVICE_TYPE)?;
        socket
            .send_to(&query, target)
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        let mut buf = [0u8; 4096];
        let (len, _peer) =
            tokio::time::timeout(self.config.probe_timeout, socket.recv_from(&mut buf))
                .await
                .map_err(|_| Error::Network(format!("{host} did not answer")))?
                .map_err(|e| Error::Network(e.to_string()))?;

        let records = dns::decode_message(&buf[..len])?;
        Ok(Self::records_to_device(records, host.as_str()))
    }

    /// Convert decoded records into a [`DiscoveredDevice`], if they describe one.
    pub fn records_to_device(
        records: Vec<dns::ResourceRecord>,
        host: &str,
    ) -> Option<DiscoveredDevice> {
        let ptr = records.iter().find(|r| r.record_type == RecordType::Ptr)?;
        let instance = match &ptr.data {
            RecordData::Ptr(target) => target
                .split('.')
                .next()
                .unwrap_or(target.as_str())
                .to_string(),
            _ => return None,
        };

        let port = records
            .iter()
            .find_map(|r| match &r.data {
                RecordData::Srv { port, .. } => Some(*port),
                _ => None,
            })
            .unwrap_or(MDNS_PORT);

        let mac_address = records
            .iter()
            .find_map(|r| match &r.data {
                RecordData::Txt(values) => {
                    values.iter().find_map(|v| v.strip_prefix("mac=")).map(|m| {
                        // TXT reports a packed MAC (a0b76562e33c); format it.
                        let mut out = String::with_capacity(17);
                        for (i, ch) in m.chars().enumerate() {
                            if i > 0 && i % 2 == 0 {
                                out.push(':');
                            }
                            out.push(ch.to_ascii_uppercase());
                        }
                        out
                    })
                }
                _ => None,
            })
            .filter(|m| !m.is_empty());

        let friendly_name = records.iter().find_map(|r| match &r.data {
            RecordData::Txt(values) => values
                .iter()
                .find_map(|v| v.strip_prefix("friendly_name="))
                .map(str::to_string),
            _ => None,
        });

        Some(DiscoveredDevice {
            name: instance,
            host: host.to_string(),
            port,
            mac_address,
            friendly_name,
            source: eam_domain::DiscoverySource::Mdns,
        })
    }

    /// Scan the local `/24` subnet for devices, streaming results as found.
    ///
    /// Bounded to `config.concurrency` in-flight probes so a routed scan stays
    /// responsive. Duplicate node names are suppressed.
    ///
    /// # Errors
    /// Returns [`Error`] when the local address cannot be determined.
    pub async fn scan_subnet(
        &self,
        local_ip: Ipv4Addr,
    ) -> Result<mpsc::Receiver<DiscoveredDevice>> {
        let (tx, rx) = mpsc::channel(64);
        let config = self.config.clone();

        info!(%local_ip, "scanning /24 subnet for ESPHome devices");
        tokio::spawn(async move {
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.concurrency));
            let mut tasks = Vec::new();
            for last in 1..=254u8 {
                let ip = Ipv4Addr::from([
                    local_ip.octets()[0],
                    local_ip.octets()[1],
                    local_ip.octets()[2],
                    last,
                ]);
                if ip == local_ip {
                    continue;
                }
                let permit = semaphore.clone().acquire_owned().await;
                let Ok(permit) = permit else { break };
                let timeout = config.probe_timeout;
                let tx = tx.clone();
                tasks.push(tokio::spawn(async move {
                    let _permit = permit;
                    if let Ok(Some(device)) = probe_one(ip, timeout).await {
                        let _ = tx.send(device).await;
                    }
                }));
            }
            for task in tasks {
                let _ = task.await;
            }
        });

        Ok(rx)
    }

    /// Listen for multicast mDNS responses for `config.listen_window`.
    ///
    /// # Errors
    /// Returns [`Error`] when the multicast socket cannot be set up.
    pub async fn listen_multicast(&self, local_ip: Ipv4Addr) -> Result<Vec<DiscoveredDevice>> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, MDNS_PORT))
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        let multicast_ip: Ipv4Addr = MDNS_ADDR.parse().expect("valid mDNS address");
        socket
            .join_multicast_v4(multicast_ip, local_ip)
            .map_err(|e| Error::Network(format!("join multicast: {e}")))?;

        let query = dns::encode_ptr_query(ESPHOME_SERVICE_TYPE)?;
        socket
            .send_to(&query, (multicast_ip, MDNS_PORT))
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        let mut buf = [0u8; 4096];
        let mut devices: HashMap<String, DiscoveredDevice> = HashMap::new();
        let deadline = tokio::time::Instant::now() + self.config.listen_window;

        loop {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or_default();
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, peer))) => {
                    if let Ok(records) = dns::decode_message(&buf[..len]) {
                        if let Some(device) =
                            Self::records_to_device(records, &peer.ip().to_string())
                        {
                            devices.insert(device.name.clone(), device);
                        }
                    }
                }
                Ok(Err(e)) => {
                    debug!("multicast read failed: {e}");
                    break;
                }
                Err(_) => break, // window elapsed
            }
        }

        Ok(devices.into_values().collect())
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine this host's primary IPv4 address.
///
/// Connects a UDP socket toward a public address (no traffic is sent) so the OS
/// picks the interface it would use for outbound traffic.
///
/// # Errors
/// Returns [`Error::Network`] when no usable interface exists.
pub fn local_ipv4() -> Result<Ipv4Addr> {
    let socket =
        std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| Error::Network(e.to_string()))?;
    socket
        .connect("8.8.8.8:53")
        .map_err(|e| Error::Network(e.to_string()))?;
    match socket.local_addr() {
        Ok(SocketAddr::V4(addr)) => Ok(*addr.ip()),
        _ => Err(Error::Network("no IPv4 address available".into())),
    }
}

/// One bounded unicast probe, used by the subnet scan.
async fn probe_one(ip: Ipv4Addr, timeout: Duration) -> Result<Option<DiscoveredDevice>> {
    let target: SocketAddr = (ip, MDNS_PORT).into();
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    let query = dns::encode_ptr_query(ESPHOME_SERVICE_TYPE)?;
    socket
        .send_to(&query, target)
        .await
        .map_err(|e| Error::Network(e.to_string()))?;

    let mut buf = [0u8; 4096];
    let (len, _peer) = match tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await {
        Ok(Ok(v)) => v,
        _ => return Ok(None), // no answer in time: host is not an ESPHome device
    };

    let records = dns::decode_message(&buf[..len])?;
    Ok(DiscoveryService::records_to_device(
        records,
        &ip.to_string(),
    ))
}
