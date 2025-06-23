//! # CGMiner-CPU-BTC-Core - 高性能CPU比特币挖矿核心
//!
//! 专门用于CPU比特币挖矿的核心库，使用真实的SHA256算法进行软件挖矿计算。
//! 该库经过高度优化，专注于CPU挖矿的性能和稳定性。
//!
//! ## 🚀 核心特性
//!
//! ### 真实算法挖矿
//! - ✅ 使用真实的SHA256双重哈希算法
//! - ✅ 产生真实可用的挖矿数据
//! - ✅ 支持多线程并行计算
//! - ✅ 比特币区块头结构完整实现
//!
//! ### 高性能优化
//! - ⚡ CPU亲和性绑定 (支持智能分配策略)
//! - ⚡ 无锁并发数据结构 (原子统计、无锁队列)
//! - ⚡ 批量处理优化 (减少系统调用开销)
//! - ⚡ 平台特定优化 (macOS/Linux/Windows)
//!
//! ### 监控和管理
//! - 📊 真实系统温度监控 (Linux/macOS)
//! - 📊 CGMiner风格算力统计 (5s/1m/5m/15m指数衰减)
//! - 📊 详细的设备状态跟踪
//! - 📊 健康检查和错误恢复
//!
//! ## 📦 模块架构
//!
//! ```text
//! cgminer-cpu-btc-core/
//! ├── core.rs                    # 核心挖矿算法实现
//! ├── device.rs                  # 设备抽象和管理
//! ├── factory.rs                 # 核心工厂模式
//! ├── cpu_affinity.rs           # CPU亲和性绑定
//! ├── concurrent_optimization.rs # 并发优化 (无锁数据结构)
//! ├── performance.rs             # 性能配置管理
//! ├── platform_optimization.rs  # 平台特定优化
//! └── temperature.rs             # 温度监控系统
//! ```
//!
//! ## 🎯 设计目标
//!
//! 1. **高性能**: 最大化CPU挖矿效率
//! 2. **低延迟**: 支持即时结果上报 (1-5μs)
//! 3. **高并发**: 无锁数据结构，减少竞争
//! 4. **可靠性**: 完整的错误处理和恢复机制
//! 5. **兼容性**: 支持cgminer-core标准接口
//!
//! ## 📋 使用示例
//!
//! ```rust
//! use cgminer_cpu_btc_core::{SoftwareCoreFactory, create_factory};
//! use cgminer_core::{CoreConfig, CoreFactory};
//!
//! // 创建CPU挖矿核心
//! let factory = create_factory();
//! let config = CoreConfig::default();
//! let core = factory.create_core(config).await?;
//!
//! // 启动挖矿
//! core.start().await?;
//! ```

// 核心库模块
pub mod core;                      // 挖矿核心实现
pub mod device;                    // 设备抽象层
pub mod factory;                   // 工厂模式
pub mod cpu_affinity;              // CPU亲和性管理
pub mod performance;               // 性能配置
pub mod platform_optimization;    // 平台优化
pub mod temperature;               // 温度监控
pub mod concurrent_optimization;   // 并发优化

// 重新导出主要类型
pub use factory::SoftwareCoreFactory;
pub use core::SoftwareMiningCore;
pub use device::SoftwareDevice;

use cgminer_core::{CoreType, CoreInfo};

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 获取CPU挖矿核心信息
pub fn get_core_info() -> CoreInfo {
    CoreInfo::new(
        "CPU Bitcoin Mining Core".to_string(),
        CoreType::Custom("cpu_btc".to_string()),
        VERSION.to_string(),
        "高性能CPU比特币挖矿核心，支持真实SHA256算法、无锁并发和智能调度".to_string(),
        "CGMiner Rust Team".to_string(),
        vec!["cpu".to_string(), "btc".to_string(), "sha256".to_string()],
    )
}

/// 创建CPU挖矿核心工厂
pub fn create_factory() -> Box<dyn cgminer_core::CoreFactory> {
    Box::new(SoftwareCoreFactory::new())
}

// 温度和性能管理
pub use temperature::{TemperatureManager, TemperatureConfig};
pub use performance::{PerformanceOptimizer, PerformanceConfig};
pub use cpu_affinity::CpuAffinityManager;

// 并发优化导出
pub use concurrent_optimization::{AtomicStatsManager, LockFreeWorkQueue, BatchStatsUpdater};
