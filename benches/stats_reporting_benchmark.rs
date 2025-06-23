//! 统计上报性能基准测试
//!
//! 对比即时上报和批量上报的性能差异，帮助选择最佳方案

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// 模拟挖矿结果
#[derive(Debug, Clone)]
struct MockMiningResult {
    pub work_id: String,
    pub device_id: u32,
    pub nonce: u32,
    pub meets_target: bool,
}

impl MockMiningResult {
    fn new(device_id: u32, nonce: u32, meets_target: bool) -> Self {
        Self {
            work_id: format!("work_{}", nonce),
            device_id,
            nonce,
            meets_target,
        }
    }
}

/// 即时上报统计器 - 你的原始方案
#[derive(Debug)]
struct ImmediateStatsReporter {
    total_hashes: AtomicU64,
    accepted_work: AtomicU64,
    rejected_work: AtomicU64,
}

impl ImmediateStatsReporter {
    fn new() -> Self {
        Self {
            total_hashes: AtomicU64::new(0),
            accepted_work: AtomicU64::new(0),
            rejected_work: AtomicU64::new(0),
        }
    }

    /// 即时上报单个结果 - 你的原始方案
    fn report_result(&self, result: MockMiningResult) {
        // 立即更新统计 - 每次都是原子操作
        self.total_hashes.fetch_add(1, Ordering::Relaxed);

        if result.meets_target {
            self.accepted_work.fetch_add(1, Ordering::Relaxed);
        } else {
            self.rejected_work.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.total_hashes.load(Ordering::Relaxed),
            self.accepted_work.load(Ordering::Relaxed),
            self.rejected_work.load(Ordering::Relaxed),
        )
    }
}

/// 批量上报统计器 - 阶段2引入的方案
#[derive(Debug)]
struct BatchStatsReporter {
    // 原子统计 - 最终存储
    total_hashes: AtomicU64,
    accepted_work: AtomicU64,
    rejected_work: AtomicU64,

    // 本地缓冲 - 减少原子操作
    local_hashes: std::sync::Mutex<u64>,
    local_accepted: std::sync::Mutex<u64>,
    local_rejected: std::sync::Mutex<u64>,

    // 批量配置
    batch_size: usize,
    last_flush: std::sync::Mutex<Instant>,
    flush_interval: Duration,
}

impl BatchStatsReporter {
    fn new(batch_size: usize, flush_interval_ms: u64) -> Self {
        Self {
            total_hashes: AtomicU64::new(0),
            accepted_work: AtomicU64::new(0),
            rejected_work: AtomicU64::new(0),
            local_hashes: std::sync::Mutex::new(0),
            local_accepted: std::sync::Mutex::new(0),
            local_rejected: std::sync::Mutex::new(0),
            batch_size,
            last_flush: std::sync::Mutex::new(Instant::now()),
            flush_interval: Duration::from_millis(flush_interval_ms),
        }
    }

    /// 批量上报结果 - 阶段2引入的方案
    fn report_result(&self, result: MockMiningResult) {
        // 更新本地缓冲 - 需要锁操作
        {
            let mut local_hashes = self.local_hashes.lock().unwrap();
            *local_hashes += 1;
        }

        if result.meets_target {
            let mut local_accepted = self.local_accepted.lock().unwrap();
            *local_accepted += 1;
        } else {
            let mut local_rejected = self.local_rejected.lock().unwrap();
            *local_rejected += 1;
        }

        // 检查是否需要刷新
        self.try_flush();
    }

    fn try_flush(&self) {
        let should_flush = {
            let last_flush = self.last_flush.lock().unwrap();
            let local_hashes = self.local_hashes.lock().unwrap();

            *local_hashes >= self.batch_size as u64 ||
            last_flush.elapsed() >= self.flush_interval
        };

        if should_flush {
            self.force_flush();
        }
    }

