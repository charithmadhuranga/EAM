# ESPHome Automation Manager Implementation Plan

## Executive Summary
ESPHome Automation Manager (EAM) is a local-first desktop application for managing, monitoring, configuring, and automating fleets of ESPHome-based devices. This plan outlines the phase-by-phase implementation approach to build this system from Phase 0 through Phase 7.

## Project Overview
- **Primary Stack**: Tauri 2, React, TypeScript, Rust, SQLite
- **Target Platforms**: Windows, Linux, macOS
- **Core Focus**: ESPHome Native API integration, local-first architecture
- **Repository Structure**: Monorepo with dedicated crates for each domain (domain, application, runtime, integrations)

## Phase-by-Phase Implementation Plan

### Phase 0: Technical Validation Spike
**Goal**: Prove highest-risk integrations before committing to architecture

#### Tasks
1. **Create Application Skeleton**
   - Minimal Tauri 2 + React + TypeScript application
   - Basic workspace creation and persistence
   - Core IPC contract definitions

2. **Create Rust Domain Skeleton**
   - Workspace with domain crates (eam-domain, eam-application, eam-runtime)
   - Basic entity and device types with UUIDs
   - SQLite migration framework

3. **Validate ESPHome Native API**
   - TCP connection and handshake testing
   - Encryption/authentication validation
   - Device information retrieval
   - Initial entity list discovery (numeric sensors, switches)
   - Live state subscriptions for key entity types
   - Switch and number commands
   - Connection loss detection and reconnect testing
   - Compatibility testing against multiple ESPHome firmware versions

4. **Test ESPHome Toolchain**
   - CLI detection and validation
   - Project validation using ESPHome toolchain
   - Build invocation and structured log capture
   - Integration with process adapter

5. **Document Architecture Decisions**
   - Architecture Decision Record (ADR) for API client strategy
   - Protocol compatibility matrix
   - Repository structure finalization

#### Acceptance Criteria
- Connect to at least two real ESPHome devices or test fixtures
- Retrieve entities and receive live states
- Control at least a switch and numeric entity
- Recover from forced disconnect
- Produce validated firmware build through toolchain adapter
- Publish architecture decision record

### Phase 1: Application Foundation and Device Manager
**Goal**: Build core application lifecycle and device management capabilities

#### Tasks
1. **Application Lifecycle**
   - Implement Rust runtime supervisor
   - Background task management
   - Graceful shutdown with database flush
   - Persistent runtime independent of UI changes

2. **SQLite Infrastructure**
   - Complete schema implementation
   - Migration framework
   - Repository pattern for all data access
   - Proper indexing and constraints

3. **Device Management**
   - mDNS discovery implementation
   - Manual device registration
   - Connection manager with exponential backoff
   - Device connection state tracking
   - Entity registry and state store

4. **Entity Controls**
   - Dynamic entity discovery from ESPHome API
   - Normalized entity registry
   - Switch and number command execution
   - Entity state quality tracking

5. **UI Integration**
   - Device fleet list and detail screens
   - Dynamic entity controls
   - Device organization (areas, groups, tags)
   - Structured logs and runtime status

#### Acceptance Criteria
- User can discover or manually add devices
- Device status updates without UI refresh
- Entities appear dynamically
- Switch and number commands work
- Reconnection after network interruption
- No UI component owns protocol connection

### Phase 2: Historical Data and Entity Controls
**Goal**: Add historical data recording and comprehensive entity support

#### Tasks
1. **Data Recording**
   - Bounded ingestion queue implementation
   - Current-state repository
   - Numeric sensor recorder with configurable policies
   - Entity event recorder for state changes
   - Batch write transactions

2. **Historical Query**
   - Time-range history queries
   - Latest-value queries
   - Chart-ready aggregation
   - Database optimization for time-based access

3. **Entity Enhancements**
   - Add remaining entity families (lights, fans, climate, locks, etc.)
   - User-defined entity names and metadata
   - Entity history preservation
   - Removed entity detection

