//! 类型转换模块
//!
//! 处理cgminer-core和主程序之间的数据结构转换

use crate::device::{DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult};
use crate::error::DeviceError;
use cgminer_core::{HashRate, Temperature};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

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

/// 将主程序的Work转换为cgminer-core的Work
pub fn convert_device_to_core_work(device_work: Work) -> cgminer_core::Work {
    // 简化的UUID到u64转换，使用时间戳的低64位
    let work_id = device_work.created_at
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos() as u64;

    cgminer_core::Work {
        id: work_id,
        header: device_work.header.to_vec(), // [u8; 80] -> Vec<u8>
        target: device_work.target.to_vec(), // [u8; 32] -> Vec<u8>
        timestamp: device_work.created_at,
        extranonce: Vec::new(), // 默认空
        difficulty: device_work.difficulty,
    }
}

/// 将cgminer-core的Work转换为主程序的Work
pub fn convert_core_to_device_work(core_work: cgminer_core::Work) -> Work {
    // 将Vec<u8>转换为固定大小数组，不足的部分用0填充
    let mut header = [0u8; 80];
    let header_len = core_work.header.len().min(80);
    header[..header_len].copy_from_slice(&core_work.header[..header_len]);

    let mut target = [0u8; 32];
    let target_len = core_work.target.len().min(32);
    target[..target_len].copy_from_slice(&core_work.target[..target_len]);

    Work {
        id: Uuid::new_v4(), // 生成新的UUID
        job_id: format!("job_{}", core_work.id), // 从work_id生成job_id
        target,
        header,
        midstate: [[0u8; 32]; 8], // 默认空midstate
        difficulty: core_work.difficulty,
        created_at: core_work.timestamp,
        expires_at: core_work.timestamp + Duration::from_secs(120), // 2分钟过期
    }
}

/// 将cgminer-core的MiningResult转换为主程序的MiningResult
pub fn convert_core_to_device_result(core_result: cgminer_core::MiningResult) -> MiningResult {
    // 简化的u64到UUID转换
    let work_id = Uuid::from_u128(core_result.work_id as u128);

    // 将extranonce Vec<u8>转换为Option<u32>
    let extra_nonce = if core_result.extranonce.len() >= 4 {
        Some(u32::from_le_bytes([
            core_result.extranonce[0],
            core_result.extranonce[1],
            core_result.extranonce[2],
            core_result.extranonce[3],
        ]))
    } else {
        None
    };

    MiningResult {
        work_id,
        device_id: core_result.device_id,
        nonce: core_result.nonce,
        extra_nonce,
        timestamp: core_result.timestamp,
        difficulty: 1.0, // 默认难度，需要从其他地方获取
        is_valid: core_result.meets_target,
    }
}

/// 将主程序的MiningResult转换为cgminer-core的MiningResult
pub fn convert_device_to_core_result(device_result: MiningResult) -> cgminer_core::MiningResult {
    // UUID到u64的简化转换
    let work_id = device_result.work_id.as_u128() as u64;

    // Option<u32>到Vec<u8>的转换
    let extranonce = if let Some(extra_nonce) = device_result.extra_nonce {
        extra_nonce.to_le_bytes().to_vec()
    } else {
        Vec::new()
    };

    cgminer_core::MiningResult {
        work_id,
        device_id: device_result.device_id,
        nonce: device_result.nonce,
        extranonce,
        timestamp: device_result.timestamp,
        hash: Vec::new(), // 默认空hash
        meets_target: device_result.is_valid,
    }
}

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
