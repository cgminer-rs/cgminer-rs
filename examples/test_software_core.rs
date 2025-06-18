//! 软算法核心功能验证测试
//!
//! 这个示例程序测试软算法核心的SHA256计算能力和算力输出

use cgminer_rs::CoreLoader;
use cgminer_core::{MiningCore, Work, CoreConfig};
use cgminer_s_btc_core::SoftwareMiningCore;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    info!("🚀 开始软算法核心功能验证测试");

    // 测试1: 核心加载和初始化
    info!("📦 测试1: 核心加载和初始化");
    let core_loader = CoreLoader::new();

    // 加载所有核心
    match core_loader.load_all_cores().await {
        Ok(_) => info!("✅ 所有核心加载成功"),
        Err(e) => {
            error!("❌ 核心加载失败: {}", e);
            return Err(e.into());
        }
    }

    let core_registry = core_loader.registry();
    let loaded_cores = core_loader.list_loaded_cores()?;
    info!("📋 已加载的核心数量: {}", loaded_cores.len());

    // 测试2: 创建软算法核心实例
    info!("🔧 测试2: 创建软算法核心实例");
    let mut software_core = SoftwareMiningCore::new("test-software-core".to_string());

    // 配置核心
    let mut core_config = CoreConfig::default();
    core_config.name = "test-software-core".to_string();

    // 添加自定义参数
    core_config.custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
    core_config.custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1_000_000.0).unwrap()));
    core_config.custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(5_000_000.0).unwrap()));
    core_config.custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap()));
    core_config.custom_params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));

    match software_core.initialize(core_config).await {
        Ok(_) => info!("✅ 软算法核心初始化成功"),
        Err(e) => {
            error!("❌ 软算法核心初始化失败: {}", e);
            return Err(e.into());
        }
    }

    // 测试3: 创建测试工作
    info!("⚒️  测试3: 创建测试工作");
    let test_work = create_test_work();
    info!("📄 测试工作创建完成，工作ID: {}", test_work.id);
    info!("🎯 目标难度: {:02x}{:02x}{:02x}{:02x}",
          test_work.target[0], test_work.target[1], test_work.target[2], test_work.target[3]);

    // 测试4: 提交工作并开始挖矿
    info!("⛏️  测试4: 提交工作并开始挖矿");
    match software_core.submit_work(test_work.clone()).await {
        Ok(_) => info!("✅ 工作提交成功"),
        Err(e) => {
            error!("❌ 工作提交失败: {}", e);
            return Err(e.into());
        }
    }

    // 启动核心
    match software_core.start().await {
        Ok(_) => info!("✅ 软算法核心启动成功"),
        Err(e) => {
            error!("❌ 软算法核心启动失败: {}", e);
            return Err(e.into());
        }
    }

    // 测试5: 监控挖矿过程
    info!("📊 测试5: 监控挖矿过程");
    let start_time = Instant::now();
    let mut total_hashes = 0u64;
    let mut valid_shares = 0u32;
    let mut last_stats_time = Instant::now();

    for i in 0..30 { // 运行30秒
        sleep(Duration::from_secs(1)).await;

        // 检查挖矿结果
        match software_core.collect_results().await {
            Ok(results) => {
                for result in results {
                    valid_shares += 1;
                    info!("🎉 发现有效份额 #{}: nonce={:08x}, hash={:02x}{:02x}{:02x}{:02x}...",
                          valid_shares, result.nonce,
                          result.hash[0], result.hash[1], result.hash[2], result.hash[3]);
                }
            }
            Err(e) => {
                warn!("⚠️  获取挖矿结果时出错: {}", e);
            }
        }

        // 获取统计信息
        match software_core.get_stats().await {
            Ok(stats) => {
                total_hashes = (stats.total_hashrate * start_time.elapsed().as_secs_f64()) as u64;

                // 每5秒输出一次统计信息
                if last_stats_time.elapsed() >= Duration::from_secs(5) {
                    let elapsed = start_time.elapsed();
                    let hashrate = stats.total_hashrate;

                    info!("📈 统计信息 [{}s]: 总哈希数={}, 算力={:.2} H/s, 有效份额={}, 错误数={}",
                          elapsed.as_secs(), total_hashes, hashrate, valid_shares, stats.hardware_errors);

                    last_stats_time = Instant::now();
                }
            }
            Err(e) => {
                warn!("⚠️  获取统计信息时出错: {}", e);
            }
        }

        // 每10秒提交新工作
        if i % 10 == 9 {
            let new_work = create_test_work();
            match software_core.submit_work(new_work).await {
                Ok(_) => info!("🔄 提交新工作成功"),
                Err(e) => warn!("⚠️  提交新工作失败: {}", e),
            }
        }
    }

    // 测试6: 最终统计和性能评估
    info!("📊 测试6: 最终统计和性能评估");
    let total_time = start_time.elapsed();
    let final_hashrate = if total_time.as_secs() > 0 {
        total_hashes as f64 / total_time.as_secs() as f64
    } else {
        0.0
    };

    info!("🏁 测试完成！");
    info!("⏱️  总运行时间: {:.2}秒", total_time.as_secs_f64());
    info!("🔢 总哈希计算数: {}", total_hashes);
    info!("⚡ 平均算力: {:.2} H/s ({:.2} KH/s)", final_hashrate, final_hashrate / 1000.0);
    info!("🎯 有效份额数: {}", valid_shares);
    info!("📈 份额率: {:.4} 份额/秒", valid_shares as f64 / total_time.as_secs_f64());

    // 性能评估
    if final_hashrate > 1000.0 {
        info!("✅ 性能评估: 优秀 (>1 KH/s)");
    } else if final_hashrate > 500.0 {
        info!("✅ 性能评估: 良好 (>500 H/s)");
    } else if final_hashrate > 100.0 {
        info!("⚠️  性能评估: 一般 (>100 H/s)");
    } else {
        warn!("❌ 性能评估: 需要优化 (<100 H/s)");
    }

    // 停止核心
    match software_core.stop().await {
        Ok(_) => info!("✅ 软算法核心停止成功"),
        Err(e) => warn!("⚠️  软算法核心停止时出错: {}", e),
    }

    info!("🎉 软算法核心功能验证测试完成！");
    Ok(())
}

