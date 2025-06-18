//! 算力计量器和美化显示

use crate::logging::formatter;
use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime};
use tracing::{info, debug};

/// 算力计量器
pub struct HashMeter {
    /// 算力历史记录
    hashrate_history: VecDeque<HashRateRecord>,
    /// 最大历史记录数
    max_history: usize,
    /// 上次显示时间
    last_display: Instant,
    /// 显示间隔
    display_interval: Duration,
    /// 是否启用美化显示
    pretty_display: bool,
}

/// 算力记录
#[derive(Debug, Clone)]
pub struct HashRateRecord {
    /// 时间戳
    pub timestamp: SystemTime,
    /// 算力值 (H/s)
    pub hashrate: f64,
    /// 设备ID
    pub device_id: Option<u32>,
    /// 矿池ID
    pub pool_id: Option<u32>,
}

/// 算力统计信息
#[derive(Debug, Clone)]
pub struct HashRateStats {
    /// 当前算力
    pub current: f64,
    /// 平均算力
    pub average: f64,
    /// 最大算力
    pub max: f64,
    /// 最小算力
    pub min: f64,
    /// 总样本数
    pub samples: usize,
    /// 时间范围
    pub duration: Duration,
}

impl HashMeter {
    /// 创建新的算力计量器
    pub fn new(display_interval: Duration, pretty_display: bool) -> Self {
        Self {
            hashrate_history: VecDeque::new(),
            max_history: 1000, // 保留最近1000个记录
            last_display: Instant::now(),
            display_interval,
            pretty_display,
        }
    }

    /// 记录算力
    pub fn record_hashrate(&mut self, hashrate: f64, device_id: Option<u32>, pool_id: Option<u32>) {
        let record = HashRateRecord {
            timestamp: SystemTime::now(),
            hashrate,
            device_id,
            pool_id,
        };

        self.hashrate_history.push_back(record);

        // 限制历史记录数量
        while self.hashrate_history.len() > self.max_history {
            self.hashrate_history.pop_front();
        }

        // 检查是否需要显示
        if self.last_display.elapsed() >= self.display_interval {
            self.display_stats();
            self.last_display = Instant::now();
        }
    }

    /// 获取算力统计
    pub fn get_stats(&self, duration: Option<Duration>) -> HashRateStats {
        if self.hashrate_history.is_empty() {
            return HashRateStats {
                current: 0.0,
                average: 0.0,
                max: 0.0,
                min: 0.0,
                samples: 0,
                duration: Duration::from_secs(0),
            };
        }

        let now = SystemTime::now();
        let cutoff_time = if let Some(dur) = duration {
            now.checked_sub(dur).unwrap_or(SystemTime::UNIX_EPOCH)
        } else {
            SystemTime::UNIX_EPOCH
        };

        let relevant_records: Vec<&HashRateRecord> = self.hashrate_history
            .iter()
            .filter(|record| record.timestamp >= cutoff_time)
            .collect();

        if relevant_records.is_empty() {
            return HashRateStats {
                current: 0.0,
                average: 0.0,
                max: 0.0,
                min: 0.0,
                samples: 0,
                duration: Duration::from_secs(0),
            };
        }

        let current = self.hashrate_history.back().unwrap().hashrate;
        let hashrates: Vec<f64> = relevant_records.iter().map(|r| r.hashrate).collect();

        let sum: f64 = hashrates.iter().sum();
        let average = sum / hashrates.len() as f64;
        let max = hashrates.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = hashrates.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        let actual_duration = if let (Some(first), Some(last)) = (relevant_records.first(), relevant_records.last()) {
            last.timestamp.duration_since(first.timestamp).unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        };

        HashRateStats {
            current,
            average,
            max,
            min,
            samples: relevant_records.len(),
            duration: actual_duration,
        }
    }

    /// 显示算力统计
    pub fn display_stats(&self) {
        if self.pretty_display {
            self.display_pretty_stats();
        } else {
            self.display_simple_stats();
        }
    }

    /// 美化显示算力统计
    fn display_pretty_stats(&self) {
        let stats_5min = self.get_stats(Some(Duration::from_secs(300)));
        let stats_1hour = self.get_stats(Some(Duration::from_secs(3600)));
        let stats_all = self.get_stats(None);

        // 创建表格
        let separator = formatter::create_separator("算力统计", 80);
        info!("{}", separator);

        // 表头
        let headers = ["时间范围", "当前算力", "平均算力", "最大算力", "最小算力"];
        let widths = [12, 16, 16, 16, 16];
        let header_row = formatter::create_table_row(&headers, &widths);
        info!("{}", header_row);

        // 分隔线
        info!("╠{}╪{}╪{}╪{}╪{}╣",
              "═".repeat(widths[0]),
              "═".repeat(widths[1]),
              "═".repeat(widths[2]),
              "═".repeat(widths[3]),
              "═".repeat(widths[4]));

        // 5分钟统计
        let row_5min = formatter::create_table_row(&[
            "5分钟",
            &format_hashrate(stats_5min.current),
            &format_hashrate(stats_5min.average),
            &format_hashrate(stats_5min.max),
            &format_hashrate(stats_5min.min),
        ], &widths);
        info!("{}", row_5min);

        // 1小时统计
        let row_1hour = formatter::create_table_row(&[
            "1小时",
            &format_hashrate(stats_1hour.current),
            &format_hashrate(stats_1hour.average),
            &format_hashrate(stats_1hour.max),
            &format_hashrate(stats_1hour.min),
        ], &widths);
        info!("{}", row_1hour);

        // 全部统计
        let row_all = formatter::create_table_row(&[
            "全部",
            &format_hashrate(stats_all.current),
            &format_hashrate(stats_all.average),
            &format_hashrate(stats_all.max),
            &format_hashrate(stats_all.min),
        ], &widths);
        info!("{}", row_all);

        // 底部
        info!("╚{}╧{}╧{}╧{}╧{}╝",
              "═".repeat(widths[0]),
              "═".repeat(widths[1]),
              "═".repeat(widths[2]),
              "═".repeat(widths[3]),
              "═".repeat(widths[4]));

        // 额外信息
        info!("📊 样本数: {} | ⏱️ 运行时间: {}",
              stats_all.samples,
              formatter::format_duration(stats_all.duration));
    }

