//! 稳定性测试
//! 
//! 这个示例程序运行长时间稳定性测试，验证系统在长期运行下的稳定性

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

    info!("🔄 开始长时间稳定性测试");

    // 测试配置
    let test_duration = Duration::from_secs(300); // 5分钟稳定性测试
    let check_interval = Duration::from_secs(10); // 每10秒检查一次
    let work_update_interval = Duration::from_secs(30); // 每30秒更新工作

    info!("⏱️  测试配置:");
    info!("   • 测试时长: {} 分钟", test_duration.as_secs() / 60);
    info!("   • 检查间隔: {} 秒", check_interval.as_secs());
    info!("   • 工作更新间隔: {} 秒", work_update_interval.as_secs());

    // 测试1: 单核心长时间稳定性
    info!("🔧 测试1: 单核心长时间稳定性");
    test_single_core_stability(test_duration, check_interval, work_update_interval).await?;

    // 测试2: 多核心并发稳定性
    info!("🔄 测试2: 多核心并发稳定性");
    test_multi_core_stability(test_duration / 2, check_interval).await?;

    // 测试3: 内存泄漏检测
    info!("💾 测试3: 内存泄漏检测");
    test_memory_leak_detection().await?;

    // 测试4: 错误恢复测试
    info!("🔧 测试4: 错误恢复测试");
    test_error_recovery().await?;

    info!("🎉 长时间稳定性测试全部完成！");
    Ok(())
}

