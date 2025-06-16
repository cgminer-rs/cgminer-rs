use crate::error::DeviceError;
use crate::device::{
    DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult,
    MiningDevice, DeviceDriver, DeviceConfig
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use fastrand;
use sha2::{Sha256, Digest};

/// 虚拟设备驱动
pub struct VirtualDeviceDriver {
    /// 驱动版本
    version: &'static str,
}

impl VirtualDeviceDriver {
    pub fn new() -> Self {
        Self {
            version: "1.0.0",
        }
    }
}

#[async_trait]
impl DeviceDriver for VirtualDeviceDriver {
    fn driver_name(&self) -> &'static str {
        "virtual-device"
    }

    fn supported_devices(&self) -> Vec<&'static str> {
        vec!["virtual"]
    }

    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, DeviceError> {
        info!("Scanning for virtual devices");

        // 创建多个虚拟设备用于测试
        let mut devices = Vec::new();

        for i in 0..4 {
            let device_info = DeviceInfo::new(
                1000 + i,
                format!("Virtual Device {}", i),
                "virtual".to_string(),
                i as u8,
            );
            devices.push(device_info);
        }

        info!("Found {} virtual devices", devices.len());
        Ok(devices)
    }

    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, DeviceError> {
        info!("Creating virtual device: {}", device_info.name);

        let device = VirtualDevice::new(device_info).await?;
        Ok(Box::new(device))
    }

    fn validate_config(&self, _config: &DeviceConfig) -> Result<(), DeviceError> {
        // 虚拟设备接受任何配置
        Ok(())
    }

    fn default_config(&self) -> DeviceConfig {
        DeviceConfig {
            chain_id: 0,
            enabled: true,
            frequency: 600, // 虚拟设备默认频率
            voltage: 900,   // 虚拟设备默认电压
            auto_tune: false, // 虚拟设备不需要自动调优
            chip_count: 64,   // 虚拟芯片数量
            temperature_limit: 80.0,
            fan_speed: Some(50),
        }
    }

    fn version(&self) -> &'static str {
        self.version
    }
}

/// 虚拟挖矿设备
pub struct VirtualDevice {
    /// 设备信息
    device_info: Arc<RwLock<DeviceInfo>>,
    /// 设备配置
    config: Arc<RwLock<DeviceConfig>>,
    /// 设备统计
    stats: Arc<RwLock<DeviceStats>>,
    /// 当前工作
    current_work: Arc<RwLock<Option<Work>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 挖矿任务句柄
    mining_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 模拟算力 (MH/s)
    simulated_hashrate: f64,
    /// 启动时间
    start_time: Arc<RwLock<Option<Instant>>>,
}

impl VirtualDevice {
    /// 执行真正的 Bitcoin SHA-256 双重哈希挖矿
    pub fn mine_bitcoin_block(header: &[u8; 80], target: &[u8; 32], start_nonce: u32, max_iterations: u32) -> Option<u32> {
        let mut work_header = *header;

        for i in 0..max_iterations {
            let nonce = start_nonce.wrapping_add(i);

            // 将 nonce 写入 header 的最后 4 个字节 (小端序)
            work_header[76..80].copy_from_slice(&nonce.to_le_bytes());

            // 第一次 SHA-256 哈希
            let mut hasher = Sha256::new();
            hasher.update(&work_header);
            let hash1 = hasher.finalize();

            // 第二次 SHA-256 哈希
            let mut hasher = Sha256::new();
            hasher.update(&hash1);
            let hash2 = hasher.finalize();

            // 检查哈希是否满足目标难度
            // Bitcoin 使用小端序比较
            if Self::hash_meets_target(&hash2, target) {
                return Some(nonce);
            }
        }

        None
    }

