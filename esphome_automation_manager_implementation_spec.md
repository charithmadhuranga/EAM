# ESPHome Automation Manager

## Product Requirements & Technical Implementation Specification

**Document type:** Product + Engineering Specification\
**Status:** Initial implementation baseline\
**Target platforms:** Windows, Linux, macOS\
**Primary stack:** Tauri 2, React, TypeScript, Rust, SQLite\
**Integration target:** ESPHome devices through the ESPHome Native API
and ESPHome toolchain

------------------------------------------------------------------------

# 1. Executive Summary

ESPHome Automation Manager (working name: **EAM**) is a local-first
desktop application for managing, monitoring, configuring, and
automating fleets of ESPHome-based devices.

The product combines capabilities commonly spread across device
configuration tools, dashboards, automation platforms, and firmware
utilities:

1.  Discover, register, organize, monitor, and control ESPHome devices.
2.  Discover device entities dynamically and expose their capabilities
    through a consistent entity model.
3.  Configure ESPHome projects using both guided visual forms and a full
    YAML editor.
4.  Compile firmware, flash devices over serial, and deploy supported
    OTA updates.
5.  Build cross-device automations using a visual rule builder and
    node-based editor.
6.  Execute automations in a persistent local runtime independent of the
    currently visible UI page.
7.  Create customizable dashboards using live values, charts, gauges,
    status indicators, and actuator controls.
8.  Store sensor history, entity events, automation traces, device
    metadata, and operational logs in SQLite.
9.  Operate locally without requiring a cloud account or internet
    connection for normal device management.
10. **Remote Fleet Management**: Access and manage devices on remote networks
    via Gateway Server and FRP tunneling (added for multi-site support).

The first release is a single-user desktop application. The architecture
must keep clear boundaries between the desktop UI, core runtime, ESPHome
transport, data storage, automation engine, and firmware toolchain so
that an always-on host service and multi-site remote management can be
added later.

**Note**: Multi-site remote management via Gateway Server is now included
in the architecture (see Phase 8).

## 1.1 Product principles

-   **Local-first:** normal operation must not depend on a vendor cloud.
-   **ESPHome-focused:** optimize for ESPHome rather than implementing a
    generic smart-home abstraction first.
-   **Capability-driven:** discover what a device actually exposes;
    never assume every device has the same entities.
-   **Safe configuration:** never silently discard YAML or configuration
    constructs the visual editor does not understand.
-   **Persistent runtime:** device connections, recording, schedules,
    and automations must not depend on an open dashboard view.
-   **Observable operations:** commands, builds, updates, and automation
    runs must have inspectable results.
-   **Progressive complexity:** simple guided workflows for new users;
    direct YAML and advanced controls for experienced users.
-   **Recoverability:** configuration revisions, backups, logs, and
    explicit recovery workflows are first-class features.

## 1.2 Out of scope for the first release

-   Cloud-hosted account platform or vendor-operated cloud.
-   General support for arbitrary non-ESPHome protocols and ecosystems.
-   Mobile applications.
-   Multi-user RBAC and enterprise tenancy.
-   Automatic generation of arbitrary ESPHome YAML from natural
    language.
-   Guaranteed deployment of every automation into device firmware.
-   Safety certification or use as a certified industrial safety
    controller.

------------------------------------------------------------------------

# 2. Users and Core Use Cases

## 2.1 User profiles

### Home automation user

Needs discovery, device status, simple controls, friendly configuration,
dashboards, and straightforward automations.

### ESPHome maker / advanced user

Needs YAML editing, substitutions, packages, secrets, custom components,
build logs, serial flashing, and direct entity access.

### Installer / fleet operator

Needs device grouping, labels, bulk operations, firmware rollout, health
visibility, configuration templates, audit history, and **remote fleet
management across multiple sites via Gateway Server**.

## 2.2 Primary user journeys

### Local Network User Journey

1.  Install the application and create a local workspace.
2.  Discover ESPHome devices on the LAN or add a device manually.
3.  Connect to a device, authenticate, and retrieve its device
    information and entities.
4.  Rename the device, assign it to an area, and organize it into a
    group.
5.  Control an exposed switch, light, fan, number, or other supported
    entity.
6.  Add numeric sensor entities to the historical recorder.
7.  Create a dashboard and arrange widgets.
8.  Create an automation using entities from two or more devices.
9.  Inspect automation execution history and diagnose failures.
10. Open an ESPHome project, edit YAML or guided settings, validate,
    compile, and deploy firmware.

### Remote Fleet Operator Journey (NEW)

1.  Deploy Gateway Server on remote network (Raspberry Pi, mini PC).
2.  Configure Gateway Server with FRP credentials.
3.  Install desktop application and add remote gateway.
4.  View all devices across multiple remote sites in unified interface.
5.  Monitor device health and status across all gateways.
6.  Execute commands on remote devices through gateway tunnel.
7.  Manage firmware updates across multiple sites.
8.  Create automations spanning devices across different sites.

------------------------------------------------------------------------

# 3. Functional Requirements

Priority labels:

-   **P0:** required for the first usable end-to-end release.
-   **P1:** required for a complete first major release.
-   **P2:** advanced or follow-on capability.

## 3.1 Workspace and application lifecycle

  -------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ----------------------
  APP-001                                         P0 Create and open a
                                                     local application
                                                     workspace.

  APP-002                                         P0 Persist application
                                                     settings and workspace
                                                     metadata.

  APP-003                                         P0 Start a persistent
                                                     Rust core when the
                                                     desktop application
                                                     starts.

  APP-004                                         P0 Keep device
                                                     connections and
                                                     automation execution
                                                     independent of UI
                                                     route changes.

  APP-005                                         P0 Gracefully stop
                                                     background tasks and
                                                     flush database writes
                                                     on application exit.

  APP-006                                         P1 Support workspace
                                                     backup and restore.

  APP-007                                         P1 Provide diagnostics
                                                     export with secrets
                                                     redacted.

  APP-008                                         P2 Support multiple local
                                                     workspaces/profiles.
  
  APP-009 (NEW)                                   P1 Support remote gateway
                                                     connections via FRP.

  -------------------------------------------------------------------------

## 3.2 Gateway Server and Remote Fleet Management (NEW SECTION)

This section defines requirements for remote fleet management via Gateway
Server and FRP integration.

### 3.2.1 Gateway Server Requirements

  -------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ----------------------
  GTW-001                                         P1 Deploy headless
                                                     Gateway Server on
                                                     remote network.

  GTW-002                                         P1 Discover and manage
                                                     local ESPHome devices.

  GTW-003                                         P1 Connect to FRP server
                                                     (frps) via frpc client.

  GTW-004                                         P1 Provide REST API for
                                                     desktop app
                                                     communication.

  GTW-005                                         P1 Provide WebSocket API
                                                     for real-time state
                                                     updates.

  GTW-006                                         P1 Support TLS encryption
                                                     for all connections.

  GTW-007                                         P1 Authenticate desktop
                                                     app connections (API
                                                     keys or JWT).

  GTW-008                                         P1 Maintain persistent
                                                     connection to FRP
                                                     server.

  GTW-009                                         P1 Auto-reconnect on
                                                     connection loss.

  GTW-010                                         P1 Buffer events during
                                                     disconnect.

  GTW-011                                         P2 Support multiple
                                                     desktop app
                                                     connections.

  GTW-012                                         P2 Provide Prometheus
                                                     metrics endpoint.

  GTW-013                                         P2 Support configuration
                                                     via file and
                                                     environment variables.
  -------------------------------------------------------------------------

