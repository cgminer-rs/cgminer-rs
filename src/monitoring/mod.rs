pub mod system;
pub mod metrics;
pub mod alerts;
pub mod simple_web;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub use system::MonitoringSystem;
pub use alerts::Alert;

/// 系统指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: SystemTime,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub temperature: f32,
    pub fan_speed: u32,
    pub power_consumption: f64,
    pub uptime: Duration,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_rx: 0,
            network_tx: 0,
            temperature: 0.0,
            fan_speed: 0,
            power_consumption: 0.0,
            uptime: Duration::from_secs(0),
        }
    }
}

/// 挖矿指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningMetrics {
    pub timestamp: SystemTime,
    pub total_hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub stale_shares: u64,
    pub best_share: f64,
    pub current_difficulty: f64,
    pub network_difficulty: f64,
    pub blocks_found: u32,
    pub efficiency: f64, // MH/J
    pub active_devices: u32,
    pub connected_pools: u32,
}

impl Default for MiningMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            total_hashrate: 0.0,
            accepted_shares: 0,
            rejected_shares: 0,
            hardware_errors: 0,
            stale_shares: 0,
            best_share: 0.0,
            current_difficulty: 1.0,
            network_difficulty: 1.0,
            blocks_found: 0,
            efficiency: 0.0,
            active_devices: 0,
            connected_pools: 0,
        }
    }
}

/// 设备指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetrics {
    pub device_id: u32,
    pub timestamp: SystemTime,
    pub temperature: f32,
    pub hashrate: f64,
    pub power_consumption: f64,
    pub fan_speed: u32,
    pub voltage: u32,
    pub frequency: u32,
    pub error_rate: f64,
    pub uptime: Duration,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
}

impl DeviceMetrics {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            timestamp: SystemTime::now(),
            temperature: 0.0,
            hashrate: 0.0,
            power_consumption: 0.0,
            fan_speed: 0,
            voltage: 0,
            frequency: 0,
            error_rate: 0.0,
            uptime: Duration::from_secs(0),
            accepted_shares: 0,
            rejected_shares: 0,
            hardware_errors: 0,
        }
    }
}

/// 矿池指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub pool_id: u32,
    pub timestamp: SystemTime,
    pub connected: bool,
    pub ping: Option<Duration>,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub difficulty: f64,
    pub last_share_time: Option<SystemTime>,
    pub connection_uptime: Duration,
}

impl PoolMetrics {
    pub fn new(pool_id: u32) -> Self {
        Self {
            pool_id,
            timestamp: SystemTime::now(),
            connected: false,
            ping: None,
            accepted_shares: 0,
            rejected_shares: 0,
            stale_shares: 0,
            difficulty: 1.0,
            last_share_time: None,
            connection_uptime: Duration::from_secs(0),
        }
    }
}

/// 监控事件
#[derive(Debug, Clone)]
pub enum MonitoringEvent {
    /// 系统指标更新
    SystemMetricsUpdate {
        metrics: SystemMetrics,
        timestamp: SystemTime,
    },
    /// 挖矿指标更新
    MiningMetricsUpdate {
        metrics: MiningMetrics,
        timestamp: SystemTime,
    },
    /// 设备指标更新
    DeviceMetricsUpdate {
        device_id: u32,
        metrics: DeviceMetrics,
        timestamp: SystemTime,
    },
    /// 矿池指标更新
    PoolMetricsUpdate {
        pool_id: u32,
        metrics: PoolMetrics,
        timestamp: SystemTime,
    },
    /// 告警触发
    AlertTriggered {
        alert: Alert,
        timestamp: SystemTime,
    },
    /// 告警解除
    AlertResolved {
        alert_id: String,
        timestamp: SystemTime,
    },
}