    /// 检查哈希是否满足目标难度
    pub fn hash_meets_target(hash: &[u8], target: &[u8; 32]) -> bool {
        // Bitcoin 哈希比较：哈希值必须小于等于目标值
        // 注意：Bitcoin 使用小端序，但这里我们直接比较字节数组
        for i in (0..32).rev() {
            match hash[i].cmp(&target[i]) {
                std::cmp::Ordering::Less => return true,
                std::cmp::Ordering::Greater => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }
        true // 完全相等也算满足
    }

    /// 根据难度值生成目标值
    pub fn difficulty_to_target(difficulty: f64) -> [u8; 32] {
        // 简化的难度到目标转换
        // 实际 Bitcoin 使用更复杂的算法
        let mut target = [0xFFu8; 32];

        if difficulty <= 1.0 {
            return target; // 最低难度
        }

        // 计算前导零的数量
        let leading_zeros = (difficulty.log2() / 8.0) as usize;
        let leading_zeros = leading_zeros.min(31);

        // 设置前导零
        for i in 0..leading_zeros {
            target[31 - i] = 0;
        }

        // 设置剩余部分
        if leading_zeros < 32 {
            let remaining_difficulty = difficulty / (1u64 << (leading_zeros * 8)) as f64;
            let target_value = (0xFF as f64 / remaining_difficulty) as u8;
            target[31 - leading_zeros] = target_value;
        }

        target
    }

    pub async fn new(mut device_info: DeviceInfo) -> Result<Self, DeviceError> {
        // 设置虚拟设备的初始状态
        device_info.chip_count = 64;
        device_info.temperature = Some(45.0);
        device_info.fan_speed = Some(50);
        device_info.voltage = Some(900);
        device_info.frequency = Some(600);

        // 模拟不同的算力 (50-200 MH/s)
        let simulated_hashrate = 50.0 + fastrand::f64() * 150.0;
        device_info.hashrate = simulated_hashrate;

        Ok(Self {
            device_info: Arc::new(RwLock::new(device_info)),
            config: Arc::new(RwLock::new(DeviceConfig::default())),
            stats: Arc::new(RwLock::new(DeviceStats::new())),
            current_work: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
            mining_handle: Arc::new(Mutex::new(None)),
            simulated_hashrate,
            start_time: Arc::new(RwLock::new(None)),
        })
    }

    /// 真正的 Bitcoin 挖矿过程
    async fn simulate_mining(&self) {
        let device_id = {
            let info = self.device_info.read().await;
            info.id
        };

        info!("Starting Bitcoin mining for virtual device {}", device_id);

        while *self.running.read().await {
            // 检查是否有工作
            let work = {
                let current_work = self.current_work.read().await;
                current_work.clone()
            };

            if let Some(work) = work {
                // 从难度生成目标值
                let target = Self::difficulty_to_target(work.difficulty);

                // 模拟算力：每秒处理的哈希数
                let hashes_per_second = (self.simulated_hashrate * 1_000_000.0) as u32; // 转换为 H/s
                let batch_size = (hashes_per_second / 10).max(1000); // 每100ms处理的哈希数

                let mut total_hashes = 0u64;
                let mut start_nonce = fastrand::u32(..);
                let start_time = Instant::now();

                loop {
                    if !*self.running.read().await {
                        break;
                    }

                    // 执行真正的 Bitcoin 挖矿
                    let found_nonce = Self::mine_bitcoin_block(&work.header, &target, start_nonce, batch_size);

                    total_hashes += batch_size as u64;
                    start_nonce = start_nonce.wrapping_add(batch_size);

                    // 更新统计信息
                    {
                        let mut stats = self.stats.write().await;
                        stats.record_hash(batch_size as u64);
                    }

                    if let Some(nonce) = found_nonce {
                        // 找到有效的 nonce！
                        info!("Virtual device {} found valid nonce: 0x{:08x} after {} hashes",
                              device_id, nonce, total_hashes);

                        // 更新统计信息
                        {
                            let mut stats = self.stats.write().await;
                            stats.record_valid_nonce();
                        }

                        // 更新设备信息
                        {
                            let mut info = self.device_info.write().await;
                            info.increment_accepted_shares();
                            info.last_share_time = Some(SystemTime::now());
                        }

                        // 创建挖矿结果
                        let result = MiningResult::new(work.id, device_id, nonce, work.difficulty);
                        debug!("Created mining result: {:?}", result);

                        // 找到有效 nonce 后，等待新工作
                        break;
                    }

                    // 模拟挖矿延迟 (100ms 批次)
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // 更新设备信息
                    {
                        let mut info = self.device_info.write().await;

                        // 模拟温度变化 (基于工作负载)
                        let elapsed = start_time.elapsed().as_secs_f32();
                        let base_temp = 45.0;
                        let load_temp = (total_hashes as f32 / 1_000_000.0) * 0.1; // 每百万哈希增加0.1度
                        let temp = base_temp + load_temp + fastrand::f32() * 5.0; // 添加一些随机变化
                        info.update_temperature(temp.min(85.0));

                        // 更新实时算力
                        if elapsed > 0.0 {
                            let current_hashrate = (total_hashes as f64) / (elapsed as f64) / 1_000_000.0; // MH/s
                            info.update_hashrate(current_hashrate);
                        }
                    }

                    // 检查是否应该继续挖这个工作（避免过时的工作）
                    if start_time.elapsed() > Duration::from_secs(30) {
                        debug!("Work timeout for virtual device {}, waiting for new work", device_id);
                        break;
                    }
                }
            } else {
                // 没有工作时等待
                tokio::time::sleep(Duration::from_millis(100)).await;

                // 空闲时降低温度
                {
                    let mut info = self.device_info.write().await;
                    let current_temp = info.temperature.unwrap_or(45.0);
                    let new_temp = (current_temp - 0.5).max(35.0); // 每100ms降低0.5度，最低35度
                    info.update_temperature(new_temp);
                    info.update_hashrate(0.0); // 空闲时算力为0
                }
            }
        }

        info!("Bitcoin mining stopped for virtual device {}", device_id);
    }
}

#[async_trait]
impl MiningDevice for VirtualDevice {
    fn device_id(&self) -> u32 {
        // 这里需要同步访问，但为了简化，我们使用一个固定值
        // 在实际实现中，应该使用Arc<AtomicU32>或其他同步原语
        1000 // 临时实现
    }

