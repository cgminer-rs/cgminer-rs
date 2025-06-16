use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cgminer_rs::device::{DeviceInfo, DeviceConfig, Work, MiningResult};
use cgminer_rs::device::virtual_device::{VirtualDeviceDriver, VirtualDevice};
use cgminer_rs::device::traits::{MiningDevice, DeviceDriver};
use std::time::{Duration, SystemTime};
use tokio::runtime::Runtime;
use uuid::Uuid;

/// 基准测试：虚拟设备驱动创建
fn bench_virtual_driver_creation(c: &mut Criterion) {
    c.bench_function("virtual_driver_creation", |b| {
        b.iter(|| {
            let driver = VirtualDeviceDriver::new();
            black_box(driver)
        })
    });
}

/// 基准测试：虚拟设备扫描
fn bench_virtual_device_scan(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let driver = VirtualDeviceDriver::new();

    c.bench_function("virtual_device_scan", |b| {
        b.iter(|| {
            rt.block_on(async {
                let devices = driver.scan_devices().await.unwrap();
                black_box(devices)
            })
        })
    });
}

/// 基准测试：虚拟设备创建
fn bench_virtual_device_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("virtual_device_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let device_info = DeviceInfo::new(
                    black_box(1000),
                    black_box("Test Virtual Device".to_string()),
                    black_box("virtual".to_string()),
                    black_box(0),
                );

                let device = VirtualDevice::new(device_info).await.unwrap();
                black_box(device)
            })
        })
    });
}

/// 基准测试：虚拟设备初始化
fn bench_virtual_device_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("virtual_device_initialization", |b| {
        b.iter(|| {
            rt.block_on(async {
                let device_info = DeviceInfo::new(
                    1000,
                    "Test Virtual Device".to_string(),
                    "virtual".to_string(),
                    0,
                );

                let mut device = VirtualDevice::new(device_info).await.unwrap();
                let config = DeviceConfig::default();

                let result = device.initialize(config).await;
                black_box(result)
            })
        })
    });
}

/// 基准测试：虚拟设备启动和停止
fn bench_virtual_device_start_stop(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("virtual_device_start_stop", |b| {
        b.iter(|| {
            rt.block_on(async {
                let device_info = DeviceInfo::new(
                    1000,
                    "Test Virtual Device".to_string(),
                    "virtual".to_string(),
                    0,
                );

                let mut device = VirtualDevice::new(device_info).await.unwrap();
                let config = DeviceConfig::default();
                device.initialize(config).await.unwrap();

                // 启动设备
                let start_result = device.start().await;

                // 等待一小段时间
                tokio::time::sleep(Duration::from_millis(10)).await;

                // 停止设备
                let stop_result = device.stop().await;

                black_box((start_result, stop_result))
            })
        })
    });
}

/// 基准测试：虚拟设备工作提交
fn bench_virtual_device_work_submission(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预先创建设备
    let mut device = rt.block_on(async {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();
        device
    });

    c.bench_function("virtual_device_work_submission", |b| {
        b.iter(|| {
            rt.block_on(async {
                let target = black_box([0u8; 32]);
                let header = black_box([0u8; 80]);
                let work = Work::new(
                    black_box("test_job".to_string()),
                    target,
                    header,
                    black_box(1024.0),
                );

                let result = device.submit_work(work).await;
                black_box(result)
            })
        })
    });
}

/// 基准测试：虚拟设备状态查询
fn bench_virtual_device_status_queries(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预先创建设备
    let device = rt.block_on(async {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();
        device
    });

    c.bench_function("virtual_device_status_queries", |b| {
        b.iter(|| {
            rt.block_on(async {
                let info = device.get_info().await;
                let status = device.get_status().await;
                let temperature = device.get_temperature().await;
                let hashrate = device.get_hashrate().await;
                let stats = device.get_stats().await;

                black_box((info, status, temperature, hashrate, stats))
            })
        })
    });
}

/// 基准测试：虚拟设备参数设置
fn bench_virtual_device_parameter_setting(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("virtual_device_parameter_setting", |b| {
        b.iter(|| {
            rt.block_on(async {
                let device_info = DeviceInfo::new(
                    1000,
                    "Test Virtual Device".to_string(),
                    "virtual".to_string(),
                    0,
                );

                let mut device = VirtualDevice::new(device_info).await.unwrap();
                let config = DeviceConfig::default();
                device.initialize(config).await.unwrap();

                // 设置各种参数
                let freq_result = device.set_frequency(black_box(700)).await;
                let voltage_result = device.set_voltage(black_box(950)).await;
                let fan_result = device.set_fan_speed(black_box(80)).await;

                black_box((freq_result, voltage_result, fan_result))
            })
        })
    });
}

/// 基准测试：虚拟设备健康检查
fn bench_virtual_device_health_check(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预先创建设备
    let device = rt.block_on(async {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();
        device
    });

    c.bench_function("virtual_device_health_check", |b| {
        b.iter(|| {
            rt.block_on(async {
                let health = device.health_check().await;
                black_box(health)
            })
        })
    });
}

