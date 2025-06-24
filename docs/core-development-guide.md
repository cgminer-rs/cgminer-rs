# CGMiner-RS 核心开发指南

## 📖 概述

本指南详细说明如何开发符合 CGMiner-RS 架构边界的外置挖矿核心。外置核心是独立的库，通过标准化接口与应用层通信，专注于挖矿算法实现和硬件控制。

## 🏗️ 核心架构原则

### 职责边界
- ✅ **核心职责**: 挖矿算法、设备控制、性能优化、硬件监控
- ❌ **禁止领域**: 网络连接、全局配置、Web界面、系统级监控

### 依赖关系
```text
外置核心 → cgminer-core (标准接口)
外置核心 ❌ cgminer-rs (应用层)
外置核心 ❌ 其他外置核心
```

## 🔌 标准化接口实现

### 1. 核心工厂接口 (CoreFactory)

所有外置核心必须实现 `CoreFactory` trait：

```rust
use cgminer_core::{CoreFactory, CoreConfig, MiningCore, Result};
use async_trait::async_trait;

pub struct MyCoreFactory;

#[async_trait]
impl CoreFactory for MyCoreFactory {
    /// 创建核心实例
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>> {
        let core = MyMiningCore::new(config).await?;
        Ok(Box::new(core))
    }
    
    /// 验证配置有效性
    fn validate_config(&self, config: &CoreConfig) -> Result<()> {
        // 验证核心特定的配置参数
        if config.device_count == 0 {
            return Err("设备数量不能为0".into());
        }
        Ok(())
    }
    
    /// 提供默认配置
    fn default_config(&self) -> CoreConfig {
        CoreConfig {
            device_count: 1,
            algorithm: "sha256d".to_string(),
            // ... 其他默认值
        }
    }
}
```

### 2. 挖矿核心接口 (MiningCore)

```rust
use cgminer_core::{MiningCore, CoreInfo, CoreCapabilities, CoreStats, Work, MiningResult};

pub struct MyMiningCore {
    info: CoreInfo,
    capabilities: CoreCapabilities,
    devices: Vec<Box<dyn MiningDevice>>,
    // ... 其他字段
}

#[async_trait]
impl MiningCore for MyMiningCore {
    /// 初始化核心
    async fn initialize(&mut self, config: CoreConfig) -> Result<()> {
        // 1. 验证配置
        self.validate_config(&config)?;
        
        // 2. 初始化设备
        self.initialize_devices(&config).await?;
        
        // 3. 设置性能优化
        self.setup_performance_optimization(&config)?;
        
        Ok(())
    }
    
    /// 启动挖矿
    async fn start(&mut self) -> Result<()> {
        for device in &mut self.devices {
            device.start().await?;
        }
        Ok(())
    }
    
    /// 停止挖矿
    async fn stop(&mut self) -> Result<()> {
        for device in &mut self.devices {
            device.stop().await?;
        }
        Ok(())
    }
    
    /// 提交工作任务
    async fn submit_work(&mut self, work: Work) -> Result<()> {
        // 将工作分发给设备
        for device in &mut self.devices {
            device.submit_work(work.clone()).await?;
        }
        Ok(())
    }
    
    /// 收集挖矿结果
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>> {
        let mut results = Vec::new();
        for device in &mut self.devices {
            if let Some(result) = device.get_result().await? {
                results.push(result);
            }
        }
        Ok(results)
    }
    
    /// 获取统计信息
    async fn get_stats(&self) -> Result<CoreStats> {
        // 聚合所有设备的统计信息
        let mut total_hashrate = 0.0;
        let mut total_accepted = 0;
        let mut total_rejected = 0;
        
        for device in &self.devices {
            let stats = device.get_stats().await?;
            total_hashrate += stats.hashrate;
            total_accepted += stats.accepted_shares;
            total_rejected += stats.rejected_shares;
        }
        
        Ok(CoreStats {
            hashrate: total_hashrate,
            accepted_shares: total_accepted,
            rejected_shares: total_rejected,
            device_count: self.devices.len() as u32,
            // ... 其他统计信息
        })
    }
    
    /// 获取核心信息
    fn get_info(&self) -> &CoreInfo {
        &self.info
    }
    
    /// 获取核心能力
    fn get_capabilities(&self) -> &CoreCapabilities {
        &self.capabilities
    }
}
```

### 3. 挖矿设备接口 (MiningDevice)

