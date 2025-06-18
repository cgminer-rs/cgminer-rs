#!/bin/bash

# F2Pool 虚拟挖矿器启动脚本
# 使用真实的 F2Pool 配置进行虚拟挖矿

echo "🔥 CGMiner-RS F2Pool 虚拟挖矿器"
echo "═══════════════════════════════════════════════════════════"
echo "📋 配置信息:"
echo "   矿工: kayuii.bbt"
echo "   密码: 21235365876986800"
echo "   主矿池: stratum+tcp://btc.f2pool.com:1314"
echo "   备用矿池: stratum+tcp://btc-asia.f2pool.com:1314"
echo "   欧洲矿池: stratum+tcp://btc-euro.f2pool.com:1314"
echo ""

# 检查 Rust 环境
if ! command -v cargo &> /dev/null; then
    echo "❌ 未找到 Cargo，请先安装 Rust"
    exit 1
fi

# 编译项目
echo "🔨 编译 F2Pool 虚拟挖矿器..."
if ! cargo build --release --bin f2pool_virtual; then
    echo "❌ 编译失败"
    exit 1
fi

echo "✅ 编译完成"
echo ""

# 运行虚拟挖矿器
echo "🚀 启动 F2Pool 虚拟挖矿器..."
echo "使用真实 F2Pool 配置进行虚拟挖矿"
echo ""

# 直接运行 F2Pool 虚拟挖矿器
cargo run --release --bin f2pool_virtual
