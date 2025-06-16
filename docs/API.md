# CGMiner-RS API Documentation

The CGMiner-RS API provides comprehensive access to mining operations, device management, and system monitoring through RESTful endpoints and WebSocket connections.

## Base URL

```
http://localhost:8080/api/v1
```

## Authentication

API requests can be authenticated using a bearer token:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:8080/api/v1/status
```

Configure the token in `config.toml`:

```toml
[api]
auth_token = "your_secret_token"
```

## Response Format

All API responses follow a consistent format:

```json
{
  "success": true,
  "data": { ... },
  "message": "Operation completed successfully",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Error responses:

```json
{
  "success": false,
  "error": "Error description",
  "code": "ERROR_CODE",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Endpoints

### System Status

#### GET /status

Get overall system status and statistics.

**Response:**
```json
{
  "success": true,
  "data": {
    "version": "1.0.0",
    "uptime": 3600,
    "mining_state": "Running",
    "total_hashrate": 75.5,
    "accepted_shares": 2500,
    "rejected_shares": 25,
    "hardware_errors": 3,
    "active_devices": 2,
    "connected_pools": 1,
    "current_difficulty": 1024.0,
    "best_share": 5000.0
  }
}
```

#### GET /stats

Get detailed mining statistics.

**Response:**
```json
{
  "success": true,
  "data": {
    "mining_stats": {
      "start_time": 1705312200,
      "uptime": 3600,
      "total_hashes": 271800000000,
      "accepted_shares": 2500,
      "rejected_shares": 25,
      "hardware_errors": 3,
      "stale_shares": 8,
      "best_share": 5000.0,
      "current_difficulty": 1024.0,
      "average_hashrate": 75.2,
      "current_hashrate": 75.5,
      "efficiency": 22.5,
      "power_consumption": 3200.0
    },
    "device_stats": [...],
    "pool_stats": [...]
  }
}
```

### Device Management

#### GET /devices

Get list of all mining devices.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "device_id": 0,
      "name": "Maijie L7 Chain 0",
      "status": "Mining",
      "temperature": 65.5,
      "hashrate": 38.0,
      "accepted_shares": 1250,
      "rejected_shares": 15,
      "hardware_errors": 2,
      "uptime": 3600,
      "last_share_time": 1705312800
    },
    {
      "device_id": 1,
      "name": "Maijie L7 Chain 1",
      "status": "Mining",
      "temperature": 67.2,
      "hashrate": 37.5,
      "accepted_shares": 1180,
      "rejected_shares": 12,
      "hardware_errors": 1,
      "uptime": 3600,
      "last_share_time": 1705312790
    }
  ]
}
```

#### GET /devices/{device_id}

Get detailed information about a specific device.

**Parameters:**
- `device_id` (integer): Device identifier

**Response:**
```json
{
  "success": true,
  "data": {
    "device_id": 0,
    "name": "Maijie L7 Chain 0",
    "status": "Mining",
    "temperature": 65.5,
    "hashrate": 38.0,
    "power_consumption": 1600.0,
    "fan_speed": 2800,
    "voltage": 875,
    "frequency": 525,
    "error_rate": 1.2,
    "uptime": 3600,
    "accepted_shares": 1250,
    "rejected_shares": 15,
    "hardware_errors": 2,
    "last_share_time": 1705312800,
    "chip_count": 76,
    "enabled": true
  }
}
```

#### POST /devices/{device_id}/restart

Restart a specific device.

**Parameters:**
- `device_id` (integer): Device identifier

**Response:**
```json
{
  "success": true,
  "data": "Device 0 restart initiated",
  "message": "Device restart command sent successfully"
}
```

#### PUT /devices/{device_id}/config

Update device configuration.

**Parameters:**
- `device_id` (integer): Device identifier

**Request Body:**
```json
{
  "frequency": 550,
  "voltage": 900,
  "enabled": true,
  "auto_tune": false
}
```

**Response:**
```json
{
  "success": true,
  "data": "Device 0 configuration updated",
  "message": "Configuration applied successfully"
}
```

### Pool Management

#### GET /pools

Get list of all configured pools.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "pool_id": 0,
      "url": "stratum+tcp://pool.example.com:4444",
      "status": "Connected",
      "priority": 1,
      "accepted_shares": 2430,
      "rejected_shares": 27,
      "stale_shares": 5,
      "difficulty": 1024.0,
      "ping": 45,
      "connected_at": 1705308600
    },
    {
      "pool_id": 1,
      "url": "stratum+tcp://backup.example.com:4444",
      "status": "Disconnected",
      "priority": 2,
      "accepted_shares": 0,
      "rejected_shares": 0,
      "stale_shares": 0,
      "difficulty": 0.0,
      "ping": null,
      "connected_at": null
    }
  ]
}
```

#### GET /pools/{pool_id}

Get detailed information about a specific pool.

**Parameters:**
- `pool_id` (integer): Pool identifier

#### PUT /pools/{pool_id}/config

Update pool configuration.

**Parameters:**
- `pool_id` (integer): Pool identifier

**Request Body:**
```json
{
  "url": "stratum+tcp://newpool.example.com:4444",
  "user": "new_username",
  "password": "new_password",
  "priority": 1,
  "enabled": true
}
```

### Control Operations

#### POST /control

Execute control commands.

**Request Body:**
```json
{
  "command": "start",
  "parameters": {}
}
```

**Available Commands:**
- `start`: Start mining
- `stop`: Stop mining
- `restart`: Restart mining
- `pause`: Pause mining
- `resume`: Resume mining

**Response:**
```json
{
  "success": true,
  "data": {
    "command": "start",
    "success": true,
    "message": "Mining started successfully",
    "result": null
  }
}
```

### Configuration

#### GET /config

Get current configuration.

**Response:**
```json
{
  "success": true,
  "data": {
    "mining": {
      "scan_interval": 5,
      "work_restart_timeout": 60,
      "enable_auto_tuning": true
    },
    "devices": {
      "scan_interval": 10,
      "chains": [...]
    },
    "pools": {
      "strategy": "Failover",
      "retry_interval": 30,
      "pools": [...]
    },
    "api": {
      "enabled": true,
      "bind_address": "127.0.0.1",
      "port": 8080
    },
    "monitoring": {
      "enabled": true,
      "metrics_interval": 30
    }
  }
}
```

#### PUT /config

Update configuration.

**Request Body:**
```json
{
  "mining": {
    "scan_interval": 10,
    "enable_auto_tuning": false
  }
}
```

## WebSocket API

Connect to real-time updates via WebSocket:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = function() {
    // Subscribe to events
    ws.send(JSON.stringify({
        type: 'subscribe',
        events: ['mining_events', 'device_events', 'pool_events']
    }));
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};
```

