//! 美化日志系统

pub mod formatter;
pub mod hashmeter;
pub mod mining_logger;

use crate::error::MiningError;
use std::path::Path;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use tracing_appender::{non_blocking, rolling};

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志级别
    pub level: String,
    /// 日志文件路径
    pub file_path: Option<String>,
    /// 是否启用彩色输出
    pub colored: bool,
    /// 是否显示时间戳
    pub show_timestamp: bool,
    /// 是否显示线程ID
    pub show_thread_id: bool,
    /// 是否显示目标模块
    pub show_target: bool,
    /// 是否启用美化输出
    pub pretty: bool,
    /// 日志轮转配置
    pub rotation: LogRotation,
}

/// 日志轮转配置
#[derive(Debug, Clone)]
pub enum LogRotation {
    /// 不轮转
    Never,
    /// 每小时轮转
    Hourly,
    /// 每天轮转
    Daily,
    /// 按大小轮转 (MB)
    Size(u64),
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            colored: true,
            show_timestamp: true,
            show_thread_id: false,
            show_target: false,
            pretty: true,
            rotation: LogRotation::Daily,
        }
    }
}

/// 初始化日志系统
pub fn init_logging(config: LogConfig) -> Result<(), MiningError> {
    let level_filter = match config.level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let env_filter = EnvFilter::from_default_env()
        .add_directive(level_filter.into());

    let registry = tracing_subscriber::registry()
        .with(env_filter);

    // 控制台输出层
    let console_layer = if config.pretty {
        fmt::layer()
            .with_ansi(config.colored)
            .with_target(config.show_target)
            .with_thread_ids(config.show_thread_id)
            .with_span_events(FmtSpan::CLOSE)
            .event_format(formatter::MiningFormatter::new(config.colored))
            .boxed()
    } else {
        fmt::layer()
            .with_ansi(config.colored)
            .with_target(config.show_target)
            .with_thread_ids(config.show_thread_id)
            .boxed()
    };

    // 文件输出层
    if let Some(file_path) = config.file_path {
        let file_path = Path::new(&file_path);
        let directory = file_path.parent().unwrap_or(Path::new("."));
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cgminer.log");

        let (non_blocking_appender, _guard) = match config.rotation {
            LogRotation::Never => {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)
                    .map_err(|e| MiningError::System(format!("Failed to open log file: {}", e)))?;

                tracing_appender::non_blocking(file)
            }
            LogRotation::Hourly => {
                let file_appender = rolling::hourly(directory, file_name);
                non_blocking(file_appender)
            }
            LogRotation::Daily => {
                let file_appender = rolling::daily(directory, file_name);
                non_blocking(file_appender)
            }
            LogRotation::Size(_size) => {
                // 简化实现，使用每日轮转
                let file_appender = rolling::daily(directory, file_name);
                non_blocking(file_appender)
            }
        };

        let file_layer = fmt::layer()
            .with_writer(non_blocking_appender)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .json();

        registry
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        registry
            .with(console_layer)
            .init();
    }

    Ok(())
}

/// 挖矿专用日志宏
#[macro_export]
macro_rules! mining_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "mining", $($arg)*)
    };
}

#[macro_export]
macro_rules! mining_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "mining", $($arg)*)
    };
}

#[macro_export]
macro_rules! mining_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "mining", $($arg)*)
    };
}

#[macro_export]
macro_rules! device_info {
    ($device_id:expr, $($arg:tt)*) => {
        tracing::info!(target: "device", device_id = $device_id, $($arg)*)
    };
}

#[macro_export]
macro_rules! device_warn {
    ($device_id:expr, $($arg:tt)*) => {
        tracing::warn!(target: "device", device_id = $device_id, $($arg)*)
    };
}

#[macro_export]
macro_rules! device_error {
    ($device_id:expr, $($arg:tt)*) => {
        tracing::error!(target: "device", device_id = $device_id, $($arg)*)
    };
}

#[macro_export]
macro_rules! pool_info {
    ($pool_id:expr, $($arg:tt)*) => {
        tracing::info!(target: "pool", pool_id = $pool_id, $($arg)*)
    };
}

#[macro_export]
macro_rules! pool_warn {
    ($pool_id:expr, $($arg:tt)*) => {
        tracing::warn!(target: "pool", pool_id = $pool_id, $($arg)*)
    };
}

#[macro_export]
macro_rules! pool_error {
    ($pool_id:expr, $($arg:tt)*) => {
        tracing::error!(target: "pool", pool_id = $pool_id, $($arg)*)
    };
}

/// 算力显示宏
#[macro_export]
macro_rules! hashrate_display {
    ($hashrate:expr) => {
        $crate::logging::hashmeter::format_hashrate($hashrate)
    };
}

/// 温度显示宏
#[macro_export]
macro_rules! temperature_display {
    ($temp:expr) => {
        $crate::logging::formatter::format_temperature($temp)
    };
}

/// 功耗显示宏
#[macro_export]
macro_rules! power_display {
    ($power:expr) => {
        $crate::logging::formatter::format_power($power)
    };
}
