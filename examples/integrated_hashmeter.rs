use cgminer_rs::mining::{Hashmeter, HashmeterConfig};
use cgminer_rs::mining::hashmeter::{MiningMetrics, DeviceMetrics};
use std::time::{Duration, SystemTime};
use tokio::time::{interval, sleep};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};



/// 集成算力计量器示例
/// 演示如何在CGMiner-RS中集成定期算力输出功能
#[tokio::main]
async fn main() {
    // 初始化日志系统
    init_logging().expect("Failed to initialize logging");

    info!("🚀 CGMiner-RS with Integrated Hashmeter");
    info!("📊 This example demonstrates periodic hashrate output similar to traditional cgminer");
    info!("⚡ Hashrate will be displayed every 30 seconds");
    info!("");

    // 创建算力计量器配置
    let hashmeter_config = HashmeterConfig {
        log_interval: 30,           // 30秒间隔
        per_device_stats: true,     // 显示设备级统计
        console_output: true,       // 控制台输出
        beautiful_output: true,     // 美化输出
        hashrate_unit: "GH".to_string(), // 使用GH/s单位
    };

    // 创建算力计量器
    let hashmeter = Hashmeter::new(hashmeter_config);

    // 启动算力计量器
    if let Err(e) = hashmeter.start().await {
        warn!("Failed to start hashmeter: {}", e);
        return;
    }

    info!("✅ Hashmeter started successfully");
    info!("📈 Monitoring hashrate with 30-second intervals");
    info!("");

    // 模拟挖矿数据更新
    let hashmeter_arc = std::sync::Arc::new(hashmeter);
    let hashmeter_clone = hashmeter_arc.clone();
    let data_update_task = tokio::spawn(async move {
        simulate_mining_data_updates(hashmeter_clone).await;
    });

    // 模拟主挖矿循环
    let mining_task = tokio::spawn(async move {
        simulate_main_mining_loop().await;
    });

    // 等待用户中断
    info!("🎯 Press Ctrl+C to stop mining");
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");

    info!("🛑 Shutdown signal received - stopping all tasks...");

    // 停止算力计量器
    if let Err(e) = hashmeter_arc.stop().await {
        warn!("Failed to stop hashmeter: {}", e);
    }

    // 取消任务
    data_update_task.abort();
    mining_task.abort();

    info!("✅ CGMiner-RS stopped successfully");
}

/// 模拟挖矿数据更新
async fn simulate_mining_data_updates(hashmeter: std::sync::Arc<Hashmeter>) {
    let mut interval = interval(Duration::from_secs(5)); // 每5秒更新一次数据
    let mut iteration = 0;

    loop {
        interval.tick().await;
        iteration += 1;

        // 模拟总体挖矿指标
        let mining_metrics = generate_mock_mining_metrics(iteration);

        // 更新总体统计
        if let Err(e) = hashmeter.update_total_stats(&mining_metrics).await {
            warn!("Failed to update total stats: {}", e);
        }

        // 模拟设备指标
        for device_id in 0..4 {
            let device_metrics = generate_mock_device_metrics(device_id, iteration);

            // 更新设备统计
            if let Err(e) = hashmeter.update_device_stats(&device_metrics).await {
                warn!("Failed to update device {} stats: {}", device_id, e);
            }
        }
    }
}

/// 模拟主挖矿循环
async fn simulate_main_mining_loop() {
    let mut interval = interval(Duration::from_secs(45)); // 每45秒一个挖矿周期
    let mut shares_found = 0;

    loop {
        interval.tick().await;
        shares_found += fastrand::u32(1..3);

        // 模拟发现份额
        if fastrand::f64() < 0.4 {
            info!("💎 Share found! Total shares: {}", shares_found);
        }

        // 模拟偶尔的事件
        if fastrand::f64() < 0.15 {
            warn!("⚠️ Device temperature slightly elevated");
        }

        if fastrand::f64() < 0.05 {
            info!("🔄 New work received from pool");
        }
    }
}

/// 生成模拟的挖矿指标
fn generate_mock_mining_metrics(iteration: u32) -> MiningMetrics {
    // 模拟算力波动 (45-65 GH/s)
    let base_hashrate = 55.0 + (iteration as f64 * 0.1).sin() * 10.0;
    let hashrate_ghps = base_hashrate + fastrand::f64() * 5.0 - 2.5;
    let total_hashrate = hashrate_ghps * 1_000_000_000.0; // 转换为 H/s

    MiningMetrics {
        total_hashrate,
        accepted_shares: 1200 + iteration as u64 * 2,
        rejected_shares: 25 + iteration as u64 / 10,
        hardware_errors: 3 + iteration as u64 / 20,
    }
}

/// 生成模拟的设备指标
fn generate_mock_device_metrics(device_id: u32, iteration: u32) -> DeviceMetrics {
    // 每个设备的基础算力不同
    let base_hashrate = match device_id {
        0 => 12.0,
        1 => 14.0,
        2 => 13.5,
        3 => 15.5,
        _ => 13.0,
    };

    let device_hashrate = base_hashrate + (iteration as f64 * 0.05).sin() * 2.0;
    let hashrate_hps = device_hashrate * 1_000_000_000.0; // 转换为 H/s

    DeviceMetrics {
        device_id,
        hashrate: hashrate_hps,
        accepted_shares: 300 + device_id as u64 * 50 + iteration as u64 / 2,
        rejected_shares: 5 + device_id as u64 + iteration as u64 / 20,
        hardware_errors: device_id as u64 / 2 + iteration as u64 / 50,
        temperature: 65.0 + device_id as f32 * 2.0 + fastrand::f32() * 5.0,
        fan_speed: 70 + device_id * 5 + fastrand::u32(0..10),
        uptime: Duration::from_secs(iteration as u64 * 5),
    }
}

/// 初始化日志系统
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

// 注意：由于Hashmeter不能直接克隆，我们使用Arc来共享实例

/*
预期输出示例：

INFO 🚀 CGMiner-RS with Integrated Hashmeter
INFO 📊 This example demonstrates periodic hashrate output similar to traditional cgminer
INFO ⚡ Hashrate will be displayed every 30 seconds

INFO ✅ Hashmeter started successfully
INFO 📈 Monitoring hashrate with 30-second intervals

INFO 💎 Share found! Total shares: 2
INFO 🔄 New work received from pool

INFO ⚡ Mining Status Update:
INFO    📈 Hashrate: 58.42 GH/s
INFO    🎯 Shares: 1206 accepted, 27 rejected (2.19% reject rate)
INFO    ⚠️  Hardware Errors: 4
INFO    🔧 Work Utility: 24.52/min
INFO    ⏱️  Uptime: 2m 30s
INFO    📊 Device Details:
INFO       • SoftCore-0: 13.45 GH/s | Temp: 67.2°C | Fan: 73%
INFO       • SoftCore-1: 15.12 GH/s | Temp: 69.8°C | Fan: 78%
INFO       • SoftCore-2: 14.67 GH/s | Temp: 68.5°C | Fan: 76%
INFO       • SoftCore-3: 16.18 GH/s | Temp: 71.3°C | Fan: 82%

INFO ⚠️ Device temperature slightly elevated
INFO 💎 Share found! Total shares: 4
*/
