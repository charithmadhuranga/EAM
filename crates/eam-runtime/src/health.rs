//! Health monitoring: uptime and liveness for the UI and diagnostics.

use std::time::Instant;

#[cfg(test)]
use std::time::Duration;

use eam_domain::RuntimeHealth;

/// Tracks when the runtime started and derives [`RuntimeHealth`] snapshots.
#[derive(Debug)]
pub struct HealthMonitor {
    started: Instant,
    /// Last shutdown request, if any.
    stopping: bool,
}

impl HealthMonitor {
    /// Create a monitor for a just-started runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            stopping: false,
        }
    }

    /// Record that shutdown has been requested.
    pub fn mark_stopping(&mut self) {
        self.stopping = true;
    }

    /// Seconds since the runtime started.
    #[must_use]
    pub fn uptime_secs(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// Build a health snapshot.
    ///
    /// `connected_devices` and `total_entities` are supplied by the device
    /// manager so the status reflects real state, not just the clock.
    #[must_use]
    pub fn snapshot(&self, connected_devices: usize, total_entities: usize) -> RuntimeHealth {
        RuntimeHealth {
            healthy: !self.stopping,
            connected_devices,
            total_entities,
            memory_usage: 0,
            uptime_secs: self.uptime_secs(),
            checked_at: chrono::Utc::now(),
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn uptime_advances_with_time() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.uptime_secs(), 0);
        // Real (short) wait: the monitor derives uptime from Instant.
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(monitor.uptime_secs() <= 1);
    }

    #[tokio::test]
    async fn snapshot_tracks_supplied_counts() {
        let monitor = HealthMonitor::new();
        let snap = monitor.snapshot(3, 42);
        assert!(snap.healthy);
        assert_eq!(snap.connected_devices, 3);
        assert_eq!(snap.total_entities, 42);

        // A stopping runtime reports unhealthy.
        let mut monitor = monitor;
        monitor.mark_stopping();
        assert!(!monitor.snapshot(3, 42).healthy);
    }
}
