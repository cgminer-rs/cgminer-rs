use cgminer_rs::{MiningManager, Config};
use cgminer_rs::monitoring::{MonitoringSystem, SystemMetrics};
use cgminer_core::{CoreRegistry, CoreType};
use cgminer_cpu_btc_core::CpuBtcCoreFactory;
use std::sync::Arc;
use tokio::time::{sleep, Duration, interval};
use tracing::{info, warn, error};
use sysinfo::{System, SystemExt, CpuExt, ProcessExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("📊 性能监控演示 - 实时系统监控");

    // 创建监控系统
    let monitoring = Arc::new(MonitoringSystem::new());
    monitoring.start().await?;
    info!("✅ 监控系统已启动");

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

    // 添加CPU核心
    let device_count = num_cpus::get().min(4); // 使用4个设备进行演示
    info!("💻 创建 {} 个CPU挖矿设备", device_count);

    for i in 0..device_count {
        let core_info = cgminer_core::CoreInfo {
            name: format!("CPU设备-{}", i + 1),
            core_type: CoreType::CpuBtc,
            version: "1.0.0".to_string(),
            description: format!("CPU挖矿设备 #{}", i + 1),
            capabilities: vec!["sha256".to_string()],
        };

        if let Err(e) = mining_manager.add_core(core_info).await {
            error!("❌ 设备 {} 添加失败: {}", i + 1, e);
        }
    }

    // 创建工作数据
    let work = cgminer_core::Work::new(
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
        1,
        vec![0u8; 80],
        1234567890,
    );

    // 提交工作到所有设备
    for _ in 0..device_count {
        if let Err(e) = mining_manager.submit_work(work.clone()).await {
            error!("❌ 工作提交失败: {}", e);
        }
    }

    info!("📈 开始性能监控演示...");
    info!("   监控项目: CPU使用率、内存使用、温度、算力、网络等");

    // 初始化系统信息
    let mut system = System::new_all();
    let mut monitoring_interval = interval(Duration::from_secs(2));

    // 性能历史记录
    let mut cpu_history = Vec::new();
    let mut memory_history = Vec::new();
    let mut hashrate_history = Vec::new();
    let mut temperature_history = Vec::new();

    let start_time = std::time::Instant::now();

    for iteration in 0..30 { // 运行60秒
        monitoring_interval.tick().await;
        system.refresh_all();

        // 收集系统指标
        let cpu_usage = system.global_cpu_info().cpu_usage();
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        // 收集挖矿统计
        let mining_stats = mining_manager.get_stats().await;
        let current_hashrate = mining_stats.hashrate;

        // 模拟温度数据
        let avg_temperature = 45.0 + (iteration as f64 * 0.5) + (cpu_usage as f64 * 0.2);

        // 记录历史数据
        cpu_history.push(cpu_usage);
        memory_history.push(memory_usage_percent);
        hashrate_history.push(current_hashrate);
        temperature_history.push(avg_temperature);

        // 保持历史记录在合理范围内
        if cpu_history.len() > 60 {
            cpu_history.remove(0);
            memory_history.remove(0);
            hashrate_history.remove(0);
            temperature_history.remove(0);
        }

        // 计算平均值和趋势
        let avg_cpu = cpu_history.iter().sum::<f32>() / cpu_history.len() as f32;
        let avg_memory = memory_history.iter().sum::<f64>() / memory_history.len() as f64;
        let avg_hashrate = hashrate_history.iter().sum::<f64>() / hashrate_history.len() as f64;
        let avg_temperature = temperature_history.iter().sum::<f64>() / temperature_history.len() as f64;

        // 计算趋势（简单的上升/下降检测）
        let hashrate_trend = if hashrate_history.len() >= 5 {
            let recent_avg = hashrate_history[hashrate_history.len()-3..].iter().sum::<f64>() / 3.0;
            let older_avg = hashrate_history[hashrate_history.len()-6..hashrate_history.len()-3].iter().sum::<f64>() / 3.0;
            if recent_avg > older_avg * 1.05 { "↗️ 上升" }
            else if recent_avg < older_avg * 0.95 { "↘️ 下降" }
            else { "→ 稳定" }
        } else { "→ 稳定" };

        // 显示实时监控数据
        if iteration % 3 == 0 || iteration < 5 {
            println!("\n{}═════════════════════════════════════════════════════════════{}",
                "=".repeat(10), "=".repeat(10));
            println!("⏱️  运行时间: {:02}:{:02} | 监控周期: #{}",
                start_time.elapsed().as_secs() / 60,
                start_time.elapsed().as_secs() % 60,
                iteration + 1
            );
            println!("{}─────────────────────────────────────────────────────────────{}",
                "-".repeat(10), "-".repeat(10));

            // 系统资源
            println!("🖥️  系统资源:");
            println!("   CPU使用率:  {: >6.1}% (平均: {:.1}%) {}",
                cpu_usage, avg_cpu,
                format_bar(cpu_usage as f64, 100.0, 20)
            );
            println!("   内存使用:   {: >6.1}% (平均: {:.1}%) {}",
                memory_usage_percent, avg_memory,
                format_bar(memory_usage_percent, 100.0, 20)
            );
            println!("   内存详情:   {:.1} GB / {:.1} GB",
                used_memory as f64 / 1_073_741_824.0,
                total_memory as f64 / 1_073_741_824.0
            );

            // 挖矿性能
            println!("⛏️  挖矿性能:");
            println!("   当前算力:   {: >6.1} Mh/s (平均: {:.1} Mh/s) {}",
                current_hashrate / 1_000_000.0,
                avg_hashrate / 1_000_000.0,
                hashrate_trend
            );
            println!("   设备数量:   {} 个活跃设备", device_count);
            println!("   单设备算力: {:.1} Mh/s",
                current_hashrate / (device_count as f64 * 1_000_000.0)
            );

            // 温度监控
            println!("🌡️  温度监控:");
            println!("   平均温度:   {: >6.1}°C (平均: {:.1}°C) {}",
                avg_temperature, avg_temperature,
                if avg_temperature > 70.0 { "🔥" }
                else if avg_temperature > 60.0 { "⚠️" }
                else { "✅" }
            );

            // 性能效率
            let efficiency = if cpu_usage > 0.0 {
                (current_hashrate / 1_000_000.0) / cpu_usage as f64
            } else { 0.0 };
            println!("📊 性能效率:  {:.2} Mh/s/CPU% (算力/CPU使用率)", efficiency);

            // 网络状态（模拟）
            let network_latency = 25 + (iteration % 10) as u32;
            println!("🌐 网络状态:  延迟 {}ms, 连接稳定", network_latency);
        }

        // 性能警告检查
        if cpu_usage > 95.0 {
            warn!("⚠️  CPU使用率过高: {:.1}%", cpu_usage);
        }
        if memory_usage_percent > 90.0 {
            warn!("⚠️  内存使用率过高: {:.1}%", memory_usage_percent);
        }
        if avg_temperature > 75.0 {
            warn!("⚠️  温度过高: {:.1}°C", avg_temperature);
        }
        if current_hashrate < avg_hashrate * 0.8 && iteration > 10 {
            warn!("⚠️  算力显著下降: 当前 {:.1} Mh/s, 平均 {:.1} Mh/s",
                current_hashrate / 1_000_000.0, avg_hashrate / 1_000_000.0);
        }

        // 性能提示
        if iteration == 15 {
            info!("💡 性能提示: CPU使用率 {:.1}%, 可考虑调整并发数", avg_cpu);
        }
        if iteration == 25 {
            info!("💡 性能建议: 平均算力 {:.1} Mh/s, 系统运行稳定", avg_hashrate / 1_000_000.0);
        }
    }

    info!("⏹️ 停止挖矿管理器...");
    mining_manager.stop().await?;

    info!("🛑 停止监控系统...");
    monitoring.stop().await?;

    // 生成最终性能报告
    println!("\n{}═════════════════════════════════════════════════════════════{}",
        "=".repeat(15), "=".repeat(15));
    println!("📋 最终性能报告");
    println!("{}═════════════════════════════════════════════════════════════{}",
        "=".repeat(15), "=".repeat(15));

    let total_time = start_time.elapsed().as_secs();
    let max_cpu = cpu_history.iter().fold(0.0f32, |a, &b| a.max(b));
    let min_cpu = cpu_history.iter().fold(100.0f32, |a, &b| a.min(b));
    let max_hashrate = hashrate_history.iter().fold(0.0f64, |a, &b| a.max(b));
    let min_hashrate = hashrate_history.iter().fold(f64::INFINITY, |a, &b| a.min(b));

    println!("⏱️  运行时间: {}秒", total_time);
    println!("🖥️  CPU使用率: 平均 {:.1}%, 最高 {:.1}%, 最低 {:.1}%",
        avg_cpu, max_cpu, min_cpu);
    println!("💾 内存使用率: 平均 {:.1}%", avg_memory);
    println!("⛏️  算力范围: {:.1} - {:.1} Mh/s (平均 {:.1} Mh/s)",
        min_hashrate / 1_000_000.0, max_hashrate / 1_000_000.0, avg_hashrate / 1_000_000.0);
    println!("🌡️  平均温度: {:.1}°C", avg_temperature);
    println!("📊 总体效率: {:.2} Mh/s/CPU%",
        (avg_hashrate / 1_000_000.0) / avg_cpu as f64);

    info!("✅ 性能监控演示完成！");

    Ok(())
}

// 辅助函数：生成进度条
fn format_bar(current: f64, max: f64, width: usize) -> String {
    let percentage = (current / max).min(1.0);
    let filled = (percentage * width as f64) as usize;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