### WebSocket Message Types

#### Subscribe to Events
```json
{
  "type": "subscribe",
  "events": ["mining_events", "device_events", "pool_events", "system_events"]
}
```

#### Unsubscribe from Events
```json
{
  "type": "unsubscribe",
  "events": ["device_events"]
}
```

#### Event Messages
```json
{
  "type": "mining_event",
  "event": "share_accepted",
  "data": {
    "work_id": "550e8400-e29b-41d4-a716-446655440000",
    "difficulty": 1024.0,
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

## Error Codes

| Code | Description |
|------|-------------|
| `INVALID_PARAMETER` | Invalid request parameter |
| `DEVICE_NOT_FOUND` | Device not found |
| `POOL_NOT_FOUND` | Pool not found |
| `OPERATION_FAILED` | Operation failed |
| `AUTHENTICATION_REQUIRED` | Authentication required |
| `INSUFFICIENT_PERMISSIONS` | Insufficient permissions |
| `RATE_LIMITED` | Rate limit exceeded |
| `INTERNAL_ERROR` | Internal server error |

## Rate Limiting

API requests are rate limited to prevent abuse:

- **Default limit**: 100 requests per minute per IP
- **Burst limit**: 20 requests per second
- **Headers**: Rate limit information is included in response headers

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705312860
```

## Examples

### Python Client

```python
import requests
import json

class CGMinerClient:
    def __init__(self, base_url, token=None):
        self.base_url = base_url
        self.headers = {}
        if token:
            self.headers['Authorization'] = f'Bearer {token}'
    
    def get_status(self):
        response = requests.get(f'{self.base_url}/status', headers=self.headers)
        return response.json()
    
    def restart_device(self, device_id):
        response = requests.post(
            f'{self.base_url}/devices/{device_id}/restart',
            headers=self.headers
        )
        return response.json()

# Usage
client = CGMinerClient('http://localhost:8080/api/v1', 'your_token')
status = client.get_status()
print(f"Total hashrate: {status['data']['total_hashrate']} GH/s")
```

### JavaScript Client

```javascript
class CGMinerAPI {
    constructor(baseUrl, token) {
        this.baseUrl = baseUrl;
        this.headers = {
            'Content-Type': 'application/json'
        };
        if (token) {
            this.headers['Authorization'] = `Bearer ${token}`;
        }
    }
    
    async getStatus() {
        const response = await fetch(`${this.baseUrl}/status`, {
            headers: this.headers
        });
        return response.json();
    }
    
    async updateDeviceConfig(deviceId, config) {
        const response = await fetch(`${this.baseUrl}/devices/${deviceId}/config`, {
            method: 'PUT',
            headers: this.headers,
            body: JSON.stringify(config)
        });
        return response.json();
    }
}

// Usage
const api = new CGMinerAPI('http://localhost:8080/api/v1', 'your_token');
api.getStatus().then(status => {
    console.log(`Mining state: ${status.data.mining_state}`);
});
```
