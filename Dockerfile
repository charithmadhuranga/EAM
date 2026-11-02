# ESPHome Automation Manager - Development Container
# Multi-stage build for optimized development and production images

# Base stage with common dependencies
FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye AS base

# Avoid warnings by switching to noninteractive
ENV DEBIAN_FRONTEND=noninteractive

# Install common dependencies
RUN apt-get update && apt-get install -y \
    # Build essentials
    build-essential \
    pkg-config \
    # SSL/TLS
    libssl-dev \
    # SQLite
    sqlite3 \
    libsqlite3-dev \
    # Git
    git \
    git-lfs \
    # Process management
    procps \
    # Network utilities
    curl \
    wget \
    iputils-ping \
    # Text editors
    nano \
    vim \
    # Clean up
    && rm -rf /var/lib/apt/lists/*

# Development stage
FROM base AS development

# Install Node.js 20 LTS
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g pnpm@8.15.0

# Install Tauri dependencies for Linux builds
RUN apt-get update && apt-get install -y \
    # GTK
    libgtk-3-dev \
    # WebKit
    libwebkit2gtk-4.0-dev \
    # AppIndicator (for system tray)
    libappindicator3-dev \
    # RSVG
    librsvg2-dev \
    # Patchelf (for AppImage)
    patchelf \
    # Clean up
    && rm -rf /var/lib/apt/lists/*

# Install Python and ESPHome CLI for testing
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    && python3 -m pip install --no-cache-dir esphome \
    && rm -rf /var/lib/apt/lists/*

# Install additional development tools
RUN cargo install \
    cargo-watch \
    cargo-edit \
    cargo-audit \
    cargo-outdated

# Configure shell
RUN echo 'export PATH="$HOME/.local/share/pnpm:$PATH"' >> ~/.bashrc

# Set working directory
WORKDIR /workspace

# Configure Git safe directory
RUN git config --global --add safe.directory /workspace

# Default command
CMD ["sleep", "infinity"]

# Builder stage for production builds
FROM base AS builder

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g pnpm@8.15.0

# Install Tauri build dependencies
RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY crates ./crates
COPY apps ./apps
COPY schemas ./schemas

# Build dependencies
RUN cargo fetch
RUN pnpm install --frozen-lockfile

# Build the application
RUN cargo build --release
RUN pnpm run build

# Production stage (minimal runtime)
FROM debian:bullseye-slim AS production

RUN apt-get update && apt-get install -y \
    libgtk-3-0 \
    libwebkit2gtk-4.0-37 \
    libappindicator3-1 \
    librsvg2-2 \
    libssl1.1 \
    sqlite3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built artifacts from builder
COPY --from=builder /build/apps/desktop/src-tauri/target/release/esphome-automation-manager ./

# Create non-root user
RUN useradd -m -s /bin/bash eam \
    && chown -R eam:eam /app

USER eam

CMD ["./esphome-automation-manager"]

# Test runner stage
FROM development AS test-runner

WORKDIR /workspace

# Copy test scripts
COPY scripts/run-tests.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/run-tests.sh

# Default test command
CMD ["run-tests.sh"]