### 3.2.2 Desktop App Gateway Integration

  -------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ----------------------
  GDA-001                                         P1 Discover local
                                                     gateways via mDNS.

  GDA-002                                         P1 Connect to remote
                                                     gateways via FRP.

  GDA-003                                         P1 Display hierarchical
                                                     view: Gateways →
                                                     Devices → Entities.

  GDA-004                                         P1 Support manual
                                                     gateway configuration.

  GDA-005                                         P1 Test gateway
                                                     connection before
                                                     saving.

  GDA-006                                         P1 Show gateway
                                                     connection status.

  GDA-007                                         P1 Execute commands on
                                                     devices through
                                                     gateway.

  GDA-008                                         P1 Subscribe to state
                                                     updates via gateway
                                                     WebSocket.

  GDA-009                                         P1 Support multiple
                                                     gateways simultaneously.

  GDA-010                                         P2 Auto-reconnect to
                                                     gateways on startup.

  GDA-011                                         P2 Cache device state
                                                     during gateway
                                                     disconnect.
  -------------------------------------------------------------------------

### 3.2.3 FRP Integration Requirements

  -------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ----------------------
  FRP-001                                         P1 Support self-hosted
                                                     FRP server.

  FRP-002                                         P2 Support FRP cloud
                                                     services.

  FRP-003                                         P1 Auto-generate frpc
                                                     configuration.

  FRP-004                                         P1 Monitor frpc process
                                                     health.

  FRP-005                                         P1 Restart frpc on
                                                     failure.

  FRP-006                                         P1 Support TLS for FRP
                                                     connections.

  FRP-007                                         P2 Provide FRP setup
                                                     guide and scripts.

  FRP-008                                         P2 Support alternative
                                                     VPN tunneling
                                                     (WireGuard).
  -------------------------------------------------------------------------

### 3.2.4 Gateway Configuration Schema

Gateway Server configuration must support:

- Gateway identity (name, ID)
- Local network settings (mDNS, manual devices)
- FRP client configuration (server address, auth token, TLS)
- API server settings (listen address, TLS, authentication)
- Security settings (audit logging, rate limiting)

See `schemas/gateway/v1.schema.json` for complete schema definition.
                                                     background tasks and
                                                     flush database writes
                                                     on application exit.

  APP-006                                         P1 Support workspace
                                                     backup and restore.

  APP-007                                         P1 Provide diagnostics
                                                     export with secrets
                                                     redacted.

  APP-008                                         P2 Support multiple local
                                                     workspaces/profiles.
  -------------------------------------------------------------------------

## 3.2 Device discovery and fleet management

  ---------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ------------------------
  DEV-001                                         P0 Discover ESPHome devices
                                                     using mDNS where
                                                     available.

  DEV-002                                         P0 Allow manual device
                                                     registration by hostname
                                                     or IP address.

  DEV-003                                         P0 Connect using the
                                                     ESPHome Native API.

  DEV-004                                         P0 Support API encryption
                                                     and configured device
                                                     credentials.

  DEV-005                                         P0 Retrieve device
                                                     identity, firmware/build
                                                     metadata where exposed,
                                                     and entity definitions.

  DEV-006                                         P0 Persist device records
                                                     and user-defined names.

  DEV-007                                         P0 Show online, connecting,
                                                     unavailable,
                                                     authentication-failed,
                                                     and disconnected states.

  DEV-008                                         P0 Reconnect with bounded
                                                     exponential backoff and
                                                     jitter.

  DEV-009                                         P0 Subscribe to entity
                                                     state updates and
                                                     publish them to the
                                                     internal event bus.

  DEV-010                                         P0 Support entity commands
                                                     for implemented entity
                                                     types.

  DEV-011                                         P1 Organize devices by
                                                     site, area, room, tag,
                                                     and user-defined group.

  DEV-012                                         P1 Support bulk actions for
                                                     selected devices.

  DEV-013                                         P1 Provide device
                                                     diagnostics, connection
                                                     logs, and last-seen
                                                     information.

  DEV-014                                         P1 Detect changed entity
                                                     inventories after
                                                     reconnect and firmware
                                                     update.

  DEV-015                                         P2 Support configurable
                                                     polling/health checks
                                                     for metadata not
                                                     provided as push state.
  ---------------------------------------------------------------------------

Device identity must not rely solely on IP address. Prefer stable
ESPHome node identity and API-provided identifiers where available.
Store network endpoints as changeable connection metadata.

## 3.3 Entity registry and control

The application must create a normalized entity registry from the Native
API's discovered entity metadata.

Initial entity families:

-   Numeric sensors
-   Binary sensors
-   Switches
-   Lights
-   Numbers
-   Selects
-   Buttons
-   Text sensors
-   Text entities, if exposed by the supported API version
-   Fans
-   Covers
-   Climate entities
-   Locks
-   Media/player or other entity types only after protocol support is
    verified

  ------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ---------------------
  ENT-001                                         P0 Store entity
                                                     identity, device
                                                     association, entity
                                                     family, name, icon,
                                                     unit, and
                                                     availability.

  ENT-002                                         P0 Store raw API
                                                     metadata needed for
                                                     forward
                                                     compatibility.

  ENT-003                                         P0 Normalize state
                                                     values while
                                                     preserving raw
                                                     protocol values.

  ENT-004                                         P0 Render controls
                                                     according to entity
                                                     family and supported
                                                     features.

  ENT-005                                         P0 Validate commands
                                                     against entity
                                                     capabilities and
                                                     ranges.

  ENT-006                                         P0 Show
                                                     unavailable/stale
                                                     state distinctly from
                                                     a valid zero or false
                                                     state.

  ENT-007                                         P1 Support user-defined
                                                     entity names, labels,
                                                     and dashboard
                                                     visibility.

  ENT-008                                         P1 Detect removed,
                                                     added, or changed
                                                     entities after
                                                     rediscovery.

  ENT-009                                         P1 Preserve entity
                                                     history when the same
                                                     stable entity
                                                     identity remains
                                                     after firmware
                                                     changes.

  ENT-010                                         P2 Provide entity
                                                     aliases and migration
                                                     tools when firmware
                                                     changes entity
                                                     identity.
  ------------------------------------------------------------------------

Every command must pass through a single command service. UI components
must not open their own Native API connections.

## 3.4 ESPHome configuration projects

Configuration is project-based, not merely an editor for a connected
device.

  ------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ---------------------
  CFG-001                                         P0 Create/open an
                                                     ESPHome configuration
                                                     project.

  CFG-002                                         P0 Store project files
                                                     and project-to-device
                                                     associations.

  CFG-003                                         P0 Provide a full YAML
                                                     editor with syntax
                                                     highlighting and
                                                     diagnostics.

  CFG-004                                         P0 Validate YAML before
                                                     build/deploy.

  CFG-005                                         P0 Preserve unknown YAML
                                                     constructs and
                                                     comments where
                                                     practical.

  CFG-006                                         P1 Provide guided visual
                                                     forms for a defined
                                                     subset of common
                                                     components.

  CFG-007                                         P1 Show a diff before
                                                     applying visual edits
                                                     to YAML.

  CFG-008                                         P1 Support
                                                     substitutions,
                                                     packages, includes,
                                                     secrets, and external
                                                     files.

  CFG-009                                         P1 Provide configuration
                                                     revisions and
                                                     rollback.

  CFG-010                                         P2 Provide component
                                                     catalogue search and
                                                     contextual
                                                     documentation links.

  CFG-011                                         P2 Provide
                                                     template-based
                                                     project creation for
                                                     common boards and
                                                     devices.
  ------------------------------------------------------------------------

### Configuration editor safety rules

1.  YAML is the source of truth.
2.  The visual editor must operate on a structured representation with
    source mapping or carefully scoped edits.
3.  Unknown keys, custom components, lambdas, packages, includes, and
    unsupported options must be retained.
