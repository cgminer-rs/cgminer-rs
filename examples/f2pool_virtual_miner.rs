use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::io::{self, Write};

#[derive(Debug)]
struct StratumConnection {
    pool_url: String,
    username: String,
    password: String,
    connected: AtomicBool,
    shares_submitted: AtomicU64,
    shares_accepted: AtomicU64,
    shares_rejected: AtomicU64,
}

impl StratumConnection {
    fn new(pool_url: String, username: String, password: String) -> Self {
        Self {
            pool_url,
            username,
            password,
            connected: AtomicBool::new(false),
            shares_submitted: AtomicU64::new(0),
            shares_accepted: AtomicU64::new(0),
            shares_rejected: AtomicU64::new(0),
        }
    }

    fn connect(&self) -> bool {
        println!("🔗 正在连接到矿池: {}", self.pool_url);
        println!("👤 用户名: {}", self.username);
        
        // 模拟连接过程
        thread::sleep(Duration::from_secs(2));
        
        // 在实际实现中，这里会建立 TCP 连接并进行 Stratum 握手
        self.connected.store(true, Ordering::Relaxed);
        println!("✅ 成功连接到 F2Pool!");
        true
    }

    fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
        println!("🔌 已断开与矿池的连接");
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn submit_share(&self) -> bool {
        if !self.is_connected() {
            return false;
        }

        self.shares_submitted.fetch_add(1, Ordering::Relaxed);
        
        // 模拟 95% 的接受率
        let accepted = rand() % 100 < 95;
        
        if accepted {
            self.shares_accepted.fetch_add(1, Ordering::Relaxed);
            println!("✅ 份额被接受! 总接受: {}", self.shares_accepted.load(Ordering::Relaxed));
        } else {
            self.shares_rejected.fetch_add(1, Ordering::Relaxed);
            println!("❌ 份额被拒绝! 总拒绝: {}", self.shares_rejected.load(Ordering::Relaxed));
        }
        
        accepted
    }

    fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.shares_submitted.load(Ordering::Relaxed),
            self.shares_accepted.load(Ordering::Relaxed),
            self.shares_rejected.load(Ordering::Relaxed),
        )
    }
}

#[derive(Debug)]
struct VirtualMiner {
    id: u32,
    name: String,
    hashrate: f64,  // TH/s
    running: AtomicBool,
    shares_found: AtomicU64,
}

impl VirtualMiner {
    fn new(id: u32, name: String, hashrate: f64) -> Self {
        Self {
            id,
            name,
            hashrate,
            running: AtomicBool::new(false),
            shares_found: AtomicU64::new(0),
        }
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        println!("🚀 {} 启动 (算力: {:.1} TH/s)", self.name, self.hashrate);
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        println!("⏹️  {} 已停止", self.name);
    }

    fn mine(&self, stratum: Arc<StratumConnection>) {
        let mut last_share = Instant::now();
        // 根据算力调整找到份额的间隔 (算力越高，间隔越短)
        let base_interval = 60.0 / self.hashrate.max(1.0); // 基础间隔(秒)
        
        while self.running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(1000));
            
            if !stratum.is_connected() {
                continue;
            }
            
            // 模拟找到份额
            let interval = Duration::from_secs_f64(base_interval + (rand() % 30) as f64);
            if last_share.elapsed() > interval {
                self.shares_found.fetch_add(1, Ordering::Relaxed);
                println!("💎 {} 找到份额! (算力: {:.1} TH/s)", self.name, self.hashrate);
                
                // 提交份额到矿池
                stratum.submit_share();
                last_share = Instant::now();
            }
        }
    }

    fn get_shares(&self) -> u64 {
        self.shares_found.load(Ordering::Relaxed)
    }
}

// 简单的随机数生成器
fn rand() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(1);
    
    let mut seed = SEED.load(Ordering::Relaxed);
    seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    SEED.store(seed, Ordering::Relaxed);
    seed
}

fn get_user_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("读取输入失败");
    input.trim().to_string()
}

