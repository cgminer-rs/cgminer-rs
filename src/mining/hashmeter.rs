use crate::error::MiningError;
use crate::monitoring::MiningMetrics;
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
    /// 算力单位 (自动适应，无需配置)
    #[serde(skip)]
    pub hashrate_unit: String,
}

impl Default for HashmeterConfig {
    fn default() -> Self {
        Self {
            enabled: true, // 默认启用
            log_interval: 5, // 5秒间隔，更频繁的统计
            per_device_stats: true,
            console_output: true,
            hashrate_unit: "AUTO".to_string(),
        }
    }
}

/// 算力统计数据
#[derive(Debug, Clone)]
pub struct HashrateStats {
    /// 当前算力 (H/s) - 用于5秒统计
    pub current_hashrate: f64,
    /// 5秒平均算力
    pub avg_5s: f64,
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
                avg_5s: 0.0,
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
        stats.avg_5s = mining_metrics.total_hashrate; // 使用当前算力作为5秒算力

        // 计算滑动窗口算力（简单的指数移动平均）
        let alpha_1m = 0.1;
        let alpha_5m = 0.02;
        let alpha_15m = 0.007;

        if stats.avg_1m == 0.0 {
            stats.avg_1m = mining_metrics.total_hashrate;
        } else {
            stats.avg_1m = stats.avg_1m * (1.0 - alpha_1m) + mining_metrics.total_hashrate * alpha_1m;
        }

        if stats.avg_5m == 0.0 {
            stats.avg_5m = mining_metrics.total_hashrate;
        } else {
            stats.avg_5m = stats.avg_5m * (1.0 - alpha_5m) + mining_metrics.total_hashrate * alpha_5m;
        }

        if stats.avg_15m == 0.0 {
            stats.avg_15m = mining_metrics.total_hashrate;
        } else {
            stats.avg_15m = stats.avg_15m * (1.0 - alpha_15m) + mining_metrics.total_hashrate * alpha_15m;
        }

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

    /// 更新设备统计信息（使用真实的滑动窗口算力）
    pub async fn update_device_stats(&self, device_stats_core: &cgminer_core::DeviceStats) -> Result<(), MiningError> {
        let mut device_stats = self.device_stats.write().await;

        let device_stat = DeviceHashrateStats {
            device_id: device_stats_core.device_id,
            device_name: format!("Device {}", device_stats_core.device_id),
            stats: HashrateStats {
                current_hashrate: device_stats_core.current_hashrate.hashes_per_second,
                avg_5s: device_stats_core.current_hashrate.hashes_per_second, // 使用当前算力作为5秒算力
                avg_1m: device_stats_core.hashrate_1m.hashes_per_second,
                avg_5m: device_stats_core.hashrate_5m.hashes_per_second,
                avg_15m: device_stats_core.hashrate_15m.hashes_per_second,
                avg_total: device_stats_core.average_hashrate.hashes_per_second,
                accepted_shares: device_stats_core.accepted_work,
                rejected_shares: device_stats_core.rejected_work,
                hardware_errors: device_stats_core.hardware_errors,
                work_utility: 0.0, // 可以后续计算
                uptime: device_stats_core.uptime,
            },
            temperature: device_stats_core.temperature.map(|t| t.celsius).unwrap_or(0.0),
            fan_speed: device_stats_core.fan_speed.unwrap_or(0),
        };

        device_stats.insert(device_stats_core.device_id, device_stat);
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

        Self::output_traditional_format(&stats, &devices, config).await;
    }

    /// 传统格式输出 (类似原版cgminer，显示滑动窗口算力)
    async fn output_traditional_format(
        stats: &HashrateStats,
        devices: &HashMap<u32, DeviceHashrateStats>,
        config: &HashmeterConfig,
    ) {
        let avg_5s = Self::format_hashrate(stats.avg_5s, &config.hashrate_unit);
        let avg_1m = Self::format_hashrate(stats.avg_1m, &config.hashrate_unit);
        let avg_5m = Self::format_hashrate(stats.avg_5m, &config.hashrate_unit);
        let avg_15m = Self::format_hashrate(stats.avg_15m, &config.hashrate_unit);

        // 设备数量显示：优先使用设备统计数据，否则使用配置的设备数量
        let device_count = if devices.is_empty() {
            4 // CPU BTC 核心配置的设备数量
        } else {
            devices.len()
        };

        // cgminer风格的状态行格式: (5s):16.896Mh/s (1m):12.374Mh/s (5m):9.649Mh/s (15m):9.054Mh/s A:782 R:0 HW:0 [16DEV]
        info!("({}s):{} (1m):{} (5m):{} (15m):{} A:{} R:{} HW:{} [{}DEV]",
              config.log_interval,
              avg_5s,
              avg_1m,
              avg_5m,
              avg_15m,
              stats.accepted_shares,
              stats.rejected_shares,
              stats.hardware_errors,
              device_count
        );

        if config.per_device_stats {
            for device in devices.values() {
                let device_5s = Self::format_hashrate(device.stats.avg_5s, &config.hashrate_unit);
                let device_1m = Self::format_hashrate(device.stats.avg_1m, &config.hashrate_unit);
                let device_5m = Self::format_hashrate(device.stats.avg_5m, &config.hashrate_unit);
                info!("{} {}: {} (1m):{} (5m):{} | A:{} R:{} HW:{} | {:.1}°C",
                      device.device_name,
                      device.device_id,
                      device_5s,
                      device_1m,
                      device_5m,
                      device.stats.accepted_shares,
                      device.stats.rejected_shares,
                      device.stats.hardware_errors,
                      device.temperature
                );
            }
        }
    }

