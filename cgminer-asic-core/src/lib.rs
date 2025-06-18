//! CGMiner ASIC Core - ASIC挖矿核心
//!
//! 这个库提供ASIC硬件挖矿设备的支持，包括Maijie L7等ASIC矿机的驱动程序。
//! 支持真实的硬件接口，包括SPI、UART、GPIO等底层通信协议。

pub mod core;
pub mod device;
pub mod factory;
pub mod hardware;
pub mod maijie_l7;

// 重新导出主要类型
pub use core::AsicMiningCore;
pub use device::AsicDevice;
pub use factory::AsicCoreFactory;
pub use hardware::{HardwareInterface, MockHardwareInterface};
pub use maijie_l7::{MaijieL7Device, MaijieL7Driver};

use cgminer_core::{CoreType, CoreInfo};

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 获取ASIC核心信息
pub fn get_core_info() -> CoreInfo {
    CoreInfo::new(
        "ASIC Mining Core".to_string(),
        CoreType::Asic,
        VERSION.to_string(),
        "ASIC挖矿核心，支持各种ASIC硬件设备的挖矿操作".to_string(),
        "CGMiner Rust Team".to_string(),
        vec!["asic".to_string(), "maijie-l7".to_string()],
    )
}

/// 创建ASIC核心工厂
pub fn create_factory() -> Box<dyn cgminer_core::CoreFactory> {
    Box::new(AsicCoreFactory::new())
}

// C FFI 导出函数，用于动态加载
#[no_mangle]
pub extern "C" fn cgminer_asic_core_info() -> *const std::os::raw::c_char {
    use std::ffi::CString;

    let info = get_core_info();
    let json = serde_json::to_string(&info).unwrap_or_default();
    let c_string = CString::new(json).unwrap_or_default();

    // 注意：这里返回的指针需要调用者负责释放
    c_string.into_raw()
}

#[no_mangle]
pub extern "C" fn cgminer_asic_create_factory() -> *mut std::os::raw::c_void {
    let factory = create_factory();
    Box::into_raw(Box::new(factory)) as *mut std::os::raw::c_void
}

#[no_mangle]
pub extern "C" fn cgminer_asic_free_string(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}
