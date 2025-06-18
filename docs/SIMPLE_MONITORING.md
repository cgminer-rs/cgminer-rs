# 简化监控系统

CGMiner-RS 现在使用轻量级的内置监控系统，替代了复杂的 Prometheus + Grafana 方案。

## 🎯 设计理念

**为个人挖矿用户设计的简单监控方案**

- ✅ **轻量级** - 无需外部服务器和复杂配置
- ✅ **易用性** - 开箱即用的Web界面
- ✅ **实时性** - 实时数据更新和状态显示
- ✅ **美观性** - 专为挖矿设计的现代化界面
- ✅ **高效性** - 低资源占用，不影响挖矿性能

## 🚀 快速开始

### 1. 配置监控

在 `config.toml` 中启用监控：

```toml
[monitoring]
# 启用监控系统
enabled = true

# 指标收集间隔 (秒)
metrics_interval = 30

# Web监控界面端口
web_port = 8888

# 告警阈值配置
[monitoring.alert_thresholds]
temperature_warning = 80.0
temperature_critical = 90.0
hashrate_drop_percent = 20.0
error_rate_percent = 5.0
max_temperature = 85.0
max_cpu_usage = 80.0
max_memory_usage = 90.0
max_device_temperature = 85.0
max_error_rate = 5.0
min_hashrate = 50.0
```

### 2. 启动挖矿程序

```bash
cargo run --release
```

### 3. 访问监控界面

打开浏览器访问：`http://localhost:8888`

## 📊 监控界面功能

### 主要指标卡片

- **⚡ 总算力** - 实时显示当前总算力
- **📊 份额统计** - 接受/拒绝份额和拒绝率
- **🌡️ 系统状态** - 温度、内存使用率、功耗
- **⏱️ 运行统计** - 运行时间、活跃设备、效率

### 设备监控

- 每个设备的详细状态
- 温度、算力、功耗监控
- 设备状态指示（正常/警告/错误）
- 错误率统计

### 矿池监控

- 矿池连接状态
- 网络延迟监控
- 份额统计
- 连接时间统计

### 详细统计

- 总运行时间
- 总份额数
- 平均算力
- 最佳份额
- 硬件错误统计

## 🎨 界面特性

### 响应式设计
- 支持桌面和移动设备
- 自适应布局
- 现代化UI设计

### 实时更新
- 每5秒自动刷新数据
- 实时状态指示器
- 自动重连机制

### 状态指示
- 🟢 正常状态 - 绿色指示
- 🟡 警告状态 - 黄色指示  
- 🔴 错误状态 - 红色指示
- ⚫ 离线状态 - 灰色指示

## 🔧 命令行监控

除了Web界面，还可以通过命令行获取状态摘要：

```rust
use cgminer_rs::monitoring::MonitoringSystem;

let monitoring = MonitoringSystem::new(config).await?;
monitoring.start().await?;

// 获取状态摘要
let summary = monitoring.get_status_summary().await;
println!("{}", summary);
```

输出示例：
```
📊 挖矿状态摘要
==================
⚡ 总算力: 105.60 GH/s
✅ 接受份额: 1250
❌ 拒绝份额: 15
📊 拒绝率: 1.18%
🔧 活跃设备: 2
🌡️ 温度: 72.3°C
💾 内存使用: 68.5%
⚡ 功耗: 165.8W
==================
🌐 Web界面: http://localhost:8888
```

## ⚙️ 配置选项

### 监控配置

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | true | 是否启用监控 |
| `metrics_interval` | u64 | 30 | 指标收集间隔（秒） |
| `web_port` | Option<u16> | 8888 | Web界面端口 |

### 告警阈值

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `temperature_warning` | f32 | 80.0 | 温度警告阈值（°C） |
| `temperature_critical` | f32 | 90.0 | 温度严重阈值（°C） |
| `hashrate_drop_percent` | f64 | 20.0 | 算力下降告警阈值（%） |
| `error_rate_percent` | f64 | 5.0 | 错误率告警阈值（%） |
| `max_temperature` | f32 | 85.0 | 最大温度阈值（°C） |
| `max_cpu_usage` | f64 | 80.0 | 最大CPU使用率（%） |
| `max_memory_usage` | f64 | 90.0 | 最大内存使用率（%） |
| `max_device_temperature` | f32 | 85.0 | 最大设备温度（°C） |
| `max_error_rate` | f64 | 5.0 | 最大错误率（%） |
| `min_hashrate` | f64 | 50.0 | 最小算力阈值（GH/s） |

## 🔄 从Prometheus迁移

如果您之前使用Prometheus监控，迁移到简化监控系统非常简单：

### 1. 更新配置文件

将 `prometheus_port` 改为 `web_port`：

```toml
# 旧配置
[monitoring]
prometheus_port = 9090

# 新配置  
[monitoring]
web_port = 8888
```

### 2. 移除外部依赖

不再需要：
- Prometheus服务器
- Grafana仪表板
- Docker容器
- 复杂的配置文件

### 3. 享受简化体验

- 更快的启动速度
- 更低的资源占用
- 更简单的配置
- 更美观的界面

## 🎯 适用场景

### ✅ 推荐使用场景

- **个人挖矿** - 家庭或小规模挖矿
- **简单监控** - 只需要基本的状态监控
- **资源受限** - 系统资源有限的环境
- **快速部署** - 需要快速启动和配置

### ❌ 不推荐场景

- **大规模矿场** - 需要复杂的监控和告警
- **企业级监控** - 需要与现有监控系统集成
- **高级分析** - 需要复杂的数据分析和查询
- **多租户环境** - 需要复杂的权限管理

## 🛠️ 故障排除

### Web界面无法访问

1. 检查端口是否被占用
2. 确认防火墙设置
3. 检查配置文件中的端口设置

### 数据不更新

1. 检查监控是否启用
2. 确认指标收集间隔设置
3. 查看程序日志

### 性能问题

1. 增加指标收集间隔
2. 减少历史数据保留数量
3. 检查系统资源使用情况

## 📝 开发说明

如果您需要自定义监控功能，可以参考：

- `src/monitoring/simple_web.rs` - Web监控器实现
- `web/dashboard.html` - 前端界面
- `web/style.css` - 样式文件
- `web/script.js` - JavaScript逻辑

## 🎉 总结

简化监控系统为个人挖矿用户提供了完美的监控解决方案：

- **简单** - 无需复杂配置
- **美观** - 现代化Web界面
- **高效** - 低资源占用
- **实用** - 专为挖矿设计

告别复杂的Prometheus配置，享受简单高效的挖矿监控体验！ 🚀
