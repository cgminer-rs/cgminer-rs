//! 挖矿专用日志记录器

use crate::logging::{formatter, hashmeter::HashMeter};
use std::time::{Duration, Instant, SystemTime};
use tracing::{info, warn, error, debug};

/// 挖矿日志记录器
pub struct MiningLogger {
    /// 算力计量器
    hashmeter: HashMeter,
    /// 上次状态报告时间
    last_status_report: Instant,
    /// 状态报告间隔
    status_report_interval: Duration,
    /// 启动时间
    start_time: SystemTime,
    /// 是否启用详细日志
    verbose: bool,
}

impl MiningLogger {
    /// 创建新的挖矿日志记录器
    pub fn new(verbose: bool) -> Self {
        Self {
            hashmeter: HashMeter::new(Duration::from_secs(30), true),
            last_status_report: Instant::now(),
            status_report_interval: Duration::from_secs(300), // 5分钟
            start_time: SystemTime::now(),
            verbose,
        }
    }

    /// 记录挖矿启动
    pub fn log_mining_start(&self, total_devices: usize, total_pools: usize) {
        let separator = formatter::create_separator("CGMiner-RS 启动", 80);
        info!("{}", separator);
        info!("🚀 CGMiner-RS 挖矿程序启动");
        info!("📅 启动时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        info!("🔧 设备数量: {}", total_devices);
        info!("🌊 矿池数量: {}", total_pools);
        info!("╚{}╝", "═".repeat(78));
    }

    /// 记录挖矿停止
    pub fn log_mining_stop(&self) {
        let uptime = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or(Duration::from_secs(0));

        let separator = formatter::create_separator("CGMiner-RS 停止", 80);
        info!("{}", separator);
        info!("🛑 CGMiner-RS 挖矿程序停止");
        info!("📅 停止时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        info!("⏱️ 运行时间: {}", formatter::format_duration(uptime));

        // 显示最终统计
        let stats = self.hashmeter.get_stats(None);
        info!("📊 最终统计:");
        info!("   ⛏️ 平均算力: {}", formatter::format_hashrate(stats.average));
        info!("   📈 最大算力: {}", formatter::format_hashrate(stats.max));
        info!("   📉 最小算力: {}", formatter::format_hashrate(stats.min));
        info!("   🔢 总样本数: {}", stats.samples);
        info!("╚{}╝", "═".repeat(78));
    }

    /// 记录设备状态
    pub fn log_device_status(&mut self, device_id: u32, online: bool, temperature: f32, hashrate: f64, power: f64) {
        let status_icon = if online { "🟢" } else { "🔴" };
        let status_text = if online { "在线" } else { "离线" };

        if self.verbose || !online {
            info!("🔧 设备 {} {} | {} | {} | {}",
                  device_id,
                  format!("{} {}", status_icon, status_text),
                  formatter::format_temperature(temperature),
                  formatter::format_hashrate(hashrate),
                  formatter::format_power(power));
        }

        // 记录算力
        if online && hashrate > 0.0 {
            self.hashmeter.record_hashrate(hashrate, Some(device_id), None);
        }
    }

    /// 记录矿池状态
    pub fn log_pool_status(&self, pool_id: u32, connected: bool, url: &str, ping: u32, accepted_shares: u64, rejected_shares: u64) {
        let status_icon = if connected { "🟢" } else { "🔴" };
        let status_text = if connected { "已连接" } else { "未连接" };

        if self.verbose || !connected {
            info!("🌊 矿池 {} {} | {} | {} | ✅ {} | ❌ {}",
                  pool_id,
                  format!("{} {}", status_icon, status_text),
                  url,
                  formatter::format_latency(ping),
                  accepted_shares,
                  rejected_shares);
        }
    }

    /// 记录份额提交
    pub fn log_share_submission(&self, device_id: u32, pool_id: u32, accepted: bool, difficulty: f64) {
        let result_icon = if accepted { "✅" } else { "❌" };
        let result_text = if accepted { "接受" } else { "拒绝" };

        if self.verbose {
            info!("📤 设备 {} -> 矿池 {} | {} {} | 难度: {:.2}",
                  device_id, pool_id, result_icon, result_text, difficulty);
        } else if !accepted {
            warn!("📤 设备 {} -> 矿池 {} | {} {} | 难度: {:.2}",
                  device_id, pool_id, result_icon, result_text, difficulty);
        }
    }

    /// 记录系统状态
    pub fn log_system_status(&self, cpu_usage: f64, memory_usage: f64, temperature: f32, power: f64) {
        if self.should_report_status() {
            info!("🖥️ 系统状态 | CPU: {} | 内存: {} | {} | {}",
                  formatter::format_percentage(cpu_usage),
                  formatter::format_percentage(memory_usage),
                  formatter::format_temperature(temperature),
                  formatter::format_power(power));

            self.update_last_status_report();
        }
    }

    /// 记录错误
    pub fn log_error(&self, component: &str, device_id: Option<u32>, message: &str) {
        if let Some(id) = device_id {
            error!("❌ {} 设备 {} 错误: {}", component, id, message);
        } else {
            error!("❌ {} 错误: {}", component, message);
        }
    }

    /// 记录警告
    pub fn log_warning(&self, component: &str, device_id: Option<u32>, message: &str) {
        if let Some(id) = device_id {
            warn!("⚠️ {} 设备 {} 警告: {}", component, id, message);
        } else {
            warn!("⚠️ {} 警告: {}", component, message);
        }
    }

    /// 记录调试信息
    pub fn log_debug(&self, component: &str, message: &str) {
        if self.verbose {
            debug!("🔍 {} 调试: {}", component, message);
        }
    }

    /// 记录性能统计
    pub fn log_performance_stats(&self) {
        let stats_5min = self.hashmeter.get_stats(Some(Duration::from_secs(300)));
        let stats_1hour = self.hashmeter.get_stats(Some(Duration::from_secs(3600)));

        let separator = formatter::create_separator("性能统计", 80);
        info!("{}", separator);

        info!("📊 5分钟统计:");
        info!("   ⛏️ 当前算力: {}", formatter::format_hashrate(stats_5min.current));
        info!("   📈 平均算力: {}", formatter::format_hashrate(stats_5min.average));
        info!("   📊 样本数: {}", stats_5min.samples);

        info!("📊 1小时统计:");
        info!("   📈 平均算力: {}", formatter::format_hashrate(stats_1hour.average));
        info!("   🔝 最大算力: {}", formatter::format_hashrate(stats_1hour.max));
        info!("   📉 最小算力: {}", formatter::format_hashrate(stats_1hour.min));
        info!("   📊 样本数: {}", stats_1hour.samples);

        info!("╚{}╝", "═".repeat(78));
    }

    /// 记录设备详细信息
    pub fn log_device_details(&self, device_id: u32, chip_count: u32, frequency: u32, voltage: u32, fan_speed: u32) {
        if self.verbose {
            info!("🔧 设备 {} 详细信息:", device_id);
            info!("   🔩 芯片数量: {}", chip_count);
            info!("   ⚡ 频率: {} MHz", frequency);
            info!("   🔋 电压: {} mV", voltage);
            info!("   🌀 风扇速度: {}%", fan_speed);
        }
    }

    /// 记录网络状态
    pub fn log_network_status(&self, rx_bytes: u64, tx_bytes: u64, connections: usize) {
        if self.verbose {
            info!("🌐 网络状态 | 接收: {} | 发送: {} | 连接数: {}",
                  formatter::format_network_traffic(rx_bytes),
                  formatter::format_network_traffic(tx_bytes),
                  connections);
        }
    }

    /// 记录算力趋势
    pub fn log_hashrate_trend(&self) {
        // 这里可以添加算力趋势图的显示
        if self.verbose {
            let stats = self.hashmeter.get_stats(Some(Duration::from_secs(3600)));
            info!("📈 算力趋势 (1小时): 平均 {} | 当前 {}",
                  formatter::format_hashrate(stats.average),
                  formatter::format_hashrate(stats.current));
        }
    }

    /// 检查是否应该报告状态
    fn should_report_status(&self) -> bool {
        self.last_status_report.elapsed() >= self.status_report_interval
    }

    /// 更新最后状态报告时间
    fn update_last_status_report(&self) {
        // 注意：这里需要使用内部可变性，但为了简化示例，我们暂时忽略
        // 在实际实现中，应该使用 RefCell 或 Mutex
    }

    /// 设置详细模式
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
        info!("🔧 详细日志模式: {}", if verbose { "启用" } else { "禁用" });
    }

    /// 设置状态报告间隔
    pub fn set_status_report_interval(&mut self, interval: Duration) {
        self.status_report_interval = interval;
        info!("🔧 状态报告间隔设置为: {:?}", interval);
    }

    /// 获取运行时间
    pub fn get_uptime(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or(Duration::from_secs(0))
    }

    /// 获取算力统计
    pub fn get_hashrate_stats(&self, duration: Option<Duration>) -> crate::logging::hashmeter::HashRateStats {
        self.hashmeter.get_stats(duration)
    }
}
