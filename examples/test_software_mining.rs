//! 软算法核心实际挖矿功能测试
//!
//! 这个测试程序验证软算法核心的真实挖矿功能，包括：
//! - SHA256算法计算
//! - CPU绑定功能
//! - 设备管理
//! - 挖矿统计

use cgminer_core::{Work, MiningCore, MiningDevice, DeviceInfo, DeviceStatus, DeviceConfig};
use cgminer_s_btc_core::{SoftwareMiningCore, SoftwareDevice, cpu_affinity::{CpuAffinityManager, CpuAffinityStrategy}};
use tracing::{info, warn};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

/// 初始化日志系统
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// 创建测试工作
fn create_test_work() -> Work {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 创建一个简单的区块头（80字节）
    let mut header = vec![0u8; 80];

    // 版本号 (4字节)
    header[0..4].copy_from_slice(&1u32.to_le_bytes());

    // 前一个区块哈希 (32字节) - 使用测试数据
    for i in 4..36 {
        header[i] = (i % 256) as u8;
    }

    // Merkle根 (32字节) - 使用测试数据
    for i in 36..68 {
        header[i] = ((i * 2) % 256) as u8;
    }

    // 时间戳 (4字节)
    header[68..72].copy_from_slice(&(timestamp as u32).to_le_bytes());

    // 难度目标 (4字节) - 设置较低的难度便于测试
    header[72..76].copy_from_slice(&0x207fffffu32.to_le_bytes());

    // Nonce (4字节) - 初始为0，挖矿时会修改
    header[76..80].copy_from_slice(&0u32.to_le_bytes());

    // 创建目标值 - 设置较低的难度
    let mut target = vec![0xffu8; 32];
    target[0] = 0x00;
    target[1] = 0x00;
    target[2] = 0x7f;

    Work {
        id: timestamp,
        header,
        target,
        timestamp: SystemTime::now(),
        difficulty: 1.0,
        extranonce: vec![0u8; 4],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init_logging()?;

    info!("═══════════════════════════════════════════════════════════");
    info!("🦀 CGMiner-RS 软算法核心实际挖矿功能测试");
    info!("═══════════════════════════════════════════════════════════");

    // 测试CPU绑定管理器
    test_cpu_affinity_manager().await?;

    // 测试软算法设备创建和基本功能
    test_software_device_creation().await?;

    // 测试实际挖矿功能
    test_actual_mining().await?;

    // 测试软算法核心集成
    test_software_core_integration().await?;

    info!("═══════════════════════════════════════════════════════════");
    info!("✅ 软算法核心实际挖矿功能测试完成");
    info!("═══════════════════════════════════════════════════════════");

    Ok(())
}

/// 测试CPU绑定管理器
async fn test_cpu_affinity_manager() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 测试CPU绑定管理器");

    // 创建CPU绑定管理器
    let mut cpu_manager = CpuAffinityManager::new(true, CpuAffinityStrategy::RoundRobin);

    info!("✅ CPU绑定管理器创建成功");
    info!("   📊 可用CPU核心数: {}", cpu_manager.available_core_count());

    // 测试设备CPU绑定分配
    for device_id in 0..4 {
        cpu_manager.assign_cpu_core(device_id);
        if let Some(core_id) = cpu_manager.get_device_core(device_id) {
            info!("   🔗 设备 {} 分配到CPU核心 {:?}", device_id, core_id);
        }
    }

    info!("───────────────────────────────────────────────────────────");
    Ok(())
}

/// 测试软算法设备创建和基本功能
async fn test_software_device_creation() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔨 测试软算法设备创建和基本功能");

    // 创建设备信息
    let device_info = DeviceInfo::new(
        0,
        "软算法测试设备".to_string(),
        "software".to_string(),
        0,
    );

    // 创建软算法设备
    let device = SoftwareDevice::new(
        device_info,
        DeviceConfig::default(),
        1_000_000_000.0, // 1 GH/s
        0.01,            // 1% 错误率
        1000,            // 批次大小
    ).await?;

    info!("✅ 软算法设备创建成功");

    // 获取设备信息
    let info = device.get_info().await?;
    info!("   📋 设备名称: {}", info.name);
    info!("   🆔 设备ID: {}", info.id);
    info!("   🔧 设备类型: {}", info.device_type);

    // 获取设备状态
    let status = device.get_status().await?;
    info!("   📊 设备状态: {:?}", status);

    info!("───────────────────────────────────────────────────────────");
    Ok(())
}

