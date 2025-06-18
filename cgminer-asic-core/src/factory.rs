//! ASIC核心工厂实现

use crate::core::AsicMiningCore;
use cgminer_core::{
    CoreFactory, CoreType, CoreInfo, CoreConfig, MiningCore, CoreError
};
use async_trait::async_trait;
use tracing::info;

/// ASIC核心工厂
pub struct AsicCoreFactory {
    /// 核心信息
    core_info: CoreInfo,
}

impl AsicCoreFactory {
    /// 创建新的ASIC核心工厂
    pub fn new() -> Self {
        let core_info = CoreInfo::new(
            "ASIC Mining Core".to_string(),
            CoreType::Asic,
            crate::VERSION.to_string(),
            "ASIC挖矿核心，支持各种ASIC硬件设备的挖矿操作，包括Maijie L7等矿机。".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["asic".to_string(), "maijie-l7".to_string()],
        );

        Self { core_info }
    }
}

impl Default for AsicCoreFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CoreFactory for AsicCoreFactory {
    /// 获取核心类型
    fn core_type(&self) -> CoreType {
        CoreType::Asic
    }

    /// 获取核心信息
    fn core_info(&self) -> CoreInfo {
        self.core_info.clone()
    }

    /// 创建核心实例
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>, CoreError> {
        info!("创建ASIC挖矿核心实例: {}", config.name);
        
        let mut core = AsicMiningCore::new(config.name.clone());
        core.initialize(config).await?;
        
        Ok(Box::new(core))
    }

    /// 验证配置
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError> {
        if config.name.is_empty() {
            return Err(CoreError::config("核心名称不能为空"));
        }

        // 验证设备配置
        for (i, device_config) in config.devices.iter().enumerate() {
            if device_config.frequency == 0 {
                return Err(CoreError::config(format!(
                    "设备 {} 的频率不能为0", i
                )));
            }

            if device_config.voltage == 0 {
                return Err(CoreError::config(format!(
                    "设备 {} 的电压不能为0", i
                )));
            }

            if device_config.temperature_limit <= 0.0 {
                return Err(CoreError::config(format!(
                    "设备 {} 的温度限制必须大于0", i
                )));
            }

            if device_config.chip_count == 0 {
                return Err(CoreError::config(format!(
                    "设备 {} 的芯片数量不能为0", i
                )));
            }

            // ASIC设备特定验证
            if device_config.frequency < 100 || device_config.frequency > 1000 {
                return Err(CoreError::config(format!(
                    "设备 {} 的频率必须在100-1000MHz之间", i
                )));
            }

            if device_config.voltage < 700 || device_config.voltage > 1200 {
                return Err(CoreError::config(format!(
                    "设备 {} 的电压必须在700-1200mV之间", i
                )));
            }

            if device_config.temperature_limit > 100.0 {
                return Err(CoreError::config(format!(
                    "设备 {} 的温度限制不能超过100°C", i
                )));
            }
        }

        // 验证自定义参数
        if let Some(chain_count) = config.custom_params.get("chain_count") {
            if let Some(count) = chain_count.as_u64() {
                if count == 0 {
                    return Err(CoreError::config("ASIC链数量不能为0"));
                }
                if count > 16 {
                    return Err(CoreError::config("ASIC链数量不能超过16"));
                }
            } else {
                return Err(CoreError::config("chain_count 必须是正整数"));
            }
        }

        if let Some(spi_speed) = config.custom_params.get("spi_speed") {
            if let Some(speed) = spi_speed.as_u64() {
                if speed == 0 {
                    return Err(CoreError::config("SPI速度不能为0"));
                }
                if speed > 50_000_000 {
                    return Err(CoreError::config("SPI速度不能超过50MHz"));
                }
            } else {
                return Err(CoreError::config("spi_speed 必须是正整数"));
            }
        }

        if let Some(uart_baud) = config.custom_params.get("uart_baud") {
            if let Some(baud) = uart_baud.as_u64() {
                let valid_bauds = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
                if !valid_bauds.contains(&baud) {
                    return Err(CoreError::config("UART波特率必须是标准值"));
                }
            } else {
                return Err(CoreError::config("uart_baud 必须是整数"));
            }
        }

        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig {
        use std::collections::HashMap;
        
        let mut custom_params = HashMap::new();
        custom_params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        custom_params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(6_000_000))); // 6MHz
        custom_params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(115200)));
        custom_params.insert("auto_detect".to_string(), serde_json::Value::Bool(true));
        custom_params.insert("power_limit".to_string(), serde_json::Value::Number(serde_json::Number::from(3000))); // 3000W
        custom_params.insert("cooling_mode".to_string(), serde_json::Value::String("auto".to_string()));

        CoreConfig {
            name: "asic-core".to_string(),
            enabled: true,
            devices: vec![
                cgminer_core::DeviceConfig {
                    chain_id: 0,
                    enabled: true,
                    frequency: 650,
                    voltage: 900,
                    auto_tune: true,
                    chip_count: 126, // Maijie L7 典型芯片数量
                    temperature_limit: 85.0,
                    fan_speed: Some(70),
                },
                cgminer_core::DeviceConfig {
                    chain_id: 1,
                    enabled: true,
                    frequency: 650,
                    voltage: 900,
                    auto_tune: true,
                    chip_count: 126,
                    temperature_limit: 85.0,
                    fan_speed: Some(70),
                },
                cgminer_core::DeviceConfig {
                    chain_id: 2,
                    enabled: true,
                    frequency: 650,
                    voltage: 900,
                    auto_tune: true,
                    chip_count: 126,
                    temperature_limit: 85.0,
                    fan_speed: Some(70),
                },
            ],
            custom_params,
        }
    }
}