    async fn get_info(&self) -> Result<DeviceInfo, DeviceError> {
        let info = self.device_info.read().await;
        Ok(info.clone())
    }

    async fn initialize(&mut self, config: DeviceConfig) -> Result<(), DeviceError> {
        info!("Initializing virtual device {}", self.device_id());

        // 保存配置
        *self.config.write().await = config.clone();

        // 更新设备信息
        {
            let mut info = self.device_info.write().await;
            info.update_status(DeviceStatus::Idle);
            info.frequency = Some(config.frequency);
            info.voltage = Some(config.voltage);
            info.chip_count = config.chip_count;
            info.temperature = Some(45.0); // 初始温度
            info.fan_speed = config.fan_speed;
        }

        info!("Virtual device {} initialized successfully", self.device_id());
        Ok(())
    }

    async fn start(&mut self) -> Result<(), DeviceError> {
        info!("Starting virtual device {}", self.device_id());

        // 设置运行状态
        *self.running.write().await = true;
        *self.start_time.write().await = Some(Instant::now());

        // 更新设备状态
        {
            let mut info = self.device_info.write().await;
            info.update_status(DeviceStatus::Mining);
        }

        // 启动挖矿模拟任务
        let device_clone = Arc::new(self.clone());
        let handle = tokio::spawn(async move {
            device_clone.simulate_mining().await;
        });

        *self.mining_handle.lock().await = Some(handle);

        info!("Virtual device {} started successfully", self.device_id());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DeviceError> {
        info!("Stopping virtual device {}", self.device_id());

        // 设置停止状态
        *self.running.write().await = false;

        // 停止挖矿任务
        if let Some(handle) = self.mining_handle.lock().await.take() {
            handle.abort();
        }

        // 更新设备状态
        {
            let mut info = self.device_info.write().await;
            info.update_status(DeviceStatus::Idle);
        }

        info!("Virtual device {} stopped successfully", self.device_id());
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), DeviceError> {
        info!("Restarting virtual device {}", self.device_id());

        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(1000)).await; // 模拟重启延迟
        self.start().await?;

        // 更新统计信息
        {
            let mut stats = self.stats.write().await;
            stats.record_restart();
        }

        info!("Virtual device {} restarted successfully", self.device_id());
        Ok(())
    }

    async fn submit_work(&mut self, work: Work) -> Result<(), DeviceError> {
        debug!("Submitting work to virtual device {}: {}", self.device_id(), work.job_id);

        // 检查工作是否过期
        if work.is_expired() {
            return Err(DeviceError::InvalidConfig {
                reason: format!("Work {} has expired", work.id),
            });
        }

        // 保存当前工作
        *self.current_work.write().await = Some(work);

        Ok(())
    }

    async fn get_result(&mut self) -> Result<Option<MiningResult>, DeviceError> {
        // 虚拟设备通过挖矿模拟任务生成结果
        // 这里简化实现，实际应该有结果队列
        Ok(None)
    }

    async fn get_status(&self) -> Result<DeviceStatus, DeviceError> {
        let info = self.device_info.read().await;
        Ok(info.status.clone())
    }

    async fn get_temperature(&self) -> Result<f32, DeviceError> {
        let info = self.device_info.read().await;
        Ok(info.temperature.unwrap_or(45.0))
    }

