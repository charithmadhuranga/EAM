# ESPHome Automation Manager — OpenCode Skill

## Project Overview
ESPHome Automation Manager (EAM) is a local-first, cross-platform desktop application for managing,
monitoring, configuring, and automating fleets of ESPHome devices. Stack: Tauri 2, React, TypeScript,
Rust, SQLite.

**Hybrid Connectivity**: Direct local access OR remote via Gateway Server + FRP for fleet management
across multiple sites.

## When to Use This Skill
Use this skill whenever the user asks about implementing, debugging, or extending the ESPHome Automation
Manager project. This includes:
- Any request to modify code in this repository
- Questions about architecture, design, or implementation patterns
- Requests to add features, fix bugs, or write tests
- Planning or scaffolding new phases

## Project Conventions
- **Architecture**: Clean separation — React (presentation), Tauri (IPC), Rust (runtime + domain), adapters (protocol), SQLite (storage), Gateway Server (remote access)
- **Phase gating**: Phase 0 (protocol spike) must complete before Phase 1; each phase gated on previous acceptance criteria
- **Domain boundaries**: Business logic lives in Rust; adapters own protocol types; UI is presentation-only
- **Data model**: UUIDs for primary keys; UTC timestamps; JSONB for capabilities/metadata; Gateways contain Devices
- **Security**: Secrets in OS credential store; never in DB or logs; CSP + strict Tauri capabilities; TLS for all remote connections
- **Performance**: Bounded queues; batched SQLite writes; coalesced UI events; 50+ devices, 2000+ entities, 100 updates/sec targets
- **Connectivity**: Local-first (mDNS direct); Remote access via Gateway Server + FRP; VPN alternative for advanced users

## Key Files
- `esphome_automation_manager_implementation_spec.md` — Product + engineering spec (source of truth)
- `docs/architecture/PLAN.md` — Phase-by-phase implementation plan (includes Phase 8: Gateway Server)
- `docs/architecture/DESIGN.md` — Architecture & design reference
- `docs/architecture/GATEWAY.md` — Gateway Server setup and configuration guide
- `AGENTS.md` — Coding agent instructions
- `schemas/automation/*.schema.json` — Canonical automation schema
- `schemas/dashboard/*.schema.json` — Canonical dashboard schema
- `schemas/gateway/*.schema.json` — Gateway configuration schema
- `docs/decisions/*.md` — Architecture Decision Records (including ADR-008 for Gateway)

## Domain Knowledge
### Gateway Hierarchy
- **Gateway Server**: Headless Rust server on remote network
  - Discovers and manages local ESPHome devices
  - Connects to FRP server (frps) via frpc
  - Provides REST/WebSocket API for desktop app
  - Can be accessed locally (mDNS) or remotely (FRP)
- **Devices**: Belong to a specific Gateway
- **Entities**: Belong to a specific Device
- **Fleet View**: Desktop aggregates all connected Gateways

### Entity Types
Numeric sensors, binary sensors, switches, lights, numbers, selects, buttons, text sensors, text entities,
fans, covers, climate entities, locks. Each entity has a `native_entity_key` derived from stable protocol
identity (never display name).

### Command States
Rejected by local validation → Queued → Sent → Acknowledged by transport → Observed state change →
Timed out → Device unavailable → Failed. Never equate transport acknowledgement with physical success.

### Automation Execution
- Trigger events delivered via internal event bus
- Each trigger has unique event ID + timestamp
- Each run has unique run ID
- Overlap policy per automation (ignore/queue/replace)
- Conditions evaluated against consistent state snapshot
- Every action has timeout + structured result
- Retry behavior bounded and configurable
- Device disconnect produces explicit unavailable result

### Configuration Safety
- YAML is source of truth
- Visual editor operates on structured representation with source mapping
- Unknown keys, custom components, lambdas, packages, includes, unsupported options preserved
- Never silently rewrite entire YAML from incomplete internal model
- Show diff before saving visual edits
- Keep secrets out of logs, crash reports, exported diagnostics

### Frontend UX
- Left sidebar: Overview, Devices, Configuration, Automations, Dashboards, History, Settings
- Top bar: workspace selector, global search, connection/runtime indicator, contextual actions
- Main content workspace
- Optional inspector panel for selected device, entity, widget, or automation node
- Toasts for transient feedback
- Persistent operation center for builds, deployments, long-running tasks

## Workflow
1. **Read the spec** — `esphome_automation_manager_implementation_spec.md`
2. **Check current phase** — `docs/architecture/PLAN.md` (note: Phase 8 added for Gateway Server)
3. **Inspect repo** — use `read`/`grep` to understand existing code
4. **Write implementation plan** — list files to change, tests, acceptance criteria
5. **Implement smallest complete vertical slice** — never create placeholder modules for future features
6. **Add tests** — unit, integration, frontend, gateway
7. **Run checks** — formatting, linting, tests, type-checking
8. **Report results** — actual test results, changed files, remaining limitations
9. **Update docs** — schemas, ADRs, documentation (including Gateway-related updates)

## Key Principles
- **Local-first**: Normal operation must not depend on vendor cloud
- **ESPHome-focused**: Optimize for ESPHome rather than generic smart-home abstraction
- **Capability-driven**: Discover what a device actually exposes; never assume entity capabilities
- **Safe configuration**: Never silently discard YAML or configuration constructs
- **Persistent runtime**: Device connections, recording, schedules, automations must not depend on open dashboard view
- **Observable operations**: Commands, builds, updates, automation runs must have inspectable results
- **Progressive complexity**: Simple guided workflows for new users; direct YAML and advanced controls for experienced users
- **Recoverability**: Configuration revisions, backups, logs, and explicit recovery workflows are first-class features
- **Hybrid connectivity**: Direct local access preferred; remote access via Gateway Server + FRP when needed
- **Fleet management**: Support multiple Gateways (sites); hierarchical view: Gateways → Devices → Entities
- **Hybrid connectivity**: Direct local access preferred; remote access via Gateway Server + FRP when needed
- **Fleet management**: Support multiple Gateways (sites); hierarchical view: Gateways → Devices → Entities

## Out of Scope
- Cloud-hosted account platform or vendor-operated cloud
- General support for arbitrary non-ESPHome protocols and ecosystems
- Mobile applications
- Multi-user RBAC and enterprise tenancy
- Automatic generation of arbitrary ESPHome YAML from natural language
- Guaranteed deployment of every automation into device firmware
- Safety certification or use as a certified industrial safety controller

## Remote Fleet Management
- **Gateway Server**: Required for remote network access (Phase 8)
- **FRP Server**: User-hosted or FRP cloud service for tunneling
- **VPN Alternative**: WireGuard supported for advanced users
- **Setup Priority**: Local-only users don't need Gateway; remote fleet operators do