/// 创建测试用的工作
fn create_test_work() -> Work {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 创建一个简单的区块头（80字节）
    let mut header = vec![0u8; 80];

    // 版本号 (4字节)
    header[0..4].copy_from_slice(&1u32.to_le_bytes());

    // 前一个区块哈希 (32字节) - 使用随机测试数据
    for i in 4..36 {
        header[i] = ((i * 7 + timestamp as usize) % 256) as u8;
    }

    // Merkle根 (32字节) - 使用随机测试数据
    for i in 36..68 {
        header[i] = ((i * 13 + timestamp as usize) % 256) as u8;
    }

    // 时间戳 (4字节)
    header[68..72].copy_from_slice(&(timestamp as u32).to_le_bytes());

    // 难度目标 (4字节) - 设置较低的难度便于测试
    header[72..76].copy_from_slice(&0x207fffffu32.to_le_bytes());

    // Nonce (4字节) - 初始为0，挖矿时会修改
    header[76..80].copy_from_slice(&0u32.to_le_bytes());

    // 创建目标值 - 设置较低的难度
    let mut target = vec![0x00u8; 32];
    target[0] = 0x00;
    target[1] = 0x00;
    target[2] = 0x7f;
    target[3] = 0xff;
    for i in 4..32 {
        target[i] = 0xff;
    }

    Work {
        id: timestamp,
        header,
        target,
        difficulty: 1.0,
        timestamp: SystemTime::now(),
        extranonce: vec![0u8; 4],
    }
}
