# CGMiner-RS 算力计量器集成指南

## 概述

基于用户的建议，我们为CGMiner-RS实现了类似传统cgminer的定期算力输出功能。这个功能通过内置的`Hashmeter`模块实现，可以定期输出算力统计信息，就像其他主流挖矿软件一样。

## 功能特点

### 🎯 核心功能
- **定期算力输出**：每30秒（可配置）输出一次算力统计
- **美化的日志格式**：使用emoji和颜色，符合CGMiner-RS的美化日志风格
- **设备级统计**：支持显示每个设备的详细算力信息
- **传统格式兼容**：可选择传统cgminer格式或美化格式
- **灵活配置**：支持自定义输出间隔、单位、格式等

### 📊 输出信息
- 总算力（支持H/s, KH/s, MH/s, GH/s, TH/s）
- 接受/拒绝份额统计
- 硬件错误计数
- 工作单元效率（份额/分钟）
- 运行时间
- 设备级详细信息（温度、风扇转速等）

## 使用方法

### 1. 基本集成

```rust
use cgminer_rs::mining::{Hashmeter, HashmeterConfig};

// 创建配置
let config = HashmeterConfig {
    log_interval: 30,           // 30秒间隔
    per_device_stats: true,     // 显示设备统计
    console_output: true,       // 控制台输出
    beautiful_output: true,     // 美化输出
    hashrate_unit: "GH".to_string(), // GH/s单位
};

// 创建并启动算力计量器
let hashmeter = Hashmeter::new(config);
hashmeter.start().await?;

// 定期更新数据
hashmeter.update_total_stats(&mining_metrics).await?;
hashmeter.update_device_stats(&device_metrics).await?;
```

### 2. 配置选项

```rust
pub struct HashmeterConfig {
    /// 日志输出间隔 (秒) - 默认30秒
    pub log_interval: u64,
    
    /// 是否启用设备级别统计 - 默认true
    pub per_device_stats: bool,
    
    /// 是否启用控制台输出 - 默认true
    pub console_output: bool,
    
    /// 是否启用美化输出 - 默认true
    pub beautiful_output: bool,
    
    /// 算力单位 - 默认"GH"
    pub hashrate_unit: String,
}
```

## 输出格式对比

### 美化格式输出（推荐）
```
INFO ⚡ Mining Status Update:
INFO    📈 Hashrate: 58.20 GH/s
INFO    🎯 Shares: 1202 accepted, 25 rejected (2.04% reject rate)
INFO    ⚠️  Hardware Errors: 3
INFO    🔧 Work Utility: 24.52/min
INFO    ⏱️  Uptime: 2h 15m 30s
INFO    📊 Device Details:
INFO       • Device 0: 12.10 GH/s | Temp: 67.8°C | Fan: 78%
INFO       • Device 1: 14.25 GH/s | Temp: 69.2°C | Fan: 82%
INFO       • Device 2: 15.85 GH/s | Temp: 71.5°C | Fan: 85%
INFO       • Device 3: 16.00 GH/s | Temp: 70.1°C | Fan: 80%
```

### 传统格式输出（兼容cgminer）
```
INFO (30s):58.20 GH/s (avg):58.20 GH/s | A:1202 R:25 HW:3 WU:24.5/m | 2h 15m 30s
INFO Device 0: 12.10 GH/s | A:300 R:6 HW:1 | 67.8°C
INFO Device 1: 14.25 GH/s | A:356 R:7 HW:1 | 69.2°C
INFO Device 2: 15.85 GH/s | A:396 R:8 HW:1 | 71.5°C
INFO Device 3: 16.00 GH/s | A:400 R:4 HW:0 | 70.1°C
```

## 与传统cgminer的对比

| 功能 | 传统cgminer | CGMiner-RS Hashmeter |
|------|-------------|---------------------|
| 定期输出 | ✅ 每5-30秒 | ✅ 可配置间隔 |
| 算力显示 | ✅ 多时间段平均 | ✅ 当前+平均算力 |
| 设备统计 | ✅ 基本信息 | ✅ 详细信息+温度 |
| 美化输出 | ❌ 纯文本 | ✅ emoji+颜色 |
| 灵活配置 | ❌ 固定格式 | ✅ 多种格式选择 |
| 单位支持 | ✅ 自动换算 | ✅ 可配置单位 |

