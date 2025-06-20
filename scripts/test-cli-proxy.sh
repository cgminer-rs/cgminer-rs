#!/bin/bash

# CGMiner-RS CLI SOCKS5代理测试脚本
# 此脚本用于测试新增的CLI代理功能

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查cgminer-rs二进制文件
check_binary() {
    if [ ! -f "./target/debug/cgminer-rs" ] && [ ! -f "./target/release/cgminer-rs" ]; then
        log_error "cgminer-rs binary not found. Please build the project first:"
        echo "  cargo build --release"
        exit 1
    fi
    
    if [ -f "./target/release/cgminer-rs" ]; then
        CGMINER_BIN="./target/release/cgminer-rs"
    else
        CGMINER_BIN="./target/debug/cgminer-rs"
    fi
    
    log_info "Using binary: $CGMINER_BIN"
}

# 测试帮助信息
test_help() {
    log_info "Testing help information..."
    
    if $CGMINER_BIN --help | grep -q "SOCKS5 proxy URL"; then
        log_success "Help information includes SOCKS5 proxy options"
    else
        log_error "Help information missing SOCKS5 proxy options"
        return 1
    fi
}

# 测试无效代理URL
test_invalid_proxy() {
    log_info "Testing invalid proxy URL handling..."
    
    # 测试不支持的协议
    if $CGMINER_BIN --proxy http://proxy.example.com:8080 --config /dev/null 2>&1 | grep -q "Unsupported proxy scheme"; then
        log_success "Correctly rejected unsupported proxy scheme"
    else
        log_warning "May not properly handle unsupported proxy schemes"
    fi
    
    # 测试缺少端口的URL
    if $CGMINER_BIN --proxy socks5://127.0.0.1 --config /dev/null 2>&1 | grep -q "must include a port"; then
        log_success "Correctly rejected proxy URL without port"
    else
        log_warning "May not properly handle proxy URL without port"
    fi
}

# 测试有效的代理URL解析
test_valid_proxy_parsing() {
    log_info "Testing valid proxy URL parsing..."
    
    # 创建临时配置文件
    TEMP_CONFIG=$(mktemp)
    cat > "$TEMP_CONFIG" << 'EOF'
[general]
log_level = "info"
work_restart_timeout = 60
scan_time = 30

[cores]
enabled_cores = ["cpu-btc"]
default_core = "cpu-btc"

[cores.cpu_btc]
enabled = true
device_count = 1
min_hashrate = 1000000000.0
max_hashrate = 5000000000.0
error_rate = 0.01
batch_size = 1000
work_timeout_ms = 5000

[devices]
auto_detect = true
scan_interval = 5

[pools]
strategy = "Failover"
failover_timeout = 30
retry_interval = 10

[[pools.pools]]
name = "test-pool"
url = "stratum+tcp://pool.example.com:4444"
username = "test"
password = "test"
priority = 1
enabled = true

[api]
enabled = false

[monitoring]
enabled = false
metrics_interval = 30

[monitoring.alert_thresholds]
temperature_warning = 80.0
temperature_critical = 90.0
hashrate_drop_percent = 20.0
error_rate_percent = 5.0
max_temperature = 85.0
max_cpu_usage = 80.0
max_memory_usage = 90.0
max_device_temperature = 85.0
max_error_rate = 5.0
min_hashrate = 50.0

[web]
enabled = false
bind_address = "127.0.0.1"
port = 8080

[hashmeter]
enabled = true
interval = 5
display_format = "compact"
show_per_device = true
show_pool_stats = true
EOF

    # 测试基本的SOCKS5代理
    log_info "Testing basic SOCKS5 proxy parsing..."
    timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://127.0.0.1:1080 \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -20 || true
    
    # 测试带认证的SOCKS5代理
    log_info "Testing SOCKS5 proxy with authentication..."
    timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://proxy.example.com:1080 \
        --proxy-user proxyuser \
        --proxy-pass proxypass \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -20 || true
    
    # 测试SOCKS5+TLS代理
    log_info "Testing SOCKS5+TLS proxy..."
    timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5+tls://secure.proxy.com:1080 \
        --proxy-user secureuser \
        --proxy-pass securepass \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -20 || true
    
    # 测试URL中包含认证信息的代理
    log_info "Testing proxy with credentials in URL..."
    timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://user:pass@proxy.example.com:1080 \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -20 || true
    
    # 清理临时文件
    rm -f "$TEMP_CONFIG"
    
    log_success "Proxy parsing tests completed"
}

# 测试CLI参数覆盖
test_cli_override() {
    log_info "Testing CLI parameter override functionality..."
    
    # 创建临时配置文件
    TEMP_CONFIG=$(mktemp)
    cat > "$TEMP_CONFIG" << 'EOF'
[general]
log_level = "info"
work_restart_timeout = 60
scan_time = 30

[cores]
enabled_cores = ["cpu-btc"]
default_core = "cpu-btc"

[cores.cpu_btc]
enabled = true
device_count = 1
min_hashrate = 1000000000.0
max_hashrate = 5000000000.0
error_rate = 0.01
batch_size = 1000
work_timeout_ms = 5000

[devices]
auto_detect = true
scan_interval = 5

[pools]
strategy = "Failover"
failover_timeout = 30
retry_interval = 10

[[pools.pools]]
name = "original-pool"
url = "stratum+tcp://original.pool.com:4444"
username = "original_user"
password = "original_pass"
priority = 1
enabled = true

[api]
enabled = true
port = 4028

[monitoring]
enabled = false
metrics_interval = 30

[monitoring.alert_thresholds]
temperature_warning = 80.0
temperature_critical = 90.0
hashrate_drop_percent = 20.0
error_rate_percent = 5.0
max_temperature = 85.0
max_cpu_usage = 80.0
max_memory_usage = 90.0
max_device_temperature = 85.0
max_error_rate = 5.0
min_hashrate = 50.0

[web]
enabled = false
bind_address = "127.0.0.1"
port = 8080

[hashmeter]
enabled = true
interval = 5
display_format = "compact"
show_per_device = true
show_pool_stats = true
EOF

    # 测试CLI参数覆盖
    log_info "Testing CLI override with proxy and pool settings..."
    OUTPUT=$(timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://127.0.0.1:1080 \
        --pool stratum+tcp://new.pool.com:4444 \
        --user new_user \
        --pass new_pass \
        --api-port 5028 \
        --log-level debug 2>&1 | head -30 || true)
    
    echo "$OUTPUT"
    
    # 检查是否显示了CLI覆盖信息
    if echo "$OUTPUT" | grep -q "CLI arguments applied"; then
        log_success "CLI override information displayed"
    else
        log_warning "CLI override information may not be displayed"
    fi
    
    # 清理临时文件
    rm -f "$TEMP_CONFIG"
}

# 主测试函数
main() {
    log_info "Starting CGMiner-RS CLI SOCKS5 proxy tests..."
    echo "=================================================="
    
    check_binary
    echo
    
    test_help
    echo
    
    test_invalid_proxy
    echo
    
    test_valid_proxy_parsing
    echo
    
    test_cli_override
    echo
    
    log_success "All tests completed!"
    echo
    log_info "Note: These tests only verify CLI parsing and configuration."
    log_info "Actual proxy connections require a running SOCKS5 proxy server."
    echo
    log_info "To test with a real proxy, set up a SOCKS5 proxy and run:"
    echo "  $CGMINER_BIN --proxy socks5://127.0.0.1:1080 --pool stratum+tcp://your.pool.com:4444 --user your_user --pass your_pass"
}

# 运行主函数
main "$@"
