use crate::error::MiningError;
use crate::monitoring::{MiningMetrics, DeviceMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::time::interval;
use tracing::info;

/// 算力计量器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashmeterConfig {
    /// 是否启用算力计量器
    pub enabled: bool,
    /// 日志输出间隔 (秒)
    pub log_interval: u64,
    /// 是否启用设备级别统计
    pub per_device_stats: bool,
    /// 是否启用控制台输出
    pub console_output: bool,
    /// 是否启用美化输出
    pub beautiful_output: bool,
    /// 算力单位 (H, KH, MH, GH, TH)
    pub hashrate_unit: String,
}

impl Default for HashmeterConfig {
    fn default() -> Self {
        Self {
            enabled: true, // 默认启用
            log_interval: 30, // 30秒间隔，类似传统cgminer
            per_device_stats: true,
            console_output: true,
            beautiful_output: true,
            hashrate_unit: "GH".to_string(),
        }
    }
}

/// 算力统计数据
#[derive(Debug, Clone)]
pub struct HashrateStats {
    /// 当前算力 (H/s)
    pub current_hashrate: f64,
    /// 1分钟平均算力
    pub avg_1m: f64,
    /// 5分钟平均算力
    pub avg_5m: f64,
    /// 15分钟平均算力
    pub avg_15m: f64,
    /// 总平均算力
    pub avg_total: f64,
    /// 接受的份额
    pub accepted_shares: u64,
    /// 拒绝的份额
    pub rejected_shares: u64,
    /// 硬件错误
    pub hardware_errors: u64,
    /// 工作单元/分钟
    pub work_utility: f64,
    /// 运行时间
    pub uptime: Duration,
}

/// 设备算力统计
#[derive(Debug, Clone)]
pub struct DeviceHashrateStats {
    pub device_id: u32,
    pub device_name: String,
    pub stats: HashrateStats,
    pub temperature: f32,
    pub fan_speed: u32,
}

/// 算力计量器
pub struct Hashmeter {
    config: HashmeterConfig,
    start_time: Instant,
    last_log_time: Arc<RwLock<Instant>>,
    total_stats: Arc<RwLock<HashrateStats>>,
    device_stats: Arc<RwLock<HashMap<u32, DeviceHashrateStats>>>,
    running: Arc<RwLock<bool>>,
    handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl Hashmeter {
    /// 创建新的算力计量器
    pub fn new(config: HashmeterConfig) -> Self {
        let start_time = Instant::now();

        Self {
            config,
            start_time,
            last_log_time: Arc::new(RwLock::new(start_time)),
            total_stats: Arc::new(RwLock::new(HashrateStats {
                current_hashrate: 0.0,
                avg_1m: 0.0,
                avg_5m: 0.0,
                avg_15m: 0.0,
                avg_total: 0.0,
                accepted_shares: 0,
                rejected_shares: 0,
                hardware_errors: 0,
                work_utility: 0.0,
                uptime: Duration::from_secs(0),
            })),
            device_stats: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动算力计量器
    pub async fn start(&self) -> Result<(), MiningError> {
        *self.running.write().await = true;

        let running = self.running.clone();
        let config = self.config.clone();
        let last_log_time = self.last_log_time.clone();
        let total_stats = self.total_stats.clone();
        let device_stats = self.device_stats.clone();
        let start_time = self.start_time;

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.log_interval));

            while *running.read().await {
                interval.tick().await;

                // 更新运行时间
                let uptime = start_time.elapsed();
                {
                    let mut stats = total_stats.write().await;
                    stats.uptime = uptime;
                }

                // 输出算力信息
                Self::output_hashrate_info(&config, &total_stats, &device_stats).await;

                // 更新最后日志时间
                *last_log_time.write().await = Instant::now();
            }
        });

        *self.handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止算力计量器
    pub async fn stop(&self) -> Result<(), MiningError> {
        *self.running.write().await = false;

        if let Some(handle) = self.handle.lock().await.take() {
            handle.abort();
        }

        Ok(())
    }

    /// 更新总体统计信息
    pub async fn update_total_stats(&self, mining_metrics: &MiningMetrics) -> Result<(), MiningError> {
        let mut stats = self.total_stats.write().await;

        stats.current_hashrate = mining_metrics.total_hashrate;
        stats.accepted_shares = mining_metrics.accepted_shares;
        stats.rejected_shares = mining_metrics.rejected_shares;
        stats.hardware_errors = mining_metrics.hardware_errors;

        // 计算工作单元/分钟
        let total_shares = stats.accepted_shares + stats.rejected_shares;
        let uptime_minutes = stats.uptime.as_secs_f64() / 60.0;
        stats.work_utility = if uptime_minutes > 0.0 {
            total_shares as f64 / uptime_minutes
        } else {
            0.0
        };

        Ok(())
    }

