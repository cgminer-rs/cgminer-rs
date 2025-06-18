#!/bin/bash

# Stratum 协议测试脚本
# 用于快速测试 Stratum 连接和协议兼容性

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示帮助信息
show_help() {
    cat << EOF
Stratum 协议测试脚本

用法: $0 [选项] [URL]

选项:
    -h, --help          显示此帮助信息
    -v, --verbose       显示详细输出
    -t, --timeout SEC   设置连接超时时间（默认: 10秒）
    -u, --username USER 设置测试用户名（默认: test.worker）
    -p, --password PASS 设置测试密码（默认: x）
    --build             编译测试工具（如果需要）
    --batch             批量测试预定义的矿池列表

示例:
    $0                                          # 测试默认地址
    $0 stratum+tcp://192.168.18.240:10203      # 测试指定地址
    $0 --verbose --timeout 30 btc.f2pool.com:1314  # 详细输出，30秒超时
    $0 --batch                                  # 批量测试多个矿池

EOF
}

# 默认参数
DEFAULT_URL="stratum+tcp://192.168.18.240:10203"
VERBOSE=false
TIMEOUT=10
USERNAME="test.worker"
PASSWORD="x"
BUILD=false
BATCH=false
URL=""

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -t|--timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        -u|--username)
            USERNAME="$2"
            shift 2
            ;;
        -p|--password)
            PASSWORD="$2"
            shift 2
            ;;
        --build)
            BUILD=true
            shift
            ;;
        --batch)
            BATCH=true
            shift
            ;;
        -*)
            error "未知选项: $1"
            show_help
            exit 1
            ;;
        *)
            if [[ -z "$URL" ]]; then
                URL="$1"
                # 如果没有协议前缀，自动添加
                if [[ ! "$URL" =~ ^stratum\+tcp:// ]]; then
                    URL="stratum+tcp://$URL"
                fi
            else
                error "只能指定一个 URL"
                exit 1
            fi
            shift
            ;;
    esac
done

# 如果没有指定 URL，使用默认值
if [[ -z "$URL" && "$BATCH" == false ]]; then
    URL="$DEFAULT_URL"
fi

# 检查是否在项目根目录
if [[ ! -f "Cargo.toml" ]]; then
    error "请在项目根目录运行此脚本"
    exit 1
fi

# 编译测试工具（如果需要）
if [[ "$BUILD" == true ]] || [[ ! -f "target/release/test-stratum-connection" ]]; then
    log "编译 Stratum 测试工具..."
    if ! cargo build --release --bin test-stratum-connection; then
        error "编译失败"
        exit 1
    fi
    success "编译完成"
fi

# 批量测试函数
batch_test() {
    log "开始批量测试..."
    
    # 预定义的矿池列表
    local pools=(
        "stratum+tcp://192.168.18.240:10203"
        "stratum+tcp://btc.f2pool.com:1314"
        "stratum+tcp://btc-asia.f2pool.com:1314"
        "stratum+tcp://btc-euro.f2pool.com:1314"
    )
    
    local success_count=0
    local total_count=${#pools[@]}
    
    for pool in "${pools[@]}"; do
        echo
        log "测试矿池: $pool"
        echo "─────────────────────────────────────────────"
        
        if run_test "$pool"; then
            success "✅ $pool - 测试通过"
            ((success_count++))
        else
            error "❌ $pool - 测试失败"
        fi
    done
    
    echo
    echo "═══════════════════════════════════════════════"
    log "批量测试完成"
    log "成功: $success_count/$total_count"
    
    if [[ $success_count -eq $total_count ]]; then
        success "所有矿池测试通过！"
        return 0
    else
        warning "部分矿池测试失败"
        return 1
    fi
}

# 运行单个测试
run_test() {
    local test_url="$1"
    local args=()
    
    args+=(--url "$test_url")
    args+=(--username "$USERNAME")
    args+=(--password "$PASSWORD")
    args+=(--timeout "$TIMEOUT")
    
    if [[ "$VERBOSE" == true ]]; then
        args+=(--verbose)
    fi
    
    # 运行测试
    if cargo run --release --bin test-stratum-connection -- "${args[@]}"; then
        return 0
    else
        return 1
    fi
}

# 主函数
main() {
    echo "🔍 CGMiner-RS Stratum 协议测试"
    echo "═══════════════════════════════════════════════"
    
    if [[ "$BATCH" == true ]]; then
        batch_test
    else
        log "测试单个矿池: $URL"
        echo
        if run_test "$URL"; then
            success "测试完成！"
            exit 0
        else
            error "测试失败！"
            exit 1
        fi
    fi
}

# 运行主函数
main
