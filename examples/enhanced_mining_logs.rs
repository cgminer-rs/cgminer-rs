//! 增强的挖矿日志示例
//!
//! 展示改进后的获取题目和提交题目的日志记录功能
//! 参考其他挖矿软件的日志格式和细节

use cgminer_rs::logging::mining_logger::MiningLogger;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化美化的日志系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_span_events(FmtSpan::CLOSE)
                .pretty()
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 启动增强挖矿日志示例");
    info!("参考其他挖矿软件的日志格式和细节");

    // 创建挖矿日志记录器（启用详细模式）
    let mut mining_logger = MiningLogger::new(true);

    // 模拟挖矿启动
    mining_logger.log_mining_start(4, 2);
    sleep(Duration::from_millis(500)).await;

    // 模拟矿池连接
    mining_logger.log_pool_connection_change(0, "stratum+tcp://btc.f2pool.com:1314", true, None);
    sleep(Duration::from_millis(300)).await;

    // 模拟接收新工作（mining.notify）
    info!("📋 模拟接收新的挖矿工作...");
    mining_logger.log_work_received(
        0,
        "4d16b6f85af6e219",
        "4d16b6f85af6e2198f44ae2a6de67f78487ae5611b77c6c0440b921e00000000",
        true,
        16384.0
    );
    sleep(Duration::from_millis(500)).await;

    // 模拟工作分发
    mining_logger.log_work_distributed("work_001", 4, 4);
    sleep(Duration::from_millis(300)).await;

    // 模拟难度调整
    info!("🎯 模拟难度调整...");
    mining_logger.log_difficulty_change(0, 16384.0, 32768.0);
    sleep(Duration::from_millis(500)).await;

    // 模拟份额提交和结果
    info!("📤 模拟份额提交...");
    for i in 0..3 {
        let device_id = i;
        let job_id = "4d16b6f85af6e219";
        let nonce = 0x12345678 + i * 1000;
        let ntime = 0x6422ca0e;
        let extranonce2 = format!("{:08x}", i * 0x1000);
        let difficulty = 32768.0;

        // 记录份额提交详情
        mining_logger.log_share_submit_details(
            0,
            device_id,
            job_id,
            nonce,
            ntime,
            &extranonce2,
            difficulty,
        );

        sleep(Duration::from_millis(200)).await;

        // 模拟不同的提交结果
        let accepted = match i {
            0 => true,  // 第一个接受
            1 => false, // 第二个拒绝
            _ => true,  // 其他接受
        };

        let reason = if !accepted {
            Some("duplicate share")
        } else {
            None
        };

        mining_logger.log_share_result(0, device_id, accepted, difficulty, reason);
        sleep(Duration::from_millis(300)).await;
    }

    // 模拟设备状态
    info!("🔧 模拟设备状态更新...");
    for device_id in 0..4 {
        let online = device_id != 2; // 设备2离线
        let temperature = 65.0 + device_id as f32 * 5.0;
        let hashrate = if online { 14_000_000_000.0 } else { 0.0 }; // 14 TH/s
        let power = if online { 1400.0 } else { 0.0 }; // 1400W

        mining_logger.log_device_status(device_id, online, temperature, hashrate, power);
        sleep(Duration::from_millis(100)).await;
    }

    // 模拟矿池状态
    info!("🌊 模拟矿池状态更新...");
    mining_logger.log_pool_status(0, true, "btc.f2pool.com:1314", 45, 156, 3);
    sleep(Duration::from_millis(300)).await;

    // 模拟系统状态
    info!("🖥️ 模拟系统状态更新...");
    mining_logger.log_system_status(75.5, 68.2, 72.0, 5600.0);
    sleep(Duration::from_millis(300)).await;

    // 模拟网络状态
    mining_logger.log_network_status(1024 * 1024, 512 * 1024, 2);
    sleep(Duration::from_millis(300)).await;

    // 模拟Stratum协议消息
    info!("🔗 模拟Stratum协议消息...");
    mining_logger.log_stratum_message("发送", "mining.subscribe", r#"{"id":1,"method":"mining.subscribe","params":["cgminer-rs/1.0.0"]}"#);
    mining_logger.log_stratum_message("接收", "mining.notify", r#"{"id":null,"method":"mining.notify","params":["4d16b6f85af6e219","4d16b6f85af6e2198f44ae2a6de67f78487ae5611b77c6c0440b921e00000000","01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff20020862062f503253482f04b8864e5008","072f736c7573682f000000000100f2052a010000001976a914d23fcdf86f7e756a64a7a9688ef9903327048ed988ac00000000",[],"00000002","1c2ac4af","504e86b9",true]}"#);
    sleep(Duration::from_millis(500)).await;

    // 模拟另一个新区块
    info!("🆕 模拟检测到新区块...");
    mining_logger.log_work_received(
        0,
        "5e27d3a4b8c9f123",
        "5e27d3a4b8c9f1238f44ae2a6de67f78487ae5611b77c6c0440b921e00000000",
        true,
        32768.0
    );
    sleep(Duration::from_millis(300)).await;

    // 模拟错误情况
    info!("❌ 模拟错误情况...");
    mining_logger.log_error("Stratum", Some(1), "连接超时");
    mining_logger.log_pool_connection_change(0, "stratum+tcp://btc.f2pool.com:1314", false, Some("网络错误"));
    sleep(Duration::from_millis(500)).await;

    // 模拟重新连接
    info!("🔄 模拟重新连接...");
    mining_logger.log_pool_connection_change(0, "stratum+tcp://btc.f2pool.com:1314", true, None);
    sleep(Duration::from_millis(300)).await;

    // 模拟挖矿停止
    mining_logger.log_mining_stop();

    info!("✅ 增强挖矿日志示例完成");
    info!("📝 日志格式参考了其他挖矿软件的最佳实践");
    info!("🎯 包含了详细的工作获取和份额提交日志");

    Ok(())
}
