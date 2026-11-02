# ESPHome Automation Manager — Agent Instructions

## Context
This project (workspace root: `C:\Users\nikhu\OneDrive\Desktop\Esphome-Automation`) implements the
**ESPHome Automation Manager (EAM)**: a local-first, cross-platform desktop application for managing,
monitoring, configuring, and automating fleets of ESPHome devices. See `docs/architecture/PLAN.md` for the
full phase-by-phase plan. See `esphome_automation_manager_implementation_spec.md` as the product baseline.

## Working Directory
All agent work operates inside `C:\Users\nikhu\OneDrive\Desktop\Esphome-Automation`. The repo will be a
pnpm workspace + Cargo workspace monorepo in this directory. Code lives under
`apps/desktop/`, `crates/`, `schemas/`, `docs/`, `scripts/`.

## Conventions (from the spec)
- React = presentation layer only. UI must not own protocol connections, automation timers, or
  historical recording.
- Tauri = desktop IPC boundary. Commands call application use cases; business logic lives in Rust.
- Rust owns persistent runtime and domain behavior. No business logic in Tauri commands or React.
- ESPHome adapter owns protocol types; keep everything versioned inside it.
- SQLite accessed only through repository/service abstractions.
- Automations run independently of UI navigation state.
- Toolchain adapter owns build/flash/OTA process integration.

## Coding Rules
1. Read the spec (`esphome_automation_manager_implementation_spec.md`) and relevant architecture docs before
   changing anything.
2. Identify the owning crate/module and its public interfaces first.
3. Write a short implementation plan listing files that will change, plus tests and acceptance criteria.
4. Implement the smallest complete vertical slice for each task. Never create placeholder modules for every
   future feature.
5. Add tests for bug fixes and edge cases. Do not claim completion if a test was not run.
6. Run formatting/linting, unit tests, integration tests, frontend type-check/tests, then report actual results
   with changed files and remaining limitations.
7. Use UTC epoch milliseconds or RFC 3339 timestamps; never store plaintext secrets.
8. Add migrations, never edit an already released schema in place.
9. Bound queues, retries, loops, and resource use. Do not block async runtime threads on long-running I/O.
10. Keep user-facing operations observable via progress and result states.

## Before Modifying Code
- Read the spec and architecture docs.
- Inspect the repo structure (`read` / `grep` the tree).
- Check existing dependencies (`Cargo.toml`, `package.json`, `pnpm-workspace.yaml`).
- Record Architecture Decision Records in `docs/decisions/` for any significant architectural choice.

## Phased Approach
Phase 0 MUST validate the highest-risk integrations (ESPHome Native API, toolchain) before committing to
architecture. Acceptance criteria for Phase 0: two real devices or fixtures, entity retrieval, live
states, switch+number control, reconnect recovery, validated build, ADR published. Start every task at
Phase 0 and gate each subsequent phase on the previous one's acceptance criteria.

## Immediate Next Steps for the Agent
1. Scaffold the pnpm + Cargo workspace.
2. Create Phase 0 spike: minimal Tauri 2 app, domain crates, ESPHome Native API test, toolchain test, ADR.
3. Then proceed Phase by Phase as documented in `docs/architecture/PLAN.md`.

## Files That Definitely Exists / Should Be Created
- `AGENTS.md` — these instructions
- `PLAN.md` — the full implementation plan
- `DESIGN.md` — design decisions and interface contracts
- `SKILL.md` — project skill for OpenCode agents working here
- `schemas/automation/*.schema.json` — canonical automation schema
- `schemas/dashboard/*.schema.json` — canonical dashboard schema
- `docs/architecture/PLAN.md`
- `docs/architecture/DESIGN.md`
- `.opencode/skills/skills/<domain>.md` — per-domain skills