    /// 简单显示算力统计
    fn display_simple_stats(&self) {
        let stats = self.get_stats(Some(Duration::from_secs(300))); // 5分钟统计

        info!("⛏️ 当前算力: {} | 平均算力: {} | 样本数: {}",
              format_hashrate(stats.current),
              format_hashrate(stats.average),
              stats.samples);
    }

    /// 获取设备算力统计
    pub fn get_device_stats(&self, device_id: u32, duration: Option<Duration>) -> HashRateStats {
        let now = SystemTime::now();
        let cutoff_time = if let Some(dur) = duration {
            now.checked_sub(dur).unwrap_or(SystemTime::UNIX_EPOCH)
        } else {
            SystemTime::UNIX_EPOCH
        };

        let device_records: Vec<&HashRateRecord> = self.hashrate_history
            .iter()
            .filter(|record| {
                record.timestamp >= cutoff_time &&
                record.device_id == Some(device_id)
            })
            .collect();

        if device_records.is_empty() {
            return HashRateStats {
                current: 0.0,
                average: 0.0,
                max: 0.0,
                min: 0.0,
                samples: 0,
                duration: Duration::from_secs(0),
            };
        }

        let current = device_records.last().unwrap().hashrate;
        let hashrates: Vec<f64> = device_records.iter().map(|r| r.hashrate).collect();

        let sum: f64 = hashrates.iter().sum();
        let average = sum / hashrates.len() as f64;
        let max = hashrates.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = hashrates.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        let actual_duration = if let (Some(first), Some(last)) = (device_records.first(), device_records.last()) {
            last.timestamp.duration_since(first.timestamp).unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        };

        HashRateStats {
            current,
            average,
            max,
            min,
            samples: device_records.len(),
            duration: actual_duration,
        }
    }

    /// 清除历史记录
    pub fn clear_history(&mut self) {
        self.hashrate_history.clear();
        debug!("算力历史记录已清除");
    }

    /// 设置显示间隔
    pub fn set_display_interval(&mut self, interval: Duration) {
        self.display_interval = interval;
        debug!("算力显示间隔设置为: {:?}", interval);
    }
}

/// 格式化算力显示
pub fn format_hashrate(hashrate: f64) -> String {
    formatter::format_hashrate(hashrate)
}

/// 创建算力趋势图 (简化版)
pub fn create_hashrate_trend(records: &[HashRateRecord], width: usize) -> String {
    if records.is_empty() || width == 0 {
        return String::new();
    }

    let hashrates: Vec<f64> = records.iter().map(|r| r.hashrate).collect();
    let max_hashrate = hashrates.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let min_hashrate = hashrates.iter().fold(f64::INFINITY, |a, &b| a.min(b));

    if max_hashrate == min_hashrate {
        return "─".repeat(width);
    }

    let range = max_hashrate - min_hashrate;
    let step = records.len() as f64 / width as f64;

    let mut trend = String::new();
    for i in 0..width {
        let index = (i as f64 * step) as usize;
        if index < hashrates.len() {
            let normalized = (hashrates[index] - min_hashrate) / range;
            let char = match (normalized * 8.0) as u8 {
                0 => '▁',
                1 => '▂',
                2 => '▃',
                3 => '▄',
                4 => '▅',
                5 => '▆',
                6 => '▇',
                _ => '█',
            };
            trend.push(char);
        } else {
            trend.push(' ');
        }
    }

    trend
}

/// 全局算力计量器实例
static mut GLOBAL_HASHMETER: Option<HashMeter> = None;
static HASHMETER_INIT: std::sync::Once = std::sync::Once::new();

/// 初始化全局算力计量器
pub fn init_global_hashmeter(display_interval: Duration, pretty_display: bool) {
    HASHMETER_INIT.call_once(|| {
        unsafe {
            GLOBAL_HASHMETER = Some(HashMeter::new(display_interval, pretty_display));
        }
    });
}

/// 记录全局算力
pub fn record_global_hashrate(hashrate: f64, device_id: Option<u32>, pool_id: Option<u32>) {
    unsafe {
        if let Some(ref mut meter) = GLOBAL_HASHMETER {
            meter.record_hashrate(hashrate, device_id, pool_id);
        }
    }
}

/// 获取全局算力统计
pub fn get_global_stats(duration: Option<Duration>) -> Option<HashRateStats> {
    unsafe {
        GLOBAL_HASHMETER.as_ref().map(|meter| meter.get_stats(duration))
    }
}
