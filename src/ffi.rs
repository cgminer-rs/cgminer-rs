use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_float, c_double};
use std::ptr;
use crate::mining::MiningManager;
use crate::device::DeviceInfo;
use crate::error::MiningError;
use std::sync::Arc;
use tokio::runtime::Runtime;

// 包含生成的绑定
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// C API 错误码
#[repr(C)]
pub enum CApiError {
    Success = 0,
    InvalidParameter = -1,
    DeviceNotFound = -2,
    InitializationFailed = -3,
    OperationFailed = -4,
    NotImplemented = -5,
}

/// 设备状态结构 (C 兼容)
#[repr(C)]
pub struct CDeviceStatus {
    pub device_id: c_uint,
    pub temperature: c_float,
    pub hashrate: c_double,
    pub power_consumption: c_double,
    pub error_count: c_uint,
    pub is_healthy: c_int, // 0 = false, 1 = true
}

/// 系统状态结构 (C 兼容)
#[repr(C)]
pub struct CSystemStatus {
    pub total_hashrate: c_double,
    pub active_devices: c_uint,
    pub connected_pools: c_uint,
    pub accepted_shares: c_uint,
    pub rejected_shares: c_uint,
    pub uptime_seconds: c_uint,
}

/// 全局运行时和挖矿管理器
static mut RUNTIME: Option<Runtime> = None;
static mut MINING_MANAGER: Option<Arc<MiningManager>> = None;

/// 初始化 CGMiner-RS
/// 
/// # Safety
/// 这个函数必须在使用任何其他 API 之前调用，且只能调用一次
#[no_mangle]
pub unsafe extern "C" fn cgminer_init(config_path: *const c_char) -> c_int {
    // 检查参数
    if config_path.is_null() {
        return CApiError::InvalidParameter as c_int;
    }
    
    // 转换配置路径
    let config_path_str = match CStr::from_ptr(config_path).to_str() {
        Ok(s) => s,
        Err(_) => return CApiError::InvalidParameter as c_int,
    };
    
    // 创建运行时
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return CApiError::InitializationFailed as c_int,
    };
    
    // 在运行时中初始化挖矿管理器
    let mining_manager = match runtime.block_on(async {
        // 加载配置
        let config = match crate::config::Config::load(config_path_str) {
            Ok(cfg) => cfg,
            Err(_) => return Err(MiningError::ConfigError("Failed to load config".to_string())),
        };
        
        // 创建挖矿管理器
        MiningManager::new(config).await
    }) {
        Ok(manager) => Arc::new(manager),
        Err(_) => return CApiError::InitializationFailed as c_int,
    };
    
    // 存储全局引用
    RUNTIME = Some(runtime);
    MINING_MANAGER = Some(mining_manager);
    
    CApiError::Success as c_int
}

