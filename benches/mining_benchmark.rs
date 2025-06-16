use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cgminer_rs::device::{Work, MiningResult, DeviceStats};
use cgminer_rs::mining::{WorkItem, ResultItem};
use cgminer_rs::monitoring::{SystemMetrics, MiningMetrics, DeviceMetrics};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// 基准测试：工作创建
fn bench_work_creation(c: &mut Criterion) {
    c.bench_function("work_creation", |b| {
        b.iter(|| {
            let target = black_box([0u8; 32]);
            let header = black_box([0u8; 80]);
            let work = Work::new(
                black_box("test_job".to_string()),
                target,
                header,
                black_box(1024.0),
            );
            black_box(work)
        })
    });
}

/// 基准测试：工作项创建
fn bench_work_item_creation(c: &mut Criterion) {
    let target = [0u8; 32];
    let header = [0u8; 80];
    let work = Work::new("test_job".to_string(), target, header, 1024.0);
    
    c.bench_function("work_item_creation", |b| {
        b.iter(|| {
            let work_item = WorkItem::new(black_box(work.clone()));
            black_box(work_item)
        })
    });
}

/// 基准测试：挖矿结果创建
fn bench_mining_result_creation(c: &mut Criterion) {
    c.bench_function("mining_result_creation", |b| {
        b.iter(|| {
            let work_id = black_box(Uuid::new_v4());
            let device_id = black_box(0);
            let nonce = black_box(0x12345678);
            let difficulty = black_box(1024.0);
            
            let result = MiningResult::new(work_id, device_id, nonce, difficulty);
            black_box(result)
        })
    });
}

/// 基准测试：结果项创建
fn bench_result_item_creation(c: &mut Criterion) {
    let work_id = Uuid::new_v4();
    let result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);
    let target = [0u8; 32];
    let header = [0u8; 80];
    let work = Work::new("test_job".to_string(), target, header, 1024.0);
    let work_item = WorkItem::new(work);
    
    c.bench_function("result_item_creation", |b| {
        b.iter(|| {
            let result_item = ResultItem::new(
                black_box(result.clone()),
                black_box(work_item.clone()),
            );
            black_box(result_item)
        })
    });
}

/// 基准测试：设备统计更新
fn bench_device_stats_update(c: &mut Criterion) {
    c.bench_function("device_stats_update", |b| {
        b.iter(|| {
            let mut stats = black_box(DeviceStats::new());
            
            // 模拟统计更新
            for _ in 0..100 {
                stats.record_valid_nonce();
                stats.add_temperature_reading(black_box(65.0 + fastrand::f32() * 10.0));
                stats.add_hashrate_reading(black_box(35.0 + fastrand::f64() * 5.0));
            }
            
            black_box(stats)
        })
    });
}

/// 基准测试：系统指标创建
fn bench_system_metrics_creation(c: &mut Criterion) {
    c.bench_function("system_metrics_creation", |b| {
        b.iter(|| {
            let metrics = SystemMetrics {
                timestamp: black_box(SystemTime::now()),
                cpu_usage: black_box(50.0),
                memory_usage: black_box(60.0),
                disk_usage: black_box(30.0),
                network_rx: black_box(1000000),
                network_tx: black_box(500000),
                temperature: black_box(65.0),
                fan_speed: black_box(2500),
                power_consumption: black_box(3200.0),
                uptime: black_box(Duration::from_secs(3600)),
            };
            black_box(metrics)
        })
    });
}

/// 基准测试：挖矿指标创建
fn bench_mining_metrics_creation(c: &mut Criterion) {
    c.bench_function("mining_metrics_creation", |b| {
        b.iter(|| {
            let metrics = MiningMetrics {
                timestamp: black_box(SystemTime::now()),
                total_hashrate: black_box(75.0),
                accepted_shares: black_box(2500),
                rejected_shares: black_box(25),
                hardware_errors: black_box(3),
                stale_shares: black_box(8),
                best_share: black_box(5000.0),
                current_difficulty: black_box(1024.0),
                network_difficulty: black_box(50000000000000.0),
                blocks_found: black_box(1),
                efficiency: black_box(22.5),
                active_devices: black_box(2),
                connected_pools: black_box(1),
            };
            black_box(metrics)
        })
    });
}

