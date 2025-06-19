//! 矿池通讯Debug日志测试
//!
//! 展示详细的矿池通讯debug日志功能

use cgminer_rs::pool::stratum::StratumClient;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化详细的日志系统，包含debug级别
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

    info!("🚀 启动矿池通讯Debug日志测试");
    info!("📋 这个示例将展示详细的Stratum协议通讯debug日志");

    // 测试连接到一个真实的矿池（这里使用一个测试地址）
    let pool_url = "stratum+tcp://192.168.18.240:10203".to_string();
    let username = "kayuii.bbt".to_string();
    let password = "123".to_string();

    info!("🌊 准备连接到矿池: {}", pool_url);
    info!("👤 用户名: {}", username);

    // 创建Stratum客户端（启用详细日志）
    let mut stratum_client = match StratumClient::new(
        pool_url.clone(),
        username,
        password,
        0, // pool_id
        true, // verbose
    ).await {
        Ok(client) => {
            info!("✅ Stratum客户端创建成功");
            client
        },
        Err(e) => {
            error!("❌ 创建Stratum客户端失败: {}", e);
            return Err(e.into());
        }
    };

    // 尝试连接
    info!("🔗 开始连接到矿池...");
    match stratum_client.connect().await {
        Ok(_) => {
            info!("✅ 成功连接到矿池！");

            // 等待一段时间以观察通讯
            info!("⏳ 等待5秒以观察矿池通讯...");
            sleep(Duration::from_secs(5)).await;

            // 检查extranonce配置
            info!("🔍 检查extranonce配置...");
            match stratum_client.validate_extranonce_config().await {
                Ok(_) => {
                    info!("✅ extranonce配置验证成功");

                    let (extranonce1, extranonce2_size) = stratum_client.get_extranonce_info().await;
                    info!("📋 extranonce1: {:?}", extranonce1);
                    info!("📋 extranonce2_size: {}", extranonce2_size);
                },
                Err(e) => {
                    error!("❌ extranonce配置验证失败: {}", e);
                }
            }

            // 获取当前难度
            let difficulty = stratum_client.get_current_difficulty().await;
            info!("🎯 当前挖矿难度: {}", difficulty);

            // 测试ping
            info!("🏓 发送ping测试...");
            match stratum_client.ping().await {
                Ok(_) => {
                    info!("✅ ping测试成功");
                },
                Err(e) => {
                    error!("❌ ping测试失败: {}", e);
                }
            }

            // 再等待一段时间
            info!("⏳ 再等待5秒以观察更多通讯...");
            sleep(Duration::from_secs(5)).await;

            // 断开连接
            info!("🔌 断开连接...");
            match stratum_client.disconnect().await {
                Ok(_) => {
                    info!("✅ 成功断开连接");
                },
                Err(e) => {
                    error!("❌ 断开连接失败: {}", e);
                }
            }
        },
        Err(e) => {
            error!("❌ 连接到矿池失败: {}", e);

            // 分析错误类型
            match &e {
                cgminer_rs::error::PoolError::ProtocolError { error, .. } => {
                    if error.contains("extranonce1") {
                        error!("🔍 这是extranonce1相关的错误");
                        error!("💡 可能的原因:");
                        error!("   1. 矿池返回的mining.subscribe响应格式不标准");
                        error!("   2. extranonce1字段缺失或格式错误");
                        error!("   3. 矿池不支持标准的Stratum协议");
                        error!("💡 建议:");
                        error!("   1. 检查矿池是否支持标准Stratum协议");
                        error!("   2. 查看debug日志中的详细响应内容");
                        error!("   3. 联系矿池管理员确认协议兼容性");
                    }
                },
                cgminer_rs::error::PoolError::ConnectionFailed { .. } => {
                    error!("🔍 这是网络连接错误");
                    error!("💡 可能的原因:");
                    error!("   1. 矿池地址或端口错误");
                    error!("   2. 网络连接问题");
                    error!("   3. 防火墙阻止连接");
                },
                cgminer_rs::error::PoolError::Timeout { .. } => {
                    error!("🔍 这是连接超时错误");
                    error!("💡 可能的原因:");
                    error!("   1. 矿池响应缓慢");
                    error!("   2. 网络延迟过高");
                    error!("   3. 矿池服务器负载过高");
                },
                _ => {
                    error!("🔍 其他类型的错误: {:?}", e);
                }
            }
        }
    }

    info!("📝 Debug日志测试完成");
    info!("💡 提示: 设置环境变量 RUST_LOG=debug 可以看到更详细的日志");
    info!("💡 示例: RUST_LOG=debug cargo run --example debug_pool_communication");

    Ok(())
}