    /// 格式化算力显示（智能单位自适应）
    fn format_hashrate(hashrate: f64, _unit: &str) -> String {
        // 始终使用自动单位选择，忽略配置的单位
        Self::format_hashrate_auto(hashrate)
    }

    /// 自动选择最合适的单位进行格式化（智能单位适配）
    fn format_hashrate_auto(hashrate: f64) -> String {
        if hashrate <= 0.0 {
            return "0.00 H/s".to_string();
        }

        // 智能选择最合适的单位，确保显示值在合理范围内（1-999）
        if hashrate >= 1_000_000_000_000.0 {
            let th_value = hashrate / 1_000_000_000_000.0;
            if th_value >= 100.0 {
                format!("{:.1} TH/s", th_value)
            } else if th_value >= 10.0 {
                format!("{:.2} TH/s", th_value)
            } else {
                format!("{:.3} TH/s", th_value)
            }
        } else if hashrate >= 1_000_000_000.0 {
            let gh_value = hashrate / 1_000_000_000.0;
            if gh_value >= 100.0 {
                format!("{:.1} GH/s", gh_value)
            } else if gh_value >= 10.0 {
                format!("{:.2} GH/s", gh_value)
            } else if gh_value >= 1.0 {
                format!("{:.3} GH/s", gh_value)
            } else {
                // 如果GH值小于1，降级到MH
                let mh_value = hashrate / 1_000_000.0;
                if mh_value >= 100.0 {
                    format!("{:.1} MH/s", mh_value)
                } else if mh_value >= 10.0 {
                    format!("{:.2} MH/s", mh_value)
                } else {
                    format!("{:.3} MH/s", mh_value)
                }
            }
        } else if hashrate >= 1_000_000.0 {
            let mh_value = hashrate / 1_000_000.0;
            if mh_value >= 100.0 {
                format!("{:.1} MH/s", mh_value)
            } else if mh_value >= 10.0 {
                format!("{:.2} MH/s", mh_value)
            } else if mh_value >= 1.0 {
                format!("{:.3} MH/s", mh_value)
            } else {
                // 如果MH值小于1，降级到KH
                let kh_value = hashrate / 1_000.0;
                if kh_value >= 100.0 {
                    format!("{:.1} KH/s", kh_value)
                } else if kh_value >= 10.0 {
                    format!("{:.2} KH/s", kh_value)
                } else {
                    format!("{:.3} KH/s", kh_value)
                }
            }
        } else if hashrate >= 1_000.0 {
            let kh_value = hashrate / 1_000.0;
            if kh_value >= 100.0 {
                format!("{:.1} KH/s", kh_value)
            } else if kh_value >= 10.0 {
                format!("{:.2} KH/s", kh_value)
            } else if kh_value >= 1.0 {
                format!("{:.3} KH/s", kh_value)
            } else {
                // 如果KH值小于1，降级到H
                if hashrate >= 100.0 {
                    format!("{:.1} H/s", hashrate)
                } else if hashrate >= 10.0 {
                    format!("{:.2} H/s", hashrate)
                } else {
                    format!("{:.3} H/s", hashrate)
                }
            }
        } else if hashrate >= 1.0 {
            if hashrate >= 100.0 {
                format!("{:.1} H/s", hashrate)
            } else if hashrate >= 10.0 {
                format!("{:.2} H/s", hashrate)
            } else {
                format!("{:.3} H/s", hashrate)
            }
        } else {
            // 对于非常小的算力值，显示更高精度
            format!("{:.6} H/s", hashrate)
        }
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashrate_auto_formatting() {
        // 测试自动单位选择 - 现在所有单位都会自动选择
        assert_eq!(Hashmeter::format_hashrate(7_399_000.0, ""), "7.399 MH/s");
        assert_eq!(Hashmeter::format_hashrate(7_399_000_000.0, ""), "7.399 GH/s");
        assert_eq!(Hashmeter::format_hashrate(7_399.0, ""), "7.399 KH/s");
        assert_eq!(Hashmeter::format_hashrate(7.399, ""), "7.399 H/s");

        // 测试边界情况
        assert_eq!(Hashmeter::format_hashrate(999_999_999.0, ""), "1000.0 MH/s");
        assert_eq!(Hashmeter::format_hashrate(1_000_000_000.0, ""), "1.000 GH/s");

        // 测试零值
        assert_eq!(Hashmeter::format_hashrate(0.0, ""), "0.00 H/s");

        // 测试小数值
        assert_eq!(Hashmeter::format_hashrate(0.007399, ""), "0.007399 H/s");
    }

    #[test]
    fn test_hashrate_unit_independence() {
        // 测试单位参数被忽略，都使用自动选择
        assert_eq!(Hashmeter::format_hashrate(7_399_000.0, "GH"), "7.399 MH/s");
        assert_eq!(Hashmeter::format_hashrate(7_399_000.0, "KH"), "7.399 MH/s");
        assert_eq!(Hashmeter::format_hashrate(7_399_000.0, "TH"), "7.399 MH/s");
        assert_eq!(Hashmeter::format_hashrate(7_399_000.0, "INVALID"), "7.399 MH/s");
    }
}
