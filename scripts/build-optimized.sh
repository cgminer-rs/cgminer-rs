#!/bin/bash
# CGMiner-RS 优化构建脚本
# 针对不同平台进行优化编译

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 获取系统信息
OS=$(uname -s)
ARCH=$(uname -m)
CPU_COUNT=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "4")

echo -e "${CYAN}🔨 CGMiner-RS 优化构建脚本${NC}"
echo -e "${CYAN}================================${NC}"
echo -e "操作系统: ${GREEN}$OS${NC}"
echo -e "架构: ${GREEN}$ARCH${NC}"
echo -e "CPU核心数: ${GREEN}$CPU_COUNT${NC}"
echo ""

# 检测平台并设置优化参数
detect_platform() {
    case "$OS" in
        "Darwin")
            if [[ "$ARCH" == "arm64" ]]; then
                PLATFORM="mac-m4"
                TARGET="aarch64-apple-darwin"
                echo -e "${PURPLE}🍎 检测到 Mac M4 (Apple Silicon)${NC}"
            else
                PLATFORM="mac-intel"
                TARGET="x86_64-apple-darwin"
                echo -e "${PURPLE}🍎 检测到 Intel Mac${NC}"
            fi
            ;;
        "Linux")
            if [[ "$ARCH" == "x86_64" ]]; then
                PLATFORM="linux-x86_64"
                TARGET="x86_64-unknown-linux-gnu"
                echo -e "${BLUE}🐧 检测到 Linux x86_64${NC}"
            elif [[ "$ARCH" == "aarch64" ]]; then
                PLATFORM="linux-aarch64"
                TARGET="aarch64-unknown-linux-gnu"
                echo -e "${BLUE}🐧 检测到 Linux ARM64${NC}"
            fi
            ;;
        *)
            PLATFORM="unknown"
            TARGET=""
            echo -e "${RED}❓ 未知平台: $OS-$ARCH${NC}"
            ;;
    esac
}

# 设置环境变量
setup_environment() {
    echo -e "${YELLOW}⚙️  设置构建环境...${NC}"
    
    # 基础环境变量
    export CARGO_BUILD_JOBS="$CPU_COUNT"
    export RUSTFLAGS=""
    
    # 平台特定的环境变量
    case "$PLATFORM" in
        "mac-m4")
            echo -e "${GREEN}🚀 配置 Mac M4 优化环境${NC}"
            export RUSTFLAGS="$RUSTFLAGS -C target-cpu=apple-m1"
            export RUSTFLAGS="$RUSTFLAGS -C target-feature=+neon,+crypto,+aes,+sha2,+sha3,+crc"
            export RUSTFLAGS="$RUSTFLAGS -C opt-level=3"
            export RUSTFLAGS="$RUSTFLAGS -C codegen-units=1"
            export RUSTFLAGS="$RUSTFLAGS -C lto=fat"
            export RUSTFLAGS="$RUSTFLAGS -C llvm-args=-enable-machine-outliner=never"
            export RUSTFLAGS="$RUSTFLAGS -C llvm-args=-enable-gvn-hoist"
            export RUSTFLAGS="$RUSTFLAGS -C llvm-args=-enable-unsafe-fp-math"
            export MACOSX_DEPLOYMENT_TARGET="11.0"
            ;;
        "mac-intel")
            echo -e "${GREEN}💻 配置 Intel Mac 优化环境${NC}"
            export RUSTFLAGS="$RUSTFLAGS -C target-cpu=native"
            export RUSTFLAGS="$RUSTFLAGS -C target-feature=+aes,+sha,+sse4.2,+avx2,+bmi2"
            export RUSTFLAGS="$RUSTFLAGS -C opt-level=3"
            export RUSTFLAGS="$RUSTFLAGS -C codegen-units=1"
            export RUSTFLAGS="$RUSTFLAGS -C lto=fat"
            ;;
        "linux-x86_64")
            echo -e "${GREEN}🐧 配置 Linux x86_64 优化环境${NC}"
            export RUSTFLAGS="$RUSTFLAGS -C target-cpu=native"
            export RUSTFLAGS="$RUSTFLAGS -C target-feature=+aes,+sha,+sse4.2,+avx2,+bmi2,+fma"
            export RUSTFLAGS="$RUSTFLAGS -C opt-level=3"
            export RUSTFLAGS="$RUSTFLAGS -C codegen-units=1"
            export RUSTFLAGS="$RUSTFLAGS -C lto=fat"
            ;;
        "linux-aarch64")
            echo -e "${GREEN}🦾 配置 Linux ARM64 优化环境${NC}"
            export RUSTFLAGS="$RUSTFLAGS -C target-cpu=native"
            export RUSTFLAGS="$RUSTFLAGS -C target-feature=+neon,+crypto,+aes,+sha2,+crc"
            export RUSTFLAGS="$RUSTFLAGS -C opt-level=3"
            export RUSTFLAGS="$RUSTFLAGS -C codegen-units=1"
            export RUSTFLAGS="$RUSTFLAGS -C lto=fat"
            ;;
    esac
    
    echo -e "RUSTFLAGS: ${CYAN}$RUSTFLAGS${NC}"
}