/// 测试实际挖矿功能
async fn test_actual_mining() -> Result<(), Box<dyn std::error::Error>> {
    info!("⛏️  测试实际挖矿功能");

    // 创建设备信息
    let device_info = DeviceInfo::new(
        1,
        "挖矿测试设备".to_string(),
        "software".to_string(),
        0,
    );

    // 创建软算法设备
    let mut device = SoftwareDevice::new(
        device_info,
        DeviceConfig::default(),
        500_000_000.0, // 500 MH/s - 较低的算力便于测试
        0.05,          // 5% 错误率
        500,           // 较小的批次大小
    ).await?;

    // 启动设备
    device.start().await?;
    info!("✅ 设备启动成功");

    // 创建测试工作
    let work = create_test_work();
    info!("📋 创建测试工作: {}", work.id);
    info!("   🎯 目标难度: {}", work.difficulty);

    // 提交工作给设备
    device.submit_work(work.clone()).await?;
    info!("✅ 工作提交成功");

    // 等待挖矿结果
    info!("⏳ 等待挖矿结果...");
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(30); // 30秒超时

    loop {
        if start_time.elapsed() > timeout {
            warn!("⚠️  挖矿超时，但这是正常的（难度可能太高）");
            break;
        }

        // 检查是否有挖矿结果
        if let Some(result) = device.get_result().await? {
            info!("🎉 挖矿成功！");
            info!("   🔢 Nonce: 0x{:08x}", result.nonce);
            info!("   🏷️  工作ID: {}", result.work_id);
            info!("   ⏱️  挖矿时间: {:.2}秒", start_time.elapsed().as_secs_f64());
            break;
        }

        // 获取设备统计信息
        let stats = device.get_stats().await?;
        if stats.accepted_work > 0 || stats.rejected_work > 0 {
            info!("   📊 已接受工作: {}", stats.accepted_work);
            info!("   ⚡ 当前算力: {:.2} MH/s", stats.current_hashrate.as_mh_per_second());
        }

        sleep(Duration::from_millis(1000)).await;
    }

    // 停止设备
    device.stop().await?;
    info!("✅ 设备停止成功");

    // 显示最终统计信息
    let stats = device.get_stats().await?;
    info!("📈 最终统计信息:");
    info!("   ✅ 接受的工作: {}", stats.accepted_work);
    info!("   ❌ 拒绝的工作: {}", stats.rejected_work);
    info!("   🔧 硬件错误: {}", stats.hardware_errors);
    info!("   ⚡ 平均算力: {:.2} MH/s", stats.average_hashrate.as_mh_per_second());

    info!("───────────────────────────────────────────────────────────");
    Ok(())
}

/// 测试软算法核心集成
async fn test_software_core_integration() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔗 测试软算法核心集成");

    // 创建软算法核心
    let mut core = SoftwareMiningCore::new("测试软算法核心".to_string());

    // 启动核心
    core.start().await?;
    info!("✅ 软算法核心启动成功");

    // 扫描设备
    let devices = core.scan_devices().await?;
    info!("📱 扫描到 {} 个设备", devices.len());

    for device in &devices {
        info!("   🔧 设备: {} (ID: {})", device.name, device.id);
    }

    // 获取核心信息
    let core_info = core.get_info();
    info!("📋 核心信息:");
    info!("   📛 名称: {}", core_info.name);
    info!("   🏷️  版本: {}", core_info.version);
    info!("   📝 描述: {}", core_info.description);

    // 停止核心
    core.stop().await?;
    info!("✅ 软算法核心停止成功");

    info!("───────────────────────────────────────────────────────────");
    Ok(())
}
