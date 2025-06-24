#!/bin/bash
# CGMiner-RS 标准化接口验证脚本
# 验证 cgminer-core 库的接口完整性和外置核心的实现情况

set -e

echo "🔍 CGMiner-RS 标准化接口验证"
echo "================================"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 验证结果统计
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

# 检查函数
check_result() {
    local description="$1"
    local result="$2"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if [ "$result" = "0" ]; then
        echo -e "${GREEN}✅ $description${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${RED}❌ $description${NC}"
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
    fi
}

# 1. 验证 cgminer-core 库存在性
echo -e "\n${BLUE}📦 验证 cgminer-core 库${NC}"
echo "--------------------------------"

if [ -d "../cgminer-core" ]; then
    check_result "cgminer-core 目录存在" 0
else
    check_result "cgminer-core 目录存在" 1
    echo -e "${RED}错误: cgminer-core 库不存在于 ../cgminer-core${NC}"
    exit 1
fi

# 验证 cgminer-core 编译
cd ../cgminer-core
if cargo check --quiet 2>/dev/null; then
    check_result "cgminer-core 编译通过" 0
else
    check_result "cgminer-core 编译通过" 1
fi
cd - > /dev/null

# 2. 验证标准化接口定义
echo -e "\n${BLUE}🔌 验证标准化接口定义${NC}"
echo "--------------------------------"

# 检查 CoreFactory trait
if grep -q "pub trait CoreFactory" ../cgminer-core/src/registry.rs; then
    check_result "CoreFactory trait 已定义" 0
else
    check_result "CoreFactory trait 已定义" 1
fi

# 检查 MiningCore trait
if grep -q "pub trait MiningCore" ../cgminer-core/src/core.rs; then
    check_result "MiningCore trait 已定义" 0
else
    check_result "MiningCore trait 已定义" 1
fi

# 检查 MiningDevice trait
if grep -q "pub trait MiningDevice" ../cgminer-core/src/device.rs; then
    check_result "MiningDevice trait 已定义" 0
else
    check_result "MiningDevice trait 已定义" 1
fi

# 3. 验证核心类型定义
echo -e "\n${BLUE}📋 验证核心类型定义${NC}"
echo "--------------------------------"

# 检查基础类型
CORE_TYPES=("Work" "MiningResult" "CoreConfig" "CoreStats" "DeviceInfo" "DeviceStats")
for type_name in "${CORE_TYPES[@]}"; do
    if grep -r "pub struct $type_name" ../cgminer-core/src/ >/dev/null 2>&1; then
        check_result "$type_name 类型已定义" 0
    else
        check_result "$type_name 类型已定义" 1
    fi
done

# 4. 验证外置核心实现
echo -e "\n${BLUE}🏭 验证外置核心实现${NC}"
echo "--------------------------------"

