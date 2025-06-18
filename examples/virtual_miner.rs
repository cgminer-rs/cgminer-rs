use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

struct VirtualCore {
    id: u32,
    hashrate: f64,
    shares: AtomicU64,
    running: AtomicBool,
}

impl VirtualCore {
    fn new(id: u32, hashrate: f64) -> Self {
        Self {
            id,
            hashrate,
            shares: AtomicU64::new(0),
            running: AtomicBool::new(false),
        }
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        println!("🚀 虚拟核心 {} 启动 (算力: {:.1} MH/s)", self.id, self.hashrate);
    }

    fn mine(&self) {
        let mut last_share = Instant::now();
        while self.running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
            
            // 模拟找到份额 (每30秒左右)
            if last_share.elapsed() > Duration::from_secs(25 + (self.id as u64 * 5)) {
                self.shares.fetch_add(1, Ordering::Relaxed);
                println!("✅ 核心 {} 找到份额! 总计: {}", self.id, self.shares.load(Ordering::Relaxed));
                last_share = Instant::now();
            }
        }
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        println!("⏹️  核心 {} 已停止", self.id);
    }

    fn get_shares(&self) -> u64 {
        self.shares.load(Ordering::Relaxed)
    }
}

fn main() {
    println!("🎯 启动 4 个虚拟挖矿核心");
    println!("");

    // 创建虚拟核心
    let cores = vec![
        VirtualCore::new(0, 85.5),
        VirtualCore::new(1, 92.3),
        VirtualCore::new(2, 78.9),
        VirtualCore::new(3, 88.1),
    ];

    // 启动所有核心
    for core in &cores {
        core.start();
    }

    // 为每个核心启动挖矿线程
    let handles: Vec<_> = cores.iter().map(|core| {
        let core_ref = core as *const VirtualCore;
        thread::spawn(move || {
            unsafe { (*core_ref).mine() };
        })
    }).collect();

    // 主循环 - 显示统计信息
    let start_time = Instant::now();
    for i in 0..30 { // 运行5分钟
        thread::sleep(Duration::from_secs(10));
        
        let total_hashrate: f64 = cores.iter().map(|c| c.hashrate).sum();
        let total_shares: u64 = cores.iter().map(|c| c.get_shares()).sum();
        let uptime = start_time.elapsed().as_secs();
        
        println!("📊 运行时间: {}分{}秒 | 总算力: {:.1} MH/s | 总份额: {}", 
                 uptime / 60, uptime % 60, total_hashrate, total_shares);
    }

    // 停止所有核心
    println!("\n🛑 正在停止虚拟挖矿核心...");
    for core in &cores {
        core.stop();
    }

    // 等待线程结束
    for handle in handles {
        let _ = handle.join();
    }

    println!("👋 虚拟挖矿演示结束!");
}
