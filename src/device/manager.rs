use crate::config::{DeviceConfig, Config};
use crate::error::DeviceError;
use cgminer_core::CoreRegistry;
use crate::device::{
    DeviceInfo, DeviceStats, Work, MiningResult,
    MiningDevice, DeviceCoreMapper,
    architecture::{UnifiedDeviceArchitecture, DeviceArchitectureConfig},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Mutex};
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use async_trait::async_trait;
use crate::logging::formatter::format_hashrate;

/// 设备算力详情
#[derive(Debug, Clone)]
pub struct DeviceHashrateDetail {
    /// 设备ID
    pub device_id: u32,
    /// 当前算力
    pub current_hashrate: f64,
    /// 1分钟平均算力
    pub avg_1m: f64,
    /// 5分钟平均算力
    pub avg_5m: f64,
    /// 15分钟平均算力
    pub avg_15m: f64,
    /// 温度
    pub temperature: f32,
}

/// 聚合算力统计信息
#[derive(Debug, Clone)]
pub struct AggregatedHashrateStats {
    /// 总当前算力
    pub total_current_hashrate: f64,
    /// 总1分钟算力
    pub total_1m_hashrate: f64,
    /// 总5分钟算力
    pub total_5m_hashrate: f64,
    /// 总15分钟算力
    pub total_15m_hashrate: f64,
    /// 总平均算力
    pub total_avg_hashrate: f64,
    /// 活跃设备数
    pub active_devices: u32,
    /// 设备详情列表
    pub device_details: Vec<DeviceHashrateDetail>,
    /// 统计时间戳
    pub timestamp: std::time::SystemTime,
}

/// 设备管理器（集成设备工厂功能）
pub struct DeviceManager {
    /// 设备列表
    devices: Arc<RwLock<HashMap<u32, Arc<Mutex<Box<dyn MiningDevice>>>>>>,
    /// 设备信息缓存
    device_info: Arc<RwLock<HashMap<u32, DeviceInfo>>>,
    /// 设备统计信息
    device_stats: Arc<RwLock<HashMap<u32, DeviceStats>>>,
    /// 核心注册表（从工厂移入）
    core_registry: Arc<CoreRegistry>,
    /// 活跃核心ID列表（从工厂移入）
    active_core_ids: Vec<String>,
    /// 设备-核心映射器
    device_core_mapper: Arc<DeviceCoreMapper>,
    /// 统一设备架构管理器
    architecture_manager: Arc<UnifiedDeviceArchitecture>,
    /// 配置
    config: DeviceConfig,
    /// 完整配置（用于访问核心配置中的设备数量）
    full_config: Option<Config>,