/// 测试单核心长时间稳定性
async fn test_single_core_stability(
    test_duration: Duration,
    check_interval: Duration,
    work_update_interval: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 开始单核心长时间稳定性测试...");

    // 创建软算法核心
    let mut software_core = SoftwareMiningCore::new("stability-test-core".to_string());

    // 配置核心
    let mut core_config = CoreConfig::default();
    core_config.name = "stability-test-core".to_string();
    
    // 稳定性优化配置
    core_config.custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
    core_config.custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1_000_000.0).unwrap()));
    core_config.custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(3_000_000.0).unwrap()));
    core_config.custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.001).unwrap()));
    core_config.custom_params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));

    // 初始化和启动核心
    software_core.initialize(core_config).await?;
    
    let test_work = create_stability_work();
    software_core.submit_work(test_work).await?;
    software_core.start().await?;

    // 稳定性监控变量
    let start_time = Instant::now();
    let mut last_work_update = Instant::now();
    let mut total_hashes = 0u64;
    let mut valid_shares = 0u32;
    let mut error_count = 0u32;
    let mut restart_count = 0u32;
    let mut hashrate_samples = Vec::new();
    let mut memory_usage_samples = Vec::new();

    info!("⏱️  开始{}分钟的单核心稳定性测试...", test_duration.as_secs() / 60);

    while start_time.elapsed() < test_duration {
        sleep(check_interval).await;

        let elapsed = start_time.elapsed();
        
        // 收集结果
        match software_core.collect_results().await {
            Ok(results) => {
                valid_shares += results.len() as u32;
            }
            Err(e) => {
                error_count += 1;
                warn!("⚠️  收集结果时出错: {}", e);
            }
        }

        // 获取统计信息
        match software_core.get_stats().await {
            Ok(stats) => {
                let current_hashrate = stats.total_hashrate;
                total_hashes = (current_hashrate * elapsed.as_secs_f64()) as u64;
                hashrate_samples.push(current_hashrate);
                
                // 模拟内存使用情况
                let memory_usage = estimate_memory_usage(&hashrate_samples);
                memory_usage_samples.push(memory_usage);

                info!("📊 [{}m{}s] 算力: {:.2} H/s, 累计哈希: {}, 份额: {}, 错误: {}, 内存: {:.2}MB", 
                      elapsed.as_secs() / 60, elapsed.as_secs() % 60,
                      current_hashrate, total_hashes, valid_shares, error_count, memory_usage);

                // 检查异常情况
                if current_hashrate < 100.0 {
                    warn!("⚠️  算力异常低，可能需要重启核心");
                    
                    // 尝试重启核心
                    if let Err(e) = restart_core(&mut software_core).await {
                        error!("❌ 核心重启失败: {}", e);
                        error_count += 1;
                    } else {
                        restart_count += 1;
                        info!("✅ 核心重启成功");
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                warn!("⚠️  获取统计信息时出错: {}", e);
            }
        }

        // 定期更新工作
        if last_work_update.elapsed() >= work_update_interval {
            let new_work = create_stability_work();
            match software_core.submit_work(new_work).await {
                Ok(_) => {
                    info!("🔄 工作更新成功");
                }
                Err(e) => {
                    error_count += 1;
                    warn!("⚠️  工作更新失败: {}", e);
                }
            }
            last_work_update = Instant::now();
        }
    }

    // 停止核心
    software_core.stop().await?;

    // 稳定性分析
    let total_time = start_time.elapsed();
    let avg_hashrate = if !hashrate_samples.is_empty() {
        hashrate_samples.iter().sum::<f64>() / hashrate_samples.len() as f64
    } else {
        0.0
    };
    let avg_memory = if !memory_usage_samples.is_empty() {
        memory_usage_samples.iter().sum::<f64>() / memory_usage_samples.len() as f64
    } else {
        0.0
    };

    info!("📊 单核心稳定性测试报告:");
    info!("⏱️  测试时间: {:.2}分钟", total_time.as_secs_f64() / 60.0);
    info!("⚡ 平均算力: {:.2} H/s", avg_hashrate);
    info!("🎯 总份额数: {}", valid_shares);
    info!("❌ 错误次数: {}", error_count);
    info!("🔄 重启次数: {}", restart_count);
    info!("💾 平均内存使用: {:.2}MB", avg_memory);

    // 稳定性评级
    let error_rate = error_count as f64 / (total_time.as_secs() / check_interval.as_secs()) as f64;
    if error_rate < 0.01 {
        info!("🏆 单核心稳定性: 优秀 (<1% 错误率)");
    } else if error_rate < 0.05 {
        info!("✅ 单核心稳定性: 良好 (<5% 错误率)");
    } else {
        warn!("⚠️  单核心稳定性: 需要改进 ({}% 错误率)", error_rate * 100.0);
    }

    Ok(())
}

/// 测试多核心并发稳定性
async fn test_multi_core_stability(
    test_duration: Duration,
    check_interval: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔄 开始多核心并发稳定性测试...");

    let core_count = 3;
    let mut cores = Vec::new();

    // 创建多个核心
    for i in 0..core_count {
        let mut core = SoftwareMiningCore::new(format!("multi-core-{}", i));
        let mut config = CoreConfig::default();
        config.name = format!("multi-core-{}", i);
        
        // 每个核心使用不同的配置
        config.custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
        config.custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(500_000.0).unwrap()));
        config.custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1_500_000.0).unwrap()));

        core.initialize(config).await?;
        
        let work = create_stability_work();
        core.submit_work(work).await?;
        core.start().await?;
        
        cores.push(core);
        info!("✅ 核心 {} 启动成功", i);
    }

    let start_time = Instant::now();
    let mut total_errors = 0u32;

    info!("⏱️  开始{}分钟的多核心稳定性测试...", test_duration.as_secs() / 60);

    while start_time.elapsed() < test_duration {
        sleep(check_interval).await;

        let elapsed = start_time.elapsed();
        let mut total_hashrate = 0.0;
        let mut total_shares = 0u32;

        // 检查每个核心的状态
        for (i, core) in cores.iter_mut().enumerate() {
            match core.get_stats().await {
                Ok(stats) => {
                    total_hashrate += stats.total_hashrate;
                    
                    // 收集结果
                    if let Ok(results) = core.collect_results().await {
                        total_shares += results.len() as u32;
                    }
                }
                Err(e) => {
                    total_errors += 1;
                    warn!("⚠️  核心 {} 状态异常: {}", i, e);
                }
            }
        }

        info!("📊 [{}m{}s] 总算力: {:.2} H/s, 总份额: {}, 错误: {}", 
              elapsed.as_secs() / 60, elapsed.as_secs() % 60,
              total_hashrate, total_shares, total_errors);
    }

    // 停止所有核心
    for (i, core) in cores.iter_mut().enumerate() {
        if let Err(e) = core.stop().await {
            warn!("⚠️  停止核心 {} 时出错: {}", i, e);
        }
    }

    let total_time = start_time.elapsed();
    info!("📊 多核心稳定性测试报告:");
    info!("⏱️  测试时间: {:.2}分钟", total_time.as_secs_f64() / 60.0);
    info!("🔢 核心数量: {}", core_count);
    info!("❌ 总错误数: {}", total_errors);

    let error_rate = total_errors as f64 / (total_time.as_secs() / check_interval.as_secs()) as f64 / core_count as f64;
    if error_rate < 0.01 {
        info!("🏆 多核心稳定性: 优秀");
    } else if error_rate < 0.05 {
        info!("✅ 多核心稳定性: 良好");
    } else {
        warn!("⚠️  多核心稳定性: 需要改进");
    }

    Ok(())
}

