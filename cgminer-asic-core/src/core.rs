//! ASIC挖矿核心实现

use cgminer_core::{
    MiningCore, CoreInfo, CoreCapabilities, CoreConfig, CoreStats, CoreError,
    DeviceInfo, MiningDevice, Work, MiningResult
};
use crate::device::AsicDevice;
use crate::hardware::{HardwareInterface, MockHardwareInterface, RealHardwareInterface};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

/// ASIC挖矿核心
pub struct AsicMiningCore {
    /// 核心信息
    core_info: CoreInfo,
    /// 核心能力
    capabilities: CoreCapabilities,
    /// 核心配置
    config: Option<CoreConfig>,
    /// 设备列表
    devices: Arc<Mutex<HashMap<u32, Box<dyn MiningDevice>>>>,
    /// 硬件接口
    hardware: Arc<dyn HardwareInterface>,
    /// 核心统计信息
    stats: Arc<RwLock<CoreStats>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 启动时间
    start_time: Option<SystemTime>,
}

impl AsicMiningCore {
    /// 创建新的ASIC挖矿核心
    pub fn new(name: String) -> Self {
        let core_info = CoreInfo::new(
            name.clone(),
            cgminer_core::CoreType::Asic,
            crate::VERSION.to_string(),
            "ASIC挖矿核心，支持各种ASIC硬件设备的挖矿操作".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["asic".to_string(), "maijie-l7".to_string()],
        );

        let capabilities = CoreCapabilities {
            supports_auto_tuning: true,
            supports_temperature_monitoring: true,
            supports_voltage_control: true,
            supports_frequency_control: true,
            supports_fan_control: true,
            supports_multiple_chains: true,
            max_devices: Some(16), // ASIC核心支持最多16个设备
            supported_algorithms: vec!["SHA256".to_string()],
        };

        let stats = CoreStats::new(name);

        // 根据编译特性选择硬件接口
        let hardware: Arc<dyn HardwareInterface> = if cfg!(feature = "mock-hardware") {
            Arc::new(MockHardwareInterface::new())
        } else {
            Arc::new(RealHardwareInterface::new())
        };

        Self {
            core_info,
            capabilities,
            config: None,
            devices: Arc::new(Mutex::new(HashMap::new())),
            hardware,
            stats: Arc::new(RwLock::new(stats)),
            running: Arc::new(RwLock::new(false)),
            start_time: None,
        }
    }

    /// 创建ASIC设备
    async fn create_asic_devices(&self, config: &CoreConfig) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        let mut devices = Vec::new();

        // 从配置中获取链数量
        let chain_count = config.custom_params
            .get("chain_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;

        info!("创建 {} 个ASIC设备链", chain_count);

        for i in 0..chain_count {
            let device_config = if (i as usize) < config.devices.len() {
                config.devices[i as usize].clone()
            } else {
                // 使用默认配置，但确保enabled状态来自配置
                let mut default_config = cgminer_core::DeviceConfig::default();
                default_config.chain_id = i as u8;
                default_config.frequency = 650;
                default_config.voltage = 900;
                default_config.auto_tune = true;
                default_config.chip_count = 126; // Maijie L7 典型芯片数量
                default_config.temperature_limit = 85.0;
                default_config.fan_speed = Some(70);
                // enabled状态保持默认值，不硬编码
                default_config
            };

            let device_info = DeviceInfo::new(
                2000 + i,
                format!("ASIC Chain {}", i),
                "asic".to_string(),
                i as u8,
            );

            let device = AsicDevice::new(
                device_info,
                device_config,
                self.hardware.clone(),
            ).await?;

            devices.push(Box::new(device) as Box<dyn MiningDevice>);
        }

        Ok(devices)
    }