/// 基准测试：设备指标创建
fn bench_device_metrics_creation(c: &mut Criterion) {
    c.bench_function("device_metrics_creation", |b| {
        b.iter(|| {
            let metrics = DeviceMetrics {
                device_id: black_box(0),
                timestamp: black_box(SystemTime::now()),
                temperature: black_box(67.5),
                hashrate: black_box(38.0),
                power_consumption: black_box(1600.0),
                fan_speed: black_box(2800),
                voltage: black_box(875),
                frequency: black_box(525),
                error_rate: black_box(1.2),
                uptime: black_box(Duration::from_secs(7200)),
                accepted_shares: black_box(1250),
                rejected_shares: black_box(12),
                hardware_errors: black_box(2),
            };
            black_box(metrics)
        })
    });
}

/// 基准测试：哈希计算模拟
fn bench_hash_calculation(c: &mut Criterion) {
    c.bench_function("hash_calculation", |b| {
        b.iter(|| {
            let mut data = black_box([0u8; 80]);
            
            // 模拟哈希计算
            for i in 0..80 {
                data[i] = black_box((i as u8).wrapping_mul(17).wrapping_add(42));
            }
            
            // 使用 SHA-256 计算哈希
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let result = hasher.finalize();
            
            black_box(result)
        })
    });
}

/// 基准测试：nonce 验证模拟
fn bench_nonce_verification(c: &mut Criterion) {
    c.bench_function("nonce_verification", |b| {
        b.iter(|| {
            let header = black_box([0u8; 80]);
            let nonce = black_box(0x12345678u32);
            let target = black_box([0u8; 32]);
            
            // 模拟 nonce 验证
            let mut work_data = header;
            work_data[76..80].copy_from_slice(&nonce.to_le_bytes());
            
            // 计算哈希
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&work_data);
            let hash1 = hasher.finalize();
            
            let mut hasher = Sha256::new();
            hasher.update(&hash1);
            let hash2 = hasher.finalize();
            
            // 检查是否满足目标
            let valid = hash2.as_slice() < target.as_slice();
            black_box(valid)
        })
    });
}

/// 基准测试：工作队列操作
fn bench_work_queue_operations(c: &mut Criterion) {
    c.bench_function("work_queue_operations", |b| {
        b.iter(|| {
            let mut queue = black_box(std::collections::VecDeque::new());
            
            // 添加工作项
            for i in 0..100 {
                let target = [0u8; 32];
                let header = [0u8; 80];
                let work = Work::new(format!("job_{}", i), target, header, 1024.0);
                let work_item = WorkItem::new(work);
                queue.push_back(work_item);
            }
            
            // 处理工作项
            while let Some(work_item) = queue.pop_front() {
                black_box(work_item);
            }
            
            black_box(queue)
        })
    });
}

/// 基准测试：并发工作处理
fn bench_concurrent_work_processing(c: &mut Criterion) {
    c.bench_function("concurrent_work_processing", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                let mut handles = Vec::new();
                
                // 启动多个并发任务
                for i in 0..10 {
                    let handle = tokio::spawn(async move {
                        let target = [0u8; 32];
                        let header = [0u8; 80];
                        let work = Work::new(format!("job_{}", i), target, header, 1024.0);
                        
                        // 模拟工作处理
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        
                        black_box(work)
                    });
                    handles.push(handle);
                }
                
                // 等待所有任务完成
                for handle in handles {
                    let _ = handle.await;
                }
            });
        })
    });
}

criterion_group!(
    benches,
    bench_work_creation,
    bench_work_item_creation,
    bench_mining_result_creation,
    bench_result_item_creation,
    bench_device_stats_update,
    bench_system_metrics_creation,
    bench_mining_metrics_creation,
    bench_device_metrics_creation,
    bench_hash_calculation,
    bench_nonce_verification,
    bench_work_queue_operations,
    bench_concurrent_work_processing
);

criterion_main!(benches);
