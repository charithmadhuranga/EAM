//! Database bootstrap: connection pool, migration runner, WAL setup.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tracing::info;

use crate::error::{Result, StorageError};

/// Default maximum connections in the pool (bounded per AGENTS.md rule 9).
pub const DEFAULT_MAX_CONNECTIONS: u32 = 4;

/// Open (or create) the EAM database at `path` and run migrations.
///
/// The pool is configured with WAL journaling, foreign keys enabled, and a
/// bounded connection count.
///
/// # Errors
/// Returns [`StorageError`] when the file cannot be opened or a migration fails.
pub async fn open(path: impl AsRef<Path>) -> Result<SqlitePool> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(StorageError::io)?;
        }
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(10));

    let pool = SqlitePoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .connect_with(options)
        .await
        .map_err(StorageError::Database)?;

    run_migrations(&pool).await?;
    info!(db = %path.display(), "database ready");
    Ok(pool)
}

/// Open an isolated in-memory database (used by tests).
///
/// # Errors
/// Returns [`StorageError`] on failure.
pub async fn open_in_memory() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(StorageError::Database)?;

    run_migrations(&pool).await?;
    Ok(pool)
}

/// Apply every pending migration from `migrations/` exactly once.
///
/// idempotent: applied versions are recorded in the `_eam_migrations` table.
///
/// # Errors
/// Returns [`StorageError`] when a migration cannot be applied.
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Track applied versions ourselves so that migrations and the runner see
    // the same source of truth, and re-runs are no-ops.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _eam_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(StorageError::Database)?;

    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _eam_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(StorageError::Database)?;

    for (version, name, sql) in embedded_migrations() {
        if applied.contains(&version) {
            continue;
        }
        info!(version, name, "applying migration");
        let mut tx = pool.begin().await.map_err(StorageError::Database)?;
        sqlx::query(sql.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::migration(version, e))?;
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO _eam_migrations (version, applied_at) VALUES (?, ?)")
            .bind(version)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::migration(version, e))?;
        tx.commit().await.map_err(StorageError::Database)?;
    }

    Ok(())
}

/// The highest migration version currently applied.
///
/// # Errors
/// Returns [`StorageError`] on query failure.
pub async fn schema_version(pool: &SqlitePool) -> Result<i64> {
    let version: Option<i64> =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _eam_migrations")
            .fetch_one(pool)
            .await
            .map_err(StorageError::Database)?;
    Ok(version.unwrap_or(0))
}

/// `(version, name, sql)` for every embedded migration, in order.
///
/// The SQL is embedded at compile time via `include_str!`, so a deployed app
/// carries its schema with it.
#[must_use]
pub fn embedded_migrations() -> Vec<(i64, &'static str, String)> {
    vec![(
        1,
        "initial schema",
        include_str!("../migrations/0001_initial_schema.sql").to_string(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let pool = open_in_memory().await.unwrap();
        assert_eq!(schema_version(&pool).await.unwrap(), 1);
        // Second run applies nothing and leaves the version unchanged.
        run_migrations(&pool).await.unwrap();
        assert_eq!(schema_version(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let pool = open_in_memory().await.unwrap();
        // An entity without a device must be rejected by the FK constraint.
        let result = sqlx::query(
            "INSERT INTO entities
                (id, device_id, native_entity_key, name, entity_type, availability, created_at, updated_at)
             VALUES ('e1', 'no-such-device', '1', 'x', 'switch', 'available', 't', 't')",
        )
        .execute(&pool)
        .await;
        assert!(result.is_err(), "FK should block entities with no device");
    }

    #[tokio::test]
    async fn node_name_uniqueness_is_enforced() {
        let pool = open_in_memory().await.unwrap();
        let insert = "INSERT INTO devices
            (id, node_name, display_name, preferred_port, status, created_at, updated_at)
            VALUES (?, 'node', 'Node', 6053, 'offline', 't', 't')";
        sqlx::query(insert).bind("d1").execute(&pool).await.unwrap();
        let dup = sqlx::query(insert).bind("d2").execute(&pool).await;
        assert!(dup.is_err(), "node_name must be unique");
    }

    #[tokio::test]
    async fn file_database_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eam.db");
        {
            let pool = open(&path).await.unwrap();
            sqlx::query(
                "INSERT INTO devices
                    (id, node_name, display_name, preferred_port, status, created_at, updated_at)
                 VALUES ('d1', 'node', 'Node', 6053, 'online', 't', 't')",
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let pool = open(&path).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "reopening an existing db must keep its rows");
    }
}