```rust
use cgminer_core::{MiningDevice, DeviceInfo, DeviceStats, Work, MiningResult};

pub struct MyMiningDevice {
    id: u32,
    info: DeviceInfo,
    // ... 设备特定字段
}

#[async_trait]
impl MiningDevice for MyMiningDevice {
    async fn start(&mut self) -> Result<()> {
        // 启动设备挖矿
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        // 停止设备挖矿
        Ok(())
    }
    
    async fn submit_work(&mut self, work: Work) -> Result<()> {
        // 处理工作任务
        Ok(())
    }
    
    async fn get_result(&mut self) -> Result<Option<MiningResult>> {
        // 获取挖矿结果
        Ok(None)
    }
    
    async fn get_stats(&self) -> Result<DeviceStats> {
        // 返回设备统计信息
        Ok(DeviceStats::default())
    }
    
    fn get_info(&self) -> &DeviceInfo {
        &self.info
    }
}
```

## 📁 项目结构模板

推荐的外置核心项目结构：

```
cgminer-{type}-{algorithm}-core/
├── Cargo.toml                 # 项目配置
├── README.md                  # 项目说明
├── src/
│   ├── lib.rs                # 库入口，导出公共接口
│   ├── factory.rs            # CoreFactory 实现
│   ├── core.rs               # MiningCore 实现
│   ├── device.rs             # MiningDevice 实现
│   ├── algorithm/            # 算法实现
│   │   ├── mod.rs
│   │   ├── sha256.rs
│   │   └── scrypt.rs
│   ├── hardware/             # 硬件抽象 (如需要)
│   │   ├── mod.rs
│   │   ├── interface.rs
│   │   └── mock.rs
│   ├── optimization/         # 性能优化
│   │   ├── mod.rs
│   │   ├── simd.rs
│   │   └── threading.rs
│   └── error.rs              # 错误定义
├── tests/                    # 集成测试
│   ├── integration_tests.rs
│   └── benchmark_tests.rs
├── benches/                  # 性能基准测试
│   └── mining_benchmark.rs
└── examples/                 # 使用示例
    ├── basic_usage.rs
    └── advanced_config.rs
```

## ⚙️ Cargo.toml 配置

```toml
[package]
name = "cgminer-{type}-{algorithm}-core"
version = "0.1.0"
edition = "2021"
description = "CGMiner-RS {Type} {Algorithm} Mining Core"
authors = ["Your Name <your.email@example.com>"]
license = "GPL-3.0"
repository = "https://github.com/your-org/cgminer-{type}-{algorithm}-core"

[dependencies]
cgminer-core = { path = "../cgminer-core" }
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

# 算法特定依赖
sha2 = "0.10"          # SHA256 算法
scrypt = "0.11"        # Scrypt 算法

# 性能优化依赖
rayon = "1.7"          # 并行计算
num_cpus = "1.15"      # CPU 核心数检测

# 硬件控制依赖 (可选)
serialport = "4.2"     # 串口通信
spidev = "0.5"         # SPI 接口

[dev-dependencies]
criterion = "0.5"      # 性能基准测试
tempfile = "3.8"       # 临时文件测试

[features]
default = ["software"]
software = []          # 软件算法实现
hardware = ["serialport", "spidev"]  # 硬件接口支持
simd = []             # SIMD 优化
mock = []             # 模拟硬件 (测试用)

[[bench]]
name = "mining_benchmark"
harness = false
```

## 🔧 开发最佳实践

### 1. 错误处理

使用 `thiserror` 定义核心特定的错误类型：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyCoreError {
    #[error("设备初始化失败: {0}")]
    DeviceInitFailed(String),
    
    #[error("算法不支持: {algorithm}")]
    UnsupportedAlgorithm { algorithm: String },
    
    #[error("硬件错误: {0}")]
    HardwareError(String),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
}
```

### 2. 日志记录

使用 `tracing` 进行结构化日志记录：

```rust
use tracing::{info, warn, error, debug, instrument};

impl MyMiningCore {
    #[instrument(skip(self))]
    async fn process_work(&mut self, work: Work) -> Result<()> {
        debug!("开始处理工作: block_height={}", work.block_height);
        
        // 处理逻辑...
        
        info!("工作处理完成: nonce_found={}", nonce_found);
        Ok(())
    }
}
```

### 3. 配置管理

定义核心特定的配置结构：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyCoreConfig {
    pub device_count: u32,
    pub batch_size: u32,
    pub thread_count: Option<u32>,
    pub algorithm_params: AlgorithmParams,
    pub hardware_config: Option<HardwareConfig>,
}

impl Default for MyCoreConfig {
    fn default() -> Self {
        Self {
            device_count: num_cpus::get() as u32,
            batch_size: 1000,
            thread_count: None,
            algorithm_params: AlgorithmParams::default(),
            hardware_config: None,
        }
    }
}
```