    /// 监控任务句柄
    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl DeviceManager {
    /// 创建新的设备管理器（集成工厂功能）
    pub fn new(config: DeviceConfig, core_registry: Arc<CoreRegistry>) -> Self {
        let device_core_mapper = DeviceCoreMapper::new(core_registry.clone());

        // 创建默认的架构配置
        let arch_config = DeviceArchitectureConfig::default();
        let architecture_manager = UnifiedDeviceArchitecture::new(arch_config, core_registry.clone());

        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            device_info: Arc::new(RwLock::new(HashMap::new())),
            device_stats: Arc::new(RwLock::new(HashMap::new())),
            core_registry,
            active_core_ids: Vec::new(),
            device_core_mapper: Arc::new(device_core_mapper),
            architecture_manager: Arc::new(architecture_manager),
            config,
            full_config: None,
            monitoring_handle: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 设置活跃核心ID列表
    pub async fn set_active_cores(&mut self, core_ids: Vec<String>) {
        self.active_core_ids = core_ids;
        info!("🏭 设备管理器接收到活跃核心: {:?}", self.active_core_ids);
    }

    /// 设置完整配置（用于访问核心配置）
    pub fn set_full_config(&mut self, config: Config) {
        self.full_config = Some(config);
    }

    /// 初始化设备管理器
    pub async fn initialize(&mut self) -> Result<(), DeviceError> {
        info!("🔧 初始化设备管理器");

        // 检查活跃核心
        if self.active_core_ids.is_empty() {
            return Err(DeviceError::InitializationFailed {
                device_id: 0,
                reason: "没有可用的活跃核心".to_string(),
            });
        }

        info!("🎉 设备管理器初始化完成，活跃核心数量: {}", self.active_core_ids.len());

        // 创建设备
        self.create_devices().await?;

        info!("✅ 设备管理器初始化成功");
        Ok(())
    }

    /// 创建设备
    async fn create_devices(&mut self) -> Result<(), DeviceError> {
        info!("🔧 创建设备");

        // 直接从core_registry获取可用核心工厂
        let available_cores = self.core_registry.list_factories().await.map_err(|e| {
            DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("获取可用核心失败: {}", e),
            }
        })?;

        if available_cores.is_empty() {
            warn!("⚠️ 没有可用的挖矿核心");
            return Ok(());
        }

        info!("📋 可用挖矿核心: {:?}", available_cores.iter().map(|c| &c.name).collect::<Vec<_>>());

        // 为每个核心扫描并创建设备
        for core in available_cores {
            match self.create_devices_for_core(&core).await {
                Ok(device_count) => {
                    info!("✅ 核心 {} 创建了 {} 个设备", core.name, device_count);
                }
                Err(e) => {
                    error!("❌ 核心 {} 设备创建失败: {}", core.name, e);
                }
            }
        }

        let total_device_count = self.devices.read().await.len();
        info!("🎯 设备创建完成，共创建 {} 个设备", total_device_count);

        Ok(())
    }

    /// 为指定核心创建设备
    async fn create_devices_for_core(&mut self, core: &cgminer_core::CoreInfo) -> Result<u32, DeviceError> {
        info!("🔍 为核心 {} 扫描设备", core.name);

        // 查找对应的活跃核心实例ID
        let core_instance_id = self.find_active_core_for_factory(&core.name).await?;

        // 使用核心实例ID扫描设备
        let scanned_devices = self.scan_devices_from_core(&core_instance_id).await.map_err(|e| {
            DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("扫描核心实例 {} 的设备失败: {}", core_instance_id, e),
            }
        })?;

        if scanned_devices.is_empty() {
            warn!("⚠️ 核心 {} 没有扫描到设备", core.name);
            return Ok(0);
        }

        let requested_device_count = scanned_devices.len() as u32;
        info!("📋 核心 {} 扫描到 {} 个设备", core.name, requested_device_count);

        // 使用架构管理器验证设备配置
        let validated_device_count = self.architecture_manager
            .validate_device_configuration(core, requested_device_count)
            .await?;

        if validated_device_count != requested_device_count {
            info!("📋 架构管理器调整设备数量: {} -> {}", requested_device_count, validated_device_count);
        }

        // 只使用验证后的设备数量
        let devices_to_create = scanned_devices.into_iter()
            .take(validated_device_count as usize)
            .collect::<Vec<_>>();

        // 创建设备映射
        let mappings = self.device_core_mapper
            .create_device_mappings_for_core(core, devices_to_create.clone())
            .await?;

        info!("📋 为核心 {} 创建了 {} 个设备映射", core.name, mappings.len());

        let mut created_count = 0u32;
        for (mapping, device_info) in mappings.into_iter().zip(devices_to_create.into_iter()) {
            match self.create_device_from_mapping(mapping, device_info).await {
                Ok(()) => {
                    created_count += 1;
                }
                Err(e) => {
                    error!("❌ 创建设备失败: {}", e);
                }
            }
        }

        Ok(created_count)
    }

    /// 查找对应工厂名称的活跃核心实例ID
    async fn find_active_core_for_factory(&self, factory_name: &str) -> Result<String, DeviceError> {
        // 根据工厂名称映射到核心类型前缀
        let core_prefix = match factory_name {
            "Software Mining Core" => "cpu-btc",
            "Maijie L7 Core" => "maijie-l7",
            _ => {
                return Err(DeviceError::InitializationFailed {
                    device_id: 0,
                    reason: format!("未知的核心工厂: {}", factory_name),
                });
            }
        };

        // 在活跃核心列表中查找匹配的核心实例
        for core_id in &self.active_core_ids {
            if core_id.starts_with(core_prefix) {
                return Ok(core_id.clone());
            }
        }

        Err(DeviceError::InitializationFailed {
            device_id: 0,
            reason: format!("未找到工厂 {} 对应的活跃核心实例", factory_name),
        })
    }

    /// 从核心实例扫描设备（从factory移植）
    async fn scan_devices_from_core(&self, core_id: &str) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        info!("从核心 {} 扫描设备", core_id);