4. **UI Components**
   - History screen with charts and timelines
-   CSV export functionality
   - Advanced filters and search
   - Entity-specific controls

#### Acceptance Criteria
- Selected numeric entities are recorded
- History persists after restart
- Time-range queries return ordered data
- High-rate ingestion doesn't cause unbounded memory growth
- Unsupported entities render safe state
- All major entity families supported

### Phase 3: Dashboard System
**Goal**: Create live-updating dashboards with comprehensive widgets

#### Tasks
1. **Dashboard Infrastructure**
   - Dashboard and widget schema definition
   - Validation and migration
   - Grid layout engine with drag-and-drop
   - Dashboard CRUD operations

2. **Widget Implementation**
   - Numeric value card
   - Sensor value with trend
   - Line chart
   - Multi-series line chart
   - Gauge
   - Progress bar
   - Binary status card
   - Switch, light, fan controls
   - Number/PWM slider
   - Select control
   - Button/action card
   - Device health card
   - Event timeline
   - Automation status/list
   - Markdown panel

3. **Dashboard Runtime**
   - Live value updates
   - Historical chart integration
   - Entity picker and widget palette
   - Preview and runtime rendering modes
   - Dashboard import/export

#### Acceptance Criteria
- Create dashboard with at least four widget types
- Widget positions and config persist after restart
- Live values update without page reload
- Historical charts use recorder
- Missing entity doesn't crash dashboard
- Multi-page support implemented

### Phase 4: Automation Runtime
**Goal**: Implement persistent automation engine with full execution capabilities

#### Tasks
1. **Core Engine**
   - Canonical automation schema
   - Internal event bus integration
   - Trigger subscriptions (entity state, numeric thresholds, schedules)
   - Condition evaluator (AND/OR/NOT, comparisons)
   - Switch and number actions
   - Delays, sequences, and timeouts
   - Overlap policies

2. **Execution Management**
   - Persistent run records
   - Step traces
   - Dry-run mode
   - Manual automation execution

3. **UI Integration**
   - Automation list and builder
   - Rule builder and graph editor
   - Validation panel
   - Execution history and traces

#### Acceptance Criteria
- Sensor trigger can execute cross-device action
- Conditions prevent action when false
- Disconnections produce explicit failures
- Run history includes all execution details
- Automations continue during navigation
- Dry-run and test modes work

### Phase 5: Automation Visual Editor
**Goal**: Create intuitive visual automation builder

#### Tasks
1. **Visual Builder**
   - Rule builder interface
   - Node palette with all node types
   - Graph canvas with drag-and-drop
   - Node configuration inspector

2. **Graph Features**
   - Edge connection management
   - Canvas positioning and viewport
   - Validation and compilation
   - Error visualization

3. **Export/Serialization**
   - Graph serialization
   - Import/export support
   - Template management
   - Duplicate automation

#### Acceptance Criteria
- Rule builder and graph editor create same canonical definition
- Invalid graphs cannot be enabled
- Graphs reopen without data loss
- Execution traces map to graph nodes
- Both builders produce identical output

### Phase 6: Configuration and Firmware Management
**Goal**: Complete project lifecycle with YAML editing and firmware deployment

#### Tasks
1. **Project Management**
   - Project creation and file tree
   - YAML editor with syntax highlighting
   - Visual configuration panel
   - Project-to-device associations

2. **Build & Deployment**
   - ESPHome CLI detection and validation
   - Build process management
   - Artifact tracking
   - OTA deployment
   - Serial flashing
   - Revision history and diff

3. **Configuration Features**
   - Guided visual configuration for common components
   - Substitutions, packages, includes
   - Configuration validation before build
   - Revision rollback

#### Acceptance Criteria
- Open existing YAML projects without rewriting
- Validation errors show useful context
- Build artifacts stored and associated
- OTA workflow reports deployment results
- Serial flashing works on supported platforms
- Unknown YAML preserved

