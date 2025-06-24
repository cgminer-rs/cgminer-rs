#!/bin/bash

# 测试混合编译时GPU核心工作分发修复效果
# 验证修复后GPU核心是否能正确接收工作并产生算力

echo "🔍 测试混合编译GPU核心工作分发修复效果"
echo "=========================================="

echo "📦 清理并重新编译..."
cargo clean > /dev/null 2>&1

echo "🔨 混合编译 (cpu-btc,gpu-btc)..."
echo "预期：自动选择GPU核心，移除CPU核心，工作只分发给GPU"

# 混合编译并运行
cargo build --release --features=cpu-btc,gpu-btc 2>&1 | tail -10

if [ $? -eq 0 ]; then
    echo "✅ 编译成功"

    echo "🚀 启动挖矿测试 (30秒)..."
    echo "监控关键日志："
    echo "  - Selected optimal core: GPU"
    echo "  - Removing unselected core: CPU"
    echo "  - Work dispatched to: core:gpu"
    echo "  - GPU算力输出"

    timeout 30s cargo run --release --features=cpu-btc,gpu-btc 2>&1 | \
    grep -E "(Selected.*core|Removing.*core|Work dispatched|算力|MH/s|GPU.*设备)" | \
    head -20

    echo ""
    echo "📊 测试结果分析："
    echo "如果看到："
    echo "  ✅ 'Selected optimal core: gpu' - GPU核心被正确选择"
    echo "  ✅ 'Removing unselected core: cpu' - CPU核心被正确移除"
    echo "  ✅ 'Work dispatched to: core:gpu' - 工作被分发到GPU核心"
    echo "  ✅ GPU算力数值 > 0 - GPU核心正常工作"
    echo ""
    echo "那么修复成功！"
else
    echo "❌ 编译失败"
    exit 1
fi

echo ""
echo "💡 使用建议："
echo "1. 单独GPU编译：cargo build --features=gpu-btc"
echo "2. 混合编译(推荐)：cargo build --features=cpu-btc,gpu-btc"
echo "3. 混合编译时会自动选择GPU核心，CPU作为备选"
