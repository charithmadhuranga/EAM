# ESPHome Automation Manager - Development Automation
# Cross-platform Makefile for Windows (Git Bash), Linux, and macOS

# Detect OS
ifeq ($(OS),Windows_NT)
    PLATFORM := windows
    SHELL := powershell.exe
    .SHELLFLAGS := -NoLogo -ExecutionPolicy Bypass -Command
    RM := Remove-Item -Recurse -Force
    MKDIR := New-Item -ItemType Directory -Force
    WHICH := Get-Command
else
    PLATFORM := unix
    SHELL := /bin/bash
    RM := rm -rf
    MKDIR := mkdir -p
    WHICH := which
endif

# Project info
PROJECT_NAME := esphome-automation-manager
VERSION := $(shell grep -m1 version package.json | cut -d'"' -f4)

# Directories
ROOT_DIR := $(shell pwd)
BUILD_DIR := $(ROOT_DIR)/dist
DATA_DIR := $(ROOT_DIR)/data
TARGET_DIR := $(ROOT_DIR)/target

# Colors (Unix only)
ifneq ($(PLATFORM),windows)
    GREEN := \033[32m
    YELLOW := \033[33m
    RED := \033[31m
    BLUE := \033[34m
    RESET := \033[0m
else
    GREEN :=
    YELLOW :=
    RED :=
    BLUE :=
    RESET :=
endif

# Phony targets
.PHONY: all help setup build dev test clean lint format check release docker-up docker-down

# Default target
all: setup build

# ============================================================================
# HELP
# ============================================================================

help:
	@echo "$(BLUE)ESPHome Automation Manager - Development Commands$(RESET)"
	@echo ""
	@echo "$(GREEN)Setup & Installation:$(RESET)"
	@echo "  setup          - Initialize development environment"
	@echo "  install        - Install dependencies only"
	@echo "  clean          - Remove build artifacts"
	@echo ""
	@echo "$(GREEN)Development:$(RESET)"
	@echo "  dev            - Start development server (Tauri + Vite)"
	@echo "  dev-frontend   - Start only frontend dev server"
	@echo "  watch-rust     - Watch Rust files and rebuild"
	@echo ""
	@echo "$(GREEN)Building:$(RESET)"
	@echo "  build          - Build all components (debug)"
	@echo "  build-release  - Build optimized release version"
	@echo "  build-tauri    - Build Tauri application bundle"
	@echo ""
	@echo "$(GREEN)Testing:$(RESET)"
	@echo "  test           - Run all tests"
	@echo "  test-unit      - Run unit tests only"
	@echo "  test-int       - Run integration tests"
	@echo "  test-e2e       - Run end-to-end tests"
	@echo "  test-coverage  - Generate test coverage report"
	@echo ""
	@echo "$(GREEN)Code Quality:$(RESET)"
	@echo "  lint           - Run all linters"
	@echo "  format         - Format all code"
	@echo "  check          - Run format, lint, and tests"
	@echo "  audit          - Security audit of dependencies"
	@echo ""
	@echo "$(GREEN)Database:$(RESET)"
	@echo "  db-setup       - Initialize database"
	@echo "  db-migrate     - Run database migrations"
	@echo "  db-reset       - Reset database (WARNING: destroys data)"
	@echo ""
	@echo "$(GREEN)Docker:$(RESET)"
	@echo "  docker-build   - Build Docker images"
	@echo "  docker-up      - Start Docker environment"
	@echo "  docker-down    - Stop Docker environment"
	@echo "  docker-logs    - View Docker logs"
	@echo "  docker-clean   - Remove Docker volumes"
	@echo ""
	@echo "$(GREEN)Release:$(RESET)"
	@echo "  release        - Build release binaries"
	@echo "  release-win    - Build Windows release"
	@echo "  release-linux  - Build Linux release"
	@echo "  release-mac    - Build macOS release"
	@echo ""
	@echo "$(GREEN)Utilities:$(RESET)"
	@echo "  docs           - Generate documentation"
	@echo "  fixtures-start - Start mock ESPHome devices"
	@echo "  fixtures-stop  - Stop mock devices"
	@echo "  perf-test      - Run performance tests"
	@echo ""

# ============================================================================
# SETUP & INSTALLATION
# ============================================================================

setup: check-prerequisites install init-dirs
	@echo "$(GREEN)✓ Development environment ready!$(RESET)"

check-prerequisites:
	@echo "$(BLUE)Checking prerequisites...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "if (!(Get-Command rustc -ErrorAction SilentlyContinue)) { Write-Host 'Rust not found. Please install from https://rustup.rs'; exit 1 }"
	@powershell -Command "if (!(Get-Command node -ErrorAction SilentlyContinue)) { Write-Host 'Node.js not found. Please install from https://nodejs.org'; exit 1 }"
	@powershell -Command "if (!(Get-Command pnpm -ErrorAction SilentlyContinue)) { Write-Host 'pnpm not found. Run: npm install -g pnpm'; exit 1 }"
else
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)Rust not found. Install from https://rustup.rs$(RESET)"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "$(RED)Node.js not found. Install from https://nodejs.org$(RESET)"; exit 1; }
	@command -v pnpm >/dev/null 2>&1 || { echo "$(RED)pnpm not found. Run: npm install -g pnpm$(RESET)"; exit 1; }
endif
	@echo "$(GREEN)✓ All prerequisites met$(RESET)"

install:
	@echo "$(BLUE)Installing dependencies...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm install"
	@powershell -Command "cargo fetch"
else
	pnpm install
	cargo fetch
endif
	@echo "$(GREEN)✓ Dependencies installed$(RESET)"

init-dirs:
	@echo "$(BLUE)Creating directories...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "New-Item -ItemType Directory -Force -Path data, dist, logs, tools/fixture-server"
else
	$(MKDIR) data dist logs tools/fixture-server
endif
	@echo "$(GREEN)✓ Directories created$(RESET)"

# ============================================================================
# DEVELOPMENT
# ============================================================================

dev:
	@echo "$(BLUE)Starting development server...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run dev"
else
	pnpm run dev
endif

dev-frontend:
	@echo "$(BLUE)Starting frontend dev server...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm --filter desktop dev"
else
	pnpm --filter desktop dev
endif

watch-rust:
	@echo "$(BLUE)Watching Rust files...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo watch -x build -x test"
else
	cargo watch -x build -x test
endif

# ============================================================================
# BUILDING
# ============================================================================

build: build-frontend build-rust
	@echo "$(GREEN)✓ Build complete!$(RESET)"

build-frontend:
	@echo "$(BLUE)Building frontend...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run build"
else
	pnpm run build
endif

build-rust:
	@echo "$(BLUE)Building Rust backend...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo build --workspace"
else
	cargo build --workspace
endif

build-release:
	@echo "$(BLUE)Building release version...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo build --release"
	@powershell -Command "pnpm run build"
else
	cargo build --release
	pnpm run build
endif
	@echo "$(GREEN)✓ Release build complete!$(RESET)"

build-tauri:
	@echo "$(BLUE)Building Tauri application...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run tauri build"
else
	pnpm run tauri build
endif

# ============================================================================
# TESTING
# ============================================================================

test: test-rust test-frontend
	@echo "$(GREEN)✓ All tests passed!$(RESET)"

test-rust:
	@echo "$(BLUE)Running Rust tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo test --workspace --all-features -- --nocapture"
else
	cargo test --workspace --all-features -- --nocapture
endif

test-frontend:
	@echo "$(BLUE)Running frontend tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run test"
else
	pnpm run test
endif

test-unit:
	@echo "$(BLUE)Running unit tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo test --workspace --lib"
	@powershell -Command "pnpm run test:unit"
else
	cargo test --workspace --lib
	pnpm run test:unit
endif

test-int:
	@echo "$(BLUE)Running integration tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo test --workspace --test '*'"
	@powershell -Command "pnpm run test:integration"
else
	cargo test --workspace --test '*'
	pnpm run test:integration
endif

test-e2e:
	@echo "$(BLUE)Running E2E tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run test:e2e"
else
	pnpm run test:e2e
endif

test-coverage:
	@echo "$(BLUE)Generating test coverage...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo tarpaulin --workspace --out Html --output-dir coverage"
else
	cargo tarpaulin --workspace --out Html --output-dir coverage
endif
	@echo "$(GREEN)Coverage report: coverage/index.html$(RESET)"

# ============================================================================
# CODE QUALITY
# ============================================================================

lint: lint-rust lint-frontend
	@echo "$(GREEN)✓ Linting complete!$(RESET)"

lint-rust:
	@echo "$(BLUE)Linting Rust code...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo clippy --workspace --all-targets -- -D warnings"
else
	cargo clippy --workspace --all-targets -- -D warnings
endif

lint-frontend:
	@echo "$(BLUE)Linting frontend code...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run lint"
else
	pnpm run lint
endif

format: format-rust format-frontend
	@echo "$(GREEN)✓ Formatting complete!$(RESET)"

format-rust:
	@echo "$(BLUE)Formatting Rust code...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo fmt --all"
else
	cargo fmt --all
endif

format-frontend:
	@echo "$(BLUE)Formatting frontend code...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "pnpm run format"
else
	pnpm run format
endif

check: format lint test
	@echo "$(GREEN)✓ All checks passed!$(RESET)"

audit:
	@echo "$(BLUE)Running security audit...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo audit"
	@powershell -Command "pnpm audit"
else
	cargo audit
	pnpm audit
endif
	@echo "$(GREEN)✓ Security audit complete!$(RESET)"

# ============================================================================
# DATABASE
# ============================================================================

db-setup:
	@echo "$(BLUE)Setting up database...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "New-Item -ItemType Directory -Force -Path data"
	@powershell -Command "cargo run --bin eam-migrate --no-default-features"
else
	$(MKDIR) data
	cargo run --bin eam-migrate --no-default-features
endif
	@echo "$(GREEN)✓ Database initialized$(RESET)"

db-migrate:
	@echo "$(BLUE)Running migrations...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo run --bin eam-migrate"
else
	cargo run --bin eam-migrate
endif
	@echo "$(GREEN)✓ Migrations applied$(RESET)"

db-reset:
	@echo "$(RED)WARNING: This will delete all data!$(RESET)"
	@echo "Press Ctrl+C to cancel, or Enter to continue..."
	@read confirm
ifeq ($(PLATFORM),windows)
	@powershell -Command "Remove-Item -Recurse -Force data/*.db -ErrorAction SilentlyContinue"
	@powershell -Command "cargo run --bin eam-migrate"
else
	$(RM) data/*.db
	cargo run --bin eam-migrate
endif
	@echo "$(GREEN)✓ Database reset$(RESET)"

# ============================================================================
# DOCKER
# ============================================================================

docker-build:
	@echo "$(BLUE)Building Docker images...$(RESET)"
	docker-compose build

docker-up:
	@echo "$(BLUE)Starting Docker environment...$(RESET)"
	docker-compose up -d
	@echo "$(GREEN)✓ Docker environment running$(RESET)"
	@echo "Development container: docker exec -it esphome-automation-manager-dev-1 bash"
	@echo "Mock device 1: localhost:6053"
	@echo "Mock device 2: localhost:6054"

docker-down:
	@echo "$(BLUE)Stopping Docker environment...$(RESET)"
	docker-compose down
	@echo "$(GREEN)✓ Docker environment stopped$(RESET)"

docker-logs:
	docker-compose logs -f

docker-clean:
	@echo "$(RED)WARNING: This will remove all Docker volumes!$(RESET)"
	docker-compose down -v
	@echo "$(GREEN)✓ Docker volumes removed$(RESET)"

# ============================================================================
# RELEASE
# ============================================================================

release: check build-tauri
	@echo "$(GREEN)✓ Release build complete!$(RESET)"

release-win:
	@echo "$(BLUE)Building Windows release...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo tauri build --target x86_64-pc-windows-msvc"
else
	cargo tauri build --target x86_64-pc-windows-msvc
endif

release-linux:
	@echo "$(BLUE)Building Linux release...$(RESET)"
	cargo tauri build --target x86_64-unknown-linux-gnu

release-mac:
	@echo "$(BLUE)Building macOS releases...$(RESET)"
	cargo tauri build --target x86_64-apple-darwin
	cargo tauri build --target aarch64-apple-darwin

# ============================================================================
# UTILITIES
# ============================================================================

docs:
	@echo "$(BLUE)Generating documentation...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo doc --workspace --no-deps"
	@powershell -Command "pnpm run docs"
else
	cargo doc --workspace --no-deps
	pnpm run docs
endif
	@echo "$(GREEN)Documentation generated in target/doc$(RESET)"

fixtures-start:
	@echo "$(BLUE)Starting mock ESPHome devices...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "Start-Process python -ArgumentList 'tools/fixture-server/mock_device.py --port 6053' -WindowStyle Hidden"
	@powershell -Command "Start-Process python -ArgumentList 'tools/fixture-server/mock_device.py --port 6054' -WindowStyle Hidden"
else
	python tools/fixture-server/mock_device.py --port 6053 &
	python tools/fixture-server/mock_device.py --port 6054 &
endif
	@echo "$(GREEN)✓ Mock devices running on ports 6053 and 6054$(RESET)"

fixtures-stop:
	@echo "$(BLUE)Stopping mock devices...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "Get-Process -Name python -ErrorAction SilentlyContinue | Where-Object {$_.CommandLine -like '*mock_device*'} | Stop-Process -Force"
else
	pkill -f mock_device.py || true
endif
	@echo "$(GREEN)✓ Mock devices stopped$(RESET)"

perf-test:
	@echo "$(BLUE)Running performance tests...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo test --workspace --release -- --nocapture performance"
else
	cargo test --workspace --release -- --nocapture performance
endif

# ============================================================================
# CLEAN
# ============================================================================

clean:
	@echo "$(BLUE)Cleaning build artifacts...$(RESET)"
ifeq ($(PLATFORM),windows)
	@powershell -Command "cargo clean"
	@powershell -Command "Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue"
	@powershell -Command "Remove-Item -Recurse -Force node_modules -ErrorAction SilentlyContinue"
	@powershell -Command "pnpm store prune"
else
	cargo clean
	$(RM) dist
	$(RM) node_modules
	pnpm store prune
endif
	@echo "$(GREEN)✓ Clean complete$(RESET)"

# ============================================================================
# CI/CD TARGETS
# ============================================================================

ci: format lint test
	@echo "$(GREEN)✓ CI checks passed$(RESET)"

ci-coverage: test-coverage
	@echo "$(GREEN)✓ Coverage report generated$(RESET)"

# ============================================================================
# GITHUB ACTIONS HELPERS
# ============================================================================

cache-rust:
	@echo "Caching Rust dependencies..."
	@echo "Target: $(TARGET_DIR)"
	@echo "Registry: ~/.cargo/registry"
	@echo "Git: ~/.cargo/git"

cache-node:
	@echo "Caching Node dependencies..."
	@echo "node_modules: $(ROOT_DIR)/node_modules"
	@echo "pnpm store: ~/.local/share/pnpm/store"
