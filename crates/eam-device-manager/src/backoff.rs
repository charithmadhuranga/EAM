//! Exponential backoff with bounded jitter and a cap.
//!
//! Used for reconnect attempts. Deterministic given a seed so tests do not
//! flake, and jitter stays within ±20% of the computed delay.

use std::time::Duration;

/// Backoff configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackoffConfig {
    /// Delay before the first retry.
    pub initial: Duration,
    /// Upper bound for any delay.
    pub max: Duration,
    /// Multiplier applied after each attempt.
    pub factor: f64,
    /// Maximum jitter fraction (0.0–1.0) of the computed delay.
    pub jitter: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial: Duration::from_millis(500),
            max: Duration::from_secs(30),
            factor: 2.0,
            jitter: 0.2,
        }
    }
}

/// Computes successive reconnect delays.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    config: BackoffConfig,
    /// Attempt number (0 = first attempt).
    attempt: u32,
}

impl Backoff {
    /// Create a backoff with the given configuration.
    #[must_use]
    pub const fn new(config: BackoffConfig) -> Self {
        Self { config, attempt: 0 }
    }

    /// Create a backoff with defaults.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BackoffConfig::default())
    }

    /// Reset the attempt counter (after a successful connection).
    pub const fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Current attempt number.
    #[must_use]
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Delay for the *next* attempt, and advance the counter.
    ///
    /// `jitter_source` (0.0–1.0) scales the jitter fraction deterministically.
    pub fn next_delay(&mut self, jitter_source: f64) -> Duration {
        let attempt = self.attempt;
        self.attempt = self.attempt.saturating_add(1);

        let growth = self.config.factor.powi(attempt as i32);
        let base_ms = self.config.initial.as_millis() as f64 * growth;
        let capped_ms = base_ms.min(self.config.max.as_millis() as f64);

        let jitter_fraction = self.config.jitter.clamp(0.0, 1.0);
        // Deterministic ±jitter around the computed delay.
        let signed = (jitter_source.clamp(0.0, 1.0) - 0.5) * 2.0; // −1..=1
        let jittered = capped_ms * (1.0 + signed * jitter_fraction);
        Duration::from_millis(jittered.max(1.0) as u64)
    }

    /// Async sleep for the next delay. Returns the delay slept.
    pub async fn wait_next(&mut self, jitter_source: f64) -> Duration {
        let delay = self.next_delay(jitter_source);
        tokio::time::sleep(delay).await;
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> BackoffConfig {
        BackoffConfig {
            initial: Duration::from_millis(100),
            max: Duration::from_millis(1_000),
            factor: 2.0,
            jitter: 0.0,
        }
    }

    #[test]
    fn grows_exponentially_without_jitter() {
        let mut backoff = Backoff::new(base_config());
        assert_eq!(backoff.next_delay(0.5), Duration::from_millis(100));
        assert_eq!(backoff.next_delay(0.5), Duration::from_millis(200));
        assert_eq!(backoff.next_delay(0.5), Duration::from_millis(400));
        assert_eq!(backoff.next_delay(0.5), Duration::from_millis(800));
        assert_eq!(backoff.attempt(), 4);
    }

    #[test]
    fn caps_at_max() {
        let mut backoff = Backoff::new(BackoffConfig {
            max: Duration::from_millis(500),
            ..base_config()
        });
        for _ in 0..10 {
            assert!(backoff.next_delay(0.5) <= Duration::from_millis(500));
        }
    }

    #[test]
    fn jitter_stays_within_bounds() {
        let config = BackoffConfig {
            jitter: 0.2,
            ..base_config()
        };
        let mut backoff = Backoff::new(config);
        for i in 0..20 {
            let source = i as f64 / 19.0;
            let delay = backoff.next_delay(source);
            // The first attempt uses 100ms base; ±20% is 80..=120ms.
            assert!(
                (80..=120).contains(&delay.as_millis()),
                "delay {delay:?} outside jitter bounds"
            );
            backoff.reset();
        }
    }

    #[test]
    fn reset_restarts_the_sequence() {
        let mut backoff = Backoff::new(base_config());
        backoff.next_delay(0.5);
        backoff.next_delay(0.5);
        backoff.reset();
        assert_eq!(backoff.next_delay(0.5), Duration::from_millis(100));
    }

    #[tokio::test]
    async fn wait_next_sleeps() {
        let mut backoff = Backoff::new(base_config());
        let delay = backoff.wait_next(0.5).await;
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn survives_extreme_growth() {
        let config = BackoffConfig {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            factor: 1000.0,
            jitter: 0.0,
        };
        let mut backoff = Backoff::new(config);
        for _ in 0..10 {
            let delay = backoff.next_delay(0.5);
            assert!(delay <= Duration::from_secs(60));
            assert!(delay >= Duration::from_millis(1));
        }
    }
}
