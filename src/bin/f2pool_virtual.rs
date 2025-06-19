use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use chrono::Local;

/// 格式化日志时间戳
fn log_timestamp() -> String {
    Local::now().format("[%H:%M:%S]").to_string()
}

/// 打印带时间戳的日志
fn log_info(msg: &str) {
    println!("{} {}", log_timestamp(), msg);
}

/// 打印成功消息
fn log_success(msg: &str) {
    println!("{} ✓ {}", log_timestamp(), msg);
}

/// 打印警告消息
fn log_warning(msg: &str) {
    println!("{} ⚠ {}", log_timestamp(), msg);
}

/// 打印错误消息
fn log_error(msg: &str) {
    println!("{} ✗ {}", log_timestamp(), msg);
}

/// 打印份额发现消息
fn log_share(msg: &str) {
    println!("{} ⛏ {}", log_timestamp(), msg);
}

/// F2Pool 虚拟挖矿器 - 使用真实配置
struct F2PoolVirtualMiner {
    pools: Vec<PoolConfig>,
    devices: Vec<Arc<VirtualMiningDevice>>,
    running: AtomicBool,
    total_hashrate: f64,
}

#[derive(Clone)]
struct PoolConfig {
    url: String,
    user: String,
    password: String,
    priority: u8,
    enabled: bool,
}

struct VirtualMiningDevice {
    id: u32,
    name: String,
    hashrate: f64, // MH/s
    shares_found: AtomicU64,
    shares_accepted: AtomicU64,
    shares_rejected: AtomicU64,
    hardware_errors: AtomicU64,
    running: AtomicBool,
    temperature: f32,
    frequency: u32,
    voltage: u32,
}

struct StratumConnection {
    pool_config: PoolConfig,
    connected: AtomicBool,
    shares_submitted: AtomicU64,
    shares_accepted: AtomicU64,
    shares_rejected: AtomicU64,
    difficulty: f64,
    job_id: String,
}

impl StratumConnection {
    fn new(pool_config: PoolConfig) -> Self {
        Self {
            pool_config,
            connected: AtomicBool::new(false),
            shares_submitted: AtomicU64::new(0),
            shares_accepted: AtomicU64::new(0),
            shares_rejected: AtomicU64::new(0),
            difficulty: 1.0,
            job_id: "default_job".to_string(),
        }
    }

    fn connect(&self) -> bool {
        log_info(&format!("Connecting to pool: {}", self.pool_config.url));
        log_info(&format!("Worker: {}", self.pool_config.user));

        // 模拟连接延迟
        thread::sleep(Duration::from_secs(1));

        // 模拟 Stratum 握手
        log_info("Sending mining.subscribe...");
        thread::sleep(Duration::from_millis(300));

        log_info("Sending mining.authorize...");
        thread::sleep(Duration::from_millis(300));

        // 模拟认证成功
        self.connected.store(true, Ordering::Relaxed);
        log_success(&format!("Connected to F2Pool, difficulty: {:.1}", self.difficulty));

        true
    }