fn main() {
    println!("🔥 CGMiner-RS F2Pool 虚拟挖矿器");
    println!("═══════════════════════════════════════════════════════════");
    println!("连接到: stratum+tcp://btc-asia.f2pool.com:1314");
    println!("");

    // 获取用户配置
    print!("请输入你的 F2Pool 用户名: ");
    io::stdout().flush().unwrap();
    let username = get_user_input();
    
    if username.is_empty() {
        println!("❌ 用户名不能为空!");
        return;
    }

    print!("请输入矿工名称 (默认: worker1): ");
    io::stdout().flush().unwrap();
    let worker_name = get_user_input();
    let worker_name = if worker_name.is_empty() { "worker1".to_string() } else { worker_name };
    
    let full_username = format!("{}.{}", username, worker_name);
    
    println!("");
    println!("📋 配置信息:");
    println!("   矿池: btc-asia.f2pool.com:1314");
    println!("   用户: {}", full_username);
    println!("   密码: x");
    println!("");

    // 创建 Stratum 连接
    let stratum = Arc::new(StratumConnection::new(
        "stratum+tcp://btc-asia.f2pool.com:1314".to_string(),
        full_username,
        "x".to_string(),
    ));

    // 连接到矿池
    if !stratum.connect() {
        println!("❌ 连接矿池失败!");
        return;
    }

    // 创建虚拟矿机
    let miners = vec![
        Arc::new(VirtualMiner::new(0, "虚拟矿机-1".to_string(), 100.0)),
        Arc::new(VirtualMiner::new(1, "虚拟矿机-2".to_string(), 120.0)),
        Arc::new(VirtualMiner::new(2, "虚拟矿机-3".to_string(), 95.0)),
        Arc::new(VirtualMiner::new(3, "虚拟矿机-4".to_string(), 110.0)),
    ];

    // 启动所有矿机
    for miner in &miners {
        miner.start();
    }

    // 为每个矿机启动挖矿线程
    let handles: Vec<_> = miners.iter().map(|miner| {
        let miner_clone = Arc::clone(miner);
        let stratum_clone = Arc::clone(&stratum);
        thread::spawn(move || {
            miner_clone.mine(stratum_clone);
        })
    }).collect();

    // 主循环 - 显示统计信息
    let start_time = Instant::now();
    println!("📊 挖矿统计信息 (每15秒更新一次):");
    println!("───────────────────────────────────────────────────────────");
    
    for i in 0..20 { // 运行5分钟
        thread::sleep(Duration::from_secs(15));
        
        let total_hashrate: f64 = miners.iter().map(|m| m.hashrate).sum();
        let total_shares: u64 = miners.iter().map(|m| m.get_shares()).sum();
        let (submitted, accepted, rejected) = stratum.get_stats();
        let uptime = start_time.elapsed().as_secs();
        
        println!("⏰ 运行时间: {:02}:{:02} | 💎 总算力: {:.1} TH/s", 
                 uptime / 60, uptime % 60, total_hashrate);
        println!("📈 本地份额: {} | 📤 已提交: {} | ✅ 已接受: {} | ❌ 已拒绝: {}", 
                 total_shares, submitted, accepted, rejected);
        
        if submitted > 0 {
            let accept_rate = accepted as f64 / submitted as f64 * 100.0;
            println!("📊 接受率: {:.1}% | 🎯 效率: {:.2} 份额/分钟", 
                     accept_rate, total_shares as f64 / (uptime as f64 / 60.0).max(1.0));
        }
        
        // 显示每个矿机的详细信息
        if i % 4 == 0 { // 每分钟显示一次详细信息
            println!("   矿机详情:");
            for miner in &miners {
                let shares = miner.get_shares();
                println!("   • {}: {:.1} TH/s, {} 份额", miner.name, miner.hashrate, shares);
            }
        }
        println!("───────────────────────────────────────────────────────────");
    }

    // 停止所有矿机
    println!("\n🛑 正在停止虚拟挖矿...");
    for miner in &miners {
        miner.stop();
    }

    // 断开矿池连接
    stratum.disconnect();

    // 等待线程结束
    for handle in handles {
        let _ = handle.join();
    }

    // 最终统计
    let (submitted, accepted, rejected) = stratum.get_stats();
    let total_shares: u64 = miners.iter().map(|m| m.get_shares()).sum();
    let total_hashrate: f64 = miners.iter().map(|m| m.hashrate).sum();
    let total_time = start_time.elapsed().as_secs();
    
    println!("\n📈 最终统计:");
    println!("═══════════════════════════════════════════════════════════");
    println!("🕐 总运行时间: {}分{}秒", total_time / 60, total_time % 60);
    println!("💎 总算力: {:.1} TH/s", total_hashrate);
    println!("📊 本地份额: {}", total_shares);
    println!("📤 提交份额: {}", submitted);
    println!("✅ 接受份额: {}", accepted);
    println!("❌ 拒绝份额: {}", rejected);
    if submitted > 0 {
        println!("📈 接受率: {:.1}%", accepted as f64 / submitted as f64 * 100.0);
    }
    println!("═══════════════════════════════════════════════════════════");
    println!("👋 F2Pool 虚拟挖矿演示结束!");
    println!("");
    println!("💡 注意事项:");
    println!("   • 这是虚拟演示，未实际连接到 F2Pool");
    println!("   • 实际使用时需要有效的 F2Pool 账户");
    println!("   • 请确保你的用户名和矿工名称正确");
    println!("   • 真实挖矿需要 ASIC 硬件设备");
}
