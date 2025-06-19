#!/bin/bash

# CGMiner-RS 软核配置助手
# 帮助用户根据系统配置生成最优的软核配置文件

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo -e "${PURPLE}$1${NC}"
}

# 检测系统信息
detect_system_info() {
    print_header "🔍 检测系统信息..."
    
    # 检测CPU信息
    if command -v nproc >/dev/null 2>&1; then
        CPU_CORES=$(nproc)
    elif command -v sysctl >/dev/null 2>&1; then
        CPU_CORES=$(sysctl -n hw.ncpu)
    else
        CPU_CORES=4
        print_warning "无法检测CPU核心数，使用默认值: 4"
    fi
    
    # 检测物理CPU核心数
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        PHYSICAL_CORES=$(lscpu | grep "Core(s) per socket" | awk '{print $4}' || echo $CPU_CORES)
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        PHYSICAL_CORES=$(sysctl -n hw.physicalcpu || echo $CPU_CORES)
    else
        PHYSICAL_CORES=$CPU_CORES
    fi
    
    # 检测内存
    if command -v free >/dev/null 2>&1; then
        MEMORY_GB=$(free -g | awk '/^Mem:/{print $2}')
    elif command -v vm_stat >/dev/null 2>&1; then
        MEMORY_BYTES=$(sysctl -n hw.memsize)
        MEMORY_GB=$((MEMORY_BYTES / 1024 / 1024 / 1024))
    else
        MEMORY_GB=8
        print_warning "无法检测内存大小，使用默认值: 8GB"
    fi
    
    # 检测操作系统
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS_TYPE="Linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS_TYPE="macOS"
    else
        OS_TYPE="Unknown"
    fi
    
    print_info "操作系统: $OS_TYPE"
    print_info "CPU核心数: $CPU_CORES (物理核心: $PHYSICAL_CORES)"
    print_info "内存大小: ${MEMORY_GB}GB"
}

# 获取用户配置偏好
get_user_preferences() {
    print_header "⚙️  配置偏好设置..."
    
    echo "请选择使用场景:"
    echo "1) 最大化CPU使用 (专用挖矿机器)"
    echo "2) 限制CPU使用 (共享服务器)"
    echo "3) 平衡配置 (推荐)"
    echo "4) 自定义配置"
    
    read -p "请输入选择 (1-4) [默认: 3]: " SCENARIO
    SCENARIO=${SCENARIO:-3}
    
    case $SCENARIO in
        1)
            SCENARIO_NAME="最大化CPU使用"
            CONFIG_TYPE="max_cpu"
            ;;
        2)
            SCENARIO_NAME="限制CPU使用"
            CONFIG_TYPE="limited_cpu"
            ;;
        3)
            SCENARIO_NAME="平衡配置"
            CONFIG_TYPE="balanced"
            ;;
        4)
            SCENARIO_NAME="自定义配置"
            CONFIG_TYPE="custom"
            ;;
        *)
            print_error "无效选择，使用默认平衡配置"
            SCENARIO_NAME="平衡配置"
            CONFIG_TYPE="balanced"
            ;;
    esac
    
    print_success "选择场景: $SCENARIO_NAME"
    
    # 获取目标CPU使用率
    if [[ "$CONFIG_TYPE" == "custom" ]]; then
        read -p "目标CPU使用率 (10-95%) [默认: 70]: " TARGET_CPU
        TARGET_CPU=${TARGET_CPU:-70}
        
        read -p "设备数量 (1-64) [默认: $CPU_CORES]: " DEVICE_COUNT
        DEVICE_COUNT=${DEVICE_COUNT:-$CPU_CORES}
    else
        case $CONFIG_TYPE in
            "max_cpu")
                TARGET_CPU=90
                DEVICE_COUNT=$((CPU_CORES * 3))
                ;;
            "limited_cpu")
                TARGET_CPU=40
                DEVICE_COUNT=$((CPU_CORES / 2))
                ;;
            "balanced")
                TARGET_CPU=70
                DEVICE_COUNT=$((CPU_CORES * 2))
                ;;
        esac
    fi
    
    # 限制设备数量范围
    if [[ $DEVICE_COUNT -lt 1 ]]; then
        DEVICE_COUNT=1
    elif [[ $DEVICE_COUNT -gt 64 ]]; then
        DEVICE_COUNT=64
    fi
    
    print_info "目标CPU使用率: ${TARGET_CPU}%"
    print_info "设备数量: $DEVICE_COUNT"
}

