# ESPHome Automation Manager

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/workflow/status/esphome-automation/manager/CI)](https://github.com/esphome-automation/manager/actions)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Node](https://img.shields.io/badge/node-20%2B-green.svg)](https://nodejs.org/)

**Local-first desktop application for managing ESPHome device fleets**

## Overview

ESPHome Automation Manager (EAM) is a cross-platform desktop application that combines device configuration, monitoring, dashboards, and automation in one unified tool.

### Key Features

- **Device Management**: Discover, configure, and monitor ESPHome devices
- **Entity Control**: Real-time control of switches, lights, sensors, and more
- **Historical Data**: SQLite-based sensor history with charts
- **Dashboards**: Customizable drag-and-drop dashboard builder
- **Automations**: Visual automation builder with cross-device logic
- **Configuration**: Full YAML editor with syntax highlighting and validation
- **Firmware Management**: Build, flash, and OTA update ESPHome firmware
- **Local-First**: No cloud dependency for normal operation
- **Remote Fleet Management**: Access devices on remote networks via Gateway Server + FRP

## Tech Stack

| Component | Technology |
|-----------|------------|
| Frontend | React 18, TypeScript 5.3, Vite |
| Desktop Framework | Tauri 2.0 |
| Backend Core | Rust, Tokio |
| Database | SQLite (WAL mode) |
| Protocol | ESPHome Native API |
| Package Manager | pnpm (frontend), Cargo (Rust) |

## Quick Start

### Prerequisites

- **Rust**: 1.75+ ([Install](https://rustup.rs/))
- **Node.js**: 20 LTS ([Install](https://nodejs.org/))
- **pnpm**: 8.15+ (`npm install -g pnpm`)
- **Git Bash**: Already installed on Windows

### Installation

```bash
# Clone the repository
git clone https://github.com/esphome-automation/manager.git
cd manager

# Setup development environment
make setup

# Start development server
make dev
```

### Development Commands

```bash
# Show all available commands
make help

# Run tests
make test

# Build for production
make build

# Run linters
make lint

# Format code
make format

# Run all checks
make check
```

## Project Structure

```
esphome-automation-manager/
├── apps/
│   ├── desktop/           # Tauri + React application
│   │   ├── src/           # React frontend
│   │   └── src-tauri/     # Rust backend
│   └── gateway-server/    # Headless gateway for remote access
├── crates/                # Rust crates
│   ├── eam-domain/        # Core domain types
│   ├── eam-application/   # Use cases & ports
│   ├── eam-runtime/       # Lifecycle management
│   ├── eam-storage/       # SQLite repositories
│   ├── eam-device-manager/
│   ├── eam-entity-registry/
│   ├── eam-automation/
│   ├── eam-history/
│   ├── eam-dashboard/
│   ├── eam-config/
│   ├── eam-toolchain/
│   ├── eam-discovery/
│   ├── eam-security/
│   ├── eam-observability/
│   └── eam-esphome-api/   # Native API client
├── schemas/               # JSON schemas
│   ├── automation/
│   ├── dashboard/
│   └── gateway/           # Gateway configuration schema
├── docs/                  # Documentation
│   └── architecture/
├── tools/                 # Development tools
└── tests/                 # Test suites
```

## Architecture

EAM follows clean architecture principles with hybrid connectivity support:

```
┌──────────────────────────────────────┐
│     React Frontend (TypeScript)       │
│  Presentation & User Interaction      │
└────────────────┬─────────────────────┘
                 │ Tauri IPC
┌────────────────┴─────────────────────┐
│     Tauri Commands (Rust)             │
│  Typed request/response handling      │
└────────────────┬─────────────────────┘
                 │
┌────────────────┴─────────────────────┐
│     Application Layer (Rust)          │
│  Use cases, ports, runtime services   │
│  Gateway Manager (remote fleet)       │
└────────────────┬─────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
┌───┴───┐   ┌───┴───┐   ┌───┴───┐
│ESPHome│   │Storage│   │Toolchain│
│ API   │   │SQLite │   │Adapter │
└───┬───┘   └───────┘   └────────┘
    │
    ├─────► Local Devices (mDNS)
    │
    └─────► Remote Gateways ──► FRP Server ──► Remote Devices
```

### Key Principles

1. **Business logic in Rust** - Never in UI or Tauri commands
2. **Local-first** - No cloud dependency for core features
3. **Persistent runtime** - Automations run independently of UI
4. **Safe configuration** - YAML is source of truth, unknown constructs preserved
5. **Hybrid connectivity** - Direct local access OR remote via Gateway Server + FRP

## Documentation

- [Implementation Plan](PLAN.md) - Phase-by-phase development roadmap (includes Phase 8: Gateway Server)
- [Architecture Design](docs/architecture/DESIGN.md) - System design and interfaces
- [Architecture Plan](docs/architecture/PLAN.md) - Detailed phase plan
- [Gateway Server Guide](docs/architecture/GATEWAY.md) - Remote fleet management setup
- [Agent Instructions](AGENTS.md) - Guidelines for AI development assistants
- [Skill Definition](SKILL.md) - Domain knowledge for automation

## Real Device Testing

Phase 0 was validated against a real ESP32 (`eam-test-device`,
ESPHome 2026.10.2) flashed over serial:

- Handshake, device info, entity listing, live states
- Switch + number command round-trips
- Reconnect recovery after abrupt disconnect
- Toolchain: `esphome config`, serial flash, OTA

See [`tests/device/README.md`](tests/device/README.md) for the firmware and
[`docs/decisions/ADR-009-esphome-api-client-strategy.md`](docs/decisions/ADR-009-esphome-api-client-strategy.md)
for the validation report.

```sh
# Run the opt-in real-device tests (skipped when the variable is unset)
EAM_LIVE_DEVICE=10.60.109.61:6053 make test-live
```

## Development

### Phase Status

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 0 | ✅ Complete | Technical Validation Spike (validated on real ESP32) |
| Phase 1 | 🔲 Pending | Application Foundation |
| Phase 2 | 🔲 Pending | Historical Data & Entity Controls |
| Phase 3 | 🔲 Pending | Dashboard System |
| Phase 4 | 🔲 Pending | Automation Runtime |
| Phase 5 | 🔲 Pending | Automation Visual Editor |
| Phase 6 | 🔲 Pending | Configuration & Firmware |
| Phase 7 | 🔲 Pending | Fleet Operations & Hardening |
| Phase 8 | 🔲 Pending | Gateway Server & Remote Fleet Management |

### Testing

```bash
# Run all tests
make test

# Run with coverage
make test-coverage

# Run specific test suites
make test-unit        # Unit tests
make test-int         # Integration tests
make test-e2e         # End-to-end tests
```

### Docker Development

```bash
# Start Docker environment
make docker-up

# Connect to dev container
docker exec -it eam-dev-1 bash

# Stop Docker environment
make docker-down
```

## Contributing

We welcome contributions! Please see our contributing guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- **Rust**: `cargo fmt` + `cargo clippy`
- **TypeScript**: `pnpm run format` + `pnpm run lint`
- **Commits**: Conventional Commits specification

## Roadmap

### v0.1.0 (MVP)
- [ ] Device discovery and connection
- [ ] Basic entity control (switch, number)
- [ ] SQLite sensor history
- [ ] Simple dashboards

### v0.2.0
- [ ] Extended entity support (lights, fans, climate)
- [ ] Dashboard widgets library
- [ ] Basic automations

### v0.3.0
- [ ] Visual automation builder
- [ ] YAML configuration editor
- [ ] Firmware build and OTA

### v0.4.0
- [ ] Gateway Server for remote access
- [ ] FRP integration
- [ ] Multi-site fleet management

### v1.0.0
- [ ] Fleet operations
- [ ] Advanced dashboards
- [ ] Production-ready packaging
- [ ] Complete remote fleet support

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [ESPHome](https://esphome.io/) - Amazing firmware framework for ESP8266/ESP32
- [Tauri](https://tauri.app/) - Cross-platform desktop framework
- [React](https://react.dev/) - UI library
- [Rust](https://www.rust-lang.org/) - Systems programming language

## Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/esphome-automation/manager/issues)
- **Discussions**: [GitHub Discussions](https://github.com/esphome-automation/manager/discussions)

---

**Note**: This project is in early development. Features and APIs may change significantly before v1.0.0.
