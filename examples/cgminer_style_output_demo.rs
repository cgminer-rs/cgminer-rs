//! CGMiner风格输出演示
//!
//! 演示新的5秒间隔CGMiner风格算力统计输出格式

use cgminer_rs::mining::hashmeter::{Hashmeter, HashmeterConfig};
use cgminer_rs::monitoring::MiningMetrics;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("🚀 启动CGMiner风格输出演示");

    // 创建算力计量器配置 - 5秒间隔，传统格式
    let config = HashmeterConfig {
        enabled: true,
        log_interval: 5, // 5秒间隔
        per_device_stats: false, // 只显示总体统计
        console_output: true,
        beautiful_output: false, // 使用传统CGMiner格式
        hashrate_unit: "AUTO".to_string(),
    };

    // 创建算力计量器
    let hashmeter = Hashmeter::new(config);

    // 启动算力计量器
    hashmeter.start().await?;

    info!("📊 算力计量器已启动，将每5秒输出CGMiner风格的统计信息");
    info!("格式: (5s):16.896Mh/s (1m):12.374Mh/s (5m):9.649Mh/s (15m):9.054Mh/s A:782 R:0 HW:0 [16DEV]");
    info!("");

    // 模拟挖矿过程，逐渐增加算力
    let mut base_hashrate = 1_000_000.0; // 1 MH/s
    let mut accepted_shares = 0u64;
    let mut rejected_shares = 0u64;
    let mut hardware_errors = 0u64;

    for i in 0..20 {
        // 模拟算力变化
        let hashrate_variation = (i as f64 * 0.1).sin() * 0.2 + 1.0;
        let current_hashrate = base_hashrate * hashrate_variation;

        // 模拟偶尔的份额发现
        if i % 3 == 0 {
            accepted_shares += 1;
        }
        if i % 15 == 0 {
            rejected_shares += 1;
        }
        if i % 20 == 0 {
            hardware_errors += 1;
        }

        // 创建挖矿指标
        let mining_metrics = MiningMetrics {
            timestamp: SystemTime::now(),
            total_hashrate: current_hashrate,
            accepted_shares,
            rejected_shares,
            hardware_errors,
            stale_shares: 0,
            best_share: 1000.0,
            current_difficulty: 1.0,
            network_difficulty: 1000000.0,
            blocks_found: 0,
            efficiency: current_hashrate / 1500.0, // 假设1500W功耗
            active_devices: 4, // 模拟4个设备
            connected_pools: 1,
        };

        // 更新算力统计
        hashmeter.update_total_stats(&mining_metrics).await?;

        // 等待1秒，模拟实时挖矿
        sleep(Duration::from_secs(1)).await;

        // 逐渐增加基础算力
        base_hashrate += 100_000.0; // 每秒增加100 KH/s
    }

    info!("");
    info!("🏁 演示完成，算力计量器将继续运行5秒以显示最终统计");

    // 让算力计量器再运行一会儿以显示最终统计
    sleep(Duration::from_secs(6)).await;

    // 停止算力计量器
    hashmeter.stop().await?;

    info!("✅ CGMiner风格输出演示结束");

    Ok(())
}
