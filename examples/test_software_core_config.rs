use cgminer_rs::config::Config;
use std::path::Path;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() {
    // 初始化日志
    init_logging().expect("Failed to initialize logging");

    info!("═══════════════════════════════════════════════════════════");
    info!("🦀 CGMiner-RS 软算法核心配置测试");
    info!("═══════════════════════════════════════════════════════════");

    // 测试主配置文件
    test_config_file("cgminer.toml", "主配置文件").await;

    // 测试软算法核心示例配置
    test_config_file("examples/configs/software_core_example.toml", "软算法核心示例配置").await;

    info!("═══════════════════════════════════════════════════════════");
    info!("✅ 配置测试完成");
    info!("═══════════════════════════════════════════════════════════");
}

async fn test_config_file(config_path: &str, description: &str) {
    info!("📋 测试配置文件: {} ({})", config_path, description);

    if !Path::new(config_path).exists() {
        error!("❌ 配置文件不存在: {}", config_path);
        return;
    }

    match Config::load(config_path) {
        Ok(config) => {
            info!("✅ 配置文件加载成功: {}", config_path);

            // 验证软算法核心配置
            validate_software_core_config(&config);

            // 显示配置摘要
            print_config_summary(&config);
        }
        Err(e) => {
            error!("❌ 配置文件加载失败: {} - {}", config_path, e);
        }
    }

    info!("───────────────────────────────────────────────────────────");
}

fn validate_software_core_config(config: &Config) {
    info!("🔍 验证软算法核心配置...");

    // 检查是否启用软算法核心
    if config.cores.enabled_cores.contains(&"software".to_string()) {
        info!("✅ 软算法核心已启用");
    } else {
        error!("❌ 软算法核心未启用");
        return;
    }

    // 检查默认核心
    if config.cores.default_core == "software" {
        info!("✅ 默认核心设置为软算法核心");
    } else {
        error!("❌ 默认核心不是软算法核心: {}", config.cores.default_core);
    }

    // 检查软算法核心配置
    if let Some(software_config) = &config.cores.software_core {
        if software_config.enabled {
            info!("✅ 软算法核心配置已启用");
            info!("   📊 设备数量: {}", software_config.device_count);
            info!("   ⚡ 算力范围: {:.1} MH/s - {:.1} MH/s",
                  software_config.min_hashrate / 1_000_000.0,
                  software_config.max_hashrate / 1_000_000.0);
            info!("   📈 错误率: {:.2}%", software_config.error_rate * 100.0);
            info!("   🔄 批次大小: {}", software_config.batch_size);
            info!("   ⏱️  工作超时: {}ms", software_config.work_timeout_ms);

            // 验证配置合理性
            if software_config.device_count > 0 && software_config.device_count <= 64 {
                info!("✅ 设备数量配置合理");
            } else {
                error!("❌ 设备数量配置不合理: {}", software_config.device_count);
            }

            if software_config.min_hashrate < software_config.max_hashrate {
                info!("✅ 算力范围配置合理");
            } else {
                error!("❌ 算力范围配置不合理");
            }

            if software_config.error_rate >= 0.0 && software_config.error_rate <= 0.1 {
                info!("✅ 错误率配置合理");
            } else {
                error!("❌ 错误率配置不合理: {:.2}%", software_config.error_rate * 100.0);
            }
        } else {
            error!("❌ 软算法核心配置已禁用");
        }
    } else {
        error!("❌ 软算法核心配置缺失");
    }

    // 检查ASIC核心是否正确禁用
    if let Some(asic_config) = &config.cores.asic_core {
        if !asic_config.enabled {
            info!("✅ ASIC核心已正确禁用");
        } else {
            error!("⚠️ ASIC核心仍然启用，可能会与软算法核心冲突");
        }
    }
}

fn print_config_summary(config: &Config) {
    info!("📊 配置摘要:");
    info!("   🔧 日志级别: {}", config.general.log_level);
    info!("   ⏱️  工作重启超时: {}s", config.general.work_restart_timeout);
    info!("   🔍 扫描间隔: {}s", config.general.scan_time);

    // 核心信息
    info!("   🎯 启用的核心: {:?}", config.cores.enabled_cores);
    info!("   🏆 默认核心: {}", config.cores.default_core);

    // 矿池信息
    let pool_count = config.pools.pools.len();
    info!("   🏊 矿池数量: {}", pool_count);
    if pool_count > 0 {
        info!("   📡 主矿池: {}", config.pools.pools[0].url);
        info!("   👤 矿工: {}", config.pools.pools[0].user);
        info!("   🔄 策略: {:?}", config.pools.strategy);
    }

    // API信息
    if config.api.enabled {
        info!("   🌐 API: {}:{}", config.api.bind_address, config.api.port);
    } else {
        info!("   🌐 API: 已禁用");
    }

    // 监控信息
    if config.monitoring.enabled {
        info!("   📈 监控: 启用 ({}s间隔)", config.monitoring.metrics_interval);
    } else {
        info!("   📈 监控: 已禁用");
    }
}

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
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
