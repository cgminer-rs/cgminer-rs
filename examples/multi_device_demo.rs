use cgminer_rs::{MiningManager, Config};
use cgminer_core::{CoreRegistry, CoreType};
use cgminer_cpu_btc_core::{CpuBtcCore, CpuBtcCoreFactory};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 多设备挖矿演示 - CGMiner风格");

    // 创建核心注册表
    let core_registry = Arc::new(CoreRegistry::new());

    // 注册CPU核心
    #[cfg(feature = "cpu-btc")]
    {
        let cpu_factory = Box::new(CpuBtcCoreFactory::new());
        core_registry.register_core(CoreType::CpuBtc, cpu_factory).await?;
        info!("✅ CPU BTC核心已注册");
    }

    // 加载配置
    let config = Config::from_file("config.toml")
        .unwrap_or_else(|_| {
            warn!("⚠️  使用默认配置");
            Config::default()
        });

    // 创建挖矿管理器
    let mining_manager = Arc::new(MiningManager::new(
        config,
        core_registry.clone(),
    ).await?);

    info!("🔧 启动挖矿管理器...");
    mining_manager.start().await?;

    // 添加多个CPU核心（模拟多设备）
    let device_count = num_cpus::get().min(8); // 最多8个设备
    info!("💻 创建 {} 个CPU挖矿设备", device_count);

    for i in 0..device_count {
        let core_info = cgminer_core::CoreInfo {
            name: format!("CPU设备-{}", i + 1),
            core_type: CoreType::CpuBtc,
            version: "1.0.0".to_string(),
            description: format!("CPU挖矿设备 #{}", i + 1),
            capabilities: vec!["sha256".to_string()],
        };

        match mining_manager.add_core(core_info).await {
            Ok(_) => info!("✅ 设备 {} 添加成功", i + 1),
            Err(e) => error!("❌ 设备 {} 添加失败: {}", i + 1, e),
        }
    }

    // 创建工作数据
    let work = cgminer_core::Work::new(
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
        1,
        vec![0u8; 80],
        1234567890,
    );

    info!("⛏️  开始多设备挖矿演示...");
    info!("📊 CGMiner风格输出格式:");
    info!("    [当前/1分钟/5分钟/15分钟]Mh/s A:[接受] R:[拒绝] HW:[硬件错误] [设备数]");

    // 提交工作到所有设备
    for i in 0..device_count {
        if let Err(e) = mining_manager.submit_work(work.clone()).await {
            error!("❌ 工作提交失败 (设备 {}): {}", i + 1, e);
        }
    }

    // 监控循环 - CGMiner风格输出
    let start_time = std::time::Instant::now();
    let mut total_accepted = 0u64;
    let mut total_rejected = 0u64;
    let mut total_hw_errors = 0u64;
    let mut hashrate_history = Vec::new();

    for iteration in 0..60 { // 运行60秒
        sleep(Duration::from_secs(1)).await;

        // 收集统计信息
        let stats = mining_manager.get_stats().await;
        let current_hashrate = stats.hashrate;
        hashrate_history.push(current_hashrate);

        // 计算不同时间窗口的平均算力
        let hashrate_1m = if hashrate_history.len() >= 60 {
            hashrate_history[hashrate_history.len()-60..].iter().sum::<f64>() / 60.0
        } else {
            hashrate_history.iter().sum::<f64>() / hashrate_history.len() as f64
        };

        let hashrate_5m = if hashrate_history.len() >= 300 {
            hashrate_history[hashrate_history.len()-300..].iter().sum::<f64>() / 300.0
        } else {
            hashrate_history.iter().sum::<f64>() / hashrate_history.len() as f64
        };

        let hashrate_15m = hashrate_history.iter().sum::<f64>() / hashrate_history.len() as f64;

        // 更新统计
        total_accepted += stats.total_hashrate as u64 / 1_000_000; // 模拟接受数
        if iteration % 10 == 0 && iteration > 0 {
            total_rejected += 1; // 模拟偶尔的拒绝
        }
        if iteration % 30 == 0 && iteration > 0 {
            total_hw_errors += 1; // 模拟偶尔的硬件错误
        }

        // CGMiner风格输出
        let elapsed = start_time.elapsed().as_secs();
        if elapsed % 5 == 0 || iteration < 10 {
            println!("{:.1}/{:.1}/{:.1}/{:.1}Mh/s A:{} R:{} HW:{} [{}DEV]",
                current_hashrate / 1_000_000.0,
                hashrate_1m / 1_000_000.0,
                hashrate_5m / 1_000_000.0,
                hashrate_15m / 1_000_000.0,
                total_accepted,
                total_rejected,
                total_hw_errors,
                device_count
            );
        }

        // 每10秒显示详细设备信息
        if iteration % 10 == 0 && iteration > 0 {
            info!("📱 设备状态:");
            for i in 0..device_count {
                let device_hashrate = current_hashrate / device_count as f64;
                info!("   设备 {}: {:.1} Mh/s, 温度: {}°C, 状态: {}",
                    i + 1,
                    device_hashrate / 1_000_000.0,
                    45 + (i % 10) as u32, // 模拟温度 45-54°C
                    if i % 4 == 0 { "正常" } else { "良好" }
                );
            }
        }
    }

    info!("⏹️ 停止挖矿管理器...");
    mining_manager.stop().await?;

    // 最终统计
    let final_stats = mining_manager.get_stats().await;
    let total_time = start_time.elapsed().as_secs();
    let avg_hashrate = hashrate_history.iter().sum::<f64>() / hashrate_history.len() as f64;

    info!("📊 最终统计报告:");
    info!("   运行时间: {}秒", total_time);
    info!("   设备数量: {}", device_count);
    info!("   平均总算力: {:.2} Mh/s", avg_hashrate / 1_000_000.0);
    info!("   平均单设备算力: {:.2} Mh/s", avg_hashrate / (device_count as f64 * 1_000_000.0));
    info!("   接受的解: {}", total_accepted);
    info!("   拒绝的解: {}", total_rejected);
    info!("   硬件错误: {}", total_hw_errors);

    if total_accepted + total_rejected > 0 {
        let success_rate = (total_accepted as f64 / (total_accepted + total_rejected) as f64) * 100.0;
        info!("   成功率: {:.2}%", success_rate);
    }

    info!("✅ 多设备挖矿演示完成！");

    Ok(())
}
