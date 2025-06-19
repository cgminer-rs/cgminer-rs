use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::time::sleep;
use tracing::info;

/// CPU优化器 - 监控和动态调整软核CPU使用
#[derive(Debug)]
pub struct CpuOptimizer {
    /// 系统信息
    system: System,
    /// 目标CPU使用率 (0.0-1.0)
    target_cpu_usage: f64,
    /// CPU使用率容忍范围
    cpu_tolerance: f64,
    /// 当前设备数量
    current_device_count: u32,
    /// 最小设备数量
    min_device_count: u32,
    /// 最大设备数量
    max_device_count: u32,
    /// 监控间隔
    monitor_interval: Duration,
    /// 调整间隔
    adjustment_interval: Duration,
    /// 上次调整时间
    last_adjustment: Instant,
}

impl CpuOptimizer {
    /// 创建新的CPU优化器
    pub fn new(
        target_cpu_usage: f64,
        min_device_count: u32,
        max_device_count: u32,
        initial_device_count: u32,
    ) -> Self {
        let mut system = System::new();
        system.refresh_cpu();

        Self {
            system,
            target_cpu_usage: target_cpu_usage.clamp(0.1, 0.95),
            cpu_tolerance: 0.05, // ±5%
            current_device_count: initial_device_count,
            min_device_count,
            max_device_count,
            monitor_interval: Duration::from_secs(30),
            adjustment_interval: Duration::from_secs(300), // 5分钟
            last_adjustment: Instant::now(),
        }
    }

    /// 启动CPU优化器
    pub async fn start(&mut self) {
        info!("🚀 启动CPU优化器");
        info!("目标CPU使用率: {:.1}%", self.target_cpu_usage * 100.0);
        info!("设备数量范围: {} - {}", self.min_device_count, self.max_device_count);
        info!("当前设备数量: {}", self.current_device_count);

        loop {
            // 刷新系统信息
            self.system.refresh_cpu();
            sleep(Duration::from_millis(100)).await; // 等待CPU信息更新

            // 获取当前CPU使用率
            let cpu_usage = self.get_average_cpu_usage();

            info!("💻 当前CPU使用率: {:.1}%", cpu_usage * 100.0);

            // 检查是否需要调整
            if self.should_adjust(cpu_usage) {
                if let Some(new_device_count) = self.calculate_optimal_device_count(cpu_usage) {
                    if new_device_count != self.current_device_count {
                        self.adjust_device_count(new_device_count).await;
                    }
                }
            }

            // 显示系统状态
            self.display_system_status(cpu_usage);

            sleep(self.monitor_interval).await;
        }
    }

    /// 获取平均CPU使用率
    fn get_average_cpu_usage(&self) -> f64 {
        let cpus = self.system.cpus();
        if cpus.is_empty() {
            return 0.0;
        }

        let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
        (total_usage / cpus.len() as f32) as f64 / 100.0
    }

    /// 检查是否应该调整
    fn should_adjust(&self, current_cpu_usage: f64) -> bool {
        // 检查调整间隔
        if self.last_adjustment.elapsed() < self.adjustment_interval {
            return false;
        }

        // 检查CPU使用率是否超出容忍范围
        let diff = (current_cpu_usage - self.target_cpu_usage).abs();
        diff > self.cpu_tolerance
    }

    /// 计算最优设备数量
    fn calculate_optimal_device_count(&self, current_cpu_usage: f64) -> Option<u32> {
        let usage_ratio = current_cpu_usage / self.target_cpu_usage;
        let new_device_count = ((self.current_device_count as f64) / usage_ratio).round() as u32;

        let clamped_count = new_device_count.clamp(self.min_device_count, self.max_device_count);

        if clamped_count != self.current_device_count {
            Some(clamped_count)
        } else {
            None
        }
    }

    /// 调整设备数量
    async fn adjust_device_count(&mut self, new_device_count: u32) {
        let old_count = self.current_device_count;
        let change = new_device_count as i32 - old_count as i32;

        if change > 0 {
            info!("📈 增加设备数量: {} -> {} (+{})", old_count, new_device_count, change);
        } else {
            info!("📉 减少设备数量: {} -> {} ({})", old_count, new_device_count, change);
        }

        // 这里应该调用CGMiner-RS的API来实际调整设备数量
        // 目前只是模拟调整
        self.simulate_device_adjustment(new_device_count).await;

        self.current_device_count = new_device_count;
        self.last_adjustment = Instant::now();
    }

