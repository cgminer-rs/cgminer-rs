//! 美化日志格式化器

use std::fmt;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use chrono::{DateTime, Local};

/// 挖矿专用日志格式化器
pub struct MiningFormatter {
    /// 是否启用彩色输出
    colored: bool,
}

impl MiningFormatter {
    /// 创建新的格式化器
    pub fn new(colored: bool) -> Self {
        Self { colored }
    }
}

/// CGMiner风格的简洁日志格式化器
pub struct CgminerFormatter {
    /// 是否启用彩色输出
    colored: bool,
}

impl CgminerFormatter {
    /// 创建新的 CGMiner 风格格式化器
    pub fn new(colored: bool) -> Self {
        Self { colored }
    }
}

impl<S, N> FormatEvent<S, N> for CgminerFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let level = metadata.level();

        // 获取当前时间，简化格式
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%H:%M:%S");

        // 级别颜色和简化标识
        let (level_str, level_color) = if self.colored {
            match *level {
                tracing::Level::ERROR => ("ERR", "\x1b[31m"), // 红色
                tracing::Level::WARN => ("WRN", "\x1b[33m"),  // 黄色
                tracing::Level::INFO => ("   ", "\x1b[32m"),  // 绿色，不显示INFO
                tracing::Level::DEBUG => ("DBG", "\x1b[36m"), // 青色
                tracing::Level::TRACE => ("TRC", "\x1b[37m"), // 白色
            }
        } else {
            match *level {
                tracing::Level::ERROR => ("ERR", ""),
                tracing::Level::WARN => ("WRN", ""),
                tracing::Level::INFO => ("   ", ""),
                tracing::Level::DEBUG => ("DBG", ""),
                tracing::Level::TRACE => ("TRC", ""),
            }
        };

        let reset = if self.colored { "\x1b[0m" } else { "" };

        // CGMiner风格：[时间] 级别 消息
        write!(writer, "[{}] {}{}{} ", timestamp, level_color, level_str, reset)?;

        // 写入消息
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

impl<S, N> FormatEvent<S, N> for MiningFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let target = metadata.target();
        let level = metadata.level();

        // 获取当前时间
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");

        // 根据目标选择图标和颜色
        let (icon, color) = match target {
            "mining" => ("⛏️", if self.colored { "\x1b[32m" } else { "" }), // 绿色
            "device" => ("🔧", if self.colored { "\x1b[34m" } else { "" }), // 蓝色
            "pool" => ("🌊", if self.colored { "\x1b[36m" } else { "" }),   // 青色
            "system" => ("🖥️", if self.colored { "\x1b[35m" } else { "" }), // 紫色
            "monitoring" => ("📊", if self.colored { "\x1b[33m" } else { "" }), // 黄色
            _ => ("📝", if self.colored { "\x1b[37m" } else { "" }),        // 白色
        };

        // 级别颜色
        let level_color = if self.colored {
            match *level {
                tracing::Level::ERROR => "\x1b[31m", // 红色
                tracing::Level::WARN => "\x1b[33m",  // 黄色
                tracing::Level::INFO => "\x1b[32m",  // 绿色
                tracing::Level::DEBUG => "\x1b[36m", // 青色
                tracing::Level::TRACE => "\x1b[37m", // 白色
            }
        } else {
            ""
        };

        let reset = if self.colored { "\x1b[0m" } else { "" };

        // 写入时间戳
        write!(writer, "{}{}{} ",
               if self.colored { "\x1b[90m" } else { "" }, // 灰色
               timestamp,
               reset)?;

        // 写入级别
        write!(writer, "{}[{:>5}]{} ", level_color, level, reset)?;

        // 写入图标和目标
        write!(writer, "{}{} {}{} ", color, icon, target.to_uppercase(), reset)?;

        // 写入消息
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

/// 格式化算力显示（智能单位自动适配）
pub fn format_hashrate(hashrate: f64) -> String {
    // 处理特殊情况
    if hashrate <= 0.0 {
        return "0.00 H/s".to_string();
    }

    // 智能选择最合适的单位，确保显示值在合理范围内（1-999）
    if hashrate >= 1_000_000_000_000.0 {
        let th_value = hashrate / 1_000_000_000_000.0;
        if th_value >= 100.0 {
            format!("{:.1} TH/s", th_value)
        } else if th_value >= 10.0 {
            format!("{:.2} TH/s", th_value)
        } else {
            format!("{:.3} TH/s", th_value)
        }
    } else if hashrate >= 1_000_000_000.0 {
        let gh_value = hashrate / 1_000_000_000.0;
        if gh_value >= 100.0 {
            format!("{:.1} GH/s", gh_value)
        } else if gh_value >= 10.0 {
            format!("{:.2} GH/s", gh_value)
        } else if gh_value >= 1.0 {
            format!("{:.3} GH/s", gh_value)
        } else {
            // 如果GH值小于1，降级到MH
            let mh_value = hashrate / 1_000_000.0;
            if mh_value >= 100.0 {
                format!("{:.1} MH/s", mh_value)
            } else if mh_value >= 10.0 {
                format!("{:.2} MH/s", mh_value)
            } else {
                format!("{:.3} MH/s", mh_value)
            }
        }
    } else if hashrate >= 1_000_000.0 {
        let mh_value = hashrate / 1_000_000.0;
        if mh_value >= 100.0 {
            format!("{:.1} MH/s", mh_value)
        } else if mh_value >= 10.0 {
            format!("{:.2} MH/s", mh_value)
        } else if mh_value >= 1.0 {
            format!("{:.3} MH/s", mh_value)
        } else {
            // 如果MH值小于1，降级到KH
            let kh_value = hashrate / 1_000.0;
            if kh_value >= 100.0 {
                format!("{:.1} KH/s", kh_value)
            } else if kh_value >= 10.0 {
                format!("{:.2} KH/s", kh_value)
            } else {
                format!("{:.3} KH/s", kh_value)
            }
        }
    } else if hashrate >= 1_000.0 {
        let kh_value = hashrate / 1_000.0;
        if kh_value >= 100.0 {
            format!("{:.1} KH/s", kh_value)
        } else if kh_value >= 10.0 {
            format!("{:.2} KH/s", kh_value)
        } else if kh_value >= 1.0 {
            format!("{:.3} KH/s", kh_value)
        } else {
            // 如果KH值小于1，降级到H
            if hashrate >= 100.0 {
                format!("{:.1} H/s", hashrate)
            } else if hashrate >= 10.0 {
                format!("{:.2} H/s", hashrate)
            } else {
                format!("{:.3} H/s", hashrate)
            }
        }
    } else if hashrate >= 1.0 {
        if hashrate >= 100.0 {
            format!("{:.1} H/s", hashrate)
        } else if hashrate >= 10.0 {
            format!("{:.2} H/s", hashrate)
        } else {
            format!("{:.3} H/s", hashrate)
        }
    } else {
        // 对于非常小的算力值，显示更高精度
        format!("{:.6} H/s", hashrate)
    }
}