        match self.core_registry.scan_devices(core_id).await {
            Ok(devices) => {
                info!("核心 {} 扫描到 {} 个设备", core_id, devices.len());
                Ok(devices)
            }
            Err(e) => {
                warn!("核心 {} 扫描设备失败: {}", core_id, e);
                // 如果核心扫描失败，回退到生成设备信息的方式
                if core_id.starts_with("cpu-btc") {
                    self.generate_software_device_infos().await
                } else if core_id.starts_with("maijie-l7") {
                    self.generate_asic_device_infos().await
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }

    /// 生成软件设备信息（从factory移植）
    async fn generate_software_device_infos(&self) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        // 从完整配置中读取设备数量
        let device_count = if let Some(ref full_config) = self.full_config {
            if let Some(ref cpu_btc_config) = full_config.cores.cpu_btc {
                cpu_btc_config.device_count
            } else {
                4 // 默认值
            }
        } else {
            4 // 默认值
        };

        info!("🔧 生成 {} 个软件设备", device_count);
        let mut devices = Vec::new();

        for i in 0..device_count {
            let device_info = cgminer_core::DeviceInfo {
                id: i + 1,
                name: format!("BTC-Software-{}", i + 1),
                device_type: "software".to_string(),
                chain_id: i as u8,
                device_path: None,
                serial_number: None,
                firmware_version: None,
                hardware_version: None,
                chip_count: Some(1),
                temperature: Some(45.0),
                voltage: Some(12),
                frequency: Some(600),
                fan_speed: None,
                created_at: std::time::SystemTime::now(),
                updated_at: std::time::SystemTime::now(),
            };
            devices.push(device_info);
        }

        Ok(devices)
    }

    /// 生成ASIC设备信息（从factory移植）
    async fn generate_asic_device_infos(&self) -> Result<Vec<cgminer_core::DeviceInfo>, cgminer_core::CoreError> {
        let device_count = 1; // 默认创建1个ASIC设备
        let mut devices = Vec::new();

        for i in 0..device_count {
            let device_info = cgminer_core::DeviceInfo {
                id: i + 100, // ASIC设备从100开始编号
                name: format!("Maijie-L7-{}", i + 1),
                device_type: "asic".to_string(),
                chain_id: i as u8,
                device_path: None,
                serial_number: None,
                firmware_version: None,
                hardware_version: None,
                chip_count: Some(126),
                temperature: Some(65.0),
                voltage: Some(900),
                frequency: Some(650),
                fan_speed: Some(70),
                created_at: std::time::SystemTime::now(),
                updated_at: std::time::SystemTime::now(),
            };
            devices.push(device_info);
        }

        Ok(devices)
    }

    /// 创建设备实例（从factory移植的核心功能）
    async fn create_device_instance(&self, device_info: cgminer_core::DeviceInfo) -> Result<Box<dyn MiningDevice>, DeviceError> {
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

                let device_config = crate::device::DeviceConfig {
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

                let device_config = crate::device::DeviceConfig {
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

        // 创建设备代理
        let device_proxy = CoreDeviceProxy::new_with_info(
            device_info,
            core_id,
            self.core_registry.clone(),
            device_config,
        ).await?;

        Ok(Box::new(device_proxy))
    }

    /// 从设备映射创建设备实例
    async fn create_device_from_mapping(
        &mut self,
        mapping: crate::device::DeviceCoreMapping,
        device_info: cgminer_core::DeviceInfo
    ) -> Result<(), DeviceError> {
        let device_id = mapping.device_id;
        let device_name = device_info.name.clone();
        let device_type = device_info.device_type.clone();

        // 验证设备ID的有效性
        // TODO: 重新启用验证 - DataValidator::validate_device_id(device_id)?;
        if device_id == 0 {
            return Err(DeviceError::InvalidConfig {
                reason: "Device ID cannot be zero".to_string(),
            });
        }

        info!("🔧 创建设备: ID={}, 名称={}, 类型={}, 核心={}",
              device_id, device_name, device_type, mapping.core_name);

        // 直接创建设备实例
        let device = self.create_device_instance(device_info.clone()).await.map_err(|e| {
            DeviceError::InitializationFailed {
                device_id,
                reason: format!("创建设备实例失败: {}", e),
            }
        })?;

        // 添加到设备列表
        let mut devices = self.devices.write().await;
        devices.insert(device_id, Arc::new(Mutex::new(device)));

        // 转换设备信息格式
        let local_device_info = crate::device::DeviceInfo {
            id: device_id, // 使用映射分配的ID
            name: format!("{} ({})", device_info.name, mapping.core_name),
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
            uptime: Duration::from_secs(0),
            last_share_time: None,
            created_at: device_info.created_at,
            updated_at: device_info.updated_at,
        };

        // 缓存设备信息
        let mut info_cache = self.device_info.write().await;
        info_cache.insert(device_id, local_device_info);

        // 初始化设备统计
        let mut stats_cache = self.device_stats.write().await;
        stats_cache.insert(device_id, DeviceStats::new());

        info!("✅ 设备创建成功: ID={}, 名称={}, 核心={}", device_id, device_name, mapping.core_name);

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
            let mut aggregated_stats_counter = 0u32;

            while *running.read().await {
                interval.tick().await;
                aggregated_stats_counter += 1;

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
                        device_stats.insert(*device_id, stats.clone());

                        // 从统计信息中获取算力并更新到设备信息
                        if let Some(avg_hashrate) = stats.get_average_hashrate() {
                            let mut info = device_info.write().await;
                            if let Some(device_info) = info.get_mut(device_id) {
                                device_info.update_hashrate(avg_hashrate);
                            }
                        }
                    }

                    // 获取温度
                    if let Ok(temperature) = device.get_temperature().await {
                        let mut info = device_info.write().await;
                        if let Some(device_info) = info.get_mut(device_id) {
                            device_info.update_temperature(temperature);
                        }
                    }
                }

                // 每3个监控周期输出一次聚合算力统计（用于测试）
                if aggregated_stats_counter % 3 == 0 {
                    // 创建临时的聚合统计输出
                    Self::log_aggregated_stats_static(&device_stats, &device_info).await;
                }
            }
        });

