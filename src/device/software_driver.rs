use crate::error::DeviceError;
use crate::device::{
    DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult,
    MiningDevice, DeviceDriver, DeviceConfig
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error, debug};

/// 软算法设备驱动
/// 用于创建和管理软算法虚拟设备
pub struct SoftwareDeviceDriver {
    /// 驱动版本
    version: &'static str,
    /// 设备数量配置
    device_count: u32,
    /// 算力配置
    min_hashrate: f64,
    max_hashrate: f64,
    /// 错误率配置
    error_rate: f64,
    /// 批次大小配置
    batch_size: u32,
}

impl SoftwareDeviceDriver {
    /// 创建新的软算法设备驱动
    pub fn new() -> Self {
        Self {
            version: "1.0.0",
            device_count: 8,
            min_hashrate: 500_000_000.0,  // 500 MH/s
            max_hashrate: 2_000_000_000.0, // 2 GH/s
            error_rate: 0.005,             // 0.5%
            batch_size: 2000,
        }
    }

    /// 使用配置创建软算法设备驱动
    pub fn with_config(
        device_count: u32,
        min_hashrate: f64,
        max_hashrate: f64,
        error_rate: f64,
        batch_size: u32,
    ) -> Self {
        Self {
            version: "1.0.0",
            device_count,
            min_hashrate,
            max_hashrate,
            error_rate,
            batch_size,
        }
    }

    /// 获取驱动版本
    pub fn version(&self) -> &'static str {
        self.version
    }
}

#[async_trait]
impl DeviceDriver for SoftwareDeviceDriver {
    /// 驱动名称
    fn driver_name(&self) -> &'static str {
        "Software Core"
    }

    /// 支持的设备类型
    fn supported_devices(&self) -> Vec<&'static str> {
        vec!["software", "cpu", "virtual"]
    }

    /// 扫描软算法设备
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, DeviceError> {
        info!("扫描软算法设备，设备数量: {}", self.device_count);

        let mut devices = Vec::new();

        // 创建指定数量的软算法虚拟设备
        for i in 0..self.device_count {
            let device_info = DeviceInfo::new(
                1000 + i,
                format!("Software Device {}", i),
                "software".to_string(),
                i as u8,
            );
            devices.push(device_info);
        }

        info!("发现 {} 个软算法设备", devices.len());
        Ok(devices)
    }

    /// 创建软算法设备实例
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, DeviceError> {
        info!("创建软算法设备实例: {}", device_info.name);

        // 为每个设备分配不同的算力
        let device_index = device_info.chain_id as f64;
        let device_hashrate = self.min_hashrate +
            (self.max_hashrate - self.min_hashrate) * (device_index / self.device_count.max(1) as f64);

        // 创建软算法设备
        let device = SoftwareDeviceWrapper::new(
            device_info,
            device_hashrate,
            self.error_rate,
            self.batch_size,
        ).await?;

        Ok(Box::new(device))
    }

    /// 验证设备配置
    fn validate_config(&self, config: &DeviceConfig) -> Result<(), DeviceError> {
        if config.frequency == 0 {
            return Err(DeviceError::configuration_error("频率不能为0"));
        }

        if config.voltage == 0 {
            return Err(DeviceError::configuration_error("电压不能为0"));
        }

        if config.chip_count == 0 {
            return Err(DeviceError::configuration_error("芯片数量不能为0"));
        }

        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> DeviceConfig {
        DeviceConfig {
            chain_id: 0,
            enabled: true,
            frequency: 1000,        // 1 GHz 虚拟频率
            voltage: 900,           // 900 mV 虚拟电压
            auto_tune: false,       // 软算法设备不支持自动调频
            chip_count: 1,          // 每个软算法设备相当于1个虚拟芯片
            temperature_limit: 75.0, // CPU温度限制
            fan_speed: Some(60),    // 默认风扇速度
        }
    }

    /// 获取驱动版本
    fn version(&self) -> &'static str {
        self.version
    }
}

/// 软算法设备包装器
/// 将cgminer-software-core的SoftwareDevice包装为设备管理器可用的设备
pub struct SoftwareDeviceWrapper {
    /// 内部软算法设备
    inner_device: cgminer_software_core::SoftwareDevice,
    /// 设备ID
    device_id: u32,
}

impl SoftwareDeviceWrapper {
    /// 创建新的软算法设备包装器
    pub async fn new(
        device_info: DeviceInfo,
        target_hashrate: f64,
        error_rate: f64,
        batch_size: u32,
    ) -> Result<Self, DeviceError> {
        let device_id = device_info.id;

        // 创建内部软算法设备
        let inner_device = cgminer_software_core::SoftwareDevice::new(
            device_info,
            DeviceConfig::default(),
            target_hashrate,
            error_rate,
            batch_size,
        ).await.map_err(|e| {
            DeviceError::initialization_failed(format!("创建软算法设备失败: {}", e))
        })?;

        Ok(Self {
            inner_device,
            device_id,
        })
    }
}

#[async_trait]
impl MiningDevice for SoftwareDeviceWrapper {
    /// 获取设备ID
    fn device_id(&self) -> u32 {
        self.device_id
    }

    /// 获取设备信息
    async fn get_info(&self) -> Result<DeviceInfo, DeviceError> {
        self.inner_device.get_info().await
    }

    /// 获取设备状态
    async fn get_status(&self) -> Result<DeviceStatus, DeviceError> {
        self.inner_device.get_status().await
    }

    /// 获取设备统计信息
    async fn get_stats(&self) -> Result<DeviceStats, DeviceError> {
        self.inner_device.get_stats().await
    }

    /// 初始化设备
    async fn initialize(&mut self, config: DeviceConfig) -> Result<(), DeviceError> {
        self.inner_device.initialize(config).await
    }

    /// 启动设备
    async fn start(&mut self) -> Result<(), DeviceError> {
        self.inner_device.start().await
    }

    /// 停止设备
    async fn stop(&mut self) -> Result<(), DeviceError> {
        self.inner_device.stop().await
    }

    /// 重置设备
    async fn reset(&mut self) -> Result<(), DeviceError> {
        self.inner_device.reset().await
    }

    /// 提交工作
    async fn submit_work(&mut self, work: Work) -> Result<(), DeviceError> {
        self.inner_device.submit_work(work).await
    }

    /// 获取挖矿结果
    async fn get_results(&mut self) -> Result<Vec<MiningResult>, DeviceError> {
        self.inner_device.get_results().await
    }

    /// 设置频率
    async fn set_frequency(&mut self, frequency: u32) -> Result<(), DeviceError> {
        self.inner_device.set_frequency(frequency).await
    }

    /// 设置电压
    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError> {
        self.inner_device.set_voltage(voltage).await
    }

    /// 设置风扇速度
    async fn set_fan_speed(&mut self, speed: u32) -> Result<(), DeviceError> {
        self.inner_device.set_fan_speed(speed).await
    }

    /// 获取温度
    async fn get_temperature(&self) -> Result<f32, DeviceError> {
        self.inner_device.get_temperature().await
    }

    /// 获取算力
    async fn get_hashrate(&self) -> Result<f64, DeviceError> {
        self.inner_device.get_hashrate().await
    }

    /// 获取功耗
    async fn get_power_consumption(&self) -> Result<f32, DeviceError> {
        self.inner_device.get_power_consumption().await
    }
}
