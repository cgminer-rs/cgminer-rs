//! 软算法核心性能基准测试
//!
//! 这个基准测试评估软算法核心的各种性能指标，包括：
//! - SHA256哈希计算性能
//! - 设备创建和管理性能
//! - CPU绑定性能
//! - 并发挖矿性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cgminer_core::{DeviceInfo, DeviceConfig, MiningDevice, Work};
use cgminer_software_core::{
    SoftwareMiningCore, SoftwareDevice,
    cpu_affinity::{CpuAffinityManager, CpuAffinityStrategy}
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

/// 创建测试用的设备信息
fn create_test_device_info(id: u32, name: &str) -> DeviceInfo {
    DeviceInfo::new(id, name.to_string(), "software".to_string(), 0)
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

    // 难度目标 (4字节) - 设置较低的难度便于测试
    header[72..76].copy_from_slice(&0x207fffffu32.to_le_bytes());

    // Nonce (4字节) - 初始为0，挖矿时会修改
    header[76..80].copy_from_slice(&0u32.to_le_bytes());

    // 创建目标值 - 设置较低的难度
    let mut target = vec![0xffu8; 32];
    target[0] = 0x00;
    target[1] = 0x00;
    target[2] = 0x7f;

    Work {
        id,
        header,
        target,
        timestamp: SystemTime::now(),
        difficulty: 1.0,
        extranonce: vec![0u8; 4],
    }
}

/// 简单哈希计算性能基准测试
fn bench_hash_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_calculation");

    // 测试不同大小的数据
    for size in [32, 64, 80, 128, 256].iter() {
        let data = vec![0x42u8; *size];

        group.bench_with_input(BenchmarkId::new("size", size), size, |b, _| {
            b.iter(|| {
                // 简单的哈希计算模拟
                let mut result = 0u64;
                for byte in black_box(&data) {
                    result = result.wrapping_mul(31).wrapping_add(*byte as u64);
                }
                black_box(result);
            });
        });
    }

    group.finish();
}

/// 设备信息创建性能基准测试
fn bench_device_creation(c: &mut Criterion) {
    c.bench_function("device_info_creation", |b| {
        b.iter(|| {
            let device_info = create_test_device_info(0, "基准测试设备");
            black_box(device_info);
        });
    });
}

/// CPU绑定性能基准测试
fn bench_cpu_affinity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_affinity");

    // 测试不同的CPU绑定策略
    let strategies = [
        ("round_robin", CpuAffinityStrategy::RoundRobin),
        ("performance_first", CpuAffinityStrategy::PerformanceFirst),
        ("physical_cores_only", CpuAffinityStrategy::PhysicalCoresOnly),
    ];

    for (name, strategy) in strategies.iter() {
        group.bench_with_input(BenchmarkId::new("strategy", name), strategy, |b, strategy| {
            b.iter(|| {
                let mut cpu_manager = CpuAffinityManager::new(true, strategy.clone());

                // 分配10个设备到CPU核心
                for device_id in 0..10 {
                    cpu_manager.assign_cpu_core(device_id);
                }

                black_box(cpu_manager);
            });
        });
    }

    group.finish();
}

/// 工作创建性能基准测试
fn bench_work_submission(c: &mut Criterion) {
    c.bench_function("work_creation", |b| {
        b.iter(|| {
            let work = create_test_work(1);
            black_box(work);
        });
    });
}

/// 并发数据结构性能基准测试
fn bench_concurrent_devices(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_data_structures");

    // 测试不同数量的并发操作
    for count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("operation_count", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut data = Vec::new();

                    // 创建多个数据项
                    for i in 0..count {
                        let device_info = create_test_device_info(i, &format!("设备{}", i));
                        data.push(device_info);
                    }

                    black_box(data);
                });
            }
        );
    }

    group.finish();
}

/// 软算法核心创建性能基准测试
fn bench_software_core_lifecycle(c: &mut Criterion) {
    c.bench_function("software_core_creation", |b| {
        b.iter(|| {
            // 创建软算法核心
            let core = SoftwareMiningCore::new("基准测试核心".to_string());
            black_box(core);
        });
    });
}

/// 内存分配性能基准测试
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // 测试不同大小的内存分配
    for size in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("allocation_size", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut data = Vec::with_capacity(size);
                    for i in 0..size {
                        data.push(create_test_device_info(i as u32, &format!("设备{}", i)));
                    }
                    black_box(data);
                });
            }
        );
    }

    group.finish();
}

/// 算力计算性能基准测试
fn bench_hashrate_calculation(c: &mut Criterion) {
    c.bench_function("hashrate_calculation", |b| {
        b.iter(|| {
            // 模拟算力计算
            let mut total_hashes = 0u64;
            let start_time = std::time::Instant::now();

            // 模拟一些计算工作
            for i in 0..1000 {
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

criterion_group!(
    benches,
    bench_hash_calculation,
    bench_device_creation,
    bench_cpu_affinity,
    bench_work_submission,
    bench_concurrent_devices,
    bench_software_core_lifecycle,
    bench_memory_usage,
    bench_hashrate_calculation
);

criterion_main!(benches);
