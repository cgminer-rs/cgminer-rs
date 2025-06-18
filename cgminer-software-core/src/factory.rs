//! 软算法核心工厂实现

use crate::core::SoftwareMiningCore;
use cgminer_core::{
    CoreFactory, CoreType, CoreInfo, CoreConfig, MiningCore, CoreError
};
use async_trait::async_trait;
use tracing::info;

/// 软算法核心工厂
pub struct SoftwareCoreFactory {
    /// 核心信息
    core_info: CoreInfo,
}

impl SoftwareCoreFactory {
    /// 创建新的软算法核心工厂
    pub fn new() -> Self {
        let core_info = CoreInfo::new(
            "Software Mining Core".to_string(),
            CoreType::Custom("software".to_string()),
            crate::VERSION.to_string(),
            "软算法挖矿核心，使用真实的SHA256算法进行CPU挖矿计算。产生真实可用的挖矿数据，适用于测试、开发和低功耗挖矿场景。".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["software".to_string(), "cpu".to_string()],
        );

        Self { core_info }
    }
}

impl Default for SoftwareCoreFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CoreFactory for SoftwareCoreFactory {
    /// 获取核心类型
    fn core_type(&self) -> CoreType {
        CoreType::Custom("software".to_string())
    }

    /// 获取核心信息
    fn core_info(&self) -> CoreInfo {
        self.core_info.clone()
    }

    /// 创建核心实例
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>, CoreError> {
        info!("创建软算法挖矿核心实例: {}", config.name);

        let mut core = SoftwareMiningCore::new(config.name.clone());
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
        }

        // 验证自定义参数
        if let Some(device_count) = config.custom_params.get("device_count") {
            if let Some(count) = device_count.as_u64() {
                if count == 0 {
                    return Err(CoreError::config("软算法设备数量不能为0"));
                }
                if count > 100 {
                    return Err(CoreError::config("软算法设备数量不能超过100"));
                }
            } else {
                return Err(CoreError::config("device_count 必须是正整数"));
            }
        }

        if let Some(min_hashrate) = config.custom_params.get("min_hashrate") {
            if let Some(hashrate) = min_hashrate.as_f64() {
                if hashrate <= 0.0 {
                    return Err(CoreError::config("最小算力必须大于0"));
                }
            } else {
                return Err(CoreError::config("min_hashrate 必须是正数"));
            }
        }

        if let Some(max_hashrate) = config.custom_params.get("max_hashrate") {
            if let Some(hashrate) = max_hashrate.as_f64() {
                if hashrate <= 0.0 {
                    return Err(CoreError::config("最大算力必须大于0"));
                }
            } else {
                return Err(CoreError::config("max_hashrate 必须是正数"));
            }
        }

        // 验证最小和最大算力的关系
        if let (Some(min_val), Some(max_val)) = (
            config.custom_params.get("min_hashrate").and_then(|v| v.as_f64()),
            config.custom_params.get("max_hashrate").and_then(|v| v.as_f64()),
        ) {
            if min_val >= max_val {
                return Err(CoreError::config("最小算力必须小于最大算力"));
            }
        }

        if let Some(error_rate) = config.custom_params.get("error_rate") {
            if let Some(rate) = error_rate.as_f64() {
                if rate < 0.0 || rate > 1.0 {
                    return Err(CoreError::config("错误率必须在0.0到1.0之间"));
                }
            } else {
                return Err(CoreError::config("error_rate 必须是0.0到1.0之间的数值"));
            }
        }

        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig {
        use std::collections::HashMap;

        let mut custom_params = HashMap::new();
        custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
        custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1000000000.0).unwrap())); // 1 GH/s
        custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(5000000000.0).unwrap())); // 5 GH/s
        custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap())); // 1% 错误率
        custom_params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));
        custom_params.insert("work_timeout_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(5000)));

        CoreConfig {
            name: "software-core".to_string(),
            enabled: true,
            devices: vec![
                cgminer_core::DeviceConfig {
                    chain_id: 0,
                    enabled: true,
                    frequency: 600,
                    voltage: 900,
                    auto_tune: false,
                    chip_count: 64,
                    temperature_limit: 80.0,
                    fan_speed: Some(50),
                },
                cgminer_core::DeviceConfig {
                    chain_id: 1,
                    enabled: true,
                    frequency: 650,
                    voltage: 920,
                    auto_tune: false,
                    chip_count: 64,
                    temperature_limit: 80.0,
                    fan_speed: Some(55),
                },
                cgminer_core::DeviceConfig {
                    chain_id: 2,
                    enabled: true,
                    frequency: 700,
                    voltage: 950,
                    auto_tune: false,
                    chip_count: 64,
                    temperature_limit: 80.0,
                    fan_speed: Some(60),
                },
                cgminer_core::DeviceConfig {
                    chain_id: 3,
                    enabled: true,
                    frequency: 750,
                    voltage: 980,
                    auto_tune: false,
                    chip_count: 64,
                    temperature_limit: 80.0,
                    fan_speed: Some(65),
                },
            ],
            custom_params,
        }
    }
}
