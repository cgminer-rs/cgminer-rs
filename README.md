# CGMiner-RS

[![Build Status](https://github.com/cgminer-rs/cgminer-rs/workflows/CI/badge.svg)](https://github.com/cgminer-rs/cgminer-rs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/docker/v/cgminer-rs/cgminer-rs?label=docker)](https://hub.docker.com/r/cgminer-rs/cgminer-rs)

A high-performance ASIC Bitcoin miner written in Rust, designed for modern mining hardware with advanced monitoring, management, and optimization capabilities.

## Features

### 🚀 Performance
- **High-throughput mining**: Optimized for maximum hash rate
- **Low latency**: Minimal work distribution and result collection overhead
- **Efficient memory usage**: Zero-copy operations where possible
- **Multi-threaded**: Concurrent device management and work processing

### 🔧 Hardware Support
- **Maijie L7**: Native support for Maijie L7 ASIC miners
- **Extensible drivers**: Plugin architecture for additional hardware
- **Auto-detection**: Automatic device discovery and configuration
- **Hot-plugging**: Dynamic device addition and removal

### 📊 Monitoring & Management
- **Real-time metrics**: System, device, and pool statistics
- **Web dashboard**: Modern web interface for monitoring and control
- **REST API**: Comprehensive API for integration and automation
- **WebSocket**: Real-time updates and notifications
- **Alerting**: Configurable alerts for temperature, errors, and performance

### 🌐 Pool Management
- **Multiple pools**: Support for multiple mining pools with failover
- **Stratum protocol**: Full Stratum v1 implementation
- **Load balancing**: Intelligent work distribution across pools
- **Connection management**: Automatic reconnection and health monitoring

### 🛡️ Reliability
- **Error handling**: Comprehensive error detection and recovery
- **Health monitoring**: Continuous device and system health checks
- **Automatic recovery**: Self-healing capabilities for common issues
- **Logging**: Detailed logging with configurable levels

## Quick Start

### Prerequisites

- Rust 1.75 or later
- Linux (Ubuntu 20.04+, Debian 11+, or similar)
- Root access for hardware control

### Installation

#### From Source

```bash
# Clone the repository
git clone https://github.com/cgminer-rs/cgminer-rs.git
cd cgminer-rs

# Build and install
make install
```

#### Using Docker

```bash
# Pull and run the latest image
docker run -d --name cgminer-rs \
  --privileged \
  -p 8080:8080 \
  -v /path/to/config.toml:/etc/cgminer-rs/config.toml \
  cgminer-rs/cgminer-rs:latest
```

#### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/cgminer-rs/cgminer-rs/releases).

### Configuration

Create a configuration file:

```bash
# Generate example configuration
cgminer-rs --generate-config > config.toml

# Edit the configuration
nano config.toml
```

Example configuration:

```toml
[mining]
scan_interval = 5
enable_auto_tuning = true

[[devices.chains]]
id = 0
enabled = true
frequency = 500
voltage = 850
chip_count = 76

[[pools.pools]]
url = "stratum+tcp://pool.example.com:4444"
user = "your_username"
password = "your_password"
priority = 1

[api]
enabled = true
bind_address = "0.0.0.0"
port = 8080
```

### Running

```bash
# Start mining
cgminer-rs --config config.toml

# Run in daemon mode
cgminer-rs --config config.toml --daemon

# Enable verbose logging
cgminer-rs --config config.toml --verbose
```

## Usage

### Web Interface

Access the web dashboard at `http://localhost:8080` to:

- Monitor real-time mining statistics
- View device status and health
- Configure mining parameters
- Manage pool connections
- View logs and alerts

### API

The REST API provides programmatic access to all functionality:

```bash
# Get system status
curl http://localhost:8080/api/v1/status

# Get device information
curl http://localhost:8080/api/v1/devices

# Restart a device
curl -X POST http://localhost:8080/api/v1/devices/0/restart

# Update configuration
curl -X PUT http://localhost:8080/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{"frequency": 550}'
```

### Command Line

```bash
# Check configuration
cgminer-rs --config config.toml --check-config

# Scan for devices
cgminer-rs --scan-devices

# Run benchmarks
cgminer-rs --benchmark

# Show version information
cgminer-rs --version
```

## Development

### Building from Source

```bash
# Install dependencies
make setup-dev

# Build debug version
make build

# Build release version
make release

# Run tests
make test

# Run benchmarks
make bench

# Generate documentation
make docs
```

### Cross Compilation

```bash
# Build for ARM64
make cross-compile-aarch64

# Build for ARMv7
make cross-compile-armv7

# Build for all targets
make cross-compile
```

### Docker Development

```bash
# Start development environment
docker-compose --profile dev up

# Run tests in container
docker-compose run cgminer-rs-dev cargo test

# Build production image
docker-compose build cgminer-rs
```

## Architecture

CGMiner-RS is built with a modular architecture:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web UI        │    │   REST API      │    │   WebSocket     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
┌─────────────────────────────────────────────────────────────────┐
│                        API Layer                                │
└─────────────────────────────────────────────────────────────────┘
         │
┌─────────────────────────────────────────────────────────────────┐
│                    Mining Manager                               │
└─────────────────────────────────────────────────────────────────┘
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Device Manager  │    │  Pool Manager   │    │ Monitor System  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│    Drivers      │    │    Stratum      │    │    Metrics      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │
┌─────────────────────────────────────────────────────────────────┐
│                      Hardware Layer                             │
└─────────────────────────────────────────────────────────────────┘
```

## Performance

CGMiner-RS is optimized for high performance:

- **Zero-copy operations**: Minimal memory allocation and copying
- **Async I/O**: Non-blocking operations for maximum throughput
- **SIMD optimizations**: Vectorized operations where applicable
- **Lock-free data structures**: Reduced contention in multi-threaded scenarios

### Benchmarks

On a typical mining setup:

- **Latency**: < 1ms work distribution
- **Throughput**: > 10,000 jobs/second
- **Memory usage**: < 50MB base memory
- **CPU usage**: < 5% on modern hardware

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use Clippy for linting (`cargo clippy`)
- Write comprehensive tests
- Document public APIs

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Documentation**: [docs.cgminer-rs.org](https://docs.cgminer-rs.org)
- **Issues**: [GitHub Issues](https://github.com/cgminer-rs/cgminer-rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/cgminer-rs/cgminer-rs/discussions)
- **Discord**: [CGMiner-RS Community](https://discord.gg/cgminer-rs)

## Acknowledgments

- Original CGMiner project for inspiration
- Rust community for excellent tooling and libraries
- Mining hardware manufacturers for documentation and support

## Roadmap

- [ ] Additional hardware driver support
- [ ] Advanced auto-tuning algorithms
- [ ] Machine learning-based optimization
- [ ] Stratum v2 protocol support
- [ ] Mobile monitoring app
- [ ] Cloud management platform

---

**Disclaimer**: This software is provided as-is. Mining cryptocurrency involves risks including hardware damage, electrical hazards, and financial loss. Use at your own risk and ensure proper safety measures.
