# CGMiner-RS 软核CPU优化配置指南

## 概述

CGMiner-RS的软算法核心（Software Core）支持灵活的CPU使用配置，可以根据需求最大化利用CPU资源或限制CPU使用。本指南详细介绍如何配置软核以实现最佳性能。

## 🎯 核心配置参数

### 基础配置

```toml
[cores.btc_software]
enabled = true
device_count = 32            # 设备数量 - 直接影响CPU使用
min_hashrate = 1000000000.0  # 最小算力 (1 GH/s)
max_hashrate = 3000000000.0  # 最大算力 (3 GH/s)
error_rate = 0.002           # 错误率 (0.2%)
batch_size = 4000            # 批处理大小
work_timeout_ms = 2000       # 工作超时
```

### CPU绑定配置

```toml
[cores.btc_software.cpu_affinity]
enabled = true               # 启用CPU绑定
strategy = "intelligent"     # CPU绑定策略
avoid_hyperthreading = false # 是否避免超线程
prefer_performance_cores = true # 优先使用性能核心
```

## 🚀 CPU使用策略

### 1. 最大化CPU使用配置

**适用场景**: 专用挖矿机器，需要最大化算力输出

```toml
[cores.btc_software]
enabled = true
device_count = 64            # 大量设备数 (建议为CPU核心数的2-4倍)
min_hashrate = 2000000000.0  # 2 GH/s
max_hashrate = 8000000000.0  # 8 GH/s
error_rate = 0.001           # 极低错误率
batch_size = 8000            # 大批次处理
work_timeout_ms = 1500       # 快速响应

[cores.btc_software.cpu_affinity]
enabled = true
strategy = "performance_first"  # 性能优先
avoid_hyperthreading = false   # 利用超线程
prefer_performance_cores = true
```

### 2. 限制CPU使用配置

**适用场景**: 共享服务器，需要为其他应用保留CPU资源

```toml
[cores.btc_software]
enabled = true
device_count = 4             # 较少设备数
min_hashrate = 500000000.0   # 500 MH/s
max_hashrate = 1500000000.0  # 1.5 GH/s
error_rate = 0.01            # 适中错误率
batch_size = 1000            # 小批次处理
work_timeout_ms = 5000       # 较长超时

[cores.btc_software.cpu_affinity]
enabled = true
strategy = "manual"          # 手动指定CPU核心
manual_mapping = { 0 = 0, 1 = 1, 2 = 2, 3 = 3 }  # 只使用前4个核心
```

### 3. 智能自适应配置

**适用场景**: 动态负载环境，根据系统状态自动调整

```toml
[cores.btc_software]
enabled = true
device_count = 16            # 中等设备数
min_hashrate = 1000000000.0  # 1 GH/s
max_hashrate = 4000000000.0  # 4 GH/s
error_rate = 0.005           # 平衡错误率
batch_size = 3000            # 中等批次
work_timeout_ms = 3000       # 平衡超时

[cores.btc_software.cpu_affinity]
enabled = true
strategy = "intelligent"     # 智能分配
avoid_hyperthreading = false
prefer_performance_cores = true
```

## 📊 CPU绑定策略详解

### 可用策略

1. **`round_robin`** - 轮询分配
   - 将设备依次分配到不同CPU核心
   - 适合均匀负载分布

2. **`manual`** - 手动指定
   - 精确控制每个设备使用的CPU核心
   - 适合精细化资源管理

3. **`performance_first`** - 性能优先
   - 优先使用高性能CPU核心
   - 适合最大化性能场景

4. **`physical_only`** - 仅物理核心
   - 只使用物理CPU核心，避免超线程
   - 适合稳定性优先场景

5. **`intelligent`** - 智能分配
   - 根据系统特性自动选择最佳策略
   - 适合大多数场景

### 手动映射示例

```toml
# 8核心系统的手动映射示例
[cores.btc_software.cpu_affinity]
enabled = true
strategy = "manual"
manual_mapping = {
  0 = 0,   # 设备0 -> CPU核心0
  1 = 1,   # 设备1 -> CPU核心1
  2 = 2,   # 设备2 -> CPU核心2
  3 = 3,   # 设备3 -> CPU核心3
  4 = 4,   # 设备4 -> CPU核心4
  5 = 5,   # 设备5 -> CPU核心5
  6 = 6,   # 设备6 -> CPU核心6
  7 = 7    # 设备7 -> CPU核心7
}
```

## ⚡ 性能优化配置

### 高性能配置

