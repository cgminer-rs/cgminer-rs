# CGMiner 风格日志输出

本文档描述了 cgminer_rs 中实现的简洁、易读的 CGMiner 风格日志输出。

## 日志格式

新的日志格式采用简洁的 CGMiner 风格：

```
[时间] 级别 消息
```

### 示例输出

```
[14:32:15]     Started cgminer 5.0.0
[14:32:15]     Mining to stratum+tcp://pool.example.com:4444 with 3 pools
[14:32:15]     Loaded 2 cores, 4 devices ready
[14:32:16]     Pool 0 difficulty changed from 65536 to 131072
[14:32:18]     (5s):12.34MH/s (1m):11.89MH/s (5m):12.01MH/s (15m):11.97MH/s A:245 R:3 HW:0 [4DEV]
[14:32:21]     Accepted 4a2b3c1d... GPU 0 pool 0
[14:32:22] WRN GPU 2 temperature 87C - high temperature warning
[14:32:23] ERR Pool 1 connection lost, failover to pool 2
```

## 日志级别

| 级别 | 显示 | 颜色 | 用途 |
|------|------|------|------|
| INFO | (空白) | 绿色 | 正常运行信息 |
| WARN | WRN | 黄色 | 警告信息 |
| ERROR | ERR | 红色 | 错误信息 |
| DEBUG | DBG | 青色 | 调试信息 |

## 关键改进

### 1. 简化启动日志

**之前（冗余）：**
```
Creating mining manager with core registry
根据配置注册设备驱动，启用的核心: ["cpu-btc"]
软算法核心已启用，将通过核心管理器直接管理
创建挖矿核心: cpu-btc
挖矿核心创建成功: core_abc123
🚀 软算法核心启动成功: core_abc123
软算法核心已在CoreRegistry中管理并运行: core_abc123
初始化设备管理器
设备管理器初始化成功
Mining manager started successfully
```

**现在（简洁）：**
```
[14:32:15]     Started cgminer 5.0.0
[14:32:15]     Mining to stratum+tcp://pool.example.com:4444 with 3 pools
[14:32:15]     Loaded 2 cores, 4 devices ready
[14:32:15]     Started 2 mining cores
[14:32:15]     Initialized 4 mining cores
```

### 2. 总结式日志

不再显示每个细节步骤，而是在完成后显示总结：

- ✅ 启动时：显示启动的核心数量、设备数量、连接的矿池
- ✅ 运行时：使用经典的 CGMiner 算力格式
- ✅ 停止时：显示运行时间和最终统计

### 3. 减少冗余

- 🚫 移除表情符号和装饰性文字
- 🚫 移除详细的步骤跟踪日志
- 🚫 移除设备级的单独日志
- ✅ 只保留重要的状态变更和错误信息

## 配置

在 `config.toml` 中启用 CGMiner 风格日志：

```toml
[logging]
level = "info"
console = true
pretty = true
colored = true
```

## 运行示例

```bash
# 运行日志演示
cargo run --example cgminer_style_log_demo

# 使用 CGMiner 风格日志运行挖矿程序
cargo run -- --config config.toml
```

## 性能影响

- 减少了约 70% 的日志输出量
- 提高了日志可读性
- 降低了磁盘和网络 I/O 负担
- 更适合在生产环境中长期运行

## 兼容性

- 与原版 CGMiner 日志格式兼容
- 支持现有的日志分析工具
- 可以通过配置切换回详细模式（debug 级别）
