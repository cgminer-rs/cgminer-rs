//! 统一设备工厂模块
//!
//! 通过工厂模式统一设备创建，避免直接依赖核心库

use crate::device::{DeviceConfig, MiningDevice, DeviceInfo, conversion};
use crate::error::DeviceError;
use cgminer_core::{CoreRegistry, CoreConfig};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

/// 统一设备工厂
///
/// 负责创建不同类型的挖矿设备，通过核心注册表管理底层核心库
pub struct UnifiedDeviceFactory {
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// BTC软算法核心ID
    btc_core_id: Option<String>,
    /// Maijie L7 ASIC核心ID
    maijie_l7_core_id: Option<String>,
    /// 下一个设备ID
    next_device_id: Arc<Mutex<u32>>,
}

impl UnifiedDeviceFactory {
    /// 创建新的设备工厂
    pub fn new(core_registry: Arc<CoreRegistry>) -> Self {
        Self {
            core_registry,
            btc_core_id: None,
            maijie_l7_core_id: None,
            next_device_id: Arc::new(Mutex::new(1)),
        }
    }

    /// 初始化工厂，注册可用的核心
    pub async fn initialize(&mut self) -> Result<(), DeviceError> {
        info!("🏭 初始化统一设备工厂...");

        // 尝试注册BTC软算法核心
        match self.register_btc_software_core().await {
            Ok(core_id) => {
                self.btc_core_id = Some(core_id.clone());
                info!("✅ BTC软算法核心注册成功: {}", core_id);
            }
            Err(e) => {
                warn!("⚠️ BTC软算法核心注册失败: {}", e);
            }
        }

        // 尝试注册Maijie L7 ASIC核心
        match self.register_maijie_l7_core().await {
            Ok(core_id) => {
                self.maijie_l7_core_id = Some(core_id.clone());
                info!("✅ Maijie L7 ASIC核心注册成功: {}", core_id);
            }
            Err(e) => {
                warn!("⚠️ Maijie L7 ASIC核心注册失败: {}", e);
            }
        }

        if self.btc_core_id.is_none() && self.maijie_l7_core_id.is_none() {
            return Err(DeviceError::InitializationFailed {
                device_id: 0,
                reason: "没有可用的核心库".to_string(),
            });
        }

        info!("🎉 设备工厂初始化完成");
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

        if self.btc_core_id.is_some() {
            types.push("btc-software".to_string());
        }

        if self.maijie_l7_core_id.is_some() {
            types.push("maijie-l7".to_string());
        }

        types
    }

    /// 创建BTC软算法设备
    async fn create_btc_software_device(
        &self,
        config: DeviceConfig,
    ) -> Result<Box<dyn MiningDevice>, DeviceError> {
        let core_id = self.btc_core_id.as_ref().ok_or_else(|| {
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
        let core_id = self.maijie_l7_core_id.as_ref().ok_or_else(|| {
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

    /// 注册BTC软算法核心
    async fn register_btc_software_core(&self) -> Result<String, DeviceError> {
        // 创建软算法核心工厂
        let factory = cgminer_s_btc_core::create_factory();

        self.core_registry.register_factory("btc-software".to_string(), factory)
            .map_err(|e| DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("注册BTC软算法核心失败: {}", e),
            })?;

        // 创建核心实例
        let core_config = CoreConfig {
            name: "cgminer-s-btc-core".to_string(),
            enabled: true,
            devices: vec![], // 设备配置将在核心内部创建
            custom_params: {
                let mut params = std::collections::HashMap::new();
                params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
                params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1000000.0).unwrap()));
                params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(10000000.0).unwrap()));
                params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap()));
                params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));
                params.insert("work_timeout_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(5000)));
                params
            },
        };

        let core_id = self.core_registry.create_core("btc-software", core_config).await
            .map_err(|e| DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("创建BTC软算法核心失败: {}", e),
            })?;

        Ok(core_id)
    }

    /// 注册Maijie L7 ASIC核心
    #[cfg(feature = "maijie-l7")]
    async fn register_maijie_l7_core(&self) -> Result<String, DeviceError> {
        // 创建ASIC核心工厂
        let factory = cgminer_a_maijie_l7_core::create_factory();

        self.core_registry.register_factory("maijie-l7".to_string(), factory)
            .map_err(|e| DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("注册Maijie L7核心失败: {}", e),
            })?;

        // 创建核心实例
        let core_config = CoreConfig {
            name: "cgminer-a-maijie-l7-core".to_string(),
            enabled: true,
            devices: vec![], // 设备配置将在核心内部创建
            custom_params: {
                let mut params = std::collections::HashMap::new();
                params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
                params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(1000000)));
                params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(115200)));
                params.insert("auto_detect".to_string(), serde_json::Value::Bool(true));
                params.insert("power_limit".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(3000.0).unwrap()));
                params.insert("cooling_mode".to_string(), serde_json::Value::String("auto".to_string()));
                params
            },
        };

        let core_id = self.core_registry.create_core("maijie-l7", core_config).await
            .map_err(|e| DeviceError::InitializationFailed {
                device_id: 0,
                reason: format!("创建Maijie L7核心失败: {}", e),
            })?;

        Ok(core_id)
    }

    /// 注册Maijie L7 ASIC核心 (未启用特性时的占位符)
    #[cfg(not(feature = "maijie-l7"))]
    async fn register_maijie_l7_core(&self) -> Result<String, DeviceError> {
        Err(DeviceError::UnsupportedDevice {
            device_type: "maijie-l7".to_string(),
        })
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
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// 设备配置
    config: DeviceConfig,
    /// 设备信息缓存
    device_cache: Arc<tokio::sync::RwLock<Option<DeviceInfo>>>,
}

impl CoreDeviceProxy {
    /// 创建新的设备代理
    pub async fn new(
        device_id: u32,
        core_id: String,
        core_registry: Arc<CoreRegistry>,
        config: DeviceConfig,
    ) -> Result<Self, DeviceError> {
        let proxy = Self {
            device_id,
            core_id,
            core_registry,
            config,
            device_cache: Arc::new(tokio::sync::RwLock::new(None)),
        };

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