# 清理构建缓存
clean_build() {
    echo -e "${YELLOW}🧹 清理构建缓存...${NC}"
    cargo clean
}

# 构建发布版本
build_release() {
    echo -e "${YELLOW}🔨 开始优化构建...${NC}"
    echo -e "目标平台: ${GREEN}$TARGET${NC}"
    
    local start_time=$(date +%s)
    
    if [[ -n "$TARGET" ]]; then
        cargo build --release --target "$TARGET" --verbose
    else
        cargo build --release --verbose
    fi
    
    local end_time=$(date +%s)
    local build_time=$((end_time - start_time))
    
    echo -e "${GREEN}✅ 构建完成！用时: ${build_time}秒${NC}"
}

# 运行基准测试
run_benchmarks() {
    echo -e "${YELLOW}📊 运行性能基准测试...${NC}"
    
    if cargo bench --help >/dev/null 2>&1; then
        cargo bench --features btc-software
    else
        echo -e "${YELLOW}⚠️  基准测试不可用，跳过...${NC}"
    fi
}

# 测试构建结果
test_build() {
    echo -e "${YELLOW}🧪 测试构建结果...${NC}"
    
    # 运行单元测试
    cargo test --release --features btc-software
    
    # 检查二进制文件
    if [[ -n "$TARGET" ]]; then
        BINARY_PATH="target/$TARGET/release/cgminer-rs"
    else
        BINARY_PATH="target/release/cgminer-rs"
    fi
    
    if [[ -f "$BINARY_PATH" ]]; then
        echo -e "${GREEN}✅ 二进制文件生成成功: $BINARY_PATH${NC}"
        
        # 显示文件大小
        local file_size=$(du -h "$BINARY_PATH" | cut -f1)
        echo -e "文件大小: ${CYAN}$file_size${NC}"
        
        # 显示文件信息
        file "$BINARY_PATH"
    else
        echo -e "${RED}❌ 二进制文件未找到: $BINARY_PATH${NC}"
        exit 1
    fi
}

# 性能分析
profile_build() {
    echo -e "${YELLOW}📈 生成性能分析信息...${NC}"
    
    # 使用 perf 进行性能分析（仅Linux）
    if [[ "$OS" == "Linux" ]] && command -v perf >/dev/null 2>&1; then
        echo -e "${BLUE}🔍 运行 perf 性能分析...${NC}"
        # 这里可以添加具体的性能分析命令
    fi
    
    # 使用 Instruments 进行性能分析（仅macOS）
    if [[ "$OS" == "Darwin" ]] && command -v instruments >/dev/null 2>&1; then
        echo -e "${PURPLE}🔍 Instruments 可用于性能分析${NC}"
    fi
}

# 显示优化建议
show_optimization_tips() {
    echo -e "${CYAN}💡 优化建议:${NC}"
    
    case "$PLATFORM" in
        "mac-m4")
            echo -e "  ${GREEN}✓${NC} 已启用 Apple Silicon 专用优化"
            echo -e "  ${GREEN}✓${NC} 已启用 NEON SIMD 指令"
            echo -e "  ${GREEN}✓${NC} 已启用硬件 AES/SHA 加速"
            echo -e "  ${YELLOW}💡${NC} 建议设备数量为 CPU 核心数的 6-10 倍"
            echo -e "  ${YELLOW}💡${NC} 监控温度，M4 在高负载下可能需要散热"
            ;;
        "mac-intel")
            echo -e "  ${GREEN}✓${NC} 已启用 Intel 优化"
            echo -e "  ${GREEN}✓${NC} 已启用 AES-NI 和 SHA 扩展"
            echo -e "  ${YELLOW}💡${NC} 注意温度控制，Intel 芯片发热较大"
            ;;
        "linux-x86_64")
            echo -e "  ${GREEN}✓${NC} 已启用 Linux x86_64 优化"
            echo -e "  ${GREEN}✓${NC} 已启用原生 CPU 特性"
            echo -e "  ${YELLOW}💡${NC} 可以使用 CPU 绑定功能"
            ;;
        "linux-aarch64")
            echo -e "  ${GREEN}✓${NC} 已启用 ARM64 Linux 优化"
            echo -e "  ${GREEN}✓${NC} 已启用 NEON 和加密扩展"
            ;;
    esac
}

# 主函数
main() {
    detect_platform
    setup_environment
    
    # 解析命令行参数
    case "${1:-build}" in
        "clean")
            clean_build
            ;;
        "build")
            build_release
            test_build
            show_optimization_tips
            ;;
        "bench")
            build_release
            run_benchmarks
            ;;
        "profile")
            build_release
            profile_build
            ;;
        "all")
            clean_build
            build_release
            test_build
            run_benchmarks
            show_optimization_tips
            ;;
        *)
            echo -e "${RED}用法: $0 [clean|build|bench|profile|all]${NC}"
            echo -e "  clean   - 清理构建缓存"
            echo -e "  build   - 优化构建（默认）"
            echo -e "  bench   - 运行基准测试"
            echo -e "  profile - 性能分析"
            echo -e "  all     - 执行所有步骤"
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@"
