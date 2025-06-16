# CGMiner-RS Configuration Guide

This guide covers all configuration options for CGMiner-RS, including mining settings, device configuration, pool management, and monitoring.

## Configuration File Format

CGMiner-RS uses TOML format for configuration. The default configuration file is `config.toml`.

## Complete Configuration Example

```toml
# CGMiner-RS Configuration File

[mining]
# Scan interval for new work (seconds)
scan_interval = 5

# Work restart timeout (seconds)
work_restart_timeout = 60

# Enable automatic tuning
enable_auto_tuning = true

[devices]
# Device scan interval (seconds)
scan_interval = 10

# Chain configurations
[[devices.chains]]
id = 0
enabled = true
frequency = 500        # MHz
voltage = 850         # mV
auto_tune = true
chip_count = 76

[[devices.chains]]
id = 1
enabled = true
frequency = 500
voltage = 850
auto_tune = true
chip_count = 76

[pools]
# Pool selection strategy: "Failover", "RoundRobin", "LoadBalance", "Quota"
strategy = "Failover"

# Retry interval for failed connections (seconds)
retry_interval = 30

# Pool configurations
[[pools.pools]]
url = "stratum+tcp://pool.example.com:4444"
user = "your_username.worker1"
password = "your_password"
priority = 1

[[pools.pools]]
url = "stratum+tcp://backup.example.com:4444"
user = "your_username.worker1"
password = "your_password"
priority = 2

[api]
# Enable REST API
enabled = true

# Bind address
bind_address = "127.0.0.1"

# Port number
port = 8080

# Authentication token (optional)
auth_token = "your_secret_token"

# Allowed origins for CORS
allow_origins = ["*"]

[monitoring]
# Enable monitoring system
enabled = true

# Metrics collection interval (seconds)
metrics_interval = 30

# Alert thresholds
[monitoring.alert_thresholds]
max_temperature = 85.0          # °C
max_device_temperature = 90.0   # °C
max_cpu_usage = 90             # %
max_memory_usage = 90          # %
max_error_rate = 5.0           # %
min_hashrate = 30.0            # GH/s
```

## Section Details

### Mining Configuration

```toml
[mining]
scan_interval = 5              # How often to check for new work (seconds)
work_restart_timeout = 60      # Timeout for work restart (seconds)
enable_auto_tuning = true      # Enable automatic frequency/voltage tuning
```

**Options:**
- `scan_interval`: Controls how frequently the system checks for new work. Lower values reduce latency but increase CPU usage.
- `work_restart_timeout`: Maximum time to wait for work restart before timing out.
- `enable_auto_tuning`: Enables automatic optimization of device parameters for maximum efficiency.

### Device Configuration

```toml
[devices]
scan_interval = 10             # Device scan interval (seconds)

[[devices.chains]]
id = 0                         # Chain identifier (0-based)
enabled = true                 # Enable this chain
frequency = 500                # Operating frequency in MHz
voltage = 850                  # Operating voltage in mV
auto_tune = true               # Enable auto-tuning for this chain
chip_count = 76                # Number of chips on this chain
```

**Chain Parameters:**
- `id`: Unique identifier for the chain (must be sequential starting from 0)
- `enabled`: Whether this chain should be used for mining
- `frequency`: Operating frequency in MHz (typically 400-600 for Maijie L7)
- `voltage`: Operating voltage in mV (typically 800-900 for Maijie L7)
- `auto_tune`: Enable automatic parameter optimization
- `chip_count`: Number of ASIC chips on the chain

**Frequency Guidelines:**
- **Conservative**: 450-500 MHz (stable, lower power)
- **Balanced**: 500-550 MHz (good performance/efficiency)
- **Aggressive**: 550-600 MHz (maximum performance, higher power)

**Voltage Guidelines:**
- **Low**: 800-850 mV (power efficient, may need lower frequency)
- **Medium**: 850-900 mV (balanced)
- **High**: 900-950 mV (for high frequencies, higher power consumption)

### Pool Configuration

```toml
[pools]
strategy = "Failover"          # Pool selection strategy
retry_interval = 30            # Retry interval for failed connections

[[pools.pools]]
url = "stratum+tcp://pool.example.com:4444"
user = "username.worker"
password = "password"
priority = 1                   # Lower number = higher priority
```

**Pool Strategies:**
- `Failover`: Use primary pool, switch to backup on failure
- `RoundRobin`: Rotate between pools evenly
- `LoadBalance`: Distribute work based on pool performance
- `Quota`: Allocate specific percentages to each pool

**Pool Parameters:**
- `url`: Stratum URL (format: `stratum+tcp://host:port`)
- `user`: Username (often includes worker name: `username.worker`)
- `password`: Password (can be "x" for many pools)
- `priority`: Pool priority (1 = highest priority)

### API Configuration

```toml
[api]
enabled = true                 # Enable REST API
bind_address = "127.0.0.1"     # Bind to specific address
port = 8080                    # Port number
auth_token = "secret"          # Optional authentication token
allow_origins = ["*"]          # CORS allowed origins
```

**Security Considerations:**
- Use `127.0.0.1` to restrict access to localhost only
- Use `0.0.0.0` to allow access from any IP (less secure)
- Always set `auth_token` for production deployments
- Restrict `allow_origins` to specific domains in production

### Monitoring Configuration

