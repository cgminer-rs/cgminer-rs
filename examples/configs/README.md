# CGMiner-RS 软算法核心配置文件说明

本目录包含了多个针对不同使用场景优化的软算法核心配置文件。

## 配置文件列表

### 1. `software_core_example.toml` - 标准示例配置
**适用场景**: 一般使用、学习和基础挖矿
**特点**:
- 平衡的性能设置
- 启用CPU绑定优化
- 包含F2Pool矿池配置
- 适中的监控和日志级别
- 包含性能优化和高级配置选项

**使用方法**:
```bash
cargo run --bin cgminer-rs --features software-core -- --config examples/configs/software_core_example.toml
```

### 2. `software_core_dev.toml` - 开发测试配置
**适用场景**: 开发、调试和功能测试
**特点**:
- 详细的调试日志 (debug级别)
- 启用所有调试功能
- 较短的超时时间便于快速测试
- 包含本地测试矿池配置
- 较小的资源占用

**使用方法**:
```bash
cargo run --bin cgminer-rs --features software-core -- --config examples/configs/software_core_dev.toml
```

### 3. `software_core_offline.toml` - 离线测试配置
**适用场景**: 离线演示、测试和开发
**特点**:
- 不依赖网络连接
- 启用硬件模拟模式
- 使用本地模拟矿池
- 适中的性能设置
- 完全离线运行

**使用方法**:
```bash
cargo run --bin cgminer-rs --features software-core -- --config examples/configs/software_core_offline.toml
```

### 4. `software_core_performance.toml` - 高性能配置
**适用场景**: 专用挖矿机器、高端硬件
**特点**:
- 最大化设备数量和算力
- 高优先级线程和大内存池
- 启用所有SIMD优化
- 负载均衡矿池策略
- 严格的性能监控阈值

**使用方法**:
```bash
cargo run --bin cgminer-rs --features software-core -- --config examples/configs/software_core_performance.toml
```

## 配置文件结构说明

### 基础配置部分

#### `[general]` - 通用配置
- `log_level`: 日志级别 (debug, info, warn, error)
- `log_file`: 日志文件路径
- `work_restart_timeout`: 工作重启超时时间
- `scan_time`: 设备扫描间隔

#### `[cores.software_core]` - 软算法核心配置
- `enabled`: 是否启用软算法核心
- `device_count`: 虚拟设备数量
- `min_hashrate`/`max_hashrate`: 算力范围
- `error_rate`: 模拟错误率
- `batch_size`: 批处理大小
- `work_timeout_ms`: 工作超时时间

#### `[cores.software_core.cpu_affinity]` - CPU绑定配置
- `enabled`: 是否启用CPU绑定
- `strategy`: 绑定策略 (round_robin, manual, performance_first)
- `manual_mapping`: 手动CPU核心映射

### 高级配置部分

#### `[performance]` - 性能优化
- `cpu_optimization`: CPU优化开关
- `thread_priority`: 线程优先级
- `memory_pool_size`: 内存池大小
- `use_sse`/`use_avx`: SIMD指令集优化

#### `[debug]` - 调试配置
- `enabled`: 调试模式开关
- `verbose_logging`: 详细日志
- `performance_profiling`: 性能分析
- `memory_tracking`: 内存跟踪

#### `[test_mode]` - 测试模式
- `enabled`: 测试模式开关
- `simulate_hardware`: 硬件模拟
- `offline_mode`: 离线模式

## 使用建议

### 开发阶段
推荐使用 `software_core_dev.toml`:
- 详细的调试信息
- 快速的测试周期
- 完整的错误跟踪

### 功能测试
推荐使用 `software_core_offline.toml`:
- 不需要网络连接
- 稳定的测试环境
- 可重复的测试结果

### 性能测试
推荐使用 `software_core_performance.toml`:
- 最大化系统资源利用
- 真实的挖矿性能评估
- 生产环境配置预览

### 日常使用
推荐使用 `software_core_example.toml`:
- 平衡的性能和稳定性
- 适中的资源占用
- 完整的功能展示

## 自定义配置

您可以基于这些示例配置创建自己的配置文件：

1. 复制最接近您需求的配置文件
2. 根据您的硬件和需求调整参数
3. 测试配置的稳定性和性能
4. 根据实际运行情况进行微调

## 配置验证

在使用自定义配置前，建议运行配置验证：

```bash
cargo run --example test_software_core_config
```

这将验证配置文件的语法和参数合理性。

## 故障排除

### 常见问题

1. **矿池连接失败**
   - 检查网络连接
   - 验证矿池URL和端口
   - 确认用户名和密码

2. **CPU使用率过高**
   - 减少 `device_count`
   - 降低 `thread_priority`
   - 启用CPU绑定优化

3. **内存使用过多**
   - 减少 `memory_pool_size`
   - 降低 `work_queue_size`
   - 减少 `batch_size`

4. **算力不稳定**
   - 检查CPU温度和频率
   - 调整 `cpu_affinity` 设置
   - 优化系统负载

### 日志分析

查看日志文件了解详细的运行状态：
```bash
tail -f logs/cgminer-*.log
```

### 性能监控

通过API接口监控实时状态：
```bash
curl http://localhost:4028/api/status
```

## 技术支持

如果遇到配置问题，请：
1. 检查日志文件中的错误信息
2. 验证配置文件语法
3. 参考本文档的故障排除部分
4. 在项目仓库提交Issue
