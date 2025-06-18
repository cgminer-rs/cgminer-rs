use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod device;
mod mining;
mod pool;
mod api;
mod monitoring;
mod error;
mod ffi;

use config::{Config, Args};
use mining::MiningManager;

#[tokio::main]
async fn main() {
    // 初始化日志系统
    if let Err(e) = init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        return;
    }

    // 解析命令行参数
    let args = Args::parse();

    // 加载配置
    let config = match Config::load(&args.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return;
        }
    };

    info!("🚀 Starting CGMiner-RS v{}", env!("CARGO_PKG_VERSION"));
    info!("📋 Configuration loaded from: {}", args.config);

    // 创建挖矿管理器
    let mining_manager = match MiningManager::new(config).await {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            error!("❌ Failed to create mining manager: {}", e);
            return;
        }
    };

    // 设置信号处理
    if let Err(e) = setup_signal_handlers(mining_manager.clone()).await {
        error!("❌ Failed to setup signal handlers: {}", e);
        return;
    }

    // 启动挖矿
    match mining_manager.start().await {
        Ok(_) => {
            info!("✅ Mining started successfully");

            // 保持程序运行
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Error waiting for signal: {}", e);
                return;
            }
            info!("🛑 Received shutdown signal");

            // 优雅关闭
            if let Err(e) = mining_manager.stop().await {
                error!("Error during shutdown: {}", e);
            }
            info!("👋 Mining stopped gracefully");
        }
        Err(e) => {
            error!("❌ Failed to start mining: {}", e);
        }
    }
}

fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cgminer_rs=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

async fn setup_signal_handlers(mining_manager: Arc<MiningManager>) -> anyhow::Result<()> {
    let manager = mining_manager.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down gracefully");
                    if let Err(e) = manager.stop().await {
                        error!("Error during graceful shutdown: {}", e);
                    }
                    std::process::exit(0);
                }
            }
        }
        #[cfg(not(unix))]
        {
            // Windows 或其他平台的处理
            info!("Signal handling not implemented for this platform");
        }
    });

    Ok(())
}