impl MonitoringEvent {
    pub fn timestamp(&self) -> SystemTime {
        match self {
            MonitoringEvent::SystemMetricsUpdate { timestamp, .. } => *timestamp,
            MonitoringEvent::MiningMetricsUpdate { timestamp, .. } => *timestamp,
            MonitoringEvent::DeviceMetricsUpdate { timestamp, .. } => *timestamp,
            MonitoringEvent::PoolMetricsUpdate { timestamp, .. } => *timestamp,
            MonitoringEvent::AlertTriggered { timestamp, .. } => *timestamp,
            MonitoringEvent::AlertResolved { timestamp, .. } => *timestamp,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            MonitoringEvent::SystemMetricsUpdate { .. } => "system_metrics_update",
            MonitoringEvent::MiningMetricsUpdate { .. } => "mining_metrics_update",
            MonitoringEvent::DeviceMetricsUpdate { .. } => "device_metrics_update",
            MonitoringEvent::PoolMetricsUpdate { .. } => "pool_metrics_update",
            MonitoringEvent::AlertTriggered { .. } => "alert_triggered",
            MonitoringEvent::AlertResolved { .. } => "alert_resolved",
        }
    }
}

/// 监控状态
#[derive(Debug, Clone, PartialEq)]
pub enum MonitoringState {
    /// 未启动
    Stopped,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 正在停止
    Stopping,
    /// 错误状态
    Error(String),
}

/// 指标历史记录
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    pub system_metrics: Vec<SystemMetrics>,
    pub mining_metrics: Vec<MiningMetrics>,
    pub device_metrics: HashMap<u32, Vec<DeviceMetrics>>,
    pub pool_metrics: HashMap<u32, Vec<PoolMetrics>>,
    pub max_entries: usize,
}

impl MetricsHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            system_metrics: Vec::new(),
            mining_metrics: Vec::new(),
            device_metrics: HashMap::new(),
            pool_metrics: HashMap::new(),
            max_entries,
        }
    }

    pub fn add_system_metrics(&mut self, metrics: SystemMetrics) {
        self.system_metrics.push(metrics);
        if self.system_metrics.len() > self.max_entries {
            self.system_metrics.remove(0);
        }
    }

    pub fn add_mining_metrics(&mut self, metrics: MiningMetrics) {
        self.mining_metrics.push(metrics);
        if self.mining_metrics.len() > self.max_entries {
            self.mining_metrics.remove(0);
        }
    }

    pub fn add_device_metrics(&mut self, device_id: u32, metrics: DeviceMetrics) {
        let device_history = self.device_metrics.entry(device_id).or_insert_with(Vec::new);
        device_history.push(metrics);
        if device_history.len() > self.max_entries {
            device_history.remove(0);
        }
    }

    pub fn add_pool_metrics(&mut self, pool_id: u32, metrics: PoolMetrics) {
        let pool_history = self.pool_metrics.entry(pool_id).or_insert_with(Vec::new);
        pool_history.push(metrics);
        if pool_history.len() > self.max_entries {
            pool_history.remove(0);
        }
    }

    pub fn get_latest_system_metrics(&self) -> Option<&SystemMetrics> {
        self.system_metrics.last()
    }

    pub fn get_latest_mining_metrics(&self) -> Option<&MiningMetrics> {
        self.mining_metrics.last()
    }

    pub fn get_latest_device_metrics(&self, device_id: u32) -> Option<&DeviceMetrics> {
        self.device_metrics.get(&device_id)?.last()
    }

    pub fn get_latest_pool_metrics(&self, pool_id: u32) -> Option<&PoolMetrics> {
        self.pool_metrics.get(&pool_id)?.last()
    }

    pub fn clear(&mut self) {
        self.system_metrics.clear();
        self.mining_metrics.clear();
        self.device_metrics.clear();
        self.pool_metrics.clear();
    }
}

/// 性能统计
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PerformanceStats {
    pub metrics_collection_time: Duration,
    pub alert_processing_time: Duration,
    pub total_metrics_collected: u64,
    pub total_alerts_triggered: u64,
    pub last_collection_time: Option<SystemTime>,
}

impl PerformanceStats {
    pub fn record_collection_time(&mut self, duration: Duration) {
        self.metrics_collection_time = duration;
        self.total_metrics_collected += 1;
        self.last_collection_time = Some(SystemTime::now());
    }

    pub fn record_alert_processing_time(&mut self, duration: Duration) {
        self.alert_processing_time = duration;
        self.total_alerts_triggered += 1;
    }

    pub fn get_average_collection_time(&self) -> Duration {
        if self.total_metrics_collected == 0 {
            Duration::from_secs(0)
        } else {
            self.metrics_collection_time / self.total_metrics_collected as u32
        }
    }
}
