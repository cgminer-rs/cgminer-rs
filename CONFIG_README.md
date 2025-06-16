# CGMiner-RS 配置文件说明

本项目提供了三种不同的配置文件，用于不同的使用场景：

## 配置文件类型

### 1. `cgminer.toml` - 通用配置文件
- 包含虚拟设备和 ASIC 设备的完整配置选项
- 通过 `device_mode` 参数控制使用哪种设备类型
- 适合需要在不同模式间切换的场景

### 2. `cgminer-virtual.toml` - 虚拟设备测试配置
- 专门用于虚拟设备测试和开发
- 优化了测试环境的参数设置
- 启用了调试功能和详细日志
- 使用测试矿池和本地矿池

### 3. `cgminer-asic.toml` - ASIC 设备生产配置
- 专门用于 ASIC 设备的实际挖矿
- 优化了生产环境的参数设置
- 启用了监控、告警和通知功能
- 配置了生产矿池和安全选项

## 使用方法

### 虚拟设备测试模式
```bash
# 使用虚拟设备配置文件
./cgminer-rs --config cgminer-virtual.toml

# 或者使用通用配置文件并设置虚拟模式
./cgminer-rs --config cgminer.toml --device-mode virtual
```

### ASIC 设备生产模式
```bash
# 使用 ASIC 设备配置文件
./cgminer-rs --config cgminer-asic.toml

# 或者使用通用配置文件并设置 ASIC 模式
./cgminer-rs --config cgminer.toml --device-mode asic
```

### 混合模式
```bash
# 同时使用虚拟设备和 ASIC 设备
./cgminer-rs --config cgminer.toml --device-mode mixed
```

## 配置参数说明

### 设备模式 (device_mode)
- `virtual`: 仅使用虚拟设备，用于测试和开发
- `asic`: 仅使用 ASIC 设备，用于实际挖矿
- `mixed`: 同时使用两种设备类型

### 虚拟设备配置 (devices.virtual)
- `device_count`: 虚拟设备数量
- `min_hashrate/max_hashrate`: 模拟算力范围
- `error_rate`: 模拟硬件错误率
- `batch_size`: 挖矿批次大小

### ASIC 设备配置 (devices.asic)
- `chains`: 链配置数组，每条链对应一个 ASIC 芯片组
- `auto_tuning`: 自动调优参数
- `frequency/voltage`: 硬件工作参数

### 矿池配置
- `pools.virtual`: 虚拟设备使用的测试矿池
- `pools.asic`: ASIC 设备使用的生产矿池

## 快速开始

### 1. 测试环境设置
```bash
# 复制虚拟设备配置
cp cgminer-virtual.toml my-test-config.toml

# 编辑配置文件，修改矿池地址和用户信息
vim my-test-config.toml

# 启动测试
./cgminer-rs --config my-test-config.toml
```

### 2. 生产环境设置
```bash
# 复制 ASIC 设备配置
cp cgminer-asic.toml my-production-config.toml

# 编辑配置文件，修改以下重要参数：
# - 矿池地址和认证信息
# - ASIC 设备硬件参数
# - 监控和告警设置
# - 安全和认证选项
vim my-production-config.toml

# 启动生产挖矿
./cgminer-rs --config my-production-config.toml
```

## 重要注意事项

### 虚拟设备测试
- 虚拟设备仅用于测试，不会产生实际的挖矿收益
- 使用较低的日志级别 (debug) 以便调试
- 可以连接到测试网络或本地测试矿池
- 适合开发和功能验证

### ASIC 设备生产
- 需要根据实际硬件规格调整频率、电压等参数
- 建议启用监控和告警功能
- 使用生产矿池和真实的用户凭据
- 注意安全设置和访问控制

### 配置文件管理
- 定期备份配置文件
- 使用版本控制管理配置变更
- 敏感信息（如密码）建议使用环境变量
- 生产环境配置文件权限应设置为只读

## 故障排除

### 常见问题
1. **设备未检测到**: 检查 `device_mode` 设置和对应的设备配置
2. **矿池连接失败**: 验证矿池地址、端口和认证信息
3. **性能问题**: 调整 `batch_size`、`worker_threads` 等性能参数
4. **温度过高**: 检查风扇设置和温度限制参数

### 日志分析
- 虚拟设备测试：使用 `debug` 级别日志
- 生产环境：使用 `info` 级别日志
- 错误排查：临时提升到 `debug` 级别

### 监控建议
- 启用 Prometheus 指标收集
- 配置适当的告警阈值
- 使用 Web UI 进行实时监控
- 定期检查设备健康状态

## 更多信息

详细的配置选项说明请参考：
- `docs/configuration.md` - 完整配置文档
- `docs/virtual-devices.md` - 虚拟设备详细说明
- `docs/asic-devices.md` - ASIC 设备配置指南
- `docs/monitoring.md` - 监控和告警配置
