# ESPHome Automation Manager - Comprehensive Implementation Plan

## Table of Contents
- [Executive Summary](#executive-summary)
- [Architecture Overview](#architecture-overview)
- [Gateway Server & FRP Integration](#gateway-server--frp-integration)
- [Development Environment Setup](#development-environment-setup)
- [Phase 0: Technical Validation Spike](#phase-0-technical-validation-spike)
- [Phase 1: Application Foundation](#phase-1-application-foundation)
- [Phase 2: Historical Data & Entity Controls](#phase-2-historical-data--entity-controls)
- [Phase 3: Dashboard System](#phase-3-dashboard-system)
- [Phase 4: Automation Runtime](#phase-4-automation-runtime)
- [Phase 5: Automation Visual Editor](#phase-5-automation-visual-editor)
- [Phase 6: Configuration & Firmware Management](#phase-6-configuration--firmware-management)
- [Phase 7: Fleet Operations & Hardening](#phase-7-fleet-operations--hardening)
- [Phase 8: Gateway Server & Remote Fleet Management](#phase-8-gateway-server--remote-fleet-management)
- [Testing Strategy](#testing-strategy)
- [Deployment & Operations](#deployment--operations)

---

## Executive Summary

ESPHome Automation Manager (EAM) is a local-first, cross-platform desktop application for managing, monitoring, configuring, and automating fleets of ESPHome devices.

**Technology Stack:**
- **Frontend**: React 18+ with TypeScript 5.3+, Vite build system
- **Desktop Framework**: Tauri 2.0 (Rust-based)
- **Backend Core**: Rust with Tokio async runtime
- **Database**: SQLite with WAL mode
- **Protocol**: ESPHome Native API
- **Package Management**: pnpm workspaces (frontend) + Cargo workspaces (Rust)

**Architecture Principles:**
- Clean separation: React (presentation) → Tauri IPC → Rust core → Adapters → Storage
- Business logic lives in Rust, never in UI or Tauri commands
- Local-first: no cloud dependency for normal operation
- Persistent runtime: automations run independently of UI state
- **Hybrid connectivity**: Direct local access OR remote via Gateway Server + FRP

---

## Architecture Overview

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        DESKTOP APPLICATION (Tauri)                          │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  React UI (Presentation Layer)                                        │  │
│  │  - Device Fleet Management  - Dashboards  - Automations              │  │
│  │  - Configuration Editor     - History     - Settings                 │  │
│  └────────────────────────────┬─────────────────────────────────────────┘  │
│                               │ Tauri IPC                                    │
│  ┌────────────────────────────▼─────────────────────────────────────────┐  │
│  │  Rust Core Runtime                                                    │  │
│  │  - Device Manager  - Entity Registry  - Automation Engine            │  │
│  │  - State Store     - History Recorder  - Toolchain Adapter           │  │
│  └────────────────────────────┬─────────────────────────────────────────┘  │
└───────────────────────────────┼─────────────────────────────────────────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
        ┌───────▼────────┐            ┌────────▼────────┐
        │  LOCAL NETWORK │            │  REMOTE ACCESS  │
        │  (Direct mDNS) │            │   via FRP/VPN   │
        └───────┬────────┘            └────────┬────────┘
                │                               │
        ┌───────▼────────┐            ┌────────▼────────┐
        │  ESPHome       │            │  FRP Server     │
        │  Devices       │            │  (frps)         │
        │  (Port 6053)   │            │  Public IP      │
        └────────────────┘            └────────┬────────┘
                                               │
                                      ┌────────▼────────┐
                                      │  Gateway Server │
                                      │  (Headless)     │
                                      │  - frpc         │
                                      │  - Local mDNS   │
                                      │  - ESPHome API  │
                                      └────────┬────────┘
                                               │
                                      ┌────────▼────────┐
                                      │  Remote ESPHome │
                                      │  Devices        │
                                      │  (Port 6053)    │
                                      └─────────────────┘
```

---

## Gateway Server & FRP Integration

### Overview

The **Gateway Server** is a headless Rust server that enables remote fleet management when the desktop application is not on the local network. It acts as a bridge between ESPHome devices on a remote network and the desktop application.

**Key Components:**

1. **Gateway Server (Headless Rust Server)**
   - Runs on a device (Raspberry Pi, mini PC, etc.) in the remote network
   - Discovers and manages local ESPHome devices via mDNS
   - Provides ESPHome Native API proxy/relay functionality
   - Maintains persistent connection to FRP server

2. **FRP Server (frps)**
   - Public-facing server with static IP/domain
   - Accepts connections from Gateway Servers (frpc clients)
   - Provides secure tunnel for desktop app to reach remote gateways
   - Can be self-hosted or use FRP cloud services

3. **Desktop Application Integration**
   - Auto-discovers local gateways via mDNS (same network)
   - Connects to remote gateways via FRP server
   - Displays hierarchical view: Gateways → Devices → Entities
   - Seamless switching between local and remote access

### Connection Modes

#### Mode 1: Local Network (Direct)

```
Desktop App ←──mDNS──→ ESPHome Devices
                    (Port 6053)
```

**Use Case:** User is on the same network as devices.

**Process:**
1. Desktop app scans local network via mDNS
2. Discovers ESPHome devices directly
3. Connects via ESPHome Native API (port 6053)
4. No Gateway Server or FRP needed

#### Mode 2: Remote Network via Gateway Server

```
Desktop App ←──TLS──→ FRP Server ←──TLS──→ Gateway Server ←──Local──→ ESPHome Devices
                     (Public IP)                (frpc)             (Port 6053)
```

**Use Case:** User is remote (different network, mobile, etc.)

**Process:**
1. Gateway Server runs on remote network
2. Gateway connects to FRP server (frps) via frpc
3. Desktop app connects to FRP server
4. Desktop discovers all connected gateways
5. Selects gateway and accesses its devices
6. All traffic routed through secure FRP tunnel

### Gateway Server Architecture

```rust
// apps/gateway-server/src/main.rs

/// Gateway Server - Headless ESPHome fleet manager for remote access
pub struct GatewayServer {
    config: GatewayConfig,
    device_manager: DeviceManager,
    frp_client: FrpClient,
    api_server: ApiServer,
    state_store: StateStore,
}

/// Gateway configuration
pub struct GatewayConfig {
    /// Gateway unique identifier
    pub gateway_id: Uuid,
    
    /// Gateway display name (user-defined)
    pub display_name: String,
    
    /// Local network configuration
    pub local_network: LocalNetworkConfig,
    
    /// FRP client configuration
    pub frp_config: FrpClientConfig,
    
    /// API server configuration (for local direct access)
    pub api_server: ApiServerConfig,
    
    /// Security settings
    pub security: SecurityConfig,
}

/// FRP client configuration
pub struct FrpClientConfig {
    /// FRP server address (domain or IP)
    pub server_addr: String,
    
    /// FRP server port
    pub server_port: u16,
    
    /// Authentication token
    pub auth_token: String,
    
    /// Gateway name (used as subdomain or proxy name)
    pub gateway_name: String,
    
    /// TLS configuration
    pub tls: TlsConfig,
    
    /// Heartbeat interval
    pub heartbeat_interval_secs: u64,
    
    /// Reconnection settings
    pub reconnect: ReconnectConfig,
}

/// Local network configuration
pub struct LocalNetworkConfig {
    /// mDNS discovery enabled
    pub mdns_enabled: bool,
    
    /// Discovery interval
    pub discovery_interval_secs: u64,
    
    /// Manual device list (if mDNS unavailable)
    pub manual_devices: Vec<ManualDeviceConfig>,
    
    /// Network interface to bind
    pub interface: Option<String>,
}

/// API server configuration (local endpoint)
pub struct ApiServerConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    
    /// TLS configuration (optional, for HTTPS)
    pub tls: Option<TlsConfig>,
    
    /// Authentication (API keys, JWT, etc.)
    pub auth: AuthConfig,
    
    /// CORS settings
    pub cors: CorsConfig,
}
```

### Gateway Server Features

**Core Capabilities:**

1. **Device Discovery & Management**
   - mDNS discovery of ESPHome devices
   - Manual device registration
   - Device health monitoring
   - Entity registry management

2. **ESPHome Native API Proxy**
   - Transparent proxy for ESPHome protocol
   - State subscription multiplexing
   - Command forwarding
   - Connection state management

3. **FRP Integration**
   - Automatic frpc process management
   - Connection monitoring and reconnection
   - Multiple tunnel support
   - Traffic statistics

4. **Local API Server**
   - REST/WebSocket API for direct local access
   - Authentication and authorization
   - Rate limiting
   - Request logging

5. **State Management**
   - Persistent SQLite database
   - Device state caching
   - Event buffering during disconnect
   - Synchronization on reconnect

6. **Security**
   - TLS encryption for all remote connections
   - API key or JWT authentication
   - Certificate pinning
   - Audit logging

### Gateway Server Configuration

**Example Configuration File (`gateway-config.toml`):**

```toml
# Gateway Server Configuration

[gateway]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Home Gateway"

[local_network]
mdns_enabled = true
discovery_interval_secs = 60
interface = "eth0"

# Manual devices (optional, if mDNS unavailable)
[[local_network.manual_devices]]
name = "basement-sensor"
host = "192.168.1.100"
port = 6053

[[local_network.manual_devices]]
name = "garage-controller"
host = "192.168.1.101"
port = 6053

[frp]
server_addr = "frp.example.com"
server_port = 7000
auth_token = "your-secure-token-here"
gateway_name = "home-gateway"

[frp.tls]
enabled = true
cert_path = "/etc/gateway/cert.pem"
key_path = "/etc/gateway/key.pem"
ca_path = "/etc/gateway/ca.pem"

[frp.reconnect]
max_retries = 10
initial_delay_secs = 1
max_delay_secs = 60
backoff_multiplier = 2.0

[api_server]
listen_addr = "0.0.0.0:8080"

[api_server.tls]
enabled = true
cert_path = "/etc/gateway/api-cert.pem"
key_path = "/etc/gateway/api-key.pem"

[api_server.auth]
type = "api_key"
api_keys = [
    "sk_live_abc123...",
    "sk_live_def456..."
]

[security]
audit_log_path = "/var/log/gateway/audit.log"
max_request_size_mb = 10
rate_limit_requests_per_minute = 1000
```

### Desktop Application Integration

**UI Flow:**

1. **Gateway Selection Screen**

```
┌─────────────────────────────────────────────────────────────┐
│  ESPHome Automation Manager                                 │
├─────────────────────────────────────────────────────────────┤
│  Connected Gateways                                         │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 🏠 Home Gateway (Local)               ● Online        │ │
│  │    Discovered via mDNS                                 │ │
│  │    5 devices connected                                 │ │
│  │    [Connect] [Configure]                               │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 🏢 Office Gateway (Remote)            ● Online        │ │
│  │    Via frp.example.com                                 │ │
│  │    12 devices connected                                │ │
│  │    [Connect] [Configure]                               │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 🏭 Factory Floor (Remote)             ○ Offline       │ │
│  │    Last seen: 2 hours ago                              │ │
│  │    8 devices                                           │ │
│  │    [Retry Connection] [Configure]                      │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  [+ Add Gateway Manually]                                   │
└─────────────────────────────────────────────────────────────┘
```

2. **Gateway Configuration Dialog**

```
┌─────────────────────────────────────────────────────────────┐
│  Configure Gateway Connection                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Connection Type:                                           │
│  ○ Local Network (Auto-discover via mDNS)                   │
│  ● Remote via FRP Server                                    │
│                                                             │
│  FRP Server Settings:                                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Server Address: [frp.example.com              ]     │   │
│  │ Server Port:    [7000                         ]     │   │
│  │ Auth Token:     [•••••••••••••••              ]     │   │
│  │ Gateway Name:   [office-gateway              ]     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  TLS Settings:                                              │
│  ☑ Enable TLS                                               │
│  ☑ Verify Certificate                                       │
│                                                             │
│  [Test Connection]  [Save]  [Cancel]                        │
└─────────────────────────────────────────────────────────────┘
```

**Desktop App Rust Types:**

```rust
// apps/desktop/src-tauri/src/gateway/mod.rs

pub struct GatewayConnection {
    pub gateway_id: Uuid,
    pub gateway_name: String,
    pub connection_type: GatewayConnectionType,
    pub status: GatewayStatus,
    pub last_seen: DateTime<Utc>,
    pub device_count: usize,
}

pub enum GatewayConnectionType {
    /// Local network discovery via mDNS
    Local {
        discovered_at: DateTime<Utc>,
        host: String,
        port: u16,
    },
    
    /// Remote connection via FRP server
    Remote {
        frp_server: String,
        frp_port: u16,
        gateway_name: String,
        tls_enabled: bool,
    },
}

pub enum GatewayStatus {
    Online,
    Offline,
    Connecting,
    Error(String),
}

/// Gateway manager in desktop app
pub struct GatewayManager {
    local_discovery: LocalDiscoveryService,
    remote_connections: HashMap<Uuid, RemoteGatewayConnection>,
    gateway_clients: HashMap<Uuid, GatewayClient>,
}

impl GatewayManager {
    /// Discover local gateways via mDNS
    pub async fn discover_local_gateways(&self) -> Result<Vec<GatewayConnection>>;
    
    /// Connect to a remote gateway via FRP
    pub async fn connect_remote_gateway(
        &self,
        config: RemoteGatewayConfig,
    ) -> Result<GatewayConnection>;
    
    /// Get all connected gateways
    pub async fn list_gateways(&self) -> Vec<GatewayConnection>;
    
    /// Get devices for a specific gateway
    pub async fn get_gateway_devices(
        &self,
        gateway_id: Uuid,
    ) -> Result<Vec<Device>>;
    
    /// Execute command on device via gateway
    pub async fn execute_device_command(
        &self,
        gateway_id: Uuid,
        device_id: Uuid,
        command: EntityCommand,
    ) -> Result<CommandResult>;
}
```

### FRP Setup Guide

#### FRP Server (frps) Setup

**Install frps on public server:**

```bash
# Download FRP
wget https://github.com/fatedier/frp/releases/download/v0.52.0/frp_0.52.0_linux_amd64.tar.gz
tar -xzf frp_0.52.0_linux_amd64.tar.gz
cd frp_0.52.0_linux_amd64

# Configure frps.toml
cat > frps.toml << EOF
bindPort = 7000
auth.token = "your-secure-token-here"

# TLS configuration
transport.tls.certFile = "/etc/frp/server.crt"
transport.tls.keyFile = "/etc/frp/server.key"

# Dashboard (optional)
webServer.addr = "0.0.0.0"
webServer.port = 7500
webServer.user = "admin"
webServer.password = "secure-password"

# Logging
log.to = "/var/log/frps.log"
log.level = "info"
log.maxDays = 30
EOF

# Run frps
./frps -c frps.toml
```

**Systemd service (`/etc/systemd/system/frps.service`):**

```ini
[Unit]
Description=FRP Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/frps -c /etc/frp/frps.toml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

#### Gateway Server Setup

**Install on Raspberry Pi / Linux:**

```bash
# Download gateway server binary
wget https://github.com/your-org/esphome-automation-manager/releases/download/v1.0.0/gateway-server-linux-arm64
chmod +x gateway-server-linux-arm64
sudo mv gateway-server-linux-arm64 /usr/local/bin/gateway-server

# Create configuration
sudo mkdir -p /etc/gateway
sudo cat > /etc/gateway/gateway-config.toml << EOF
[gateway]
id = "$(uuidgen)"
name = "Home Gateway"

[local_network]
mdns_enabled = true
discovery_interval_secs = 60

[frp]
server_addr = "frp.example.com"
server_port = 7000
auth_token = "your-secure-token-here"
gateway_name = "home-gateway"

[api_server]
listen_addr = "0.0.0.0:8080"
EOF

# Create systemd service
sudo cat > /etc/systemd/system/gateway-server.service << EOF
[Unit]
Description=ESPHome Gateway Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/gateway-server --config /etc/gateway/gateway-config.toml
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable gateway-server
sudo systemctl start gateway-server
```

### Gateway Server API

**REST API Endpoints:**

```
GET    /api/v1/gateway/info              # Gateway information
GET    /api/v1/devices                   # List discovered devices
GET    /api/v1/devices/{id}              # Get device details
GET    /api/v1/devices/{id}/entities     # Get device entities
POST   /api/v1/devices/{id}/command      # Execute command
GET    /api/v1/entities/{id}/state       # Get entity state
POST   /api/v1/entities/{id}/command     # Execute entity command
GET    /api/v1/status                    # Gateway health status
GET    /api/v1/metrics                   # Prometheus metrics
```

**WebSocket API:**

```
ws://gateway:8080/api/v1/ws
  - Subscribe to device state changes
  - Real-time entity updates
  - Command status updates
  - Gateway events
```

### Security Considerations

1. **Transport Security**
   - All FRP connections use TLS
   - Certificate verification enabled
   - API server uses HTTPS

2. **Authentication**
   - FRP auth token required
   - API key or JWT for API access
   - Rate limiting enabled

3. **Authorization**
   - Gateway-level permissions
   - Device-level access control
   - Audit logging

4. **Network Security**
   - Firewall rules (only required ports open)
   - VPN option as alternative to FRP
   - IP whitelisting (optional)

### Alternative: VPN Mode

For users who prefer VPN over FRP:

```
Desktop App ←──VPN Tunnel──→ Gateway Server ←──Local──→ ESPHome Devices
```

**WireGuard Configuration Example:**

```toml
# /etc/wireguard/wg0.conf (on Gateway Server)
[Interface]
Address = 10.0.0.2/24
PrivateKey = <gateway-private-key>
ListenPort = 51820

[Peer]
PublicKey = <desktop-public-key>
AllowedIPs = 10.0.0.1/32
```

```toml
# WireGuard config on Desktop
[Interface]
Address = 10.0.0.1/24
PrivateKey = <desktop-private-key>

[Peer]
PublicKey = <gateway-public-key>
Endpoint = gateway.example.com:51820
AllowedIPs = 10.0.0.2/32, 192.168.1.0/24
```

### Gateway Server Deployment

**Docker Compose for Gateway Server:**

```yaml
version: '3.8'

services:
  gateway-server:
    image: esphome/eam-gateway:latest
    container_name: eam-gateway
    restart: unless-stopped
    network_mode: host
    volumes:
      - ./gateway-config.toml:/etc/gateway/config.toml:ro
      - gateway-data:/data
      - gateway-logs:/var/log/gateway
    environment:
      - RUST_LOG=info
      - EAM_CONFIG_PATH=/etc/gateway/config.toml
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

volumes:
  gateway-data:
  gateway-logs:
```

**Kubernetes Deployment (for advanced users):**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: eam-gateway
spec:
  replicas: 1
  selector:
    matchLabels:
      app: eam-gateway
  template:
    metadata:
      labels:
        app: eam-gateway
    spec:
      containers:
      - name: gateway
        image: esphome/eam-gateway:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: config
          mountPath: /etc/gateway
        - name: data
          mountPath: /data
      volumes:
      - name: config
        configMap:
          name: gateway-config
      - name: data
        persistentVolumeClaim:
          claimName: gateway-data-pvc
```

---

### Prerequisites

```yaml
Required Tools:
  - Rust: 1.75+ (stable toolchain)
  - Node.js: 20 LTS
  - pnpm: 8.15+
  - Git: 2.40+
  - SQLite: 3.40+
  - ESPHome CLI: Latest stable (for testing)
  
Windows-Specific:
  - Microsoft Visual Studio Build Tools 2022
  - WebView2 Runtime (comes with Windows 10/11)
  - Git Bash (already installed)

Optional:
  - Docker Desktop: For containerized development
  - VS Code: With rust-analyzer, Tauri extensions
```

### Repository Structure

```
esphome-automation-manager/
├── .cargo/
│   └── config.toml                    # Cargo configuration
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # Continuous integration
│   │   ├── release.yml                # Release automation
│   │   └── security-audit.yml         # Security scanning
│   └── ISSUE_TEMPLATE/
├── .kiro/
│   ├── hooks/                         # Agent hooks
│   ├── steering/                      # Project steering files
│   └── settings/
│       └── mcp.json                   # MCP configuration
├── .vscode/
│   ├── settings.json
│   ├── extensions.json
│   └── launch.json
├── apps/
│   ├── desktop/
│   │   ├── src/
│   │   │   ├── app/                   # App-wide providers, routing
│   │   │   ├── components/            # Shared UI components
│   │   │   ├── features/
│   │   │   │   ├── overview/
│   │   │   │   ├── devices/
│   │   │   │   ├── entities/
│   │   │   │   ├── gateways/          # Gateway management UI
│   │   │   │   ├── configuration/
│   │   │   │   ├── firmware/
│   │   │   │   ├── automations/
│   │   │   │   ├── dashboards/
│   │   │   │   ├── history/
│   │   │   │   └── settings/
│   │   │   ├── hooks/                 # React hooks for IPC
│   │   │   ├── lib/                   # Utilities, API clients
│   │   │   ├── stores/                # Zustand state management
│   │   │   ├── styles/                # Global styles, themes
│   │   │   └── types/                 # TypeScript types
│   │   ├── src-tauri/
│   │   │   ├── src/
│   │   │   │   ├── commands/          # Tauri IPC commands
│   │   │   │   ├── events/            # Event emission
│   │   │   │   ├── gateway/           # Gateway connection manager
│   │   │   │   ├── app/               # App state management
│   │   │   │   └── main.rs
│   │   │   ├── capabilities/
│   │   │   ├── icons/
│   │   │   ├── Cargo.toml
│   │   │   └── tauri.conf.json
│   │   ├── index.html
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── vite.config.ts
│   │   └── tailwind.config.js
│   │
│   └── gateway-server/                # Headless gateway server
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
├── crates/
│   ├── eam-domain/                    # Core domain types
│   ├── eam-application/               # Use cases, ports
│   ├── eam-runtime/                   # Lifecycle, task supervisor
│   ├── eam-storage/                   # SQLite repositories
│   ├── eam-device-manager/            # Device connection management
│   ├── eam-entity-registry/           # Entity state management
│   ├── eam-automation/                # Automation engine
│   ├── eam-history/                   # Historical data recording
│   ├── eam-dashboard/                 # Dashboard query service
│   ├── eam-config/                    # Configuration project service
│   ├── eam-toolchain/                 # ESPHome CLI integration
│   ├── eam-discovery/                 # mDNS discovery
│   ├── eam-security/                  # Credential management
│   ├── eam-observability/             # Logging, metrics
│   └── eam-esphome-api/               # Native API client
├── docs/
│   ├── architecture/
│   │   ├── PLAN.md
│   │   └── DESIGN.md
│   ├── decisions/                     # ADRs
│   ├── development/
│   └── user/
├── fixtures/
│   ├── esphome-api/                   # Mock devices for testing
│   ├── configurations/                # Sample YAML configs
│   └── automation/                    # Sample automations
├── schemas/
│   ├── automation/
│   │   └── v1.schema.json
│   ├── dashboard/
│   │   └── v1.schema.json
│   └── ipc/
│       └── v1.schema.json
├── scripts/
│   ├── build.ps1                      # Windows build script
│   ├── build.sh                       # Unix build script
│   ├── dev-setup.ps1                  # Development setup
│   ├── test-coverage.ps1
│   └── release.ps1
├── tests/
│   ├── integration/
│   ├── e2e/
│   └── performance/
├── tools/
│   └── fixture-server/                # Mock ESPHome device server
├── .dockerignore
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── Makefile
├── package.json
├── pnpm-lock.yaml
├── pnpm-workspace.yaml
├── README.md
├── AGENTS.md
├── SKILL.md
└── PLAN.md                            # This file
```

### Docker Environment

**Dockerfile** - Development container:
```dockerfile
FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install Node.js and pnpm
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g pnpm@8.15.0

# Install additional tools
RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libssl-dev \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Install ESPHome CLI (optional, for testing)
RUN pip3 install esphome

WORKDIR /workspace
```

**docker-compose.yml** - Development services:
```yaml
version: '3.8'

services:
  dev:
    build:
      context: .
      dockerfile: Dockerfile
    volumes:
      - ..:/workspace:cached
      - cargo-registry:/usr/local/cargo/registry
      - pnpm-store:/root/.local/share/pnpm/store
    command: sleep infinity
    environment:
      - DISPLAY=${DISPLAY}
    network_mode: host

  # Mock ESPHome device fixture for testing
  fixture-device:
    image: python:3.11-slim
    volumes:
      - ../tools/fixture-server:/fixture
    command: python /fixture/mock_device.py
    ports:
      - "6053:6053"
    environment:
      - ESPHOME_API_PORT=6053

volumes:
  cargo-registry:
  pnpm-store:
```

### Makefile

```makefile
# ESPHome Automation Manager - Development Automation

.PHONY: all setup build test clean dev lint format check help

# Default target
all: setup build

# Help target
help:
	@echo "ESPHome Automation Manager - Available Commands:"
	@echo ""
	@echo "  setup        - Initialize development environment"
	@echo "  build        - Build all components"
	@echo "  dev          - Start development server"
	@echo "  test         - Run all tests"
	@echo "  test-unit    - Run unit tests only"
	@echo "  test-int     - Run integration tests"
	@echo "  test-e2e     - Run end-to-end tests"
	@echo "  lint         - Run linters"
	@echo "  format       - Format code"
	@echo "  check        - Run all checks (format + lint + test)"
	@echo "  clean        - Clean build artifacts"
	@echo "  release      - Build release binaries"
	@echo "  docker-up    - Start Docker development environment"
	@echo "  docker-down  - Stop Docker environment"
	@echo ""

# Setup
setup:
	@echo "Setting up development environment..."
	pnpm install
	cargo fetch
	@echo "Setup complete!"

# Build
build: build-frontend build-rust
	@echo "Build complete!"

build-frontend:
	pnpm run build

build-rust:
	cargo build --workspace

build-release:
	cargo build --release

# Development
dev:
	pnpm run dev

# Testing
test: test-rust test-frontend
	@echo "All tests passed!"

test-rust:
	cargo test --workspace --all-features

test-rust-coverage:
	cargo tarpaulin --workspace --out Html

test-frontend:
	pnpm run test

test-unit:
	cargo test --workspace --lib
	pnpm run test:unit

test-int:
	cargo test --workspace --test '*'
	pnpm run test:integration

test-e2e:
	pnpm run test:e2e

# Code Quality
lint: lint-rust lint-frontend
	@echo "Linting complete!"

lint-rust:
	cargo clippy --workspace --all-targets -- -D warnings

lint-frontend:
	pnpm run lint

format: format-rust format-frontend
	@echo "Formatting complete!"

format-rust:
	cargo fmt --all

format-frontend:
	pnpm run format

check: format lint test
	@echo "All checks passed!"

# Clean
clean:
	cargo clean
	pnpm run clean
	rm -rf dist/
	rm -rf target/
	@echo "Cleaned build artifacts"

# Release
release: build-release
	@echo "Building release binaries..."
	cargo tauri build

# Docker
docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-logs:
	docker-compose logs -f

# Database
db-migrate:
	cargo run --bin eam-migrate

db-reset:
	rm -rf ~/.eam/data/*.db
	cargo run --bin eam-migrate

# Fixtures
fixtures-start:
	python tools/fixture-server/mock_device.py &

fixtures-stop:
	pkill -f mock_device.py

# Performance testing
perf-test:
	cargo test --workspace --release -- --nocapture performance

# Security audit
audit:
	cargo audit
	pnpm audit

# Generate documentation
docs:
	cargo doc --workspace --no-deps
	pnpm run docs

# Development utilities
watch-rust:
	cargo watch -x test

watch-frontend:
	pnpm run dev

# Platform-specific builds
build-windows:
	cargo tauri build --target x86_64-pc-windows-msvc

build-linux:
	cargo tauri build --target x86_64-unknown-linux-gnu

build-macos:
	cargo tauri build --target x86_64-apple-darwin
	cargo tauri build --target aarch64-apple-darwin
```

---

## Phase 0: Technical Validation Spike

**Duration**: 2-3 weeks
**Goal**: Validate highest-risk integrations before committing to architecture

### Objectives
1. Prove ESPHome Native API connectivity and protocol compatibility
2. Validate Tauri 2 + React integration
3. Test ESPHome CLI toolchain integration
4. Establish repository structure and development workflow

### Tasks

#### 0.1 Repository Initialization

```bash
# Create workspace structure
- Initialize Cargo workspace with all crates
- Initialize pnpm workspace
- Configure .gitignore for Rust, Node, Tauri
- Set up VS Code workspace settings
- Create initial README.md
```

**Deliverables:**
- [x] Cargo workspace configured
- [x] pnpm workspace configured
- [ ] VS Code settings and extensions
- [ ] CI/CD pipeline skeleton (GitHub Actions)

#### 0.2 Minimal Tauri Application

**Frontend (apps/desktop/src/):**
```typescript
// App.tsx - Minimal app with IPC test
- Device list placeholder
- "Ping" button to test Tauri command
- State display for runtime status
```

**Tauri Commands (apps/desktop/src-tauri/src/):**
```rust
// commands/mod.rs
pub fn ping() -> Result<String, String>
pub fn get_runtime_status() -> Result<RuntimeStatus, String>
pub fn get_workspace_info() -> Result<WorkspaceInfo, String>
```

**Acceptance Criteria:**
- [ ] Tauri app launches successfully
- [ ] IPC commands work from React to Rust
- [ ] Basic state management in React
- [ ] Hot reload works for frontend changes

#### 0.3 Domain Crate Skeleton

**eam-domain crate:**
```rust
// Core types without external dependencies
pub struct Device { ... }
pub struct Entity { ... }
pub struct EntityValue { ... }
pub enum DeviceConnectionStatus { ... }
pub enum EntityType { ... }
```

**Acceptance Criteria:**
- [ ] Domain types compile
- [ ] No Tauri, SQLite, or protocol dependencies
- [ ] Unit tests for value objects

#### 0.4 ESPHome Native API Spike

**eam-esphome-api crate:**
```rust
// Protocol client implementation
pub struct EspHomeClient { ... }

impl EspHomeClient {
    pub async fn connect(host: &str, port: u16, password: Option<&str>) -> Result<Self>;
    pub async fn get_device_info(&self) -> Result<DeviceInfo>;
    pub async fn list_entities(&self) -> Result<Vec<EntityInfo>>;
    pub async fn subscribe_states(&self) -> Result<impl Stream<Item = EntityState>>;
    pub async fn send_command(&self, entity_id: u32, command: Command) -> Result<()>;
}
```

**Test Suite:**
- Connection handshake with encryption
- Device info retrieval
- Entity list discovery (sensors, switches, numbers)
- State subscription
- Switch on/off command
- Number set command
- Connection loss and reconnect

**Acceptance Criteria:**
- [ ] Connect to at least 2 real ESPHome devices or fixtures
- [ ] Retrieve entity list
- [ ] Receive live state updates
- [ ] Execute switch and number commands
- [ ] Recover from forced disconnect
- [ ] Test against 2+ firmware versions

#### 0.5 Toolchain Integration Spike

**eam-toolchain crate:**
```rust
pub struct ToolchainAdapter { ... }

impl ToolchainAdapter {
    pub fn detect() -> Result<ToolchainInfo>;
    pub async fn validate_project(&self, path: &Path) -> Result<ValidationResult>;
    pub async fn build(&self, request: BuildRequest) -> Result<BuildResult>;
    pub async fn flash_serial(&self, request: FlashRequest) -> Result<FlashResult>;
    pub async fn deploy_ota(&self, request: OtaRequest) -> Result<OtaResult>;
}
```

**Test Suite:**
- CLI detection on Windows, Linux, macOS
- Project validation
- Build invocation with log capture
- OTA deployment (simulated or real device)

**Acceptance Criteria:**
- [ ] Detect ESPHome CLI installation
- [ ] Validate a sample project
- [ ] Build firmware with captured output
- [ ] OTA deploy to test device (or mock)

#### 0.6 Architecture Decision Records

Create in `docs/decisions/`:
- `ADR-001-esphome-api-client-strategy.md` - Native Rust vs Python bridge
- `ADR-002-repository-structure.md` - Monorepo layout
- `ADR-003-database-design.md` - SQLite schema decisions
- `ADR-004-ipc-contract-versioning.md` - Command/event versioning

**Acceptance Criteria:**
- [ ] ADRs published for key decisions
- [ ] Protocol compatibility matrix documented

### Phase 0 Test Plan

```yaml
Unit Tests:
  - Domain type construction and validation
  - Entity value parsing
  - Command serialization

Integration Tests:
  - ESPHome client connection (real or fixture)
  - Entity discovery
  - State subscription
  - Command execution
  - Toolchain detection and build

Manual Tests:
  - Launch Tauri app
  - Connect to real ESPHome device
  - View entities in UI
  - Toggle switch, verify state change
  - Disconnect and reconnect
```

### Phase 0 Completion Criteria

- [ ] Repository structure complete
- [ ] Minimal Tauri app runs
- [ ] ESPHome API client works with real devices
- [ ] Toolchain adapter validates and builds
- [ ] ADRs published
- [ ] CI/CD pipeline runs tests

---

## Phase 1: Application Foundation & Device Manager

**Duration**: 4-5 weeks
**Goal**: Build core application lifecycle and device management

### Objectives
1. Implement persistent Rust runtime
2. Create SQLite infrastructure with migrations
3. Build device discovery and connection management
4. Implement entity registry and state management

### Tasks

#### 1.1 Application Lifecycle (eam-runtime crate)

```rust
pub struct RuntimeSupervisor {
    shutdown_token: CancellationToken,
    task_handles: Vec<JoinHandle<()>>,
    event_bus: EventBus,
}

impl RuntimeSupervisor {
    pub async fn start(config: RuntimeConfig) -> Result<Self>;
    pub async fn shutdown(&self) -> Result<()>;
    pub fn health_check(&self) -> RuntimeHealth;
}
```

**Components:**
- Task supervisor for background services
- Graceful shutdown with timeout
- Health monitoring
- Database flush on exit

**Tests:**
- Runtime starts and stops cleanly
- Background tasks are tracked
- Shutdown flushes pending data

#### 1.2 SQLite Infrastructure (eam-storage crate)

**Migration System:**
```rust
pub struct MigrationRunner { ... }

impl MigrationRunner {
    pub async fn run_migrations(&self) -> Result<()>;
    pub async fn get_version(&self) -> Result<u32>;
}
```

**Schema Implementation:**
- All tables from spec
- Proper indexes
- Foreign key constraints
- JSON columns for flexibility

**Repository Pattern:**
```rust
pub trait DeviceRepository {
    async fn create(&self, device: &Device) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Device>>;
    async fn list(&self) -> Result<Vec<Device>>;
    async fn update(&self, device: &Device) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}
```

**Tests:**
- Migration from clean state
- Idempotent migrations
- CRUD operations for all repositories
- Transaction handling

#### 1.3 Device Discovery (eam-discovery crate)

```rust
pub struct DiscoveryService { ... }

impl DiscoveryService {
    pub async fn start_mdns_discovery(&self) -> Result<impl Stream<Item = DiscoveredDevice>>;
    pub async fn register_manual(&self, host: &str, port: u16) -> Result<Device>;
}
```

**Features:**
- mDNS/MDNS service discovery
- Manual device registration
- Device fingerprinting (node_name, MAC)

**Tests:**
- Discover real or mock devices
- Manual registration
- Duplicate detection

#### 1.4 Connection Manager (eam-device-manager crate)

```rust
pub struct ConnectionManager { ... }

impl ConnectionManager {
    pub async fn connect(&self, device_id: Uuid) -> Result<()>;
    pub async fn disconnect(&self, device_id: Uuid) -> Result<()>;
    pub async fn get_connection_status(&self, device_id: Uuid) -> DeviceConnectionStatus;
    pub async fn reconnect_all(&self) -> Result<()>;
}
```

**Features:**
- Exponential backoff with jitter
- Connection state tracking
- Automatic reconnect on disconnect
- Credential management via OS store

**Tests:**
- Successful connection
- Failed authentication
- Network interruption
- Reconnection behavior

#### 1.5 Entity Registry (eam-entity-registry crate)

```rust
pub struct EntityRegistry { ... }

impl EntityRegistry {
    pub async fn refresh_entities(&self, device_id: Uuid) -> Result<Vec<Entity>>;
    pub async fn get_entity(&self, entity_id: Uuid) -> Result<Option<Entity>>;
    pub async fn list_by_device(&self, device_id: Uuid) -> Result<Vec<Entity>>;
}
```

**Features:**
- Entity discovery from device
- Normalization of entity types
- Capability tracking
- Metadata storage

**Tests:**
- Entity discovery
- Type normalization
- Capability extraction

#### 1.6 Entity State Store (eam-entity-registry crate)

```rust
pub struct StateStore { ... }

impl StateStore {
    pub async fn update_state(&self, entity_id: Uuid, state: EntityState);
    pub async fn get_state(&self, entity_id: Uuid) -> Option<EntityState>;
    pub async fn subscribe(&self) -> impl Stream<Item = StateUpdate>;
}
```

**Features:**
- In-memory current state
- State quality tracking
- Event emission on change
- Coalesced updates

**Tests:**
- State updates
- Subscription delivery
- Quality tracking

#### 1.7 Command Service (eam-entity-registry crate)

```rust
pub struct CommandService { ... }

impl CommandService {
    pub async fn execute(&self, command: EntityCommand) -> Result<CommandResult>;
}
```

**Features:**
- Command validation
- Queue management
- Timeout handling
- Result tracking

**Tests:**
- Valid command execution
- Validation failures
- Timeout behavior
- Device unavailable handling

#### 1.8 Frontend: Device Fleet UI

**Components:**
```typescript
// features/devices/
- DeviceList.tsx          // Grid/list view of devices
- DeviceCard.tsx          // Single device summary
- DeviceDetail.tsx        // Full device view
- DeviceDiscovery.tsx     // Discovery modal
- DeviceRegistration.tsx  // Manual add form
- EntityList.tsx          // Device entities
- EntityControl.tsx       // Generic control component
```

**State Management:**
```typescript
// stores/deviceStore.ts
interface DeviceStore {
  devices: Map<string, Device>;
  connectionStatus: Map<string, ConnectionStatus>;
  selectedDevice: string | null;
  
  fetchDevices: () => Promise<void>;
  connectDevice: (id: string) => Promise<void>;
  disconnectDevice: (id: string) => Promise<void>;
}

// stores/entityStore.ts
interface EntityStore {
  entities: Map<string, Entity>;
  states: Map<string, EntityState>;
  
  subscribeToStates: () => void;
  sendCommand: (entityId: string, command: any) => Promise<void>;
}
```

**Tests:**
- Component rendering
- State updates
- IPC integration
- Error handling

#### 1.9 Frontend: Entity Controls

**Components:**
```typescript
// features/entities/controls/
- SwitchControl.tsx
- NumberControl.tsx
- SensorDisplay.tsx
- BinarySensorDisplay.tsx
```

**Tests:**
- Control rendering for each type
- Command execution
- State updates

### Phase 1 Test Plan

```yaml
Unit Tests:
  - Domain model validation
  - Repository CRUD operations
  - State store logic
  - Command validation

Integration Tests:
  - Device connection lifecycle
  - Entity discovery
  - State subscription
  - Command execution
  - Database persistence

E2E Tests:
  - Launch app
  - Discover device
  - Connect to device
  - View entities
  - Control switch/number
  - Disconnect
  - Reconnect after restart

Performance Tests:
  - Connect to 10 devices concurrently
  - Handle 100 state updates/second
  - UI remains responsive during ingestion
```

### Phase 1 Completion Criteria

- [ ] Application starts with persistent runtime
- [ ] SQLite migrations run successfully
- [ ] mDNS discovery finds devices
- [ ] Manual device registration works
- [ ] Devices connect via ESPHome API
- [ ] Entities appear in UI dynamically
- [ ] Switch and number controls work
- [ ] Reconnection after network interruption
- [ ] State persists across restarts
- [ ] All tests passing

---

## Phase 2: Historical Data & Entity Controls

**Duration**: 3-4 weeks
**Goal**: Add historical data recording and comprehensive entity support

### Tasks

#### 2.1 Data Recording Service (eam-history crate)

```rust
pub struct RecorderService { ... }

impl RecorderService {
    pub async fn record_numeric(&self, reading: NumericReading) -> Result<()>;
    pub async fn record_event(&self, event: EntityEvent) -> Result<()>;
    pub async fn set_policy(&self, entity_id: Uuid, policy: RecordingPolicy) -> Result<()>;
}
```

**Features:**
- Bounded ingestion queue
- Batch inserts
- Configurable policies per entity
- Deadband/minimum change threshold

**Tests:**
- Recording numeric values
- Recording state changes
- Policy application
- Queue overflow handling

#### 2.2 Historical Query Service (eam-history crate)

```rust
pub struct HistoryQueryService { ... }

impl HistoryQueryService {
    pub async fn query_series(&self, query: SeriesQuery) -> Result<Vec<NumericReading>>;
    pub async fn query_events(&self, query: EventQuery) -> Result<Vec<EntityEvent>>;
    pub async fn query_latest(&self, entity_id: Uuid) -> Result<Option<NumericReading>>;
    pub async fn export_csv(&self, query: ExportQuery) -> Result<Vec<u8>>;
}
```

**Tests:**
- Time range queries
- Aggregation queries
- CSV export

#### 2.3 Extended Entity Support

**Additional entity types:**
- Lights (brightness, color, effects)
- Fans (speed, oscillation)
- Climate (temperature, mode, fan)
- Covers (position, tilt)
- Locks (lock/unlock)
- Selects (options)
- Buttons (press)
- Text entities

**Tests:**
- Each entity type rendering
- Command execution
- Capability detection

#### 2.4 Frontend: History UI

**Components:**
```typescript
// features/history/
- HistoryScreen.tsx
- TimeRangeSelector.tsx
- ChartContainer.tsx
- EventTimeline.tsx
- ExportDialog.tsx
```

**Tests:**
- Chart rendering
- Time range selection
- Export functionality

### Phase 2 Test Plan

```yaml
Integration Tests:
  - Record 1000 numeric values
  - Query time ranges
  - Export CSV
  - High-rate ingestion
  - Database cleanup

Performance Tests:
  - 100 updates/second sustained
  - Query 7 days of data
  - Memory bounded during ingestion
```

### Phase 2 Completion Criteria

- [ ] Numeric entities can be recorded
- [ ] History persists after restart
- [ ] Time-range queries work
- [ ] Charts display historical data
- [ ] CSV export works
- [ ] All entity families supported
- [ ] High-rate ingestion doesn't crash
- [ ] Memory usage bounded

---

## Phase 3: Dashboard System

**Duration**: 3-4 weeks

### Tasks

#### 3.1 Dashboard Schema & Storage

```rust
pub struct DashboardRepository { ... }

impl DashboardRepository {
    async fn create(&self, dashboard: &Dashboard) -> Result<()>;
    async fn get(&self, id: Uuid) -> Result<Option<Dashboard>>;
    async fn list(&self) -> Result<Vec<Dashboard>>;
    async fn update(&self, dashboard: &Dashboard) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}
```

#### 3.2 Widget Runtime

```rust
pub struct WidgetRuntime { ... }

impl WidgetRuntime {
    pub async fn render(&self, widget: &Widget) -> WidgetRender;
    pub async fn subscribe(&self, widget_id: Uuid) -> impl Stream<Item = WidgetUpdate>;
}
```

#### 3.3 Widget Implementations

**Widget Types:**
- Numeric value card
- Sensor value with trend
- Line chart
- Multi-series chart
- Gauge
- Progress bar
- Binary status
- Switch control
- Light control
- Number slider
- Select control
- Button action
- Fan control
- Device health
- Event timeline
- Markdown panel

#### 3.4 Frontend: Dashboard Builder

**Components:**
```typescript
// features/dashboards/
- DashboardList.tsx
- DashboardCanvas.tsx
- WidgetPalette.tsx
- WidgetRenderer.tsx
- WidgetConfigPanel.tsx
- GridLayout.tsx
```

**Features:**
- Drag-and-drop widgets
- Grid layout system
- Widget configuration
- Live preview
- Import/export

### Phase 3 Test Plan

```yaml
Unit Tests:
  - Dashboard CRUD
  - Widget validation
  - Layout calculations

Integration Tests:
  - Create dashboard with widgets
  - Persist and reload
  - Live updates
  - Import/export

E2E Tests:
  - Create dashboard
  - Add 4+ widget types
  - Configure widgets
  - Verify live updates
```

### Phase 3 Completion Criteria

- [ ] Create/edit/delete dashboards
- [ ] Add/configure/remove widgets
- [ ] Widget positions persist
- [ ] Live values update
- [ ] Historical charts work
- [ ] Missing entities handled gracefully
- [ ] Multi-page support
- [ ] Import/export dashboards

---

## Phase 4: Automation Runtime

**Duration**: 4-5 weeks

### Tasks

#### 4.1 Automation Schema & Validation

```rust
pub struct AutomationValidator { ... }

impl AutomationValidator {
    pub fn validate(&self, definition: &AutomationDefinition) -> Result<ValidatedAutomation>;
}
```

#### 4.2 Trigger System

```rust
pub struct TriggerManager { ... }

impl TriggerManager {
    pub async fn register(&self, trigger: Trigger) -> Result<()>;
    pub async fn unregister(&self, trigger_id: Uuid) -> Result<()>;
    pub async fn fire(&self, event: TriggerEvent) -> Result<()>;
}
```

**Trigger Types:**
- Entity state changed
- Numeric threshold
- Device online/offline
- Schedule
- Manual

#### 4.3 Condition Evaluator

```rust
pub struct ConditionEvaluator { ... }

impl ConditionEvaluator {
    pub async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool>;
}
```

**Condition Types:**
- Entity state comparison
- Numeric comparison
- Device availability
- Time/day
- AND/OR/NOT groups
- Variable comparison

#### 4.4 Action Executor

```rust
pub struct ActionExecutor { ... }

impl ActionExecutor {
    pub async fn execute(&self, action: &Action, context: &ExecutionContext) -> Result<ActionResult>;
}
```

**Action Types:**
- Set entity state
- Delay
- Sequence
- Parallel
- Conditional
- Log message
- Run automation

#### 4.5 Execution Engine

```rust
pub struct AutomationEngine { ... }

impl AutomationEngine {
    pub async fn start(&self) -> Result<()>;
    pub async fn stop(&self) -> Result<()>;
    pub async fn run_automation(&self, automation_id: Uuid, trigger: TriggerEvent) -> Result<RunResult>;
    pub async fn cancel_run(&self, run_id: Uuid) -> Result<()>;
}
```

**Features:**
- Overlap policies
- Retry with backoff
- Timeout handling
- Execution tracing
- Dry-run mode

#### 4.6 Frontend: Automation UI

**Components:**
```typescript
// features/automations/
- AutomationList.tsx
- AutomationBuilder.tsx
- RuleBuilder.tsx
- TriggerConfig.tsx
- ConditionConfig.tsx
- ActionConfig.tsx
- ExecutionHistory.tsx
- RunTrace.tsx
```

### Phase 4 Test Plan

```yaml
Unit Tests:
  - Trigger matching
  - Condition evaluation
  - Action execution
  - Overlap policies

Integration Tests:
  - End-to-end automation run
  - Cross-device action
  - Failure handling
  - Device disconnect

Performance Tests:
  - 100 concurrent automations
  - High-frequency triggers
```

### Phase 4 Completion Criteria

- [ ] Create/edit/delete automations
- [ ] Triggers fire correctly
- [ ] Conditions evaluated properly
- [ ] Actions execute with timeout
- [ ] Cross-device automations work
- [ ] Execution history recorded
- [ ] Traces available for debugging
- [ ] Automations run independently of UI

---

## Phase 5: Automation Visual Editor

**Duration**: 3-4 weeks

### Tasks

#### 5.1 Node-Based Graph Editor

**Components:**
- Graph canvas (React Flow or similar)
- Node palette
- Edge connections
- Viewport management
- Undo/redo

#### 5.2 Node Types

**Trigger Nodes:**
- State change
- Numeric threshold
- Schedule
- Device event

**Condition Nodes:**
- Comparison
- Logic gates
- Availability check

**Action Nodes:**
- Entity command
- Delay
- Flow control
- Log

#### 5.3 Graph Compilation

```typescript
function compileGraph(nodes: Node[], edges: Edge[]): AutomationDefinition {
  // Convert visual graph to canonical schema
}
```

#### 5.4 Validation & Errors

- Real-time validation
- Error highlighting
- Compilation errors
- Runtime errors

### Phase 5 Test Plan

```yaml
Unit Tests:
  - Graph compilation
  - Validation rules
  - Serialization

Integration Tests:
  - Create automation via graph
  - Execute compiled automation
  - Import/export graphs

E2E Tests:
  - Build automation visually
  - Verify execution matches design
```

### Phase 5 Completion Criteria

- [ ] Visual graph editor works
- [ ] All node types available
- [ ] Graph compiles to canonical schema
- [ ] Invalid graphs show errors
- [ ] Graphs persist and reload
- [ ] Execution traces map to nodes

---

## Phase 6: Configuration & Firmware Management

**Duration**: 4-5 weeks

### Tasks

#### 6.1 Project Management

```rust
pub struct ProjectService { ... }

impl ProjectService {
    pub async fn create(&self, request: CreateProjectRequest) -> Result<Project>;
    pub async fn open(&self, path: &Path) -> Result<Project>;
    pub async fn list(&self) -> Result<Vec<Project>>;
    pub async fn associate_device(&self, project_id: Uuid, device_id: Uuid) -> Result<()>;
}
```

#### 6.2 YAML Editor Integration

**Features:**
- Monaco editor or CodeMirror
- Syntax highlighting
- Linting integration
- Go to definition
- Autocomplete (basic)

#### 6.3 Configuration Validation

```rust
pub struct ConfigValidator { ... }

impl ConfigValidator {
    pub async fn validate(&self, project: &Project) -> Result<ValidationResult>;
    pub async fn check_secrets(&self, project: &Project) -> Result<SecretsStatus>;
}
```

#### 6.4 Visual Configuration Editor

**Features:**
- Form-based component editing
- Safe YAML preservation
- Diff preview before save
- Revision history

#### 6.5 Build System Integration

```rust
pub struct BuildService { ... }

impl BuildService {
    pub async fn build(&self, request: BuildRequest) -> Result<BuildResult>;
    pub async fn cancel(&self, build_id: Uuid) -> Result<()>;
    pub async fn get_status(&self, build_id: Uuid) -> Result<BuildStatus>;
}
```

#### 6.6 OTA Deployment

```rust
pub struct DeploymentService { ... }

impl DeploymentService {
    pub async fn deploy_ota(&self, request: OtaRequest) -> Result<DeploymentResult>;
    pub async fn flash_serial(&self, request: SerialRequest) -> Result<FlashResult>;
    pub async fn get_history(&self, device_id: Uuid) -> Result<Vec<Deployment>>;
}
```

#### 6.7 Frontend: Configuration UI

**Components:**
```typescript
// features/configuration/
- ProjectList.tsx
- ProjectExplorer.tsx
- YamlEditor.tsx
- VisualEditor.tsx
- ValidationPanel.tsx
- BuildProgress.tsx
- DeploymentWizard.tsx
```

### Phase 6 Test Plan

```yaml
Integration Tests:
  - Create/open projects
  - YAML validation
  - Build execution
  - OTA deployment
  - Serial flashing

E2E Tests:
  - Open existing project
  - Edit YAML
  - Validate
  - Build
  - Deploy OTA
```

### Phase 6 Completion Criteria

- [ ] Create/open ESPHome projects
- [ ] YAML editor with highlighting
- [ ] Validation shows errors
- [ ] Build produces artifacts
- [ ] OTA deploys to devices
- [ ] Serial flashing works
- [ ] Unknown YAML preserved
- [ ] Revision history available

---

## Phase 7: Fleet Operations & Hardening

**Duration**: 4-5 weeks

### Tasks

#### 7.1 Fleet Organization

**Features:**
- Areas (rooms, zones)
- Groups (user-defined)
- Tags (labels)
- Bulk operations

#### 7.2 Staged Rollout

```rust
pub struct RolloutService { ... }

impl RolloutService {
    pub async fn create_rollout(&self, request: RolloutRequest) -> Result<Rollout>;
    pub async fn pause_rollout(&self, rollout_id: Uuid) -> Result<()>;
    pub async fn get_progress(&self, rollout_id: Uuid) -> Result<RolloutProgress>;
}
```

#### 7.3 Backup & Restore

```rust
pub struct BackupService { ... }

impl BackupService {
    pub async fn create_backup(&self) -> Result<Backup>;
    pub async fn restore_backup(&self, backup: Backup) -> Result<()>;
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>>;
}
```

#### 7.4 Diagnostics Export

```rust
pub struct DiagnosticsService { ... }

impl DiagnosticsService {
    pub async fn export(&self) -> Result<DiagnosticsBundle>;
}
```

#### 7.5 Security Hardening

**Features:**
- Permission hardening
- CSP implementation
- Secret management
- Input validation
- Safe defaults

#### 7.6 Performance Optimization

**Targets:**
- 50+ devices
- 2000+ entities
- 100 updates/second
- <100MB memory baseline
- <500ms UI response

#### 7.7 Cross-Platform Packaging

**Platforms:**
- Windows (MSI, NSIS)
- Linux (AppImage, deb, rpm)
- macOS (dmg, app)

#### 7.8 Documentation

**User Documentation:**
- Getting started
- Device management
- Dashboards
- Automations
- Configuration
- Troubleshooting

**Developer Documentation:**
- Architecture
- Contributing
- Testing
- Releasing

### Phase 7 Test Plan

```yaml
Performance Tests:
  - 50 devices connected
  - 2000 entities registered
  - 100 updates/second sustained
  - Memory profiling
  - CPU profiling

Security Tests:
  - Secret storage
  - Input validation
  - Permission checks
  - CSP verification

Platform Tests:
  - Windows build and run
  - Linux build and run
  - macOS build and run

Reliability Tests:
  - Network interruption
  - Device failure
  - Database corruption
  - Long-running stability
```

### Phase 7 Completion Criteria

- [ ] Fleet organization works
- [ ] Bulk operations available
- [ ] Backup/restore functional
- [ ] Diagnostics export works
- [ ] Security requirements met
- [ ] Performance targets achieved
- [ ] All platforms build
- [ ] Documentation complete

---

## Phase 8: Gateway Server & Remote Fleet Management

**Duration**: 5-6 weeks
**Goal**: Implement Gateway Server for remote fleet access via FRP

### Objectives
1. Build headless Gateway Server for remote network sites
2. Implement FRP client integration for secure tunneling
3. Create Gateway discovery and connection management in Desktop app
4. Support both FRP and VPN connection modes
5. Provide hierarchical fleet view: Gateways → Devices → Entities

### Tasks

#### 8.1 Gateway Server Core (apps/gateway-server/)

**Create Gateway Server Application:**

```rust
// apps/gateway-server/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    let config = GatewayConfig::load()?;
    let gateway = GatewayServer::new(config).await?;
    gateway.run().await
}

// apps/gateway-server/src/server/mod.rs
pub struct GatewayServer {
    config: GatewayConfig,
    device_manager: Arc<DeviceManager>,
    frp_client: Arc<FrpClient>,
    api_server: Arc<ApiServer>,
    event_bus: Arc<EventBus>,
}

impl GatewayServer {
    pub async fn new(config: GatewayConfig) -> Result<Self> {
        // Initialize SQLite database
        let db = Database::new(&config.database.path).await?;
        
        // Create device manager
        let device_manager = Arc::new(
            DeviceManager::new(config.local_network.clone(), db.clone())
                .await?
        );
        
        // Create FRP client
        let frp_client = Arc::new(
            FrpClient::new(config.frp.clone())?
        );
        
        // Create API server
        let api_server = Arc::new(
            ApiServer::new(config.api_server.clone(), device_manager.clone())?
        );
        
        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        
        Ok(Self {
            config,
            device_manager,
            frp_client,
            api_server,
            event_bus,
        })
    }
    
    pub async fn run(self) -> Result<()> {
        // Start device discovery
        self.device_manager.start_discovery().await?;
        
        // Start FRP client (connect to frps)
        self.frp_client.start().await?;
        
        // Start API server
        self.api_server.start().await?;
        
        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        
        // Graceful shutdown
        self.shutdown().await
    }
}
```

**Deliverables:**
- [ ] Gateway server binary
- [ ] Configuration file parser
- [ ] Multi-platform builds (Linux ARM64, AMD64)

#### 8.2 Device Discovery & Management

```rust
// apps/gateway-server/src/device/manager.rs

pub struct DeviceManager {
    config: LocalNetworkConfig,
    discovery: DiscoveryService,
    devices: Arc<RwLock<HashMap<Uuid, DeviceConnection>>>,
    db: Database,
}

impl DeviceManager {
    /// Start mDNS discovery
    pub async fn start_discovery(&self) -> Result<()> {
        let mut discovery = self.discovery.clone();
        
        tokio::spawn(async move {
            while let Some(device) = discovery.next().await {
                match self.connect_device(device).await {
                    Ok(conn) => {
                        self.devices.write().await.insert(device.id, conn);
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to device: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Connect to discovered device
    async fn connect_device(&self, discovered: DiscoveredDevice) -> Result<DeviceConnection> {
        let client = EspHomeClient::connect(
            &discovered.host,
            discovered.port,
            discovered.password.as_deref(),
        ).await?;
        
        let device_info = client.get_device_info().await?;
        let entities = client.list_entities().await?;
        
        // Subscribe to state updates
        let state_stream = client.subscribe_states().await?;
        
        Ok(DeviceConnection {
            id: discovered.id,
            info: device_info,
            entities,
            client,
            state_stream,
        })
    }
}
```

**Tests:**
- [ ] mDNS discovery finds devices
- [ ] Manual device registration
- [ ] Device connection establishment
- [ ] Entity discovery
- [ ] State subscription

#### 8.3 FRP Client Integration

```rust
// apps/gateway-server/src/frp/client.rs

pub struct FrpClient {
    config: FrpClientConfig,
    process: Option<Child>,
    status: Arc<RwLock<FrpStatus>>,
}

impl FrpClient {
    /// Start frpc process
    pub async fn start(&self) -> Result<()> {
        // Generate frpc config file
        let frpc_config = self.generate_config()?;
        frpc_config.write_to_file("/tmp/frpc.ini")?;
        
        // Start frpc process
        let mut child = Command::new("frpc")
            .arg("-c")
            .arg("/tmp/frpc.ini")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        // Monitor process
        self.monitor_process(child);
        
        Ok(())
    }
    
    /// Generate frpc configuration
    fn generate_config(&self) -> Result<FrpConfigFile> {
        Ok(FrpConfigFile {
            common: FrpCommonSection {
                server_addr: self.config.server_addr.clone(),
                server_port: self.config.server_port,
                auth_token: self.config.auth_token.clone(),
                tls: self.config.tls.clone(),
            },
            proxies: vec![
                FrpProxySection {
                    name: self.config.gateway_name.clone(),
                    type_: "tcp",
                    local_ip: "127.0.0.1",
                    local_port: self.config.api_server_port,
                    remote_port: None, // Auto-assign
                },
            ],
        })
    }
    
    /// Monitor frpc process health
    fn monitor_process(&self, mut child: Child) {
        tokio::spawn(async move {
            let status = child.wait().await;
            match status {
                Ok(status) => {
                    tracing::error!("frpc process exited: {}", status);
                    // Attempt reconnection
                }
                Err(e) => {
                    tracing::error!("frpc process error: {}", e);
                }
            }
        });
    }
    
    /// Get FRP connection status
    pub async fn get_status(&self) -> FrpStatus {
        self.status.read().await.clone()
    }
}
```

**Tests:**
- [ ] frpc process starts successfully
- [ ] Configuration file generated correctly
- [ ] Connection to frps established
- [ ] Reconnection on disconnect
- [ ] Process monitoring

#### 8.4 Gateway API Server

```rust
// apps/gateway-server/src/api/server.rs

pub struct ApiServer {
    config: ApiServerConfig,
    device_manager: Arc<DeviceManager>,
    router: Router,
}

impl ApiServer {
    pub fn new(
        config: ApiServerConfig,
        device_manager: Arc<DeviceManager>,
    ) -> Result<Self> {
        let router = Router::new()
            .route("/api/v1/gateway/info", get(Self::get_gateway_info))
            .route("/api/v1/devices", get(Self::list_devices))
            .route("/api/v1/devices/:id", get(Self::get_device))
            .route("/api/v1/devices/:id/entities", get(Self::list_entities))
            .route("/api/v1/devices/:id/command", post(Self::device_command))
            .route("/api/v1/entities/:id/state", get(Self::get_entity_state))
            .route("/api/v1/entities/:id/command", post(Self::entity_command))
            .route("/api/v1/status", get(Self::get_status))
            .route("/api/v1/ws", get(Self::websocket_handler))
            .layer(middleware::from_fn_with_state(
                config.clone(),
                Self::auth_middleware,
            ));
        
        Ok(Self {
            config,
            device_manager,
            router,
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        let addr = self.config.listen_addr;
        
        if let Some(tls) = &self.config.tls {
            // HTTPS server
            let config = RustlsConfig::from_pem_file(
                &tls.cert_path,
                &tls.key_path,
            ).await?;
            
            axum_server::bind_rustls(addr, config)
                .serve(self.router.clone().into_make_service())
                .await?;
        } else {
            // HTTP server
            axum_server::bind(addr)
                .serve(self.router.clone().into_make_service())
                .await?;
        }
        
        Ok(())
    }
    
    // API handlers
    async fn get_gateway_info(...) -> Result<Json<GatewayInfo>>;
    async fn list_devices(...) -> Result<Json<Vec<Device>>>;
    async fn get_device(...) -> Result<Json<Device>>;
    async fn list_entities(...) -> Result<Json<Vec<Entity>>>;
    async fn device_command(...) -> Result<Json<CommandResult>>;
    async fn get_entity_state(...) -> Result<Json<EntityState>>;
    async fn entity_command(...) -> Result<Json<CommandResult>>;
    async fn get_status(...) -> Result<Json<GatewayStatus>>;
    async fn websocket_handler(...) -> Result<()>;
}
```

**Tests:**
- [ ] REST API endpoints work
- [ ] WebSocket connections established
- [ ] Authentication enforced
- [ ] TLS configuration works
- [ ] Rate limiting functional

#### 8.5 Desktop App Gateway Integration

**Gateway Manager in Desktop App:**

```rust
// apps/desktop/src-tauri/src/gateway/manager.rs

pub struct GatewayManager {
    local_discovery: LocalDiscoveryService,
    remote_gateways: HashMap<Uuid, RemoteGatewayClient>,
    gateway_states: Arc<RwLock<HashMap<Uuid, GatewayState>>>,
}

impl GatewayManager {
    /// Discover local gateways via mDNS
    pub async fn discover_local_gateways(&self) -> Result<Vec<GatewayInfo>> {
        self.local_discovery.scan().await
    }
    
    /// Connect to remote gateway via FRP
    pub async fn connect_remote_gateway(
        &self,
        config: RemoteGatewayConfig,
    ) -> Result<GatewayConnection> {
        // Build connection URL
        let url = format!(
            "{}://{}:{}",
            if config.tls_enabled { "https" } else { "http" },
            config.frp_server,
            config.remote_port,
        );
        
        // Create HTTP client
        let client = reqwest::Client::new();
        
        // Test connection
        let info = client
            .get(&format!("{}/api/v1/gateway/info", url))
            .header("Authorization", format!("Bearer {}", config.api_key))
            .send()
            .await?
            .json::<GatewayInfo>()
            .await?;
        
        // Create WebSocket connection for real-time updates
        let ws_url = url.replace("http", "ws");
        let ws = WebSocket::connect(&format!("{}/api/v1/ws", ws_url)).await?;
        
        // Store connection
        let connection = GatewayConnection {
            gateway_id: info.id,
            client: client.clone(),
            websocket: ws,
        };
        
        self.remote_gateways.insert(info.id, connection);
        
        Ok(connection)
    }
    
    /// Get devices from gateway
    pub async fn get_gateway_devices(
        &self,
        gateway_id: Uuid,
    ) -> Result<Vec<Device>> {
        let client = self.get_gateway_client(gateway_id).await?;
        
        client
            .get(&format!("{}/api/v1/devices", client.base_url))
            .send()
            .await?
            .json()
            .await
    }
    
    /// Execute command on device via gateway
    pub async fn execute_device_command(
        &self,
        gateway_id: Uuid,
        device_id: Uuid,
        command: EntityCommand,
    ) -> Result<CommandResult> {
        let client = self.get_gateway_client(gateway_id).await?;
        
        client
            .post(&format!(
                "{}/api/v1/devices/{}/command",
                client.base_url, device_id
            ))
            .json(&command)
            .send()
            .await?
            .json()
            .await
    }
}
```

**Tauri IPC Commands:**

```rust
// apps/desktop/src-tauri/src/commands/gateway.rs

#[tauri::command]
pub async fn gateway_discover_local() -> Result<Vec<GatewayInfo>, String> {
    GATEWAY_MANAGER.discover_local_gateways().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gateway_connect_remote(
    config: RemoteGatewayConfig,
) -> Result<GatewayConnection, String> {
    GATEWAY_MANAGER.connect_remote_gateway(config).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gateway_list() -> Result<Vec<GatewayState>, String> {
    GATEWAY_MANAGER.list_gateways().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gateway_get_devices(
    gateway_id: String,
) -> Result<Vec<Device>, String> {
    let id = Uuid::parse_str(&gateway_id).map_err(|e| e.to_string())?;
    GATEWAY_MANAGER.get_gateway_devices(id).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gateway_execute_command(
    gateway_id: String,
    device_id: String,
    command: EntityCommand,
) -> Result<CommandResult, String> {
    let gid = Uuid::parse_str(&gateway_id).map_err(|e| e.to_string())?;
    let did = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    GATEWAY_MANAGER.execute_device_command(gid, did, command).await
        .map_err(|e| e.to_string())
}
```

**Frontend Components:**

```typescript
// apps/desktop/src/features/gateways/GatewayList.tsx

import { useQuery, useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/tauri';

export function GatewayList() {
  const { data: gateways, isLoading } = useQuery({
    queryKey: ['gateways'],
    queryFn: () => invoke<GatewayState[]>('gateway_list'),
  });

  const connectMutation = useMutation({
    mutationFn: (config: RemoteGatewayConfig) =>
      invoke('gateway_connect_remote', { config }),
  });

  return (
    <div className="gateway-list">
      <h2>Connected Gateways</h2>
      
      {gateways?.map((gateway) => (
        <GatewayCard
          key={gateway.id}
          gateway={gateway}
          onSelect={() => navigate(`/gateways/${gateway.id}`)}
        />
      ))}
      
      <button onClick={() => showAddGatewayDialog()}>
        Add Gateway
      </button>
    </div>
  );
}

// apps/desktop/src/features/gateways/GatewayDetail.tsx

export function GatewayDetail({ gatewayId }: { gatewayId: string }) {
  const { data: devices } = useQuery({
    queryKey: ['gateways', gatewayId, 'devices'],
    queryFn: () => invoke('gateway_get_devices', { gatewayId }),
  });

  return (
    <div className="gateway-detail">
      <h2>Gateway: {gateway.name}</h2>
      
      <div className="devices-grid">
        {devices?.map((device) => (
          <DeviceCard
            key={device.id}
            device={device}
            onSelect={() => navigate(`/devices/${device.id}`)}
          />
        ))}
      </div>
    </div>
  );
}
```

**Tests:**
- [ ] Local gateway discovery
- [ ] Remote gateway connection
- [ ] Device listing via gateway
- [ ] Command execution through gateway
- [ ] WebSocket state updates

#### 8.6 Gateway Configuration UI

**Frontend Settings Page:**

```typescript
// apps/desktop/src/features/settings/GatewaySettings.tsx

export function GatewaySettings() {
  const [config, setConfig] = useState<RemoteGatewayConfig>({
    frp_server: '',
    frp_port: 7000,
    auth_token: '',
    gateway_name: '',
    tls_enabled: true,
  });

  const testConnection = async () => {
    try {
      const result = await invoke('gateway_test_connection', { config });
      toast.success('Connection successful!');
    } catch (error) {
      toast.error(`Connection failed: ${error}`);
    }
  };

  const saveGateway = async () => {
    try {
      await invoke('gateway_connect_remote', { config });
      toast.success('Gateway added successfully!');
    } catch (error) {
      toast.error(`Failed to add gateway: ${error}`);
    }
  };

  return (
    <form className="gateway-settings">
      <h3>Configure Remote Gateway</h3>
      
      <FormField label="FRP Server Address">
        <input
          value={config.frp_server}
          onChange={(e) => setConfig({ ...config, frp_server: e.target.value })}
          placeholder="frp.example.com"
        />
      </FormField>
      
      <FormField label="FRP Server Port">
        <input
          type="number"
          value={config.frp_port}
          onChange={(e) => setConfig({ ...config, frp_port: parseInt(e.target.value) })}
        />
      </FormField>
      
      <FormField label="Auth Token">
        <input
          type="password"
          value={config.auth_token}
          onChange={(e) => setConfig({ ...config, auth_token: e.target.value })}
        />
      </FormField>
      
      <FormField label="Gateway Name">
        <input
          value={config.gateway_name}
          onChange={(e) => setConfig({ ...config, gateway_name: e.target.value })}
          placeholder="my-home-gateway"
        />
      </FormField>
      
      <FormField>
        <checkbox
          checked={config.tls_enabled}
          onChange={(e) => setConfig({ ...config, tls_enabled: e.target.checked })}
        >
          Enable TLS
        </checkbox>
      </FormField>
      
      <div className="actions">
        <button type="button" onClick={testConnection}>
          Test Connection
        </button>
        <button type="button" onClick={saveGateway}>
          Save Gateway
        </button>
      </div>
    </form>
  );
}
```

#### 8.7 FRP Server Setup Guide

**Create FRP Server Setup Script:**

```bash
#!/bin/bash
# scripts/setup-frp-server.sh

set -e

FRP_VERSION="0.52.0"
FRP_DIR="/opt/frp"

echo "Installing FRP Server..."

# Download FRP
wget -q https://github.com/fatedier/frp/releases/download/v${FRP_VERSION}/frp_${FRP_VERSION}_linux_amd64.tar.gz
tar -xzf frp_${FRP_VERSION}_linux_amd64.tar.gz
rm frp_${FRP_VERSION}_linux_amd64.tar.gz

# Install
mkdir -p $FRP_DIR
cp -r frp_${FRP_VERSION}_linux_amd64/* $FRP_DIR/
rm -rf frp_${FRP_VERSION}_linux_amd64

# Create config
cat > $FRP_DIR/frps.toml << EOF
bindPort = 7000
auth.token = "$(openssl rand -hex 32)"

# TLS
transport.tls.certFile = "/etc/frp/server.crt"
transport.tls.keyFile = "/etc/frp/server.key"

# Dashboard
webServer.addr = "0.0.0.0"
webServer.port = 7500
webServer.user = "admin"
webServer.password = "$(openssl rand -base64 16)"

# Logging
log.to = "/var/log/frps.log"
log.level = "info"
log.maxDays = 30
EOF

# Generate TLS certificates
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/frp/server.key \
  -out /etc/frp/server.crt \
  -subj "/CN=frp.example.com"

# Create systemd service
cat > /etc/systemd/system/frps.service << EOF
[Unit]
Description=FRP Server
After=network.target

[Service]
Type=simple
ExecStart=$FRP_DIR/frps -c $FRP_DIR/frps.toml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
systemctl daemon-reload
systemctl enable frps
systemctl start frps

echo "FRP Server installed successfully!"
echo "Dashboard: http://$(hostname -I | awk '{print $1}'):7500"
echo "Auth token: $(grep auth.token $FRP_DIR/frps.toml | cut -d'=' -f2 | tr -d ' ')"
```

#### 8.8 Gateway Server Deployment

**Docker Image:**

```dockerfile
# apps/gateway-server/Dockerfile

FROM rust:1.75 as builder

WORKDIR /build
COPY . .
RUN cargo build --release -p eam-gateway-server

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/eam-gateway-server /usr/local/bin/
COPY --from=builder /build/apps/gateway-server/config.example.toml /etc/gateway/config.toml

RUN mkdir -p /data /var/log/gateway

EXPOSE 8080

CMD ["eam-gateway-server", "--config", "/etc/gateway/config.toml"]
```

**Docker Compose:**

```yaml
# apps/gateway-server/docker-compose.yml

version: '3.8'

services:
  gateway:
    build: .
    image: esphome/eam-gateway:latest
    container_name: eam-gateway
    restart: unless-stopped
    network_mode: host
    volumes:
      - ./gateway-config.toml:/etc/gateway/config.toml:ro
      - gateway-data:/data
    environment:
      - RUST_LOG=info
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

volumes:
  gateway-data:
```

#### 8.9 Testing & Validation

**Integration Tests:**

```rust
// apps/gateway-server/tests/integration_test.rs

#[tokio::test]
async fn test_gateway_discovers_devices() {
    let mut gateway = create_test_gateway().await;
    gateway.start().await.unwrap();
    
    // Wait for discovery
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    let devices = gateway.list_devices().await.unwrap();
    assert!(!devices.is_empty());
}

#[tokio::test]
async fn test_gateway_connects_via_frp() {
    let frp_server = start_test_frp_server().await;
    let gateway = start_test_gateway_with_frp(&frp_server).await;
    
    // Connect desktop client
    let client = GatewayClient::connect_remote(RemoteGatewayConfig {
        frp_server: frp_server.addr,
        ..Default::default()
    }).await.unwrap();
    
    let info = client.get_gateway_info().await.unwrap();
    assert_eq!(info.id, gateway.id);
}

#[tokio::test]
async fn test_command_execution_via_gateway() {
    let gateway = start_test_gateway().await;
    let device = create_mock_esphome_device().await;
    
    let result = gateway.execute_command(
        device.id,
        EntityCommand::Switch(SwitchCommand::TurnOn),
    ).await.unwrap();
    
    assert!(result.success);
}
```

**Performance Tests:**

```rust
#[tokio::test]
async fn test_gateway_handles_multiple_connections() {
    let gateway = start_test_gateway().await;
    
    // Simulate 10 concurrent desktop clients
    let mut handles = vec![];
    
    for i in 0..10 {
        let gateway_addr = gateway.addr.clone();
        handles.push(tokio::spawn(async move {
            let client = GatewayClient::connect(gateway_addr).await.unwrap();
            let devices = client.list_devices().await.unwrap();
            assert!(!devices.is_empty());
        }));
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

### Phase 8 Test Plan

```yaml
Unit Tests:
  - Gateway config parsing
  - FRP config generation
  - Device connection logic
  - API request handling

Integration Tests:
  - Gateway discovers devices via mDNS
  - Gateway connects to FRP server
  - Desktop connects to gateway
  - Command execution through gateway
  - WebSocket state streaming

E2E Tests:
  - Full flow: Desktop → FRP → Gateway → Device
  - Device discovery and control
  - State updates propagation
  - Reconnection scenarios

Performance Tests:
  - 10+ concurrent gateway connections
  - 100+ devices per gateway
  - 1000+ state updates/second
  - Latency measurements

Security Tests:
  - TLS certificate validation
  - Authentication enforcement
  - API key validation
  - Rate limiting
```

### Phase 8 Completion Criteria

- [ ] Gateway server runs as headless service
- [ ] mDNS discovery finds local devices
- [ ] FRP client establishes tunnel
- [ ] Desktop app discovers gateways
- [ ] Remote gateway connection works
- [ ] Devices accessible through gateway
- [ ] Commands execute successfully
- [ ] State updates stream in real-time
- [ ] Reconnection works after disconnect
- [ ] TLS encryption enabled
- [ ] Authentication functional
- [ ] Documentation complete
- [ ] Docker image available

---

## Testing Strategy

### Test Categories

#### Unit Tests
- **Coverage Target**: 80%+
- **Tools**: `cargo test`, `vitest`
- **Scope**: Individual functions, methods, components

#### Integration Tests
- **Tools**: `cargo test --test`, `@tauri-apps/api-mocks`
- **Scope**: Multi-module interactions, database, IPC

#### E2E Tests
- **Tools**: `Playwright`, `WebdriverIO`
- **Scope**: Full user workflows

#### Performance Tests
- **Tools**: Custom benchmarks, `criterion`
- **Scope**: Scalability targets

#### Contract Tests
- **Tools**: JSON Schema validation
- **Scope**: IPC, automation schema, dashboard schema

### Test Fixtures

#### Mock ESPHome Device
```python
# tools/fixture-server/mock_device.py
class MockEspHomeDevice:
    """Simulates ESPHome device for testing"""
    def __init__(self, port=6053, password=None):
        self.port = port
        self.password = password
        self.entities = [
            SensorEntity(key="temperature", value=23.5),
            SensorEntity(key="humidity", value=65.0),
            SwitchEntity(key="light", state=False),
            NumberEntity(key="brightness", value=50, min=0, max=100),
        ]
```

#### Test Database
```rust
pub fn create_test_db() -> SqlitePool {
    // In-memory SQLite for tests
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    
    run_migrations(&pool).await.unwrap();
    pool
}
```

### Continuous Integration

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test-rust:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test --workspace

  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: pnpm/action-setup@v2
      - run: pnpm install
      - run: pnpm test

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
      - run: pnpm lint

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo audit
      - run: pnpm audit
```

---

## Deployment & Operations

### Release Process

1. **Version Bump**
   - Update `Cargo.toml` versions
   - Update `package.json` version
   - Update `tauri.conf.json` version
   - Generate changelog

2. **Build Release**
   ```bash
   make release
   ```

3. **Sign Binaries**
   - Windows: Code signing certificate
   - macOS: Developer ID signing
   - Linux: GPG signing (optional)

4. **Publish**
   - GitHub Releases
   - Auto-update feed (Tauri built-in)

### Update Mechanism

Tauri's built-in updater:
```json
{
  "updater": {
    "active": true,
    "endpoints": ["https://releases.eam.dev/{{target}}/{{arch}}/{{current_version}}"],
    "pubkey": "..."
  }
}
```

### Monitoring & Diagnostics

**Application Telemetry:**
- Error reporting (opt-in)
- Performance metrics
- Usage analytics (anonymized, opt-in)

**User Diagnostics:**
- Export bundle with:
  - Logs (last 7 days)
  - Database schema version
  - Device count
  - Entity count
  - Configuration (redacted)
  - System info

### Support Matrix

| Platform | Architecture | Minimum Version |
|----------|-------------|-----------------|
| Windows | x86_64 | Windows 10 1809+ |
| Linux | x86_64 | glibc 2.31+ |
| macOS | x86_64, aarch64 | macOS 10.15+ |

---

## Success Metrics

### Phase Completion

Each phase is considered complete when:
1. All acceptance criteria met
2. All tests passing (unit, integration, e2e)
3. Code reviewed and merged
4. Documentation updated
5. No critical bugs

### Project Success

The project is successful when:
1. All phases complete
2. Performance targets met
3. All platforms build and run
4. Documentation complete
5. Initial user testing positive

---

## Risk Register

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| ESPHome API incompatibility | High | Medium | Phase 0 spike validates before commitment |
| Tauri 2 stability issues | High | Low | Monitor Tauri releases, have fallback to Tauri 1 |
| Performance targets not met | Medium | Medium | Early performance testing, profiling, optimization |
| Cross-platform issues | Medium | Medium | CI on all platforms, platform-specific testing |
| Scope creep | Medium | High | Strict phase gating, MVP focus |

---

## Next Steps

1. **Immediate**: Complete Phase 0 tasks
   - Set up CI/CD
   - Implement ESPHome API spike
   - Validate toolchain integration
   - Publish ADRs

2. **Short-term**: Begin Phase 1 after Phase 0 acceptance
   - Implement runtime supervisor
   - Set up database migrations
   - Build discovery service

3. **Ongoing**: Update this plan as decisions are made and lessons are learned

---

*This plan is a living document. Update as the project evolves.*
