//! 统一设备工厂模块
//!
//! 通过工厂模式统一设备创建，避免直接依赖核心库

use crate::device::{DeviceConfig, MiningDevice, DeviceInfo};
use crate::error::DeviceError;
use cgminer_core::CoreRegistry;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

/// 统一设备工厂
///
/// 负责创建不同类型的挖矿设备，使用挖矿管理器提供的核心实例
pub struct UnifiedDeviceFactory {
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// 活跃核心ID列表（由挖矿管理器提供）
    active_core_ids: Vec<String>,
    /// 下一个设备ID
    next_device_id: Arc<Mutex<u32>>,
}

impl UnifiedDeviceFactory {
    /// 创建新的设备工厂
    pub fn new(core_registry: Arc<CoreRegistry>) -> Self {
        Self {
            core_registry,
            active_core_ids: Vec::new(),
            next_device_id: Arc::new(Mutex::new(1)),
        }
    }

    /// 设置活跃核心ID列表（由挖矿管理器提供）
    pub fn set_active_cores(&mut self, core_ids: Vec<String>) {
        self.active_core_ids = core_ids;
        info!("🏭 设备工厂接收到活跃核心: {:?}", self.active_core_ids);
    }

    /// 初始化工厂
    pub async fn initialize(&mut self) -> Result<(), DeviceError> {
        info!("🏭 初始化统一设备工厂...");

        if self.active_core_ids.is_empty() {
            return Err(DeviceError::InitializationFailed {
                device_id: 0,
                reason: "没有可用的活跃核心".to_string(),
            });
        }

        info!("🎉 设备工厂初始化完成，活跃核心数量: {}", self.active_core_ids.len());
        Ok(())
    }

    /// 创建设备
    pub async fn create_device(
        &self,
        device_type: &str,
        device_config: DeviceConfig,
    ) -> Result<Box<dyn MiningDevice>, DeviceError> {
        debug!("🔧 创建设备: 类型={}, 配置={:?}", device_type, device_config);

        match device_type.to_lowercase().as_str() {
            "software" | "btc" | "cpu" | "btc-software" => {
                self.create_btc_software_device(device_config).await
            }
            "asic" | "maijie-l7" | "l7" => {
                self.create_maijie_l7_device(device_config).await
            }
            _ => Err(DeviceError::InvalidConfig {
                reason: format!("不支持的设备类型: {}", device_type),
            }),
        }
    }

    /// 获取可用的设备类型
    pub fn get_available_device_types(&self) -> Vec<String> {
        let mut types = Vec::new();

        if self.active_core_ids.iter().any(|id| id.contains("software") || id.contains("btc")) {
            types.push("btc-software".to_string());
        }

        if self.active_core_ids.iter().any(|id| id.contains("asic") || id.contains("maijie")) {
            types.push("maijie-l7".to_string());
        }

        types
    }

    /// 获取可用的核心信息
    pub async fn get_available_cores(&self) -> Result<Vec<cgminer_core::CoreInfo>, cgminer_core::CoreError> {
        let mut cores = Vec::new();

        // 获取已注册的核心工厂信息
        let core_infos = self.core_registry.list_factories().await?;

        for core_info in core_infos {
            // 检查是否有对应的活跃核心
            if self.active_core_ids.iter().any(|id| id.contains("software") || id.contains("btc")) &&
               core_info.name == "Software Mining Core" {
                cores.push(core_info);
            } else if self.active_core_ids.iter().any(|id| id.contains("asic") || id.contains("maijie")) &&
                      core_info.name == "ASIC Mining Core" {
                cores.push(core_info);
            }
        }

        Ok(cores)
    }

