use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

/// 内置算力显示示例
/// 演示如何在CGMiner-RS中添加实时算力显示功能
#[tokio::main]
async fn main() {
    // 初始化日志系统
    init_logging().expect("Failed to initialize logging");

    // 显示启动信息
    print_startup_info();

    // 模拟挖矿程序启动
    info!("🚀 Starting CGMiner-RS with built-in hashrate display...");

    // 启动内置算力显示任务
    let hashrate_task = tokio::spawn(builtin_hashrate_display());

    // 模拟主挖矿循环
    let mining_task = tokio::spawn(simulate_mining_loop());

    // 等待用户中断
    info!("🎯 Press Ctrl+C to stop mining");
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");

    info!("🛑 Shutdown signal received - stopping all tasks...");

    // 取消任务
    hashrate_task.abort();
    mining_task.abort();

    info!("✅ CGMiner-RS stopped successfully");
}

/// 内置算力显示功能
/// 这个函数演示了如何在主程序中集成实时算力显示
async fn builtin_hashrate_display() {
    let mut interval = interval(Duration::from_secs(10)); // 每10秒更新一次
    let mut iteration = 0;

    loop {
        interval.tick().await;
        iteration += 1;

        // 模拟获取实时算力数据
        let hashrate_data = get_current_hashrate_data().await;

        // 显示算力信息
        display_hashrate_summary(&hashrate_data, iteration);
    }
}

/// 模拟主挖矿循环
async fn simulate_mining_loop() {
    let mut interval = interval(Duration::from_secs(30)); // 每30秒一个挖矿周期
    let mut shares_found = 0;

    loop {
        interval.tick().await;
        shares_found += fastrand::u32(1..5);

        // 模拟发现份额
        if fastrand::f64() < 0.3 {
            info!("💎 Share found! Total shares: {}", shares_found);
        }

        // 模拟偶尔的事件
        if fastrand::f64() < 0.1 {
            warn!("⚠️ Minor temperature increase detected");
        }
    }
}

/// 获取当前算力数据（模拟）
async fn get_current_hashrate_data() -> HashrateData {
    // 在实际实现中，这里会调用API或直接访问内部数据结构
    let base_hashrate = 45.0 + fastrand::f64() * 20.0; // 45-65 GH/s
    let device_count = 32;
    let active_devices = device_count - fastrand::u32(0..3);

    HashrateData {
        total_hashrate: base_hashrate,
        device_count,
        active_devices,
        accepted_shares: 1250 + fastrand::u64(0..100),
        rejected_shares: 15 + fastrand::u64(0..5),
        pool_connected: true,
        uptime: Duration::from_secs(3600 + fastrand::u64(0..7200)),
        cpu_usage: 75.0 + fastrand::f64() * 20.0,
        memory_usage: 60.0 + fastrand::f64() * 15.0,
    }
}

/// 显示算力摘要
fn display_hashrate_summary(data: &HashrateData, iteration: u32) {
    // 使用特殊的日志格式来突出显示算力信息
    info!("═══════════════════════════════════════════════════════════");
    info!("📊 CGMiner-RS Performance Summary (Update #{}) 📊", iteration);
    info!("═══════════════════════════════════════════════════════════");

    // 算力状态
    let hashrate_status = if data.total_hashrate >= 50.0 {
        "✅ Target Achieved"
    } else {
        "⚠️ Below Target"
    };

    info!("⚡ Total Hashrate: {:.2} GH/s ({})", data.total_hashrate, hashrate_status);
    info!("🔧 Active Devices: {}/{}", data.active_devices, data.device_count);

    // 份额统计
    let total_shares = data.accepted_shares + data.rejected_shares;
    let reject_rate = if total_shares > 0 {
        (data.rejected_shares as f64 / total_shares as f64) * 100.0
    } else {
        0.0
    };

    info!("🎯 Shares: {} accepted, {} rejected ({:.2}% reject rate)",
          data.accepted_shares, data.rejected_shares, reject_rate);

    // 连接状态
    let pool_status = if data.pool_connected { "✅ Connected" } else { "❌ Disconnected" };
    info!("🏊 Pool Status: {}", pool_status);

    // 系统资源
    info!("💻 System: CPU {:.1}%, Memory {:.1}%", data.cpu_usage, data.memory_usage);

    // 运行时间
    let hours = data.uptime.as_secs() / 3600;
    let minutes = (data.uptime.as_secs() % 3600) / 60;
    info!("⏱️  Uptime: {}h {}m", hours, minutes);

    // 性能建议
    if data.total_hashrate < 50.0 {
        info!("💡 Suggestion: Consider increasing device count or optimizing CPU affinity");
    } else if data.total_hashrate > 80.0 {
        info!("🎉 Excellent performance! System is running optimally");
    }

    info!("═══════════════════════════════════════════════════════════");
}

/// 算力数据结构
#[derive(Debug, Clone)]
struct HashrateData {
    total_hashrate: f64,
    device_count: u32,
    active_devices: u32,
    accepted_shares: u64,
    rejected_shares: u64,
    pool_connected: bool,
    uptime: Duration,
    cpu_usage: f64,
    memory_usage: f64,
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

/// 显示启动信息
fn print_startup_info() {
    info!("═══════════════════════════════════════════════════════════");
    info!("🦀 CGMiner-RS with Built-in Hashrate Display");
    info!("📊 This example shows how to integrate real-time hashrate display");
    info!("⚡ Updates every 10 seconds with comprehensive mining statistics");
    info!("═══════════════════════════════════════════════════════════");
}

/*
添加到Cargo.toml的依赖项说明：

[dependencies]
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
fastrand = "2.0"
*/