    /// 更新统计信息
    async fn update_stats(&self) -> Result<(), CoreError> {
        let devices = self.devices.lock().await;
        let mut total_hashrate = 0.0;
        let mut total_accepted = 0;
        let mut total_rejected = 0;
        let mut total_errors = 0;
        let mut active_devices = 0;

        for device in devices.values() {
            if let Ok(stats) = device.get_stats().await {
                total_hashrate += stats.current_hashrate.hashes_per_second;
                total_accepted += stats.accepted_work;
                total_rejected += stats.rejected_work;
                total_errors += stats.hardware_errors;
                active_devices += 1;
            }
        }

        let mut stats = self.stats.write().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire write lock: {}", e))
        })?;

        stats.device_count = devices.len() as u32;
        stats.active_devices = active_devices;
        stats.total_hashrate = total_hashrate;
        stats.average_hashrate = if active_devices > 0 {
            total_hashrate / active_devices as f64
        } else {
            0.0
        };
        stats.accepted_work = total_accepted;
        stats.rejected_work = total_rejected;
        stats.hardware_errors = total_errors;

        if let Some(start_time) = self.start_time {
            stats.uptime = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::from_secs(0));
        }

        stats.last_updated = SystemTime::now();

        Ok(())
    }
}

#[async_trait]
impl MiningCore for AsicMiningCore {
    /// 获取核心信息
    fn get_info(&self) -> &CoreInfo {
        &self.core_info
    }

    /// 获取核心能力
    fn get_capabilities(&self) -> &CoreCapabilities {
        &self.capabilities
    }

    /// 初始化核心
    async fn initialize(&mut self, config: CoreConfig) -> Result<(), CoreError> {
        info!("初始化ASIC挖矿核心: {}", config.name);

        // 验证配置
        self.validate_config(&config)?;

        // 初始化硬件接口
        if let Err(e) = self.hardware.initialize().await {
            error!("初始化硬件接口失败: {}", e);
            return Err(CoreError::Device(e));
        }

        // 创建设备
        let devices = self.create_asic_devices(&config).await?;

        // 存储设备
        {
            let mut device_map = self.devices.lock().await;
            for device in devices {
                let device_id = device.device_id();
                device_map.insert(device_id, device);
            }
        }

        // 初始化所有设备
        {
            let mut device_map = self.devices.lock().await;
            for (device_id, device) in device_map.iter_mut() {
                let device_config = config.devices
                    .iter()
                    .find(|dc| dc.chain_id == (*device_id - 2000) as u8)
                    .cloned()
                    .unwrap_or_default();

                if let Err(e) = device.initialize(device_config).await {
                    error!("初始化设备 {} 失败: {}", device_id, e);
                    return Err(CoreError::Device(e));
                }
            }
        }

        self.config = Some(config);
        info!("ASIC挖矿核心初始化完成");
        Ok(())
    }

    /// 启动核心
    async fn start(&mut self) -> Result<(), CoreError> {
        info!("启动ASIC挖矿核心");

        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;

            if *running {
                return Err(CoreError::runtime("核心已经在运行中"));
            }
            *running = true;
        }

        // 启动所有设备
        {
            let mut devices = self.devices.lock().await;
            for (device_id, device) in devices.iter_mut() {
                if let Err(e) = device.start().await {
                    error!("启动设备 {} 失败: {}", device_id, e);
                    // 继续启动其他设备，不因为一个设备失败而停止
                }
            }
        }