/// 格式化温度显示
pub fn format_temperature(temp: f32) -> String {
    let color = if temp > 85.0 {
        "\x1b[31m" // 红色 - 危险
    } else if temp > 75.0 {
        "\x1b[33m" // 黄色 - 警告
    } else if temp > 65.0 {
        "\x1b[32m" // 绿色 - 正常
    } else {
        "\x1b[36m" // 青色 - 冷
    };

    format!("{}🌡️ {:.1}°C\x1b[0m", color, temp)
}

/// 格式化功耗显示
pub fn format_power(power: f64) -> String {
    if power >= 1000.0 {
        format!("⚡ {:.2} kW", power / 1000.0)
    } else {
        format!("⚡ {:.0} W", power)
    }
}

/// 格式化内存使用
pub fn format_memory(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("💾 {:.2} {}", size, UNITS[unit_index])
}

/// 格式化网络流量
pub fn format_network_traffic(bytes: u64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("🌐 {:.2} {}", size, UNITS[unit_index])
}

/// 格式化时间间隔
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("⏱️ {}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("⏱️ {}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("⏱️ {}m {}s", minutes, seconds)
    } else {
        format!("⏱️ {}s", seconds)
    }
}

/// 格式化百分比
pub fn format_percentage(value: f64) -> String {
    let color = if value > 90.0 {
        "\x1b[31m" // 红色
    } else if value > 75.0 {
        "\x1b[33m" // 黄色
    } else {
        "\x1b[32m" // 绿色
    };

    format!("{}📊 {:.1}%\x1b[0m", color, value)
}

/// 格式化状态指示器
pub fn format_status(online: bool) -> String {
    if online {
        "🟢 在线".to_string()
    } else {
        "🔴 离线".to_string()
    }
}

/// 格式化错误率
pub fn format_error_rate(rate: f64) -> String {
    let color = if rate > 5.0 {
        "\x1b[31m" // 红色
    } else if rate > 2.0 {
        "\x1b[33m" // 黄色
    } else {
        "\x1b[32m" // 绿色
    };

    format!("{}❌ {:.2}%\x1b[0m", color, rate)
}

/// 格式化延迟
pub fn format_latency(ms: u32) -> String {
    let color = if ms > 1000 {
        "\x1b[31m" // 红色
    } else if ms > 500 {
        "\x1b[33m" // 黄色
    } else {
        "\x1b[32m" // 绿色
    };

    format!("{}🕐 {} ms\x1b[0m", color, ms)
}

/// 创建分隔线
pub fn create_separator(title: &str, width: usize) -> String {
    let title_len = title.len();
    let padding = if width > title_len + 4 {
        (width - title_len - 4) / 2
    } else {
        0
    };

    let left_padding = "═".repeat(padding);
    let right_padding = "═".repeat(width - title_len - 4 - padding);

    format!("╔{}╡ {} ╞{}╗", left_padding, title, right_padding)
}

/// 创建表格行
pub fn create_table_row(columns: &[&str], widths: &[usize]) -> String {
    let mut row = String::from("║");

    for (i, (column, width)) in columns.iter().zip(widths.iter()).enumerate() {
        if i > 0 {
            row.push_str("│");
        }
        row.push_str(&format!(" {:<width$} ", column, width = width - 2));
    }

    row.push_str("║");
    row
}

/// 格式化挖矿难度
pub fn format_difficulty(difficulty: f64) -> String {
    if difficulty >= 1_000_000_000_000.0 {
        format!("🎯 {:.2}T", difficulty / 1_000_000_000_000.0)
    } else if difficulty >= 1_000_000_000.0 {
        format!("🎯 {:.2}G", difficulty / 1_000_000_000.0)
    } else if difficulty >= 1_000_000.0 {
        format!("🎯 {:.2}M", difficulty / 1_000_000.0)
    } else if difficulty >= 1_000.0 {
        format!("🎯 {:.2}K", difficulty / 1_000.0)
    } else {
        format!("🎯 {:.2}", difficulty)
    }
}
