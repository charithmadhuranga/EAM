//! In-process event bus.
//!
//! A broadcast channel so many consumers (UI emitters, history recorder,
//! automation engine) see the same domain events without the producers knowing
//! about them. Lagging receivers drop the oldest events rather than blocking
//! publishers — bounded queues per AGENTS.md rule 9.

/// Capacity per receiver; producers never block, slow readers lose old events.
pub const DEFAULT_CAPACITY: usize = 1_024;

/// A domain event carried by the bus.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    /// A device changed connection status.
    DeviceStatus {
        /// Device that changed.
        device_id: String,
        /// New status, as a domain string (`"online"`, `"offline"`, …).
        status: String,
    },
    /// An entity's state changed.
    StateChanged {
        /// Entity id.
        entity_id: String,
        /// New value as a domain value (JSON).
        value: serde_json::Value,
    },
    /// Runtime lifecycle transition.
    Lifecycle {
        /// Phase, e.g. `"starting"` or `"stopping"`.
        phase: String,
    },
}

/// Publisher half of the event bus.
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<RuntimeEvent>,
}

impl EventBus {
    /// Create a bus with [`DEFAULT_CAPACITY`] per receiver.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a bus with a custom per-receiver capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _receiver) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event. Never blocks; drops for receivers that fell behind.
    pub fn publish(&self, event: RuntimeEvent) {
        // Ignored on purpose: zero receivers is normal when the UI is closed.
        let _ = self.sender.send(event);
    }

    /// Subscribe to future events.
    #[must_use]
    pub fn subscribe(&self) -> EventBusReceiver {
        EventBusReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    /// Number of live subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Consumer half of the event bus.
#[derive(Debug)]
pub struct EventBusReceiver {
    receiver: tokio::sync::broadcast::Receiver<RuntimeEvent>,
}

impl EventBusReceiver {
    /// Wait for the next event. Returns `None` when the bus is closed.
    ///
    /// # Errors
    /// Returns [`EventBusError::Lagged`] when this receiver fell behind; the
    /// caller should keep going (missed events are logged, not fatal).
    pub async fn next(&mut self) -> Option<Result<RuntimeEvent, EventBusError>> {
        match self.receiver.recv().await {
            Ok(event) => Some(Ok(event)),
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                Some(Err(EventBusError::Lagged(n)))
            }
        }
    }
}

/// Receive-side failures.
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    /// Receiver missed `n` messages because the publisher ran ahead.
    #[error("receiver lagged behind by {0} messages")]
    Lagged(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_reaches_all_subscribers() {
        let bus = EventBus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        bus.publish(RuntimeEvent::Lifecycle {
            phase: "starting".into(),
        });

        assert_eq!(
            a.next().await.unwrap().unwrap(),
            RuntimeEvent::Lifecycle {
                phase: "starting".into()
            }
        );
        assert!(b.next().await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_fine() {
        let bus = EventBus::new();
        bus.publish(RuntimeEvent::Lifecycle {
            phase: "ignored".into(),
        });
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn receiver_reports_lagging_instead_of_blocking() {
        let bus = EventBus::with_capacity(2);
        let mut rx = bus.subscribe();
        for i in 0..5 {
            bus.publish(RuntimeEvent::DeviceStatus {
                device_id: format!("d{i}"),
                status: "online".into(),
            });
        }
        // The first read must not deadlock; it may report lag.
        let first = rx.next().await.unwrap();
        assert!(first.is_ok() || matches!(first, Err(EventBusError::Lagged(_))));
    }

    #[tokio::test]
    async fn bus_close_ends_the_stream() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        drop(bus);
        assert!(rx.next().await.is_none());
    }
}
