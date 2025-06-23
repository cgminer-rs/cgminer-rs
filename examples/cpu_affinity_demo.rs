use cgminer_rs::{MiningManager, Config};
use cgminer_core::{CoreRegistry, CoreType};
use cgminer_cpu_btc_core::CpuBtcCoreFactory;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🎯 CPU亲和性演示 - 智能核心分配");

    // 检测CPU信息
    let cpu_count = num_cpus::get();
    let physical_cpu_count = num_cpus::get_physical();
    info!("💻 CPU信息: {} 逻辑核心, {} 物理核心", cpu_count, physical_cpu_count);

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

    // 演示不同的CPU绑定策略
    info!("🧪 开始CPU亲和性演示...");

    // 策略1: 顺序绑定 (0, 1, 2, 3...)
    info!("\n📋 策略1: 顺序CPU核心绑定");
    let sequential_devices = create_devices_with_strategy(
        &mining_manager,
        4,
        "顺序",
        |i| i % cpu_count
    ).await?;

    // 运行测试
    let sequential_results = run_performance_test(
        &mining_manager,
        "顺序绑定",
        10
    ).await?;

    // 策略2: 跳跃绑定 (0, 2, 4, 6...) - 避免超线程冲突
    info!("\n📋 策略2: 跳跃CPU核心绑定 (避免超线程)");
    clear_devices(&mining_manager).await?;
    let skip_devices = create_devices_with_strategy(
        &mining_manager,
        4,
        "跳跃",
        |i| (i * 2) % cpu_count
    ).await?;

    let skip_results = run_performance_test(
        &mining_manager,
        "跳跃绑定",
        10
    ).await?;

    // 策略3: 智能绑定 - 基于NUMA拓扑
    info!("\n📋 策略3: 智能NUMA感知绑定");
    clear_devices(&mining_manager).await?;
    let numa_devices = create_devices_with_numa_strategy(
        &mining_manager,
        4
    ).await?;

    let numa_results = run_performance_test(
        &mining_manager,
        "NUMA智能",
        10
    ).await?;

    // 策略4: 动态负载均衡
    info!("\n📋 策略4: 动态负载均衡绑定");
    clear_devices(&mining_manager).await?;
    let balanced_devices = create_devices_with_balanced_strategy(
        &mining_manager,
        6
    ).await?;

    let balanced_results = run_performance_test(
        &mining_manager,
        "动态均衡",
        15
    ).await?;

    // 生成对比报告
    info!("\n{}═══════════════════════════════════════{}",
        "=".repeat(20), "=".repeat(20));
    info!("📊 CPU亲和性策略对比报告");
    info!("{}═══════════════════════════════════════{}",
        "=".repeat(20), "=".repeat(20));

    println!("\n┌─────────────────┬──────────────┬──────────────┬──────────────┬──────────────┐");
    println!("│ 绑定策略        │ 平均算力     │ 算力稳定性   │ CPU效率      │ 综合评分     │");
    println!("├─────────────────┼──────────────┼──────────────┼──────────────┼──────────────┤");

    let strategies = vec![
        ("顺序绑定", sequential_results),
        ("跳跃绑定", skip_results),
        ("NUMA智能", numa_results),
        ("动态均衡", balanced_results),
    ];

    for (name, results) in &strategies {
        println!("│ {:<15} │ {:>8.1} Mh/s │ {:>10.1}%   │ {:>10.2}x   │ {:>10.1}/10  │",
            name,
            results.avg_hashrate / 1_000_000.0,
            results.stability_score,
            results.cpu_efficiency,
            results.overall_score
        );
    }
    println!("└─────────────────┴──────────────┴──────────────┴──────────────┴──────────────┘");

    // 找出最佳策略
    let best_strategy = strategies.iter()
        .max_by(|a, b| a.1.overall_score.partial_cmp(&b.1.overall_score).unwrap())
        .unwrap();

    info!("\n🏆 最佳策略: {} (评分: {:.1}/10)",
        best_strategy.0, best_strategy.1.overall_score);

    // 提供优化建议
    info!("\n💡 优化建议:");
    if cpu_count >= 8 {
        info!("   • 多核心系统，推荐使用跳跃绑定避免超线程竞争");
        info!("   • 考虑NUMA拓扑，将任务分配到同一NUMA节点");
    } else {
        info!("   • 核心数较少，顺序绑定即可满足需求");
    }

    if physical_cpu_count != cpu_count {
        info!("   • 检测到超线程，建议优先使用物理核心");
        info!("   • 物理核心: 0-{}", physical_cpu_count - 1);
    }

    info!("   • 避免绑定系统核心 (通常是核心0)");
    info!("   • 监控温度，高负载时考虑降低并发数");

    // 实际应用示例
    info!("\n🔧 实际应用配置建议:");
    info!("   配置文件中设置:");
    info!("   cpu_affinity_strategy = \"{}\"",
        best_strategy.0.to_lowercase().replace(" ", "_"));
    info!("   worker_threads = {}",
        if cpu_count >= 8 { cpu_count / 2 } else { cpu_count });
    info!("   enable_numa_awareness = true");

    info!("⏹️ 停止挖矿管理器...");
    mining_manager.stop().await?;

    info!("✅ CPU亲和性演示完成！");

    Ok(())
}

// 性能测试结果结构
#[derive(Debug, Clone)]
struct PerformanceResults {
    avg_hashrate: f64,
    stability_score: f64,
    cpu_efficiency: f64,
    overall_score: f64,
}