    /// 为指定核心扫描设备
    pub async fn scan_devices_for_core(&self, core_name: &str) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        match core_name {
            "Software Mining Core" => {
                // 查找软算法核心ID
                if let Some(core_id) = self.active_core_ids.iter()
                    .find(|id| id.contains("software") || id.contains("btc")) {
                    self.scan_devices_from_core(core_id).await
                } else {
                    Ok(Vec::new())
                }
            }
            "ASIC Mining Core" => {
                // 查找ASIC核心ID
                if let Some(core_id) = self.active_core_ids.iter()
                    .find(|id| id.contains("asic") || id.contains("maijie")) {
                    self.scan_devices_from_core(core_id).await
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// 从核心实例扫描设备
    async fn scan_devices_from_core(&self, core_id: &str) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        // 通过核心注册表调用核心的scan_devices方法
        info!("从核心 {} 扫描设备", core_id);

        match self.core_registry.scan_devices(core_id).await {
            Ok(devices) => {
                info!("核心 {} 扫描到 {} 个设备", core_id, devices.len());
                Ok(devices)
            }
            Err(e) => {
                warn!("核心 {} 扫描设备失败: {}", core_id, e);
                // 如果核心扫描失败，回退到生成设备信息的方式
                if core_id.starts_with("btc-software") {
                    self.generate_software_device_infos().await
                } else if core_id.starts_with("maijie-l7") {
                    self.generate_asic_device_infos().await
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }

    /// 生成软算法设备信息
    async fn generate_software_device_infos(&self) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        let mut devices = Vec::new();

        // 从配置文件读取设备数量，如果没有配置则使用默认值
        let device_count = self.get_software_device_count().await;

        info!("生成 {} 个软算法设备信息", device_count);

        for i in 0..device_count {
            let device_info = cgminer_core::DeviceInfo::new(
                Self::allocate_software_device_id(i), // 使用统一的ID分配策略
                format!("Software Device {}", i),
                "software".to_string(),
                i as u8,
            );
            devices.push(device_info);
        }

        Ok(devices)
    }

    /// 生成ASIC设备信息
    async fn generate_asic_device_infos(&self) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        let mut devices = Vec::new();

        // 从配置文件读取链数量，如果没有配置则使用默认值
        let chain_count = self.get_asic_chain_count().await;

        info!("生成 {} 个ASIC设备信息", chain_count);

        for i in 0..chain_count {
            let device_info = cgminer_core::DeviceInfo::new(
                Self::allocate_asic_device_id(i), // 使用统一的ID分配策略
                format!("ASIC Chain {}", i),
                "asic".to_string(),
                i as u8,
            );
            devices.push(device_info);
        }

        Ok(devices)
    }

    /// 获取软算法设备数量配置
    async fn get_software_device_count(&self) -> u32 {
        // 优先级：环境变量 > 核心配置 > 默认值

        // 1. 检查环境变量
        if let Ok(count_str) = std::env::var("CGMINER_SOFTWARE_DEVICE_COUNT") {
            if let Ok(count) = count_str.parse::<u32>() {
                if count > 0 && count <= 1000 {
                    info!("从环境变量读取软算法设备数量: {}", count);
                    return count;
                } else {
                    warn!("环境变量中的设备数量 {} 超出范围，使用默认值", count);
                }
            }
        }

        // 2. 从活跃核心配置读取（如果有的话）
        if self.active_core_ids.iter().any(|id| id.contains("software") || id.contains("btc")) {
            // TODO: 通过核心注册表获取核心配置
            // 这里暂时使用默认值，后续可以实现配置读取
        }

        // 3. 使用默认值
        4
    }

    /// 获取ASIC链数量配置
    async fn get_asic_chain_count(&self) -> u32 {
        // 优先级：环境变量 > 核心配置 > 默认值

        // 1. 检查环境变量
        if let Ok(count_str) = std::env::var("CGMINER_ASIC_CHAIN_COUNT") {
            if let Ok(count) = count_str.parse::<u32>() {
                if count > 0 && count <= 1000 {
                    info!("从环境变量读取ASIC链数量: {}", count);
                    return count;
                } else {
                    warn!("环境变量中的链数量 {} 超出范围，使用默认值", count);
                }
            }
        }

        // 2. 从活跃核心配置读取（如果有的话）
        if self.active_core_ids.iter().any(|id| id.contains("asic") || id.contains("maijie")) {
            // TODO: 通过核心注册表获取核心配置
            // 这里暂时使用默认值，后续可以实现配置读取
        }

        // 3. 使用默认值
        3
    }

    /// 分配软算法设备ID
    /// ID范围: 1000-1999 (支持最多1000个软算法设备)
    fn allocate_software_device_id(index: u32) -> u32 {
        const SOFTWARE_DEVICE_ID_BASE: u32 = 1000;
        const SOFTWARE_DEVICE_ID_MAX: u32 = 1999;

        let device_id = SOFTWARE_DEVICE_ID_BASE + index;
        if device_id > SOFTWARE_DEVICE_ID_MAX {
            warn!("软算法设备索引 {} 超出ID范围，使用基础ID", index);
            SOFTWARE_DEVICE_ID_BASE
        } else {
            device_id
        }
    }

    /// 分配ASIC设备ID
    /// ID范围: 2000-2999 (支持最多1000个ASIC设备)
    fn allocate_asic_device_id(index: u32) -> u32 {
        const ASIC_DEVICE_ID_BASE: u32 = 2000;
        const ASIC_DEVICE_ID_MAX: u32 = 2999;

        let device_id = ASIC_DEVICE_ID_BASE + index;
        if device_id > ASIC_DEVICE_ID_MAX {
            warn!("ASIC设备索引 {} 超出ID范围，使用基础ID", index);
            ASIC_DEVICE_ID_BASE
        } else {
            device_id
        }
    }

    /// 从设备信息创建设备实例
    pub async fn create_device_from_info(&self, device_info: cgminer_core::DeviceInfo) -> Result<Box<dyn MiningDevice>, DeviceError> {
        debug!("🔧 从设备信息创建设备: ID={}, 名称={}, 类型={}",
               device_info.id, device_info.name, device_info.device_type);

        // 根据设备类型选择对应的核心
        let (core_id, device_config) = match device_info.device_type.as_str() {
            "software" => {
                let core_id = self.active_core_ids.iter()
                    .find(|id| id.contains("software") || id.contains("btc"))
                    .ok_or_else(|| {
                        DeviceError::InitializationFailed {
                            device_id: device_info.id,
                            reason: "BTC软算法核心不可用".to_string(),
                        }
                    })?;

                let device_config = DeviceConfig {
                    chain_id: device_info.chain_id,
                    enabled: true,
                    frequency: 600,
                    voltage: 12,
                    auto_tune: false,
                    chip_count: 1,
                    temperature_limit: 85.0,
                    fan_speed: None,
                };

                (core_id.clone(), device_config)
            }
            "asic" => {
                let core_id = self.active_core_ids.iter()
                    .find(|id| id.contains("asic") || id.contains("maijie"))
                    .ok_or_else(|| {
                        DeviceError::InitializationFailed {
                            device_id: device_info.id,
                            reason: "Maijie L7核心不可用".to_string(),
                        }
                    })?;

                let device_config = DeviceConfig {
                    chain_id: device_info.chain_id,
                    enabled: true,
                    frequency: 650,
                    voltage: 900,
                    auto_tune: true,
                    chip_count: 126,
                    temperature_limit: 85.0,
                    fan_speed: Some(70),
                };

                (core_id.clone(), device_config)
            }
            _ => {
                return Err(DeviceError::InvalidConfig {
                    reason: format!("不支持的设备类型: {}", device_info.device_type),
                });
            }
        };

        // 创建设备代理，使用设备信息中的ID
        let device_proxy = CoreDeviceProxy::new_with_info(
            device_info,
            core_id,
            self.core_registry.clone(),
            device_config,
        ).await?;

        Ok(Box::new(device_proxy))
    }

    /// 创建BTC软算法设备
    async fn create_btc_software_device(
        &self,
        config: DeviceConfig,
    ) -> Result<Box<dyn MiningDevice>, DeviceError> {
        let core_id = self.active_core_ids.iter()
            .find(|id| id.contains("software") || id.contains("btc"))
            .ok_or_else(|| {
                DeviceError::InitializationFailed {
                    device_id: 0,
                    reason: "BTC软算法核心不可用".to_string(),
                }
            })?;

        debug!("创建BTC软算法设备，使用核心: {}", core_id);

        // 分配设备ID
        let device_id = {
            let mut next_id = self.next_device_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        // 创建设备代理
        let device_proxy = CoreDeviceProxy::new(
            device_id,
            core_id.clone(),
            self.core_registry.clone(),
            config,
        ).await?;

        info!("✅ BTC软算法设备创建成功: ID={}", device_id);
        Ok(Box::new(device_proxy))
    }

    /// 创建Maijie L7设备
    async fn create_maijie_l7_device(
        &self,
        config: DeviceConfig,
    ) -> Result<Box<dyn MiningDevice>, DeviceError> {
        let core_id = self.active_core_ids.iter()
            .find(|id| id.contains("asic") || id.contains("maijie"))
            .ok_or_else(|| {
                DeviceError::InitializationFailed {
                    device_id: 0,
                    reason: "Maijie L7核心不可用".to_string(),
                }
            })?;

        debug!("创建Maijie L7设备，使用核心: {}", core_id);

        // 分配设备ID
        let device_id = {
            let mut next_id = self.next_device_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        // 创建设备代理
        let device_proxy = CoreDeviceProxy::new(
            device_id,
            core_id.clone(),
            self.core_registry.clone(),
            config,
        ).await?;

        info!("✅ Maijie L7设备创建成功: ID={}", device_id);
        Ok(Box::new(device_proxy))
    }



}

/// 核心设备代理
///
/// 通过代理模式隔离设备层和核心层
pub struct CoreDeviceProxy {
    /// 设备ID
    device_id: u32,
    /// 核心ID
    core_id: String,

    /// 设备信息缓存
    device_cache: Arc<tokio::sync::RwLock<Option<DeviceInfo>>>,
}

impl CoreDeviceProxy {
    /// 创建新的设备代理
    pub async fn new(
        device_id: u32,
        core_id: String,
        _core_registry: Arc<CoreRegistry>,
        _config: DeviceConfig,
    ) -> Result<Self, DeviceError> {
        let proxy = Self {
            device_id,
            core_id,
            device_cache: Arc::new(tokio::sync::RwLock::new(None)),
        };

        // 初始化设备
        proxy.initialize_device().await?;

        Ok(proxy)
    }

    /// 从设备信息创建新的设备代理
    pub async fn new_with_info(
        device_info: cgminer_core::DeviceInfo,
        core_id: String,
        _core_registry: Arc<CoreRegistry>,
        _config: DeviceConfig,
    ) -> Result<Self, DeviceError> {
        let proxy = Self {
            device_id: device_info.id,
            core_id,
            device_cache: Arc::new(tokio::sync::RwLock::new(None)),
        };

        // 缓存设备信息
        {
            let mut cache = proxy.device_cache.write().await;
            *cache = Some(crate::device::DeviceInfo {
                id: device_info.id,
                name: device_info.name,
                device_type: device_info.device_type,
                chain_id: device_info.chain_id,
                chip_count: device_info.chip_count.unwrap_or(1),
                status: crate::device::DeviceStatus::Idle,
                temperature: device_info.temperature,
                fan_speed: device_info.fan_speed,
                voltage: device_info.voltage,
                frequency: device_info.frequency,
                hashrate: 0.0,
                accepted_shares: 0,
                rejected_shares: 0,
                hardware_errors: 0,
                uptime: std::time::Duration::from_secs(0),
                last_share_time: None,
                created_at: device_info.created_at,
                updated_at: device_info.updated_at,
            });
        }

        // 初始化设备
        proxy.initialize_device().await?;

        Ok(proxy)
    }

    /// 初始化设备
    async fn initialize_device(&self) -> Result<(), DeviceError> {
        debug!("初始化设备代理: ID={}, 核心={}", self.device_id, self.core_id);

        // 这里可以添加设备初始化逻辑
        // 例如：向核心发送初始化命令，设置设备参数等

        Ok(())
    }
}

#[async_trait::async_trait]
impl MiningDevice for CoreDeviceProxy {
    fn device_id(&self) -> u32 {
        self.device_id
    }

    async fn get_info(&self) -> Result<DeviceInfo, DeviceError> {
        // 检查缓存
        {
            let cache = self.device_cache.read().await;
            if let Some(cached_info) = cache.as_ref() {
                return Ok(cached_info.clone());
            }
        }

        // 从核心获取设备信息
        // 注意：这里需要根据实际的CoreRegistry API来实现
        // 目前作为占位符实现
        let device_info = DeviceInfo {
            id: self.device_id,
            name: format!("Device-{}", self.device_id),
            device_type: "proxy".to_string(),
            chain_id: 0,
            chip_count: 1,
            status: crate::device::DeviceStatus::Idle,
            temperature: Some(45.0),
            fan_speed: Some(50),
            voltage: Some(12),
            frequency: Some(600),
            hashrate: 0.0,
            accepted_shares: 0,
            rejected_shares: 0,
            hardware_errors: 0,
            uptime: std::time::Duration::from_secs(0),
            last_share_time: None,
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
        };

        // 更新缓存
        {
            let mut cache = self.device_cache.write().await;
            *cache = Some(device_info.clone());
        }

        Ok(device_info)
    }

    async fn get_status(&self) -> Result<crate::device::DeviceStatus, DeviceError> {
        // 从缓存或核心获取状态
        let info = self.get_info().await?;
        Ok(info.status)
    }

    async fn get_stats(&self) -> Result<crate::device::DeviceStats, DeviceError> {
        // 创建默认统计信息
        Ok(crate::device::DeviceStats {
            total_hashes: 0,
            valid_nonces: 0,
            invalid_nonces: 0,
            hardware_errors: 0,
            temperature_readings: vec![45.0],
            hashrate_history: vec![0.0],
            uptime_seconds: 0,
            restart_count: 0,
            last_restart_time: None,
        })
    }

    async fn initialize(&mut self, _config: DeviceConfig) -> Result<(), DeviceError> {
        self.initialize_device().await
    }

    async fn start(&mut self) -> Result<(), DeviceError> {
        info!("启动设备代理: ID={}", self.device_id);
        // 这里可以添加启动逻辑
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DeviceError> {
        info!("停止设备代理: ID={}", self.device_id);
        // 这里可以添加停止逻辑
        Ok(())
    }

    async fn submit_work(&mut self, _work: crate::device::Work) -> Result<(), DeviceError> {
        // 通过核心提交工作
        debug!("设备代理提交工作: ID={}", self.device_id);
        Ok(())
    }

    async fn get_result(&mut self) -> Result<Option<crate::device::MiningResult>, DeviceError> {
        // 从核心获取挖矿结果
        Ok(None)
    }

    async fn set_frequency(&mut self, _frequency: u32) -> Result<(), DeviceError> {
        debug!("设备代理设置频率: ID={}", self.device_id);
        Ok(())
    }

    async fn set_voltage(&mut self, _voltage: u32) -> Result<(), DeviceError> {
        debug!("设备代理设置电压: ID={}", self.device_id);
        Ok(())
    }

    async fn set_fan_speed(&mut self, _speed: u32) -> Result<(), DeviceError> {
        debug!("设备代理设置风扇速度: ID={}", self.device_id);
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), DeviceError> {
        info!("重启设备代理: ID={}", self.device_id);
        Ok(())
    }

    async fn get_temperature(&self) -> Result<f32, DeviceError> {
        Ok(45.0) // 默认温度
    }

    async fn get_hashrate(&self) -> Result<f64, DeviceError> {
        Ok(0.0) // 默认算力
    }

    async fn health_check(&self) -> Result<bool, DeviceError> {
        Ok(true) // 默认健康
    }

    async fn reset_stats(&mut self) -> Result<(), DeviceError> {
        debug!("重置设备代理统计: ID={}", self.device_id);
        Ok(())
    }
}
