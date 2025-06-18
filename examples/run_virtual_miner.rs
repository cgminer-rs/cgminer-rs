#!/usr/bin/env rust-script

//! 简化的虚拟挖矿器，用于在 macOS 环境下运行虚拟核
//! 
//! 使用方法：
//! ```bash
//! cargo install rust-script
//! rust-script run_virtual_miner.rs
//! ```

use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 虚拟设备结构
#[derive(Debug)]
struct VirtualDevice {
    id: u32,
    name: String,
    hashrate: f64,  // MH/s
    temperature: f32,
    accepted_shares: AtomicU64,
    rejected_shares: AtomicU64,
    hardware_errors: AtomicU64,
    running: AtomicBool,
    start_time: Instant,
}

impl VirtualDevice {
    fn new(id: u32, name: String, hashrate: f64) -> Self {
        Self {
            id,
            name,
            hashrate,
            temperature: 45.0,
            accepted_shares: AtomicU64::new(0),
            rejected_shares: AtomicU64::new(0),
            hardware_errors: AtomicU64::new(0),
            running: AtomicBool::new(false),
            start_time: Instant::now(),
        }
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        println!("🚀 虚拟设备 {} ({}) 已启动，算力: {:.1} MH/s", 
                 self.id, self.name, self.hashrate);
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        println!("⏹️  虚拟设备 {} ({}) 已停止", self.id, self.name);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn simulate_mining(&self) {
        while self.is_running() {
            // 模拟挖矿过程
            thread::sleep(Duration::from_millis(100));
            
            // 随机生成份额
            if fastrand::f32() < 0.01 { // 1% 概率找到份额
                if fastrand::f32() < 0.95 { // 95% 接受率
                    self.accepted_shares.fetch_add(1, Ordering::Relaxed);
                    println!("✅ 设备 {} 找到有效份额！总计: {}", 
                             self.id, self.accepted_shares.load(Ordering::Relaxed));
                } else {
                    self.rejected_shares.fetch_add(1, Ordering::Relaxed);
                    println!("❌ 设备 {} 份额被拒绝，总计: {}", 
                             self.id, self.rejected_shares.load(Ordering::Relaxed));
                }
            }

            // 随机硬件错误
            if fastrand::f32() < 0.001 { // 0.1% 概率硬件错误
                self.hardware_errors.fetch_add(1, Ordering::Relaxed);
                println!("⚠️  设备 {} 硬件错误，总计: {}", 
                         self.id, self.hardware_errors.load(Ordering::Relaxed));
            }
        }
    }

    fn get_stats(&self) -> DeviceStats {
        let uptime = self.start_time.elapsed();
        DeviceStats {
            id: self.id,
            name: self.name.clone(),
            hashrate: self.hashrate,
            temperature: self.temperature,
            accepted_shares: self.accepted_shares.load(Ordering::Relaxed),
            rejected_shares: self.rejected_shares.load(Ordering::Relaxed),
            hardware_errors: self.hardware_errors.load(Ordering::Relaxed),
            uptime_seconds: uptime.as_secs(),
            running: self.is_running(),
        }
    }
}

#[derive(Debug)]
struct DeviceStats {
    id: u32,
    name: String,
    hashrate: f64,
    temperature: f32,
    accepted_shares: u64,
    rejected_shares: u64,
    hardware_errors: u64,
    uptime_seconds: u64,
    running: bool,
}

/// 虚拟挖矿管理器
struct VirtualMiningManager {
    devices: Vec<Arc<VirtualDevice>>,
    running: AtomicBool,
}

impl VirtualMiningManager {
    fn new() -> Self {
        let mut devices = Vec::new();
        
        // 创建虚拟设备
        for i in 0..4 {
            let hashrate = 50.0 + fastrand::f64() * 100.0; // 50-150 MH/s
            let device = Arc::new(VirtualDevice::new(
                i,
                format!("虚拟核心 {}", i),
                hashrate,
            ));
            devices.push(device);
        }

        Self {
            devices,
            running: AtomicBool::new(false),
        }
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        println!("🎯 启动虚拟挖矿管理器");
        
        // 启动所有设备
        for device in &self.devices {
            device.start();
            
            // 为每个设备启动挖矿线程
            let device_clone = Arc::clone(device);
            thread::spawn(move || {
                device_clone.simulate_mining();
            });
        }
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        println!("🛑 停止虚拟挖矿管理器");
        
        // 停止所有设备
        for device in &self.devices {
            device.stop();
        }
    }