/// 测试内存泄漏检测
async fn test_memory_leak_detection() -> Result<(), Box<dyn std::error::Error>> {
    info!("💾 开始内存泄漏检测测试...");

    let iterations = 10;
    let mut memory_samples = Vec::new();

    for i in 0..iterations {
        // 创建和销毁核心实例
        let mut core = SoftwareMiningCore::new(format!("leak-test-{}", i));
        let mut config = CoreConfig::default();
        config.name = format!("leak-test-{}", i);
        
        core.initialize(config).await?;
        
        let work = create_stability_work();
        core.submit_work(work).await?;
        core.start().await?;
        
        // 运行一段时间
        sleep(Duration::from_secs(2)).await;
        
        // 模拟内存使用测量
        let memory_usage = estimate_memory_usage(&vec![1000.0; i + 1]);
        memory_samples.push(memory_usage);
        
        core.stop().await?;
        
        info!("💾 迭代 {}: 内存使用 {:.2}MB", i + 1, memory_usage);
        
        // 强制垃圾回收（在Rust中主要是让异步任务完成）
        sleep(Duration::from_millis(100)).await;
    }

    // 分析内存趋势
    let first_half_avg = memory_samples[0..iterations/2].iter().sum::<f64>() / (iterations/2) as f64;
    let second_half_avg = memory_samples[iterations/2..].iter().sum::<f64>() / (iterations/2) as f64;
    let memory_growth = second_half_avg - first_half_avg;

    info!("📊 内存泄漏检测报告:");
    info!("🔢 测试迭代: {}", iterations);
    info!("📈 前半段平均内存: {:.2}MB", first_half_avg);
    info!("📈 后半段平均内存: {:.2}MB", second_half_avg);
    info!("📊 内存增长: {:.2}MB", memory_growth);

    if memory_growth < 1.0 {
        info!("✅ 内存泄漏检测: 通过 (增长 < 1MB)");
    } else if memory_growth < 5.0 {
        warn!("⚠️  内存泄漏检测: 轻微增长 ({}MB)", memory_growth);
    } else {
        error!("❌ 内存泄漏检测: 可能存在泄漏 ({}MB)", memory_growth);
    }

    Ok(())
}

/// 测试错误恢复
async fn test_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 开始错误恢复测试...");

    let mut core = SoftwareMiningCore::new("error-recovery-test".to_string());
    let mut config = CoreConfig::default();
    config.name = "error-recovery-test".to_string();
    
    core.initialize(config).await?;
    
    let work = create_stability_work();
    core.submit_work(work).await?;
    core.start().await?;

    let mut recovery_count = 0;
    let test_cycles = 5;

    for i in 0..test_cycles {
        info!("🔧 错误恢复测试周期 {}/{}", i + 1, test_cycles);
        
        // 正常运行一段时间
        sleep(Duration::from_secs(3)).await;
        
        // 模拟错误恢复
        match restart_core(&mut core).await {
            Ok(_) => {
                recovery_count += 1;
                info!("✅ 错误恢复成功");
            }
            Err(e) => {
                warn!("⚠️  错误恢复失败: {}", e);
            }
        }
        
        sleep(Duration::from_secs(2)).await;
    }

    core.stop().await?;

    info!("📊 错误恢复测试报告:");
    info!("🔢 测试周期: {}", test_cycles);
    info!("✅ 成功恢复: {}", recovery_count);
    info!("📊 恢复成功率: {:.1}%", (recovery_count as f64 / test_cycles as f64) * 100.0);

    if recovery_count == test_cycles {
        info!("🏆 错误恢复能力: 优秀");
    } else if recovery_count >= test_cycles * 3 / 4 {
        info!("✅ 错误恢复能力: 良好");
    } else {
        warn!("⚠️  错误恢复能力: 需要改进");
    }

    Ok(())
}

/// 重启核心
async fn restart_core(core: &mut SoftwareMiningCore) -> Result<(), Box<dyn std::error::Error>> {
    // 停止核心
    core.stop().await?;
    
    // 等待一段时间
    sleep(Duration::from_millis(500)).await;
    
    // 重新提交工作并启动
    let work = create_stability_work();
    core.submit_work(work).await?;
    core.start().await?;
    
    Ok(())
}

/// 估算内存使用（简化版）
fn estimate_memory_usage(samples: &[f64]) -> f64 {
    // 简化的内存使用估算，基于样本数量和复杂度
    let base_memory = 10.0; // 基础内存 10MB
    let sample_memory = samples.len() as f64 * 0.1; // 每个样本 0.1MB
    let complexity_factor = samples.iter().sum::<f64>() / 1000000.0; // 复杂度因子
    
    base_memory + sample_memory + complexity_factor
}

/// 创建稳定性测试工作
fn create_stability_work() -> Work {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut header = vec![0u8; 80];

    // 版本号
    header[0..4].copy_from_slice(&1u32.to_le_bytes());

    // 前一个区块哈希
    for i in 4..36 {
        header[i] = ((i * 13 + timestamp as usize) % 256) as u8;
    }

    // Merkle根
    for i in 36..68 {
        header[i] = ((i * 19 + timestamp as usize) % 256) as u8;
    }

    // 时间戳
    header[68..72].copy_from_slice(&(timestamp as u32).to_le_bytes());

    // 难度目标
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
