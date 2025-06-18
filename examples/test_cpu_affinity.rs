//! CPU绑定功能测试
//!
//! 这个示例程序测试CPU绑定功能在Mac环境下的效果

use cgminer_s_btc_core::cpu_affinity::{CpuAffinityManager, CpuAffinityStrategy};
use std::time::Instant;
use tracing::{info, warn};
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

    info!("🖥️  开始CPU绑定功能测试");

    // 测试1: 系统CPU信息检测
    info!("🔍 测试1: 系统CPU信息检测");
    test_cpu_detection().await?;

    // 测试2: CPU绑定管理器创建
    info!("🏗️  测试2: CPU绑定管理器创建");
    test_cpu_affinity_manager_creation().await?;

    // 测试3: 不同策略测试
    info!("🎯 测试3: 不同策略测试");
    test_different_strategies().await?;

    // 测试4: Mac环境兼容性测试
    info!("🍎 测试4: Mac环境兼容性测试");
    test_mac_compatibility().await?;

    info!("🎉 CPU绑定功能测试全部完成！");
    Ok(())
}

/// 测试CPU检测功能
async fn test_cpu_detection() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔍 检测系统CPU信息...");

    // 获取CPU核心数
    let logical_cores = num_cpus::get();
    let physical_cores = num_cpus::get_physical();

    info!("💻 逻辑CPU核心数: {}", logical_cores);
    info!("🔧 物理CPU核心数: {}", physical_cores);

    // 检查超线程
    if logical_cores > physical_cores {
        info!("✅ 检测到超线程技术 (HT/SMT)");
        info!("📊 超线程比例: {}:1", logical_cores / physical_cores);
    } else {
        info!("ℹ️  未检测到超线程技术");
    }

    // 检查CPU架构
    let arch = std::env::consts::ARCH;
    info!("🏗️  CPU架构: {}", arch);

    // Mac特定检查
    if cfg!(target_os = "macos") {
        info!("🍎 运行在macOS环境");

        // 检查是否为Apple Silicon
        if arch == "aarch64" {
            info!("🚀 检测到Apple Silicon (ARM64)");
            warn!("⚠️  Apple Silicon的CPU绑定可能有限制");
        } else {
            info!("💻 检测到Intel Mac");
        }
    }

    Ok(())
}

/// 测试CPU绑定管理器创建
async fn test_cpu_affinity_manager_creation() -> Result<(), Box<dyn std::error::Error>> {
    info!("🏗️  创建CPU绑定管理器...");

    // 测试不同策略的管理器创建
    let strategies = vec![
        ("智能策略", CpuAffinityStrategy::Intelligent),
        ("轮询策略", CpuAffinityStrategy::RoundRobin),
        ("负载均衡策略", CpuAffinityStrategy::LoadBalanced),
        ("仅物理核心策略", CpuAffinityStrategy::PhysicalCoresOnly),
        ("性能优先策略", CpuAffinityStrategy::PerformanceFirst),
    ];

    for (name, strategy) in strategies {
        info!("🔧 测试{}: {:?}", name, strategy);

        let _manager = CpuAffinityManager::new(true, strategy);
        info!("✅ {} 管理器创建成功", name);

        // 获取可用CPU核心数量（使用系统信息）
        let logical_cores = num_cpus::get();
        info!("📊 可用CPU核心数: {}", logical_cores);

        if logical_cores > 0 {
            info!("🎯 可用核心范围: 0-{}", logical_cores - 1);
        }
    }

    Ok(())
}