### 4. 性能优化

#### CPU 亲和性绑定
```rust
use core_affinity;

fn bind_thread_to_core(core_id: usize) -> Result<()> {
    let core_ids = core_affinity::get_core_ids().ok_or("无法获取CPU核心ID")?;
    if let Some(core) = core_ids.get(core_id) {
        core_affinity::set_for_current(*core);
    }
    Ok(())
}
```

#### SIMD 优化
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(feature = "simd")]
unsafe fn sha256_simd(data: &[u8]) -> [u8; 32] {
    // SIMD 优化的 SHA256 实现
    // ...
}
```

## 🧪 测试指南

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_core_initialization() {
        let factory = MyCoreFactory;
        let config = factory.default_config();
        let mut core = factory.create_core(config).await.unwrap();
        
        assert!(core.initialize(config).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_work_processing() {
        let mut core = create_test_core().await;
        let work = create_test_work();
        
        assert!(core.submit_work(work).await.is_ok());
        
        // 等待处理完成
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let results = core.collect_results().await.unwrap();
        assert!(!results.is_empty());
    }
}
```

### 2. 集成测试

```rust
// tests/integration_tests.rs
use cgminer_my_core::*;
use cgminer_core::*;

#[tokio::test]
async fn test_full_mining_cycle() {
    // 测试完整的挖矿周期
    let factory = MyCoreFactory;
    let config = factory.default_config();
    let mut core = factory.create_core(config).await.unwrap();
    
    // 初始化
    core.initialize(config).await.unwrap();
    
    // 启动
    core.start().await.unwrap();
    
    // 提交工作
    let work = create_realistic_work();
    core.submit_work(work).await.unwrap();
    
    // 收集结果
    tokio::time::sleep(Duration::from_secs(1)).await;
    let results = core.collect_results().await.unwrap();
    
    // 获取统计信息
    let stats = core.get_stats().await.unwrap();
    assert!(stats.hashrate > 0.0);
    
    // 停止
    core.stop().await.unwrap();
}
```

### 3. 性能基准测试

```rust
// benches/mining_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use cgminer_my_core::*;

fn bench_mining_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("mining_performance");
    
    for device_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("devices", device_count),
            device_count,
            |b, &device_count| {
                b.to_async(&rt).iter(|| async {
                    let factory = MyCoreFactory;
                    let mut config = factory.default_config();
                    config.device_count = device_count;
                    
                    let mut core = factory.create_core(config).await.unwrap();
                    core.initialize(config).await.unwrap();
                    core.start().await.unwrap();
                    
                    let work = create_benchmark_work();
                    core.submit_work(work).await.unwrap();
                    
                    // 运行一段时间
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    
                    let results = core.collect_results().await.unwrap();
                    core.stop().await.unwrap();
                    
                    results.len()
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_mining_performance);
criterion_main!(benches);
```

## 📦 发布和集成

### 1. 版本管理

遵循语义化版本控制：
- `0.1.0` - 初始版本
- `0.1.1` - 补丁版本 (bug 修复)
- `0.2.0` - 次要版本 (新功能)
- `1.0.0` - 主要版本 (破坏性变更)

### 2. 文档要求

每个外置核心必须包含：
- `README.md` - 项目概述和快速开始
- `CHANGELOG.md` - 版本变更记录
- API 文档 (通过 `cargo doc` 生成)
- 使用示例

### 3. CI/CD 配置

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Run tests
      run: cargo test --all-features
    - name: Run benchmarks
      run: cargo bench
    - name: Check formatting
      run: cargo fmt -- --check
    - name: Run clippy
      run: cargo clippy -- -D warnings
```

## 🔍 调试和故障排除

### 1. 常见问题

**问题**: 核心加载失败
```rust
// 解决方案: 检查接口实现
impl CoreFactory for MyCoreFactory {
    // 确保所有方法都正确实现
}
```

**问题**: 性能不佳
```rust
// 解决方案: 启用性能优化
#[cfg(feature = "simd")]
fn optimized_hash() { /* SIMD 实现 */ }

#[cfg(not(feature = "simd"))]
fn optimized_hash() { /* 标准实现 */ }
```

### 2. 调试工具

使用 `tracing-subscriber` 启用详细日志：

```rust
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // 核心代码...
}
```

## 📚 参考资源

- [cgminer-core API 文档](../cgminer-core/README.md)
- [架构边界定义](./ARCHITECTURE_BOUNDARIES.md)
- [配置参考](./configuration-reference.md)
- [故障排除指南](./troubleshooting.md)

---

**最后更新**: 2024-12-19  
**版本**: 1.0  
**状态**: 完成
