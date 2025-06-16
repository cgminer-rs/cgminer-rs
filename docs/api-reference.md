# CGMiner-RS API 参考文档

CGMiner-RS 提供了完整的 RESTful API 和 WebSocket 接口，用于监控和控制挖矿设备。

## 基础信息

- **Base URL**: `http://localhost:4028/api/v1`
- **Content-Type**: `application/json`
- **认证**: Bearer Token (可选)

## 认证

如果启用了 API 认证，需要在请求头中包含认证令牌：

```http
Authorization: Bearer your-auth-token
```

## 系统状态 API

### 获取系统状态

获取整个挖矿系统的状态概览。

```http
GET /api/v1/status
```

**响应示例:**

```json
{
  "status": "ok",
  "data": {
    "version": "0.1.0",
    "uptime": 3600,
    "total_hashrate": 110.5,
    "total_devices": 2,
    "active_devices": 2,
    "total_accepted_shares": 1250,
    "total_rejected_shares": 15,
    "current_pool": {
      "url": "stratum+tcp://pool.example.com:4444",
      "user": "username",
      "status": "connected"
    },
    "system_load": {
      "cpu_usage": 25.5,
      "memory_usage": 45.2,
      "disk_usage": 12.8
    }
  }
}
```

### 获取系统统计

获取详细的系统统计信息。

```http
GET /api/v1/stats
```

**响应示例:**

```json
{
  "status": "ok",
  "data": {
    "runtime_stats": {
      "start_time": "2024-01-01T00:00:00Z",
      "uptime_seconds": 3600,
      "restart_count": 0
    },
    "mining_stats": {
      "total_hashes": 1500000000,
      "accepted_shares": 1250,
      "rejected_shares": 15,
      "hardware_errors": 5,
      "difficulty": 1024.0,
      "best_share": 2048.0
    },
    "network_stats": {
      "bytes_sent": 1024000,
      "bytes_received": 2048000,
      "connection_count": 2
    }
  }
}
```

## 设备管理 API

### 获取设备列表

获取所有挖矿设备的列表。

```http
GET /api/v1/devices
```

**查询参数:**
- `status` (可选): 按状态过滤 (`active`, `idle`, `error`, `disabled`)
- `chain_id` (可选): 按链ID过滤

**响应示例:**

```json
{
  "status": "ok",
  "data": [
    {
      "id": 0,
      "name": "Chain 0",
      "device_type": "maijie_l7",
      "chain_id": 0,
      "chip_count": 76,
      "status": "mining",
      "temperature": 75.5,
      "fan_speed": 3000,
      "voltage": 850,
      "frequency": 500,
      "hashrate": 55.2,
      "accepted_shares": 625,
      "rejected_shares": 8,
      "hardware_errors": 2,
      "uptime": 3600,
      "last_share_time": "2024-01-01T12:30:00Z"
    },
    {
      "id": 1,
      "name": "Chain 1",
      "device_type": "maijie_l7",
      "chain_id": 1,
      "chip_count": 76,
      "status": "mining",
      "temperature": 73.2,
      "fan_speed": 2800,
      "voltage": 850,
      "frequency": 500,
      "hashrate": 55.3,
      "accepted_shares": 625,
      "rejected_shares": 7,
      "hardware_errors": 3,
      "uptime": 3600,
      "last_share_time": "2024-01-01T12:29:45Z"
    }
  ]
}
```

### 获取特定设备信息

获取指定设备的详细信息。

```http
GET /api/v1/devices/{device_id}
```

**路径参数:**
- `device_id`: 设备ID

**响应示例:**

```json
{
  "status": "ok",
  "data": {
    "id": 0,
    "name": "Chain 0",
    "device_type": "maijie_l7",
    "chain_id": 0,
    "chip_count": 76,
    "status": "mining",
    "temperature": 75.5,
    "fan_speed": 3000,
    "voltage": 850,
    "frequency": 500,
    "hashrate": 55.2,
    "accepted_shares": 625,
    "rejected_shares": 8,
    "hardware_errors": 2,
    "uptime": 3600,
    "last_share_time": "2024-01-01T12:30:00Z",
    "detailed_stats": {
      "total_hashes": 750000000,
      "valid_nonces": 625,
      "invalid_nonces": 8,
      "temperature_history": [74.2, 75.1, 75.5],
      "hashrate_history": [55.0, 55.1, 55.2],
      "error_rate": 1.28,
      "efficiency": 0.065
    }
  }
}
```

### 重启设备

重启指定的挖矿设备。

```http
POST /api/v1/devices/{device_id}/restart
```

**路径参数:**
- `device_id`: 设备ID

**响应示例:**

```json
{
  "status": "ok",
  "message": "Device restart initiated",
  "data": {
    "device_id": 0,
    "restart_time": "2024-01-01T12:35:00Z"
  }
}
```

### 启用/禁用设备

启用或禁用指定的挖矿设备。

```http
POST /api/v1/devices/{device_id}/enable
POST /api/v1/devices/{device_id}/disable
```

**路径参数:**
- `device_id`: 设备ID

**响应示例:**

```json
{
  "status": "ok",
  "message": "Device enabled successfully",
  "data": {
    "device_id": 0,
    "status": "enabled"
  }
}
```

### 设置设备参数

设置设备的频率、电压等参数。

```http
PUT /api/v1/devices/{device_id}/config
```

**路径参数:**
- `device_id`: 设备ID

**请求体:**

```json
{
  "frequency": 520,
  "voltage": 870,
  "fan_speed": 3200
}
```

**响应示例:**

