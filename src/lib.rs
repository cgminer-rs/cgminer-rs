//! # CGMiner-RS - Rust挖矿应用程序
//!
//! CGMiner-RS 是一个高性能的比特币挖矿应用程序，支持多种硬件类型。
//! 本项目专注于应用层功能，通过标准化接口调用外置挖矿核心。
//!
//! ## 🏗️ 架构设计
//!
//! ```text
//! cgminer-rs (应用层)
//! ├── 🎯 应用程序生命周期管理
//! ├── ⚙️ 配置管理 (TOML/CLI)
//! ├── 🌐 矿池连接和网络管理
//! ├── 📡 API服务和Web界面
//! ├── 📊 监控、日志和告警
//! └── 🔧 核心编排和调度
//! ```
//!
//! ## 🎯 应用层职责
//!
//! ### ✅ 核心功能
//! - **应用入口**: 主程序启动、信号处理、优雅关闭
//! - **配置管理**: TOML解析、CLI参数、环境变量集成
//! - **矿池连接**: Stratum协议、连接池、故障转移
//! - **工作调度**: 工作分发、结果收集、核心编排
//! - **API服务**: RESTful API、WebSocket、Web管理界面
//! - **监控日志**: 系统监控、日志管理、告警系统
//!
//! ### ❌ 不负责领域 (由外置核心处理)
//! - 具体挖矿算法实现
//! - 硬件设备直接控制
//! - 底层性能优化
//! - 硬件温度/电压监控
//! - CPU亲和性绑定
//!
//! ## 🔌 外置核心集成
//!
//! ```rust
//! use cgminer_rs::{Config, MiningManager};
//! use cgminer_core::CoreRegistry;
//!
//! // 应用层使用示例
//! let config = Config::load("config.toml")?;
//! let core_registry = CoreRegistry::new();
//! let mining_manager = MiningManager::new(config, core_registry).await?;
//! mining_manager.start().await?;
//! ```

// ==================== 应用层模块 ====================

// 核心应用模块
pub mod config;           // 配置管理
pub mod mining;           // 挖矿管理器
pub mod pool;             // 矿池连接
pub mod api;              // API服务
pub mod web;              // Web界面
pub mod monitoring;       // 监控系统
pub mod logging;          // 日志管理
pub mod error;            // 错误处理

// 支撑模块
pub mod device;           // 设备管理 (应用层抽象)
pub mod core_loader;      // 核心加载器
pub mod performance;      // 性能监控 (应用层)
pub mod security;         // 安全管理
pub mod utils;            // 工具函数
pub mod validation;       // 数据验证

// ==================== 应用层公共接口 ====================

// 主要应用组件
pub use config::{Config, Args};
pub use mining::MiningManager;
pub use error::{Error, Result};

// 设备管理 (应用层抽象 - 不是具体设备实现)
pub use device::{DeviceManager, DeviceInfo, DeviceStats};

// 核心加载和注册
pub use core_loader::StaticCoreRegistry;
pub use cgminer_core::CoreRegistry;

// ==================== 应用信息 ====================

/// 应用程序版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 应用程序名称
pub const APP_NAME: &str = "CGMiner-RS";

/// 应用程序描述
pub const APP_DESCRIPTION: &str = "High-performance Bitcoin mining application with multi-core support";

/// 获取应用程序信息
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: APP_NAME.to_string(),
        version: VERSION.to_string(),
        description: APP_DESCRIPTION.to_string(),
        authors: vec!["CGMiner Rust Team".to_string()],
        features: vec![
            "Multi-core mining support".to_string(),
            "RESTful API".to_string(),
            "Web management interface".to_string(),
            "Pool failover".to_string(),
            "SOCKS5 proxy support".to_string(),
        ],
    }
}

/// 应用程序信息结构
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub features: Vec<String>,
}

// ==================== 注意事项 ====================

// 🚫 以下功能已移至外置核心，不再在应用层导出:
// - TemperatureManager, TemperatureConfig (→ cgminer-cpu-btc-core)
// - PerformanceOptimizer, PerformanceConfig (→ cgminer-cpu-btc-core)
// - CpuAffinityManager (→ cgminer-cpu-btc-core)
// - SoftwareDevice (→ cgminer-cpu-btc-core)
// - AtomicStatsManager, LockFreeWorkQueue (→ cgminer-cpu-btc-core)

// ✅ 应用层通过 cgminer-core 标准接口与外置核心通信
// ✅ 配置通过 CoreConfig 传递给外置核心
// ✅ 结果通过标准化回调接口收集
