#!/bin/bash

# CGMiner-RS 高性能运行脚本
# 目标: 达到50+ GH/s总算力

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${BLUE}=== CGMiner-RS 高性能挖矿启动 ===${NC}"
echo -e "${YELLOW}目标算力: 50+ GH/s${NC}"
echo -e "${YELLOW}配置文件: configs/high-performance-50ghs.toml${NC}"
echo ""

# 检查配置文件
if [ ! -f "configs/high-performance-50ghs.toml" ]; then
    echo -e "${RED}错误: 高性能配置文件不存在${NC}"
    exit 1
fi

# 创建日志目录
mkdir -p logs

# 检查是否已有进程在运行
if [ -f "/tmp/cgminer-high-performance.pid" ]; then
    PID=$(cat /tmp/cgminer-high-performance.pid)
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}检测到CGMiner进程正在运行 (PID: $PID)${NC}"
        echo -e "${YELLOW}是否要停止现有进程? (y/N)${NC}"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}正在停止现有进程...${NC}"
            kill $PID
            sleep 2
        else
            echo -e "${RED}退出启动${NC}"
            exit 1
        fi
    fi
fi

# 构建项目
echo -e "${BLUE}正在构建项目...${NC}"
if ! cargo build --release; then
    echo -e "${RED}构建失败${NC}"
    exit 1
fi

echo -e "${GREEN}构建完成${NC}"
echo ""

# 显示系统信息
echo -e "${BLUE}=== 系统信息 ===${NC}"
echo "CPU核心数: $(sysctl -n hw.ncpu)"
echo "物理内存: $(sysctl -n hw.memsize | awk '{print int($1/1024/1024/1024)"GB"}')"
echo "系统版本: $(sw_vers -productName) $(sw_vers -productVersion)"
echo ""

# 显示配置摘要
echo -e "${BLUE}=== 高性能配置摘要 ===${NC}"
echo "设备数量: 64个软算法设备"
echo "单设备算力: 1-2 GH/s"
echo "预期总算力: 64-128 GH/s"
echo "CPU绑定策略: 性能优先"
echo "批处理大小: 8000"
echo "最小算力阈值: 50 GH/s"
echo ""

# 启动挖矿程序
echo -e "${GREEN}=== 启动高性能挖矿 ===${NC}"
echo -e "${YELLOW}使用配置: configs/high-performance-50ghs.toml${NC}"
echo -e "${YELLOW}日志文件: logs/cgminer-high-performance.log${NC}"
echo -e "${YELLOW}API端口: 4028${NC}"
echo -e "${YELLOW}Prometheus端口: 9090${NC}"
echo ""

# 启动命令
exec ./target/release/cgminer-rs --config configs/high-performance-50ghs.toml

# 如果程序异常退出
echo -e "${RED}CGMiner异常退出${NC}"
exit 1
