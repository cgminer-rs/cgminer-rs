#!/bin/bash
# CGMiner-RS 架构边界检查脚本
# 自动验证架构边界遵循情况，实现边界检查清单的自动化验证

set -e

echo "🏗️ CGMiner-RS 架构边界检查"
echo "================================"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 检查结果统计
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

# 检查函数
check_result() {
    local description="$1"
    local result="$2"
    local level="${3:-error}" # error, warning
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if [ "$result" = "0" ]; then
        echo -e "${GREEN}✅ $description${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        if [ "$level" = "warning" ]; then
            echo -e "${YELLOW}⚠️  $description${NC}"
            WARNINGS=$((WARNINGS + 1))
        else
            echo -e "${RED}❌ $description${NC}"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
        fi
    fi
}

# 1. 应用层边界检查
echo -e "\n${BLUE}📱 应用层边界检查${NC}"
echo "--------------------------------"

# 检查应用层不包含具体挖矿算法
echo "检查应用层是否包含挖矿算法实现..."
if grep -r "sha256\|scrypt\|blake2b" src/ --include="*.rs" | grep -v "algorithm.*String" | grep -v "test" >/dev/null 2>&1; then
    check_result "应用层不包含具体挖矿算法实现" 1
else
    check_result "应用层不包含具体挖矿算法实现" 0
fi

# 检查应用层不直接控制硬件设备
echo "检查应用层是否直接控制硬件..."
if grep -r "serialport\|spidev\|i2c\|gpio" src/ --include="*.rs" | grep -v "test" >/dev/null 2>&1; then
    check_result "应用层不直接控制硬件设备" 1
else
    check_result "应用层不直接控制硬件设备" 0
fi

# 检查应用层不重复导出引擎层功能
echo "检查应用层是否重复导出引擎层功能..."
DUPLICATE_EXPORTS=$(grep -r "pub use.*core::" src/lib.rs 2>/dev/null | grep -E "(TemperatureManager|SoftwareDevice|HardwareDevice)" | wc -l)
if [ "$DUPLICATE_EXPORTS" -gt 0 ]; then
    check_result "应用层不重复导出引擎层功能" 1
else
    check_result "应用层不重复导出引擎层功能" 0
fi

# 检查应用层专注于服务编排和用户界面
echo "检查应用层服务编排功能..."
if grep -r "pool\|api\|web\|config\|monitor" src/ --include="*.rs" >/dev/null 2>&1; then
    check_result "应用层专注于服务编排和用户界面" 0
else
    check_result "应用层专注于服务编排和用户界面" 1
fi

# 2. 引擎层边界检查（外置核心）
echo -e "\n${BLUE}⚙️ 引擎层边界检查${NC}"
echo "--------------------------------"

# 检查外置核心目录
CORES_FOUND=0
for core_dir in ../cgminer-*-core; do
    if [ -d "$core_dir" ]; then
        CORES_FOUND=$((CORES_FOUND + 1))
        core_name=$(basename "$core_dir")
        echo "检查外置核心: $core_name"
        
        # 检查核心不处理网络连接
        if grep -r "pool\|stratum\|tcp\|http" "$core_dir/src/" --include="*.rs" | grep -v "test" >/dev/null 2>&1; then
            check_result "$core_name 不处理网络连接" 1
        else
            check_result "$core_name 不处理网络连接" 0
        fi
        
        # 检查核心不管理全局配置
        if grep -r "global.*config\|config.*global" "$core_dir/src/" --include="*.rs" >/dev/null 2>&1; then
            check_result "$core_name 不管理全局配置" 1
        else
            check_result "$core_name 不管理全局配置" 0
        fi
        
        # 检查核心不提供Web界面
        if grep -r "web\|html\|css\|javascript" "$core_dir/src/" --include="*.rs" >/dev/null 2>&1; then
            check_result "$core_name 不提供Web界面" 1
        else
            check_result "$core_name 不提供Web界面" 0
        fi
        
        # 检查核心专注于挖矿性能和硬件控制
        if grep -r "mining\|device\|hardware\|algorithm" "$core_dir/src/" --include="*.rs" >/dev/null 2>&1; then
            check_result "$core_name 专注于挖矿性能和硬件控制" 0
        else
            check_result "$core_name 专注于挖矿性能和硬件控制" 1
        fi
    fi
