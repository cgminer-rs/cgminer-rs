//! 高性能模式演示程序
//!
//! 此程序展示了cgminer_rs的高性能同步热路径处理器，
//! 相比传统异步处理方式的性能优势。
//!
//! 主要优化特性：
//! 1. 减少异步边界：将热路径改为同步操作
//! 2. 内联关键路径：直接在设备层进行工作生成和处理
//! 3. 批量操作：减少单个工作的处理开销
//! 4. 专用工作线程：避免通用任务调度器的开销
//! 5. 零拷贝优化：使用Arc<Work>避免内存拷贝

use anyhow::Result;
use cgminer_rs::{
    config::Config,
    mining::MiningManager,
};
use cgminer_core::{CoreRegistry, CoreConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("🚀 cgminer_rs 高性能模式演示程序启动");

    // 创建高性能配置
    let config = create_high_performance_config()?;

    // 显示配置信息
    print_performance_config(&config);

    // 创建核心注册表并注册CPU核心
    let core_registry = Arc::new(CoreRegistry::new());

    #[cfg(feature = "cpu-btc")]
    {
        use cgminer_cpu_btc_core::CpuBtcCoreFactory;

        let factory = Box::new(CpuBtcCoreFactory::new());
        core_registry.register_factory("cpu-btc".to_string(), factory).await?;
        info!("✅ CPU-BTC核心工厂已注册");
    }

    // 创建挖矿管理器
    let mining_manager = Arc::new(MiningManager::new(config.clone(), core_registry.clone()).await?);

    // 启动性能基准测试
    run_performance_benchmark(mining_manager.clone(), &config).await?;

    info!("🎯 高性能模式演示程序完成");
    Ok(())
}

/// 创建高性能配置
fn create_high_performance_config() -> Result<Config> {
    let mut config = Config::default();

    // 启用高性能模式
    config.general.enable_high_performance_mode = Some(true);
    config.general.high_performance_config = Some(cgminer_rs::config::HighPerformanceConfig {
        batch_size: 200,           // 增大批次，减少处理开销
        processing_interval_us: 5, // 5微秒处理间隔，实现超低延迟
        work_accumulation_interval_ms: 1,  // 1ms累积间隔
        result_accumulation_interval_ms: 1,
        thread_priority: Some("high".to_string()),
        enable_zero_copy: true,
        enable_batch_operations: true,
        enable_inline_critical_path: true,
    });

    // 设置CPU-BTC核心配置
    if let Some(ref mut cpu_btc_config) = config.cores.cpu_btc {
        cpu_btc_config.device_count = 8;        // 8个设备并行
        cpu_btc_config.batch_size = 2000;       // 大批次提高吞吐量
        cpu_btc_config.work_timeout_ms = 1000;  // 1秒超时
        cpu_btc_config.min_hashrate = 1000000000.0;  // 1 GH/s 最小算力
        cpu_btc_config.max_hashrate = 5000000000.0;  // 5 GH/s 最大算力
    }

    // 连接到本地矿池
    config.pools.strategy = cgminer_rs::config::PoolStrategy::Failover;
    config.pools.pools = vec![
        cgminer_rs::config::PoolInfo {
            name: Some("Local Pool".to_string()),
            url: "stratum+tcp://127.0.0.1:1314".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            priority: 0,
            quota: None,
            enabled: true,
            proxy: None,
        }
    ];

    // 启用API和监控
    config.api.enabled = true;
    config.monitoring.enabled = true;

    Ok(config)
}

/// 显示性能配置信息
fn print_performance_config(config: &Config) {
    info!("📊 高性能模式配置:");

    if let Some(hp_config) = &config.general.high_performance_config {
        info!("  • 批次大小: {}", hp_config.batch_size);
        info!("  • 处理间隔: {} μs", hp_config.processing_interval_us);
        info!("  • 工作累积间隔: {} ms", hp_config.work_accumulation_interval_ms);
        info!("  • 零拷贝优化: {}", hp_config.enable_zero_copy);
        info!("  • 批量操作: {}", hp_config.enable_batch_operations);
        info!("  • 内联关键路径: {}", hp_config.enable_inline_critical_path);
    }

    if let Some(cpu_config) = &config.cores.cpu_btc {
        info!("  • CPU设备数量: {}", cpu_config.device_count);
        info!("  • CPU批次大小: {}", cpu_config.batch_size);
    }
}

