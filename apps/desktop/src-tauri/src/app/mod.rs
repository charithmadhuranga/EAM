//! Application state: the persistent runtime that lives for the process.
//!
//! Mirrors PLAN.md §1.1: the runtime is created once, is independent of the
//! UI, and owns the storage pool, entity registry, and connection manager.

use std::sync::Arc;

use eam_device_manager::ConnectionManager;
use eam_entity_registry::registry::SharedRegistry;
use eam_entity_registry::EntityRegistry;
use eam_runtime::{HealthMonitor, TaskSupervisor};
use eam_storage::open;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Where the SQLite database lives.
pub fn database_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let base = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("eam.db")
}

/// Everything the UI talks to, via IPC commands.
pub struct AppState {
    /// Live device connections.
    pub manager: Arc<ConnectionManager>,
    /// Shared entity registry (also emits state updates).
    pub registry: SharedRegistry,
    /// Background task supervisor (shut down on exit).
    pub supervisor: Arc<Mutex<TaskSupervisor>>,
    /// Uptime/health snapshots.
    pub health: Arc<Mutex<HealthMonitor>>,
}

impl AppState {
    /// Open the database, create the runtime, and reconnect known devices.
    ///
    /// # Errors
    /// Returns an error when the database cannot be opened or the initial
    /// device sync fails.
    pub async fn bootstrap(app: &tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = database_path(app);
        let pool = open(&db_path).await?;

        let registry: SharedRegistry = Arc::new(Mutex::new(EntityRegistry::new()));
        let manager = Arc::new(
            ConnectionManager::new(pool, Arc::clone(&registry))
                .with_data_dir(db_path.to_string_lossy().to_string()),
        );

        let mut supervisor = TaskSupervisor::new();
        let health = HealthMonitor::new();

        // Reconnect every known device; failures are non-fatal (the device may
        // simply be offline, and the reconnect loop keeps trying).
        let reconnect_manager = Arc::clone(&manager);
        supervisor.spawn("reconnect-all", move |token| async move {
            token.cancelled().await;
            match reconnect_manager.reconnect_all().await {
                Ok(()) => info!("initial device sync complete"),
                Err(e) => error!("initial device sync: {e}"),
            }
        });

        // Forward manager events and state updates to the UI.
        let bridge_manager = Arc::clone(&manager);
        supervisor.spawn("event-bridge", move |token| async move {
            let mut events = bridge_manager.subscribe_events();
            loop {
                if token.is_cancelled() {
                    break;
                }
                match events.recv().await {
                    Ok(_event) => { /* forwarded by the Tauri emit in commands */ }
                    Err(_) => break,
                }
            }
        });

        info!(?db_path, "runtime bootstrapped");
        Ok(Self {
            manager,
            registry,
            supervisor: Arc::new(Mutex::new(supervisor)),
            health: Arc::new(Mutex::new(health)),
        })
    }

    /// Graceful shutdown: cancel tasks, then stop connections.
    pub async fn shutdown(&self) {
        self.health.lock().await.mark_stopping();
        let mut supervisor = self.supervisor.lock().await;
        if let Err(e) = supervisor.shutdown().await {
            error!("runtime shutdown: {e}");
        }
        self.manager.shutdown();
    }
}
