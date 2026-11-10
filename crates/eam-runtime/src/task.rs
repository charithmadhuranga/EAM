//! Task supervisor: spawns, tracks, and stops background services.
//!
//! Every long-running EAM service (connection manager, state recorder, …) is
//! registered here, so the app can stop everything from one place. Tasks must
//! observe the cancellation token and exit promptly.

use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::error::{Error, Result};

/// A tracked background task.
struct TrackedTask {
    /// Registration name (for logs and shutdown errors).
    name: String,
    /// Handle used to await the task during shutdown.
    handle: JoinHandle<()>,
}

/// Owns background tasks and their cancellation.
pub struct TaskSupervisor {
    /// Signalled to every task when shutdown begins.
    token: CancellationToken,
    /// Tasks awaiting shutdown.
    tasks: Vec<TrackedTask>,
    /// How long to wait for tasks during shutdown.
    shutdown_timeout: Duration,
}

impl TaskSupervisor {
    /// Create a supervisor with the default 10-second shutdown deadline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            tasks: Vec::new(),
            shutdown_timeout: Duration::from_secs(10),
        }
    }

    /// Override the shutdown deadline.
    #[must_use]
    pub const fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// The token shared with supervised tasks; cancelled on shutdown.
    #[must_use]
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Register a task under `name`.
    ///
    /// The task receives a clone of the cancellation token and must return
    /// promptly once it fires (flush what it needs to, then exit).
    pub fn spawn<F, Fut>(&mut self, name: impl Into<String>, task: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let token = self.token.clone();
        let spawned_name = name.clone();
        let handle = tokio::spawn(async move {
            task(token).await;
            debug!(task = %spawned_name, "task finished");
        });
        self.tasks.push(TrackedTask { name, handle });
    }

    /// Number of registered tasks.
    #[must_use]
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Whether shutdown has been requested.
    #[must_use]
    pub fn is_shutting_down(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Stop every task, waiting up to the shutdown deadline.
    ///
    /// Tasks are signalled first so they can flush, then awaited. On timeout
    /// the remaining tasks are aborted so the process can still exit.
    ///
    /// # Errors
    /// Returns [`Error::ShutdownTimeout`] if tasks are still running when the
    /// deadline expires, or [`Error::TaskPanicked`] if a task panicked.
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            return Ok(());
        }
        info!(tasks = self.tasks.len(), "runtime shutdown starting");
        self.token.cancel();

        let mut running = Vec::new();
        let deadline = tokio::time::Instant::now() + self.shutdown_timeout;

        while let Some(mut task) = self.tasks.pop() {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or_default();
            match tokio::time::timeout(remaining, &mut task.handle).await {
                Ok(Ok(())) => {}
                Ok(Err(join_err)) if join_err.is_panic() => {
                    return Err(Error::TaskPanicked { name: task.name });
                }
                Ok(Err(_)) => {
                    warn!(task = %task.name, "task was cancelled before finishing");
                }
                Err(_) => {
                    // Out of time: abort and remember this task ran long.
                    warn!(task = %task.name, "shutdown deadline hit, aborting");
                    task.handle.abort();
                }
            }
            let still_running = task.handle.is_finished();
            if !still_running {
                running.push(task.name);
            }
        }

        if !running.is_empty() {
            return Err(Error::ShutdownTimeout {
                seconds: self.shutdown_timeout.as_secs(),
                running: running.len(),
            });
        }
        info!("runtime shutdown complete");
        Ok(())
    }
}

impl Default for TaskSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn spawns_and_tracks_tasks() {
        let mut supervisor = TaskSupervisor::new();
        supervisor.spawn("idler", |token| async move {
            token.cancelled().await;
        });
        assert_eq!(supervisor.task_count(), 1);
        assert!(!supervisor.is_shutting_down());
        supervisor.shutdown().await.unwrap();
        assert_eq!(supervisor.task_count(), 0);
    }

    #[tokio::test]
    async fn tasks_observe_cancellation() {
        let counter = Arc::new(AtomicUsize::new(0));
        let token_observed = Arc::new(tokio::sync::Notify::new());
        let mut supervisor = TaskSupervisor::new();
        let c = counter.clone();
        supervisor.spawn("worker", move |token| async move {
            c.fetch_add(1, Ordering::SeqCst);
            token_observed.notify_one();
            token.cancelled().await;
        });
        supervisor.shutdown().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn slow_tasks_time_out() {
        let mut supervisor = TaskSupervisor::new().with_shutdown_timeout(Duration::from_millis(50));
        supervisor.spawn("hog", |_token| async move {
            // Never observes cancellation: keeps the supervisor busy past the deadline.
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let err = supervisor.shutdown().await.unwrap_err();
        assert!(matches!(err, Error::ShutdownTimeout { .. }));
    }

    #[tokio::test]
    async fn shutdown_with_no_tasks_is_a_noop() {
        let mut supervisor = TaskSupervisor::new();
        supervisor.shutdown().await.unwrap();
        assert_eq!(supervisor.task_count(), 0);
    }

    #[tokio::test]
    async fn cancellation_token_is_shared() {
        let supervisor = TaskSupervisor::new();
        let token = supervisor.cancellation_token();
        assert!(!token.is_cancelled());
        supervisor.token.cancel();
        assert!(token.is_cancelled());
    }
}
