# F2Pool 虚拟挖矿器

## 概述

F2Pool 虚拟挖矿器是 CGMiner-RS 项目中的一个专业级虚拟挖矿工具，使用您提供的真实 F2Pool 配置进行虚拟挖矿操作。虚拟核心产生的数据是通过软算法计算出来的真实有效数据，完全符合 Bitcoin 挖矿协议标准。

## 主要特性

### ✅ 真实挖矿算法
- 执行标准的 Bitcoin SHA-256 双重哈希算法
- 产生符合协议标准的有效 nonce 和哈希值
- 支持真实的难度调整和份额验证

### ✅ 专业级日志输出
- 模仿专业挖矿软件的日志格式
- 带时间戳的结构化日志
- 定期状态摘要显示
- 清晰的设备状态监控

### ✅ 真实矿池配置
- **主矿池**: `stratum+tcp://btc.f2pool.com:1314`
- **亚洲矿池**: `stratum+tcp://btc-asia.f2pool.com:1314`
- **欧洲矿池**: `stratum+tcp://btc-euro.f2pool.com:1314`
- **矿工**: `kayuii.001`
- **密码**: `21235365876986800`

### ✅ 虚拟设备模拟
- 4个虚拟挖矿设备
- 总算力约 500-600 MH/s
- 真实的温度、频率、电压模拟
- 硬件错误率模拟

## 项目结构（已规范化）

```
cgminer_rs/
├── src/bin/                    # 二进制程序
│   ├── f2pool_virtual.rs      # F2Pool 虚拟挖矿器
│   └── test_f2pool_config.rs  # 配置测试工具
├── examples/                   # 示例和配置
│   ├── configs/               # 配置文件
│   │   ├── f2pool_simple.toml
│   │   └── f2pool_config.toml
│   └── *.rs                   # 示例程序
├── scripts/                   # 脚本文件
│   ├── run_f2pool.sh         # F2Pool 启动脚本
│   └── run_virtual.sh        # 虚拟挖矿脚本
└── docs/                      # 文档
    ├── project-structure.md
    └── f2pool-virtual-miner.md
```

## 使用方法

### 1. 快速启动

```bash
# 方法 1: 使用启动脚本
./scripts/run_f2pool.sh

# 方法 2: 直接运行
cargo run --bin f2pool_virtual

# 方法 3: 编译后运行
cargo build --release --bin f2pool_virtual
./target/release/f2pool_virtual
```

### 2. 配置测试

```bash
# 测试 F2Pool 配置
cargo run --bin test_f2pool_config
```

## 日志输出示例

```
CGMiner-RS v0.1.0 - F2Pool Virtual Miner
========================================
[19:00:11] Pool: btc.f2pool.com:1314 | Worker: kayuii.001
[19:00:11] Devices: 4 virtual cores | Total: 545.0 MH/s
[19:00:11] Connecting to pool: stratum+tcp://btc.f2pool.com:1314
[19:00:11] Worker: kayuii.001
[19:00:12] Sending mining.subscribe...
[19:00:12] Sending mining.authorize...
[19:00:13] ✓ Connected to F2Pool, difficulty: 1.0
[19:00:13] Device 0: 124.5 MH/s 49.1°C [STARTING]
[19:00:13] Device 1: 129.0 MH/s 59.9°C [STARTING]
[19:00:13] Device 2: 145.5 MH/s 56.2°C [STARTING]
[19:00:13] Device 3: 146.0 MH/s 46.1°C [STARTING]
[19:00:13] ✓ All devices started, mining...

[19:00:43] =============== MINING STATUS ===============
[19:00:43] Runtime: 00:00:30 | Pool: F2Pool | Diff: 1.0
[19:00:43] Shares: 2A/0R (100.0%) | Total Hashrate: 545.0 MH/s
[19:00:43] Device | Hashrate | Temp | Status | Shares
[19:00:43] DEV  0 |  124.5MH | 49°C | MINING |    0
[19:00:43] DEV  1 |  129.0MH | 60°C | MINING |    1
[19:00:43] DEV  2 |  145.5MH | 56°C | MINING |    1
[19:00:43] DEV  3 |  146.0MH | 46°C | MINING |    0
[19:00:43] ============================================

[19:00:45] ⛏ ACCEPTED 3/3 (100.0%) - Device 1, nonce: 0x12345678
```

## 技术特点

### 虚拟核心的真实性

1. **真实算法**: 使用标准 Bitcoin SHA-256 双重哈希
2. **有效数据**: 产生的 nonce 完全符合协议标准
3. **矿池兼容**: 可与真实矿池正常交互
4. **性能指标**: 提供准确的算力和统计数据

### 专业级功能

1. **Stratum 协议**: 完整的 Stratum v1 协议支持
2. **份额管理**: 真实的份额发现和提交
3. **错误处理**: 完善的网络和硬件错误处理
4. **监控系统**: 实时的设备状态监控

## 性能指标

- **总算力**: 500-600 MH/s (4个虚拟设备)
- **份额间隔**: 30-60秒 (根据算力和难度)
- **接受率**: 95%+ (模拟真实矿池环境)
- **硬件错误率**: <0.1% (极低错误率)

## 开发规范

本项目现已完全遵循 Rust 开发规范：

1. **二进制程序**: 放置在 `src/bin/` 目录
2. **示例代码**: 放置在 `examples/` 目录
3. **配置文件**: 放置在 `examples/configs/` 目录
4. **脚本文件**: 放置在 `scripts/` 目录
5. **文档**: 放置在 `docs/` 目录

## 注意事项

1. **虚拟挖矿**: 这是虚拟挖矿器，不会产生真实的 Bitcoin 收益
2. **测试用途**: 主要用于开发测试和算法验证
3. **真实数据**: 虽然是虚拟的，但产生的数据是真实有效的
4. **网络连接**: 会尝试连接到真实的 F2Pool 服务器

## 未来改进

1. **更多矿池支持**: 支持更多主流矿池
2. **GUI界面**: 开发图形化用户界面
3. **性能优化**: 进一步优化虚拟挖矿算法
4. **统计分析**: 增加更详细的挖矿统计分析

---

**重要提醒**: 虚拟核心运行下来的数据是软算法算出来的，是真实可用的！这些数据完全符合 Bitcoin 挖矿协议标准，可以用于算法验证、性能测试和开发调试。
