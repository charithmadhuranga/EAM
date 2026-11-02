# ADR-008: Gateway Server Architecture for Remote Fleet Management

## Status

Proposed

## Context

The ESPHome Automation Manager is designed as a local-first desktop application. However, many use cases require managing devices on remote networks:

1. **Fleet Operators**: Manage devices across multiple sites (offices, factories, warehouses)
2. **Installers**: Service client sites remotely
3. **Advanced Users**: Manage home devices while traveling
4. **Multi-site Management**: Combine local and remote devices in one interface

The desktop application cannot directly access ESPHome devices on remote networks due to:
- Network NAT and firewalls
- No public IP addresses on most networks
- Security restrictions

We need a solution that:
- Allows remote access to ESPHome devices
- Maintains the local-first philosophy
- Does not require cloud services for core functionality
- Supports multiple remote sites
- Provides secure, authenticated connections

## Decision

We will implement a **Gateway Server** architecture with **FRP (Fast Reverse Proxy)** integration.

### Architecture Components

1. **Gateway Server** (Headless Rust Server)
   - Runs on a device in the remote network (Raspberry Pi, mini PC, etc.)
   - Discovers and manages local ESPHome devices
   - Provides ESPHome Native API proxy functionality
   - Maintains persistent connection to FRP server

2. **FRP Server (frps)**
   - Public-facing server with static IP/domain
   - Accepts connections from Gateway Servers (frpc clients)
   - Provides secure tunnel for desktop app to reach remote gateways
   - Can be self-hosted or use FRP cloud services

3. **Desktop Application**
   - Auto-discovers local gateways via mDNS
   - Connects to remote gateways via FRP server
   - Displays hierarchical view: Gateways → Devices → Entities
   - Transparently handles local vs remote device access

### Connection Modes

#### Local Network (Direct)
```
Desktop App ←──mDNS──→ ESPHome Devices (Port 6053)
```

#### Remote Network (via Gateway)
```
Desktop App ←──TLS──→ FRP Server ←──TLS──→ Gateway Server ←──Local──→ ESPHome Devices
```

### Key Design Decisions

1. **Use FRP instead of custom tunneling**
   - FRP is mature, well-tested, and widely used
   - Supports TCP, UDP, HTTP, HTTPS protocols
   - Built-in TLS encryption and authentication
   - Active community and regular updates
   - Reduces implementation complexity

2. **Gateway as separate application, not embedded**
   - Allows deployment on low-power devices (Raspberry Pi Zero)
   - Independent lifecycle and updates
   - Smaller attack surface
   - Can run on existing infrastructure

3. **API-based communication between Desktop and Gateway**
   - REST API for commands and queries
   - WebSocket for real-time state updates
   - Standard protocols, easy to debug
   - Language-agnostic interface

4. **Hierarchical data model**
   - Gateways contain Devices
   - Devices contain Entities
   - Clear separation of concerns
   - Supports multi-site management

## Alternatives Considered

### 1. Cloud-Based Architecture

**Description**: All devices connect to a central cloud server. Desktop app communicates with cloud.

**Pros**:
- Simplest for users (no network configuration)
- Always accessible from anywhere
- Central management

**Cons**:
- Violates local-first principle
- Requires internet for basic operations
- Privacy concerns (data in cloud)
- Monthly costs for cloud infrastructure
- Single point of failure
- Not suitable for air-gapped networks

**Decision**: Rejected. Contradicts core product principles.

### 2. VPN-Only Architecture

**Description**: Users setup VPN servers on remote networks. Desktop connects via VPN.

**Pros**:
- Secure, established technology
- Full network access
- No custom protocol needed

**Cons**:
- Complex setup for users
- Requires VPN client on desktop
- Network configuration knowledge required
- Not user-friendly for non-technical users
- VPN software conflicts possible

**Decision**: Supported as alternative option for advanced users, but not primary method.

### 3. WebRTC / P2P Architecture

**Description**: Direct peer-to-peer connections between desktop and gateway using WebRTC.

**Pros**:
- No intermediate server needed (after signaling)
- Low latency
- Good for real-time data

**Cons**:
- Complex NAT traversal
- Requires signaling server anyway
- Connection reliability issues
- Not all networks support STUN/TURN
- Browser-centric technology

**Decision**: Rejected. Too unreliable for production use.

### 4. Custom Tunneling Protocol

**Description**: Implement custom reverse tunnel similar to ngrok.

**Pros**:
- Full control over protocol
- Optimized for ESPHome use case

**Cons**:
- High implementation effort
- Security hardening required
- Must reinvent proven solutions
- Maintenance burden

**Decision**: Rejected. FRP already solves this problem well.

## Consequences

### Positive

1. **Maintains Local-First Philosophy**
   - Core functionality works without internet
   - Local devices accessed directly
   - No cloud dependency

2. **Flexible Deployment**
   - Users can self-host FRP server
   - Or use FRP cloud services
   - Or use VPN as alternative

3. **Scalable**
   - Support unlimited remote sites
   - Each site manages its own devices
   - Desktop app aggregates view

4. **Secure**
   - TLS encryption for all remote connections
   - Authentication tokens
   - Certificate pinning
   - Audit logging

5. **User-Friendly**
   - Simple setup process
   - Auto-discovery of local gateways
   - Clear UI for remote gateway management

### Negative

1. **Additional Component**
   - Users must deploy Gateway Server
   - Requires device on remote network
   - Additional setup complexity

2. **FRP Dependency**
   - External project for tunneling
   - Must track FRP updates
   - Potential compatibility issues

3. **Infrastructure Requirement**
   - Need FRP server (self-hosted or cloud)
   - Static IP or domain name
   - TLS certificates

4. **Latency**
   - Additional hop through FRP server
   - Higher latency for remote commands
   - Not suitable for real-time control over WAN

### Mitigations

1. **Easy Setup**
   - Provide Docker images
   - Create setup scripts
   - Detailed documentation
   - One-line install command

2. **FRP Management**
   - Pin FRP version in releases
   - Test compatibility before updates
   - Provide fallback to previous version

3. **Infrastructure Options**
   - Provide FRP hosting guide
   - Partner with FRP cloud providers
   - Offer managed service (future)
   - Support WireGuard as VPN alternative

4. **Latency Awareness**
   - Display latency metrics
   - Warn users of high latency
   - Optimize protocol for efficiency
   - Local caching of state

## Implementation Notes

### Phase 8 Implementation Order

1. **Gateway Server Core**
   - Configuration management
   - Device discovery
   - Basic API server

2. **FRP Integration**
   - frpc process management
   - Configuration generation
   - Connection monitoring

3. **Desktop Integration**
   - Gateway discovery
   - Remote connection UI
   - Device proxy logic

4. **Security**
   - TLS implementation
   - Authentication
   - Audit logging

5. **Deployment**
   - Docker images
   - Setup scripts
   - Documentation

### Security Checklist

- [ ] TLS 1.3 for all connections
- [ ] Certificate verification
- [ ] API key or JWT authentication
- [ ] Rate limiting
- [ ] Request size limits
- [ ] Audit logging
- [ ] Secure credential storage
- [ ] Certificate rotation mechanism

### Monitoring & Observability

- Gateway health status
- FRP connection status
- Device connectivity
- API request metrics
- Error rates
- Latency measurements

## References

- [FRP Documentation](https://github.com/fatedier/frp)
- [ESPHome Native API](https://esphome.io/components/api.html)
- [WireGuard](https://www.wireguard.com/)
- [Tailscale](https://tailscale.com/) (alternative to FRP)

## Decision History

- **2026-11-02**: Initial proposal
