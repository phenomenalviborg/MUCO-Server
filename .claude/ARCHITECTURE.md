# MUCO Architecture Documentation

## System Components

### Server (port 1302)
- **Purpose**: VR headset connection server
- **Location**: `server/` directory
- **Port**: 1302 (VR clients connect here)
- **Binary**: `target/debug/server`
- **Function**:
  - Accepts connections from VR headsets
  - Manages VR client state
  - Broadcasts mDNS as `_muco-server._tcp.local.`
  - Does NOT have a WebSocket API for frontends

### Manager (ports 9080/9443)
- **Purpose**: Management/monitoring application that connects TO servers
- **Location**: `manager/` directory
- **Ports**:
  - 9080 (HTTP)
  - 9443 (HTTPS via Caddy proxy)
- **Binary**: `target/debug/manager`
- **Function**:
  - WebSocket server for frontend connections (`/api/ws`)
  - mDNS discovery service (finds MUCO servers on network)
  - Connects TO MUCO servers to read/send commands
  - Relay client that connects to servers on port 888
  - Exposes headset data from servers to frontend

### Frontend
- **Purpose**: Web UI for managing VR sessions
- **Location**: `MUCO-Manager/frontend/`
- **Dev Port**: 4173 (preview), 5173 (dev)
- **Function**:
  - Connects to Manager WebSocket
  - Displays discovered servers
  - Shows headset status from connected servers
  - Sends commands to headsets via manager

## Architecture Flow

```
VR Headset --> Server (1302) <-- Manager (relay client on 888)
                                    |
                                    v
                              WebSocket (9080/9443)
                                    |
                                    v
                                Frontend
```

### Normal Operation
1. **Server** runs on the machine with VR headsets
2. **Manager** runs on the SAME machine OR remotely
3. Manager connects TO Server on port 888 (relay port)
4. VR Headsets connect TO Server on port 1302
5. Frontend connects TO Manager WebSocket
6. Manager relays headset data from Server to Frontend

## Key Insights

- Server and Manager are **typically run on the same machine**, not in separate Docker containers
- The hardcoded port 888 in manager/src/main.rs:39 expects the Server to accept relay connections
- Discovery shows available servers, but Manager needs to CONNECT to them to get headset data
- The current Docker setup only runs Manager, not Server (which is correct for remote management scenarios)

## Current Issue

The Manager is discovering the Server via mDNS but **is NOT connected to it** to receive headset data. The hardcoded localhost:888 connection doesn't match the running server at 172.20.10.14:1302.

## Solution Needed

The Manager needs a way to:
1. Connect to discovered servers (not just detect them)
2. Use the correct relay port (888) that the server exposes
3. OR: Server needs to expose port 888 for relay connections