/// 启动挖矿
#[no_mangle]
pub unsafe extern "C" fn cgminer_start() -> c_int {
    let runtime = match RUNTIME.as_ref() {
        Some(rt) => rt,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    let mining_manager = match MINING_MANAGER.as_ref() {
        Some(manager) => manager,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    match runtime.block_on(mining_manager.start()) {
        Ok(_) => CApiError::Success as c_int,
        Err(_) => CApiError::OperationFailed as c_int,
    }
}

/// 停止挖矿
#[no_mangle]
pub unsafe extern "C" fn cgminer_stop() -> c_int {
    let runtime = match RUNTIME.as_ref() {
        Some(rt) => rt,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    let mining_manager = match MINING_MANAGER.as_ref() {
        Some(manager) => manager,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    match runtime.block_on(mining_manager.stop()) {
        Ok(_) => CApiError::Success as c_int,
        Err(_) => CApiError::OperationFailed as c_int,
    }
}

/// 获取系统状态
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_system_status(status: *mut CSystemStatus) -> c_int {
    if status.is_null() {
        return CApiError::InvalidParameter as c_int;
    }
    
    let runtime = match RUNTIME.as_ref() {
        Some(rt) => rt,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    let mining_manager = match MINING_MANAGER.as_ref() {
        Some(manager) => manager,
        None => return CApiError::InitializationFailed as c_int,
    };
    
    let system_status = match runtime.block_on(mining_manager.get_system_status()) {
        Ok(status) => status,
        Err(_) => return CApiError::OperationFailed as c_int,
    };
    
    // 填充 C 结构
    (*status).total_hashrate = system_status.total_hashrate;
    (*status).active_devices = system_status.active_devices;
    (*status).connected_pools = system_status.connected_pools;
    (*status).accepted_shares = system_status.accepted_shares;
    (*status).rejected_shares = system_status.rejected_shares;
    (*status).uptime_seconds = system_status.uptime.as_secs() as c_uint;
    
    CApiError::Success as c_int
}

/// 获取设备状态
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_device_status(device_id: c_uint, status: *mut CDeviceStatus) -> c_int {
    if status.is_null() {
        return CApiError::InvalidParameter as c_int;
    }
    
    // 这里应该从设备管理器获取实际的设备状态
    // 为了简化，我们返回模拟数据
    (*status).device_id = device_id;
    (*status).temperature = 65.5;
    (*status).hashrate = 38.0;
    (*status).power_consumption = 1500.0;
    (*status).error_count = 2;
    (*status).is_healthy = 1;
    
    CApiError::Success as c_int
}

/// 重启设备
#[no_mangle]
pub unsafe extern "C" fn cgminer_restart_device(device_id: c_uint) -> c_int {
    // 这里应该调用设备管理器的重启方法
    // 为了简化，我们只是返回成功
    if device_id > 1 {
        return CApiError::DeviceNotFound as c_int;
    }
    
    CApiError::Success as c_int
}

/// 设置设备频率
#[no_mangle]
pub unsafe extern "C" fn cgminer_set_device_frequency(device_id: c_uint, frequency: c_uint) -> c_int {
    if device_id > 1 || frequency < 100 || frequency > 800 {
        return CApiError::InvalidParameter as c_int;
    }
    
    // 这里应该调用设备管理器的设置频率方法
    CApiError::Success as c_int
}

/// 设置设备电压
#[no_mangle]
pub unsafe extern "C" fn cgminer_set_device_voltage(device_id: c_uint, voltage: c_uint) -> c_int {
    if device_id > 1 || voltage < 600 || voltage > 1000 {
        return CApiError::InvalidParameter as c_int;
    }
    
    // 这里应该调用设备管理器的设置电压方法
    CApiError::Success as c_int
}

/// 获取版本信息
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// 获取构建信息
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_build_info() -> *const c_char {
    static BUILD_INFO: &str = concat!(
        env!("CARGO_PKG_VERSION"), " (",
        env!("GIT_HASH"), " on ",
        env!("TARGET_OS"), "-",
        env!("TARGET_ARCH"), ")\0"
    );
    BUILD_INFO.as_ptr() as *const c_char
}

/// 清理资源
#[no_mangle]
pub unsafe extern "C" fn cgminer_cleanup() -> c_int {
    // 停止挖矿管理器
    if let (Some(runtime), Some(mining_manager)) = (RUNTIME.as_ref(), MINING_MANAGER.as_ref()) {
        let _ = runtime.block_on(mining_manager.stop());
    }
    
    // 清理全局引用
    MINING_MANAGER = None;
    RUNTIME = None;
    
    CApiError::Success as c_int
}

/// 错误码转字符串
#[no_mangle]
pub unsafe extern "C" fn cgminer_error_string(error_code: c_int) -> *const c_char {
    let error_str = match error_code {
        0 => "Success\0",
        -1 => "Invalid parameter\0",
        -2 => "Device not found\0",
        -3 => "Initialization failed\0",
        -4 => "Operation failed\0",
        -5 => "Not implemented\0",
        _ => "Unknown error\0",
    };
    
    error_str.as_ptr() as *const c_char
}

/// 日志回调函数类型
pub type LogCallback = unsafe extern "C" fn(level: c_int, message: *const c_char);

/// 设置日志回调
#[no_mangle]
pub unsafe extern "C" fn cgminer_set_log_callback(callback: LogCallback) -> c_int {
    // 这里应该设置日志回调
    // 为了简化，我们只是返回成功
    CApiError::Success as c_int
}

/// 获取设备数量
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_device_count() -> c_uint {
    // 这里应该从设备管理器获取实际的设备数量
    // 为了简化，我们返回固定值
    2
}

/// 获取设备列表
#[no_mangle]
pub unsafe extern "C" fn cgminer_get_device_list(devices: *mut c_uint, max_devices: c_uint) -> c_int {
    if devices.is_null() || max_devices == 0 {
        return CApiError::InvalidParameter as c_int;
    }
    
    // 填充设备ID列表
    let device_count = std::cmp::min(2, max_devices);
    for i in 0..device_count {
        *devices.add(i as usize) = i;
    }
    
    device_count as c_int
}