4.  If a visual operation cannot safely preserve an unsupported section,
    disable that operation or require explicit user confirmation.
5.  Never silently rewrite the entire YAML file from an incomplete
    internal model.
6.  Show the generated diff before saving changes from visual mode.
7.  Keep secrets out of logs, crash reports, and exported diagnostics.

## 3.5 Build, flash, and OTA

  ------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ---------------------
  FW-001                                          P0 Detect or configure
                                                     an ESPHome
                                                     CLI/toolchain
                                                     installation.

  FW-002                                          P0 Validate a project
                                                     using the ESPHome
                                                     toolchain.

  FW-003                                          P0 Compile firmware and
                                                     capture structured
                                                     build output plus raw
                                                     logs.

  FW-004                                          P0 Store build metadata
                                                     and artifact paths.

  FW-005                                          P0 Support OTA
                                                     deployment using
                                                     supported ESPHome
                                                     tooling/protocols.

  FW-006                                          P1 Support serial/USB
                                                     flashing and port
                                                     selection.

  FW-007                                          P1 Show progress, logs,
                                                     failure reasons, and
                                                     reconnect
                                                     verification.

  FW-008                                          P1 Maintain firmware
                                                     deployment history
                                                     per device.

  FW-009                                          P1 Support staged
                                                     rollout to device
                                                     groups.

  FW-010                                          P1 Verify device
                                                     reconnect and
                                                     expected firmware
                                                     metadata after
                                                     deployment where
                                                     available.

  FW-011                                          P2 Support rollout
                                                     policies, maintenance
                                                     windows, and rollback
                                                     guidance.
  ------------------------------------------------------------------------

Do not implement a parallel firmware compiler or OTA protocol unless a
documented limitation requires it. Prefer invoking the supported ESPHome
toolchain through a controlled process adapter. Pin and test supported
ESPHome CLI versions.

OTA is not guaranteed to work for every device or firmware state. The UI
must explain prerequisites and offer serial recovery where available.

## 3.6 Historical data and event recording

  ------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ---------------------
  HIS-001                                         P0 Record selected
                                                     numeric sensor states
                                                     with timestamps.

  HIS-002                                         P0 Record entity state
                                                     changes and important
                                                     events.

  HIS-003                                         P0 Provide latest-value
                                                     queries.

  HIS-004                                         P0 Provide time-range
                                                     history queries.

  HIS-005                                         P0 Provide chart-ready
                                                     aggregation for
                                                     larger time ranges.

  HIS-006                                         P1 Configure per-entity
                                                     sampling and
                                                     retention policies.

  HIS-007                                         P1 Batch writes and use
                                                     bounded queues to
                                                     protect the runtime.

  HIS-008                                         P1 Export history to
                                                     CSV.

  HIS-009                                         P2 Add downsampling
                                                     tiers and advanced
                                                     analytics.
  ------------------------------------------------------------------------

Data must include source timestamps when trustworthy and a local receive
timestamp. Use UTC internally and convert to the user's local timezone
in the UI.

## 3.7 Dashboards and widgets

  ------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ---------------------
  DSH-001                                         P0 Create, rename,
                                                     duplicate, and delete
                                                     dashboards.

  DSH-002                                         P0 Add, configure, move,
                                                     resize, and remove
                                                     widgets.

  DSH-003                                         P0 Persist dashboard
                                                     layouts.

  DSH-004                                         P0 Display live sensor
                                                     values and actuator
                                                     controls.

  DSH-005                                         P0 Display historical
                                                     line charts for
                                                     numeric entities.

  DSH-006                                         P1 Provide gauges,
                                                     meters, binary
                                                     status, device
                                                     status, and event
                                                     widgets.

  DSH-007                                         P1 Support multiple
                                                     pages and dashboard
                                                     folders/categories.

  DSH-008                                         P1 Support dashboard
                                                     templates and
                                                     import/export.

  DSH-009                                         P1 Support theme, color,
                                                     icon, unit,
                                                     precision, and
                                                     threshold
                                                     customization.

  DSH-010                                         P2 Provide advanced
                                                     responsive layouts
                                                     and widget groups.
  ------------------------------------------------------------------------

### Initial widget catalogue

-   Numeric value card
-   Sensor value with trend indicator
-   Line chart
-   Multi-series line chart
-   Gauge
-   Progress bar
-   Binary status card
-   Switch control
-   Light control
-   Number/PWM slider
-   Select control
-   Button/action card
-   Fan control
-   Cover control
-   Device health card
-   Event timeline
-   Automation status/list
-   Markdown/text panel

Widgets reference stable entity IDs and subscribe to the shared state
store. They must not create independent device connections.

## 3.8 Automation engine

  -------------------------------------------------------------------------
  ID                                        Priority Requirement
  --------------------- ---------------------------- ----------------------
  AUT-001                                         P0 Create, edit, enable,
                                                     disable, and delete
                                                     automations.

  AUT-002                                         P0 Support entity state
                                                     and numeric threshold
                                                     triggers.

  AUT-003                                         P0 Support AND/OR
                                                     condition groups and
                                                     numeric/string/state
                                                     comparisons.

  AUT-004                                         P0 Support switch and
                                                     number actions for
                                                     initial end-to-end
                                                     scope.

  AUT-005                                         P0 Support actions
                                                     targeting entities on
                                                     different devices.

  AUT-006                                         P0 Support delays and
                                                     sequential actions.

  AUT-007                                         P0 Execute from a
                                                     persistent runtime,
                                                     not from the UI.

  AUT-008                                         P0 Record each automation
                                                     run and its result.

  AUT-009                                         P1 Provide schedules,
                                                     time windows,
                                                     cooldowns, and
                                                     debounce.

  AUT-010                                         P1 Provide visual rule
                                                     builder.

  AUT-011                                         P1 Provide node-based
                                                     graph editor.

  AUT-012                                         P1 Support parallel
                                                     branches, waits,
                                                     timeouts, and
                                                     repeat/loop with
                                                     limits.

  AUT-013                                         P1 Support retry policy
                                                     and failure branches.

  AUT-014                                         P1 Provide dry-run/test
                                                     mode and execution
                                                     trace.

  AUT-015                                         P2 Explore deployment of
                                                     a restricted
                                                     automation subset to
                                                     device-local ESPHome
                                                     logic.
  -------------------------------------------------------------------------

### Automation execution semantics

-   Trigger events are delivered to the automation engine through an
    internal event bus.
-   Each trigger event has a unique event ID and timestamp.
-   Automation runs have unique run IDs.
-   The engine must define whether overlapping runs are allowed, queued,
    ignored, or replace an active run. This is a per-automation policy.
-   Conditions are evaluated against a consistent state snapshot where
    possible.
-   Every action has a timeout and produces a structured result.
-   Retry behavior must be bounded and configurable.
-   A device disconnect must produce an explicit unavailable result, not
    an indefinite wait.
-   Delays and schedules must be persisted or safely reconstructed after
    runtime restart.
-   The runtime must not claim an action succeeded merely because it was
    queued; command acknowledgement and observed state are distinct
    statuses.
-   Potentially hazardous actions require configurable limits,
    confirmation policies, and safe defaults.

### Initial automation node types

**Triggers** - Entity state changed - Numeric value above/below/within
range - Device became online/offline - Schedule/time trigger - Manual
trigger

**Conditions** - Entity state equals/not equals - Numeric comparison -
AND / OR / NOT - Device availability - Time/day condition - Automation
variable comparison

**Actions** - Turn switch on/off/toggle - Set numeric value - Set
supported light properties - Select option - Press button - Delay -
Write log/event - Run another automation, with cycle protection

