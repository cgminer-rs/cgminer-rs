//! 统一的算力格式化工具
//! 
//! 提供智能的算力单位自适应显示功能

/// 格式化算力显示（智能单位自适应）
/// 
/// 根据算力数值大小自动选择最合适的单位：
/// - H/s (哈希每秒)
/// - KH/s (千哈希每秒) 
/// - MH/s (兆哈希每秒)
/// - GH/s (吉哈希每秒)
/// - TH/s (太哈希每秒)
/// - PH/s (拍哈希每秒)
/// 
/// # 参数
/// * `hashrate` - 算力值（H/s）
/// 
/// # 返回值
/// 格式化后的算力字符串，包含合适的单位和精度
/// 
/// # 示例
/// ```
/// use cgminer_rs::utils::hashrate_formatter::format_hashrate;
/// 
/// assert_eq!(format_hashrate(1234.0), "1.234 KH/s");
/// assert_eq!(format_hashrate(1234567.0), "1.235 MH/s");
/// assert_eq!(format_hashrate(1234567890.0), "1.235 GH/s");
/// ```
pub fn format_hashrate(hashrate: f64) -> String {
    // 处理特殊情况
    if hashrate <= 0.0 {
        return "0.00 H/s".to_string();
    }
    
    if !hashrate.is_finite() {
        return "∞ H/s".to_string();
    }

    // 智能选择最合适的单位，确保显示值在合理范围内（1-999）
    if hashrate >= 1_000_000_000_000_000.0 {
        // PH/s (拍哈希每秒)
        let ph_value = hashrate / 1_000_000_000_000_000.0;
        if ph_value >= 100.0 {
            format!("{:.1} PH/s", ph_value)
        } else if ph_value >= 10.0 {
            format!("{:.2} PH/s", ph_value)
        } else {
            format!("{:.3} PH/s", ph_value)
        }
    } else if hashrate >= 1_000_000_000_000.0 {
        // TH/s (太哈希每秒)
        let th_value = hashrate / 1_000_000_000_000.0;
        if th_value >= 100.0 {
            format!("{:.1} TH/s", th_value)
        } else if th_value >= 10.0 {
            format!("{:.2} TH/s", th_value)
        } else {
            format!("{:.3} TH/s", th_value)
        }
    } else if hashrate >= 1_000_000_000.0 {
        // GH/s (吉哈希每秒)
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
        // MH/s (兆哈希每秒)
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
        // KH/s (千哈希每秒)
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
        // H/s (哈希每秒)
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

/// 格式化算力显示（紧凑模式）
/// 
/// 类似于 `format_hashrate`，但使用更紧凑的格式，适合在空间有限的地方显示
/// 
/// # 参数
/// * `hashrate` - 算力值（H/s）
/// 
/// # 返回值
/// 紧凑格式的算力字符串
/// 
/// # 示例
/// ```
/// use cgminer_rs::utils::hashrate_formatter::format_hashrate_compact;
/// 
/// assert_eq!(format_hashrate_compact(1234567890.0), "1.23G");
/// ```
pub fn format_hashrate_compact(hashrate: f64) -> String {
    if hashrate <= 0.0 {
        return "0H".to_string();
    }
    
    if !hashrate.is_finite() {
        return "∞H".to_string();
    }

    if hashrate >= 1_000_000_000_000_000.0 {
        format!("{:.1}P", hashrate / 1_000_000_000_000_000.0)
    } else if hashrate >= 1_000_000_000_000.0 {
        format!("{:.1}T", hashrate / 1_000_000_000_000.0)
    } else if hashrate >= 1_000_000_000.0 {
        format!("{:.1}G", hashrate / 1_000_000_000.0)
    } else if hashrate >= 1_000_000.0 {
        format!("{:.1}M", hashrate / 1_000_000.0)
    } else if hashrate >= 1_000.0 {
        format!("{:.1}K", hashrate / 1_000.0)
    } else {
        format!("{:.0}H", hashrate)
    }
}

/// 解析算力字符串为数值
/// 
/// 支持解析带单位的算力字符串，返回以H/s为单位的数值
/// 
/// # 参数
/// * `hashrate_str` - 算力字符串，如 "1.5 GH/s" 或 "2.3G"
/// 
/// # 返回值
/// 解析后的算力数值（H/s），解析失败返回 None
/// 
/// # 示例
/// ```
/// use cgminer_rs::utils::hashrate_formatter::parse_hashrate;
/// 
/// assert_eq!(parse_hashrate("1.5 GH/s"), Some(1_500_000_000.0));
/// assert_eq!(parse_hashrate("2.3G"), Some(2_300_000_000.0));
/// ```
pub fn parse_hashrate(hashrate_str: &str) -> Option<f64> {
    let s = hashrate_str.trim().to_uppercase();
    
    // 移除 "/S" 后缀
    let s = s.strip_suffix("/S").unwrap_or(&s);
    
    // 查找数字和单位的分界点
    let mut split_pos = 0;
    for (i, c) in s.char_indices() {
        if c.is_alphabetic() {
            split_pos = i;
            break;
        }
    }
    
    if split_pos == 0 {
        // 没有找到单位，假设是H/s
        return s.parse::<f64>().ok();
    }
    
    let (number_part, unit_part) = s.split_at(split_pos);
    let number = number_part.trim().parse::<f64>().ok()?;
    
    let multiplier = match unit_part.trim() {
        "H" => 1.0,
        "KH" | "K" => 1_000.0,
        "MH" | "M" => 1_000_000.0,
        "GH" | "G" => 1_000_000_000.0,
        "TH" | "T" => 1_000_000_000_000.0,
        "PH" | "P" => 1_000_000_000_000_000.0,
        _ => return None,
    };
    
    Some(number * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hashrate() {
        assert_eq!(format_hashrate(0.0), "0.00 H/s");
        assert_eq!(format_hashrate(123.0), "123.000 H/s");
        assert_eq!(format_hashrate(1234.0), "1.234 KH/s");
        assert_eq!(format_hashrate(1234567.0), "1.235 MH/s");
        assert_eq!(format_hashrate(1234567890.0), "1.235 GH/s");
        assert_eq!(format_hashrate(1234567890123.0), "1.235 TH/s");
    }

    #[test]
    fn test_format_hashrate_compact() {
        assert_eq!(format_hashrate_compact(0.0), "0H");
        assert_eq!(format_hashrate_compact(1234567890.0), "1.2G");
        assert_eq!(format_hashrate_compact(1234567890123.0), "1.2T");
    }

    #[test]
    fn test_parse_hashrate() {
        assert_eq!(parse_hashrate("1.5 GH/s"), Some(1_500_000_000.0));
        assert_eq!(parse_hashrate("2.3G"), Some(2_300_000_000.0));
        assert_eq!(parse_hashrate("100 MH/s"), Some(100_000_000.0));
        assert_eq!(parse_hashrate("invalid"), None);
    }
}
