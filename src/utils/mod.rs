//! 实用工具模块
//!
//! 提供各种通用的工具函数和格式化功能

pub mod hashrate_formatter;

// 重新导出常用函数
pub use hashrate_formatter::{format_hashrate, format_hashrate_compact, parse_hashrate};

/// 算力显示宏 - 智能单位自适应
///
/// 使用示例：
/// ```
/// use cgminer_rs::hashrate;
///
/// let rate = 1234567890.0;
/// println!("当前算力: {}", hashrate!(rate));
/// // 输出: 当前算力: 1.235 GH/s
/// ```
#[macro_export]
macro_rules! hashrate {
    ($hashrate:expr) => {
        $crate::utils::format_hashrate($hashrate)
    };
}

/// 紧凑算力显示宏
///
/// 使用示例：
/// ```
/// use cgminer_rs::hashrate_compact;
///
/// let rate = 1234567890.0;
/// println!("算力: {}", hashrate_compact!(rate));
/// // 输出: 算力: 1.2G
/// ```
#[macro_export]
macro_rules! hashrate_compact {
    ($hashrate:expr) => {
        $crate::utils::format_hashrate_compact($hashrate)
    };
}