    /// 模拟设备调整 (实际实现中应该调用CGMiner-RS API)
    async fn simulate_device_adjustment(&self, new_device_count: u32) {
        info!("🔧 正在调整设备配置...");

        // 模拟调整时间
        sleep(Duration::from_secs(2)).await;

        info!("✅ 设备配置调整完成，当前设备数量: {}", new_device_count);
    }

    /// 显示系统状态
    fn display_system_status(&self, cpu_usage: f64) {
        let cpu_count = self.system.cpus().len();
        let target_percent = self.target_cpu_usage * 100.0;
        let current_percent = cpu_usage * 100.0;

        println!("\n=== CPU优化器状态 ===");
        println!("🖥️  CPU核心数: {}", cpu_count);
        println!("🎯 目标CPU使用率: {:.1}%", target_percent);
        println!("📊 当前CPU使用率: {:.1}%", current_percent);
        println!("⚙️  当前设备数量: {}", self.current_device_count);
        println!("📈 设备数量范围: {} - {}", self.min_device_count, self.max_device_count);

        let status = if (cpu_usage - self.target_cpu_usage).abs() <= self.cpu_tolerance {
            "✅ 正常"
        } else if cpu_usage > self.target_cpu_usage {
            "⚠️  过高"
        } else {
            "📉 过低"
        };
        println!("🔍 CPU状态: {}", status);

        let next_check = self.monitor_interval.as_secs();
        println!("⏰ 下次检查: {}秒后", next_check);
        println!("==================\n");
    }

    /// 获取CPU使用建议
    pub fn get_cpu_usage_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let cpu_count = self.system.cpus().len();

        // 基于CPU核心数的建议
        if cpu_count >= 16 {
            recommendations.push("💡 检测到高核心数CPU，建议使用最大化CPU配置".to_string());
            recommendations.push(format!("   推荐设备数量: {}-{}", cpu_count * 2, cpu_count * 4));
            recommendations.push("   推荐策略: performance_first".to_string());
        } else if cpu_count >= 8 {
            recommendations.push("💡 检测到中等核心数CPU，建议使用平衡配置".to_string());
            recommendations.push(format!("   推荐设备数量: {}-{}", cpu_count, cpu_count * 2));
            recommendations.push("   推荐策略: intelligent".to_string());
        } else {
            recommendations.push("💡 检测到少核心数CPU，建议使用限制配置".to_string());
            recommendations.push(format!("   推荐设备数量: {}-{}", cpu_count / 2, cpu_count));
            recommendations.push("   推荐策略: round_robin".to_string());
        }

        // 基于当前使用率的建议
        let current_usage = self.get_average_cpu_usage();
        if current_usage > 0.9 {
            recommendations.push("⚠️  CPU使用率过高，建议减少设备数量或降低算力目标".to_string());
        } else if current_usage < 0.3 {
            recommendations.push("📈 CPU使用率较低，可以增加设备数量提高算力".to_string());
        }

        recommendations
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 CGMiner-RS CPU优化器");
    println!("========================");

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();

    let target_cpu = if args.len() > 1 {
        args[1].parse::<f64>().unwrap_or(0.7) / 100.0
    } else {
        0.7 // 默认70%
    };

    let min_devices = if args.len() > 2 {
        args[2].parse::<u32>().unwrap_or(4)
    } else {
        4
    };

    let max_devices = if args.len() > 3 {
        args[3].parse::<u32>().unwrap_or(32)
    } else {
        32
    };

    let initial_devices = if args.len() > 4 {
        args[4].parse::<u32>().unwrap_or(8)
    } else {
        8
    };

    println!("使用方法: {} [目标CPU使用率%] [最小设备数] [最大设备数] [初始设备数]", args[0]);
    println!("当前参数: 目标CPU={:.1}%, 设备范围={}-{}, 初始设备={}",
             target_cpu * 100.0, min_devices, max_devices, initial_devices);
    println!();

    // 创建并启动CPU优化器
    let mut optimizer = CpuOptimizer::new(target_cpu, min_devices, max_devices, initial_devices);

    // 显示建议
    let recommendations = optimizer.get_cpu_usage_recommendations();
    if !recommendations.is_empty() {
        println!("📋 配置建议:");
        for rec in recommendations {
            println!("   {}", rec);
        }
        println!();
    }

    // 启动监控
    optimizer.start().await;

    Ok(())
}
