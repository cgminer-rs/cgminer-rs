pub mod manager;
pub mod chain;
pub mod traits;
pub mod virtual_device;
pub mod conversion;
pub mod factory;

#[cfg(test)]
mod tests;

use crate::error::DeviceError;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub use manager::DeviceManager;
pub use traits::ChainController;
pub use traits::{MiningDevice, DeviceDriver};

/// 设备状态枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    /// 设备未初始化
    Uninitialized,
    /// 设备正在初始化
    Initializing,
    /// 设备空闲，等待工作
    Idle,
    /// 设备正在挖矿
    Mining,
    /// 设备出现错误
    Error(String),
    /// 设备过热
    Overheated,
    /// 设备已禁用
    Disabled,
    /// 设备正在重启
    Restarting,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        DeviceStatus::Uninitialized
    }
}

/// 设备信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub device_type: String,
    pub chain_id: u8,
    pub chip_count: u32,
    pub status: DeviceStatus,
    pub temperature: Option<f32>,
    pub fan_speed: Option<u32>,
    pub voltage: Option<u32>,
    pub frequency: Option<u32>,
    pub hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub uptime: Duration,
    pub last_share_time: Option<SystemTime>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl DeviceInfo {
    pub fn new(id: u32, name: String, device_type: String, chain_id: u8) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            device_type,
            chain_id,
            chip_count: 0,
            status: DeviceStatus::default(),
            temperature: None,
            fan_speed: None,
            voltage: None,
            frequency: None,
            hashrate: 0.0,
            accepted_shares: 0,
            rejected_shares: 0,
            hardware_errors: 0,
            uptime: Duration::from_secs(0),
            last_share_time: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_status(&mut self, status: DeviceStatus) {
        self.status = status;
        self.updated_at = SystemTime::now();
    }

    pub fn update_temperature(&mut self, temperature: f32) {
        self.temperature = Some(temperature);
        self.updated_at = SystemTime::now();
    }

    pub fn update_hashrate(&mut self, hashrate: f64) {
        self.hashrate = hashrate;
        self.updated_at = SystemTime::now();
    }

    pub fn increment_accepted_shares(&mut self) {
        self.accepted_shares += 1;
        self.last_share_time = Some(SystemTime::now());
        self.updated_at = SystemTime::now();
    }

    pub fn increment_rejected_shares(&mut self) {
        self.rejected_shares += 1;
        self.updated_at = SystemTime::now();
    }

    pub fn increment_hardware_errors(&mut self) {
        self.hardware_errors += 1;
        self.updated_at = SystemTime::now();
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, DeviceStatus::Idle | DeviceStatus::Mining)
    }

    pub fn is_overheated(&self) -> bool {
        matches!(self.status, DeviceStatus::Overheated)
    }

    pub fn get_error_rate(&self) -> f64 {
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

/// 工作数据结构
#[derive(Debug, Clone)]
pub struct Work {
    pub id: Uuid,
    pub job_id: String,
    pub target: [u8; 32],
    pub header: [u8; 80],
    pub midstate: [[u8; 32]; 8],
    pub difficulty: f64,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
}

impl Work {
    pub fn new(job_id: String, target: [u8; 32], header: [u8; 80], difficulty: f64) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            job_id,
            target,
            header,
            midstate: [[0u8; 32]; 8],
            difficulty,
            created_at: now,
            expires_at: now + Duration::from_secs(120), // 2分钟过期
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    pub fn time_to_expire(&self) -> Duration {
        self.expires_at.duration_since(SystemTime::now())
            .unwrap_or(Duration::from_secs(0))
    }
}

/// 挖矿结果结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    pub work_id: Uuid,
    pub device_id: u32,
    pub nonce: u32,
    pub extra_nonce: Option<u32>,
    pub timestamp: SystemTime,
    pub difficulty: f64,
    pub is_valid: bool,
}

impl MiningResult {
    pub fn new(work_id: Uuid, device_id: u32, nonce: u32, difficulty: f64) -> Self {
        Self {
            work_id,
            device_id,
            nonce,
            extra_nonce: None,
            timestamp: SystemTime::now(),
            difficulty,
            is_valid: false, // 需要验证后设置
        }
    }

    pub fn with_extra_nonce(mut self, extra_nonce: u32) -> Self {
        self.extra_nonce = Some(extra_nonce);
        self
    }

    pub fn mark_valid(mut self) -> Self {
        self.is_valid = true;
        self
    }
}

/// 设备配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub chain_id: u8,
    pub enabled: bool,
    pub frequency: u32,
    pub voltage: u32,
    pub auto_tune: bool,
    pub chip_count: u32,
    pub temperature_limit: f32,
    pub fan_speed: Option<u32>,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            chain_id: 0,
            enabled: true,
            frequency: 500,
            voltage: 850,
            auto_tune: true,
            chip_count: 76,
            temperature_limit: 85.0,
            fan_speed: None,
        }
    }
}

/// 设备统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceStats {
    pub total_hashes: u64,
    pub valid_nonces: u64,
    pub invalid_nonces: u64,
    pub hardware_errors: u64,
    pub temperature_readings: Vec<f32>,
    pub hashrate_history: Vec<f64>,
    pub uptime_seconds: u64,
    pub restart_count: u32,
    pub last_restart_time: Option<SystemTime>,
}

impl DeviceStats {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn record_hash(&mut self, count: u64) {
        self.total_hashes += count;
    }

    pub fn record_valid_nonce(&mut self) {
        self.valid_nonces += 1;
    }

    pub fn record_invalid_nonce(&mut self) {
        self.invalid_nonces += 1;
    }

    pub fn record_hardware_error(&mut self) {
        self.hardware_errors += 1;
    }

    pub fn record_temperature(&mut self, temp: f32) {
        self.temperature_readings.push(temp);
        // 保持最近100个温度读数
        if self.temperature_readings.len() > 100 {
            self.temperature_readings.remove(0);
        }
    }

    pub fn record_hashrate(&mut self, hashrate: f64) {
        self.hashrate_history.push(hashrate);
        // 保持最近100个算力读数
        if self.hashrate_history.len() > 100 {
            self.hashrate_history.remove(0);
        }
    }

    pub fn record_restart(&mut self) {
        self.restart_count += 1;
        self.last_restart_time = Some(SystemTime::now());
    }

    pub fn get_average_temperature(&self) -> Option<f32> {
        if self.temperature_readings.is_empty() {
            None
        } else {
            let sum: f32 = self.temperature_readings.iter().sum();
            Some(sum / self.temperature_readings.len() as f32)
        }
    }

    pub fn get_average_hashrate(&self) -> Option<f64> {
        if self.hashrate_history.is_empty() {
            None
        } else {
            let sum: f64 = self.hashrate_history.iter().sum();
            Some(sum / self.hashrate_history.len() as f64)
        }
    }

    pub fn get_error_rate(&self) -> f64 {
        let total_nonces = self.valid_nonces + self.invalid_nonces;
        if total_nonces == 0 {
            0.0
        } else {
            self.invalid_nonces as f64 / total_nonces as f64 * 100.0
        }
    }

    pub fn get_hardware_error_rate(&self) -> f64 {
        let total_work = self.valid_nonces + self.invalid_nonces + self.hardware_errors;
        if total_work == 0 {
            0.0
        } else {
            self.hardware_errors as f64 / total_work as f64 * 100.0
        }
    }
}
