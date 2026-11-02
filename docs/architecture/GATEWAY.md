# Gateway Server - Remote Fleet Management

## Overview

The Gateway Server enables remote management of ESPHome devices when the desktop application is not on the local network. It acts as a bridge between ESPHome devices on a remote network and the desktop application via FRP (Fast Reverse Proxy) tunnels.

## Architecture

![Gateway Server Architecture Summary](kiro-artifact://137554f5-9bc5-49af-b563-6a3c707937ed)

## Quick Start

### 1. Setup FRP Server

```bash
# Download FRP
wget https://github.com/fatedier/frp/releases/download/v0.52.0/frp_0.52.0_linux_amd64.tar.gz
tar -xzf frp_0.52.0_linux_amd64.tar.gz

# Configure frps.toml
cat > frps.toml << EOF
bindPort = 7000
auth.token = "your-secure-token"
EOF

# Start FRP server
./frps -c frps.toml
```

### 2. Install Gateway Server

```bash
# Download gateway server
wget https://github.com/your-org/esphome-automation-manager/releases/download/v1.0.0/gateway-server-linux-arm64

# Create configuration
cat > gateway-config.toml << EOF
[gateway]
id = "$(uuidgen)"
name = "My Gateway"

[local_network]
mdns_enabled = true

[frp]
server_addr = "frp.example.com"
server_port = 7000
auth_token = "your-secure-token"
gateway_name = "my-gateway"

[api_server]
listen_addr = "0.0.0.0:8080"
EOF

# Run gateway server
./gateway-server-linux-arm64 --config gateway-config.toml
```

### 3. Connect from Desktop App

1. Open ESPHome Automation Manager
2. Go to Settings → Gateways
3. Click "Add Gateway"
4. Enter FRP server details:
   - Server: `frp.example.com`
   - Port: `7000`
   - Token: `your-secure-token`
   - Gateway Name: `my-gateway`
5. Click "Connect"

## Configuration

### Gateway Configuration File

```toml
# Gateway Server Configuration

[gateway]
# Unique identifier (auto-generated if not provided)
id = "550e8400-e29b-41d4-a716-446655440000"

# Display name for this gateway
name = "Home Gateway"

[local_network]
# Enable mDNS discovery
mdns_enabled = true

# Discovery interval in seconds
discovery_interval_secs = 60

# Network interface to bind (optional)
interface = "eth0"

# Manual devices (if mDNS unavailable)
[[local_network.manual_devices]]
name = "basement-sensor"
host = "192.168.1.100"
port = 6053
password = "optional-password"

[frp]
# FRP server address
server_addr = "frp.example.com"

# FRP server port
server_port = 7000

# Authentication token (must match frps)
auth_token = "your-secure-token"

# Gateway name (used as proxy name)
gateway_name = "home-gateway"

[frp.tls]
# Enable TLS encryption
enabled = true

# Certificate paths
cert_path = "/etc/gateway/cert.pem"
key_path = "/etc/gateway/key.pem"
ca_path = "/etc/gateway/ca.pem"

[frp.reconnect]
# Reconnection settings
max_retries = 10
initial_delay_secs = 1
max_delay_secs = 60
backoff_multiplier = 2.0

[api_server]
# API server listen address
listen_addr = "0.0.0.0:8080"

[api_server.tls]
# Enable HTTPS
enabled = true
cert_path = "/etc/gateway/api-cert.pem"
key_path = "/etc/gateway/api-key.pem"

[api_server.auth]
# Authentication type: api_key, jwt, none
type = "api_key"

# API keys
api_keys = [
    "sk_live_abc123...",
    "sk_live_def456..."
]

[security]
# Audit log path
audit_log_path = "/var/log/gateway/audit.log"

# Maximum request size in MB
max_request_size_mb = 10

# Rate limit (requests per minute)
rate_limit_requests_per_minute = 1000

[database]
# SQLite database path
path = "/data/gateway.db"

[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Log file path
path = "/var/log/gateway/gateway.log"

# Maximum log file size in MB
max_size_mb = 10

# Maximum number of log files
max_files = 5
```

## API Reference

### REST API

#### Get Gateway Info

```http
GET /api/v1/gateway/info
Authorization: Bearer sk_live_abc123...
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Home Gateway",
  "status": "online",
  "version": "1.0.0",
  "uptime_secs": 86400,
  "device_count": 5,
  "entity_count": 23
}
```

#### List Devices

```http
GET /api/v1/devices
Authorization: Bearer sk_live_abc123...
```

Response:
```json
[
  {
    "id": "device-uuid-1",
    "name": "basement-sensor",
    "connection_status": "online",
    "entity_count": 5,
    "last_seen_at": "2026-11-02T10:30:00Z"
  },
  ...
]
```

#### Get Device Details

```http
GET /api/v1/devices/{device_id}
Authorization: Bearer sk_live_abc123...
```

#### List Device Entities

```http
GET /api/v1/devices/{device_id}/entities
Authorization: Bearer sk_live_abc123...
```

#### Execute Device Command

```http
POST /api/v1/devices/{device_id}/command
Authorization: Bearer sk_live_abc123...
Content-Type: application/json

{
  "entity_id": "entity-uuid",
  "command": "switch.turn_on",
  "params": {}
}
```

#### Get Entity State

```http
GET /api/v1/entities/{entity_id}/state
Authorization: Bearer sk_live_abc123...
```

#### Execute Entity Command

```http
POST /api/v1/entities/{entity_id}/command
Authorization: Bearer sk_live_abc123...
Content-Type: application/json

{
  "command": "turn_on"
}
```

### WebSocket API

Connect to real-time state updates:

```javascript
const ws = new WebSocket('wss://gateway:8080/api/v1/ws');

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('State update:', update);
};

// Subscribe to entity updates
ws.send(JSON.stringify({
  type: 'subscribe',
  entity_ids: ['entity-uuid-1', 'entity-uuid-2']
}));
```

Message types:
- `subscribe`: Subscribe to entity state updates
- `unsubscribe`: Unsubscribe from updates
- `state_update`: Entity state changed
- `device_status`: Device connection status changed
- `gateway_status`: Gateway status changed

## Deployment

### Docker

```bash
# Pull image
docker pull esphome/eam-gateway:latest

# Run container
docker run -d \
  --name eam-gateway \
  --network host \
  -v $(pwd)/gateway-config.toml:/etc/gateway/config.toml:ro \
  -v gateway-data:/data \
  esphome/eam-gateway:latest
```

### Docker Compose

```yaml
version: '3.8'

services:
  gateway:
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
```

### Systemd Service

```ini
[Unit]
Description=ESPHome Gateway Server
After=network.target

[Service]
Type=simple
User=gateway
Group=gateway
ExecStart=/usr/local/bin/gateway-server --config /etc/gateway/config.toml
Restart=on-failure
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/data /var/log/gateway

[Install]
WantedBy=multi-user.target
```

### Kubernetes

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
          readOnly: true
        - name: data
          mountPath: /data
        resources:
          limits:
            memory: "512Mi"
            cpu: "500m"
          requests:
            memory: "256Mi"
            cpu: "250m"
      volumes:
      - name: config
        configMap:
          name: gateway-config
      - name: data
        persistentVolumeClaim:
          claimName: gateway-data-pvc
```

## Security

### TLS Certificates

Generate self-signed certificates:

```bash
# CA certificate
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 365 -key ca.key -out ca.crt

# Server certificate
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key -set_serial 01 -out server.crt
```

Or use Let's Encrypt:

```bash
certbot certonly --standalone -d gateway.example.com
```

### API Keys

Generate secure API keys:

```bash
# Generate random API key
openssl rand -hex 32
```

Add to configuration:

```toml
[api_server.auth]
type = "api_key"
api_keys = [
  "sk_live_generated_key_here"
]
```

### Firewall

Required ports:
- **6053**: ESPHome Native API (local, inbound)
- **8080**: Gateway API server (local, inbound)
- **7000**: FRP server connection (outbound)
- **5353**: mDNS discovery (local, inbound/outbound)

UFW rules:

```bash
# Allow local ESPHome devices
sudo ufw allow from 192.168.1.0/24 to any port 6053

# Allow local API access
sudo ufw allow from 192.168.1.0/24 to any port 8080

# Allow mDNS
sudo ufw allow 5353/udp

# Allow FRP client connection (outbound)
sudo ufw allow out 7000/tcp
```

## Monitoring

### Health Check Endpoint

```http
GET /api/v1/status
```

Response:
```json
{
  "status": "healthy",
  "uptime_secs": 86400,
  "devices": {
    "total": 5,
    "online": 4,
    "offline": 1
  },
  "frp": {
    "connected": true,
    "server": "frp.example.com:7000",
    "uptime_secs": 86000
  },
  "database": {
    "status": "healthy",
    "size_bytes": 1048576
  }
}
```

### Prometheus Metrics

Metrics endpoint: `GET /api/v1/metrics`

Available metrics:
- `eam_gateway_uptime_seconds`
- `eam_gateway_devices_total`
- `eam_gateway_devices_online`
- `eam_gateway_entities_total`
- `eam_gateway_commands_total`
- `eam_gateway_commands_failed_total`
- `eam_gateway_api_requests_total`
- `eam_gateway_api_request_duration_seconds`

### Logging

Structured JSON logs:

```json
{
  "timestamp": "2026-11-02T10:30:00Z",
  "level": "INFO",
  "target": "eam_gateway::device",
  "message": "Device connected",
  "device_id": "device-uuid",
  "device_name": "basement-sensor"
}
```

## Troubleshooting

### Gateway won't start

```bash
# Check logs
journalctl -u eam-gateway -f

# Verify configuration
./gateway-server --config gateway-config.toml --validate

# Check permissions
ls -la /etc/gateway/
ls -la /data/
```

### FRP connection fails

```bash
# Test FRP server connectivity
nc -zv frp.example.com 7000

# Check FRP logs
tail -f /var/log/frpc.log

# Verify auth token
grep auth_token gateway-config.toml
```

### Devices not discovered

```bash
# Check mDNS
avahi-browse -at

# Test manual connection
./gateway-server --manual-device 192.168.1.100:6053

# Check firewall
sudo ufw status
```

### API authentication fails

```bash
# Verify API key
grep api_keys gateway-config.toml

# Test API call
curl -H "Authorization: Bearer sk_live_abc123..." \
  http://localhost:8080/api/v1/gateway/info
```

## Performance Tuning

### Resource Limits

Minimum requirements:
- CPU: 1 core
- RAM: 256 MB
- Storage: 1 GB

Recommended for 50+ devices:
- CPU: 2 cores
- RAM: 512 MB
- Storage: 5 GB

### Database Optimization

```toml
[database]
# Enable WAL mode
wal_mode = true

# Connection pool size
pool_size = 5

# Busy timeout in ms
busy_timeout_ms = 5000
```

### API Server Tuning

```toml
[api_server]
# Maximum concurrent connections
max_connections = 100

# Request timeout in seconds
request_timeout_secs = 30

# Keep-alive timeout in seconds
keep_alive_timeout_secs = 5
```

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/your-org/esphome-automation-manager.git
cd esphome-automation-manager

# Build gateway server
cargo build --release -p eam-gateway-server

# Run with example config
./target/release/eam-gateway-server --config apps/gateway-server/config.example.toml
```

### Running Tests

```bash
# Unit tests
cargo test -p eam-gateway-server

# Integration tests
cargo test -p eam-gateway-server --test integration

# With coverage
cargo tarpaulin -p eam-gateway-server
```

### Debug Mode

```bash
# Run with debug logging
RUST_LOG=debug ./gateway-server --config gateway-config.toml

# Enable trace logging for specific module
RUST_LOG=eam_gateway::device=trace ./gateway-server
```

## Support

- Documentation: https://docs.eam.dev/gateway
- Issues: https://github.com/your-org/esphome-automation-manager/issues
- Discord: https://discord.gg/eam

## License

MIT License
