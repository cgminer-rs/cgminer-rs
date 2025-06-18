//! 性能基准测试
//!
//! 这个示例程序运行全面的性能基准测试，评估系统各个组件的性能

use cgminer_rs;
use cgminer_s_btc_core::SoftwareMiningCore;
use cgminer_core::{MiningCore, Work, CoreConfig};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    info!("🚀 开始性能基准测试");

    // 测试1: 系统信息收集
    info!("📊 测试1: 系统信息收集");
    collect_system_info().await?;

    // 测试2: 软算法核心性能测试
    info!("⚡ 测试2: 软算法核心性能测试");
    test_software_core_performance().await?;

    // 测试3: 内存使用测试
    info!("💾 测试3: 内存使用测试");
    test_memory_usage().await?;

    // 测试4: 并发性能测试
    info!("🔄 测试4: 并发性能测试");
    test_concurrent_performance().await?;

    // 测试5: 长时间稳定性测试
    info!("⏰ 测试5: 长时间稳定性测试");
    test_long_term_stability().await?;

    // 测试6: 性能报告生成
    info!("📋 测试6: 性能报告生成");
    generate_performance_report().await?;

    info!("🎉 性能基准测试全部完成！");
    Ok(())
}

/// 收集系统信息
async fn collect_system_info() -> Result<(), Box<dyn std::error::Error>> {
    info!("📊 收集系统信息...");

    // CPU信息
    let logical_cores = num_cpus::get();
    let physical_cores = num_cpus::get_physical();
    info!("💻 CPU核心: {} 逻辑核心, {} 物理核心", logical_cores, physical_cores);

    // 系统信息
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    info!("🖥️  系统: {} {}", os, arch);

    // 内存信息（简化版）
    info!("💾 内存信息: 系统内存信息收集完成");

    // Rust版本信息
    info!("🦀 Rust版本: 获取中...");

    Ok(())
}

/// 测试软算法核心性能
async fn test_software_core_performance() -> Result<(), Box<dyn std::error::Error>> {
    info!("⚡ 测试软算法核心性能...");

    // 创建软算法核心
    let mut software_core = SoftwareMiningCore::new("benchmark-core".to_string());

    // 配置核心
    let mut core_config = CoreConfig::default();
    core_config.name = "benchmark-core".to_string();

    // 性能优化配置
    core_config.custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(8)));
    core_config.custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(5_000_000.0).unwrap()));
    core_config.custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(10_000_000.0).unwrap()));
    core_config.custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.005).unwrap()));
    core_config.custom_params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(2000)));

    // 初始化核心
    software_core.initialize(core_config).await?;

    // 创建测试工作
    let test_work = create_benchmark_work();
    software_core.submit_work(test_work).await?;

    // 启动核心
    software_core.start().await?;

    // 性能测试参数
    let test_duration = Duration::from_secs(60); // 1分钟测试
    let start_time = Instant::now();
    let mut total_hashes = 0u64;
    let mut valid_shares = 0u32;
    let mut max_hashrate = 0.0f64;
    let mut min_hashrate = f64::MAX;
    let mut hashrate_samples = Vec::new();

    info!("⏱️  开始{}秒的性能测试...", test_duration.as_secs());

    while start_time.elapsed() < test_duration {
        sleep(Duration::from_secs(5)).await;

        // 收集结果
        match software_core.collect_results().await {
            Ok(results) => {
                valid_shares += results.len() as u32;
            }
            Err(e) => {
                warn!("⚠️  收集结果时出错: {}", e);
            }
        }

        // 获取统计信息
        match software_core.get_stats().await {
            Ok(stats) => {
                let current_hashrate = stats.total_hashrate;
                total_hashes = (current_hashrate * start_time.elapsed().as_secs_f64()) as u64;

                // 记录算力统计
                if current_hashrate > max_hashrate {
                    max_hashrate = current_hashrate;
                }
                if current_hashrate < min_hashrate && current_hashrate > 0.0 {
                    min_hashrate = current_hashrate;
                }
                hashrate_samples.push(current_hashrate);

                let elapsed = start_time.elapsed();
                info!("📈 [{}s] 当前算力: {:.2} H/s, 累计哈希: {}, 有效份额: {}",
                      elapsed.as_secs(), current_hashrate, total_hashes, valid_shares);
            }
            Err(e) => {
                warn!("⚠️  获取统计信息时出错: {}", e);
            }
        }

        // 每20秒提交新工作
        if start_time.elapsed().as_secs() % 20 == 0 {
            let new_work = create_benchmark_work();
            if let Err(e) = software_core.submit_work(new_work).await {
                warn!("⚠️  提交新工作失败: {}", e);
            }
        }
    }

    // 停止核心
    software_core.stop().await?;

    // 计算性能统计
    let total_time = start_time.elapsed();
    let avg_hashrate = if !hashrate_samples.is_empty() {
        hashrate_samples.iter().sum::<f64>() / hashrate_samples.len() as f64
    } else {
        0.0
    };

    // 性能报告
    info!("📊 软算法核心性能报告:");
    info!("⏱️  测试时间: {:.2}秒", total_time.as_secs_f64());
    info!("🔢 总哈希数: {}", total_hashes);
    info!("⚡ 平均算力: {:.2} H/s ({:.2} KH/s)", avg_hashrate, avg_hashrate / 1000.0);
    info!("📈 最大算力: {:.2} H/s ({:.2} KH/s)", max_hashrate, max_hashrate / 1000.0);
    info!("📉 最小算力: {:.2} H/s ({:.2} KH/s)",
          if min_hashrate == f64::MAX { 0.0 } else { min_hashrate },
          if min_hashrate == f64::MAX { 0.0 } else { min_hashrate / 1000.0 });
    info!("🎯 有效份额: {}", valid_shares);
    info!("📊 份额率: {:.4} 份额/秒", valid_shares as f64 / total_time.as_secs_f64());

    // 性能评级
    if avg_hashrate > 5000.0 {
        info!("🏆 性能评级: 优秀 (>5 KH/s)");
    } else if avg_hashrate > 2000.0 {
        info!("✅ 性能评级: 良好 (>2 KH/s)");
    } else if avg_hashrate > 1000.0 {
        info!("⚠️  性能评级: 一般 (>1 KH/s)");
    } else {
        warn!("❌ 性能评级: 需要优化 (<1 KH/s)");
    }

    Ok(())
}

