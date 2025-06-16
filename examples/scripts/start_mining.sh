#!/bin/bash

# CGMiner-RS Startup Script
# This script provides a robust way to start CGMiner-RS with proper error handling

set -euo pipefail

# Configuration
CGMINER_BIN="${CGMINER_BIN:-/usr/local/bin/cgminer-rs}"
CONFIG_FILE="${CONFIG_FILE:-/etc/cgminer-rs/config.toml}"
LOG_FILE="${LOG_FILE:-/var/log/cgminer-rs/cgminer.log}"
PID_FILE="${PID_FILE:-/var/run/cgminer-rs.pid}"
USER="${CGMINER_USER:-cgminer}"
GROUP="${CGMINER_GROUP:-cgminer}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root"
        exit 1
    fi
}

# Check if binary exists
check_binary() {
    if [[ ! -f "$CGMINER_BIN" ]]; then
        error "CGMiner-RS binary not found at $CGMINER_BIN"
        exit 1
    fi
    
    if [[ ! -x "$CGMINER_BIN" ]]; then
        error "CGMiner-RS binary is not executable"
        exit 1
    fi
}

# Check if config file exists
check_config() {
    if [[ ! -f "$CONFIG_FILE" ]]; then
        error "Configuration file not found at $CONFIG_FILE"
        exit 1
    fi
    
    # Validate configuration
    log "Validating configuration..."
    if ! sudo -u "$USER" "$CGMINER_BIN" --config "$CONFIG_FILE" --check-config; then
        error "Configuration validation failed"
        exit 1
    fi
    success "Configuration is valid"
}

# Check if already running
check_running() {
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            error "CGMiner-RS is already running (PID: $pid)"
            exit 1
        else
            warning "Stale PID file found, removing..."
            rm -f "$PID_FILE"
        fi
    fi
}

# Create necessary directories
setup_directories() {
    local log_dir=$(dirname "$LOG_FILE")
    local pid_dir=$(dirname "$PID_FILE")
    
    # Create log directory
    if [[ ! -d "$log_dir" ]]; then
        log "Creating log directory: $log_dir"
        mkdir -p "$log_dir"
        chown "$USER:$GROUP" "$log_dir"
        chmod 755 "$log_dir"
    fi
    
    # Create PID directory
    if [[ ! -d "$pid_dir" ]]; then
        log "Creating PID directory: $pid_dir"
        mkdir -p "$pid_dir"
        chown "$USER:$GROUP" "$pid_dir"
        chmod 755 "$pid_dir"
    fi
}

# Check system requirements
check_system() {
    log "Checking system requirements..."
    
    # Check available memory
    local mem_available=$(awk '/MemAvailable/ {print $2}' /proc/meminfo)
    local mem_required=102400  # 100MB in KB
    
    if [[ $mem_available -lt $mem_required ]]; then
        warning "Low memory available: ${mem_available}KB (recommended: ${mem_required}KB+)"
    fi
    
    # Check CPU load
    local load_avg=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
    local cpu_cores=$(nproc)
    local load_threshold=$(echo "$cpu_cores * 0.8" | bc -l)
    
    if (( $(echo "$load_avg > $load_threshold" | bc -l) )); then
        warning "High CPU load: $load_avg (threshold: $load_threshold)"
    fi
    
    # Check disk space
    local log_dir=$(dirname "$LOG_FILE")
    local disk_available=$(df "$log_dir" | awk 'NR==2 {print $4}')
    local disk_required=1048576  # 1GB in KB
    
    if [[ $disk_available -lt $disk_required ]]; then
        warning "Low disk space in $log_dir: ${disk_available}KB (recommended: ${disk_required}KB+)"
    fi
    
    success "System requirements check completed"
}

# Start CGMiner-RS
start_cgminer() {
    log "Starting CGMiner-RS..."
    
    # Set environment variables
    export RUST_LOG="${RUST_LOG:-info}"
    export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
    
    # Start the process
    sudo -u "$USER" nohup "$CGMINER_BIN" \
        --config "$CONFIG_FILE" \
        --daemon \
        --log-file "$LOG_FILE" \
        > /dev/null 2>&1 &
    
    local pid=$!
    echo $pid > "$PID_FILE"
    chown "$USER:$GROUP" "$PID_FILE"
    
    # Wait a moment and check if process is still running
    sleep 2
    if kill -0 "$pid" 2>/dev/null; then
        success "CGMiner-RS started successfully (PID: $pid)"
        log "Log file: $LOG_FILE"
        log "PID file: $PID_FILE"
        log "Configuration: $CONFIG_FILE"
    else
        error "CGMiner-RS failed to start"
        rm -f "$PID_FILE"
        exit 1
    fi
}

# Monitor startup
monitor_startup() {
    log "Monitoring startup process..."
    
    local max_wait=30
    local wait_time=0
    
    while [[ $wait_time -lt $max_wait ]]; do
        if curl -s -f "http://localhost:8080/api/v1/status" > /dev/null 2>&1; then
            success "CGMiner-RS API is responding"
            return 0
        fi
        
        sleep 1
        ((wait_time++))
        echo -n "."
    done
    
    echo
    warning "API not responding after ${max_wait}s, but process may still be starting"
    log "Check logs: tail -f $LOG_FILE"
}

# Show status
show_status() {
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            log "CGMiner-RS is running (PID: $pid)"
            
            # Try to get status from API
            if command -v curl >/dev/null 2>&1; then
                local status=$(curl -s "http://localhost:8080/api/v1/status" 2>/dev/null || echo "API not responding")
                if [[ "$status" != "API not responding" ]]; then
                    log "API Status: $(echo "$status" | jq -r '.data.mining_state' 2>/dev/null || echo "Unknown")"
                fi
            fi
        else
            warning "PID file exists but process is not running"
        fi
    else
        log "CGMiner-RS is not running"
    fi
}

# Main function
main() {
    log "CGMiner-RS Startup Script"
    log "========================="
    
    case "${1:-start}" in
        start)
            check_root
            check_binary
            check_config
            check_running
            setup_directories
            check_system
            start_cgminer
            monitor_startup
            show_status
            ;;
        status)
            show_status
            ;;
        check)
            check_binary
            check_config
            check_system
            success "All checks passed"
            ;;
        *)
            echo "Usage: $0 {start|status|check}"
            echo "  start  - Start CGMiner-RS with full checks"
            echo "  status - Show current status"
            echo "  check  - Run system checks without starting"
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
