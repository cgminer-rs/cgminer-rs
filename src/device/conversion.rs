//! 类型转换模块
//!
//! 处理cgminer-core和主程序之间的数据结构转换

use crate::device::{DeviceInfo, DeviceStatus, DeviceStats};
use crate::error::DeviceError;
use cgminer_core::{HashRate, Temperature};
use std::time::Duration;

/// 将cgminer-core的DeviceInfo转换为主程序的DeviceInfo
pub fn convert_core_to_device_info(core_info: cgminer_core::DeviceInfo) -> DeviceInfo {
    DeviceInfo {
        id: core_info.id,
        name: core_info.name,
        device_type: core_info.device_type,
        chain_id: core_info.chain_id,
        chip_count: core_info.chip_count.unwrap_or(0), // Option<u32> -> u32
        status: DeviceStatus::Uninitialized, // 默认状态，需要单独获取
        temperature: core_info.temperature,
        fan_speed: core_info.fan_speed,
        voltage: core_info.voltage,
        frequency: core_info.frequency,
        hashrate: 0.0, // 默认值，需要从stats获取
        accepted_shares: 0, // 默认值，需要从stats获取
        rejected_shares: 0, // 默认值，需要从stats获取
        hardware_errors: 0, // 默认值，需要从stats获取
        uptime: Duration::from_secs(0), // 默认值，需要从stats获取
        last_share_time: None, // 默认值
        created_at: core_info.created_at,
        updated_at: core_info.updated_at,
    }
}

/// 将主程序的DeviceInfo转换为cgminer-core的DeviceInfo
pub fn convert_device_to_core_info(device_info: DeviceInfo) -> cgminer_core::DeviceInfo {
    cgminer_core::DeviceInfo {
        id: device_info.id,
        name: device_info.name,
        device_type: device_info.device_type,
        chain_id: device_info.chain_id,
        device_path: None,
        serial_number: None,
        firmware_version: None,
        hardware_version: None,
        chip_count: Some(device_info.chip_count), // u32 -> Option<u32>
        temperature: device_info.temperature,
        voltage: device_info.voltage,
        frequency: device_info.frequency,
        fan_speed: device_info.fan_speed,
        created_at: device_info.created_at,
        updated_at: device_info.updated_at,
    }
}

/// 将cgminer-core的DeviceStatus转换为主程序的DeviceStatus
pub fn convert_core_to_device_status(core_status: cgminer_core::DeviceStatus) -> DeviceStatus {
    match core_status {
        cgminer_core::DeviceStatus::Uninitialized => DeviceStatus::Uninitialized,
        cgminer_core::DeviceStatus::Initializing => DeviceStatus::Initializing,
        cgminer_core::DeviceStatus::Idle => DeviceStatus::Idle,
        cgminer_core::DeviceStatus::Running => DeviceStatus::Mining, // 映射Running到Mining
        cgminer_core::DeviceStatus::Paused => DeviceStatus::Idle, // 映射Paused到Idle
        cgminer_core::DeviceStatus::Error(msg) => DeviceStatus::Error(msg),
        cgminer_core::DeviceStatus::Offline => DeviceStatus::Disabled, // 映射Offline到Disabled
    }
}

/// 将主程序的DeviceStatus转换为cgminer-core的DeviceStatus
pub fn convert_device_to_core_status(device_status: DeviceStatus) -> cgminer_core::DeviceStatus {
    match device_status {
        DeviceStatus::Uninitialized => cgminer_core::DeviceStatus::Uninitialized,
        DeviceStatus::Initializing => cgminer_core::DeviceStatus::Initializing,
        DeviceStatus::Idle => cgminer_core::DeviceStatus::Idle,
        DeviceStatus::Mining => cgminer_core::DeviceStatus::Running, // 映射Mining到Running
        DeviceStatus::Error(msg) => cgminer_core::DeviceStatus::Error(msg),
        DeviceStatus::Overheated => cgminer_core::DeviceStatus::Error("Overheated".to_string()),
        DeviceStatus::Disabled => cgminer_core::DeviceStatus::Offline,
        DeviceStatus::Restarting => cgminer_core::DeviceStatus::Initializing,
    }
}