    async fn get_hashrate(&self) -> Result<f64, DeviceError> {
        let info = self.device_info.read().await;
        Ok(info.hashrate)
    }

    async fn get_stats(&self) -> Result<DeviceStats, DeviceError> {
        let mut stats = self.stats.read().await.clone();

        // 更新运行时间
        if let Some(start_time) = *self.start_time.read().await {
            stats.uptime_seconds = start_time.elapsed().as_secs();
        }

        Ok(stats)
    }

    async fn set_frequency(&mut self, frequency: u32) -> Result<(), DeviceError> {
        info!("Setting frequency for virtual device {} to {} MHz", self.device_id(), frequency);

        // 更新配置
        {
            let mut config = self.config.write().await;
            config.frequency = frequency;
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().await;
            info.frequency = Some(frequency);
            info.updated_at = SystemTime::now();
        }

        // 模拟频率变化对算力的影响
        let frequency_factor = frequency as f64 / 600.0; // 基准频率600MHz
        let new_hashrate = self.simulated_hashrate * frequency_factor;

        {
            let mut info = self.device_info.write().await;
            info.update_hashrate(new_hashrate);
        }

        Ok(())
    }

    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError> {
        info!("Setting voltage for virtual device {} to {} mV", self.device_id(), voltage);

        // 更新配置
        {
            let mut config = self.config.write().await;
            config.voltage = voltage;
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().await;
            info.voltage = Some(voltage);
            info.updated_at = SystemTime::now();
        }

        // 模拟电压变化对温度的影响
        let voltage_factor = voltage as f32 / 900.0; // 基准电压900mV
        let base_temp = 45.0;
        let new_temp = base_temp + (voltage_factor - 1.0) * 20.0; // 电压高温度高

        {
            let mut info = self.device_info.write().await;
            info.update_temperature(new_temp.max(30.0).min(90.0)); // 限制温度范围
        }

        Ok(())
    }

    async fn set_fan_speed(&mut self, speed: u32) -> Result<(), DeviceError> {
        info!("Setting fan speed for virtual device {} to {}%", self.device_id(), speed);

        // 更新配置
        {
            let mut config = self.config.write().await;
            config.fan_speed = Some(speed);
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().await;
            info.fan_speed = Some(speed);
            info.updated_at = SystemTime::now();
        }

        // 模拟风扇速度对温度的影响
        let current_temp = {
            let info = self.device_info.read().await;
            info.temperature.unwrap_or(45.0)
        };

        let fan_factor = speed as f32 / 100.0; // 风扇速度百分比
        let cooling_effect = (1.0 - fan_factor) * 10.0; // 最多降温10度
        let new_temp = (current_temp + cooling_effect).max(30.0).min(90.0);

        {
            let mut info = self.device_info.write().await;
            info.update_temperature(new_temp);
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, DeviceError> {
        let info = self.device_info.read().await;

        // 检查设备状态
        let status_ok = matches!(info.status, DeviceStatus::Idle | DeviceStatus::Mining);

        // 检查温度
        let temp_ok = info.temperature.map_or(true, |t| t < 85.0);

        // 检查错误率
        let error_rate_ok = info.get_hardware_error_rate() < 5.0; // 硬件错误率小于5%

        Ok(status_ok && temp_ok && error_rate_ok)
    }

    async fn reset_stats(&mut self) -> Result<(), DeviceError> {
        info!("Resetting stats for virtual device {}", self.device_id());

        // 重置统计信息
        *self.stats.write().await = DeviceStats::new();

        // 重置设备信息中的统计数据
        {
            let mut info = self.device_info.write().await;
            info.accepted_shares = 0;
            info.rejected_shares = 0;
            info.hardware_errors = 0;
            info.last_share_time = None;
            info.updated_at = SystemTime::now();
        }

        Ok(())
    }
}

// 为了支持clone，需要实现Clone trait
impl Clone for VirtualDevice {
    fn clone(&self) -> Self {
        Self {
            device_info: Arc::clone(&self.device_info),
            config: Arc::clone(&self.config),
            stats: Arc::clone(&self.stats),
            current_work: Arc::clone(&self.current_work),
            running: Arc::clone(&self.running),
            mining_handle: Arc::clone(&self.mining_handle),
            simulated_hashrate: self.simulated_hashrate,
            start_time: Arc::clone(&self.start_time),
        }
    }
}
