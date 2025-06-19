//! 设备扩展测试程序
//!
//! 测试从4个设备扩展到32个设备的设备管理功能

use cgminer_rs::CoreLoader;
use cgminer_rs::device::DeviceManager;
use cgminer_rs::config::DeviceConfig;
use std::env;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 开始设备扩展测试");

    // 测试不同的设备数量
    let test_counts = vec![4, 8, 16, 32];

    for device_count in test_counts {
        info!("📊 测试 {} 个设备", device_count);

        match test_device_count(device_count).await {
            Ok(()) => {
                info!("✅ {} 个设备测试成功", device_count);
            }
            Err(e) => {
                error!("❌ {} 个设备测试失败: {}", device_count, e);
                break;
            }
        }

        // 测试间隔
        sleep(Duration::from_secs(2)).await;
    }

    info!("🎯 设备扩展测试完成");
    Ok(())
}

async fn test_device_count(device_count: u32) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 配置 {} 个软算法设备", device_count);

    // 设置环境变量来控制设备数量
    env::set_var("CGMINER_SOFTWARE_DEVICE_COUNT", device_count.to_string());

    // 创建核心加载器
    let core_loader = CoreLoader::new();

    // 加载所有可用的核心
    core_loader.load_all_cores().await?;

    // 获取核心注册表
    let core_registry = core_loader.registry();

    // 创建设备管理器
    let device_config = DeviceConfig {
        auto_detect: true,
        scan_interval: 5,
        chains: vec![],
    };
    let mut device_manager = DeviceManager::new(device_config, core_registry);

    // 初始化设备管理器
    device_manager.initialize().await?;

    // 获取创建的设备信息
    let device_infos = device_manager.get_all_device_info().await;
    let actual_count = device_infos.len();

    info!("📋 实际创建了 {} 个设备", actual_count);

    // 验证设备数量
    if actual_count != device_count as usize {
        warn!("⚠️ 期望 {} 个设备，实际创建 {} 个", device_count, actual_count);
    }

    // 显示设备信息
    for device_info in &device_infos {
        info!("📱 设备: ID={}, 名称={}, 类型={}",
              device_info.id, device_info.name, device_info.device_type);
    }

    // 启动设备管理器
    device_manager.start().await?;

    // 运行一段时间
    info!("⏱️ 运行设备管理器 5 秒...");
    sleep(Duration::from_secs(5)).await;

    // 检查设备健康状态
    let mut healthy_count = 0;
    for device_info in &device_infos {
        match device_manager.health_check(device_info.id).await {
            Ok(true) => {
                healthy_count += 1;
            }
            Ok(false) => {
                warn!("⚠️ 设备 {} 不健康", device_info.id);
            }
            Err(e) => {
                error!("❌ 设备 {} 健康检查失败: {}", device_info.id, e);
            }
        }
    }

    info!("💚 健康设备数量: {}/{}", healthy_count, actual_count);

    // 获取总算力
    let total_hashrate = device_manager.get_total_hashrate().await;
    info!("⚡ 总算力: {:.2} H/s", total_hashrate);

    // 停止设备管理器
    device_manager.stop().await?;

    info!("✅ {} 个设备测试完成", device_count);

    // 清理环境变量
    env::remove_var("CGMINER_SOFTWARE_DEVICE_COUNT");

    Ok(())
}

/// 性能基准测试
async fn benchmark_device_performance(device_count: u32) -> Result<(), Box<dyn std::error::Error>> {
    info!("🏃 开始 {} 个设备的性能基准测试", device_count);

    let start_time = std::time::Instant::now();

    // 设置环境变量
    env::set_var("CGMINER_SOFTWARE_DEVICE_COUNT", device_count.to_string());

    // 创建和初始化设备管理器
    let core_loader = CoreLoader::new();
    core_loader.load_all_cores().await?;
    let core_registry = core_loader.registry();
    let device_config = DeviceConfig {
        auto_detect: true,
        scan_interval: 5,
        chains: vec![],
    };
    let mut device_manager = DeviceManager::new(device_config, core_registry);

    let init_start = std::time::Instant::now();
    device_manager.initialize().await?;
    let init_duration = init_start.elapsed();

    let start_start = std::time::Instant::now();
    device_manager.start().await?;
    let start_duration = start_start.elapsed();

    let total_duration = start_time.elapsed();

    info!("📊 性能指标:");
    info!("  - 初始化时间: {:?}", init_duration);
    info!("  - 启动时间: {:?}", start_duration);
    info!("  - 总时间: {:?}", total_duration);

    // 内存使用情况
    let device_count_actual = device_manager.get_all_device_info().await.len();
    info!("  - 实际设备数: {}", device_count_actual);

    device_manager.stop().await?;
    env::remove_var("CGMINER_SOFTWARE_DEVICE_COUNT");

    Ok(())
}
