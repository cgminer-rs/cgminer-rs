//! cgminer-core 能力测试
//! 
//! 这个测试模块验证不同类型的挖矿核心是否正确实现了其声明的能力，
//! 确保接口一致性和功能正确性。

use cgminer_core::{MiningCore, CoreCapabilities, CoreConfig, CoreError};
use std::collections::HashMap;
use tokio;

/// 核心能力测试特征
/// 
/// 定义了所有核心都应该通过的基础测试
#[async_trait::async_trait]
pub trait CoreCapabilityTest {
    /// 测试核心基础信息
    async fn test_core_info(&self) -> Result<(), CoreError>;
    
    /// 测试核心能力声明与实际功能的一致性
    async fn test_capability_consistency(&mut self) -> Result<(), CoreError>;
    
    /// 测试设备扫描功能
    async fn test_device_scanning(&self) -> Result<(), CoreError>;
    
    /// 测试配置验证
    async fn test_config_validation(&self) -> Result<(), CoreError>;
    
    /// 测试错误处理
    async fn test_error_handling(&mut self) -> Result<(), CoreError>;
}

/// 通用核心测试套件
pub struct CoreTestSuite;

impl CoreTestSuite {
    /// 运行完整的核心测试套件
    pub async fn run_full_test_suite<T: MiningCore>(
        mut core: T,
        test_config: CoreConfig,
    ) -> Result<(), CoreError> {
        println!("🧪 开始运行核心测试套件: {}", core.get_info().name);
        
        // 1. 测试核心信息
        Self::test_core_info(&core).await?;
        
        // 2. 测试初始化
        Self::test_initialization(&mut core, test_config.clone()).await?;
        
        // 3. 测试能力一致性
        Self::test_capability_consistency(&core).await?;
        
        // 4. 测试设备管理
        Self::test_device_management(&core).await?;
        
        // 5. 测试生命周期管理
        Self::test_lifecycle_management(&mut core).await?;
        
        println!("✅ 核心测试套件完成: {}", core.get_info().name);
        Ok(())
    }
    
    /// 测试核心基础信息
    async fn test_core_info<T: MiningCore>(core: &T) -> Result<(), CoreError> {
        let info = core.get_info();
        
        // 验证基础信息完整性
        assert!(!info.name.is_empty(), "核心名称不能为空");
        assert!(!info.version.is_empty(), "版本信息不能为空");
        assert!(!info.description.is_empty(), "描述信息不能为空");
        assert!(!info.author.is_empty(), "作者信息不能为空");
        assert!(!info.supported_devices.is_empty(), "支持的设备类型不能为空");
        
        println!("✓ 核心信息验证通过: {}", info.name);
        Ok(())
    }
    
    /// 测试核心初始化
    async fn test_initialization<T: MiningCore>(
        core: &mut T,
        config: CoreConfig,
    ) -> Result<(), CoreError> {
        // 测试正常初始化
        core.initialize(config.clone()).await?;
        
        // 测试重复初始化（应该处理优雅）
        let result = core.initialize(config).await;
        match result {
            Ok(_) => println!("✓ 支持重复初始化"),
            Err(_) => println!("✓ 拒绝重复初始化（符合预期）"),
        }
        
        println!("✓ 初始化测试通过");
        Ok(())
    }
    
    /// 测试能力一致性
    async fn test_capability_consistency<T: MiningCore>(core: &T) -> Result<(), CoreError> {
        let capabilities = core.get_capabilities();
        let info = core.get_info();
        
        // 验证能力声明的逻辑一致性
        if capabilities.supports_voltage_control {
            assert!(
                capabilities.supports_temperature_monitoring,
                "支持电压控制的核心通常也应该支持温度监控"
            );
        }
        
        if capabilities.supports_fan_control {
            assert!(
                capabilities.supports_temperature_monitoring,
                "支持风扇控制的核心必须支持温度监控"
            );
        }
        
        // 验证算法支持
        assert!(
            !capabilities.supported_algorithms.is_empty(),
            "必须至少支持一种算法"
        );
        
        // 根据核心类型验证特定能力
        match info.core_type {
            cgminer_core::CoreType::Asic => {
                assert!(
                    capabilities.supports_temperature_monitoring,
                    "ASIC核心必须支持温度监控"
                );
                assert!(
                    capabilities.supports_voltage_control,
                    "ASIC核心通常支持电压控制"
                );
            }
            cgminer_core::CoreType::Custom(ref type_name) => {
                match type_name.as_str() {
                    "software" => {
                        assert!(
                            !capabilities.supports_voltage_control,
                            "软算法核心不应该支持电压控制"
                        );
                        assert!(
                            !capabilities.supports_fan_control,
                            "软算法核心不应该支持风扇控制"
                        );
                    }
                    "gpu" => {
                        assert!(
                            capabilities.supports_temperature_monitoring,
                            "GPU核心必须支持温度监控"
                        );
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        println!("✓ 能力一致性验证通过");
        Ok(())
    }
    
    /// 测试设备管理
    async fn test_device_management<T: MiningCore>(core: &T) -> Result<(), CoreError> {
        // 测试设备扫描
        let devices = core.scan_devices().await?;
        
        // 验证设备数量限制
        if let Some(max_devices) = core.get_capabilities().max_devices {
            assert!(
                devices.len() <= max_devices as usize,
                "扫描到的设备数量不能超过最大限制"
            );
        }
        
        // 验证设备信息完整性
        for device in &devices {
            assert!(!device.name.is_empty(), "设备名称不能为空");
            assert!(!device.device_type.is_empty(), "设备类型不能为空");
        }
        
        println!("✓ 设备管理测试通过，扫描到 {} 个设备", devices.len());
        Ok(())
    }
    
    /// 测试生命周期管理
    async fn test_lifecycle_management<T: MiningCore>(core: &mut T) -> Result<(), CoreError> {
        // 测试启动
        core.start().await?;
        
        // 测试停止
        core.stop().await?;
        
        // 测试重启
        core.restart().await?;
        
        println!("✓ 生命周期管理测试通过");
        Ok(())
    }
}

/// 创建测试配置
pub fn create_test_config(core_name: &str, custom_params: HashMap<String, serde_json::Value>) -> CoreConfig {
    CoreConfig {
        name: core_name.to_string(),
        enabled: true,
        devices: Vec::new(),
        custom_params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    /// 测试配置创建
    #[test]
    fn test_config_creation() {
        let mut params = HashMap::new();
        params.insert("test_param".to_string(), json!("test_value"));
        
        let config = create_test_config("test_core", params);
        
        assert_eq!(config.name, "test_core");
        assert!(config.enabled);
        assert_eq!(config.custom_params.get("test_param").unwrap(), &json!("test_value"));
    }
}
