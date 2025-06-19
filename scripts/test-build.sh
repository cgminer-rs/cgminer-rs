#!/bin/bash
# 简单的构建测试脚本

set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 测试 Mac M4 优化构建结果${NC}"

# 检查二进制文件
BINARY_PATH="target/aarch64-apple-darwin/release/cgminer-rs"

if [[ -f "$BINARY_PATH" ]]; then
    echo -e "${GREEN}✅ 二进制文件生成成功: $BINARY_PATH${NC}"

    # 显示文件大小
    file_size=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "文件大小: ${file_size}"

    # 显示文件信息
    file "$BINARY_PATH"

    # 检查是否包含优化信息
    echo -e "\n${BLUE}🔍 检查优化信息:${NC}"
    otool -l "$BINARY_PATH" | grep -A 5 "LC_BUILD_VERSION" || echo "无法获取构建版本信息"

else
    echo -e "${RED}❌ 二进制文件未找到: $BINARY_PATH${NC}"
    exit 1
fi

echo -e "\n${GREEN}✅ 构建测试完成！${NC}"
