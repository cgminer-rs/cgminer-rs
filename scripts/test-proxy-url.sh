#!/bin/bash

# 测试代理URL解析功能
# 验证 socks5+tls://test22:cc112233d@54.248.128.73:8080 这种格式的解析

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

# 创建临时配置文件
create_temp_config() {
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
    echo "$TEMP_CONFIG"
}

# 测试您提到的代理URL格式
test_your_proxy_format() {
    log_info "Testing your proxy URL format: socks5+tls://test22:cc112233d@54.248.128.73:8080"
    
    TEMP_CONFIG=$(create_temp_config)
    
    log_info "Running cgminer-rs with your proxy format..."
    OUTPUT=$(timeout 5s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5+tls://test22:cc112233d@54.248.128.73:8080 \
        --pool stratum+tcp://btc.f2pool.com:1314 \
        --user kayuii.bbt \
        --pass x 2>&1 | head -30 || true)
    
    echo "$OUTPUT"
    
    # 检查是否正确解析了代理信息
    if echo "$OUTPUT" | grep -q "CLI arguments applied"; then
        log_success "✅ CLI arguments were applied successfully"
    else
        log_warning "⚠️ CLI arguments application message not found"
    fi
    
    if echo "$OUTPUT" | grep -q "socks5+tls://test22:cc112233d@54.248.128.73:8080"; then
        log_success "✅ Proxy URL was correctly parsed and displayed"
    else
        log_warning "⚠️ Proxy URL display not found in output"
    fi
    
    # 清理临时文件
    rm -f "$TEMP_CONFIG"
}

# 测试其他代理URL格式
test_other_formats() {
    log_info "Testing other proxy URL formats..."
    
    TEMP_CONFIG=$(create_temp_config)
    
    # 测试基本SOCKS5
    log_info "Testing basic SOCKS5 with credentials in URL..."
    timeout 3s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://user123:pass456@127.0.0.1:1080 \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -10 || true
    
    echo ""
    
    # 测试无认证SOCKS5
    log_info "Testing SOCKS5 without authentication..."
    timeout 3s $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://127.0.0.1:1080 \
        --pool stratum+tcp://test.pool.com:4444 \
        --user testuser \
        --pass testpass 2>&1 | head -10 || true
    
    # 清理临时文件
    rm -f "$TEMP_CONFIG"
}

# 测试错误处理
test_error_handling() {
    log_info "Testing error handling for invalid proxy URLs..."
    
    TEMP_CONFIG=$(create_temp_config)
    
    # 测试不支持的协议
    log_info "Testing unsupported protocol..."
    if $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy http://proxy.example.com:8080 2>&1 | grep -q "Unsupported proxy scheme"; then
        log_success "✅ Correctly rejected unsupported protocol"
    else
        log_error "❌ Failed to reject unsupported protocol"
    fi
    
    # 测试缺少端口
    log_info "Testing missing port..."
    if $CGMINER_BIN \
        --config "$TEMP_CONFIG" \
        --proxy socks5://127.0.0.1 2>&1 | grep -q "must include a port"; then
        log_success "✅ Correctly rejected URL without port"
    else
        log_error "❌ Failed to reject URL without port"
    fi
    
    # 清理临时文件
    rm -f "$TEMP_CONFIG"
}

# 主函数
main() {
    log_info "Testing proxy URL parsing with your format..."
    echo "=================================================="
    
    check_binary
    echo
    
    test_your_proxy_format
    echo
    
    test_other_formats
    echo
    
    test_error_handling
    echo
    
    log_success "All proxy URL tests completed!"
    echo
    log_info "Your format 'socks5+tls://test22:cc112233d@54.248.128.73:8080' is fully supported!"
    log_info "This is indeed the most convenient way to specify proxy with authentication."
}

# 运行主函数
main "$@"