**Flow control** - Sequence - Parallel - If/else - Wait for event with
timeout - Repeat with a strict maximum iteration count

------------------------------------------------------------------------

# 4. Non-Functional Requirements

  -----------------------------------------------------------------------
  Area                                Requirement
  ----------------------------------- -----------------------------------
  Reliability                         A transient device disconnect must
                                      not crash the application or
                                      runtime.

  Performance                         UI updates must be decoupled from
                                      high-frequency sensor ingestion.

  Responsiveness                      Long builds, scans, database
                                      queries, and deployments must run
                                      asynchronously.

  Storage                             SQLite writes must be batched and
                                      bounded; database migrations must
                                      be versioned.

  Security                            Store API passwords and encryption
                                      keys using OS credential storage
                                      where available.

  Privacy                             Device data remains local unless
                                      the user explicitly exports or
                                      connects a remote service.

  Portability                         Support Windows, Linux, and macOS
                                      with platform-specific adapters
                                      where required.

  Accessibility                       Keyboard navigation, visible focus,
                                      labels, and adequate contrast.

  Observability                       Structured logs, correlation IDs,
                                      diagnostics, and actionable errors.

  Maintainability                     Domain logic must not be embedded
                                      in React components.

  Testability                         Protocol adapters, clocks,
                                      schedulers, storage, and command
                                      services must be injectable.
  -----------------------------------------------------------------------

## 4.1 Performance targets for initial testing

These are engineering targets, not guarantees for every hardware/network
environment:

-   Handle at least 50 connected devices in the initial performance test
    profile.
-   Handle at least 2,000 registered entities.
-   Sustain 100 incoming state updates/second in a synthetic test
    without unbounded memory growth.
-   Keep ordinary UI interactions responsive while ingestion and builds
    run.
-   Use pagination or virtualization for large device/entity/event
    lists.
-   Keep chart queries bounded by time range and point count.
-   Avoid one SQLite write transaction per individual sensor event.

The architecture should permit higher limits later through profiling and
optimization.

------------------------------------------------------------------------

# 5. System Architecture

## 5.1 High-level architecture

``` text
+--------------------------------------------------------------+
|                    Tauri Desktop Application                 |
|                                                              |
|  React + TypeScript                                          |
|  +-------------+ +-------------+ +-------------------------+ |
|  | Fleet UI    | | Dashboards  | | YAML / Visual Config    | |
|  +-------------+ +-------------+ +-------------------------+ |
|  | Automations | | History     | | Firmware Management     | |
|  | Gateways    | | Settings    | | Remote Fleet Mgmt       | |
|  +-------------+ +-------------+ +-------------------------+ |
|                         |                                    |
|                 Typed Tauri IPC                              |
+-------------------------|------------------------------------+
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

+-------------------------v------------------------------------+
|                    Rust Core Runtime                         |
|                                                              |
|  Application Lifecycle / Task Supervisor                     |
|  Internal Event Bus / State Registry                         |
|  Device Manager / Connection Supervisor                       |
|  Gateway Manager (NEW)                                       |
|  ESPHome Native API Adapter                                   |
|  Entity Command Service                                       |
|  Automation Engine / Scheduler                               |
|  Historical Data Recorder                                    |
|  Dashboard Query Service                                     |
|  ESPHome CLI / Build / Flash Process Adapter                  |
|  Configuration Project Service                               |
|  Logging / Diagnostics                                       |
|                         |                                    |
|              +----------+-----------+                        |
|              |                      |                        |
|        SQLite Repository      External Processes             |
|                               ESPHome CLI / Serial tools     |
+-------------------------------|------------------------------+
                                |
                       ESPHome Devices
```

## 5.2 Architectural boundaries

### Frontend

Responsible for presentation, user interaction, local view state, form
validation, and rendering. It must not own protocol connections,
automation timers, or historical recording.

### Tauri command/event layer

Provides typed request/response commands and event streams. Commands are
for user-initiated operations and queries. Events deliver asynchronous
runtime updates.

### Rust core

Owns application lifecycle, device connections, entity state, command
execution, recording, automation execution, process management, and
domain validation.

### ESPHome adapter

Translates between the internal domain model and the ESPHome Native API.
It owns protocol-specific types and version compatibility.

### Storage

Owns SQLite schema, migrations, repositories, transactions, and
retention jobs. UI code must never query SQLite directly.

### Toolchain adapter

Owns ESPHome CLI detection, invocation, environment setup, log
streaming, artifact discovery, serial port integration, and OTA
orchestration.

### Gateway Server (NEW)

Headless Rust server for remote network access. Owns local device
discovery, FRP client management, REST/WebSocket API, and state
synchronization with desktop application.

### FRP Integration (NEW)

Provides secure tunneling for remote fleet access without cloud
dependency. Owns frpc process management, connection monitoring, and
auto-reconnection logic.

## 5.3 Native API implementation strategy

The preferred long-term strategy is a native Rust ESPHome API client,
but it is a gated technical decision.

Before implementing the full product, complete a protocol spike that
proves:

1.  TCP connection and handshake.
2.  Encryption/authentication behavior supported by the target ESPHome
    version.
3.  Device information retrieval.
4.  Entity list retrieval for the initial supported families.
5.  Live state subscriptions.
6.  Commands for switch and number entities.
7.  Connection loss detection and reconnect.
8.  Compatibility tests against at least two ESPHome firmware versions.

Use the official ESPHome API protocol definitions and documentation as
the source of truth. Do not reverse-engineer from assumptions.

If Rust coverage is incomplete, use a narrow adapter process as a
temporary bridge, such as a Python `aioesphomeapi` worker with a
versioned local IPC contract. Keep that bridge isolated so it can later
be replaced without changing the UI, entity model, automation engine, or
database. Do not make the Python bridge the owner of application domain
state.

------------------------------------------------------------------------

# 6. Repository Structure

Recommended monorepo:

``` text
esphome-automation-manager/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── app/
│       │   ├── components/
│       │   ├── features/
│       │   │   ├── overview/
│       │   │   ├── devices/
│       │   │   ├── entities/
│       │   │   ├── configuration/
│       │   │   ├── firmware/
│       │   │   ├── automations/
│       │   │   ├── dashboards/
│       │   │   ├── history/
│       │   │   └── settings/
│       │   ├── hooks/
│       │   ├── lib/
│       │   ├── stores/
│       │   ├── styles/
│       │   └── types/
│       ├── src-tauri/
│       │   ├── src/
│       │   │   ├── commands/
│       │   │   ├── gateway/           # NEW
│       │   │   ├── events/
│       │   │   ├── app/
│       │   │   └── main.rs
│       │   ├── capabilities/
│       │   └── tauri.conf.json
│       ├── package.json
│       └── vite.config.ts
│   │
│   └── gateway-server/                # NEW
│       ├── src/
│       │   ├── main.rs                # Server entry point
│       │   ├── server/                # Server core logic
│       │   ├── device/                # Device discovery & management
│       │   ├── frp/                   # FRP client integration
│       │   ├── api/                   # REST/WebSocket API
│       │   ├── config/                # Configuration management
│       │   └── state/                 # State management
│       ├── config.example.toml        # Example configuration
│       ├── Cargo.toml
│       ├── Dockerfile
│       └── docker-compose.yml
│
├── crates/
│   ├── eam-domain/
│   ├── eam-application/
│   ├── eam-runtime/
│   ├── eam-esphome-api/
│   ├── eam-device-manager/
│   ├── eam-entity-registry/
│   ├── eam-automation/
│   ├── eam-storage/
│   ├── eam-history/
│   ├── eam-dashboard/
│   ├── eam-config/
│   ├── eam-toolchain/
│   ├── eam-discovery/
│   ├── eam-security/
│   └── eam-observability/
│
├── schemas/
│   ├── automation/
│   ├── dashboard/
│   ├── gateway/                       # NEW
│   └── ipc/
├── fixtures/
│   ├── esphome-api/
│   ├── configurations/
│   └── automation/
├── docs/
│   ├── architecture/
│   ├── decisions/
│   ├── development/
│   └── user/
├── scripts/
├── Cargo.toml
├── pnpm-workspace.yaml
├── package.json
└── README.md
```

