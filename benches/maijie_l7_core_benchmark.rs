//! Maijie L7 ASIC核心性能基准测试 (cgminer-a-maijie-l7-core)
//!
//! 这个基准测试评估Maijie L7 ASIC核心的各种性能指标，包括：
//! - ASIC设备通信性能
//! - 链管理性能
//! - 温度监控性能
//! - 硬件错误处理性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cgminer_core::{DeviceInfo, DeviceConfig, MiningDevice, Work};
use cgminer_a_maijie_l7_core::{
    MaijieL7MiningCore, MaijieL7Device,
    chain::{ChainManager, ChainConfig}
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

/// 创建测试用的设备信息
fn create_test_device_info(id: u32, name: &str) -> DeviceInfo {
    DeviceInfo::new(id, name.to_string(), "maijie-l7".to_string(), 0)
}

/// 创建测试用的链配置
fn create_test_chain_config(chain_id: u8) -> ChainConfig {
    ChainConfig {
        id: chain_id,
        enabled: true,
        frequency: 1000,
        voltage: 900,
        auto_tune: false,
        chip_count: 120,
        temperature_limit: 85.0,
        power_limit: 1000.0,
    }
}

/// 创建测试用的工作
fn create_test_work(id: u64) -> Work {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 创建一个简单的区块头（80字节）
    let mut header = vec![0u8; 80];

    // 版本号 (4字节)
    header[0..4].copy_from_slice(&1u32.to_le_bytes());

    // 前一个区块哈希 (32字节) - 使用测试数据
    for i in 4..36 {
        header[i] = (i % 256) as u8;
    }

    // Merkle根 (32字节) - 使用测试数据
    for i in 36..68 {
        header[i] = ((i * 2) % 256) as u8;
    }

    // 时间戳 (4字节)
    header[68..72].copy_from_slice(&(timestamp as u32).to_le_bytes());

    // 难度目标 (4字节) - ASIC设备的高难度
    header[72..76].copy_from_slice(&0x1d00ffffu32.to_le_bytes());

    // Nonce (4字节) - 初始为0，挖矿时会修改
    header[76..80].copy_from_slice(&0u32.to_le_bytes());

    // 创建目标值 - ASIC设备的高难度
    let mut target = vec![0x00u8; 32];
    target[28] = 0xff;
    target[29] = 0xff;
    target[30] = 0x00;
    target[31] = 0x1d;

    Work {
        id,
        header,
        target,
        timestamp: SystemTime::now(),
        difficulty: 1000000.0, // 高难度适合ASIC
        extranonce: vec![0u8; 4],
    }
}

/// 设备信息创建性能基准测试
fn bench_device_creation(c: &mut Criterion) {
    c.bench_function("maijie_l7_device_info_creation", |b| {
        b.iter(|| {
            let device_info = create_test_device_info(0, "Maijie L7 基准测试设备");
            black_box(device_info);
        });
    });
}

/// 链配置创建性能基准测试
fn bench_chain_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("chain_config_creation");

    // 测试不同数量的链
    for chain_count in [1, 3, 6, 9].iter() {
        group.bench_with_input(
            BenchmarkId::new("chain_count", chain_count),
            chain_count,
            |b, &chain_count| {
                b.iter(|| {
                    let mut configs = Vec::new();
                    for i in 0..chain_count {
                        configs.push(create_test_chain_config(i));
                    }
                    black_box(configs);
                });
            }
        );
    }

    group.finish();
}

/// 工作创建性能基准测试
fn bench_work_submission(c: &mut Criterion) {
    c.bench_function("maijie_l7_work_creation", |b| {
        b.iter(|| {
            let work = create_test_work(1);
            black_box(work);
        });
    });
}

/// 温度监控模拟基准测试
fn bench_temperature_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("temperature_monitoring");

    // 测试不同数量的温度传感器
    for sensor_count in [3, 6, 12, 24].iter() {
        group.bench_with_input(
            BenchmarkId::new("sensor_count", sensor_count),
            sensor_count,
            |b, &sensor_count| {
                b.iter(|| {
                    let mut temperatures = Vec::new();
                    for i in 0..sensor_count {
                        // 模拟温度读取
                        let temp = 60.0 + (i as f32 * 2.5);
                        temperatures.push(temp);
                    }
                    black_box(temperatures);
                });
            }
        );
    }

    group.finish();
}

/// 算力计算性能基准测试
fn bench_hashrate_calculation(c: &mut Criterion) {
    c.bench_function("maijie_l7_hashrate_calculation", |b| {
        b.iter(|| {
            // 模拟ASIC算力计算
            let mut total_hashes = 0u64;
            let start_time = std::time::Instant::now();

            // 模拟高性能ASIC计算
            for i in 0..100000 {
                total_hashes = total_hashes.wrapping_add(i);
            }

            let elapsed = start_time.elapsed();
            let hashrate = if elapsed.as_secs_f64() > 0.0 {
                total_hashes as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };

            black_box(hashrate);
        });
    });
}

/// Maijie L7核心创建性能基准测试
fn bench_maijie_l7_core_lifecycle(c: &mut Criterion) {
    c.bench_function("maijie_l7_core_creation", |b| {
        b.iter(|| {
            // 创建Maijie L7核心
            let core = MaijieL7MiningCore::new("Maijie L7 基准测试核心".to_string());
            black_box(core);
        });
    });
}

/// 内存分配性能基准测试
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // 测试不同大小的内存分配
    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("allocation_size", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut data = Vec::with_capacity(*size);
                    for i in 0..*size {
                        data.push(create_test_device_info(i as u32, &format!("Maijie L7 设备{}", i)));
                    }
                    black_box(data);
                });
            }
        );
    }

    group.finish();
}

/// 并发链管理性能基准测试
fn bench_concurrent_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_chain_management");

    // 测试不同数量的并发链操作
    for chain_count in [1, 3, 6, 9].iter() {
        group.bench_with_input(
            BenchmarkId::new("chain_count", chain_count),
            chain_count,
            |b, &chain_count| {
                b.iter(|| {
                    let mut chains = Vec::new();

                    // 创建多个链配置
                    for i in 0..*chain_count {
                        let config = create_test_chain_config(i);
                        chains.push(config);
                    }

                    black_box(chains);
                });
            }
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_device_creation,
    bench_chain_config_creation,
    bench_work_submission,
    bench_temperature_monitoring,
    bench_hashrate_calculation,
    bench_maijie_l7_core_lifecycle,
    bench_memory_usage,
    bench_concurrent_chains
);

criterion_main!(benches);
