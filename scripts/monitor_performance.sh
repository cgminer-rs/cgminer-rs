#!/bin/bash

# CGMiner-RS 性能监控脚本
# 实时监控算力和性能指标

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# API配置
API_HOST="127.0.0.1"
API_PORT="4028"

# 检查cgminer是否运行
check_cgminer_running() {
    if ! nc -z $API_HOST $API_PORT 2>/dev/null; then
        echo -e "${RED}错误: CGMiner API不可访问 ($API_HOST:$API_PORT)${NC}"
        echo -e "${YELLOW}请确保CGMiner正在运行并且API已启用${NC}"
        exit 1
    fi
}

# 发送API命令
send_api_command() {
    local command="$1"
    echo "$command" | nc $API_HOST $API_PORT 2>/dev/null || echo "API_ERROR"
}

# 解析JSON响应 (简单版本)
parse_hashrate() {
    local response="$1"
    echo "$response" | grep -o '"hashrate":[0-9.]*' | cut -d':' -f2 | head -1
}

parse_device_count() {
    local response="$1"
    echo "$response" | grep -o '"device_count":[0-9]*' | cut -d':' -f2 | head -1
}

# 格式化算力显示
format_hashrate() {
    local hashrate="$1"
    if [ -z "$hashrate" ] || [ "$hashrate" = "0" ]; then
        echo "0 H/s"
        return
    fi
    
    # 转换为GH/s
    local ghs=$(echo "scale=2; $hashrate / 1000000000" | bc -l 2>/dev/null || echo "0")
    echo "${ghs} GH/s"
}

# 主监控循环
monitor_performance() {
    echo -e "${BLUE}=== CGMiner-RS 性能监控 ===${NC}"
    echo -e "${YELLOW}目标算力: 50+ GH/s${NC}"
    echo -e "${YELLOW}API地址: $API_HOST:$API_PORT${NC}"
    echo -e "${CYAN}按 Ctrl+C 退出监控${NC}"
    echo ""
    
    local iteration=0
    while true; do
        clear
        echo -e "${BLUE}=== CGMiner-RS 实时性能监控 ===${NC}"
        echo -e "${CYAN}更新时间: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
        echo ""
        
        # 获取总体统计
        local summary=$(send_api_command "summary")
        if [ "$summary" != "API_ERROR" ]; then
            local total_hashrate=$(parse_hashrate "$summary")
            local formatted_hashrate=$(format_hashrate "$total_hashrate")
            
            echo -e "${GREEN}=== 总体性能 ===${NC}"
            echo -e "总算力: ${YELLOW}$formatted_hashrate${NC}"
            
            # 检查是否达到目标
            local ghs_value=$(echo "$formatted_hashrate" | cut -d' ' -f1)
            local target_check=$(echo "$ghs_value >= 50" | bc -l 2>/dev/null || echo "0")
            if [ "$target_check" = "1" ]; then
                echo -e "状态: ${GREEN}✓ 已达到50+ GH/s目标${NC}"
            else
                echo -e "状态: ${RED}✗ 未达到50 GH/s目标${NC}"
            fi
            echo ""
        fi
        
        # 获取设备统计
        local devices=$(send_api_command "devs")
        if [ "$devices" != "API_ERROR" ]; then
            echo -e "${GREEN}=== 设备性能 ===${NC}"
            
            # 简单解析设备信息 (这里需要更复杂的JSON解析)
            local device_count=$(parse_device_count "$devices")
            if [ -n "$device_count" ] && [ "$device_count" != "0" ]; then
                echo -e "活跃设备数: ${YELLOW}$device_count${NC}"
            else
                echo -e "活跃设备数: ${YELLOW}检测中...${NC}"
            fi
            echo ""
        fi
        
        # 获取矿池统计
        local pools=$(send_api_command "pools")
        if [ "$pools" != "API_ERROR" ]; then
            echo -e "${GREEN}=== 矿池状态 ===${NC}"
            if echo "$pools" | grep -q "Alive"; then
                echo -e "矿池连接: ${GREEN}✓ 已连接${NC}"
            else
                echo -e "矿池连接: ${RED}✗ 连接异常${NC}"
            fi
            echo ""
        fi
        
        # 系统资源监控
        echo -e "${GREEN}=== 系统资源 ===${NC}"
        local cpu_usage=$(top -l 1 -n 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')
        local memory_pressure=$(memory_pressure | grep "System-wide memory free percentage" | awk '{print $5}' | sed 's/%//')
        
        echo -e "CPU使用率: ${YELLOW}${cpu_usage}%${NC}"
        if [ -n "$memory_pressure" ]; then
            local memory_used=$((100 - memory_pressure))
            echo -e "内存使用率: ${YELLOW}${memory_used}%${NC}"
        fi
        echo ""
        
        # 性能建议
        echo -e "${GREEN}=== 性能建议 ===${NC}"
        if [ "$target_check" != "1" ]; then
            echo -e "${YELLOW}• 当前算力未达到50 GH/s目标${NC}"
            echo -e "${YELLOW}• 建议检查设备配置和CPU绑定${NC}"
            echo -e "${YELLOW}• 可以尝试增加设备数量或优化参数${NC}"
        else
            echo -e "${GREEN}• 性能表现良好，已达到目标算力${NC}"
        fi
        echo ""
        
        echo -e "${CYAN}下次更新: 10秒后 (第$((++iteration))次)${NC}"
        sleep 10
    done
}

# 主程序
main() {
    # 检查依赖
    if ! command -v nc &> /dev/null; then
        echo -e "${RED}错误: 需要安装netcat (nc)${NC}"
        exit 1
    fi
    
    if ! command -v bc &> /dev/null; then
        echo -e "${RED}错误: 需要安装bc计算器${NC}"
        exit 1
    fi
    
    # 检查CGMiner状态
    check_cgminer_running
    
    # 开始监控
    monitor_performance
}

# 信号处理
trap 'echo -e "\n${YELLOW}监控已停止${NC}"; exit 0' INT TERM

# 运行主程序
main "$@"