/// 测试不同策略
async fn test_different_strategies() -> Result<(), Box<dyn std::error::Error>> {
    info!("🎯 测试不同CPU绑定策略...");

    // 智能策略测试
    info!("🧠 测试智能策略");
    let mut intelligent_manager = CpuAffinityManager::new(true, CpuAffinityStrategy::Intelligent);

    for device_id in 1000..1004 {
        intelligent_manager.assign_cpu_core(device_id);
        let assigned_core = intelligent_manager.get_device_core(device_id);
        if let Some(core) = assigned_core {
            info!("📱 设备 {} 分配到CPU核心: {:?}", device_id, core);
        } else {
            warn!("⚠️  设备 {} 未能分配CPU核心", device_id);
        }
    }

    // 轮询策略测试
    info!("🔄 测试轮询策略");
    let mut round_robin_manager = CpuAffinityManager::new(true, CpuAffinityStrategy::RoundRobin);

    for device_id in 2000..2004 {
        round_robin_manager.assign_cpu_core(device_id);
        let assigned_core = round_robin_manager.get_device_core(device_id);
        if let Some(core) = assigned_core {
            info!("📱 设备 {} 分配到CPU核心: {:?}", device_id, core);
        }
    }

    // 测试设备CPU分配
    info!("📱 测试设备CPU分配...");

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::Intelligent);

    // 模拟多个设备分配
    let device_count = 8;
    info!("🔢 模拟 {} 个设备的CPU分配", device_count);

    for device_id in 0..device_count {
        manager.assign_cpu_core(device_id);

        if let Some(core) = manager.get_device_core(device_id) {
            info!("✅ 设备 {} -> CPU核心 {:?}", device_id, core);
        } else {
            warn!("❌ 设备 {} 分配失败", device_id);
        }
    }

    // 测试重复分配
    info!("🔄 测试重复分配...");
    manager.assign_cpu_core(0);
    if let Some(core) = manager.get_device_core(0) {
        info!("🔄 设备 0 重新分配到CPU核心: {:?}", core);
    }

    // 测试分配状态检查
    info!("🔍 测试分配状态检查...");
    if manager.get_device_core(0).is_some() {
        info!("✅ 设备 0 的CPU分配状态正常");
    } else {
        warn!("⚠️  设备 0 的CPU分配状态异常");
    }

    Ok(())
}

/// 测试Mac环境兼容性
async fn test_mac_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    info!("🍎 测试Mac环境兼容性...");

    if cfg!(target_os = "macos") {
        info!("✅ 运行在macOS环境");

        // 测试CPU绑定是否可用
        let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::Intelligent);

        // 尝试分配CPU核心
        let test_device_id = 9999;
        manager.assign_cpu_core(test_device_id);

        if let Some(core) = manager.get_device_core(test_device_id) {
            info!("✅ macOS CPU绑定功能正常，分配核心: {:?}", core);

            // 尝试实际绑定（这在macOS上可能会失败）
            match manager.bind_current_thread(test_device_id) {
                Ok(_) => info!("🎉 macOS线程CPU绑定成功"),
                Err(e) => {
                    warn!("⚠️  macOS线程CPU绑定失败: {}", e);
                    info!("ℹ️  这在macOS上是正常的，系统限制了CPU绑定功能");
                }
            }
        } else {
            warn!("❌ macOS CPU核心分配失败");
        }

        // 检查系统限制
        info!("🔍 检查macOS系统限制...");

        // Apple Silicon特殊处理
        if std::env::consts::ARCH == "aarch64" {
            warn!("🚨 Apple Silicon检测到以下限制:");
            warn!("   • CPU绑定功能受限");
            warn!("   • 性能核心和效率核心混合");
            warn!("   • 系统调度器优先级更高");
            info!("💡 建议: 在Apple Silicon上依赖系统调度器");
        } else {
            info!("💻 Intel Mac环境，CPU绑定支持更好");
        }

        // 性能测试
        info!("⚡ 简单性能测试...");
        let start_time = Instant::now();

        // 执行一些CPU密集型工作
        let mut result = 0u64;
        for i in 0..1000000 {
            result = result.wrapping_add(i * i);
        }

        let duration = start_time.elapsed();
        info!("⏱️  计算耗时: {:.2}ms (结果: {})", duration.as_millis(), result % 1000);

        if duration.as_millis() < 100 {
            info!("🚀 CPU性能良好");
        } else {
            info!("📊 CPU性能正常");
        }

    } else {
        info!("ℹ️  非macOS环境，跳过Mac兼容性测试");
    }

    Ok(())
}
