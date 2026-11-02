# ESPHome Automation Manager — Architecture & Design

## Document Purpose
This document defines the system architecture, data models, interface contracts, and design decisions
for the ESPHome Automation Manager (EAM). It is the primary reference for code changes and serves as
the canonical source for:
- High-level component boundaries
- Domain types and validation
- IPC contract versions
- Protocol adapter interfaces
- Schema versions for automation, dashboard, and IPC

## Overview
EAM is a local-first desktop application with hybrid connectivity support:
- **Frontend** (React + TypeScript) → Presentation and user interaction
- **Tauri IPC** → Typed commands and event streams, routes to Rust core
- **Rust Core** → Application lifecycle, runtime services, domain logic
- **Adapters** → Protocol-specific implementations (ESPHome Native API, ESPHome CLI)
- **Storage** → SQLite repositories, migrations, retention
- **Gateway Server** → Headless server for remote network access via FRP
- **FRP Integration** → Secure tunneling for remote fleet management

## Architectural Boundaries (from spec)
- **Frontend** does presentation, local view state, form validation; must not own protocol connections,
  automation timers, or historical recording.
- **Tauri commands** call application use cases; never implement business logic.
- **Rust core** owns connections, entity state, command execution, recording, automation, process management,
  and domain validation.
- **ESPHome adapter** owns protocol types and version compatibility.
- **Storage** owns SQLite schema, migrations, repositories, and transactions.
- **Toolchain adapter** owns ESPHome CLI detection, invocation, and build process integration.
- **Gateway Server** is a separate headless application for remote network access; owns local device discovery
  and FRP client management.
- **FRP integration** provides secure tunneling for remote fleet access without cloud dependency.

## High-Level Architecture Diagram
```text
+------------------------------+
|    Tauri Desktop App          |
| +-------------+ +-------------+ |
| | Fleet UI    | | Dashboards  | |
| +-------------+ +-------------+ |
| | Automations | | History     | |
| | Gateways    | | Settings    | |
| +-------------+ +-------------+ |
|        |                          |
|   Typed Tauri IPC              |
+-------------------------|----------+
                          |
          ┌───────────────┴───────────────┐
          │                               │
    ┌─────▼──────┐              ┌────────▼────────┐
    │   LOCAL    │              │  REMOTE ACCESS  │
    │  NETWORK   │              │   via FRP/VPN   │
    └─────┬──────┘              └────────┬────────┘
          │                               │
    ┌─────▼──────┐              ┌────────▼────────┐
    │  ESPHome   │              │  FRP Server     │
    │  Devices   │              │  (frps)         │
    │ (Port 6053)│              │  Public IP      │
    └────────────┘              └────────┬────────┘
                                         │
                                ┌────────▼────────┐
                                │ Gateway Server  │
                                │ (Headless)      │
                                │ - frpc          │
                                │ - Local mDNS    │
                                │ - ESPHome API   │
                                └────────┬────────┘
                                         │
                                ┌────────▼────────┐
                                │ Remote ESPHome  │
                                │ Devices         │
                                └─────────────────┘

+-------------------------|----------+
|    Rust Core Runtime      |
|                          |
|  Lifecycle / Task Supervisor|
|  Internal Event Bus      |
|  Device Manager         |
|  Gateway Manager (NEW)  |
|  ESPHome Native API Adapter |
|  Entity Command Service |
|  Automation Engine     |
|  Historical Data Recorder |
|  Dashboard Query Service |
|  ESPHome CLI / Process Adapter |
|  Configuration Service |
|  Logging / Diagnostics |
|                          |
|    SQLite Repository     |    External Processes   |
|                         |   ESPHome CLI / Serial |
+--------------------------|-----------------------+
```

## Domain Model
All entities use UUIDs for primary keys. External identifiers are separate fields. Consistent timestamp format: RFC 3339 in UTC.