/// 运行性能基准测试
async fn run_performance_benchmark(
    mining_manager: Arc<MiningManager>,
    _config: &Config,
) -> Result<()> {
    info!("🏁 开始性能基准测试");

    // 启动挖矿管理器
    mining_manager.start().await?;
    info!("✅ 挖矿管理器已启动");

    // 等待系统稳定和矿池连接
    info!("⏳ 等待矿池连接和系统稳定...");
    sleep(Duration::from_secs(5)).await;

    // 创建测试核心
    let core_config = CoreConfig::default();
    let core_id = mining_manager.create_core("cpu-btc", core_config).await?;
    info!("✅ 测试核心创建成功: {}", core_id);

    // 性能测试参数
    let test_duration = Duration::from_secs(60); // 增加到60秒，给足够时间获取工作
    info!("🚀 开始矿池工作获取和算力测试");
    info!("  • 测试时长: {:?}", test_duration);
    info!("  • 矿池地址: 127.0.0.1:1314");

    // 等待矿池工作开始流入
    sleep(Duration::from_secs(3)).await;

    // 定期监控算力和统计
    let monitor_start = Instant::now();
    let mut last_stats_time = Instant::now();

    while monitor_start.elapsed() < test_duration {
        sleep(Duration::from_secs(5)).await;

        // 获取当前统计
        let stats = mining_manager.get_stats().await;
        let system_status = mining_manager.get_system_status().await;

        let elapsed = last_stats_time.elapsed().as_secs_f64();
        last_stats_time = Instant::now();

        info!("📊 实时统计 ({}s):", monitor_start.elapsed().as_secs());
        info!("  💎 总哈希数: {}", stats.total_hashes);
        info!("  ⚡ 当前算力: {:.2} MH/s", stats.current_hashrate / 1_000_000.0);
        info!("  📈 平均算力: {:.2} MH/s", stats.average_hashrate / 1_000_000.0);
        info!("  ✅ 接受份额: {}", stats.accepted_shares);
        info!("  ❌ 拒绝份额: {}", stats.rejected_shares);
        info!("  🖥️  活跃设备: {}", system_status.active_devices);
        info!("  ⚙️  系统总算力: {:.2} MH/s", system_status.total_hashrate / 1_000_000.0);
        info!("  🔗 矿池连接: {}", system_status.connected_pools);

        // 显示矿池状态
        if system_status.connected_pools == 0 {
            warn!("⚠️  未连接到矿池，检查矿池是否运行在 127.0.0.1:1314");
        } else {
            info!("✅ 矿池连接正常");
        }
    }

    // 最终统计
    let final_stats = mining_manager.get_stats().await;
    let final_system_status = mining_manager.get_system_status().await;

    info!("🎯 最终性能统计:");
    info!("  💎 总哈希数: {}", final_stats.total_hashes);
    info!("  ⚡ 峰值算力: {:.2} MH/s", final_stats.current_hashrate / 1_000_000.0);
    info!("  📈 平均算力: {:.2} MH/s", final_stats.average_hashrate / 1_000_000.0);
    info!("  ✅ 接受份额: {}", final_stats.accepted_shares);
    info!("  ❌ 拒绝份额: {}", final_stats.rejected_shares);
    info!("  🔗 矿池连接数: {}", final_system_status.connected_pools);
    info!("  ⚙️  效率: {:.2} MH/J", final_system_status.efficiency);

    if final_stats.total_hashes > 0 {
        let test_duration_secs = test_duration.as_secs_f64();
        let actual_hashrate = final_stats.total_hashes as f64 / test_duration_secs;
        info!("  🔥 实际测试算力: {:.2} MH/s", actual_hashrate / 1_000_000.0);

        // 性能评估
        let performance_grade = if actual_hashrate > 50_000_000.0 {
            "🚀 极致性能"
        } else if actual_hashrate > 20_000_000.0 {
            "⚡ 优秀性能"
        } else if actual_hashrate > 10_000_000.0 {
            "✅ 良好性能"
        } else if actual_hashrate > 1_000_000.0 {
            "⚠️  一般性能"
        } else {
            "❌ 性能待优化"
        };

        info!("🏆 性能等级: {}", performance_grade);
    } else {
        warn!("⚠️  未检测到算力输出，可能矿池未提供工作");
    }

    // 停止挖矿管理器
    mining_manager.stop().await?;
    info!("✅ 挖矿管理器已停止");

    info!("🎯 性能基准测试完成");
    Ok(())
}

/// 创建测试工作
fn create_test_work(index: usize) -> cgminer_core::Work {
    let target_vec = hex::decode("00000000ffff0000000000000000000000000000000000000000000000000000").unwrap();
    let mut target = [0u8; 32];
    target.copy_from_slice(&target_vec);

    let header = [0u8; 80];  // 简化的区块头

    cgminer_core::Work {
        id: Uuid::new_v4(),
        work_id: index as u64,
        job_id: format!("test_job_{}", index),
        header,
        target,
        merkle_root: [0u8; 32],
        midstate: [[0u8; 32]; 8],
        extranonce1: format!("extranonce1_{}", index).into_bytes(),
        extranonce2: Vec::new(),
        extranonce2_size: 4,
        coinbase1: format!("coinbase1_{}", index).into_bytes(),
        coinbase2: format!("coinbase2_{}", index).into_bytes(),
        merkle_branches: vec![],
        version: 0x20000000,
        nbits: 0x207fffff,
        ntime: 1640995200 + index as u32,
        difficulty: 1.0,
        created_at: std::time::SystemTime::now(),
        expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(120),
        clean_jobs: index % 100 == 0,
    }
}