```toml
[performance.hashrate_optimization]
base_hashrate = 3000000000.0      # 3 GH/s基础算力
hashrate_variance = 0.1           # ±10%变化范围
frequency_hashrate_factor = 1.8   # 频率-算力因子
voltage_hashrate_factor = 1.5     # 电压-算力因子
temperature_impact_factor = 0.98  # 温度影响因子
adaptive_adjustment = true        # 自适应调整

[performance.memory_optimization]
work_cache_size = 3000           # 工作缓存大小
result_cache_size = 30000        # 结果缓存大小
stats_retention_seconds = 3600   # 统计保留时间
enable_memory_pool = true        # 启用内存池
preallocated_memory_mb = 128     # 预分配内存

[performance.thread_optimization]
worker_threads_per_device = 1    # 每设备线程数
thread_priority = "High"         # 线程优先级
thread_stack_size_kb = 512       # 线程栈大小
enable_thread_pool = true        # 启用线程池
```

## 🔧 系统资源限制

### 资源限制配置

```toml
[limits]
max_memory_mb = 4096         # 最大内存使用 (4GB)
max_cpu_percent = 85         # 最大CPU使用率 (85%)
max_open_files = 8192        # 最大打开文件数
max_network_connections = 100 # 最大网络连接数
```

## 📈 不同场景的推荐配置

### 场景1: 专用挖矿服务器 (16核心)

```toml
[cores.btc_software]
device_count = 32            # 2倍核心数
strategy = "performance_first"
max_cpu_percent = 95         # 使用95%CPU
```

### 场景2: 开发测试环境 (8核心)

```toml
[cores.btc_software]
device_count = 4             # 保守配置
strategy = "round_robin"
max_cpu_percent = 50         # 限制50%CPU
```

### 场景3: 共享服务器 (24核心)

```toml
[cores.btc_software]
device_count = 12            # 使用一半核心
strategy = "manual"
max_cpu_percent = 60         # 限制60%CPU
manual_mapping = { 0 = 0, 1 = 2, 2 = 4, 3 = 6, 4 = 8, 5 = 10, 6 = 12, 7 = 14, 8 = 16, 9 = 18, 10 = 20, 11 = 22 }
```

## 🎛️ 动态调整

### 运行时监控

系统会自动监控以下指标：
- CPU使用率
- 内存使用率
- 算力输出
- 错误率
- 温度

### 自动调整策略

当检测到以下情况时，系统会自动调整：
- CPU使用率过高 -> 减少设备数量
- 温度过高 -> 降低算力目标
- 错误率过高 -> 调整批处理大小
- 内存不足 -> 清理缓存

## 🔍 监控和调试

### 查看CPU绑定状态

```bash
# 查看进程CPU绑定情况
ps -eo pid,psr,comm | grep cgminer

# 查看系统CPU使用率
htop

# 查看cgminer日志
tail -f cgminer.log | grep "CPU"
```

### 性能调优建议

1. **设备数量**: 建议为CPU核心数的1-4倍
2. **批处理大小**: 根据内存大小调整，通常1000-8000
3. **CPU绑定**: 在多核心系统上启用以提高性能
4. **内存预分配**: 启用内存池减少内存分配开销
5. **监控温度**: 确保CPU温度在安全范围内

## 🛠️ 配置工具

### 自动配置脚本

使用配置助手快速生成配置文件：

```bash
# 运行配置助手
./scripts/configure_software_core.sh
```

脚本会自动：
- 检测系统CPU和内存信息
- 根据使用场景推荐配置
- 生成优化的配置文件
- 提供使用建议

### CPU优化器

实时监控和动态调整CPU使用：

```bash
# 编译CPU优化器
cargo build --release --bin cpu_optimizer

# 运行CPU优化器 (目标70%CPU使用率)
./target/release/cpu_optimizer 70 4 32 8
```

参数说明：
- 第1个参数：目标CPU使用率 (%)
- 第2个参数：最小设备数量
- 第3个参数：最大设备数量
- 第4个参数：初始设备数量

## 📝 配置文件示例

完整的配置文件示例请参考：
- `examples/configs/software_core_max_cpu.toml` - 最大化CPU使用
- `examples/configs/software_core_limited_cpu.toml` - 限制CPU使用
- `examples/configs/software_core_balanced.toml` - 平衡配置

### 快速开始

1. **自动配置** (推荐)：
   ```bash
   ./scripts/configure_software_core.sh
   ```

2. **手动配置**：
   ```bash
   cp examples/configs/software_core_balanced.toml cgminer.toml
   # 编辑 cgminer.toml 根据需要调整参数
   ```

3. **启动挖矿**：
   ```bash
   ./target/release/cgminer-rs --config cgminer.toml
   ```

4. **监控优化**：
   ```bash
   ./target/release/cpu_optimizer 70 4 32 16
   ```
