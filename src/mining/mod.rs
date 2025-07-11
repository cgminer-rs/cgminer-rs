pub mod manager;
pub mod work_queue;
pub mod hashmeter;

use crate::config::Config;
use cgminer_core::Work;
use cgminer_core::types::MiningResult;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub use manager::MiningManager;

pub use hashmeter::{Hashmeter, HashmeterConfig};

/// 挖矿状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MiningState {
    /// 未启动
    Stopped,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 正在停止
    Stopping,
    /// 暂停
    Paused,
    /// 错误状态
    Error(String),
}

/// 挖矿统计信息
#[derive(Debug, Clone, Default)]
pub struct MiningStats {
    pub start_time: Option<SystemTime>,
    pub uptime: Duration,
    pub total_hashes: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub stale_shares: u64,
    pub best_share: f64,
    pub current_difficulty: f64,
    pub network_difficulty: f64,
    pub blocks_found: u32,
    pub last_share_time: Option<SystemTime>,
    pub average_hashrate: f64,
    pub current_hashrate: f64,
    pub efficiency: f64, // MH/J
    pub power_consumption: f64, // Watts
}

impl MiningStats {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn start(&mut self) {
        self.start_time = Some(SystemTime::now());
    }

    pub fn update_uptime(&mut self) {
        if let Some(start_time) = self.start_time {
            self.uptime = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::from_secs(0));
        }
    }

    pub fn record_accepted_share(&mut self, difficulty: f64) {
        self.accepted_shares += 1;
        self.last_share_time = Some(SystemTime::now());
        if difficulty > self.best_share {
            self.best_share = difficulty;
        }
    }

    pub fn record_rejected_share(&mut self) {
        self.rejected_shares += 1;
    }

    pub fn record_hardware_error(&mut self) {
        self.hardware_errors += 1;
    }

    pub fn record_stale_share(&mut self) {
        self.stale_shares += 1;
    }

    pub fn update_hashrate(&mut self, hashrate: f64) {
        self.current_hashrate = hashrate;

        // 计算平均算力 (简单的指数移动平均)
        if self.average_hashrate == 0.0 {
            self.average_hashrate = hashrate;
        } else {
            self.average_hashrate = self.average_hashrate * 0.9 + hashrate * 0.1;
        }
    }

    pub fn update_power_consumption(&mut self, power: f64) {
        self.power_consumption = power;

        // 计算效率 (MH/J)
        if power > 0.0 {
            self.efficiency = self.current_hashrate / power * 1000.0; // 转换为 MH/J
        }
    }

    pub fn get_accept_rate(&self) -> f64 {
        let total_shares = self.accepted_shares + self.rejected_shares;
        if total_shares == 0 {
            0.0
        } else {
            self.accepted_shares as f64 / total_shares as f64 * 100.0
        }
    }

    pub fn get_reject_rate(&self) -> f64 {
        let total_shares = self.accepted_shares + self.rejected_shares;
        if total_shares == 0 {
            0.0
        } else {
            self.rejected_shares as f64 / total_shares as f64 * 100.0
        }
    }

    pub fn get_hardware_error_rate(&self) -> f64 {
        let total_work = self.accepted_shares + self.rejected_shares + self.hardware_errors;
        if total_work == 0 {
            0.0
        } else {
            self.hardware_errors as f64 / total_work as f64 * 100.0
        }
    }
}

/// 工作分发策略
#[derive(Debug, Clone)]
pub enum WorkDistributionStrategy {
    /// 轮询分发
    RoundRobin,
    /// 负载均衡
    LoadBalance,
    /// 优先级分发
    Priority,
    /// 随机分发
    Random,
}

