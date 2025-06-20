# CGMiner-RS 核心架构

CGMiner-RS 采用了模块化的核心架构，将不同类型的挖矿设备驱动分离到独立的库中，支持动态加载和配置。

## 架构概述

```
cgminer-rs (主程序)
├── cgminer-core (核心特征和类型定义)
├── cgminer-cpu-btc-core (软算法Bitcoin挖矿核心)
├── cgminer-asic-maijie-l7-core (Maijie L7 ASIC硬件挖矿核心)
└── 其他核心库...
```

## 核心库说明

### 1. cgminer-core
基础库，定义了所有挖矿核心必须实现的特征和类型：

- `MiningCore` - 挖矿核心特征
- `MiningDevice` - 挖矿设备特征
- `CoreFactory` - 核心工厂特征
- `CoreRegistry` - 核心注册表
- 基础类型：`Work`, `MiningResult`, `HashRate`, `Temperature` 等

### 2. cgminer-cpu-btc-core
软算法挖矿核心，使用CPU进行真实的SHA256算法计算：

- 支持多个虚拟设备
- 真实的软件算法实现
- 可配置的算力范围和错误率
- 适用于测试、开发和低功耗挖矿场景

**特性：**
- 真实的SHA256双重哈希计算
- 可配置的设备数量和算力
- 模拟的温度、电压、频率监控
- 支持批量处理以优化性能

### 3. cgminer-asic-maijie-l7-core
ASIC硬件挖矿核心，支持真实的ASIC矿机：

- 支持Maijie L7等ASIC矿机
- 硬件接口抽象层（SPI、UART、GPIO、PWM）
- 温度、电压、电流、功率监控
- 自动调优和风扇控制

**特性：**
- 真实的硬件接口支持
- Maijie L7专用驱动
- 完整的传感器监控
- 自动调优和保护机制

## 使用方法

### 1. 基本使用

```rust
use cgminer_rs::{CoreLoader, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建核心加载器
    let core_loader = CoreLoader::new();

    // 加载所有可用的核心
    core_loader.load_all_cores().await?;

    // 列出已加载的核心
    let cores = core_loader.list_loaded_cores()?;
    for core in cores {
        println!("已加载核心: {} ({})", core.name, core.core_type);
    }

    Ok(())
}
```

### 2. 创建软算法核心

```rust
use cgminer_core::{CoreConfig, CoreRegistry};
use std::collections::HashMap;

// 创建软算法核心配置
let mut custom_params = HashMap::new();
custom_params.insert("device_count".to_string(), serde_json::Value::Number(4.into()));
custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(1_000_000_000.0.into()));
custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(5_000_000_000.0.into()));

let config = CoreConfig {
    name: "my-software-core".to_string(),
    enabled: true,
    devices: vec![/* 设备配置 */],
    custom_params,
};

// 创建核心实例
let registry = core_loader.registry();
let core_id = registry.create_core("software", config).await?;
```

### 3. 配置文件

#### 软算法核心配置 (cgminer-software.toml)
```toml
[cores]
enabled_cores = ["software"]
default_core = "software"

[cores.software_core]
enabled = true
device_count = 4
min_hashrate = 1000000000.0  # 1 GH/s
max_hashrate = 5000000000.0  # 5 GH/s
error_rate = 0.01            # 1%
batch_size = 1000
work_timeout_ms = 5000
```

#### ASIC核心配置 (cgminer-asic-maijie-l7-core.toml)
```toml
[cores]
enabled_cores = ["asic"]
default_core = "asic"

[cores.asic_core]
enabled = true
chain_count = 3
spi_speed = 6000000          # 6MHz
uart_baud = 115200
auto_detect = true
power_limit = 3500.0         # 3.5kW
cooling_mode = "auto"
```

## 编译特性

- `software-core` - 启用软算法核心支持
- `asic-core` - 启用ASIC核心支持
- `mock-hardware` - 使用模拟硬件接口（用于测试）
- `dynamic-loading` - 启用动态库加载支持

### 编译示例

```bash
# 仅软算法核心
cargo build --features software-core

# 仅ASIC核心
cargo build --features asic-core

# 同时支持两种核心
cargo build --features "software-core,asic-core"

# 使用模拟硬件（测试）
cargo build --features "asic-core,mock-hardware"
```

## 扩展新核心

要添加新的挖矿核心类型，需要：

1. 创建新的核心库（如 `cgminer-fpga-core`）
2. 实现 `cgminer-core` 中定义的特征
3. 提供 C FFI 导出函数用于动态加载
4. 在主程序中注册新核心

### 示例核心库结构

```
cgminer-new-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── core.rs      # 实现 MiningCore
│   ├── device.rs    # 实现 MiningDevice
│   ├── factory.rs   # 实现 CoreFactory
│   └── hardware.rs  # 硬件接口（如需要）
```

## 测试

运行集成测试：

```bash
# 测试所有核心
cargo test --features "software-core,asic-core"

# 仅测试软算法核心
cargo test --features software-core test_software_core

# 仅测试ASIC核心
cargo test --features asic-core test_asic_core
```

## 性能考虑

### 软算法核心
- CPU密集型，建议根据CPU核心数配置设备数量
- 批次大小影响CPU使用率和响应性
- 适合开发测试，不适合大规模生产挖矿

### ASIC核心
- 硬件加速，算力远超软算法核心
- 需要适当的散热和电源管理
- 支持自动调优以平衡性能和稳定性

## 故障排除

### 常见问题

1. **核心加载失败**
   - 检查编译特性是否正确启用
   - 验证配置文件格式
   - 查看日志中的详细错误信息

2. **ASIC设备无法检测**
   - 确认硬件连接正常
   - 检查SPI/UART设备权限
   - 验证设备驱动是否正确安装

3. **软算法核心性能低**
   - 调整批次大小
   - 减少设备数量
   - 检查CPU使用率

### 调试模式

启用详细日志：
```bash
RUST_LOG=debug cargo run --features "software-core,asic-core"
```

## 贡献

欢迎贡献新的核心实现或改进现有核心。请确保：

1. 遵循现有的代码风格
2. 添加适当的测试
3. 更新相关文档
4. 通过所有CI检查

## 许可证

本项目采用 GPL-3.0 许可证。详见 LICENSE 文件。
