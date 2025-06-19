//! 简单的挖矿日志测试
//!
//! 测试改进后的获取题目和提交题目的日志记录功能

use cgminer_rs::logging::mining_logger::MiningLogger;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化简单的日志系统，设置为info级别
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 启动简单挖矿日志测试");

    // 创建挖矿日志记录器（启用详细模式）
    let mut mining_logger = MiningLogger::new(true);

    // 测试挖矿启动日志
    println!("\n=== 测试挖矿启动日志 ===");
    mining_logger.log_mining_start(4, 2);
    sleep(Duration::from_millis(500)).await;

    // 测试矿池连接日志
    println!("\n=== 测试矿池连接日志 ===");
    mining_logger.log_pool_connection_change(0, "stratum+tcp://btc.f2pool.com:1314", true, None);
    sleep(Duration::from_millis(300)).await;

    // 测试工作接收日志（mining.notify）
    println!("\n=== 测试工作接收日志 (mining.notify) ===");
    mining_logger.log_work_received(
        0,
        "4d16b6f85af6e219",
        "4d16b6f85af6e2198f44ae2a6de67f78487ae5611b77c6c0440b921e00000000",
        true,
        16384.0
    );
    sleep(Duration::from_millis(500)).await;

    // 测试难度调整日志
    println!("\n=== 测试难度调整日志 ===");
    mining_logger.log_difficulty_change(0, 16384.0, 32768.0);
    sleep(Duration::from_millis(500)).await;

    // 测试份额提交日志（mining.submit）
    println!("\n=== 测试份额提交日志 (mining.submit) ===");

    // 提交详情
    mining_logger.log_share_submit_details(
        0,
        1,
        "4d16b6f85af6e219",
        0x12345678,
        0x6422ca0e,
        "deadbeef",
        32768.0,
    );
    sleep(Duration::from_millis(200)).await;

    // 提交结果 - 接受
    mining_logger.log_share_result(0, 1, true, 32768.0, None);
    sleep(Duration::from_millis(300)).await;

    // 提交详情 - 第二个份额
    mining_logger.log_share_submit_details(
        0,
        2,
        "4d16b6f85af6e219",
        0x87654321,
        0x6422ca0e,
        "beefdead",
        32768.0,
    );
    sleep(Duration::from_millis(200)).await;

    // 提交结果 - 拒绝
    mining_logger.log_share_result(0, 2, false, 32768.0, Some("duplicate share"));
    sleep(Duration::from_millis(300)).await;

    // 测试设备状态日志
    println!("\n=== 测试设备状态日志 ===");
    mining_logger.log_device_status(1, true, 68.5, 14_000_000_000.0, 1400.0);
    mining_logger.log_device_status(2, false, 0.0, 0.0, 0.0);
    sleep(Duration::from_millis(300)).await;

    // 测试矿池状态日志
    println!("\n=== 测试矿池状态日志 ===");
    mining_logger.log_pool_status(0, true, "btc.f2pool.com:1314", 45, 156, 3);
    sleep(Duration::from_millis(300)).await;

    // 测试错误日志
    println!("\n=== 测试错误日志 ===");
    mining_logger.log_error("Stratum", Some(1), "连接超时");
    sleep(Duration::from_millis(300)).await;

    // 测试挖矿停止日志
    println!("\n=== 测试挖矿停止日志 ===");
    mining_logger.log_mining_stop();

    println!("\n✅ 简单挖矿日志测试完成");
    println!("📝 展示了参考其他挖矿软件的日志格式");
    println!("🎯 包含了详细的工作获取和份额提交日志");

    Ok(())
}
