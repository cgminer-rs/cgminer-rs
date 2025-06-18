# Stratum 协议测试指南

## 概述

本文档介绍如何使用 CGMiner-RS 项目中的 Stratum 协议测试工具来验证矿池连接和协议兼容性。

## Stratum 协议简介

### 什么是 Stratum 协议？

Stratum 是专为矿池挖矿设计的协议，具有以下特点：
- **低延迟**: 最小化网络通信延迟
- **高效率**: 减少网络带宽使用
- **可扩展**: 支持各种算力的硬件
- **实时性**: 快速获取新工作，减少拒绝率

### URL 格式

正确的 Stratum URL 格式：
```
stratum+tcp://服务器地址:端口
```

示例：
- `stratum+tcp://btc.f2pool.com:1314` - F2Pool BTC 矿池
- `stratum+tcp://192.168.18.240:10203` - 本地或内网矿池
- `stratum+tcp://pool.example.com:4444` - 通用矿池

## 测试工具使用

### 编译测试工具

```bash
# 编译所有二进制程序
cargo build --release

# 或者只编译测试工具
cargo build --release --bin test-stratum-connection
```

### 基本使用

```bash
# 测试默认地址 (192.168.18.240:10203)
cargo run --bin test-stratum-connection

# 测试指定地址
cargo run --bin test-stratum-connection -- --url "stratum+tcp://192.168.18.240:10203"

# 使用自定义用户名和密码
cargo run --bin test-stratum-connection -- \
  --url "stratum+tcp://192.168.18.240:10203" \
  --username "test.worker" \
  --password "x"

# 显示详细输出
cargo run --bin test-stratum-connection -- \
  --url "stratum+tcp://192.168.18.240:10203" \
  --verbose

# 设置连接超时
cargo run --bin test-stratum-connection -- \
  --url "stratum+tcp://192.168.18.240:10203" \
  --timeout 30
```

### 命令行参数

| 参数 | 短参数 | 默认值 | 说明 |
|------|--------|--------|------|
| `--url` | `-u` | `stratum+tcp://192.168.18.240:10203` | Stratum 服务器 URL |
| `--username` | `-u` | `test.worker` | 测试用户名 |
| `--password` | `-p` | `x` | 测试密码 |
| `--timeout` | `-t` | `10` | 连接超时时间（秒） |
| `--verbose` | `-v` | `false` | 显示详细输出 |

## 测试流程

测试工具执行以下步骤：

### 1. URL 验证
- 检查 URL 是否以 `stratum+tcp://` 开头
- 解析服务器地址和端口

### 2. TCP 连接测试
- 尝试建立 TCP 连接
- 验证网络连通性
- 显示连接信息（如果启用详细输出）

### 3. Stratum 协议测试
- 发送 `mining.subscribe` 请求
- 解析服务器响应
- 发送 `mining.authorize` 请求
- 分析认证结果

### 4. 矿池识别
- 分析响应内容
- 尝试识别矿池类型
- 检测 F2Pool 特征

## 输出解释

### 成功输出示例

```
🔍 Stratum 协议测试工具
═══════════════════════════════════════════════════════════
📋 测试配置:
   URL: stratum+tcp://192.168.18.240:10203
   用户名: test.worker
   密码: x
   超时: 10 秒

🔗 正在连接到: 192.168.18.240:10203
✅ TCP 连接成功
✅ Stratum 协议测试成功
📊 矿池信息:
   支持方法: mining.notify, 认证成功, 🐟 检测到 F2Pool 矿池特征

🎉 所有测试完成！
```

### 错误输出示例

```
❌ TCP 连接失败: Connection refused (os error 61)
```

```
❌ Stratum 协议测试失败: 服务器响应不符合 Stratum 协议格式
```

## 常见问题排查

### 连接被拒绝
- 检查服务器地址和端口是否正确
- 确认服务器是否在运行
- 检查防火墙设置

### 协议不兼容
- 确认服务器支持 Stratum 协议
- 检查服务器是否为 HTTP 而非 Stratum
- 验证服务器配置

### 认证失败
- 检查用户名和密码是否正确
- 确认矿池是否需要注册
- 验证矿工名称格式

## F2Pool 特征检测

测试工具会尝试检测以下 F2Pool 特征：
- 响应中包含 "f2pool" 或 "F2Pool" 字符串
- 特定的订阅响应格式
- F2Pool 特有的方法支持

## 集成到配置文件

测试成功后，可以将地址添加到配置文件：

```toml
[[pools.pools]]
url = "stratum+tcp://192.168.18.240:10203"
user = "your_username.worker"
password = "your_password"
priority = 1
enabled = true
```

## 高级用法

### 批量测试多个地址

创建脚本测试多个矿池：

```bash
#!/bin/bash

pools=(
    "stratum+tcp://192.168.18.240:10203"
    "stratum+tcp://btc.f2pool.com:1314"
    "stratum+tcp://btc-asia.f2pool.com:1314"
)

for pool in "${pools[@]}"; do
    echo "测试矿池: $pool"
    cargo run --bin test-stratum-connection -- --url "$pool"
    echo "---"
done
```

### 性能测试

使用 `time` 命令测试连接性能：

```bash
time cargo run --release --bin test-stratum-connection -- \
  --url "stratum+tcp://192.168.18.240:10203"
```

## 故障排除

如果测试失败，请检查：

1. **网络连接**: 确保能够访问目标服务器
2. **端口开放**: 确认目标端口未被防火墙阻止
3. **协议支持**: 确认服务器支持 Stratum 协议
4. **服务状态**: 确认矿池服务正在运行

## 相关文档

- [配置文件说明](configuration.md)
- [矿池配置指南](pool-configuration.md)
- [网络故障排除](network-troubleshooting.md)
