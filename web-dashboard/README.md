# SSH Bastion Web Dashboard

A real-time web dashboard for monitoring active reverse SSH connections on the bastion host.

## Features

- 🔐 Real-time monitoring of all active reverse SSH tunnels
- 📊 Display connection details: ports, users, bastion info
- 🎨 Modern dark-themed web UI with responsive design
- 🔄 Auto-refresh every 5 seconds
- 📱 Mobile-friendly dashboard
- 🏥 Health check endpoint

## Building

### Prerequisites

- Rust 1.75+ (or use Docker)
- `cargo` package manager

### Build from Source

```bash
cd web-dashboard
cargo build --release
```

The binary will be at `target/release/ssh-bastion-dashboard`

### Build with Docker

```bash
docker build -f web-dashboard/Dockerfile -t ssh-bastion-dashboard .
```

## Running

### Standalone

```bash
./target/release/ssh-bastion-dashboard
```

The dashboard will be available at `http://localhost:8080`

### With Docker Compose

```bash
docker-compose up -d ssh-bastion-dashboard
```

## API Endpoints

- `GET /` - Main dashboard HTML
- `GET /api/connections/json` - Get connections as JSON
- `GET /api/connections` - Get connections as HTML table
- `GET /api/health` - Health check endpoint
- `GET /style.css` - Stylesheet
- `GET /script.js` - JavaScript

## JSON API Response

```json
{
  "total_connections": 2,
  "timestamp": "2024-02-16T10:30:45.123Z",
  "connections": [
    {
      "pid": 12345,
      "user": "root",
      "remote_bind": "*:8080",
      "local_bind": "localhost",
      "remote_port": 8080,
      "local_port": 22,
      "bastion_host": "bastion.example.com",
      "bastion_port": 2222,
      "command": "autossh -M 0 -R *:8080:localhost:22 ...",
      "uptime": "running",
      "status": "Connected"
    }
  ]
}
```

## How It Works

1. The dashboard monitors all processes on the system
2. It parses for `autossh` and `ssh` processes with reverse port forwarding (`-R` flag)
3. Extracts connection details (ports, hosts, users)
4. Presents them in a real-time updating web UI

## Configuration

You can control the refresh interval by modifying `script.js`:

```javascript
const CONFIG = {
  REFRESH_INTERVAL: 5000, // milliseconds
  API_ENDPOINT: "/api/connections/json",
};
```

## Docker Compose Integration

The `docker-compose.yml` includes the dashboard service with:

- Port mapping: `8080:8080`
- Volume mounts for process monitoring
- Auto-restart policy
- Health checks
- Resource limits

Access the dashboard at: `http://localhost:8080`

## Resource Usage

- **CPU**: ~0.1-0.5 cores
- **Memory**: ~64-256 MB
- **Disk**: Minimal (binary only ~10 MB)

## Troubleshooting

### Dashboard shows "No active connections"

This usually means:

1. No reverse SSH tunnels are currently active
2. The monitoring is working correctly - wait for a tunnel to be established

### "Permission denied" errors

The application reads from `/proc` to get process information. Ensure:

```bash
# With Docker
docker exec ssh-bastion-dashboard ls /proc | head

# Standalone
ps aux | grep autossh
```

### Build failures

Ensure you have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Architecture

```
┌─────────────────────────────────────┐
│   Web Browser                       │
│   (http://localhost:8080)           │
└──────────────┬──────────────────────┘
               │
        ┌──────▼───────┐
        │  Actix-web   │
        │  HTTP Server │
        └──────┬───────┘
               │
        ┌──────▼──────────┐
        │  SSH Monitor    │
        │  (Process List) │
        └─────────────────┘
               │
        ┌──────▼──────────┐
        │  /proc parsing  │
        │  Regex matching │
        └─────────────────┘
```

## Development

### Adding New Metrics

1. Update `models.rs` with new fields
2. Modify `ssh_monitor.rs` to extract the data
3. Update `handlers.rs` to send the data
4. Update `script.js` to display it

### Local Development

```bash
# Terminal 1: Start the server
cd web-dashboard
RUST_LOG=debug cargo run

# Terminal 2: Test requests
curl http://localhost:8080/api/connections/json
curl http://localhost:8080/api/health
```

## License

MIT
