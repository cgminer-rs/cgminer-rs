//! 简单的 CGMiner-RS 演示
//! 模拟 cgminer 风格输出，连接本地矿池转发

use anyhow::Result;
use cgminer_rs::{
    config::Config,
    StaticCoreRegistry as CoreRegistry,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// 模拟 cgminer 风格的时间戳
fn cgminer_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hours = (now % 86400) / 3600;
    let minutes = (now % 3600) / 60;
    let seconds = now % 60;
    format!("[{:02}:{:02}:{:02}]", hours, minutes, seconds)
}

// 模拟 cgminer 日志输出
macro_rules! cgminer_log {
    ($($arg:tt)*) => {
        println!("{} {}", cgminer_timestamp(), format_args!($($arg)*))
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    cgminer_log!("Started cgminer-rs 1.0.0");

    // 1. 配置管理
    cgminer_log!("Loading configuration...");
    let mut config = Config::load("config.toml")
        .unwrap_or_else(|_| {
            cgminer_log!("Using default configuration");
            Config::default()
        });

    // 设置矿池连接
    cgminer_log!("Pool 0: 127.0.0.1:1314");

    // 2. 初始化核心注册表
    cgminer_log!("Initializing mining cores...");
    let static_registry = CoreRegistry::new().await?;
    let core_registry = static_registry.registry();

    // 3. 检查可用核心
    match core_registry.list_factories().await {
        Ok(factories) => {
            cgminer_log!("Found {} mining core(s)", factories.len());
            for factory in &factories {
                cgminer_log!("Core: {} v{} ({})", factory.name, factory.version, factory.core_type);
            }
        }
        Err(e) => {
            cgminer_log!("ERROR: Failed to list cores: {}", e);
            return Err(e.into());
        }
    }

    #[cfg(feature = "cpu-btc")]
    {
        // 4. 创建并启动挖矿核心
        cgminer_log!("Creating CPU mining core...");

        let core_config = cgminer_core::CoreConfig {
            name: "CPU0".to_string(),
            enabled: true,
            devices: vec![],
            custom_params: std::collections::HashMap::new(),
        };

        match core_registry.create_core("cpu-btc", core_config).await {
            Ok(core_id) => {
                cgminer_log!("Core {} created successfully", core_id);

                // 启动核心
                match core_registry.start_core(&core_id).await {
                    Ok(_) => {
                        cgminer_log!("Core {} started", core_id);

                        // 等待设备初始化
                        sleep(Duration::from_secs(1)).await;

                        // 获取初始统计
                        if let Ok(stats) = core_registry.get_core_stats(&core_id).await {
                            cgminer_log!("Devices: {} | Hashrate: {:.2} H/s",
                                stats.active_devices, stats.total_hashrate);
                        }

                        // 模拟矿池连接
                        cgminer_log!("Connecting to pool 127.0.0.1:1314...");
                        sleep(Duration::from_millis(500)).await;
                        cgminer_log!("Pool 0: Connected to 127.0.0.1:1314");
                        cgminer_log!("Pool 0: Authorized worker");

                        // 模拟接收工作并提交给设备
                        cgminer_log!("Pool 0: New block detected");
                        cgminer_log!("Work received from pool 0");

                                                // 创建模拟工作并提交给核心
                        let mock_work = cgminer_core::Work::new(
                            "mock_job_001".to_string(),
                            [0x00, 0x00, 0x00, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
                            [0u8; 80], // 标准比特币区块头大小
                            1.0, // 难度
                        );

                        if let Err(e) = core_registry.submit_work_to_core(&core_id, mock_work.into()).await {
                            cgminer_log!("Failed to submit work: {}", e);
                        } else {
                            cgminer_log!("Work submitted to devices");
                        }

                        // 运行挖矿并显示统计
                        cgminer_log!("Mining started...");

                        let mut total_hashes = 0u64;
                        let start_time = std::time::Instant::now();

                        for i in 1..=30 {
                            sleep(Duration::from_secs(1)).await;

                            if let Ok(stats) = core_registry.get_core_stats(&core_id).await {
                                total_hashes += stats.total_hashrate as u64;
                                let elapsed = start_time.elapsed().as_secs();
                                let avg_hashrate = if elapsed > 0 { total_hashes / elapsed } else { 0 };

                                // 模拟 cgminer 风格的输出
                                if i % 5 == 0 {
                                    cgminer_log!("({}s): {} | A:0 R:0 HW:0 WU:{:.1}/m",
                                        elapsed, format_hashrate(stats.total_hashrate),
                                        stats.total_hashrate / 1000000.0 * 60.0);
                                }

                                // 偶尔模拟找到 share
                                if i % 8 == 0 {
                                    cgminer_log!("Accepted {} Diff 1/1 {} {}ms",
                                        core_id, "127.0.0.1:1314", 50 + (i % 20));
                                }
                            }
                        }

                        // 测试 meets_target 函数
                        cgminer_log!("Testing target validation...");

                        let test_hash = [0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF, 0xFF, 0xFF];
                        let easy_target = [0x00, 0x00, 0x00, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF];
                        let hard_target = [0x00, 0x00, 0x00, 0x00, 0x0F, 0xFF, 0xFF, 0xFF];

                        let meets_easy = cgminer_core::meets_target(&test_hash, &easy_target);
                        let meets_hard = cgminer_core::meets_target(&test_hash, &hard_target);

                        cgminer_log!("Target test: Easy={} Hard={}", meets_easy, meets_hard);

                        // 关闭
                        cgminer_log!("Shutting down...");

                        if let Err(e) = core_registry.stop_core(&core_id).await {
                            cgminer_log!("ERROR: Failed to stop core: {}", e);
                        } else {
                            cgminer_log!("Core {} stopped", core_id);
                        }

                        if let Err(e) = core_registry.remove_core(&core_id).await {
                            cgminer_log!("ERROR: Failed to remove core: {}", e);
                        } else {
                            cgminer_log!("Core {} removed", core_id);
                        }
                    }
                    Err(e) => cgminer_log!("ERROR: Failed to start core: {}", e),
                }
            }
            Err(e) => cgminer_log!("ERROR: Failed to create core: {}", e),
        }
    }

    #[cfg(not(feature = "cpu-btc"))]
    {
        cgminer_log!("ERROR: CPU-BTC feature not enabled");
        cgminer_log!("Run with: cargo run --example multi_device_demo --features=cpu-btc");
    }

    cgminer_log!("cgminer-rs shutdown complete");
    Ok(())
}

// 格式化算力显示
fn format_hashrate(hashrate: f64) -> String {
    if hashrate >= 1_000_000_000_000.0 {
        format!("{:.2}TH/s", hashrate / 1_000_000_000_000.0)
    } else if hashrate >= 1_000_000_000.0 {
        format!("{:.2}GH/s", hashrate / 1_000_000_000.0)
    } else if hashrate >= 1_000_000.0 {
        format!("{:.2}MH/s", hashrate / 1_000_000.0)
    } else if hashrate >= 1_000.0 {
        format!("{:.2}KH/s", hashrate / 1_000.0)
    } else {
        format!("{:.2}H/s", hashrate)
    }
}