done

if [ "$CORES_FOUND" -eq 0 ]; then
    check_result "发现外置核心模块" 1
else
    check_result "发现外置核心模块 ($CORES_FOUND 个)" 0
fi

# 3. 接口层边界检查
echo -e "\n${BLUE}🔌 接口层边界检查${NC}"
echo "--------------------------------"

# 检查 cgminer-core 库存在
if [ -d "../cgminer-core" ]; then
    check_result "cgminer-core 标准接口库存在" 0
    
    # 检查标准接口定义
    if grep -q "pub trait CoreFactory" ../cgminer-core/src/*.rs 2>/dev/null; then
        check_result "CoreFactory 接口已定义" 0
    else
        check_result "CoreFactory 接口已定义" 1
    fi
    
    if grep -q "pub trait MiningCore" ../cgminer-core/src/*.rs 2>/dev/null; then
        check_result "MiningCore 接口已定义" 0
    else
        check_result "MiningCore 接口已定义" 1
    fi
    
    if grep -q "pub trait MiningDevice" ../cgminer-core/src/*.rs 2>/dev/null; then
        check_result "MiningDevice 接口已定义" 0
    else
        check_result "MiningDevice 接口已定义" 1
    fi
else
    check_result "cgminer-core 标准接口库存在" 1
fi

# 4. 依赖关系检查
echo -e "\n${BLUE}📦 依赖关系检查${NC}"
echo "--------------------------------"

# 检查应用层依赖
if grep -q "cgminer-core" Cargo.toml; then
    check_result "应用层正确依赖 cgminer-core" 0
else
    check_result "应用层正确依赖 cgminer-core" 1
fi

# 检查外置核心依赖
for core_dir in ../cgminer-*-core; do
    if [ -d "$core_dir" ]; then
        core_name=$(basename "$core_dir")
        
        # 检查核心依赖 cgminer-core
        if grep -q "cgminer-core" "$core_dir/Cargo.toml"; then
            check_result "$core_name 正确依赖 cgminer-core" 0
        else
            check_result "$core_name 正确依赖 cgminer-core" 1
        fi
        
        # 检查核心不依赖应用层
        if grep -q "cgminer-rs" "$core_dir/Cargo.toml"; then
            check_result "$core_name 不依赖应用层" 1
        else
            check_result "$core_name 不依赖应用层" 0
        fi
        
        # 检查核心不依赖其他核心
        OTHER_CORES=$(grep "cgminer-.*-core" "$core_dir/Cargo.toml" | grep -v "cgminer-core" | wc -l)
        if [ "$OTHER_CORES" -gt 0 ]; then
            check_result "$core_name 不依赖其他外置核心" 1
        else
            check_result "$core_name 不依赖其他外置核心" 0
        fi
    fi
done

# 5. 配置管理边界检查
echo -e "\n${BLUE}⚙️ 配置管理边界检查${NC}"
echo "--------------------------------"

# 检查配置传递机制
if grep -r "CoreConfig" src/ --include="*.rs" >/dev/null 2>&1; then
    check_result "使用标准化配置传递机制" 0
else
    check_result "使用标准化配置传递机制" 1
fi

# 检查配置验证层次
if grep -r "validate_config" src/ --include="*.rs" >/dev/null 2>&1; then
    check_result "实现配置验证机制" 0
else
    check_result "实现配置验证机制" 1 warning
fi

# 6. 编译时检查
echo -e "\n${BLUE}🔨 编译时边界检查${NC}"
echo "--------------------------------"

# 检查应用层编译
if cargo check --quiet 2>/dev/null; then
    check_result "应用层编译通过" 0
else
    check_result "应用层编译通过" 1
fi

# 检查各个特性编译
FEATURES=("cpu-btc" "maijie-l7" "all-cores")
for feature in "${FEATURES[@]}"; do
    if cargo check --features "$feature" --quiet 2>/dev/null; then
        check_result "特性 $feature 编译通过" 0
    else
        check_result "特性 $feature 编译通过" 1
    fi
done

# 7. 生成边界检查报告
echo -e "\n${BLUE}📄 生成边界检查报告${NC}"
echo "--------------------------------"

REPORT_FILE="docs/architecture-boundary-check-report.md"
cat > "$REPORT_FILE" << EOF
# CGMiner-RS 架构边界检查报告

**生成时间**: $(date)
**检查脚本**: scripts/check-architecture-boundaries.sh

## 检查结果摘要

- **总检查项**: $TOTAL_CHECKS
- **通过检查**: $PASSED_CHECKS
- **失败检查**: $FAILED_CHECKS
- **警告项目**: $WARNINGS
- **合规率**: $(( PASSED_CHECKS * 100 / TOTAL_CHECKS ))%

## 架构边界合规性

### ✅ 应用层边界合规性

应用层应该：
- ✅ 不包含具体挖矿算法实现
- ✅ 不直接控制硬件设备  
- ✅ 不重复导出引擎层功能
- ✅ 专注于服务编排和用户界面

### ⚙️ 引擎层边界合规性

外置核心应该：
- ✅ 不处理网络连接
- ✅ 不管理全局配置
- ✅ 不提供Web界面
- ✅ 专注于挖矿性能和硬件控制

### 🔌 接口层边界合规性

标准化接口应该：
- ✅ 提供统一的接口定义
- ✅ 确保版本兼容性
- ✅ 支持清晰的错误处理

## 依赖关系合规性

### 允许的依赖关系
- ✅ cgminer-rs → cgminer-core
- ✅ cgminer-*-core → cgminer-core

### 禁止的依赖关系
- ❌ cgminer-*-core → cgminer-rs
- ❌ cgminer-*-core → 其他外置核心

## 改进建议

EOF

if [ $FAILED_CHECKS -gt 0 ]; then
    cat >> "$REPORT_FILE" << EOF
### 🚨 需要修复的问题

检测到 $FAILED_CHECKS 个边界违反问题，建议：

1. **立即修复**: 违反架构边界的代码
2. **重构代码**: 将违反边界的功能移动到正确的层次
3. **更新文档**: 确保开发团队了解架构边界要求

EOF
fi

if [ $WARNINGS -gt 0 ]; then
    cat >> "$REPORT_FILE" << EOF
### ⚠️ 需要关注的警告

检测到 $WARNINGS 个警告项目，建议：

1. **完善实现**: 补充缺失的功能
2. **增强验证**: 添加更多的验证机制
3. **持续监控**: 定期检查这些项目的状态

EOF
fi

cat >> "$REPORT_FILE" << EOF
## 自动化检查

### CI/CD 集成

建议将此检查脚本集成到 CI/CD 流程中：

\`\`\`yaml
# .github/workflows/architecture-check.yml
name: Architecture Boundary Check

on: [push, pull_request]

jobs:
  boundary-check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Check Architecture Boundaries
      run: ./scripts/check-architecture-boundaries.sh
\`\`\`

### 定期检查

建议定期运行边界检查：
- **每次提交前**: 开发者本地检查
- **每次PR**: 自动化CI检查  
- **每周**: 完整的架构审查

---

**检查工具**: scripts/check-architecture-boundaries.sh
**相关文档**: [架构边界定义](./ARCHITECTURE_BOUNDARIES.md)
EOF

check_result "生成架构边界检查报告" 0

# 8. 显示最终结果
echo -e "\n${BLUE}📊 边界检查结果摘要${NC}"
echo "================================"
echo -e "总检查项: ${BLUE}$TOTAL_CHECKS${NC}"
echo -e "通过检查: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "失败检查: ${RED}$FAILED_CHECKS${NC}"
echo -e "警告项目: ${YELLOW}$WARNINGS${NC}"

if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 架构边界检查通过！${NC}"
    if [ $WARNINGS -gt 0 ]; then
        echo -e "${YELLOW}⚠️  但有 $WARNINGS 个警告项目需要关注${NC}"
    fi
    echo -e "边界检查报告已生成: ${BLUE}$REPORT_FILE${NC}"
    exit 0
else
    echo -e "\n${RED}🚨 发现 $FAILED_CHECKS 个架构边界违反问题${NC}"
    echo -e "详细信息请查看: ${BLUE}$REPORT_FILE${NC}"
    exit 1
fi
