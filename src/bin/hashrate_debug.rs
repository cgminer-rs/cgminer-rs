//! 算力计算调试工具
//! 用于测试和调试算力计算逻辑

use cgminer_core::DeviceStats;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🔧 算力计算调试工具");
    println!("==================");

    // 测试1: 基本算力计算
    test_basic_hashrate_calculation().await;

    // 测试2: 滑动窗口算法
    test_rolling_window_algorithm().await;

    // 测试3: 单位格式化
    test_hashrate_formatting().await;

    // 测试4: 边界条件
    test_edge_cases().await;

    println!("\n✅ 所有测试完成");
    Ok(())
}

/// 测试基本算力计算
async fn test_basic_hashrate_calculation() {
    println!("\n📊 测试1: 基本算力计算");
    println!("{}", "-".repeat(30));

    let mut stats = DeviceStats::new(1);

    // 模拟不同的哈希次数和时间间隔
    let test_cases = vec![
        (1000u64, 1.0f64),      // 1000 H/s
        (5000u64, 1.0f64),      // 5000 H/s
        (1000000u64, 1.0f64),   // 1 MH/s
        (1000000000u64, 1.0f64), // 1 GH/s
        (1000u64, 0.1f64),      // 10000 H/s (短时间)
        (100u64, 0.001f64),     // 100000 H/s (极短时间)
    ];

    for (i, (hashes, time_diff)) in test_cases.iter().enumerate() {
        stats.update_hashrate(*hashes, *time_diff);

        let expected = *hashes as f64 / *time_diff;
        let actual = stats.current_hashrate.hashes_per_second;

        println!("  测试 {}: {} 哈希 / {:.3}s", i + 1, hashes, time_diff);
        println!("    期望算力: {:.2} H/s", expected);
        println!("    实际算力: {:.2} H/s", actual);
        println!("    格式化显示: {}", format_hashrate_auto(actual));
        println!("    平均算力: {:.2} H/s", stats.average_hashrate.hashes_per_second);
        println!();
    }
}

/// 测试滑动窗口算法
async fn test_rolling_window_algorithm() {
    println!("📈 测试2: 滑动窗口算法");
    println!("{}", "-".repeat(30));

    let mut stats = DeviceStats::new(2);

    // 模拟连续的挖矿操作
    let base_hashrate = 1_000_000.0; // 1 MH/s

    for i in 1..=10 {
        let hashes = (base_hashrate * 0.1) as u64; // 每次0.1秒的哈希
        let time_diff = 0.1;

        stats.update_hashrate(hashes, time_diff);

        println!("  更新 {}: {:.2} H/s", i, stats.current_hashrate.hashes_per_second);
        println!("    1分钟: {:.2} H/s", stats.hashrate_1m.hashes_per_second);
        println!("    5分钟: {:.2} H/s", stats.hashrate_5m.hashes_per_second);
        println!("    15分钟: {:.2} H/s", stats.hashrate_15m.hashes_per_second);
        println!();

        // 短暂延迟模拟真实情况
        sleep(Duration::from_millis(10)).await;
    }
}

/// 测试单位格式化
async fn test_hashrate_formatting() {
    println!("📝 测试3: 单位格式化");
    println!("{}", "-".repeat(30));

    let test_values = vec![
        0.0,
        0.5,
        1.0,
        999.0,
        1_000.0,
        999_999.0,
        1_000_000.0,
        999_999_999.0,
        1_000_000_000.0,
        999_999_999_999.0,
        1_000_000_000_000.0,
        5_500_000_000_000.0,
    ];

    for value in test_values {
        println!("  {:.0} H/s -> {}", value, format_hashrate_auto(value));
    }
}

/// 测试边界条件
async fn test_edge_cases() {
    println!("⚠️  测试4: 边界条件");
    println!("{}", "-".repeat(30));

    let mut stats = DeviceStats::new(3);

    // 测试极小时间差
    println!("  测试极小时间差:");
    stats.update_hashrate(1000, 0.0001);
    println!("    0.0001s: {:.2} H/s", stats.current_hashrate.hashes_per_second);

    // 测试零时间差
    println!("  测试零时间差:");
    stats.update_hashrate(1000, 0.0);
    println!("    0.0s: {:.2} H/s", stats.current_hashrate.hashes_per_second);

    // 测试负时间差
    println!("  测试负时间差:");
    stats.update_hashrate(1000, -1.0);
    println!("    -1.0s: {:.2} H/s", stats.current_hashrate.hashes_per_second);

    // 测试零哈希
    println!("  测试零哈希:");
    stats.update_hashrate(0, 1.0);
    println!("    0 哈希: {:.2} H/s", stats.current_hashrate.hashes_per_second);
}

/// 自动选择最合适的单位进行格式化（智能单位适配）
fn format_hashrate_auto(hashrate: f64) -> String {
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
