//! 简化监控系统使用示例
//! 
//! 展示如何使用轻量级的内置监控系统替代复杂的Prometheus

use cgminer_rs::monitoring::{SimpleWebMonitor, SystemMetrics, MiningMetrics, DeviceMetrics, PoolMetrics};
use cgminer_rs::config::{MonitoringConfig, AlertThresholds};
use cgminer_rs::monitoring::MonitoringSystem;
use std::time::{SystemTime, Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();

    println!("🌐 简化监控系统演示");
    println!("===========================================");

    // 1. 创建简化的监控配置
    let monitoring_config = MonitoringConfig {
        enabled: true,
        metrics_interval: 5, // 5秒收集一次指标
        web_port: Some(8888), // Web界面端口
        alert_thresholds: AlertThresholds {
            temperature_warning: 80.0,
            temperature_critical: 90.0,
            hashrate_drop_percent: 20.0,
            error_rate_percent: 5.0,
            max_temperature: 85.0,
            max_cpu_usage: 80.0,
            max_memory_usage: 90.0,
            max_device_temperature: 85.0,
            max_error_rate: 5.0,
            min_hashrate: 50.0,
        },
    };

    // 2. 创建监控系统
    let monitoring_system = MonitoringSystem::new(monitoring_config).await?;

    // 3. 启动监控系统
    monitoring_system.start().await?;
    println!("✅ 监控系统已启动");

    // 4. 创建独立的Web监控器演示
    println!("\n📊 启动独立Web监控器演示:");
    let mut web_monitor = SimpleWebMonitor::new(9999);
    web_monitor.start().await?;

    // 5. 模拟数据更新
    println!("📈 开始模拟挖矿数据...");
    
    for i in 0..10 {
        // 模拟系统指标
        let system_metrics = SystemMetrics {
            timestamp: SystemTime::now(),
            cpu_usage: 45.0 + (i as f64 * 2.0),
            memory_usage: 60.0 + (i as f64 * 1.5),
            disk_usage: 30.0,
            network_rx: 1024 * 1024 * (i + 1) as u64,
            network_tx: 512 * 1024 * (i + 1) as u64,
            temperature: 65.0 + (i as f32 * 0.5),
            fan_speed: 2000 + (i * 50) as u32,
            power_consumption: 150.0 + (i as f64 * 5.0),
            uptime: Duration::from_secs((i + 1) * 3600),
        };

        // 模拟挖矿指标
        let mining_metrics = MiningMetrics {
            timestamp: SystemTime::now(),
            total_hashrate: 100.0 + (i as f64 * 2.0),
            accepted_shares: (i + 1) as u64 * 10,
            rejected_shares: i as u64,
            hardware_errors: if i > 5 { i as u64 - 5 } else { 0 },
            stale_shares: i as u64 / 2,
            best_share: 1000000.0 + (i as f64 * 50000.0),
            current_difficulty: 1000.0,
            network_difficulty: 50000000000000.0,
            blocks_found: if i > 8 { 1 } else { 0 },
            efficiency: 0.5 + (i as f64 * 0.01),
            active_devices: 2,
            connected_pools: 1,
        };

        // 模拟设备指标
        for device_id in 0..2 {
            let device_metrics = DeviceMetrics {
                device_id,
                timestamp: SystemTime::now(),
                temperature: 70.0 + (device_id as f32 * 2.0) + (i as f32 * 0.3),
                hashrate: 50.0 + (device_id as f64 * 5.0) + (i as f64 * 1.0),
                power_consumption: 75.0 + (device_id as f64 * 10.0),
                fan_speed: 2500 + (device_id * 100) as u32,
                voltage: 12000 + (device_id * 100) as u32,
                frequency: 600 + (device_id * 50) as u32,
                error_rate: if i > 6 { (i - 6) as f64 * 0.5 } else { 0.0 },
                uptime: Duration::from_secs((i + 1) * 1800),
                accepted_shares: (i + 1) as u64 * 5 * (device_id as u64 + 1),
                rejected_shares: i as u64 / 2,
                hardware_errors: if i > 7 { i as u64 - 7 } else { 0 },
            };

            web_monitor.update_device_metrics(device_id, device_metrics).await;
        }

        // 模拟矿池指标
        let pool_metrics = PoolMetrics {
            pool_id: 0,
            timestamp: SystemTime::now(),
            connected: true,
            ping: Some(Duration::from_millis(50 + (i * 5) as u64)),
            accepted_shares: (i + 1) as u64 * 8,
            rejected_shares: i as u64 / 3,
            stale_shares: i as u64 / 4,
            difficulty: 1000.0 + (i as f64 * 10.0),
            last_share_time: Some(SystemTime::now()),
            connection_uptime: Duration::from_secs((i + 1) * 600),
        };

        // 更新Web监控器
        web_monitor.update_system_metrics(system_metrics).await;
        web_monitor.update_mining_metrics(mining_metrics).await;
        web_monitor.update_pool_metrics(0, pool_metrics).await;

        // 显示当前状态摘要
        if i % 3 == 0 {
            let summary = web_monitor.get_status_summary().await;
            println!("\n{}", summary);
        }

        // 等待一段时间
        sleep(Duration::from_secs(2)).await;
    }

    println!("\n🎉 监控演示完成！");
    println!("===========================================");
    println!("📱 访问监控界面:");
    println!("   主监控系统: http://localhost:8888");
    println!("   演示监控器: http://localhost:9999");
    println!("");
    println!("💡 简化监控系统的优势:");
    println!("   ✅ 无需外部Prometheus服务器");
    println!("   ✅ 内置美观的Web界面");
    println!("   ✅ 实时数据更新");
    println!("   ✅ 轻量级资源占用");
    println!("   ✅ 专为个人挖矿设计");
    println!("   ✅ 简单易用的配置");
    println!("");
    println!("🔧 功能特性:");
    println!("   📊 实时算力监控");
    println!("   🌡️  温度和功耗监控");
    println!("   📈 份额统计和趋势");
    println!("   🔧 设备状态监控");
    println!("   🏊 矿池连接状态");
    println!("   ⚠️  简单的告警提示");

    // 保持程序运行，让用户可以访问Web界面
    println!("\n⏳ 按 Ctrl+C 退出程序...");
    
    // 等待用户中断
    tokio::signal::ctrl_c().await?;
    
    println!("\n🛑 正在停止监控系统...");
    
    // 停止监控系统
    web_monitor.stop().await?;
    monitoring_system.stop().await?;
    
    println!("✅ 监控系统已停止");
    
    Ok(())
}

