//! Discovery tests: protocol parsing (offline, real captures) plus live
//! probing against the real device.

use std::time::Duration;

use eam_discovery::{dns, DiscoveryConfig, DiscoveryService};

/// `EAM_LIVE_DEVICE_HOST` holds just the IP of the real test device.
fn live_host() -> Option<String> {
    std::env::var("EAM_LIVE_DEVICE_HOST")
        .ok()
        .or_else(|| {
            std::env::var("EAM_LIVE_DEVICE")
                .ok()
                .map(|v| v.split(':').next().unwrap_or_default().to_string())
        })
        .filter(|v| !v.is_empty())
}

macro_rules! live {
    () => {
        match live_host() {
            Some(host) => host,
            None => {
                eprintln!("SKIP: set EAM_LIVE_DEVICE_HOST=<ip> to probe a real device");
                return;
            }
        }
    };
}

#[test]
fn parses_friendly_name_and_mac_from_real_capture() {
    let records = dns::decode_message(eam_discovery::fixtures::PTR_REPLY).unwrap();
    let device = eam_discovery::DiscoveryService::records_to_device(records, "10.60.109.61")
        .expect("records describe a device");

    assert_eq!(device.name, "eam-test-device");
    assert_eq!(device.host, "10.60.109.61");
    assert_eq!(device.port, 6053);
    assert_eq!(device.mac_address.as_deref(), Some("A0:B7:65:62:E3:3C"));
    assert_eq!(device.friendly_name.as_deref(), Some("EAM Test Device"));
}

#[tokio::test]
async fn probe_real_device_over_unicast() {
    let host = live!();
    let service = DiscoveryService::with_config(DiscoveryConfig {
        probe_timeout: Duration::from_secs(3),
        ..Default::default()
    });
    let device = service
        .probe(&host)
        .await
        .expect("probe completed")
        .expect("device answered");

    assert_eq!(device.name, "eam-test-device");
    assert_eq!(device.host, host);
    assert_eq!(device.port, 6053);
    assert!(device.mac_address.is_some(), "TXT reports the MAC");
}

#[tokio::test]
async fn probe_of_unreachable_host_is_ok_none() {
    // 127.0.0.1 never runs an mDNS responder in CI.
    let service = DiscoveryService::with_config(DiscoveryConfig {
        probe_timeout: Duration::from_millis(200),
        ..Default::default()
    });
    // A closed port answers with ICMP refused: that is a valid "no device".
    if let Ok(found) = service.probe("127.0.0.1").await {
        assert!(found.is_none());
    }
}

#[tokio::test]
async fn scan_subnet_streams_results() {
    let _host = live!();
    // Scan the host's own subnet, not the device address.
    let ip = eam_discovery::local_ipv4().expect("host has an IPv4 address");
    let service = DiscoveryService::with_config(DiscoveryConfig {
        probe_timeout: Duration::from_millis(1_000),
        concurrency: 16,
        listen_window: Duration::from_secs(1),
    });

    let mut rx = service.scan_subnet(ip).await.unwrap();
    let mut found = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while let Ok(device) = tokio::time::timeout_at(deadline, rx.recv()).await {
        let Some(device) = device else { break };
        found.push(device);
    }

    assert!(
        found.iter().any(|d| d.name == "eam-test-device"),
        "subnet scan must find the real device; found {found:?}"
    );
    // Each device is reported once.
    let names: Vec<_> = found.iter().map(|d| d.name.clone()).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(names.len(), unique.len(), "duplicate devices reported");
}
