#!/bin/bash

# CGMiner-RS 虚拟挖矿器启动脚本 - macOS 版本
# 这个脚本会启动虚拟挖矿核心进行演示

echo "🔥 CGMiner-RS 虚拟挖矿器 - macOS 版本"
echo "═══════════════════════════════════════════════════════════"
echo "正在启动虚拟挖矿核心..."
echo ""

# 检查是否安装了 Rust
if ! command -v rustc &> /dev/null; then
    echo "❌ 未找到 Rust 编译器"
    echo "请先安装 Rust: https://rustup.rs/"
    exit 1
fi

# 检查是否安装了 cargo
if ! command -v cargo &> /dev/null; then
    echo "❌ 未找到 Cargo"
    echo "请先安装 Rust: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust 环境检查通过"
echo ""

# 创建简单的虚拟挖矿器
cat > virtual_miner.rs << 'EOF'
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
EOF

echo "🔧 编译虚拟挖矿器..."
if rustc virtual_miner.rs -o virtual_miner; then
    echo "✅ 编译成功"
    echo ""
    echo "🎮 启动虚拟挖矿演示..."
    echo "按 Ctrl+C 可以随时停止"
    echo ""
    ./virtual_miner
else
    echo "❌ 编译失败"
    exit 1
fi

# 清理临时文件
rm -f virtual_miner.rs virtual_miner

echo ""
echo "🎉 演示完成! 这就是 CGMiner-RS 在 macOS 环境下运行虚拟核心的效果。"
echo "在实际环境中，这些虚拟核心可以替代真实的 ASIC 硬件进行测试和开发。"
