use cgminer_rs::config::Config;
use cgminer_s_btc_core::cpu_affinity::{CpuAffinityManager, CpuAffinityStrategy, CpuAffinityConfig};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() {
    // 初始化日志
    init_logging().expect("Failed to initialize logging");

    info!("═══════════════════════════════════════════════════════════");
    info!("🔗 CGMiner-RS CPU绑定功能测试");
    info!("═══════════════════════════════════════════════════════════");

    // 显示系统CPU信息
    show_system_cpu_info();

    // 测试不同的CPU绑定策略
    test_round_robin_strategy().await;
    test_manual_strategy().await;
    test_performance_first_strategy().await;
    test_physical_cores_only_strategy().await;

    // 测试配置文件中的CPU绑定设置
    test_config_cpu_affinity().await;

    info!("═══════════════════════════════════════════════════════════");
    info!("✅ CPU绑定功能测试完成");
    info!("═══════════════════════════════════════════════════════════");
}

fn show_system_cpu_info() {
    info!("🖥️  系统CPU信息:");
    info!("   💻 逻辑CPU核心数: {}", CpuAffinityManager::get_cpu_count());
    info!("   🔧 物理CPU核心数: {}", CpuAffinityManager::get_physical_cpu_count());
    info!("───────────────────────────────────────────────────────────");
}

async fn test_round_robin_strategy() {
    info!("🔄 测试轮询分配策略 (Round Robin)");

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::RoundRobin);

    // 为8个设备分配CPU核心
    for device_id in 0..8 {
        if let Some(core_id) = manager.assign_cpu_core(device_id) {
            info!("   ✅ 设备 {} → CPU核心 {:?}", device_id, core_id);
        } else {
            error!("   ❌ 设备 {} 分配失败", device_id);
        }
    }

    manager.print_affinity_status();
    info!("───────────────────────────────────────────────────────────");
}

async fn test_manual_strategy() {
    info!("🎯 测试手动分配策略 (Manual)");

    // 创建手动映射
    let mut manual_mapping = HashMap::new();
    manual_mapping.insert(0, 0);  // 设备0 → CPU核心0
    manual_mapping.insert(1, 2);  // 设备1 → CPU核心2
    manual_mapping.insert(2, 1);  // 设备2 → CPU核心1
    manual_mapping.insert(3, 3);  // 设备3 → CPU核心3

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::Manual(manual_mapping));

    // 为4个设备分配CPU核心
    for device_id in 0..4 {
        if let Some(core_id) = manager.assign_cpu_core(device_id) {
            info!("   ✅ 设备 {} → CPU核心 {:?}", device_id, core_id);
        } else {
            error!("   ❌ 设备 {} 分配失败", device_id);
        }
    }

    manager.print_affinity_status();
    info!("───────────────────────────────────────────────────────────");
}

async fn test_performance_first_strategy() {
    info!("⚡ 测试性能核心优先策略 (Performance First)");

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::PerformanceFirst);

    // 为6个设备分配CPU核心
    for device_id in 0..6 {
        if let Some(core_id) = manager.assign_cpu_core(device_id) {
            info!("   ✅ 设备 {} → CPU核心 {:?} (性能核心)", device_id, core_id);
        } else {
            error!("   ❌ 设备 {} 分配失败", device_id);
        }
    }

    manager.print_affinity_status();
    info!("───────────────────────────────────────────────────────────");
}

async fn test_physical_cores_only_strategy() {
    info!("🔧 测试物理核心策略 (Physical Cores Only)");

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::PhysicalCoresOnly);

    // 为4个设备分配CPU核心
    for device_id in 0..4 {
        if let Some(core_id) = manager.assign_cpu_core(device_id) {
            info!("   ✅ 设备 {} → CPU核心 {:?} (物理核心)", device_id, core_id);
        } else {
            error!("   ❌ 设备 {} 分配失败", device_id);
        }
    }

    manager.print_affinity_status();
    info!("───────────────────────────────────────────────────────────");
}

async fn test_config_cpu_affinity() {
    info!("📋 测试配置文件CPU绑定设置");

    // 加载配置文件
    match Config::load("cgminer.toml") {
        Ok(config) => {
            info!("✅ 配置文件加载成功");

            if let Some(software_config) = &config.cores.software_core {
                info!("📊 软算法核心配置:");
                info!("   🔧 设备数量: {}", software_config.device_count);
                info!("   🔗 CPU绑定启用: {}", software_config.cpu_affinity.enabled);
                info!("   📋 绑定策略: {}", software_config.cpu_affinity.strategy);

                if software_config.cpu_affinity.enabled {
                    // 根据配置创建CPU绑定管理器
                    let strategy = match software_config.cpu_affinity.strategy.as_str() {
                        "round_robin" => CpuAffinityStrategy::RoundRobin,
                        "performance_first" => CpuAffinityStrategy::PerformanceFirst,
                        "physical_only" => CpuAffinityStrategy::PhysicalCoresOnly,
                        "manual" => {
                            if let Some(mapping) = &software_config.cpu_affinity.manual_mapping {
                                CpuAffinityStrategy::Manual(mapping.clone())
                            } else {
                                info!("   ⚠️ 手动策略但未提供映射，回退到轮询策略");
                                CpuAffinityStrategy::RoundRobin
                            }
                        }
                        _ => {
                            info!("   ⚠️ 未知策略 '{}', 使用轮询策略", software_config.cpu_affinity.strategy);
                            CpuAffinityStrategy::RoundRobin
                        }
                    };

                    let mut manager = CpuAffinityManager::new(true, strategy);

                    // 为配置的设备数量分配CPU核心
                    for device_id in 0..software_config.device_count {
                        if let Some(core_id) = manager.assign_cpu_core(device_id) {
                            info!("   ✅ 设备 {} → CPU核心 {:?}", device_id, core_id);
                        } else {
                            error!("   ❌ 设备 {} 分配失败", device_id);
                        }
                    }

                    manager.print_affinity_status();
                } else {
                    info!("   ⚠️ CPU绑定已禁用");
                }
            } else {
                error!("   ❌ 软算法核心配置缺失");
            }
        }
        Err(e) => {
            error!("❌ 配置文件加载失败: {}", e);
        }
    }

    info!("───────────────────────────────────────────────────────────");
}

async fn test_thread_binding() {
    info!("🧵 测试线程CPU绑定");

    let mut manager = CpuAffinityManager::new(true, CpuAffinityStrategy::RoundRobin);

    // 分配CPU核心
    let device_id = 0;
    if let Some(_core_id) = manager.assign_cpu_core(device_id) {
        // 在新线程中测试CPU绑定
        let manager_clone = std::sync::Arc::new(std::sync::RwLock::new(manager));
        let manager_for_thread = manager_clone.clone();

        let handle = thread::spawn(move || {
            let manager = manager_for_thread.read().unwrap();
            match manager.bind_current_thread(device_id) {
                Ok(_) => {
                    info!("   ✅ 线程成功绑定到CPU核心");

                    // 模拟一些CPU密集型工作
                    let start = std::time::Instant::now();
                    let mut sum = 0u64;
                    for i in 0..1_000_000 {
                        sum = sum.wrapping_add(i);
                    }
                    let elapsed = start.elapsed();

                    info!("   📊 计算完成: sum={}, 耗时: {:?}", sum, elapsed);
                }
                Err(e) => {
                    error!("   ❌ 线程绑定失败: {}", e);
                }
            }
        });

        handle.join().unwrap();
    } else {
        error!("   ❌ CPU核心分配失败");
    }

    info!("───────────────────────────────────────────────────────────");
}

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