    /// 更新设备统计信息
    pub async fn update_device_stats(&self, device_metrics: &DeviceMetrics) -> Result<(), MiningError> {
        let mut device_stats = self.device_stats.write().await;

        let device_stat = DeviceHashrateStats {
            device_id: device_metrics.device_id,
            device_name: format!("Device {}", device_metrics.device_id),
            stats: HashrateStats {
                current_hashrate: device_metrics.hashrate,
                avg_1m: device_metrics.hashrate, // 简化实现
                avg_5m: device_metrics.hashrate,
                avg_15m: device_metrics.hashrate,
                avg_total: device_metrics.hashrate,
                accepted_shares: device_metrics.accepted_shares,
                rejected_shares: device_metrics.rejected_shares,
                hardware_errors: device_metrics.hardware_errors,
                work_utility: 0.0,
                uptime: device_metrics.uptime,
            },
            temperature: device_metrics.temperature,
            fan_speed: device_metrics.fan_speed,
        };

        device_stats.insert(device_metrics.device_id, device_stat);
        Ok(())
    }

    /// 输出算力信息
    async fn output_hashrate_info(
        config: &HashmeterConfig,
        total_stats: &Arc<RwLock<HashrateStats>>,
        device_stats: &Arc<RwLock<HashMap<u32, DeviceHashrateStats>>>,
    ) {
        let stats = total_stats.read().await;
        let devices = device_stats.read().await;

        if config.beautiful_output {
            Self::output_beautiful_format(&stats, &devices, config).await;
        } else {
            Self::output_traditional_format(&stats, &devices, config).await;
        }
    }

    /// 美化格式输出 (CGMiner-RS风格)
    async fn output_beautiful_format(
        stats: &HashrateStats,
        devices: &HashMap<u32, DeviceHashrateStats>,
        config: &HashmeterConfig,
    ) {
        let hashrate_display = Self::format_hashrate(stats.current_hashrate, &config.hashrate_unit);
        let uptime_display = Self::format_uptime(stats.uptime);
        let reject_rate = Self::calculate_reject_rate(stats.accepted_shares, stats.rejected_shares);

        info!("⚡ Mining Status Update:");
        info!("   📈 Hashrate: {}", hashrate_display);
        info!("   🎯 Shares: {} accepted, {} rejected ({:.2}% reject rate)",
              stats.accepted_shares, stats.rejected_shares, reject_rate);
        info!("   ⚠️  Hardware Errors: {}", stats.hardware_errors);
        info!("   🔧 Work Utility: {:.2}/min", stats.work_utility);
        info!("   ⏱️  Uptime: {}", uptime_display);

        if config.per_device_stats && !devices.is_empty() {
            info!("   📊 Device Details:");
            for device in devices.values() {
                let device_hashrate = Self::format_hashrate(device.stats.current_hashrate, &config.hashrate_unit);
                info!("      • {}: {} | Temp: {:.1}°C | Fan: {}%",
                      device.device_name, device_hashrate, device.temperature, device.fan_speed);
            }
        }
    }

    /// 传统格式输出 (类似原版cgminer)
    async fn output_traditional_format(
        stats: &HashrateStats,
        devices: &HashMap<u32, DeviceHashrateStats>,
        config: &HashmeterConfig,
    ) {
        let hashrate_display = Self::format_hashrate(stats.current_hashrate, &config.hashrate_unit);
        let uptime_display = Self::format_uptime(stats.uptime);

        // 类似cgminer的状态行格式
        info!("({}s):{} (avg):{} | A:{} R:{} HW:{} WU:{:.1}/m | {}",
              config.log_interval,
              hashrate_display,
              hashrate_display, // 简化实现，使用当前算力作为平均算力
              stats.accepted_shares,
              stats.rejected_shares,
              stats.hardware_errors,
              stats.work_utility,
              uptime_display
        );

        if config.per_device_stats {
            for device in devices.values() {
                let device_hashrate = Self::format_hashrate(device.stats.current_hashrate, &config.hashrate_unit);
                info!("{} {}: {} | A:{} R:{} HW:{} | {:.1}°C",
                      device.device_name,
                      device.device_id,
                      device_hashrate,
                      device.stats.accepted_shares,
                      device.stats.rejected_shares,
                      device.stats.hardware_errors,
                      device.temperature
                );
            }
        }
    }

    /// 格式化算力显示
    fn format_hashrate(hashrate: f64, unit: &str) -> String {
        match unit {
            "H" => format!("{:.2} H/s", hashrate),
            "KH" => format!("{:.2} KH/s", hashrate / 1_000.0),
            "MH" => format!("{:.2} MH/s", hashrate / 1_000_000.0),
            "GH" => format!("{:.2} GH/s", hashrate / 1_000_000_000.0),
            "TH" => format!("{:.2} TH/s", hashrate / 1_000_000_000_000.0),
            _ => format!("{:.2} GH/s", hashrate / 1_000_000_000.0),
        }
    }

    /// 格式化运行时间
    fn format_uptime(uptime: Duration) -> String {
        let total_seconds = uptime.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    /// 计算拒绝率
    fn calculate_reject_rate(accepted: u64, rejected: u64) -> f64 {
        let total = accepted + rejected;
        if total > 0 {
            (rejected as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}