    fn force_flush(&self) {
        // 刷新统计数据
        {
            let mut local_hashes = self.local_hashes.lock().unwrap();
            if *local_hashes > 0 {
                self.total_hashes.fetch_add(*local_hashes, Ordering::Relaxed);
                *local_hashes = 0;
            }
        }

        {
            let mut local_accepted = self.local_accepted.lock().unwrap();
            if *local_accepted > 0 {
                self.accepted_work.fetch_add(*local_accepted, Ordering::Relaxed);
                *local_accepted = 0;
            }
        }

        {
            let mut local_rejected = self.local_rejected.lock().unwrap();
            if *local_rejected > 0 {
                self.rejected_work.fetch_add(*local_rejected, Ordering::Relaxed);
                *local_rejected = 0;
            }
        }

        // 更新刷新时间
        {
            let mut last_flush = self.last_flush.lock().unwrap();
            *last_flush = Instant::now();
        }
    }

    fn get_stats(&self) -> (u64, u64, u64) {
        // 强制刷新以获取最新统计
        self.force_flush();

        (
            self.total_hashes.load(Ordering::Relaxed),
            self.accepted_work.load(Ordering::Relaxed),
            self.rejected_work.load(Ordering::Relaxed),
        )
    }
}

/// 基准测试：即时上报 vs 批量上报 - 单线程性能
fn bench_single_thread_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread_performance");

    for result_count in [1000, 10000, 50000].iter() {
        // 即时上报
        group.bench_with_input(
            BenchmarkId::new("immediate", result_count),
            result_count,
            |b, &result_count| {
                b.iter(|| {
                    let reporter = ImmediateStatsReporter::new();

                    for i in 0..result_count {
                        let result = MockMiningResult::new(1, i as u32, i % 100 == 0);
                        reporter.report_result(black_box(result));
                    }

                    let stats = reporter.get_stats();
                    black_box(stats);
                });
            },
        );

        // 批量上报
        group.bench_with_input(
            BenchmarkId::new("batch", result_count),
            result_count,
            |b, &result_count| {
                b.iter(|| {
                    let reporter = BatchStatsReporter::new(1000, 50); // 1000批次，50ms间隔

                    for i in 0..result_count {
                        let result = MockMiningResult::new(1, i as u32, i % 100 == 0);
                        reporter.report_result(black_box(result));
                    }

                    let stats = reporter.get_stats();
                    black_box(stats);
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：延迟对比 - 单个操作延迟
fn bench_latency_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_comparison");
    group.measurement_time(Duration::from_secs(10));

        // 即时上报延迟
    group.bench_function("immediate_single_result", |b| {
        let reporter = ImmediateStatsReporter::new();
        let mut counter = 0u32;
        b.iter(|| {
            counter = counter.wrapping_add(1);
            let result = MockMiningResult::new(1, counter, true);
            let start = Instant::now();
            reporter.report_result(black_box(result));
            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // 批量上报延迟（单个结果）
    group.bench_function("batch_single_result", |b| {
        let reporter = BatchStatsReporter::new(1000, 50); // 大批次，测试单个延迟
        let mut counter = 0u32;
        b.iter(|| {
            counter = counter.wrapping_add(1);
            let result = MockMiningResult::new(1, counter, true);
            let start = Instant::now();
            reporter.report_result(black_box(result));
            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

/// 基准测试：吞吐量对比 - 每秒处理多少结果
fn bench_throughput_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_comparison");
    group.measurement_time(Duration::from_secs(5));

    // 即时上报吞吐量
    group.bench_function("immediate_throughput", |b| {
        b.iter(|| {
            let reporter = ImmediateStatsReporter::new();
            let start = Instant::now();
            let mut count = 0;

            // 运行100ms，看能处理多少结果
            while start.elapsed() < Duration::from_millis(100) {
                let result = MockMiningResult::new(1, count, count % 100 == 0);
                reporter.report_result(black_box(result));
                count += 1;
            }

            black_box(count);
        });
    });

    // 批量上报吞吐量
    group.bench_function("batch_throughput", |b| {
        b.iter(|| {
            let reporter = BatchStatsReporter::new(1000, 10); // 小间隔，高吞吐量
            let start = Instant::now();
            let mut count = 0;

            // 运行100ms，看能处理多少结果
            while start.elapsed() < Duration::from_millis(100) {
                let result = MockMiningResult::new(1, count, count % 100 == 0);
                reporter.report_result(black_box(result));
                count += 1;
            }

            // 确保所有数据都被处理
            reporter.force_flush();
            black_box(count);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_thread_performance,
    bench_latency_comparison,
    bench_throughput_comparison
);

criterion_main!(benches);
