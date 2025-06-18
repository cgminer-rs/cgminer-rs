use crate::config::{DeviceConfig, ChainConfig};
use crate::error::{DeviceError, MiningError};
use crate::device::{
    DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult,
    MiningDevice, DeviceDriver
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// 设备管理器
pub struct DeviceManager {
    /// 设备列表
    devices: Arc<RwLock<HashMap<u32, Arc<Mutex<Box<dyn MiningDevice>>>>>>,
    /// 设备信息缓存
    device_info: Arc<RwLock<HashMap<u32, DeviceInfo>>>,
    /// 设备统计信息
    device_stats: Arc<RwLock<HashMap<u32, DeviceStats>>>,
    /// 设备驱动
    drivers: Vec<Box<dyn DeviceDriver>>,
    /// 配置
    config: DeviceConfig,
    /// 工作队列发送器
    work_sender: Option<mpsc::UnboundedSender<(u32, Work)>>,
    /// 结果队列接收器
    result_receiver: Option<mpsc::UnboundedReceiver<MiningResult>>,
    /// 监控任务句柄
    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl DeviceManager {
    /// 创建新的设备管理器
    pub fn new(config: DeviceConfig) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            device_info: Arc::new(RwLock::new(HashMap::new())),
            device_stats: Arc::new(RwLock::new(HashMap::new())),
            drivers: Vec::new(),
            config,
            work_sender: None,
            result_receiver: None,
            monitoring_handle: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 注册设备驱动
    pub fn register_driver(&mut self, driver: Box<dyn DeviceDriver>) {
        info!("Registering device driver: {}", driver.driver_name());
        self.drivers.push(driver);
    }

    /// 初始化设备管理器
    pub async fn initialize(&mut self) -> Result<(), DeviceError> {
        info!("Initializing device manager");

        // 扫描设备
        self.scan_devices().await?;

        // 初始化所有设备
        self.initialize_devices().await?;

        info!("Device manager initialized successfully");
        Ok(())
    }

    /// 扫描设备
    pub async fn scan_devices(&mut self) -> Result<Vec<DeviceInfo>, DeviceError> {
        info!("Scanning for devices");
        let mut all_devices = Vec::new();

        for driver in &self.drivers {
            match driver.scan_devices().await {
                Ok(devices) => {
                    info!("Found {} devices with driver {}", devices.len(), driver.driver_name());
                    all_devices.extend(devices);
                }
                Err(e) => {
                    warn!("Failed to scan devices with driver {}: {}", driver.driver_name(), e);
                }
            }
        }

        // 更新设备信息缓存
        let mut device_info = self.device_info.write().await;
        for device in &all_devices {
            device_info.insert(device.id, device.clone());
        }

        info!("Total {} devices found", all_devices.len());
        Ok(all_devices)
    }

    /// 初始化所有设备
    pub async fn initialize_devices(&mut self) -> Result<(), DeviceError> {
        info!("Initializing devices");
        let device_info = self.device_info.read().await;
        let mut devices = self.devices.write().await;
        let mut stats = self.device_stats.write().await;

        for (device_id, info) in device_info.iter() {
            // 查找合适的驱动
            let driver = self.drivers.iter()
                .find(|d| d.supported_devices().contains(&info.device_type.as_str()))
                .ok_or_else(|| DeviceError::InitializationFailed {
                    device_id: *device_id,
                    reason: format!("No driver found for device type: {}", info.device_type),
                })?;

            // 创建设备实例
            match driver.create_device(info.clone()).await {
                Ok(mut device) => {
                    // 获取设备配置
                    let device_config = self.get_device_config(info.chain_id);

                    // 初始化设备
                    match device.initialize(device_config).await {
                        Ok(_) => {
                            info!("Device {} initialized successfully", device_id);
                            devices.insert(*device_id, Arc::new(Mutex::new(device)));
                            stats.insert(*device_id, DeviceStats::new());
                        }
                        Err(e) => {
                            error!("Failed to initialize device {}: {}", device_id, e);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create device {}: {}", device_id, e);
                    return Err(e);
                }
            }
        }

        info!("All devices initialized successfully");
        Ok(())
    }

    /// 启动设备管理器
    pub async fn start(&mut self) -> Result<(), DeviceError> {
        info!("Starting device manager");

        // 设置运行状态
        *self.running.write().await = true;

        // 启动所有设备
        self.start_all_devices().await?;

        // 启动监控任务
        self.start_monitoring().await?;

        info!("Device manager started successfully");
        Ok(())
    }

    /// 停止设备管理器
    pub async fn stop(&mut self) -> Result<(), DeviceError> {
        info!("Stopping device manager");

        // 设置停止状态
        *self.running.write().await = false;

        // 停止监控任务
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
        }

        // 停止所有设备
        self.stop_all_devices().await?;

        info!("Device manager stopped successfully");
        Ok(())
    }

    /// 启动所有设备
    async fn start_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;

        for (device_id, device) in devices.iter() {
            let mut device = device.lock().await;
            match device.start().await {
                Ok(_) => {
                    info!("Device {} started successfully", device_id);
                }
                Err(e) => {
                    error!("Failed to start device {}: {}", device_id, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// 停止所有设备
    async fn stop_all_devices(&self) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;

        for (device_id, device) in devices.iter() {
            let mut device = device.lock().await;
            match device.stop().await {
                Ok(_) => {
                    info!("Device {} stopped successfully", device_id);
                }
                Err(e) => {
                    warn!("Failed to stop device {}: {}", device_id, e);
                }
            }
        }

        Ok(())
    }

    /// 启动监控任务
    async fn start_monitoring(&mut self) -> Result<(), DeviceError> {
        let devices = self.devices.clone();
        let device_info = self.device_info.clone();
        let device_stats = self.device_stats.clone();
        let running = self.running.clone();
        let scan_interval = Duration::from_secs(self.config.scan_interval);

        let handle = tokio::spawn(async move {
            let mut interval = interval(scan_interval);

            while *running.read().await {
                interval.tick().await;

                // 更新设备状态和统计信息
                let devices = devices.read().await;
                for (device_id, device) in devices.iter() {
                    let device = device.lock().await;

                    // 获取设备状态
                    if let Ok(status) = device.get_status().await {
                        let mut info = device_info.write().await;
                        if let Some(device_info) = info.get_mut(device_id) {
                            device_info.update_status(status);
                        }
                    }

                    // 获取设备统计信息
                    if let Ok(stats) = device.get_stats().await {
                        let mut device_stats = device_stats.write().await;
                        device_stats.insert(*device_id, stats);
                    }

                    // 获取温度
                    if let Ok(temperature) = device.get_temperature().await {
                        let mut info = device_info.write().await;
                        if let Some(device_info) = info.get_mut(device_id) {
                            device_info.update_temperature(temperature);
                        }
                    }

                    // 获取算力
                    if let Ok(hashrate) = device.get_hashrate().await {
                        let mut info = device_info.write().await;
                        if let Some(device_info) = info.get_mut(device_id) {
                            device_info.update_hashrate(hashrate);
                        }
                    }
                }
            }
        });

        self.monitoring_handle = Some(handle);
        Ok(())
    }

    /// 获取设备配置
    fn get_device_config(&self, chain_id: u8) -> crate::device::DeviceConfig {
        // 查找对应链的配置
        for chain in &self.config.chains {
            if chain.id == chain_id {
                return crate::device::DeviceConfig {
                    chain_id: chain.id,
                    enabled: chain.enabled,
                    frequency: chain.frequency,
                    voltage: chain.voltage,
                    auto_tune: chain.auto_tune,
                    chip_count: chain.chip_count,
                    temperature_limit: 85.0, // 默认温度限制
                    fan_speed: None,
                };
            }
        }

        // 返回默认配置
        crate::device::DeviceConfig::default()
    }

    /// 获取设备信息
    pub async fn get_device_info(&self, device_id: u32) -> Option<DeviceInfo> {
        let device_info = self.device_info.read().await;
        device_info.get(&device_id).cloned()
    }

    /// 获取所有设备信息
    pub async fn get_all_device_info(&self) -> Vec<DeviceInfo> {
        let device_info = self.device_info.read().await;
        device_info.values().cloned().collect()
    }

    /// 获取设备统计信息
    pub async fn get_device_stats(&self, device_id: u32) -> Option<DeviceStats> {
        let device_stats = self.device_stats.read().await;
        device_stats.get(&device_id).cloned()
    }

    /// 重启设备
    pub async fn restart_device(&self, device_id: u32) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let mut device = device.lock().await;
            device.restart().await?;
            info!("Device {} restarted successfully", device_id);
            Ok(())
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 提交工作到设备
    pub async fn submit_work(&self, device_id: u32, work: Work) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let mut device = device.lock().await;
            device.submit_work(work).await?;
            debug!("Work submitted to device {}", device_id);
            Ok(())
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 从设备获取结果
    pub async fn get_result(&self, device_id: u32) -> Result<Option<MiningResult>, DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let mut device = device.lock().await;
            device.get_result().await
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 设置设备频率
    pub async fn set_device_frequency(&self, device_id: u32, frequency: u32) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let mut device = device.lock().await;
            device.set_frequency(frequency).await?;
            info!("Device {} frequency set to {} MHz", device_id, frequency);
            Ok(())
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 设置设备电压
    pub async fn set_device_voltage(&self, device_id: u32, voltage: u32) -> Result<(), DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let mut device = device.lock().await;
            device.set_voltage(voltage).await?;
            info!("Device {} voltage set to {} mV", device_id, voltage);
            Ok(())
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 检查设备健康状态
    pub async fn health_check(&self, device_id: u32) -> Result<bool, DeviceError> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(&device_id) {
            let device = device.lock().await;
            device.health_check().await
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 获取活跃设备数量
    pub async fn get_active_device_count(&self) -> u32 {
        let device_info = self.device_info.read().await;
        device_info.values()
            .filter(|info| info.is_healthy())
            .count() as u32
    }

    /// 获取总算力
    pub async fn get_total_hashrate(&self) -> f64 {
        let device_info = self.device_info.read().await;
        device_info.values()
            .filter(|info| info.is_healthy())
            .map(|info| info.hashrate)
            .sum()
    }
}