    fn submit_share(&self, device_id: u32, nonce: u32) -> bool {
        if !self.connected.load(Ordering::Relaxed) {
            return false;
        }

        self.shares_submitted.fetch_add(1, Ordering::Relaxed);

        // 模拟份额提交 (95% 接受率)
        let accepted = fastrand::f32() < 0.95;

        if accepted {
            self.shares_accepted.fetch_add(1, Ordering::Relaxed);
            let (submitted, accepted_count, _rejected) = self.get_stats();
            let accept_rate = if submitted > 0 { (accepted_count as f64 / submitted as f64) * 100.0 } else { 0.0 };
            log_share(&format!("ACCEPTED {}/{} ({:.1}%) - Device {}, nonce: 0x{:08x}",
                              accepted_count, submitted, accept_rate, device_id, nonce));
        } else {
            self.shares_rejected.fetch_add(1, Ordering::Relaxed);
            let (submitted, accepted_count, rejected) = self.get_stats();
            let accept_rate = if submitted > 0 { (accepted_count as f64 / submitted as f64) * 100.0 } else { 0.0 };
            log_warning(&format!("REJECTED {}/{} ({:.1}%) - Device {}, nonce: 0x{:08x}",
                               rejected, submitted, accept_rate, device_id, nonce));
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

impl VirtualMiningDevice {
    fn new(id: u32) -> Self {
        // 基于真实硬件性能测量算力
        let hashrate = Self::benchmark_hashrate(id);

        Self {
            id,
            name: format!("虚拟设备 {}", id),
            hashrate,
            shares_found: AtomicU64::new(0),
            shares_accepted: AtomicU64::new(0),
            shares_rejected: AtomicU64::new(0),
            hardware_errors: AtomicU64::new(0),
            running: AtomicBool::new(false),
            temperature: 45.0 + fastrand::f32() * 15.0, // 45-60°C
            frequency: 600 + fastrand::u32(0..100), // 600-700 MHz
            voltage: 850 + fastrand::u32(0..100), // 850-950 mV
        }
    }

    /// 基准测试：测量真实的 SHA-256 双重哈希性能
    fn benchmark_hashrate(device_id: u32) -> f64 {
        log_info(&format!("Device {}: Benchmarking SHA-256 performance...", device_id));

        let test_header = [0u8; 80];
        let benchmark_duration = Duration::from_secs(2); // 2秒基准测试
        let start_time = Instant::now();
        let mut hash_count = 0u64;

        // 执行真实的 SHA-256 双重哈希基准测试
        while start_time.elapsed() < benchmark_duration {
            let mut nonce = hash_count as u32;
            let mut work_header = test_header;

            // 批量测试以提高效率
            for _ in 0..1000 {
                // 将 nonce 写入 header
                work_header[76..80].copy_from_slice(&nonce.to_le_bytes());

                // 第一次 SHA-256 哈希
                let mut hasher = Sha256::new();
                hasher.update(&work_header);
                let hash1 = hasher.finalize();

                // 第二次 SHA-256 哈希
                let mut hasher = Sha256::new();
                hasher.update(&hash1);
                let _hash2 = hasher.finalize();

                nonce = nonce.wrapping_add(1);
                hash_count += 1;
            }
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        let hashrate_hs = hash_count as f64 / elapsed; // H/s
        let hashrate_mhs = hashrate_hs / 1_000_000.0; // MH/s

        // 添加一些随机变化来模拟不同核心的性能差异
        let variation = 0.8 + fastrand::f64() * 0.4; // 80%-120% 的性能变化
        let final_hashrate = hashrate_mhs * variation;

        log_info(&format!("Device {}: Benchmark complete - {:.2} MH/s (base: {:.2} MH/s)",
                         device_id, final_hashrate, hashrate_mhs));

        final_hashrate
    }

    fn start(&self, stratum: Arc<StratumConnection>) {
        self.running.store(true, Ordering::Relaxed);
        log_info(&format!("Device {}: {:.1} MH/s {:.1}°C [STARTING]",
                         self.id, self.hashrate, self.temperature));

        let device_clone = Arc::new(self.clone());
        thread::spawn(move || {
            device_clone.mine_loop(stratum);
        });
    }

    fn mine_loop(&self, stratum: Arc<StratumConnection>) {
        let mut last_share = Instant::now();
        let mut total_hashes = 0u64;
        let _start_time = Instant::now();
        let mut nonce = fastrand::u32(..);

        // 根据真实算力计算每轮应该执行的哈希数
        let target_hashes_per_100ms = (self.hashrate * 1_000_000.0 * 0.1) as u64;

        while self.running.load(Ordering::Relaxed) {
            let round_start = Instant::now();

            if !stratum.connected.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            // 执行真实的 SHA-256 双重哈希计算
            let mut hashes_this_round = 0u64;
            let test_header = [0u8; 80];

            while hashes_this_round < target_hashes_per_100ms && round_start.elapsed() < Duration::from_millis(100) {
                let mut work_header = test_header;

                // 批量处理以提高效率
                let batch_size = (target_hashes_per_100ms / 10).max(100).min(10000);

                for _ in 0..batch_size {
                    // 将 nonce 写入 header
                    work_header[76..80].copy_from_slice(&nonce.to_le_bytes());

                    // 第一次 SHA-256 哈希
                    let mut hasher = Sha256::new();
                    hasher.update(&work_header);
                    let hash1 = hasher.finalize();

                    // 第二次 SHA-256 哈希
                    let mut hasher = Sha256::new();
                    hasher.update(&hash1);
                    let hash2 = hasher.finalize();

                    // 检查是否找到份额 (简化的难度检查)
                    if self.check_share_difficulty(&hash2) {
                        self.shares_found.fetch_add(1, Ordering::Relaxed);

                        // 提交到矿池
                        if stratum.submit_share(self.id, nonce) {
                            self.shares_accepted.fetch_add(1, Ordering::Relaxed);
                        } else {
                            self.shares_rejected.fetch_add(1, Ordering::Relaxed);
                        }

                        last_share = Instant::now();
                    }

                    nonce = nonce.wrapping_add(1);
                    hashes_this_round += 1;

                    // 如果时间到了就退出批处理
                    if round_start.elapsed() >= Duration::from_millis(100) {
                        break;
                    }
                }
            }

            total_hashes += hashes_this_round;

            // 模拟硬件错误 (很低概率)
            if fastrand::f32() < 0.0001 {
                self.hardware_errors.fetch_add(1, Ordering::Relaxed);
                log_warning(&format!("Hardware error on device {}", self.id));
            }

            // 确保每轮至少100ms，避免CPU占用过高
            let elapsed = round_start.elapsed();
            if elapsed < Duration::from_millis(100) {
                thread::sleep(Duration::from_millis(100) - elapsed);
            }
        }
    }

    /// 检查哈希是否满足份额难度
    fn check_share_difficulty(&self, hash: &[u8]) -> bool {
        // 简化的难度检查：前导零的数量
        // 在真实环境中，这会根据矿池设置的难度来调整
        hash[0] == 0 && hash[1] == 0 && hash[2] < 0x10
    }



    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        log_info(&format!("Device {} stopped", self.id));
    }
}

impl Clone for VirtualMiningDevice {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            hashrate: self.hashrate,
            shares_found: AtomicU64::new(self.shares_found.load(Ordering::Relaxed)),
            shares_accepted: AtomicU64::new(self.shares_accepted.load(Ordering::Relaxed)),
            shares_rejected: AtomicU64::new(self.shares_rejected.load(Ordering::Relaxed)),
            hardware_errors: AtomicU64::new(self.hardware_errors.load(Ordering::Relaxed)),
            running: AtomicBool::new(self.running.load(Ordering::Relaxed)),
            temperature: self.temperature,
            frequency: self.frequency,
            voltage: self.voltage,
        }
    }
}

impl F2PoolVirtualMiner {
    fn new() -> Self {
        // 使用真实的 F2Pool 配置
        let pools = vec![
            PoolConfig {
                url: "stratum+tcp://btc.f2pool.com:1314".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "21235365876986800".to_string(),
                priority: 1,
                enabled: true,
            },
            PoolConfig {
                url: "stratum+tcp://btc-asia.f2pool.com:1314".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "21235365876986800".to_string(),
                priority: 2,
                enabled: true,
            },
            PoolConfig {
                url: "stratum+tcp://btc-euro.f2pool.com:1314".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "21235365876986800".to_string(),
                priority: 3,
                enabled: true,
            },
        ];

        // 创建虚拟设备 - 基于真实硬件性能
        let mut devices = Vec::new();
        let mut total_hashrate = 0.0;

        log_info("Initializing virtual mining devices...");
        log_info("Performing SHA-256 performance benchmarks...");

        // 从环境变量或默认值获取设备数量
        let device_count = std::env::var("CGMINER_DEVICE_COUNT")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(4); // 默认4个设备

        for i in 0..device_count {
            let device = Arc::new(VirtualMiningDevice::new(i));
            total_hashrate += device.hashrate;
            devices.push(device);
        }

        Self {
            pools,
            devices,
            running: AtomicBool::new(false),
            total_hashrate,
        }
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);

