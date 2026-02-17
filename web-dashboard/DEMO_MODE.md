# Demo Mode for Local Testing

The SSH Bastion Dashboard includes a demo mode for testing the UI layout and functionality without requiring a running SSH monitor backend.

## How to Use Demo Mode

### Option 1: URL Query Parameter

Add `?demo=1` to your dashboard URL:

```
http://localhost:3000/?demo=1
http://127.0.0.1:8080/?demo=1
```

### Option 2: Browser Console

Toggle demo mode in the browser console:

```javascript
demoMode = true;
loadConnections();
```

## Demo Data

The demo data is located in: `src/templates/demo-data.json`

The file contains sample SSH tunnel connections with:

- 3 example reverse tunnel connections
- Different reverse ports (8951, 2222, 3333)
- Realistic connection details and SSH commands

## Example Demo Data Structure

```json
{
  "total_connections": 3,
  "timestamp": 1708104000,
  "connections": [
    {
      "pid": 9071,
      "user": "bastion",
      "remote_bind": "*:8951",
      "local_bind": "localhost:22",
      "remote_port": 8951,
      "local_port": 22,
      "bastion_host": "nas.example.com",
      "bastion_port": 22,
      "command": "ssh -p 8951 root@192.168.1.12",
      "uptime": "running",
      "status": "Connected"
    }
  ]
}
```

## Console Output

When demo mode is active, you'll see a warning in the browser console:

```
🔐 SSH Bastion Dashboard
📋 DEMO MODE ENABLED
Using demo-data.json for testing
```

## Modifying Demo Data

To add or modify connections in the demo:

1. Edit `src/templates/demo-data.json`
2. Reload the page with `?demo=1` parameter
3. The dashboard will display the updated demo connections

## Notes

- Demo mode is useful for UI testing and development without backend dependencies
- Demo data uses fictional servers and ports for illustration
- When demo mode is disabled, the dashboard connects to the real API endpoint (`/api/connections/json`)
