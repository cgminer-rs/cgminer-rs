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
async fn main() -> Result<()> {
    // 初始化日志系统
    init_logging()?;

    // 解析命令行参数
    let args = Args::parse();

    // 加载配置
    let config = Config::load(&args.config)?;

    info!("Starting CGMiner-RS v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded from: {}", args.config);

    // 创建挖矿管理器
    let mining_manager = Arc::new(MiningManager::new(config).await?);

    // 设置信号处理
    setup_signal_handlers(mining_manager.clone()).await?;

    // 启动挖矿
    match mining_manager.start().await {
        Ok(_) => {
            info!("Mining started successfully");

            // 保持程序运行
            tokio::signal::ctrl_c().await?;
            info!("Received shutdown signal");

            // 优雅关闭
            mining_manager.stop().await?;
            info!("Mining stopped gracefully");
        }
        Err(e) => {
            error!("Failed to start mining: {}", e);
            return Err(e);
        }
    }

    Ok(())
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

async fn setup_signal_handlers(mining_manager: Arc<MiningManager>) -> Result<()> {
    let manager = mining_manager.clone();
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate()
        ).expect("Failed to create SIGTERM handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully");
                if let Err(e) = manager.stop().await {
                    error!("Error during graceful shutdown: {}", e);
                }
                std::process::exit(0);
            }
        }
    });

    Ok(())
}
