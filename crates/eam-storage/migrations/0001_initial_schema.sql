-- Migration 0001: Phase 1 initial schema
--
-- Schema baseline from the implementation spec §8 ("SQLite Database Design").
-- Never edit this file after release: add a new migration instead
-- (docs/architecture/PLAN.md, AGENTS.md rule 8).

PRAGMA foreign_keys = ON;

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    node_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    preferred_host TEXT,
    preferred_port INTEGER NOT NULL DEFAULT 6053,
    api_encryption_secret_ref TEXT,
    status TEXT NOT NULL,
    last_seen_at TEXT,
    firmware_version TEXT,
    mac_address TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_devices_node_name ON devices(node_name);

CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    native_entity_key TEXT NOT NULL,
    object_id TEXT,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    unit TEXT,
    icon TEXT,
    device_class TEXT,
    state_class TEXT,
    capabilities_json TEXT NOT NULL DEFAULT '{}',
    raw_metadata_json TEXT NOT NULL DEFAULT '{}',
    availability TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(device_id, native_entity_key)
);

CREATE INDEX idx_entities_device ON entities(device_id);
CREATE INDEX idx_entities_type ON entities(entity_type);

CREATE TABLE entity_current_states (
    entity_id TEXT PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    value_type TEXT NOT NULL,
    value_json TEXT NOT NULL,
    source_timestamp TEXT,
    received_at TEXT NOT NULL,
    quality TEXT NOT NULL
);

-- Applied-migration bookkeeping. sqlx's own _sqlx_migrations table is used for
-- tooling; this one records the same data for the MigrationRunner API.