# 生成配置文件
generate_config() {
    print_header "📝 生成配置文件..."
    
    CONFIG_FILE="cgminer-${CONFIG_TYPE}-$(date +%Y%m%d-%H%M%S).toml"
    
    # 计算相关参数
    MIN_HASHRATE=$((500000000 * DEVICE_COUNT / 8))  # 基于设备数量调整
    MAX_HASHRATE=$((2000000000 * DEVICE_COUNT / 8))
    BATCH_SIZE=$((1000 + DEVICE_COUNT * 50))
    
    # 选择CPU绑定策略
    if [[ $CPU_CORES -ge 16 ]]; then
        CPU_STRATEGY="intelligent"
    elif [[ $CPU_CORES -ge 8 ]]; then
        CPU_STRATEGY="round_robin"
    else
        CPU_STRATEGY="manual"
    fi
    
    # 生成配置文件内容
    cat > "$CONFIG_FILE" << EOF
# CGMiner-RS 软核配置 - $SCENARIO_NAME
# 自动生成于: $(date)
# 系统信息: $OS_TYPE, ${CPU_CORES}核心, ${MEMORY_GB}GB内存

[general]
log_level = "info"
log_file = "logs/cgminer-${CONFIG_TYPE}.log"
api_port = 4028
api_bind = "0.0.0.0"

[cores]
enabled_cores = ["btc-software"]
default_core = "btc-software"

# Bitcoin软算法核心配置
[cores.btc_software]
enabled = true
device_count = $DEVICE_COUNT
min_hashrate = ${MIN_HASHRATE}.0
max_hashrate = ${MAX_HASHRATE}.0
error_rate = 0.005
batch_size = $BATCH_SIZE
work_timeout_ms = 3000

# CPU绑定配置
[cores.btc_software.cpu_affinity]
enabled = true
strategy = "$CPU_STRATEGY"
avoid_hyperthreading = false
prefer_performance_cores = true

# 禁用ASIC核心
[cores.maijie_l7]
enabled = false

# 矿池配置
[pools]
strategy = "LoadBalance"
failover_timeout = 30
retry_interval = 15

[[pools.pools]]
name = "f2pool"
url = "stratum+tcp://btc.f2pool.com:1314"
username = "kayuii.bbt"
password = "x"
priority = 1
enabled = true

[[pools.pools]]
name = "f2pool-backup"
url = "stratum+tcp://btc-asia.f2pool.com:1314"
username = "kayuii.bbt"
password = "x"
priority = 2
enabled = true

# 系统资源限制
[limits]
max_memory_mb = $((MEMORY_GB * 1024 / 2))
max_cpu_percent = $TARGET_CPU
max_open_files = 4096
max_network_connections = 50

# 监控配置
[monitoring]
enabled = true
metrics_interval = 30
web_port = 8080

[monitoring.thresholds]
max_temperature = 80.0
max_cpu_usage = $((TARGET_CPU + 5)).0
max_memory_usage = 80.0
max_device_temperature = 80.0
max_error_rate = 3.0
min_hashrate = $((DEVICE_COUNT * 500000000 / 1000000000)).0

# 算力计量器配置
[hashmeter]
enabled = true
log_interval = 30
per_device_stats = false
console_output = true
beautiful_output = true
hashrate_unit = "GH"

# Web界面配置
[web]
enabled = true
port = 8080
bind_address = "0.0.0.0"
static_files_dir = "web/static"
template_dir = "web/templates"

# 日志配置
[logging]
level = "info"
file = "logs/cgminer-${CONFIG_TYPE}.log"
max_size = "200MB"
max_files = 3
console = true
json_format = false
rotation = "daily"
EOF

    print_success "配置文件已生成: $CONFIG_FILE"
}

# 显示使用建议
show_recommendations() {
    print_header "💡 使用建议:"
    
    echo "1. 启动挖矿:"
    echo "   ./target/release/cgminer-rs --config $CONFIG_FILE"
    echo ""
    
    echo "2. 监控CPU使用率:"
    echo "   ./target/release/cpu_optimizer $TARGET_CPU $((DEVICE_COUNT/2)) $((DEVICE_COUNT*2)) $DEVICE_COUNT"
    echo ""
    
    echo "3. 查看实时状态:"
    echo "   curl http://localhost:4028/api/summary"
    echo ""
    
    echo "4. Web界面:"
    echo "   http://localhost:8080"
    echo ""
    
    if [[ $TARGET_CPU -gt 80 ]]; then
        print_warning "高CPU使用率配置，请确保系统散热良好"
    fi
    
    if [[ $DEVICE_COUNT -gt $((CPU_CORES * 2)) ]]; then
        print_warning "设备数量较多，可能会导致线程竞争"
    fi
    
    print_info "配置文件位置: $(pwd)/$CONFIG_FILE"
}

# 主函数
main() {
    clear
    print_header "🚀 CGMiner-RS 软核配置助手"
    print_header "================================"
    echo ""
    
    detect_system_info
    echo ""
    
    get_user_preferences
    echo ""
    
    generate_config
    echo ""
    
    show_recommendations
    echo ""
    
    print_success "配置完成！"
}

# 运行主函数
main "$@"