### Phase 7: Fleet Operations, Reliability, and Hardening
**Goal**: Production-ready system with comprehensive fleet management

#### Tasks
1. **Fleet Operations**
   - Areas, groups, tags management
   - Bulk operations with per-device results
   - Staged fleet deployment
   - Device health monitoring

2. **System Reliability**
   - Workspace backup and restore
   - Database health and retention
   - Performance optimization
   - Diagnostics export

3. **Security & Packaging**
   - Permission hardening
   - CSP implementation
   - Secret management
   - Cross-platform packaging

4. **Documentation & Testing**
   - Platform-specific setup docs
   - End-to-end regression suite
   - Performance testing against targets
   - User documentation

#### Acceptance Criteria
- Manage representative multi-device fleet
- Bulk operations show detailed results
- Backup/restore preserves all data
- System survives device/network failures
- Windows, Linux, macOS builds pass smoke tests
- All performance targets met

## Implementation Guidelines

### Before Coding
1. Read and understand the complete specification
2. Inspect existing repository (if any)
3. Identify owning crates and modules
4. Write implementation plan and list expected files
5. Identify tests and acceptance criteria
6. Don't introduce new dependencies without justification

### During Implementation
- Implement working behavior, not mocks (unless explicitly asked)
- Keep protocol-specific types isolated in ESPHome adapter
- Keep business logic out of UI components and IPC handlers
- Use typed errors and structured logs
- Avoid `unwrap()`/`expect()` in production paths
- Block async runtime with long-running processes
- Use cancellation tokens for cancellable tasks
- Bound queues, retries, loops, and resource use
- Add migrations for schema changes
- Store credentials securely

### After Implementation
1. Run formatting and linting
2. Run relevant unit tests
3. Run integration tests
4. Run frontend type checking and tests
5. Report files changed and test results
6. Update schemas and documentation
7. List known limitations

## Critical Success Factors
1. **Phase 0**: Must validate ESPHome Native API and toolchain integrations before proceeding
2. **Architecture**: Maintain clear boundaries between frontend, Tauri, Rust core, adapters, and storage
3. **Local-First**: Never depend on cloud services in normal operation
4. **Safety**: Preserve unknown YAML constructs and never rewrite files destructively
5. **Performance**: Meet scalability targets (50+ devices, 2000+ entities, 100 updates/sec)

## Dependencies and Tools
- **ESPHome**: Latest stable CLI, Native API protocol
- **Tauri 2**: Desktop application framework
- **React + TypeScript**: Frontend framework
- **Rust**: With Tokio async runtime
- **SQLite**: With WAL mode and foreign keys
- **aioesphomeapi**: Python adapter (if needed)
- **VS Code / Rust Analyzer**: Development tools

## Risk Mitigation
1. **Protocol Compatibility**: Test against multiple ESPHome firmware versions
2. **Adapter Strategy**: Have both Rust native client and Python bridge ready
3. **Data Loss**: Never overwrite existing YAML, preserve unknown constructs
4. **Performance**: Use bounded queues and transactions for database writes
5. **Security**: Use OS credential store for secrets, never log sensitive data

## Phase 0 Spike Artifacts
1. **Protocol Spike Report**: ESPHome Native API compatibility matrix
2. **Architecture Decision Record**: Native client vs adapter strategy
3. **Repository Skeleton**: Workspace structure with domain crates
4. **Integration Tests**: Real device testing or comprehensive fixtures
5. **Documentation**: Updated specification with validated boundaries

## Initial Release Definition
The first meaningful public release will include:
- Device discovery and manual registration
- Native API connectivity and live entity discovery
- Device fleet management
- Dynamic entity controls (switch, number)
- SQLite sensor history
- Custom dashboards with basic widgets
- Cross-device automations
- Automation execution traces
- YAML project editor
- OTA updates
- Cross-platform desktop packaging
- Backup and basic diagnostics