```toml
[monitoring]
enabled = true                 # Enable monitoring
metrics_interval = 30          # Collection interval (seconds)

[monitoring.alert_thresholds]
max_temperature = 85.0         # System temperature limit (°C)
max_device_temperature = 90.0  # Device temperature limit (°C)
max_cpu_usage = 90            # CPU usage limit (%)
max_memory_usage = 90         # Memory usage limit (%)
max_error_rate = 5.0          # Error rate limit (%)
min_hashrate = 30.0           # Minimum hashrate (GH/s)
```

## Environment Variables

Configuration can be overridden using environment variables:

```bash
# API configuration
export CGMINER_API_PORT=9090
export CGMINER_API_BIND_ADDRESS=0.0.0.0
export CGMINER_API_AUTH_TOKEN=my_secret_token

# Mining configuration
export CGMINER_MINING_SCAN_INTERVAL=10
export CGMINER_MINING_AUTO_TUNING=false

# Device configuration
export CGMINER_DEVICE_0_FREQUENCY=550
export CGMINER_DEVICE_0_VOLTAGE=900
export CGMINER_DEVICE_0_ENABLED=true

# Pool configuration
export CGMINER_POOL_0_URL=stratum+tcp://mypool.com:4444
export CGMINER_POOL_0_USER=myuser.worker1
export CGMINER_POOL_0_PASSWORD=mypassword
```

## Configuration Validation

CGMiner-RS validates configuration on startup. Common validation errors:

### Device Configuration Errors
- **Invalid frequency range**: Must be between 100-800 MHz
- **Invalid voltage range**: Must be between 600-1000 mV
- **Invalid chip count**: Must be between 1-200
- **Duplicate chain IDs**: Each chain must have a unique ID

### Pool Configuration Errors
- **Invalid URL format**: Must start with `stratum+tcp://`
- **Empty username**: Username cannot be empty
- **Invalid priority**: Must be between 1-10
- **No enabled pools**: At least one pool must be configured

### API Configuration Errors
- **Invalid port**: Must be between 1-65535
- **Invalid bind address**: Must be a valid IP address
- **Invalid CORS origins**: Must be valid URLs or "*"

## Performance Tuning

### Optimal Settings for Different Scenarios

#### Maximum Hashrate
```toml
[[devices.chains]]
frequency = 580
voltage = 920
auto_tune = false
```

#### Power Efficiency
```toml
[[devices.chains]]
frequency = 480
voltage = 830
auto_tune = true
```

#### Stability (24/7 Operation)
```toml
[[devices.chains]]
frequency = 500
voltage = 850
auto_tune = true
```

### Auto-Tuning Parameters

When `auto_tune = true`, CGMiner-RS automatically adjusts:
- Frequency based on error rate and temperature
- Voltage based on stability requirements
- Fan speed based on temperature

Auto-tuning algorithm:
1. Start with configured values
2. Monitor error rate and temperature
3. Adjust frequency ±10 MHz based on performance
4. Adjust voltage ±25 mV based on stability
5. Repeat every 10 minutes

## Troubleshooting

### Common Configuration Issues

#### High Error Rate
```toml
# Reduce frequency or increase voltage
frequency = 480  # Reduce from 520
voltage = 870    # Increase from 850
```

#### High Temperature
```toml
# Reduce frequency and voltage
frequency = 460
voltage = 830
```

#### Low Hashrate
```toml
# Increase frequency (if temperature allows)
frequency = 540
voltage = 880
```

#### Pool Connection Issues
```toml
# Increase retry interval
retry_interval = 60

# Add backup pools
[[pools.pools]]
url = "stratum+tcp://backup1.example.com:4444"
priority = 2

[[pools.pools]]
url = "stratum+tcp://backup2.example.com:4444"
priority = 3
```

### Configuration Testing

Test configuration without starting mining:

```bash
# Validate configuration
cgminer-rs --config config.toml --check-config

# Test device detection
cgminer-rs --config config.toml --scan-devices

# Test pool connections
cgminer-rs --config config.toml --test-pools
```

## Best Practices

1. **Start Conservative**: Begin with lower frequencies and voltages
2. **Monitor Temperature**: Keep device temperature below 85°C
3. **Use Auto-Tuning**: Enable auto-tuning for optimal performance
4. **Multiple Pools**: Configure at least 2 pools for redundancy
5. **Regular Monitoring**: Check logs and metrics regularly
6. **Backup Configuration**: Keep backup copies of working configurations
7. **Gradual Changes**: Make small incremental changes to parameters
8. **Test Thoroughly**: Test configuration changes under load

## Configuration Templates

### Home Mining Setup
```toml
[mining]
scan_interval = 5
enable_auto_tuning = true

[[devices.chains]]
id = 0
frequency = 500
voltage = 850
auto_tune = true

[pools]
strategy = "Failover"

[[pools.pools]]
url = "stratum+tcp://pool.example.com:4444"
user = "username.home"
priority = 1

[api]
enabled = true
bind_address = "127.0.0.1"
port = 8080
```

### Industrial Mining Farm
```toml
[mining]
scan_interval = 3
enable_auto_tuning = true

# Multiple chains
[[devices.chains]]
id = 0
frequency = 520
voltage = 860

[[devices.chains]]
id = 1
frequency = 520
voltage = 860

[pools]
strategy = "LoadBalance"

# Multiple pools for redundancy
[[pools.pools]]
url = "stratum+tcp://primary.pool.com:4444"
priority = 1

[[pools.pools]]
url = "stratum+tcp://backup.pool.com:4444"
priority = 2

[api]
enabled = true
bind_address = "0.0.0.0"
port = 8080
auth_token = "secure_token_here"

[monitoring]
enabled = true
metrics_interval = 15
```
