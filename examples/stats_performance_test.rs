//! 统计上报性能测试示例
//!
//! 对比即时上报和批量上报的性能差异

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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
    fn report_result(&self, meets_target: bool) {
        // 立即更新统计 - 每次都是原子操作
        self.total_hashes.fetch_add(1, Ordering::Relaxed);

        if meets_target {
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
    fn report_result(&self, meets_target: bool) {
        // 更新本地缓冲 - 需要锁操作
        {
            let mut local_hashes = self.local_hashes.lock().unwrap();
            *local_hashes += 1;
        }

        if meets_target {
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

fn main() {
    println!("🔥 统计上报性能对比测试");
    println!("=" .repeat(60));

    let test_counts = vec![10_000, 100_000, 500_000];

    for &count in &test_counts {
        println!("\n📊 测试规模: {} 次上报", count);
        println!("-".repeat(40));

        // 测试即时上报
        let immediate_reporter = ImmediateStatsReporter::new();
        let start = Instant::now();

        for i in 0..count {
            immediate_reporter.report_result(i % 100 == 0);
        }

        let immediate_duration = start.elapsed();
        let immediate_stats = immediate_reporter.get_stats();

        println!("⚡ 即时上报:");
        println!("   时间: {:?}", immediate_duration);
        println!("   速度: {:.2} ops/μs", count as f64 / immediate_duration.as_micros() as f64);
        println!("   统计: {} hashes, {} accepted, {} rejected",
                immediate_stats.0, immediate_stats.1, immediate_stats.2);

        // 测试批量上报 - 小批次
        let batch_reporter_small = BatchStatsReporter::new(100, 50);
        let start = Instant::now();

        for i in 0..count {
            batch_reporter_small.report_result(i % 100 == 0);
        }

        let batch_duration_small = start.elapsed();
        let batch_stats_small = batch_reporter_small.get_stats();

        println!("📦 批量上报(100批次):");
        println!("   时间: {:?}", batch_duration_small);
        println!("   速度: {:.2} ops/μs", count as f64 / batch_duration_small.as_micros() as f64);
        println!("   统计: {} hashes, {} accepted, {} rejected",
                batch_stats_small.0, batch_stats_small.1, batch_stats_small.2);

        // 测试批量上报 - 大批次
        let batch_reporter_large = BatchStatsReporter::new(1000, 50);
        let start = Instant::now();

        for i in 0..count {
            batch_reporter_large.report_result(i % 100 == 0);
        }

        let batch_duration_large = start.elapsed();
        let batch_stats_large = batch_reporter_large.get_stats();

        println!("📦 批量上报(1000批次):");
        println!("   时间: {:?}", batch_duration_large);
        println!("   速度: {:.2} ops/μs", count as f64 / batch_duration_large.as_micros() as f64);
        println!("   统计: {} hashes, {} accepted, {} rejected",
                batch_stats_large.0, batch_stats_large.1, batch_stats_large.2);

        // 性能对比
        let immediate_ops_per_sec = count as f64 / immediate_duration.as_secs_f64();
        let batch_ops_per_sec_small = count as f64 / batch_duration_small.as_secs_f64();
        let batch_ops_per_sec_large = count as f64 / batch_duration_large.as_secs_f64();

        println!("\n🏆 性能对比:");
        println!("   即时上报: {:.0} ops/s", immediate_ops_per_sec);
        println!("   批量上报(100): {:.0} ops/s ({:.1}x)",
                batch_ops_per_sec_small, batch_ops_per_sec_small / immediate_ops_per_sec);
        println!("   批量上报(1000): {:.0} ops/s ({:.1}x)",
                batch_ops_per_sec_large, batch_ops_per_sec_large / immediate_ops_per_sec);
    }

    println!("\n" + "=".repeat(60));
    println!("🎯 延迟测试 - 单个操作延迟");
    println!("-".repeat(40));

    let immediate_reporter = ImmediateStatsReporter::new();
    let batch_reporter = BatchStatsReporter::new(1000, 50);

    let test_iterations = 10000;
    let mut immediate_total = Duration::ZERO;
    let mut batch_total = Duration::ZERO;

    // 测试即时上报延迟
    for i in 0..test_iterations {
        let start = Instant::now();
        immediate_reporter.report_result(i % 100 == 0);
        immediate_total += start.elapsed();
    }

    // 测试批量上报延迟
    for i in 0..test_iterations {
        let start = Instant::now();
        batch_reporter.report_result(i % 100 == 0);
        batch_total += start.elapsed();
    }

    let immediate_avg = immediate_total / test_iterations;
    let batch_avg = batch_total / test_iterations;

    println!("⚡ 即时上报平均延迟: {:?}", immediate_avg);
    println!("📦 批量上报平均延迟: {:?}", batch_avg);
    println!("🔍 延迟比较: 批量上报是即时上报的 {:.2}x",
            batch_avg.as_nanos() as f64 / immediate_avg.as_nanos() as f64);

    println!("\n" + "=".repeat(60));
    println!("💡 结论和建议:");
    println!("1. 即时上报: 超低延迟 (~{}ns), 适合实时性要求", immediate_avg.as_nanos());
    println!("2. 批量上报: 更高吞吐量, 但延迟较高 (~{}ns)", batch_avg.as_nanos());
    println!("3. 对于挖矿场景，建议使用即时上报保证实时性");
    println!("4. 批量上报适合非关键路径的统计聚合");
}
