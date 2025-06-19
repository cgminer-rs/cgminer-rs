use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, error, warn, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

mod config;
mod device;
mod mining;
mod pool;
mod api;
mod monitoring;
mod error;
mod core_loader;
mod web;
mod logging;
mod performance;
mod security;

use config::{Config, Args};
use mining::MiningManager;
use core_loader::CoreLoader;

#[tokio::main]
async fn main() {
    let start_time = Instant::now();

    // 初始化日志系统
    if let Err(e) = init_logging() {
        eprintln!("❌ Failed to initialize logging: {}", e);
        return;
    }

    // 显示启动横幅
    print_startup_banner();

    // 解析命令行参数
    let args = Args::parse();
    debug!("📝 Command line arguments parsed successfully");

    // 加载配置
    let config = match Config::load(&args.config) {
        Ok(cfg) => {
            info!("📋 Configuration loaded from: {}", args.config);
            cfg
        },
        Err(e) => {
            error!("❌ Failed to load configuration file '{}': {}", args.config, e);
            error!("💡 Please check if the file exists and has valid TOML syntax");
            return;
        }
    };

    // 显示配置摘要
    print_config_summary(&config);

    // 创建核心加载器
    info!("🔧 Initializing mining core loader...");
    let core_loader = CoreLoader::new();

    // 加载所有可用的挖矿核心
    info!("📦 Loading mining cores...");
    if let Err(e) = core_loader.load_all_cores().await {
        error!("❌ Failed to load mining cores: {}", e);
        error!("💡 Please check if the core libraries are properly installed");
        return;
    }

    // 显示加载的核心信息
    match core_loader.get_load_stats().await {
        Ok(stats) => {
            info!("✅ Mining cores loaded successfully");
            info!("📊 {}", stats);
            info!("═══════════════════════════════════════════════════════════");

            // 列出所有已加载的核心
            if let Ok(cores) = core_loader.list_loaded_cores().await {
                info!("🎯 Available Mining Cores:");
                for core in cores {
                    info!("   ✓ {} ({}): {}", core.name, core.core_type, core.description);
                }
                info!("═══════════════════════════════════════════════════════════");
            }
        }
        Err(e) => {
            warn!("⚠️ Failed to get core load statistics: {}", e);
        },
    }

    // 创建挖矿管理器
    info!("⚙️ Initializing mining manager...");
    let mining_manager = match MiningManager::new(config, core_loader.registry()).await {
        Ok(manager) => {
            info!("✅ Mining manager initialized successfully");
            Arc::new(manager)
        },
        Err(e) => {
            error!("❌ Failed to create mining manager: {}", e);
            error!("💡 Please check your device and pool configurations");
            return;
        }
    };

    // 设置信号处理
    debug!("🔧 Setting up signal handlers...");
    if let Err(e) = setup_signal_handlers(mining_manager.clone(), core_loader).await {
        error!("❌ Failed to setup signal handlers: {}", e);
        return;
    }

    // 显示启动完成信息
    let startup_duration = start_time.elapsed();
    info!("🚀 CGMiner-RS initialization completed in {:.2}s", startup_duration.as_secs_f64());
    info!("═══════════════════════════════════════════════════════════");

    // 启动挖矿
    info!("⛏️  Starting mining operations...");
    match mining_manager.start().await {
        Ok(_) => {
            info!("✅ Mining operations started successfully!");
            info!("💎 CGMiner-RS is now mining Bitcoin...");
            info!("📊 Monitor your mining progress through the API or logs");
            info!("🔗 API available at: http://127.0.0.1:4028");
            info!("═══════════════════════════════════════════════════════════");
            info!("🎯 Press Ctrl+C to stop mining gracefully");

            // 保持程序运行
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("❌ Error waiting for shutdown signal: {}", e);
                return;
            }

            info!("═══════════════════════════════════════════════════════════");
            info!("🛑 Shutdown signal received - stopping mining operations...");

            // 优雅关闭
            if let Err(e) = mining_manager.stop().await {
                error!("❌ Error during mining shutdown: {}", e);
            } else {
                info!("✅ Mining operations stopped successfully");
            }

            let total_runtime = start_time.elapsed();
            info!("⏱️  Total runtime: {:.2}s", total_runtime.as_secs_f64());
            info!("👋 CGMiner-RS shutdown completed. Thank you for mining!");
        }
        Err(e) => {
            error!("❌ Failed to start mining operations: {}", e);
            error!("💡 Please check your hardware connections and pool settings");
        }
    }
}

fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cgminer_rs=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_span_events(FmtSpan::NONE)
                .with_ansi(true)
        )
        .init();

    Ok(())
}

async fn setup_signal_handlers(mining_manager: Arc<MiningManager>, core_loader: CoreLoader) -> anyhow::Result<()> {
    let manager = mining_manager.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("🛑 Received SIGTERM signal - initiating graceful shutdown...");
                    if let Err(e) = manager.stop().await {
                        error!("❌ Error during mining shutdown: {}", e);
                    } else {
                        info!("✅ Mining operations stopped successfully");
                    }

                    // 关闭所有核心
                    info!("🔧 Shutting down mining cores...");
                    if let Err(e) = core_loader.shutdown().await {
                        error!("❌ Error shutting down cores: {}", e);
                    } else {
                        info!("✅ Mining cores shutdown completed");
                    }

                    info!("👋 CGMiner-RS terminated gracefully");
                    std::process::exit(0);
                }
            }
        }
        #[cfg(not(unix))]
        {
            // Windows 或其他平台的处理
            warn!("⚠️ Advanced signal handling not available on this platform");
            info!("💡 Use Ctrl+C to stop the miner");
        }
    });

    Ok(())
}

/// 显示启动横幅
fn print_startup_banner() {
    info!("═══════════════════════════════════════════════════════════");
    info!("🦀 CGMiner-RS v{} - High-Performance Bitcoin Miner", env!("CARGO_PKG_VERSION"));
    info!("⚡ Built with Rust for maximum performance and safety");
    info!("🏗️  Build: {} ({})",
          std::env::consts::OS,
          option_env!("GIT_HASH").unwrap_or("unknown"));
    info!("═══════════════════════════════════════════════════════════");
}

/// 显示配置摘要
fn print_config_summary(config: &Config) {
    info!("📊 Configuration Summary:");
    info!("   🔧 Log Level: {}", config.general.log_level);
    info!("   ⏱️  Work Restart Timeout: {}s", config.general.work_restart_timeout);
    info!("   🔍 Scan Interval: {}s", config.general.scan_time);

    // 显示矿池信息
    let pool_count = config.pools.pools.len();
    info!("   🏊 Mining Pools: {} configured", pool_count);
    if pool_count > 0 {
        info!("   📡 Primary Pool: {}", config.pools.pools[0].url);
        info!("   👤 Worker: {}", config.pools.pools[0].username);
    }

    // 显示API信息
    if config.api.enabled {
        info!("   🌐 API Server: {}:{}", config.api.bind_address, config.api.port);
    } else {
        info!("   🌐 API Server: Disabled");
    }

    // 显示监控信息
    if config.monitoring.enabled {
        info!("   📈 Monitoring: Enabled ({}s interval)", config.monitoring.metrics_interval);
    } else {
        info!("   📈 Monitoring: Disabled");
    }
}