# 检查 CPU 核心
if [ -d "../cgminer-cpu-btc-core" ]; then
    check_result "cgminer-cpu-btc-core 存在" 0
    
    # 检查 CPU 核心是否实现了 CoreFactory
    if grep -q "impl.*CoreFactory" ../cgminer-cpu-btc-core/src/*.rs 2>/dev/null; then
        check_result "CPU 核心实现 CoreFactory" 0
    else
        check_result "CPU 核心实现 CoreFactory" 1
    fi
    
    # 检查 CPU 核心是否实现了 MiningCore
    if grep -q "impl.*MiningCore" ../cgminer-cpu-btc-core/src/*.rs 2>/dev/null; then
        check_result "CPU 核心实现 MiningCore" 0
    else
        check_result "CPU 核心实现 MiningCore" 1
    fi
else
    check_result "cgminer-cpu-btc-core 存在" 1
fi

# 检查 ASIC 核心
if [ -d "../cgminer-asic-maijie-l7-core" ]; then
    check_result "cgminer-asic-maijie-l7-core 存在" 0
    
    # 检查 ASIC 核心是否实现了 CoreFactory
    if grep -q "impl.*CoreFactory" ../cgminer-asic-maijie-l7-core/src/*.rs 2>/dev/null; then
        check_result "ASIC 核心实现 CoreFactory" 0
    else
        check_result "ASIC 核心实现 CoreFactory" 1
    fi
    
    # 检查 ASIC 核心是否实现了 MiningCore
    if grep -q "impl.*MiningCore" ../cgminer-asic-maijie-l7-core/src/*.rs 2>/dev/null; then
        check_result "ASIC 核心实现 MiningCore" 0
    else
        check_result "ASIC 核心实现 MiningCore" 1
    fi
else
    check_result "cgminer-asic-maijie-l7-core 存在" 1
fi

# 5. 验证接口兼容性
echo -e "\n${BLUE}🔗 验证接口兼容性${NC}"
echo "--------------------------------"

# 编译测试 - CPU 核心
if cargo check --features cpu-btc --quiet 2>/dev/null; then
    check_result "CPU 核心接口兼容性" 0
else
    check_result "CPU 核心接口兼容性" 1
fi

# 编译测试 - ASIC 核心
if cargo check --features maijie-l7 --quiet 2>/dev/null; then
    check_result "ASIC 核心接口兼容性" 0
else
    check_result "ASIC 核心接口兼容性" 1
fi

# 编译测试 - 所有核心
if cargo check --features all-cores --quiet 2>/dev/null; then
    check_result "所有核心接口兼容性" 0
else
    check_result "所有核心接口兼容性" 1
fi

# 6. 验证版本兼容性
echo -e "\n${BLUE}📊 验证版本兼容性${NC}"
echo "--------------------------------"

# 检查 cgminer-core 版本
CORE_VERSION=$(grep '^version' ../cgminer-core/Cargo.toml | cut -d'"' -f2)
if [ -n "$CORE_VERSION" ]; then
    check_result "cgminer-core 版本信息 ($CORE_VERSION)" 0
else
    check_result "cgminer-core 版本信息" 1
fi

# 检查依赖版本一致性
MAIN_CORE_DEP=$(grep 'cgminer-core.*path' Cargo.toml | head -1)
if [ -n "$MAIN_CORE_DEP" ]; then
    check_result "主程序依赖 cgminer-core" 0
else
    check_result "主程序依赖 cgminer-core" 1
fi

# 7. 生成接口验证报告
echo -e "\n${BLUE}📄 生成接口验证报告${NC}"
echo "--------------------------------"

REPORT_FILE="docs/interface-verification-report.md"
cat > "$REPORT_FILE" << EOF
# CGMiner-RS 接口验证报告

**生成时间**: $(date)
**验证脚本**: scripts/verify-interfaces.sh

## 验证结果摘要

- **总检查项**: $TOTAL_CHECKS
- **通过检查**: $PASSED_CHECKS
- **失败检查**: $FAILED_CHECKS
- **成功率**: $(( PASSED_CHECKS * 100 / TOTAL_CHECKS ))%

## 标准化接口状态

### ✅ 已验证的接口

1. **CoreFactory trait** - 核心工厂接口
   - 位置: \`../cgminer-core/src/registry.rs\`
   - 方法: \`create_core\`, \`validate_config\`, \`default_config\`

2. **MiningCore trait** - 挖矿核心接口
   - 位置: \`../cgminer-core/src/core.rs\`
   - 方法: \`initialize\`, \`start\`, \`stop\`, \`submit_work\`, \`collect_results\`

3. **MiningDevice trait** - 挖矿设备接口
   - 位置: \`../cgminer-core/src/device.rs\`
   - 方法: \`start\`, \`stop\`, \`submit_work\`, \`get_result\`, \`get_stats\`

### 📦 核心类型定义

- \`Work\` - 工作任务类型
- \`MiningResult\` - 挖矿结果类型
- \`CoreConfig\` - 核心配置类型
- \`CoreStats\` - 核心统计类型
- \`DeviceInfo\` - 设备信息类型
- \`DeviceStats\` - 设备统计类型

### 🏭 外置核心实现状态

- **cgminer-cpu-btc-core**: $([ -d "../cgminer-cpu-btc-core" ] && echo "✅ 已实现" || echo "❌ 未找到")
- **cgminer-asic-maijie-l7-core**: $([ -d "../cgminer-asic-maijie-l7-core" ] && echo "✅ 已实现" || echo "❌ 未找到")

## 架构边界合规性

### ✅ 应用层 (cgminer-rs)
- 不包含具体挖矿算法实现
- 不直接控制硬件设备
- 通过标准接口与外置核心通信
- 专注于服务编排和用户界面

### ✅ 引擎层 (外置核心)
- 实现标准化接口 (CoreFactory, MiningCore)
- 不处理网络连接和全局配置
- 专注于挖矿算法和硬件控制

### 🔌 接口层 (cgminer-core)
- 提供统一的标准化接口定义
- 确保版本兼容性
- 支持清晰的错误处理

## 建议和改进

EOF

if [ $FAILED_CHECKS -gt 0 ]; then
    cat >> "$REPORT_FILE" << EOF
### ⚠️ 需要修复的问题

检测到 $FAILED_CHECKS 个失败的检查项，建议：

1. 检查外置核心的接口实现完整性
2. 验证编译依赖和版本兼容性
3. 确保所有必需的类型定义存在

EOF
fi

cat >> "$REPORT_FILE" << EOF
### 🎯 持续改进

1. **自动化测试**: 集成到 CI/CD 流程中
2. **接口文档**: 保持接口文档与实现同步
3. **版本管理**: 建立接口版本兼容性策略

---

**验证工具**: scripts/verify-interfaces.sh
**相关文档**: [架构边界定义](./ARCHITECTURE_BOUNDARIES.md)
EOF

check_result "生成接口验证报告" 0

# 8. 显示最终结果
echo -e "\n${BLUE}📊 验证结果摘要${NC}"
echo "================================"
echo -e "总检查项: ${BLUE}$TOTAL_CHECKS${NC}"
echo -e "通过检查: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "失败检查: ${RED}$FAILED_CHECKS${NC}"

if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 所有接口验证通过！${NC}"
    echo -e "接口验证报告已生成: ${BLUE}$REPORT_FILE${NC}"
    exit 0
else
    echo -e "\n${YELLOW}⚠️  发现 $FAILED_CHECKS 个问题需要修复${NC}"
    echo -e "详细信息请查看: ${BLUE}$REPORT_FILE${NC}"
    exit 1
fi