        self.start_time = Some(SystemTime::now());
        info!("ASIC挖矿核心启动完成");
        Ok(())
    }

    /// 停止核心
    async fn stop(&mut self) -> Result<(), CoreError> {
        info!("停止ASIC挖矿核心");

        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            *running = false;
        }

        // 停止所有设备
        {
            let mut devices = self.devices.lock().await;
            for (device_id, device) in devices.iter_mut() {
                if let Err(e) = device.stop().await {
                    error!("停止设备 {} 失败: {}", device_id, e);
                }
            }
        }

        info!("ASIC挖矿核心已停止");
        Ok(())
    }

    /// 重启核心
    async fn restart(&mut self) -> Result<(), CoreError> {
        info!("重启ASIC挖矿核心");
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.start().await?;
        Ok(())
    }

    /// 扫描设备
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, CoreError> {
        debug!("扫描ASIC设备");

        let devices = self.devices.lock().await;
        let mut device_infos = Vec::new();

        for device in devices.values() {
            match device.get_info().await {
                Ok(info) => device_infos.push(info),
                Err(e) => warn!("获取设备信息失败: {}", e),
            }
        }

        Ok(device_infos)
    }

    /// 创建设备
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, CoreError> {
        info!("创建ASIC设备: {}", device_info.name);

        let device_config = cgminer_core::DeviceConfig::default();
        let device = AsicDevice::new(
            device_info,
            device_config,
            self.hardware.clone(),
        ).await?;

        Ok(Box::new(device))
    }

    /// 获取所有设备
    async fn get_devices(&self) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        Err(CoreError::runtime("get_devices 方法暂未实现"))
    }

    /// 获取设备数量
    async fn device_count(&self) -> Result<u32, CoreError> {
        let devices = self.devices.lock().await;
        Ok(devices.len() as u32)
    }

    /// 提交工作到所有设备
    async fn submit_work(&mut self, work: Work) -> Result<(), CoreError> {
        debug!("提交工作到所有ASIC设备: {}", work.id);

        let mut devices = self.devices.lock().await;
        for (device_id, device) in devices.iter_mut() {
            if let Err(e) = device.submit_work(work.clone()).await {
                warn!("向设备 {} 提交工作失败: {}", device_id, e);
            }
        }

        Ok(())
    }

    /// 收集所有设备的挖矿结果
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>, CoreError> {
        let mut results = Vec::new();
        let mut devices = self.devices.lock().await;

        for device in devices.values_mut() {
            match device.get_result().await {
                Ok(Some(result)) => results.push(result),
                Ok(None) => {}, // 没有结果
                Err(e) => warn!("获取设备挖矿结果失败: {}", e),
            }
        }

        debug!("收集到 {} 个挖矿结果", results.len());
        Ok(results)
    }

    /// 获取核心统计信息
    async fn get_stats(&self) -> Result<CoreStats, CoreError> {
        self.update_stats().await?;
        let stats = self.stats.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(stats.clone())
    }

    /// 健康检查
    async fn health_check(&self) -> Result<bool, CoreError> {
        let devices = self.devices.lock().await;
        let mut healthy_devices = 0;

        for device in devices.values() {
            match device.health_check().await {
                Ok(true) => healthy_devices += 1,
                Ok(false) => {},
                Err(e) => warn!("设备健康检查失败: {}", e),
            }
        }

        // 如果至少有一半设备健康，则认为核心健康
        let health_threshold = (devices.len() + 1) / 2;
        Ok(healthy_devices >= health_threshold)
    }

    /// 验证配置
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError> {
        if config.name.is_empty() {
            return Err(CoreError::config("核心名称不能为空"));
        }

        // 验证链数量
        if let Some(chain_count) = config.custom_params.get("chain_count") {
            if let Some(count) = chain_count.as_u64() {
                if count == 0 {
                    return Err(CoreError::config("ASIC链数量不能为0"));
                }
                if count > 16 {
                    return Err(CoreError::config("ASIC链数量不能超过16"));
                }
            }
        }

        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig {
        use std::collections::HashMap;

        let mut custom_params = HashMap::new();
        custom_params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        custom_params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(6_000_000)));
        custom_params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(115200)));

        CoreConfig {
            name: "asic-core".to_string(),
            enabled: true,
            devices: vec![cgminer_core::DeviceConfig::default(); 3],
            custom_params,
        }
    }

    /// 关闭核心
    async fn shutdown(&mut self) -> Result<(), CoreError> {
        info!("关闭ASIC挖矿核心");
        self.stop().await?;

        // 关闭硬件接口
        if let Err(e) = self.hardware.shutdown().await {
            error!("关闭硬件接口失败: {}", e);
        }

        // 清空设备列表
        {
            let mut devices = self.devices.lock().await;
            devices.clear();
        }

        info!("ASIC挖矿核心已关闭");
        Ok(())
    }
}