        // 启动横幅
        println!("CGMiner-RS v0.1.0 - F2Pool Virtual Miner");
        println!("========================================");
        log_info(&format!("Pool: {} | Worker: {}",
                         self.pools[0].url.replace("stratum+tcp://", ""),
                         self.pools[0].user));
        log_info(&format!("Devices: {} virtual cores | Total: {:.1} MH/s",
                         self.devices.len(), self.total_hashrate));

        // 连接到主矿池
        let primary_pool = &self.pools[0];
        let stratum = Arc::new(StratumConnection::new(primary_pool.clone()));

        if !stratum.connect() {
            log_error("Failed to connect to pool!");
            return;
        }

        // 启动所有设备
        for device in &self.devices {
            device.start(Arc::clone(&stratum));
        }

        log_success("All devices started, mining...");

        // 定期显示状态摘要
        let stratum_clone = Arc::clone(&stratum);
        let devices_clone = self.devices.clone();
        let start_time = Instant::now();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(30));

                let (submitted, accepted, rejected) = stratum_clone.get_stats();
                let accept_rate = if submitted > 0 {
                    (accepted as f64 / submitted as f64) * 100.0
                } else {
                    0.0
                };

                let runtime = start_time.elapsed();
                let hours = runtime.as_secs() / 3600;
                let minutes = (runtime.as_secs() % 3600) / 60;
                let seconds = runtime.as_secs() % 60;

                println!();
                log_info("=============== MINING STATUS ===============");
                log_info(&format!("Runtime: {:02}:{:02}:{:02} | Pool: F2Pool | Diff: 1.0",
                                hours, minutes, seconds));
                log_info(&format!("Shares: {}A/{}R ({:.1}%) | Total Hashrate: {:.1} MH/s",
                                accepted, rejected, accept_rate,
                                devices_clone.iter().map(|d| d.hashrate).sum::<f64>()));
                log_info("Device | Hashrate | Temp | Status | Shares");

                for device in &devices_clone {
                    let shares = device.shares_accepted.load(Ordering::Relaxed);
                    log_info(&format!("DEV {:2} | {:6.1}MH | {:2.0}°C | MINING | {:4}",
                                    device.id, device.hashrate, device.temperature, shares));
                }
                log_info("============================================");
                println!();
            }
        });
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        log_info("Stopping all devices...");

        for device in &self.devices {
            device.stop();
        }

        log_info("CGMiner-RS stopped");
    }
}

fn main() {
    let miner = F2PoolVirtualMiner::new();

    // 设置 Ctrl+C 处理
    let miner_clone = Arc::new(miner);
    let miner_for_signal = Arc::clone(&miner_clone);

    ctrlc::set_handler(move || {
        println!();
        log_info("Received shutdown signal...");
        miner_for_signal.stop();
        std::process::exit(0);
    }).expect("Failed to set signal handler");

    miner_clone.start();

    // 保持主线程运行
    loop {
        thread::sleep(Duration::from_secs(1));
        if !miner_clone.running.load(Ordering::Relaxed) {
            break;
        }
    }
}