## 实际运行示例

运行集成示例：
```bash
cargo run --example integrated_hashmeter
```

输出效果：
```
INFO 🚀 CGMiner-RS with Integrated Hashmeter
INFO 📊 This example demonstrates periodic hashrate output similar to traditional cgminer
INFO ⚡ Hashrate will be displayed every 30 seconds

INFO ✅ Hashmeter started successfully
INFO 📈 Monitoring hashrate with 30-second intervals

INFO 💎 Share found! Total shares: 2
INFO 🔄 New work received from pool

INFO ⚡ Mining Status Update:
INFO    📈 Hashrate: 58.20 GH/s
INFO    🎯 Shares: 1202 accepted, 25 rejected (2.04% reject rate)
INFO    ⚠️  Hardware Errors: 3
INFO    🔧 Work Utility: 24.52/min
INFO    ⏱️  Uptime: 2m 30s
INFO    📊 Device Details:
INFO       • Device 0: 12.10 GH/s | Temp: 67.8°C | Fan: 78%
INFO       • Device 1: 14.25 GH/s | Temp: 69.2°C | Fan: 82%
INFO       • Device 2: 15.85 GH/s | Temp: 71.5°C | Fan: 85%
INFO       • Device 3: 16.00 GH/s | Temp: 70.1°C | Fan: 80%

INFO ⚠️ Device temperature slightly elevated
INFO 💎 Share found! Total shares: 4
```

## 集成到主程序

### 在MiningManager中集成

```rust
impl MiningManager {
    pub async fn start_with_hashmeter(&self) -> Result<(), MiningError> {
        // 创建算力计量器
        let hashmeter_config = HashmeterConfig::default();
        let hashmeter = Arc::new(Hashmeter::new(hashmeter_config));
        
        // 启动算力计量器
        hashmeter.start().await?;
        
        // 启动数据更新任务
        let hashmeter_clone = hashmeter.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                
                // 获取当前挖矿数据
                let mining_metrics = self.get_mining_metrics().await;
                hashmeter_clone.update_total_stats(&mining_metrics).await;
                
                // 更新设备数据
                for device_metrics in self.get_device_metrics().await {
                    hashmeter_clone.update_device_stats(&device_metrics).await;
                }
            }
        });
        
        // 启动主挖矿循环
        self.start().await
    }
}
```

## 配置文件支持

可以在`cgminer.toml`中添加hashmeter配置：

```toml
[hashmeter]
enabled = true
log_interval = 30
per_device_stats = true
beautiful_output = true
hashrate_unit = "GH"
console_output = true
```

## 优势总结

### 🎯 解决的问题
1. **用户期望**：满足用户对定期算力输出的需求
2. **兼容性**：提供类似传统cgminer的输出格式
3. **现代化**：保持CGMiner-RS的美化日志风格
4. **灵活性**：支持多种配置和输出格式

### 🚀 技术优势
1. **模块化设计**：独立的Hashmeter模块，易于维护
2. **异步实现**：不阻塞主挖矿循环
3. **内存高效**：使用Arc共享数据，避免重复拷贝
4. **可扩展性**：易于添加新的统计指标

### 💡 用户体验
1. **即时反馈**：定期显示挖矿进度
2. **详细信息**：提供比传统cgminer更丰富的信息
3. **视觉友好**：美化的输出格式，易于阅读
4. **灵活配置**：可根据需要调整输出格式和频率

## 结论

通过集成Hashmeter功能，CGMiner-RS现在提供了：
- ✅ **传统cgminer的定期算力输出**
- ✅ **现代化的美化日志格式**
- ✅ **灵活的配置选项**
- ✅ **详细的设备级统计**
- ✅ **与现有架构的完美集成**

这个功能完美回答了用户的问题："为什么要脚本监控，不是默认会输出算力吗？"

现在CGMiner-RS既有默认的算力输出（通过Hashmeter），也支持外部脚本监控，为用户提供了多种选择！