/// 挖矿配置
#[derive(Debug, Clone)]
pub struct MiningConfig {
    pub work_restart_timeout: Duration,
    pub scan_interval: Duration,
    /// 结果收集间隔 - 从配置文件读取
    pub result_collection_interval: Duration,
    pub work_distribution_strategy: WorkDistributionStrategy,
    pub max_work_queue_size: usize,
    pub max_result_queue_size: usize,
    pub batch_size: usize,
    pub enable_auto_tuning: bool,
    pub target_temperature: f32,
    pub max_temperature: f32,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            work_restart_timeout: Duration::from_secs(60),
            scan_interval: Duration::from_secs(30),
            result_collection_interval: Duration::from_millis(20),
            work_distribution_strategy: WorkDistributionStrategy::LoadBalance,
            max_work_queue_size: 1000,
            max_result_queue_size: 1000,
            batch_size: 100,
            enable_auto_tuning: true,
            target_temperature: 75.0,
            max_temperature: 85.0,
        }
    }
}

impl From<&Config> for MiningConfig {
    fn from(config: &Config) -> Self {
        Self {
            work_restart_timeout: Duration::from_secs(config.general.work_restart_timeout),
            scan_interval: Duration::from_secs(config.general.scan_time),
            result_collection_interval: Duration::from_millis(config.general.result_collection_interval_ms),
            work_distribution_strategy: WorkDistributionStrategy::LoadBalance,
            max_work_queue_size: 1000, // 可以从配置中读取
            max_result_queue_size: 1000,
            batch_size: 100,
            enable_auto_tuning: config.devices.auto_detect, // 临时映射
            target_temperature: 75.0,
            max_temperature: config.monitoring.alert_thresholds.temperature_critical,
        }
    }
}

/// 工作项
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub work: Work,
    pub assigned_device: Option<u32>,
    pub created_at: SystemTime,
    pub priority: u8,
    pub retry_count: u32,
}

impl WorkItem {
    pub fn new(work: Work) -> Self {
        Self {
            work,
            assigned_device: None,
            created_at: SystemTime::now(),
            priority: 0,
            retry_count: 0,
        }
    }

    pub fn with_device(mut self, device_id: u32) -> Self {
        self.assigned_device = Some(device_id);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn is_expired(&self) -> bool {
        self.work.is_expired()
    }

    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::from_secs(0))
    }

    /// 验证工作数据完整性
    pub fn validate_work_integrity(&self) -> Result<(), String> {
        // 验证基本字段
        if self.work.job_id.is_empty() {
            return Err("Job ID is empty".to_string());
        }

        // 验证区块头不全为零
        if self.work.header.iter().all(|&b| b == 0) {
            return Err("Block header is all zeros".to_string());
        }

        // 验证目标值不全为零
        if self.work.target.iter().all(|&b| b == 0) {
            return Err("Target is all zeros".to_string());
        }

        // 验证难度值
        if self.work.difficulty <= 0.0 || !self.work.difficulty.is_finite() {
            return Err(format!("Invalid difficulty: {}", self.work.difficulty));
        }

        // 验证extranonce字段
        if self.work.extranonce1.is_empty() {
            return Err("Extranonce1 is empty".to_string());
        }

        if self.work.extranonce2.len() != self.work.extranonce2_size {
            return Err(format!(
                "Extranonce2 size mismatch: expected {}, got {}",
                self.work.extranonce2_size,
                self.work.extranonce2.len()
            ));
        }

        // 验证coinbase数据
        if self.work.coinbase1.is_empty() {
            return Err("Coinbase1 is empty".to_string());
        }

        if self.work.coinbase2.is_empty() {
            return Err("Coinbase2 is empty".to_string());
        }

        // 验证时间戳
        if self.work.ntime == 0 {
            return Err("nTime is zero".to_string());
        }

        // 验证版本
        if self.work.version == 0 {
            return Err("Version is zero".to_string());
        }

        // 验证nBits
        if self.work.nbits == 0 {
            return Err("nBits is zero".to_string());
        }

        Ok(())
    }

    /// 验证工作是否适合指定设备
    pub fn is_suitable_for_device(&self, device_id: u32) -> bool {
        // 如果已分配给特定设备，检查是否匹配
        if let Some(assigned_device) = self.assigned_device {
            return assigned_device == device_id;
        }

        // 如果未分配，检查工作是否仍然有效
        !self.is_expired() && self.validate_work_integrity().is_ok()
    }

    /// 创建工作的深拷贝（确保数据完整性）
    pub fn deep_clone(&self) -> Self {
        Self {
            work: self.work.clone(),
            assigned_device: self.assigned_device,
            created_at: self.created_at,
            priority: self.priority,
            retry_count: self.retry_count,
        }
    }
}

