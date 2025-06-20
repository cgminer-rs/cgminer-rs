//! GPU Bitcoin核心性能基准测试 (cgminer-gpu-btc-core)
//!
//! 这个基准测试用于评估GPU Bitcoin挖矿核心的性能，包括：
//! - GPU设备初始化性能
//! - 挖矿计算性能
//! - 内存使用效率
//! - OpenCL/CUDA后端性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use std::time::Duration;

use cgminer_gpu_btc_core::{
    GpuMiningCore, GpuDevice,
    gpu_manager::GpuManager,
    opencl_backend::OpenCLBackend
};
use cgminer_core::{
    CoreConfig, DeviceConfig, Work, WorkData, Target
};

/// 创建测试用的工作数据
fn create_test_work() -> Work {
    let work_data = WorkData::new(
        hex::decode("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        hex::decode("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap(),
    );
    
    let target = Target::from_difficulty(1.0);
    
    Work::new(
        "test_job_001".to_string(),
        work_data,
        target,
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    )
}

/// GPU核心初始化基准测试
fn bench_gpu_core_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("gpu_core_initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let mut core = black_box(GpuMiningCore::new("Benchmark GPU Core".to_string()));
            let config = CoreConfig::default();
            
            let result = core.initialize(config).await;
            black_box(result).unwrap();
        });
    });
}

/// GPU设备创建基准测试
fn bench_gpu_device_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("gpu_device_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let gpu_manager = black_box(GpuManager::new().unwrap());
            gpu_manager.initialize().await.unwrap();
            
            let device_info = cgminer_core::DeviceInfo::new(0, "Test GPU".to_string(), "gpu".to_string(), 0);
            let device_config = DeviceConfig::default();
            let target_hashrate = 1_000_000_000_000.0; // 1 TH/s
            
            let device = GpuDevice::new(
                device_info,
                device_config,
                target_hashrate,
                std::sync::Arc::new(gpu_manager),
            ).await;
            
            black_box(device).unwrap();
        });
    });
}

/// GPU挖矿计算基准测试
fn bench_gpu_mining_computation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // 准备测试环境
    let (gpu_manager, mut device) = rt.block_on(async {
        let gpu_manager = GpuManager::new().unwrap();
        gpu_manager.initialize().await.unwrap();
        
        let device_info = cgminer_core::DeviceInfo::new(0, "Test GPU".to_string(), "gpu".to_string(), 0);
        let device_config = DeviceConfig::default();
        let target_hashrate = 1_000_000_000_000.0; // 1 TH/s
        
        let mut device = GpuDevice::new(
            device_info,
            device_config,
            target_hashrate,
            std::sync::Arc::new(gpu_manager.clone()),
        ).await.unwrap();
        
        device.initialize(DeviceConfig::default()).await.unwrap();
        device.start().await.unwrap();
        
        (gpu_manager, device)
    });
    
    c.bench_function("gpu_mining_computation", |b| {
        b.to_async(&rt).iter(|| async {
            let work = black_box(create_test_work());
            
            // 提交工作
            device.submit_work(work).await.unwrap();
            
            // 等待一小段时间让GPU处理
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // 获取结果
            let result = device.get_result().await.unwrap();
            black_box(result);
        });
    });
}

/// OpenCL后端性能基准测试
fn bench_opencl_backend(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("opencl_backend_initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let mut backend = black_box(OpenCLBackend::new());
            let result = backend.initialize().await;
            black_box(result).unwrap();
        });
    });
}

/// 不同GPU设备数量的性能基准测试
fn bench_multiple_gpu_devices(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("multiple_gpu_devices");
    
    for device_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("device_count", device_count),
            device_count,
            |b, &device_count| {
                b.to_async(&rt).iter(|| async {
                    let gpu_manager = std::sync::Arc::new(GpuManager::new().unwrap());
                    gpu_manager.initialize().await.unwrap();
                    
                    let mut devices = Vec::new();
                    
                    // 创建多个GPU设备
                    for i in 0..device_count {
                        let device_info = cgminer_core::DeviceInfo::new(
                            i as u32,
                            format!("Test GPU {}", i),
                            "gpu".to_string(),
                            0
                        );
                        let device_config = DeviceConfig::default();
                        let target_hashrate = 1_000_000_000_000.0; // 1 TH/s
                        
                        let mut device = GpuDevice::new(
                            device_info,
                            device_config,
                            target_hashrate,
                            gpu_manager.clone(),
                        ).await.unwrap();
                        
                        device.initialize(DeviceConfig::default()).await.unwrap();
                        device.start().await.unwrap();
                        
                        devices.push(device);
                    }
                    
                    // 向所有设备提交工作
                    let work = create_test_work();
                    for device in &mut devices {
                        device.submit_work(work.clone()).await.unwrap();
                    }
                    
                    // 等待处理
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    
                    // 收集结果
                    let mut results = Vec::new();
                    for device in &mut devices {
                        if let Some(result) = device.get_result().await.unwrap() {
                            results.push(result);
                        }
                    }
                    
                    black_box(results);
                });
            },
        );
    }
    
    group.finish();
}

/// 不同算力目标的性能基准测试
fn bench_different_hashrates(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("different_hashrates");
    
    // 测试不同的目标算力：100 GH/s, 1 TH/s, 10 TH/s, 100 TH/s
    let hashrates = vec![
        (100_000_000_000.0, "100GH"),
        (1_000_000_000_000.0, "1TH"),
        (10_000_000_000_000.0, "10TH"),
        (100_000_000_000_000.0, "100TH"),
    ];
    
    for (hashrate, label) in hashrates {
        group.bench_with_input(
            BenchmarkId::new("hashrate", label),
            &hashrate,
            |b, &hashrate| {
                b.to_async(&rt).iter(|| async {
                    let gpu_manager = std::sync::Arc::new(GpuManager::new().unwrap());
                    gpu_manager.initialize().await.unwrap();
                    
                    let device_info = cgminer_core::DeviceInfo::new(0, "Test GPU".to_string(), "gpu".to_string(), 0);
                    let device_config = DeviceConfig::default();
                    
                    let mut device = GpuDevice::new(
                        device_info,
                        device_config,
                        hashrate,
                        gpu_manager,
                    ).await.unwrap();
                    
                    device.initialize(DeviceConfig::default()).await.unwrap();
                    device.start().await.unwrap();
                    
                    let work = create_test_work();
                    device.submit_work(work).await.unwrap();
                    
                    // 等待处理
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    
                    let result = device.get_result().await.unwrap();
                    black_box(result);
                });
            },
        );
    }
    
    group.finish();
}

/// GPU内存使用基准测试
fn bench_gpu_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("gpu_memory_usage", |b| {
        b.to_async(&rt).iter(|| async {
            let gpu_manager = std::sync::Arc::new(GpuManager::new().unwrap());
            gpu_manager.initialize().await.unwrap();
            
            // 获取GPU信息
            let gpu_infos = gpu_manager.scan_gpus().await.unwrap();
            black_box(gpu_infos);
            
            // 更新GPU状态
            gpu_manager.update_gpu_status().await.unwrap();
            
            // 检查健康状态
            let healthy = gpu_manager.is_healthy().await;
            black_box(healthy);
        });
    });
}

criterion_group!(
    benches,
    bench_gpu_core_initialization,
    bench_gpu_device_creation,
    bench_gpu_mining_computation,
    bench_opencl_backend,
    bench_multiple_gpu_devices,
    bench_different_hashrates,
    bench_gpu_memory_usage
);

criterion_main!(benches);
