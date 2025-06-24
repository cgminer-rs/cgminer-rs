#!/bin/bash

echo "运行 CGMiner-RS 简单演示..."
echo "确保本地矿池转发运行在 127.0.0.1:1314"
echo ""

cd /Users/gecko/project/linux/cgminer_rs

# 运行演示程序
cargo run --release --example multi_device_demo --features=cpu-btc

echo ""
echo "演示完成！"