// Work和MiningResult现在已经统一，不再需要转换函数

/// 将cgminer-core的DeviceStats转换为主程序的DeviceStats
pub fn convert_core_to_device_stats(core_stats: cgminer_core::DeviceStats) -> DeviceStats {
    DeviceStats {
        total_hashes: core_stats.accepted_work + core_stats.rejected_work, // 近似计算
        valid_nonces: core_stats.accepted_work,
        invalid_nonces: core_stats.rejected_work,
        hardware_errors: core_stats.hardware_errors,
        temperature_readings: if let Some(temp) = core_stats.temperature {
            vec![temp.celsius]
        } else {
            Vec::new()
        },
        hashrate_history: vec![core_stats.current_hashrate.hashes_per_second],
        uptime_seconds: core_stats.uptime.as_secs(),
        restart_count: 0, // 默认值
        last_restart_time: None, // 默认值
    }
}

/// 将cgminer-core的DeviceError转换为主程序的DeviceError
pub fn convert_core_to_device_error(core_error: cgminer_core::DeviceError) -> DeviceError {
    match core_error {
        cgminer_core::DeviceError::NotFound { device_id } =>
            DeviceError::NotFound { device_id },
        cgminer_core::DeviceError::InitializationFailed { message } =>
            DeviceError::InitializationFailed { device_id: 0, reason: message },
        cgminer_core::DeviceError::CommunicationError { message } =>
            DeviceError::CommunicationError { device_id: 0, error: message },
        cgminer_core::DeviceError::Overheating { current, limit: _ } =>
            DeviceError::Overheated { device_id: 0, temperature: current },
        cgminer_core::DeviceError::HardwareError { message: _ } =>
            DeviceError::HardwareError { device_id: 0, error_code: 0 },
        cgminer_core::DeviceError::Timeout { operation: _ } =>
            DeviceError::Timeout { device_id: 0 },
        _ => DeviceError::CommunicationError { device_id: 0, error: "Unknown core error".to_string() },
    }
}

/// 将主程序的DeviceError转换为cgminer-core的DeviceError
pub fn convert_device_to_core_error(device_error: DeviceError) -> cgminer_core::DeviceError {
    match device_error {
        DeviceError::NotFound { device_id } =>
            cgminer_core::DeviceError::NotFound { device_id },
        DeviceError::InitializationFailed { device_id: _, reason } =>
            cgminer_core::DeviceError::InitializationFailed { message: reason },
        DeviceError::CommunicationError { device_id: _, error } =>
            cgminer_core::DeviceError::CommunicationError { message: error },
        DeviceError::Overheated { device_id: _, temperature } =>
            cgminer_core::DeviceError::Overheating { current: temperature, limit: 85.0 },
        DeviceError::HardwareError { device_id: _, error_code: _ } =>
            cgminer_core::DeviceError::HardwareError { message: "Hardware error".to_string() },
        DeviceError::Timeout { device_id: _ } =>
            cgminer_core::DeviceError::Timeout { operation: "Unknown operation".to_string() },
        _ => cgminer_core::DeviceError::CommunicationError { message: "Unknown device error".to_string() },
    }
}

/// 辅助函数：从HashRate提取f64值
pub fn hashrate_to_f64(hashrate: HashRate) -> f64 {
    hashrate.hashes_per_second
}

/// 辅助函数：从f64创建HashRate
pub fn f64_to_hashrate(value: f64) -> HashRate {
    HashRate::new(value)
}

/// 辅助函数：从Temperature提取f32值
pub fn temperature_to_f32(temperature: Temperature) -> f32 {
    temperature.celsius
}

/// 辅助函数：从f32创建Temperature
pub fn f32_to_temperature(value: f32) -> Temperature {
    Temperature::new(value)
}