```json
{
  "status": "ok",
  "message": "Device configuration updated",
  "data": {
    "device_id": 0,
    "updated_config": {
      "frequency": 520,
      "voltage": 870,
      "fan_speed": 3200
    }
  }
}
```

## 矿池管理 API

### 获取矿池列表

获取所有配置的矿池信息。

```http
GET /api/v1/pools
```

**响应示例:**

```json
{
  "status": "ok",
  "data": [
    {
      "id": 0,
      "url": "stratum+tcp://pool.example.com:4444",
      "user": "username",
      "priority": 1,
      "status": "connected",
      "active": true,
      "accepted_shares": 1200,
      "rejected_shares": 15,
      "last_share_time": "2024-01-01T12:30:00Z",
      "connection_time": "2024-01-01T11:00:00Z",
      "difficulty": 1024.0
    },
    {
      "id": 1,
      "url": "stratum+tcp://backup.example.com:4444",
      "user": "username",
      "priority": 2,
      "status": "standby",
      "active": false,
      "accepted_shares": 0,
      "rejected_shares": 0,
      "last_share_time": null,
      "connection_time": null,
      "difficulty": 0.0
    }
  ]
}
```

### 切换矿池

切换到指定的矿池。

```http
POST /api/v1/pools/switch
```

**请求体:**

```json
{
  "pool_id": 1
}
```

**响应示例:**

```json
{
  "status": "ok",
  "message": "Pool switched successfully",
  "data": {
    "old_pool_id": 0,
    "new_pool_id": 1,
    "switch_time": "2024-01-01T12:35:00Z"
  }
}
```

### 添加矿池

添加新的矿池配置。

```http
POST /api/v1/pools
```

**请求体:**

```json
{
  "url": "stratum+tcp://newpool.example.com:4444",
  "user": "username",
  "password": "password",
  "priority": 3,
  "enabled": true
}
```

**响应示例:**

```json
{
  "status": "ok",
  "message": "Pool added successfully",
  "data": {
    "pool_id": 2,
    "url": "stratum+tcp://newpool.example.com:4444"
  }
}
```

## 配置管理 API

### 获取当前配置

获取当前的系统配置。

```http
GET /api/v1/config
```

**响应示例:**

```json
{
  "status": "ok",
  "data": {
    "general": {
      "log_level": "info",
      "scan_time": 30
    },
    "devices": {
      "auto_detect": true,
      "scan_interval": 5
    },
    "pools": {
      "strategy": "failover",
      "failover_timeout": 30
    },
    "api": {
      "enabled": true,
      "port": 4028
    },
    "monitoring": {
      "enabled": true,
      "metrics_interval": 30
    }
  }
}
```

### 更新配置

更新系统配置。

```http
PUT /api/v1/config
```

**请求体:**

```json
{
  "general": {
    "log_level": "debug"
  },
  "monitoring": {
    "metrics_interval": 60
  }
}
```

**响应示例:**

```json
{
  "status": "ok",
  "message": "Configuration updated successfully",
  "data": {
    "updated_fields": ["general.log_level", "monitoring.metrics_interval"],
    "restart_required": false
  }
}
```

## 监控 API

### 获取实时指标

获取系统的实时监控指标。

```http
GET /api/v1/metrics
```

**查询参数:**
- `device_id` (可选): 指定设备ID
- `duration` (可选): 时间范围，如 `1h`, `24h`, `7d`

**响应示例:**

```json
{
  "status": "ok",
  "data": {
    "timestamp": "2024-01-01T12:30:00Z",
    "system_metrics": {
      "total_hashrate": 110.5,
      "power_consumption": 3200.0,
      "efficiency": 0.0345,
      "uptime": 3600
    },
    "device_metrics": [
      {
        "device_id": 0,
        "hashrate": 55.2,
        "temperature": 75.5,
        "power_consumption": 1600.0,
        "fan_speed": 3000,
        "error_rate": 1.28
      }
    ]
  }
}
```

### 获取告警信息

获取当前的告警信息。

```http
GET /api/v1/alerts
```

**查询参数:**
- `severity` (可选): 告警级别 (`info`, `warning`, `critical`, `emergency`)
- `active_only` (可选): 只返回活跃告警 (`true`, `false`)

**响应示例:**

```json
{
  "status": "ok",
  "data": [
    {
      "id": "alert-001",
      "device_id": 0,
      "alert_type": "temperature",
      "severity": "warning",
      "message": "Device temperature is above warning threshold",
      "value": 82.5,
      "threshold": 80.0,
      "timestamp": "2024-01-01T12:25:00Z",
      "active": true
    }
  ]
}
```

## 错误响应

所有 API 在出错时都会返回统一的错误格式：

```json
{
  "status": "error",
  "error": {
    "code": "DEVICE_NOT_FOUND",
    "message": "Device with ID 999 not found",
    "details": {
      "device_id": 999,
      "available_devices": [0, 1]
    }
  }
}
```

### 常见错误代码

- `DEVICE_NOT_FOUND` - 设备未找到
- `DEVICE_OFFLINE` - 设备离线
- `INVALID_PARAMETER` - 参数无效
- `POOL_CONNECTION_FAILED` - 矿池连接失败
- `CONFIGURATION_ERROR` - 配置错误
- `AUTHENTICATION_REQUIRED` - 需要认证
- `RATE_LIMIT_EXCEEDED` - 请求频率超限
- `INTERNAL_ERROR` - 内部错误

## 状态码

- `200` - 成功
- `400` - 请求错误
- `401` - 未认证
- `403` - 权限不足
- `404` - 资源未找到
- `429` - 请求频率超限
- `500` - 服务器内部错误
- `503` - 服务不可用
