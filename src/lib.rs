//! CGMiner-RS - 高性能比特币挖矿程序
//!
//! CGMiner-RS是一个用Rust编写的高性能比特币挖矿程序，支持多种挖矿核心：
//! - ASIC挖矿核心（硬件挖矿）
//! - CPU软算法核心（软件挖矿）
//! - GPU挖矿核心（计划中）
//!
//! ## 架构特点
//!
//! ### 动态核心加载
//! - 支持运行时加载不同的挖矿核心
//! - 每个核心作为独立的库存在
//! - 统一的核心接口和管理
//!
//! ### 高性能设计
//! - 异步I/O和并发处理
//! - 内存池和对象复用
//! - 智能负载均衡
//! - 实时性能监控
//!
//! ### 企业级特性
//! - 完整的API接口
//! - Web管理界面
//! - 详细的日志和监控
//! - 安全的远程管理

pub mod config;
pub mod device;
pub mod mining;
pub mod pool;
pub mod api;
pub mod monitoring;
pub mod error;
pub mod core_loader;
pub mod web;
pub mod logging;
pub mod performance;
pub mod security;
pub mod utils;
pub mod validation;

pub use config::Config;
pub use mining::MiningManager;
pub use core_loader::StaticCoreRegistry;
pub use error::MiningError;

/// 程序版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 程序名称
pub const NAME: &str = "cgminer-rs";

/// 初始化CGMiner-RS
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::fmt::init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(NAME, "cgminer-rs");
    }

    #[test]
    fn test_init() {
        // This test might fail if logger is already initialized
        // but that's okay for testing purposes
        let _ = init();
    }
}