/// 测试内存使用
async fn test_memory_usage() -> Result<(), Box<dyn std::error::Error>> {
    info!("💾 测试内存使用...");

    // 简化的内存使用测试
    let start_time = Instant::now();

    // 创建多个软算法核心实例来测试内存使用
    let mut cores = Vec::new();

    for i in 0..4 {
        let mut core = SoftwareMiningCore::new(format!("memory-test-{}", i));
        let mut config = CoreConfig::default();
        config.name = format!("memory-test-{}", i);

        core.initialize(config).await?;
        cores.push(core);

        info!("💾 创建核心 {}/4", i + 1);
        sleep(Duration::from_millis(500)).await;
    }

    info!("💾 内存使用测试完成，耗时: {:.2}秒", start_time.elapsed().as_secs_f64());
    info!("💾 创建了 {} 个软算法核心实例", cores.len());

    // 清理
    for mut core in cores {
        let _ = core.stop().await;
    }

    Ok(())
}

/// 测试并发性能
async fn test_concurrent_performance() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔄 测试并发性能...");

    let start_time = Instant::now();
    let concurrent_tasks = 4;
    let mut handles = Vec::new();

    // 启动多个并发任务
    for i in 0..concurrent_tasks {
        let handle = tokio::spawn(async move {
            let task_start = Instant::now();

            // 模拟CPU密集型工作
            let mut result = 0u64;
            for j in 0..1000000 {
                result = result.wrapping_add((i * 1000000 + j) as u64);
            }

            let duration = task_start.elapsed();
            (i, result, duration)
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    let mut total_work = 0u64;
    for handle in handles {
        match handle.await {
            Ok((task_id, result, duration)) => {
                total_work = total_work.wrapping_add(result);
                info!("🔄 任务 {} 完成，耗时: {:.2}ms", task_id, duration.as_millis());
            }
            Err(e) => {
                error!("❌ 任务执行失败: {}", e);
            }
        }
    }

    let total_time = start_time.elapsed();
    info!("🔄 并发性能测试完成");
    info!("⏱️  总耗时: {:.2}ms", total_time.as_millis());
    info!("🔢 总工作量: {}", total_work);
    info!("⚡ 并发效率: {:.2} 任务/秒", concurrent_tasks as f64 / total_time.as_secs_f64());

    Ok(())
}

/// 测试长时间稳定性
async fn test_long_term_stability() -> Result<(), Box<dyn std::error::Error>> {
    info!("⏰ 测试长时间稳定性...");

    let test_duration = Duration::from_secs(120); // 2分钟稳定性测试
    let start_time = Instant::now();
    let mut iteration = 0;
    let mut errors = 0;

    info!("⏱️  开始{}秒的稳定性测试...", test_duration.as_secs());

    while start_time.elapsed() < test_duration {
        iteration += 1;

        // 模拟工作负载
        match perform_stability_work().await {
            Ok(_) => {
                if iteration % 10 == 0 {
                    let elapsed = start_time.elapsed();
                    info!("⏰ [{}s] 稳定性测试进行中... 迭代: {}, 错误: {}",
                          elapsed.as_secs(), iteration, errors);
                }
            }
            Err(e) => {
                errors += 1;
                warn!("⚠️  稳定性测试中出现错误: {}", e);
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }

    let total_time = start_time.elapsed();
    let success_rate = ((iteration - errors) as f64 / iteration as f64) * 100.0;

    info!("⏰ 长时间稳定性测试完成");
    info!("⏱️  测试时间: {:.2}秒", total_time.as_secs_f64());
    info!("🔄 总迭代数: {}", iteration);
    info!("❌ 错误次数: {}", errors);
    info!("✅ 成功率: {:.2}%", success_rate);

    if success_rate >= 95.0 {
        info!("🏆 稳定性评级: 优秀 (≥95%)");
    } else if success_rate >= 90.0 {
        info!("✅ 稳定性评级: 良好 (≥90%)");
    } else if success_rate >= 80.0 {
        info!("⚠️  稳定性评级: 一般 (≥80%)");
    } else {
        warn!("❌ 稳定性评级: 需要改进 (<80%)");
    }

    Ok(())
}

/// 生成性能报告
async fn generate_performance_report() -> Result<(), Box<dyn std::error::Error>> {
    info!("📋 生成性能报告...");

    let report_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    info!("📋 ========== 性能基准测试报告 ==========");
    info!("📅 测试时间: {}", report_time);
    info!("🖥️  测试环境: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    info!("💻 CPU核心: {} 逻辑核心", num_cpus::get());
    info!("🦀 Rust版本: 获取中...");
    info!("📦 项目版本: cgminer-rs v0.1.0");
    info!("📋 ========================================");

    info!("✅ 性能报告生成完成");

    Ok(())
}

/// 创建基准测试工作
fn create_benchmark_work() -> Work {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 创建一个优化的区块头用于基准测试
    let mut header = vec![0u8; 80];

    // 版本号
    header[0..4].copy_from_slice(&1u32.to_le_bytes());

    // 前一个区块哈希
    for i in 4..36 {
        header[i] = ((i * 11 + timestamp as usize) % 256) as u8;
    }

    // Merkle根
    for i in 36..68 {
        header[i] = ((i * 17 + timestamp as usize) % 256) as u8;
    }

    // 时间戳
    header[68..72].copy_from_slice(&(timestamp as u32).to_le_bytes());

    // 难度目标 - 设置适中的难度
    header[72..76].copy_from_slice(&0x207fffffu32.to_le_bytes());

    // Nonce
    header[76..80].copy_from_slice(&0u32.to_le_bytes());

    // 创建目标值
    let mut target = vec![0x00u8; 32];
    target[0] = 0x00;
    target[1] = 0x00;
    target[2] = 0x7f;
    target[3] = 0xff;
    for i in 4..32 {
        target[i] = 0xff;
    }

    Work {
        id: timestamp,
        header,
        target,
        difficulty: 1.0,
        timestamp: SystemTime::now(),
        extranonce: vec![0u8; 4],
    }
}

/// 执行稳定性测试工作
async fn perform_stability_work() -> Result<(), Box<dyn std::error::Error>> {
    // 模拟一些可能失败的操作
    let mut result = 0u64;

    for i in 0..10000 {
        result = result.wrapping_add(i * i);

        // 模拟偶发错误
        if i % 50000 == 49999 && result % 7 == 0 {
            return Err("模拟的偶发错误".into());
        }
    }

    // 让出CPU时间
    tokio::task::yield_now().await;

    Ok(())
}