/// 演示命令行状态显示
async fn demo_console_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📟 命令行监控演示:");
    println!("===================");

    let mut web_monitor = SimpleWebMonitor::new(0); // 端口0表示不启动Web服务器
    web_monitor.set_enabled(true);

    // 模拟一些数据
    let system_metrics = SystemMetrics {
        timestamp: SystemTime::now(),
        cpu_usage: 55.2,
        memory_usage: 68.5,
        temperature: 72.3,
        power_consumption: 165.8,
        uptime: Duration::from_secs(7200),
        ..Default::default()
    };

    let mining_metrics = MiningMetrics {
        timestamp: SystemTime::now(),
        total_hashrate: 105.6,
        accepted_shares: 1250,
        rejected_shares: 15,
        active_devices: 2,
        efficiency: 0.64,
        ..Default::default()
    };

    web_monitor.update_system_metrics(system_metrics).await;
    web_monitor.update_mining_metrics(mining_metrics).await;

    // 显示状态摘要
    let summary = web_monitor.get_status_summary().await;
    println!("{}", summary);

    Ok(())
}

/// 演示配置简化
fn demo_config_comparison() {
    println!("\n⚖️  配置对比:");
    println!("=============");
    
    println!("❌ 复杂的Prometheus配置:");
    println!("   - prometheus.yml 配置文件");
    println!("   - alert_rules.yml 告警规则");
    println!("   - docker-compose.yml 容器编排");
    println!("   - Grafana 仪表板配置");
    println!("   - 复杂的PromQL查询语言");
    println!("   - 额外的资源消耗");
    
    println!("\n✅ 简化的Web监控配置:");
    println!("   [monitoring]");
    println!("   enabled = true");
    println!("   web_port = 8888");
    println!("   metrics_interval = 30");
    println!("");
    println!("   就这么简单！🎉");
}