        self.monitoring_handle = Some(handle);
        Ok(())
    }

    /// 静态方法用于在监控任务中输出聚合统计
    async fn log_aggregated_stats_static(
        device_stats: &Arc<RwLock<HashMap<u32, DeviceStats>>>,
        device_info: &Arc<RwLock<HashMap<u32, DeviceInfo>>>,
    ) {
        let device_stats = device_stats.read().await;
        let device_info = device_info.read().await;

        let mut total_current = 0.0;
        let mut active_devices = 0;
        let mut device_details = Vec::new();

        for (device_id, info) in device_info.iter() {
            if info.is_healthy() {
                active_devices += 1;

                // 优先使用设备统计信息中的算力
                let device_hashrate = if let Some(stats) = device_stats.get(device_id) {
                    if let Some(avg_hashrate) = stats.get_average_hashrate() {
                        avg_hashrate
                    } else {
                        // 如果没有算力历史，使用设备信息中的算力
                        info.hashrate
                    }
                } else {
                    // 如果没有统计信息，使用设备信息中的算力
                    info.hashrate
                };

                total_current += device_hashrate;
                device_details.push((*device_id, device_hashrate, info.temperature.unwrap_or(0.0)));
            }
        }

        if active_devices == 0 {
            // 即使没有活跃设备，也输出一条信息表明监控正在运行
            debug!("📊 算力统计汇总 | 活跃设备: 0 | 监控系统正在运行");
            return;
        }

        // 输出总体统计（使用自适应单位）
        info!("📊 算力统计汇总 | 活跃设备: {} | 总算力: {} | 平均: {}",
              active_devices,
              format_hashrate(total_current),
              format_hashrate(total_current / active_devices as f64));

        // 输出设备详情（分组显示，每行最多5个设备，使用自适应单位）
        for chunk in device_details.chunks(5) {
            let device_info_str: Vec<String> = chunk.iter().map(|(device_id, hashrate, temp)| {
                format!("设备{}: {} ({:.1}°C)", device_id, format_hashrate(*hashrate), temp)
            }).collect();

            debug!("   📱 {}", device_info_str.join(" | "));
        }
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
        // 优先从设备统计信息中获取算力，这样更准确
        let device_stats = self.device_stats.read().await;
        let device_info = self.device_info.read().await;

        let mut total_hashrate = 0.0;

        for (device_id, info) in device_info.iter() {
            if info.is_healthy() {
                // 优先使用设备统计信息中的平均算力
                if let Some(stats) = device_stats.get(device_id) {
                    if let Some(avg_hashrate) = stats.get_average_hashrate() {
                        total_hashrate += avg_hashrate;
                    } else {
                        // 如果没有算力历史，使用设备信息中的算力
                        total_hashrate += info.hashrate;
                    }
                } else {
                    // 如果没有统计信息，则使用设备信息中的算力
                    total_hashrate += info.hashrate;
                }
            }
        }

        total_hashrate
    }

    /// 获取聚合算力统计信息
    pub async fn get_aggregated_hashrate_stats(&self) -> AggregatedHashrateStats {
        let device_stats = self.device_stats.read().await;
        let device_info = self.device_info.read().await;

        let mut total_current = 0.0;
        let total_1m = 0.0;
        let total_5m = 0.0;
        let total_15m = 0.0;
        let mut total_avg = 0.0;
        let mut active_devices = 0;
        let mut device_details = Vec::new();

        for (device_id, info) in device_info.iter() {
            if info.is_healthy() {
                if let Some(stats) = device_stats.get(device_id) {
                    if let Some(avg_hashrate) = stats.get_average_hashrate() {
                        total_current += avg_hashrate;
                        total_avg += avg_hashrate;
                        active_devices += 1;

                        device_details.push(DeviceHashrateDetail {
                            device_id: *device_id,
                            current_hashrate: avg_hashrate,
                            avg_1m: avg_hashrate, // 简化处理，实际应该从stats获取
                            avg_5m: avg_hashrate,
                            avg_15m: avg_hashrate,
                            temperature: info.temperature.unwrap_or(0.0),
                        });
                    }
                }
            }
        }

        AggregatedHashrateStats {
            total_current_hashrate: total_current,
            total_1m_hashrate: total_1m,
            total_5m_hashrate: total_5m,
            total_15m_hashrate: total_15m,
            total_avg_hashrate: total_avg,
            active_devices,
            device_details,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// 输出聚合算力统计日志
    pub async fn log_aggregated_hashrate_stats(&self) {
        let stats = self.get_aggregated_hashrate_stats().await;

        if stats.active_devices == 0 {
            return;
        }

        // 输出总体统计（使用自适应单位）
        info!("📊 算力统计汇总 | 活跃设备: {} | 总算力: {} | 平均: {}",
              stats.active_devices,
              format_hashrate(stats.total_current_hashrate),
              format_hashrate(stats.total_avg_hashrate / stats.active_devices as f64));

        // 输出设备详情（分组显示，每行最多5个设备，使用自适应单位）
        for chunk in stats.device_details.chunks(5) {
            let device_info: Vec<String> = chunk.iter().map(|d| {
                format!("设备{}: {}", d.device_id, format_hashrate(d.current_hashrate))
            }).collect();

            debug!("   📱 {}", device_info.join(" | "));
        }
    }

    /// 获取设备的核心映射信息
    pub async fn get_device_core_mapping(&self, device_id: u32) -> Option<crate::device::DeviceCoreMapping> {
        self.device_core_mapper.get_device_mapping(device_id).await
    }

    /// 获取核心的所有设备ID
    pub async fn get_core_devices(&self, core_name: &str) -> Vec<u32> {
        self.device_core_mapper.get_core_devices(core_name).await
    }

    /// 获取映射统计信息
    pub async fn get_mapping_stats(&self) -> crate::device::MappingStats {
        self.device_core_mapper.get_mapping_stats().await
    }

    /// 验证设备映射一致性
    pub async fn validate_device_mappings(&self) -> Result<(), DeviceError> {
        self.device_core_mapper.validate_mappings().await
    }

    /// 按核心类型获取设备
    pub async fn get_devices_by_core_type(&self, core_type: &str) -> Vec<u32> {
        let mappings = self.device_core_mapper.get_all_mappings().await;
        mappings.into_iter()
            .filter(|(_, mapping)| mapping.core_type == core_type && mapping.active)
            .map(|(device_id, _)| device_id)
            .collect()
    }

    /// 获取设备架构统计信息
    pub async fn get_architecture_stats(&self) -> crate::device::architecture::ArchitectureStats {
        self.architecture_manager.get_architecture_stats().await
    }

    /// 更新系统资源使用情况
    pub async fn update_resource_usage(&self, memory_mb: u64, cpu_percent: f64) {
        self.architecture_manager.update_resource_usage(memory_mb, cpu_percent).await;
    }

    /// 获取设备数量统计
    pub async fn get_device_count_by_core(&self) -> HashMap<String, usize> {
        let mappings = self.device_core_mapper.get_all_mappings().await;
        let mut counts = HashMap::new();

        for (_, mapping) in mappings {
            if mapping.active {
                *counts.entry(mapping.core_name).or_insert(0) += 1;
            }
        }

        counts
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
    /// 核心注册表引用
    core_registry: Arc<CoreRegistry>,
}

impl CoreDeviceProxy {
    /// 从设备信息创建新的设备代理
    pub async fn new_with_info(
        device_info: cgminer_core::DeviceInfo,
        core_id: String,
        core_registry: Arc<CoreRegistry>,
        _config: crate::device::DeviceConfig,
    ) -> Result<Self, crate::error::DeviceError> {
        let proxy = Self {
            device_id: device_info.id,
            core_id,
            device_cache: Arc::new(tokio::sync::RwLock::new(None)),
            core_registry,
        };

        // 缓存设备信息
        {
            let mut cache = proxy.device_cache.write().await;
            *cache = Some(DeviceInfo {
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
                updated_at: std::time::SystemTime::now(),
            });
        }

        Ok(proxy)
    }
}

#[async_trait]
impl MiningDevice for CoreDeviceProxy {
    fn device_id(&self) -> u32 {
        self.device_id
    }

    async fn get_info(&self) -> Result<DeviceInfo, crate::error::DeviceError> {
        // 检查缓存
        {
            let cache = self.device_cache.read().await;
            if let Some(cached_info) = cache.as_ref() {
                return Ok(cached_info.clone());
            }
        }

        // 如果缓存为空，返回默认设备信息
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

    async fn start(&mut self) -> Result<(), crate::error::DeviceError> {
        debug!("启动设备代理: ID={}, 核心={}", self.device_id, self.core_id);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), crate::error::DeviceError> {
        debug!("停止设备代理: ID={}, 核心={}", self.device_id, self.core_id);
        Ok(())
    }

    async fn submit_work(&mut self, _work: Work) -> Result<(), crate::error::DeviceError> {
        // 通过核心提交工作
        Ok(())
    }

    async fn get_result(&mut self) -> Result<Option<MiningResult>, crate::error::DeviceError> {
        // 从核心获取结果
        Ok(None)
    }

    async fn set_frequency(&mut self, _frequency: u32) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }

    async fn set_voltage(&mut self, _voltage: u32) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }

    async fn initialize(&mut self, _config: crate::device::DeviceConfig) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<crate::device::DeviceStatus, crate::error::DeviceError> {
        Ok(crate::device::DeviceStatus::Idle)
    }

    async fn get_temperature(&self) -> Result<f32, crate::error::DeviceError> {
        Ok(45.0)
    }

    async fn get_hashrate(&self) -> Result<f64, crate::error::DeviceError> {
        // 尝试从核心获取算力统计
        match self.core_registry.get_core_stats(&self.core_id).await {
            Ok(core_stats) => {
                // 如果核心有多个设备，计算平均算力
                if core_stats.active_devices > 0 {
                    Ok(core_stats.total_hashrate / core_stats.active_devices as f64)
                } else {
                    Ok(0.0)
                }
            }
            Err(_) => {
                // 如果无法获取核心统计信息，返回0
                Ok(0.0)
            }
        }
    }

    async fn set_fan_speed(&mut self, _speed: u32) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }

    async fn get_stats(&self) -> Result<crate::device::DeviceStats, crate::error::DeviceError> {
        // 尝试从核心获取真实的统计数据
        match self.core_registry.get_core_stats(&self.core_id).await {
            Ok(core_stats) => {
                let mut device_stats = crate::device::DeviceStats::new();

                // 如果核心有多个设备，计算平均算力
                let device_hashrate = if core_stats.active_devices > 0 {
                    core_stats.total_hashrate / core_stats.active_devices as f64
                } else {
                    0.0
                };

                // 记录算力历史
                device_stats.record_hashrate(device_hashrate);

                // 设置其他统计信息（使用正确的字段名和类型转换）
                let active_devices = core_stats.active_devices.max(1) as u64;
                device_stats.valid_nonces = core_stats.accepted_work / active_devices;
                device_stats.invalid_nonces = core_stats.rejected_work / active_devices;
                device_stats.hardware_errors = core_stats.hardware_errors / active_devices;

                Ok(device_stats)
            }
            Err(_) => {
                // 如果无法获取核心统计信息，返回默认统计
                Ok(crate::device::DeviceStats::new())
            }
        }
    }

    async fn health_check(&self) -> Result<bool, crate::error::DeviceError> {
        Ok(true)
    }

    async fn reset_stats(&mut self) -> Result<(), crate::error::DeviceError> {
        Ok(())
    }
}