    fn print_stats(&self) {
        println!("\n📊 虚拟挖矿统计信息");
        println!("═══════════════════════════════════════════════════════════");
        
        let mut total_hashrate = 0.0;
        let mut total_accepted = 0;
        let mut total_rejected = 0;
        let mut total_errors = 0;

        for device in &self.devices {
            let stats = device.get_stats();
            total_hashrate += stats.hashrate;
            total_accepted += stats.accepted_shares;
            total_rejected += stats.rejected_shares;
            total_errors += stats.hardware_errors;

            let status = if stats.running { "🟢 运行中" } else { "🔴 已停止" };
            let uptime_hours = stats.uptime_seconds / 3600;
            let uptime_minutes = (stats.uptime_seconds % 3600) / 60;
            
            println!("设备 {}: {} | 算力: {:.1} MH/s | 温度: {:.1}°C | 运行时间: {}h{}m", 
                     stats.id, status, stats.hashrate, stats.temperature, 
                     uptime_hours, uptime_minutes);
            println!("  ✅ 接受: {} | ❌ 拒绝: {} | ⚠️  错误: {}", 
                     stats.accepted_shares, stats.rejected_shares, stats.hardware_errors);
        }

        println!("───────────────────────────────────────────────────────────");
        println!("总算力: {:.1} MH/s | 总接受: {} | 总拒绝: {} | 总错误: {}", 
                 total_hashrate, total_accepted, total_rejected, total_errors);
        
        if total_accepted + total_rejected > 0 {
            let accept_rate = total_accepted as f64 / (total_accepted + total_rejected) as f64 * 100.0;
            println!("接受率: {:.2}%", accept_rate);
        }
        println!("═══════════════════════════════════════════════════════════\n");
    }
}

fn main() {
    println!("🔥 CGMiner-RS 虚拟挖矿器 - macOS 版本");
    println!("═══════════════════════════════════════════════════════════");
    println!("这是一个用于测试和演示的虚拟比特币挖矿器");
    println!("它模拟了真实的挖矿过程，包括算力、份额和硬件错误");
    println!("═══════════════════════════════════════════════════════════\n");

    let manager = VirtualMiningManager::new();
    
    // 启动挖矿
    manager.start();
    
    // 主循环 - 每10秒打印一次统计信息
    let start_time = Instant::now();
    loop {
        thread::sleep(Duration::from_secs(10));
        manager.print_stats();
        
        // 运行5分钟后自动停止
        if start_time.elapsed() > Duration::from_secs(300) {
            println!("⏰ 演示时间结束，正在停止挖矿器...");
            break;
        }
    }
    
    manager.stop();
    thread::sleep(Duration::from_secs(1)); // 等待线程结束
    
    println!("👋 虚拟挖矿器已停止，感谢使用！");
}

// 简单的随机数生成器
mod fastrand {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u64>> = Cell::new(Wrapping(1));
    }

    pub fn f32() -> f32 {
        (u32() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    pub fn f64() -> f64 {
        (u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn u32() -> u32 {
        u64() as u32
    }

    fn u64() -> u64 {
        RNG.with(|rng| {
            let mut x = rng.get();
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            rng.set(x);
            (x * Wrapping(0x2545F4914F6CDD1D)).0
        })
    }
}