### Gateway (NEW)
```rust
type Gateway struct {
    id: Uuid,
    name: String,
    connection_type: GatewayConnectionType,
    status: GatewayStatus,
    frp_config: Option<FrpClientConfig>,
    endpoint: String,              // API endpoint (local or via FRP)
    last_seen_at: Option<DateTime<Utc>>,
    device_count: usize,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

type enum GatewayConnectionType {
    Local { host: String, port: u16 },
    Remote { frp_server: String, gateway_name: String },
}

type enum GatewayStatus {
    Online,
    Offline,
    Connecting,
    Error(String),
}
```

### Device
```rust
type Device struct {
    id: Uuid,
    node_name: String,                     // ESPHome node_name (stable)
    display_name: String,                  // User-friendly name
    area_id: Option<Uuid>,
    group_ids: Vec<Uuid>,
    preferred_host: Option<String>,       // mutable connection endpoint
    preferred_port: u16,                   // defaults to 6053
    api_encryption_secret_ref: Option<String>, // credential reference
    connection_status: DeviceConnectionStatus,
    last_seen_at: Option<DateTime<Utc>>,
    firmware_version: Option<String>,
    mac_address: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

### Entity
```rust
type Entity struct {
    id: Uuid,
    device_id: Uuid,                       // foreign key
    native_entity_key: String,             // stable from protocol, never display name
    object_id: Option<String>,
    name: String,
    entity_type: EntityType,
    unit: Option<String>,
    icon: Option<String>,
    state_class: Option<String>,
    device_class: Option<String>,
    capabilities: serde_json::Value,
    raw_metadata: serde_json::Value,
    availability: EntityAvailability,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

### Entity State (Current)
```rust
type EntityState struct {
    entity_id: Uuid,
    value: EntityValue,
    raw_value: Option<serde_json::Value>,
    source_timestamp: Option<DateTime<Utc>>,  // from device if available
    received_at: DateTime<Utc>,
    quality: StateQuality,
}
```

### EntityValue enum
```rust
type enum EntityValue {
    Number(f64),
    Bool(bool),
    Text(String),
    Json(serde_json::Value),
    Unknown,
}
```

### Command
```rust
type Command struct {
    id: Uuid,
    entity_id: Uuid,
    command: EntityCommandKind,
    requested_at: DateTime<Utc>,
    requested_by: CommandOrigin,
    timeout_ms: u64,
}
```

### Automation Definition (canonical JSON schema)
See `schemas/automation/automation.schema.json` for full definition and validation rules.

## Data Model Details

### SQLite Schema (Logical view)
All tables use UUID strings. `*_json` fields store JSONB values as TEXT.

- **entities** table: Indexes on `device_id`, `entity_type`, unique `(device_id, native_entity_key)`
- **entity_current_states** table: Partial indexes for numeric vs non-numeric
- **sensor_readings** table: Composite index on `(entity_id, recorded_at)`
- **entity_events** table: Composite index on `(entity_id, recorded_at)`
- **automation_definitions** table: Index on `enabled`
- **firmware_builds** table: Index on `status`
- **firmware_deployments** table: Index on `device_id`, `started_at`
- **operation_logs** table: Index on `(level, target_type, target_id, created_at)`

### JSON Schema versions
- `schemas/automation/v1.json` — Automation definition schema
- `schemas/dashboard/v1.json` — Dashboard/widget schema
- `schemas/ipc/v1.json` — IPC commands/events envelope

### Retention policies
- Numeric sensor readings: configurable per entity, default 365 days
- Entity events: 90 days
- Operation logs: 30 days
- Automation run history: 90 days

## IPC Contract (Tauri commands)
All commands versioned. Use `AppError` envelope.

### Gateways (NEW)
- `gateway_discover_local` - Discover local gateways via mDNS
- `gateway_connect_remote` - Connect to remote gateway via FRP
- `gateway_list` - List all connected gateways
- `gateway_get_devices` - Get devices from specific gateway
- `gateway_get_info` - Get gateway information
- `gateway_disconnect` - Disconnect from gateway
- `gateway_test_connection` - Test gateway connection

### Workspace
- `workspace_get_info`
- `workspace_backup`
- `workspace_restore`
- `app_get_runtime_status`

### Devices
- `device_discover`
- `device_register`
- `device_update`
- `device_remove`
- `device_list`
- `device_get_details`
- `device_connect`
- `device_disconnect`
- `device_refresh_entities`
- `device_get_logs`

### Entities
- `entity_list`
- `entity_get_state`
- `entity_send_command`
- `entity_update_metadata`
- `entity_get_history`

### Configuration
- `config_project_create`
- `config_project_open`
- `config_project_list`
- `config_read_file`
- `config_write_file`
- `config_validate`
- `config_get_revision_history`
- `config_restore_revision`

### Firmware
- `toolchain_detect`
- `firmware_build`
- `firmware_cancel_build`
- `firmware_flash_serial`
- `firmware_deploy_ota`
- `firmware_get_build`
- `firmware_get_deployment_history`

### Automations
- `automation_create`
- `automation_update`
- `automation_delete`
- `automation_enable`
- `automation_disable`
- `automation_list`
- `automation_validate`
- `automation_run_now`
- `automation_get_run`
- `automation_get_run_trace`

### Dashboards
- `dashboard_create`
- `dashboard_update`
- `dashboard_delete`
- `dashboard_list`
- `dashboard_get`
- `dashboard_widget_add`
- `dashboard_widget_update`
- `dashboard_widget_remove`

### History
- `history_query_series`
- `history_query_events`
- `history_export_csv`
- `history_set_retention_policy`

## Events to Frontend
High-frequency events (e.g., `entity:state`) should be coalesced/batched before reaching React.
- `runtime:status`
- `device:discovered`
- `device:status`
- `device:updated`
- `entity:state`
- `entity:availability`
- `command:status`
- `automation:run`
- `firmware:build`
- `firmware:deployment`
- `history:recorder-status`
- `operation:log`

## Protocol Adapter (ESPHome Native API)
### Adapter interface
- **Port**: `eam-esphome-api` (crate). Owns protocol-specific types; versioned locally.
- **Entry point**: `DeviceManager::connect()`, `DeviceManager::disconnect()`
- **Types**: `NativeEntityKey`, `EntityInfo`, `EntityStatePacket`, `CommandRequest`, `CommandResponse`
- **Strategy**: Rust native client first; narrow adapter process allowed as temporary bridge.

### Events (as internal runtime events)
```rust
enum RuntimeEvent {
    DeviceDiscovered(DeviceDiscovered),
    DeviceConnected(DeviceConnected),
    DeviceDisconnected(DeviceDisconnected),
    EntityRegistryUpdated(EntityRegistryUpdated),
    EntityStateChanged(EntityStateChanged),
    EntityAvailabilityChanged(EntityAvailabilityChanged),
    CommandStatusChanged(CommandStatusChanged),
    AutomationRunUpdated(AutomationRunUpdated),
    FirmwareBuildUpdated(FirmwareBuildUpdated),
    FirmwareDeploymentUpdated(FirmwareDeploymentUpdated),
    RuntimeHealthChanged(RuntimeHealthChanged),
}
```

### Adapter requirements
- Start with mDNS discovery + manual registration.
- Support TLS and authentication using device API password/secret.
- Subscribe to entity list changes and live state updates.
- Implement commands for switch, number, light, fan, climate, lock, etc.
- Detect and recover from connection loss.
- Test with at least two firmware versions.

## Toolchain Adapter (ESPHome CLI)
### Adapter interface
- **Port**: `eam-toolchain` (crate). Owns CLI detection, invocation, environment setup, log streaming, artifact discovery, serial integration, OTA orchestration.
- **Commands**:
  - `ToolchainDetectResult` (available, version, path, issues)
  - `BuildRequest` / `BuildResult` with logs and artifact metadata
  - `FlashSerialRequest` with port selection and progress
  - `DeployOTARequest` with device selection and verification
- **Process management**: Controlled subprocess, timeout, cancellation, health monitoring.

## Event Bus Design
- Use `mpsc` channels or custom event store with correlation IDs.
- Independent subscribers: state registry, recorder, automation engine, logs, UI forwarding.
- Bounded channels with backpressure; slow subscribers cannot block others.
- Each event includes: event ID, event type, occurred_at, correlation_id, target IDs, typed payload.

## Concurrency & Persistence
- SQLite: `sqlx` with connection pool, read-committed or serializable isolation.
- High-frequency sensor ingestion: batch insert in transactions; bounded in-memory queue.
- Long-running tasks (builds, deploys): async runtime tasks with cancellation tokens.
- Runtime uptime: daemon process managed by Tauri.

## Configuration
- Application settings stored in `app_settings` table (JSON).
- Workspace location: user-selected directory; workspace metadata stored in DB.
- Credential storage: OS credential manager (Windows Credential Manager, macOS Keychain, Linux Secret Service).

## Security
- Secrets never stored in DB or logs.
- CSP, strict Tauri capabilities, validate all frontend inputs.
- YAML and JSON parsed with `serde` and validated against schemas; unknown keys preserved.
- Safe defaults: confirmation for hazardous actions, action limits, timeouts.
- Emergency disable-automations control.

## Logging
- Structured logs with correlation IDs in `operation_logs` table.
- Separate console/structured log output.
- Sensitive data redacted from logs.
- Levels: ERROR, WARN, INFO, DEBUG (controlled by settings).

## Testing Strategy
- **Rust unit tests**: entity normalization, condition evaluation, command validation, retention logic.
- **Integration tests**: Native API connections, device info retrieval, live state subscriptions, commands, SQLite migrations.
- **Frontend tests**: device lists, entity controls, dashboard widgets, automation validation, gateway management UI.
- **Fixtures**: simulated ESPHome API server, dummy devices for offline testing, mock FRP server, mock gateway server.
- **Gateway tests**: local discovery, FRP connection, device proxy, state synchronization, reconnection scenarios.
- **E2E tests**: desktop to device via gateway, multi-gateway fleet management, failover scenarios.

## Development Environment
- Rust: `rustc` nightly or stable (as per crate editions).
- Frontend: Node + pnpm (managed via Deno or system install).
- Build: `pnpm install` then `cargo build` or `cargo test`.
- Tools: `tauri` CLI for desktop build, `sqlite3` for db inspection, `gh` for releases.

## Architecture Decision Records (ADRs)
Maintain in `docs/decisions/`:
- `ADR-001-esphome-api-client-strategy.md` (native vs adapter)
- `ADR-002-repo-structure.md` (monorepo layout)
- `ADR-003-database-design.md` (schema decisions)
- `ADR-004-event-bus-design.md` (event delivery strategy)
- `ADR-005-ipc-contract-versioning.md`
- `ADR-006-toolchain-integration.md` (CLI vs embedded)
- `ADR-007-automation-persistence.md` (storage strategy)
- `ADR-008-gateway-server-architecture.md` (remote fleet access via FRP)
- etc.

## Glossary
- **Native API**: ESPHome's native API protocol (WebSocket over TLS)
- **OTA**: Over-the-Air firmware deployment via ESPHome protocol
- **mDNS**: Multicast DNS for device discovery
- **QoS**: Quality of Service levels for entity state delivery
- **Stale state**: Entity state not updated within configurable window
- **Deadband**: Minimum change threshold before recording a sensor reading
- **Correlation ID**: UUID linking related operations/events across domains
- **Gateway Server**: Headless Rust server for remote network access, manages local ESPHome devices
- **FRP**: Fast Reverse Proxy - tunneling solution for remote access without cloud dependency
- **frps**: FRP server component (public-facing)
- **frpc**: FRP client component (runs on Gateway Server)

This design document is the primary reference for implementation. Keep it in sync as the system evolves. Use version control for schema and interface changes.