Crates should be split by domain responsibility, not merely to create
many small packages. If the project is initially easier to maintain with
fewer crates, begin with a workspace containing domain, runtime,
integrations, and storage modules, then split when boundaries are
stable.

## 6.1 Rust dependency direction

``` text
eam-domain
   ^
   |
eam-application
   ^
   |
eam-runtime
   |---------- eam-device-manager ---- eam-esphome-api
   |---------- eam-automation
   |---------- eam-history
   |---------- eam-toolchain
   |---------- eam-config
   |---------- eam-storage
   |---------- eam-observability
```

-   `eam-domain` must not depend on Tauri, SQLite, or ESPHome protocol
    libraries.
-   `eam-application` defines use cases and ports/interfaces.
-   Integration crates implement ports.
-   `eam-runtime` composes services and owns long-lived tasks.
-   Tauri commands call application use cases; they do not implement
    business logic.

------------------------------------------------------------------------

# 7. Core Domain Model

Use UUIDs for internal primary keys. Keep external ESPHome identifiers
as separate fields. Use UTC timestamps in RFC 3339 or integer epoch
milliseconds consistently.

## 7.1 Device

``` rust
struct Device {
    id: Uuid,
    node_name: String,
    display_name: String,
    area_id: Option<Uuid>,
    group_ids: Vec<Uuid>,
    preferred_host: Option<String>,
    preferred_port: u16,
    api_encryption_secret_ref: Option<String>,
    connection_status: DeviceConnectionStatus,
    last_seen_at: Option<DateTime<Utc>>,
    firmware_version: Option<String>,
    mac_address: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Do not store plaintext API passwords or encryption secrets in this
structure or ordinary database columns. Store a credential reference
into the OS credential store or an encrypted secret vault.

## 7.2 Entity

``` rust
struct Entity {
    id: Uuid,
    device_id: Uuid,
    native_entity_key: String,
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

`native_entity_key` must be derived from the stable identity supplied by
the protocol where possible. Do not use display names as primary
identity.

## 7.3 Entity state

``` rust
struct EntityState {
    entity_id: Uuid,
    value: EntityValue,
    raw_value: Option<serde_json::Value>,
    source_timestamp: Option<DateTime<Utc>>,
    received_at: DateTime<Utc>,
    quality: StateQuality,
}
```

Represent values with a tagged enum rather than converting every state
to a string:

``` rust
enum EntityValue {
    Number(f64),
    Bool(bool),
    Text(String),
    Json(serde_json::Value),
    Unknown,
}
```

## 7.4 Command

``` rust
struct EntityCommand {
    id: Uuid,
    entity_id: Uuid,
    command: EntityCommandKind,
    requested_at: DateTime<Utc>,
    requested_by: CommandOrigin,
    timeout_ms: u64,
}
```

Command results must distinguish:

-   Rejected by local validation
-   Queued
-   Sent
-   Acknowledged by transport, if available
-   Observed state change
-   Timed out
-   Device unavailable
-   Failed

Do not equate transport acknowledgement with proof that a physical
actuator achieved its intended effect.

------------------------------------------------------------------------

# 8. SQLite Database Design

Use SQLite with foreign keys enabled, WAL mode where appropriate,
prepared statements, migrations, and a single controlled write path or
carefully managed connection pool.

The following is a logical schema baseline. Add indexes and constraints
through versioned migrations.

## 8.1 Main tables

``` sql
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE areas (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES areas(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE device_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    node_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,
    preferred_host TEXT,
    preferred_port INTEGER NOT NULL DEFAULT 6053,
    firmware_version TEXT,
    mac_address TEXT,
    status TEXT NOT NULL,
    last_seen_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_devices_node_name ON devices(node_name);

CREATE TABLE device_group_members (
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    group_id TEXT NOT NULL REFERENCES device_groups(id) ON DELETE CASCADE,
    PRIMARY KEY (device_id, group_id)
);

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

CREATE TABLE sensor_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    value REAL NOT NULL,
    source_timestamp TEXT,
    recorded_at TEXT NOT NULL
);

CREATE INDEX idx_sensor_readings_entity_time
    ON sensor_readings(entity_id, recorded_at);

CREATE TABLE entity_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    old_value_json TEXT,
    new_value_json TEXT,
    source_timestamp TEXT,
    recorded_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_entity_events_entity_time
    ON entity_events(entity_id, recorded_at);

CREATE TABLE automation_definitions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 0,
    execution_policy_json TEXT NOT NULL DEFAULT '{}',
    definition_json TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE automation_runs (
    id TEXT PRIMARY KEY,
    automation_id TEXT NOT NULL REFERENCES automation_definitions(id) ON DELETE CASCADE,
    trigger_event_id TEXT,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    result_json TEXT NOT NULL DEFAULT '{}',
    error TEXT
);

CREATE INDEX idx_automation_runs_automation_time
    ON automation_runs(automation_id, started_at);

CREATE TABLE automation_run_steps (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES automation_runs(id) ON DELETE CASCADE,
    step_key TEXT NOT NULL,
    step_type TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    input_json TEXT,
    output_json TEXT,
    error TEXT
);

CREATE TABLE dashboards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    layout_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE dashboard_widgets (
    id TEXT PRIMARY KEY,
    dashboard_id TEXT NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    widget_type TEXT NOT NULL,
    title TEXT,
    config_json TEXT NOT NULL,
    layout_json TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE config_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    primary_yaml_path TEXT NOT NULL,
    associated_device_id TEXT REFERENCES devices(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE config_revisions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES config_projects(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, revision)
);

CREATE TABLE firmware_builds (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES config_projects(id) ON DELETE CASCADE,
    config_revision_id TEXT REFERENCES config_revisions(id) ON DELETE SET NULL,
    status TEXT NOT NULL,
    toolchain_version TEXT,
    artifact_path TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    logs_path TEXT,
    error TEXT
);

CREATE TABLE firmware_deployments (
    id TEXT PRIMARY KEY,
    build_id TEXT NOT NULL REFERENCES firmware_builds(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    result_json TEXT NOT NULL DEFAULT '{}',
    error TEXT
);

CREATE TABLE operation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    correlation_id TEXT,
    message TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_operation_logs_time ON operation_logs(created_at);
```

## 8.2 Data retention and ingestion

-   Keep current entity state separately from historical readings.
-   Record numeric sensor history according to per-entity policy.
-   Record binary and text transitions in `entity_events`.
-   Apply minimum-change/deadband recording where configured.
-   Support a maximum sample rate per entity.
-   Batch insert readings in transactions.
-   Bound the in-memory ingestion queue; expose dropped/overloaded
    counters.
-   Run retention and aggregation as background jobs.
-   Never block the Native API receive loop on a slow chart query or
    database maintenance job.
-   Do not store every repeated identical state unless explicitly
    configured.
-   Provide database backup using SQLite's supported backup mechanism or
    a safe checkpoint/close/copy procedure.

------------------------------------------------------------------------

# 9. Internal Event Model

Use a typed internal event bus. Avoid coupling the device adapter
directly to React or automation UI concepts.

Example events:

``` rust
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

Each event should contain:

-   Event ID
-   Event type/version
-   Occurred-at timestamp
-   Correlation ID where applicable
-   Relevant domain identifiers
-   Typed payload

The event bus should support independent subscribers for state registry,
recorder, automation engine, logs, and UI event forwarding.

Do not allow a slow subscriber to block all other subscribers. Use
bounded channels, backpressure policies, and metrics.

------------------------------------------------------------------------

# 10. Tauri IPC Contract

All IPC contracts must be versioned and typed. Keep command names stable
and avoid passing arbitrary internal Rust structures directly to the
frontend.

## 10.1 Commands

### Workspace

-   `workspace_get_info`
-   `workspace_backup`
-   `workspace_restore`
-   `app_get_runtime_status`

### Devices

-   `device_discover`
-   `device_register`
-   `device_update`
-   `device_remove`
-   `device_list`
-   `device_get_details`
-   `device_connect`
-   `device_disconnect`
-   `device_refresh_entities`
-   `device_get_logs`

### Entities

-   `entity_list`
-   `entity_get_state`
-   `entity_send_command`
-   `entity_update_metadata`
-   `entity_get_history`

### Configuration

-   `config_project_create`
-   `config_project_open`
-   `config_project_list`
-   `config_read_file`
-   `config_write_file`
-   `config_validate`
-   `config_get_revision_history`
-   `config_restore_revision`

### Firmware

-   `toolchain_detect`
-   `firmware_build`
-   `firmware_cancel_build`
-   `firmware_flash_serial`
-   `firmware_deploy_ota`
-   `firmware_get_build`
-   `firmware_get_deployment_history`

### Automations

-   `automation_create`
-   `automation_update`
-   `automation_delete`
-   `automation_enable`
-   `automation_disable`
-   `automation_list`
-   `automation_validate`
-   `automation_run_now`
-   `automation_get_run`
-   `automation_get_run_trace`

### Dashboards

-   `dashboard_create`
-   `dashboard_update`
-   `dashboard_delete`
-   `dashboard_list`
-   `dashboard_get`
-   `dashboard_widget_add`
-   `dashboard_widget_update`
-   `dashboard_widget_remove`

### History

-   `history_query_series`
-   `history_query_events`
-   `history_export_csv`
-   `history_set_retention_policy`

## 10.2 Events to frontend

-   `runtime:status`
-   `device:discovered`
-   `device:status`
-   `device:updated`
-   `entity:state`
-   `entity:availability`
-   `command:status`
-   `automation:run`
-   `firmware:build`
-   `firmware:deployment`
-   `history:recorder-status`
-   `operation:log`

High-frequency entity updates should be coalesced or batched before
reaching React. Do not emit an unbounded number of UI events for every
incoming protocol message.

## 10.3 IPC error envelope

Use a consistent serializable error structure:

``` typescript
type AppError = {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  retryable: boolean;
  correlationId?: string;
};
```

Never expose secrets, raw credential material, or sensitive local paths
unnecessarily in user-facing errors.

------------------------------------------------------------------------

# 11. Automation Definition Schema

Persist automation definitions as versioned JSON. The visual editor and
rule builder must use the same canonical model.

Example:

``` json
{
  "schemaVersion": 1,
  "id": "automation-uuid",
  "name": "Ventilate greenhouse",
  "enabled": true,
  "executionPolicy": {
    "overlap": "ignore",
    "cooldownMs": 30000,
    "maxRunDurationMs": 120000,
    "retry": {
      "maxAttempts": 2,
      "backoffMs": 1000
    }
  },
  "triggers": [
    {
      "id": "trigger-1",
      "type": "numeric_threshold",
      "entityId": "temperature-entity-uuid",
      "operator": "greater_than",
      "value": 30,
      "forMs": 60000
    }
  ],
  "condition": {
    "type": "group",
    "operator": "and",
    "children": [
      {
        "type": "numeric_comparison",
        "entityId": "humidity-entity-uuid",
        "operator": "greater_than",
        "value": 65
      },
      {
        "type": "device_available",
        "deviceId": "fan-controller-uuid"
      }
    ]
  },
  "actions": [
    {
      "id": "action-1",
      "type": "set_number",
      "entityId": "fan-speed-entity-uuid",
      "value": 75,
      "timeoutMs": 5000
    },
    {
      "id": "action-2",
      "type": "log_event",
      "message": "Greenhouse ventilation activated"
    }
  ]
}
```

The schema is illustrative. Define formal JSON Schema files, Rust types,
TypeScript types, validation rules, and migration tests. Reject unknown
schema versions safely. Avoid arbitrary code execution inside automation
definitions.

## 11.1 Graph editor representation

The graph editor should persist:

-   Node IDs and node types
-   Node configuration
-   Input/output port identifiers
-   Edge source and target IDs
-   Canvas position and viewport
-   Graph schema version

Separate visual graph layout from execution semantics. Compile the graph
into the canonical automation execution model and validate it before
enabling the automation.

------------------------------------------------------------------------

# 12. Dashboard Definition Schema

Example:

``` json
{
  "schemaVersion": 1,
  "id": "dashboard-uuid",
  "name": "Greenhouse",
  "layout": {
    "columns": 12,
    "rowHeight": 72,
    "gap": 12
  },
  "widgets": [
    {
      "id": "widget-temperature",
      "type": "sensor_value",
      "title": "Temperature",
      "entityId": "temperature-entity-uuid",
      "config": {
        "unit": "°C",
        "precision": 1,
        "showTrend": true
      },
      "layout": {
        "x": 0,
        "y": 0,
        "w": 3,
        "h": 2
      }
    },
    {
      "id": "widget-history",
      "type": "line_chart",
      "title": "Temperature history",
      "entityIds": ["temperature-entity-uuid"],
      "config": {
        "range": "24h",
        "aggregation": "auto",
        "showLegend": false
      },
      "layout": {
        "x": 3,
        "y": 0,
        "w": 9,
        "h": 4
      }
    }
  ]
}
```

Widget configuration must be validated against a widget-specific schema.
A missing entity should produce a recoverable "entity unavailable"
state, not break the dashboard.

------------------------------------------------------------------------

# 13. Frontend UX Specification

## 13.1 Main application layout

-   Left sidebar with Overview, Devices, Configuration, Automations,
    Dashboards, History, and Settings.
-   Top bar with workspace selector, global search, connection/runtime
    indicator, and contextual actions.
-   Main content workspace.
-   Optional inspector panel for selected device, entity, widget, or
    automation node.
-   Toasts for transient success/error feedback.
-   Persistent operation center for builds, deployments, and
    long-running tasks.

## 13.2 Required screens

### Overview

-   Total devices, online devices, unavailable devices.
-   Recent events and automation runs.
-   Active operations.
-   Favorite entity controls.
-   Sensor summary cards.

### Device fleet

-   Search, filter, sort, group, and bulk selection.
-   Columns: name, node name, status, area, firmware, last seen.
-   Device detail tabs: Overview, Entities, History, Logs,
    Configuration, Firmware.

### Entity browser

-   Filter by device, family, area, availability, and text.
-   Display unit, current value, capabilities, and last update.
-   Provide appropriate control based on entity type.

### Configuration projects

-   Project list and file tree.
-   YAML editor.
-   Visual configuration panel.
-   Validation diagnostics.
-   Diff/revision panel.
-   Build and deployment actions.

### Firmware center

-   Build queue and status.
-   Build logs.
-   Artifact details.
-   OTA/serial deployment wizard.
-   Deployment history and device verification.

### Automation builder

-   Simple rule builder.
-   Advanced graph editor.
-   Trigger, condition, action, and flow-control catalogue.
-   Validation panel.
-   Test/dry-run.
-   Execution history and step trace.

### Dashboard editor

-   Widget palette.
-   Grid canvas.
-   Widget inspector.
-   Entity picker.
-   Time range and chart settings.
-   Preview mode and normal view mode.

### History

-   Entity selector.
-   Time range.
-   Multi-series chart.
-   Event timeline.
-   CSV export.

### Settings

-   Workspace and backup.
-   Discovery/network.
-   ESPHome toolchain.
-   Credential management.
-   Recorder and retention.
-   Runtime diagnostics.
-   Theme and UI preferences.

## 13.3 UX rules

-   Every long operation must show progress and allow cancellation when
    safe.
-   Destructive actions require confirmation.
-   Device offline and stale entity data must be visually distinct.
-   Actuator controls must show pending, confirmed, failed, and
    unavailable states.
-   YAML errors must identify file and line/column when available.
-   A dashboard must remain usable when one widget's entity is
    unavailable.
-   Avoid modal dialogs for routine navigation; use side panels and
    dedicated workflows.
-   Provide empty states with a clear next action.

------------------------------------------------------------------------

# 14. Security and Safety

## 14.1 Credential management

-   Use Windows Credential Manager, macOS Keychain, or Linux Secret
    Service where available.
-   Store only secret references in SQLite.
-   Never log API passwords or encryption keys.
-   Redact credentials from process arguments where possible; use secure
    environment or protected files when supported.
-   Restrict file access to user-selected project directories and
    application data directories.

## 14.2 Tauri security

-   Use a strict Content Security Policy.
-   Enable only required Tauri capabilities and permissions.
-   Avoid broad filesystem and shell permissions.
-   Validate all command arguments in Rust.
-   Do not expose arbitrary shell execution to the frontend.
-   Use an allowlisted process adapter for ESPHome toolchain commands.
-   Treat YAML, project files, imported dashboards, and automation JSON
    as untrusted input.

## 14.3 Automation safety

-   Require explicit user enablement before an automation executes.
-   Provide dry-run mode.
-   Bound delays, loops, retries, and total execution time.
-   Prevent infinite automation cycles and recursive automation
    invocation.
-   Validate target entity capabilities and numeric ranges.
-   Offer confirmation policies for configured high-impact actions.
-   Record action intent and result.
-   Provide an emergency disable-all-automations control.
-   Do not describe the product as a certified safety controller.

------------------------------------------------------------------------

# 15. Testing Strategy

## 15.1 Rust unit tests

-   Entity normalization and capability mapping.
-   State quality and stale-state handling.
-   Command validation.
-   Automation condition evaluation.
-   Automation overlap and cooldown policies.
-   Graph compilation and validation.
-   Retention and aggregation logic.
-   Configuration revision and diff behavior.
-   Error mapping and secret redaction.

## 15.2 Integration tests

-   Native API connection and authentication.
-   Device information and entity discovery.
-   Live state subscriptions.
-   Switch and number commands.
-   Disconnect and reconnect.
-   Entity inventory changes.
-   SQLite migrations and repository behavior.
-   Recorder ingestion under load.
-   Automation across two simulated devices.
-   ESPHome CLI validation/build process adapter.
-   OTA deployment against test firmware where practical.

## 15.3 Frontend tests

-   Device list and filters.
-   Dynamic entity controls.
-   Pending/failed command states.
-   Dashboard widget configuration.
-   Dashboard persistence.
-   Automation form validation.
-   Graph editor serialization.
-   History chart query states.
-   Accessibility and keyboard navigation.

## 15.4 End-to-end acceptance environment

Maintain at least: - One ESPHome device exposing a numeric sensor. - One
device exposing a switch. - One device exposing a number/PWM-style
entity. - One test device exposing a binary sensor. - A controllable
network environment for disconnect/reconnect tests.

Also create protocol fixtures and simulated adapters so most tests do
not require physical hardware.

------------------------------------------------------------------------

# 16. Phase-by-Phase Implementation Plan

Each phase must end with a working, testable increment. Do not proceed
by creating placeholder modules for every future feature.

## Phase 0 --- Technical validation spike

**Goal:** prove the highest-risk integrations before committing to the
architecture.

Tasks: 1. Create a minimal Tauri 2 + React + TypeScript application. 2.
Create a Rust workspace and domain skeleton. 3. Test ESPHome Native API
handshake and encryption. 4. Retrieve device info and entity
inventories. 5. Subscribe to numeric and binary state updates. 6. Send
switch and number commands. 7. Test reconnect and authentication
failure. 8. Test ESPHome CLI availability, validation, and build
invocation. 9. Document protocol coverage and unsupported cases. 10.
Decide native Rust client versus isolated adapter process.

**Acceptance criteria** - Connect to at least two real ESPHome devices
or a real device plus a protocol test fixture. - Retrieve entities and
receive live states. - Control at least a switch and numeric entity. -
Recover from a forced disconnect. - Produce a validated firmware build
through the toolchain adapter. - Publish an architecture decision record
for the API client strategy.

## Phase 1 --- Application foundation and device manager

Tasks: 1. Implement application lifecycle and runtime supervisor. 2.
Implement SQLite migrations and repositories. 3. Implement device
registration and mDNS discovery. 4. Implement connection manager and
reconnect policy. 5. Implement entity registry and state store. 6.
Implement device fleet and device detail screens. 7. Implement dynamic
switch and number controls. 8. Implement structured logs and runtime
status.

**Acceptance criteria** - User can discover or manually add devices. -
Device status updates without refreshing the UI. - Entities appear
dynamically. - Switch and number commands work. - Reconnection works
after network interruption. - No UI component owns a protocol
connection.

## Phase 2 --- Historical data and entity controls

Tasks: 1. Implement bounded ingestion queue. 2. Implement current-state
repository. 3. Implement numeric sensor recorder. 4. Implement entity
event recorder. 5. Implement retention configuration. 6. Implement
history query and aggregation. 7. Add initial entity-family controls. 8.
Add history screen and chart components.

**Acceptance criteria** - Selected numeric entities are recorded. -
History remains available after application restart. - Time-range
queries return ordered points. - High-rate test ingestion does not cause
unbounded memory growth. - Unsupported entities render a safe
generic/unsupported state.

## Phase 3 --- Dashboard system

Tasks: 1. Define and validate dashboard/widget schemas. 2. Implement
dashboard CRUD. 3. Implement widget palette and entity picker. 4.
Implement drag-and-drop grid layout. 5. Implement live value, chart,
gauge, binary status, switch, and slider widgets. 6. Implement dashboard
preview and runtime rendering. 7. Implement dashboard import/export.

**Acceptance criteria** - User can create a dashboard with at least four
widget types. - Widget positions and configuration persist after
restart. - Live values update without page reload. - Historical charts
use the recorder. - A missing device/entity does not crash the
dashboard.

## Phase 4 --- Automation runtime

Tasks: 1. Define canonical automation schema. 2. Implement schema
validation and migrations. 3. Implement internal event bus integration.
4. Implement trigger subscriptions. 5. Implement condition evaluator. 6.
Implement switch and number actions. 7. Implement delays, sequences,
timeouts, and overlap policy. 8. Implement persistent run records and
step traces. 9. Implement automation enable/disable and manual run. 10.
Implement dry-run mode.

**Acceptance criteria** - A sensor on device A can trigger a switch or
number action on device B. - Conditions prevent action when false. -
Disconnects produce explicit failures/timeouts. - Run history includes
trigger, steps, timestamps, and outcomes. - Automations continue while
the user navigates between screens. - Runtime restart behavior for
active runs is deterministic and tested.

## Phase 5 --- Automation visual editor

Tasks: 1. Implement friendly rule builder. 2. Implement node palette and
graph canvas. 3. Implement node configuration inspector. 4. Implement
graph validation and compilation. 5. Implement graph serialization. 6.
Implement execution trace visualization. 7. Implement templates and
duplicate automation.

**Acceptance criteria** - Rule builder and graph editor create the same
canonical automation definition. - Invalid graphs cannot be enabled. -
Graphs can be reopened without losing nodes or edges. - Execution traces
map back to graph nodes.

## Phase 6 --- Configuration and firmware management

Tasks: 1. Implement project creation and file tree. 2. Integrate YAML
editor. 3. Implement toolchain detection and version display. 4.
Implement validation and diagnostics. 5. Implement build process
management and logs. 6. Implement artifact tracking. 7. Implement OTA
deployment. 8. Implement serial flashing. 9. Implement revision history
and diff. 10. Implement guided visual configuration for a carefully
scoped set of common options.

**Acceptance criteria** - Existing YAML projects can be opened without
destructive rewriting. - Validation/build errors are visible with useful
context. - A firmware build artifact is stored and associated with its
project revision. - OTA workflow reports deployment and reconnect
verification separately. - Serial flashing works on supported target
platforms. - Unknown YAML is preserved.

## Phase 7 --- Fleet operations, reliability, and release hardening

Tasks: 1. Implement areas, groups, tags, and bulk selection. 2.
Implement staged fleet deployment. 3. Implement workspace
backup/restore. 4. Implement database health and retention tooling. 5.
Implement diagnostics export. 6. Test performance targets. 7. Harden
permissions, CSP, secret handling, and installer packaging. 8. Add
platform-specific setup and recovery documentation. 9. Run full
end-to-end regression suite.

**Acceptance criteria** - User can manage a representative multi-device
fleet. - Bulk operations show per-device results. - Backup/restore
preserves projects, dashboards, automations, and metadata. - Application
survives device/network failures. - Windows, Linux, and macOS release
builds pass smoke tests.

------------------------------------------------------------------------

# 17. Development Standards for Coding Agents

These rules apply to every implementation task.

## 17.1 Before coding

1.  Read this specification and the relevant architecture decision
    records.
2.  Inspect the existing repository before changing files.
3.  Identify the owning crate/module and its public interfaces.
4.  Check whether a domain type or service already exists.
5.  Write a short implementation plan and list files expected to change.
6.  Identify tests and acceptance criteria for the task.
7.  Do not introduce a new dependency without explaining why it is
    needed.

## 17.2 During implementation

-   Implement working behavior; do not substitute mock data for real
    integration unless the task explicitly asks for a simulator.
-   Keep protocol-specific types inside the ESPHome adapter.
-   Keep business logic out of Tauri command handlers and React
    components.
-   Use typed errors and structured logs.
-   Avoid `unwrap()`/`expect()` in production paths unless an invariant
    is documented and genuinely guaranteed.
-   Do not block async runtime threads with long-running processes or
    synchronous I/O.
-   Use cancellation tokens for cancellable long-running tasks.
-   Bound queues, retries, loops, and resource use.
-   Add migrations rather than editing an already released schema in
    place.
-   Do not store credentials in source code, fixtures, logs, or ordinary
    settings.
-   Keep UI state derived from backend events and queries.
-   Keep all user-facing operations observable through progress and
    result states.
-   Add tests for bug fixes and edge cases.

## 17.3 After implementation

1.  Run formatting and linting.
2.  Run relevant unit tests.
3.  Run integration tests affected by the change.
4.  Run frontend type checking and tests.
5.  Report files changed.
6.  Report commands/tests executed and their results.
7.  List known limitations.
8.  Update documentation and schemas.
9.  Do not claim completion if a test was not run.

## 17.4 Required engineering artifacts

-   Architecture Decision Records (ADRs) for major choices.
-   Versioned IPC contracts.
-   Versioned automation and dashboard schemas.
-   Database migrations.
-   Protocol compatibility matrix.
-   Test fixtures for Native API messages.
-   Build and deployment logs with redaction.
-   Phase acceptance checklist.

------------------------------------------------------------------------

# 18. Definition of Done

A feature is done only when:

-   It meets its stated functional requirements.
-   Its public types and IPC contracts are documented.
-   Validation and error states are implemented.
-   Unit tests cover core logic.
-   Integration tests cover external boundaries where practical.
-   UI loading, empty, offline, pending, success, and failure states are
    handled.
-   Logs contain useful context without secrets.
-   No unrelated existing functionality is broken.
-   The coding agent reports actual test results and remaining
    limitations.

A phase is done only when all its acceptance criteria pass on the target
development environment and the phase's documentation is updated.

------------------------------------------------------------------------

# 19. Initial Release Definition

The first meaningful public release should include:

-   Device discovery and manual registration.
-   Native API connectivity and live entity discovery.
-   Device fleet list and device details.
-   Dynamic controls for common entity types.
-   SQLite sensor history and event history.
-   Custom dashboards with live values, charts, gauges, and actuator
    controls.
-   Cross-device automation using triggers, conditions, and actions.
-   Persistent automation execution and trace history.
-   YAML project editor and ESPHome validation/build integration.
-   OTA update workflow.
-   Windows, Linux, and macOS packaging.
-   Backup, logs, and basic diagnostics.

Advanced visual configuration, complex graph flows, staged fleet
rollouts, and device-local automation deployment can follow after the
core system is stable.

------------------------------------------------------------------------

# 20. Official Technical References

Use current official documentation and protocol definitions during
implementation. Verify version compatibility rather than relying on old
examples.

-   ESPHome documentation: https://esphome.io/
-   ESPHome Native API: https://esphome.io/components/api/
-   ESPHome OTA: https://esphome.io/components/ota/
-   ESPHome component index: https://esphome.io/components/
-   ESPHome source and protocol definitions:
    https://github.com/esphome/esphome
-   `aioesphomeapi` reference implementation:
    https://github.com/esphome/aioesphomeapi
-   Tauri 2 documentation: https://v2.tauri.app/
-   React documentation: https://react.dev/
-   Rust documentation: https://doc.rust-lang.org/
-   SQLite documentation: https://www.sqlite.org/docs.html

------------------------------------------------------------------------

# 21. Master Instruction for the Coding Agent

You are implementing **ESPHome Automation Manager**, a local-first,
cross-platform desktop application for managing ESPHome device fleets,
configuring firmware, monitoring live and historical sensor data,
building dashboards, and executing cross-device automations.

Follow this specification as the product and architecture baseline.

Work phase by phase. Do not attempt to implement the entire product in
one pass. Start with Phase 0 and prove the ESPHome Native API and
ESPHome toolchain integrations before building dependent modules.

For every task:

1.  Inspect the repository and existing implementation.
2.  State the planned changes and relevant acceptance criteria.
3.  Implement the smallest complete vertical slice.
4.  Add or update tests.
5.  Run the relevant checks and report their actual results.
6.  Update schemas and documentation.
7.  Identify unresolved issues instead of hiding them behind
    placeholders.

Preserve the architectural boundaries: - React is the presentation
layer. - Tauri is the desktop and IPC boundary. - Rust owns persistent
runtime and domain behavior. - The ESPHome adapter owns Native API
protocol details. - SQLite is accessed through repository/services. -
The automation engine runs independently of UI navigation. - The ESPHome
toolchain adapter owns build, flash, and OTA process integration.

Do not silently discard user YAML, invent unsupported ESPHome
capabilities, equate command submission with physical success, or claim
an automation is persistent when its runtime is not running.

**Begin with Phase 0: produce the repository skeleton, protocol
compatibility investigation, a minimal connection test, and an
architecture decision record. Do not proceed to Phase 1 until the Phase
0 acceptance criteria are demonstrated.**