/// 挖矿结果项
#[derive(Debug, Clone)]
pub struct ResultItem {
    pub result: MiningResult,
    pub work_item: WorkItem,
    pub processed_at: SystemTime,
    pub validation_status: ValidationStatus,
}

/// 验证状态
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    /// 未验证
    Pending,
    /// 验证通过
    Valid,
    /// 验证失败
    Invalid(String),
    /// 过期
    Stale,
}

impl ResultItem {
    pub fn new(result: MiningResult, work_item: WorkItem) -> Self {
        Self {
            result,
            work_item,
            processed_at: SystemTime::now(),
            validation_status: ValidationStatus::Pending,
        }
    }

    pub fn mark_valid(mut self) -> Self {
        self.validation_status = ValidationStatus::Valid;
        self
    }

    pub fn mark_invalid(mut self, reason: String) -> Self {
        self.validation_status = ValidationStatus::Invalid(reason);
        self
    }

    pub fn mark_stale(mut self) -> Self {
        self.validation_status = ValidationStatus::Stale;
        self
    }

    pub fn is_valid(&self) -> bool {
        matches!(self.validation_status, ValidationStatus::Valid)
    }
}

/// 挖矿事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MiningEvent {
    /// 状态变更
    StateChanged {
        old_state: MiningState,
        new_state: MiningState,
        timestamp: SystemTime,
    },
    /// 工作接收
    WorkReceived {
        work_id: Uuid,
        job_id: String,
        difficulty: f64,
        timestamp: SystemTime,
    },
    /// 份额提交
    ShareSubmitted {
        work_id: Uuid,
        device_id: u32,
        nonce: u32,
        difficulty: f64,
        timestamp: SystemTime,
    },
    /// 份额接受
    ShareAccepted {
        work_id: Uuid,
        difficulty: f64,
        timestamp: SystemTime,
    },
    /// 份额拒绝
    ShareRejected {
        work_id: Uuid,
        reason: String,
        timestamp: SystemTime,
    },
    /// 硬件错误
    HardwareError {
        device_id: u32,
        error: String,
        timestamp: SystemTime,
    },
    /// 设备状态变更
    DeviceStateChanged {
        device_id: u32,
        old_state: String,
        new_state: String,
        timestamp: SystemTime,
    },
    /// 矿池连接状态变更
    PoolConnectionChanged {
        pool_id: u32,
        connected: bool,
        timestamp: SystemTime,
    },
}

impl MiningEvent {
    pub fn timestamp(&self) -> SystemTime {
        match self {
            MiningEvent::StateChanged { timestamp, .. } => *timestamp,
            MiningEvent::WorkReceived { timestamp, .. } => *timestamp,
            MiningEvent::ShareSubmitted { timestamp, .. } => *timestamp,
            MiningEvent::ShareAccepted { timestamp, .. } => *timestamp,
            MiningEvent::ShareRejected { timestamp, .. } => *timestamp,
            MiningEvent::HardwareError { timestamp, .. } => *timestamp,
            MiningEvent::DeviceStateChanged { timestamp, .. } => *timestamp,
            MiningEvent::PoolConnectionChanged { timestamp, .. } => *timestamp,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            MiningEvent::StateChanged { .. } => "state_changed",
            MiningEvent::WorkReceived { .. } => "work_received",
            MiningEvent::ShareSubmitted { .. } => "share_submitted",
            MiningEvent::ShareAccepted { .. } => "share_accepted",
            MiningEvent::ShareRejected { .. } => "share_rejected",
            MiningEvent::HardwareError { .. } => "hardware_error",
            MiningEvent::DeviceStateChanged { .. } => "device_state_changed",
            MiningEvent::PoolConnectionChanged { .. } => "pool_connection_changed",
        }
    }
}