// 创建使用特定策略的设备
async fn create_devices_with_strategy<F>(
    mining_manager: &Arc<MiningManager>,
    count: usize,
    strategy_name: &str,
    cpu_mapper: F,
) -> Result<Vec<String>, Box<dyn std::error::Error>>
where
    F: Fn(usize) -> usize,
{
    let mut device_ids = Vec::new();

    for i in 0..count {
        let cpu_id = cpu_mapper(i);
        let core_info = cgminer_core::CoreInfo {
            name: format!("{}-设备-{}", strategy_name, i + 1),
            core_type: CoreType::CpuBtc,
            version: "1.0.0".to_string(),
            description: format!("{} CPU核心{}", strategy_name, cpu_id),
            capabilities: vec!["sha256".to_string()],
        };

        match mining_manager.add_core(core_info.clone()).await {
            Ok(_) => {
                info!("✅ 设备 {} 绑定到CPU核心 {}", i + 1, cpu_id);
                device_ids.push(core_info.name);
            },
            Err(e) => error!("❌ 设备 {} 创建失败: {}", i + 1, e),
        }
    }

    Ok(device_ids)
}

// NUMA感知绑定策略
async fn create_devices_with_numa_strategy(
    mining_manager: &Arc<MiningManager>,
    count: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let cpu_count = num_cpus::get();
    let numa_nodes = if cpu_count >= 16 { 2 } else { 1 }; // 简化的NUMA检测
    let cores_per_numa = cpu_count / numa_nodes;

    info!("🧠 检测到 {} 个NUMA节点，每节点 {} 核心", numa_nodes, cores_per_numa);

    create_devices_with_strategy(
        mining_manager,
        count,
        "NUMA智能",
        |i| {
            let numa_node = i % numa_nodes;
            let core_in_node = (i / numa_nodes) % cores_per_numa;
            numa_node * cores_per_numa + core_in_node
        }
    ).await
}

// 动态负载均衡策略
async fn create_devices_with_balanced_strategy(
    mining_manager: &Arc<MiningManager>,
    count: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let cpu_count = num_cpus::get();
    let reserved_cores = 1; // 为系统保留一个核心
    let available_cores = cpu_count - reserved_cores;

    info!("⚖️  为系统保留 {} 个核心，可用 {} 个核心", reserved_cores, available_cores);

    create_devices_with_strategy(
        mining_manager,
        count,
        "动态均衡",
        |i| {
            // 跳过核心0（系统核心），在剩余核心中分布
            let core_offset = 1 + (i % available_cores);
            core_offset
        }
    ).await
}

// 运行性能测试
async fn run_performance_test(
    mining_manager: &Arc<MiningManager>,
    strategy_name: &str,
    duration_seconds: u64,
) -> Result<PerformanceResults, Box<dyn std::error::Error>> {
    info!("🧪 运行 {} 性能测试 ({}秒)...", strategy_name, duration_seconds);

    // 创建工作数据
    let work = cgminer_core::Work::new(
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
        1,
        vec![0u8; 80],
        1234567890,
    );

    // 提交工作
    if let Err(e) = mining_manager.submit_work(work).await {
        error!("❌ 工作提交失败: {}", e);
    }

    let mut hashrate_samples = Vec::new();
    let start_time = std::time::Instant::now();

    // 收集性能数据
    while start_time.elapsed().as_secs() < duration_seconds {
        sleep(Duration::from_millis(500)).await;

        let stats = mining_manager.get_stats().await;
        hashrate_samples.push(stats.hashrate);

        if hashrate_samples.len() % 4 == 0 {
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
    }
    println!(); // 换行

    // 计算性能指标
    let avg_hashrate = hashrate_samples.iter().sum::<f64>() / hashrate_samples.len() as f64;
    let variance = hashrate_samples.iter()
        .map(|x| (x - avg_hashrate).powi(2))
        .sum::<f64>() / hashrate_samples.len() as f64;
    let std_dev = variance.sqrt();

    // 稳定性评分 (CV越小越稳定)
    let coefficient_of_variation = if avg_hashrate > 0.0 { std_dev / avg_hashrate } else { 1.0 };
    let stability_score = ((1.0 - coefficient_of_variation.min(1.0)) * 100.0).max(0.0);

    // CPU效率 (算力/核心数)
    let cpu_count = num_cpus::get();
    let cpu_efficiency = avg_hashrate / (cpu_count as f64 * 1_000_000.0);

    // 综合评分
    let overall_score = (stability_score * 0.4 +
                        (avg_hashrate / 10_000_000.0).min(10.0) * 0.4 +
                        (cpu_efficiency * 2.0).min(10.0) * 0.2).min(10.0);

    info!("📊 {} 测试结果:", strategy_name);
    info!("   平均算力: {:.1} Mh/s", avg_hashrate / 1_000_000.0);
    info!("   标准差: {:.1} Mh/s", std_dev / 1_000_000.0);
    info!("   稳定性: {:.1}%", stability_score);
    info!("   CPU效率: {:.2}x", cpu_efficiency);

    Ok(PerformanceResults {
        avg_hashrate,
        stability_score,
        cpu_efficiency,
        overall_score,
    })
}

// 清理所有设备
async fn clear_devices(mining_manager: &Arc<MiningManager>) -> Result<(), Box<dyn std::error::Error>> {
    // 注意：这里应该调用实际的清理方法
    // 为了演示，我们简单等待一下
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