/// 基准测试：多个虚拟设备并发操作
fn bench_multiple_virtual_devices(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    for device_count in [1, 2, 4, 8].iter() {
        c.bench_with_input(
            BenchmarkId::new("multiple_virtual_devices", device_count),
            device_count,
            |b, &device_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // 创建多个虚拟设备
                        for i in 0..device_count {
                            let handle = tokio::spawn(async move {
                                let device_info = DeviceInfo::new(
                                    1000 + i as u32,
                                    format!("Virtual Device {}", i),
                                    "virtual".to_string(),
                                    i as u8,
                                );

                                let mut device = VirtualDevice::new(device_info).await.unwrap();
                                let config = DeviceConfig::default();
                                device.initialize(config).await.unwrap();

                                // 启动设备
                                device.start().await.unwrap();

                                // 提交一些工作
                                for j in 0..10 {
                                    let target = [0u8; 32];
                                    let header = [0u8; 80];
                                    let work = Work::new(
                                        format!("job_{}_{}", i, j),
                                        target,
                                        header,
                                        1024.0,
                                    );
                                    let _ = device.submit_work(work).await;
                                }

                                // 等待一小段时间
                                tokio::time::sleep(Duration::from_millis(50)).await;

                                // 停止设备
                                device.stop().await.unwrap();

                                black_box(device)
                            });
                            handles.push(handle);
                        }

                        // 等待所有设备完成
                        for handle in handles {
                            let _ = handle.await;
                        }
                    })
                })
            },
        );
    }
}

/// 基准测试：虚拟设备长时间运行
fn bench_virtual_device_long_running(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("virtual_device_long_running", |b| {
        b.iter(|| {
            rt.block_on(async {
                let device_info = DeviceInfo::new(
                    1000,
                    "Test Virtual Device".to_string(),
                    "virtual".to_string(),
                    0,
                );

                let mut device = VirtualDevice::new(device_info).await.unwrap();
                let config = DeviceConfig::default();
                device.initialize(config).await.unwrap();

                // 启动设备
                device.start().await.unwrap();

                // 持续提交工作并监控状态
                for i in 0..100 {
                    let target = [0u8; 32];
                    let header = [0u8; 80];
                    let work = Work::new(
                        format!("job_{}", i),
                        target,
                        header,
                        1024.0,
                    );
                    let _ = device.submit_work(work).await;

                    // 每10次查询一次状态
                    if i % 10 == 0 {
                        let _ = device.get_info().await;
                        let _ = device.get_stats().await;
                    }
                }

                // 停止设备
                device.stop().await.unwrap();

                black_box(device)
            })
        })
    });
}

/// 基准测试：Bitcoin 挖矿算法性能
fn bench_bitcoin_mining_algorithm(c: &mut Criterion) {
    c.bench_function("bitcoin_mining_algorithm", |b| {
        b.iter(|| {
            let header = black_box([0u8; 80]);
            let target = black_box([0xFFu8; 32]); // 简单目标
            let start_nonce = black_box(0);
            let max_iterations = black_box(1000);

            let result = VirtualDevice::mine_bitcoin_block(&header, &target, start_nonce, max_iterations);
            black_box(result)
        })
    });
}

/// 基准测试：哈希目标检查
fn bench_hash_meets_target(c: &mut Criterion) {
    let hash = [0x00u8; 32];
    let target = [0x01u8; 32];

    c.bench_function("hash_meets_target", |b| {
        b.iter(|| {
            let result = VirtualDevice::hash_meets_target(black_box(&hash), black_box(&target));
            black_box(result)
        })
    });
}

/// 基准测试：难度到目标转换
fn bench_difficulty_to_target(c: &mut Criterion) {
    c.bench_function("difficulty_to_target", |b| {
        b.iter(|| {
            let difficulty = black_box(1024.0);
            let target = VirtualDevice::difficulty_to_target(difficulty);
            black_box(target)
        })
    });
}

/// 基准测试：不同难度下的挖矿性能
fn bench_mining_different_difficulties(c: &mut Criterion) {
    for difficulty in [1.0, 10.0, 100.0, 1000.0].iter() {
        c.bench_with_input(
            BenchmarkId::new("mining_difficulty", difficulty),
            difficulty,
            |b, &difficulty| {
                b.iter(|| {
                    let header = black_box([0u8; 80]);
                    let target = VirtualDevice::difficulty_to_target(difficulty);
                    let start_nonce = black_box(fastrand::u32(..));
                    let max_iterations = black_box(100); // 限制迭代次数以保持基准测试时间合理

                    let result = VirtualDevice::mine_bitcoin_block(&header, &target, start_nonce, max_iterations);
                    black_box(result)
                })
            },
        );
    }
}

/// 基准测试：SHA-256 双重哈希
fn bench_double_sha256(c: &mut Criterion) {
    use sha2::{Sha256, Digest};

    c.bench_function("double_sha256", |b| {
        b.iter(|| {
            let data = black_box([0u8; 80]);

            // 第一次 SHA-256
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash1 = hasher.finalize();

            // 第二次 SHA-256
            let mut hasher = Sha256::new();
            hasher.update(&hash1);
            let hash2 = hasher.finalize();

            black_box(hash2)
        })
    });
}

criterion_group!(
    virtual_device_benches,
    bench_virtual_driver_creation,
    bench_virtual_device_scan,
    bench_virtual_device_creation,
    bench_virtual_device_initialization,
    bench_virtual_device_start_stop,
    bench_virtual_device_work_submission,
    bench_virtual_device_status_queries,
    bench_virtual_device_parameter_setting,
    bench_virtual_device_health_check,
    bench_multiple_virtual_devices,
    bench_virtual_device_long_running,
    bench_bitcoin_mining_algorithm,
    bench_hash_meets_target,
    bench_difficulty_to_target,
    bench_mining_different_difficulties,
    bench_double_sha256
);

criterion_main!(virtual_device_benches);
