//! 核心集成测试

use cgminer_rs::{CoreLoader, Config};
use cgminer_core::{CoreType, CoreConfig};
use std::collections::HashMap;
use tokio_test;

#[tokio::test]
async fn test_core_loader_basic_functionality() {
    // 创建核心加载器
    let core_loader = CoreLoader::new();
    
    // 加载所有可用的核心
    let result = core_loader.load_all_cores().await;
    assert!(result.is_ok(), "Failed to load cores: {:?}", result.err());
    
    // 检查加载统计
    let stats = core_loader.get_load_stats().unwrap();
    println!("Load stats: {}", stats);
    
    // 至少应该有一个核心被加载
    assert!(stats.total_loaded > 0, "No cores were loaded");
    
    // 列出所有已加载的核心
    let cores = core_loader.list_loaded_cores().unwrap();
    assert!(!cores.is_empty(), "No cores found in registry");
    
    for core in &cores {
        println!("Loaded core: {} ({}): {}", core.name, core.core_type, core.description);
    }
}

#[cfg(feature = "software-core")]
#[tokio::test]
async fn test_software_core_creation() {
    let core_loader = CoreLoader::new();
    core_loader.load_all_cores().await.unwrap();
    
    // 创建软算法核心配置
    let mut custom_params = HashMap::new();
    custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
    custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(500_000_000.0).unwrap()));
    custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1_000_000_000.0).unwrap()));
    custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap()));
    
    let core_config = CoreConfig {
        name: "test-software-core".to_string(),
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
        ],
        custom_params,
    };
    
    // 创建软算法核心实例
    let registry = core_loader.registry();
    let core_id = registry.create_core("software", core_config).await;
    assert!(core_id.is_ok(), "Failed to create software core: {:?}", core_id.err());
    
    let core_id = core_id.unwrap();
    println!("Created software core with ID: {}", core_id);
    
    // 验证核心已创建
    let active_cores = registry.list_active_cores().unwrap();
    assert!(active_cores.contains(&core_id), "Core not found in active cores list");
    
    // 清理
    registry.remove_core(&core_id).await.unwrap();
}

#[cfg(feature = "asic-core")]
#[tokio::test]
async fn test_asic_core_creation() {
    let core_loader = CoreLoader::new();
    core_loader.load_all_cores().await.unwrap();
    
    // 创建ASIC核心配置
    let mut custom_params = HashMap::new();
    custom_params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
    custom_params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(6_000_000)));
    custom_params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(115200)));
    custom_params.insert("auto_detect".to_string(), serde_json::Value::Bool(true));
    
    let core_config = CoreConfig {
        name: "test-asic-core".to_string(),
        enabled: true,
        devices: vec![
            cgminer_core::DeviceConfig {
                chain_id: 0,
                enabled: true,
                frequency: 650,
                voltage: 900,
                auto_tune: true,
                chip_count: 126,
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
    };
    
    // 创建ASIC核心实例
    let registry = core_loader.registry();
    let core_id = registry.create_core("asic", core_config).await;
    assert!(core_id.is_ok(), "Failed to create ASIC core: {:?}", core_id.err());
    
    let core_id = core_id.unwrap();
    println!("Created ASIC core with ID: {}", core_id);
    
    // 验证核心已创建
    let active_cores = registry.list_active_cores().unwrap();
    assert!(active_cores.contains(&core_id), "Core not found in active cores list");
    
    // 清理
    registry.remove_core(&core_id).await.unwrap();
}

#[tokio::test]
async fn test_core_type_filtering() {
    let core_loader = CoreLoader::new();
    core_loader.load_all_cores().await.unwrap();
    
    // 测试按类型获取核心
    #[cfg(feature = "software-core")]
    {
        let software_cores = core_loader.get_cores_by_type(&CoreType::Custom("software".to_string())).unwrap();
        assert!(!software_cores.is_empty(), "No software cores found");
        
        for core in &software_cores {
            println!("Software core: {}", core.name);
            assert_eq!(core.core_type, CoreType::Custom("software".to_string()));
        }
    }
    
    #[cfg(feature = "asic-core")]
    {
        let asic_cores = core_loader.get_cores_by_type(&CoreType::Asic).unwrap();
        assert!(!asic_cores.is_empty(), "No ASIC cores found");
        
        for core in &asic_cores {
            println!("ASIC core: {}", core.name);
            assert_eq!(core.core_type, CoreType::Asic);
        }
    }
}

#[tokio::test]
async fn test_core_loader_shutdown() {
    let core_loader = CoreLoader::new();
    core_loader.load_all_cores().await.unwrap();
    
    // 验证核心已加载
    let stats_before = core_loader.get_load_stats().unwrap();
    assert!(stats_before.total_loaded > 0, "No cores loaded before shutdown");
    
    // 关闭所有核心
    let result = core_loader.shutdown().await;
    assert!(result.is_ok(), "Failed to shutdown cores: {:?}", result.err());
    
    // 验证核心已关闭
    let stats_after = core_loader.get_load_stats().unwrap();
    assert_eq!(stats_after.total_loaded, 0, "Cores still loaded after shutdown");
    assert_eq!(stats_after.active_cores, 0, "Active cores still exist after shutdown");
}

#[tokio::test]
async fn test_config_validation() {
    // 测试有效配置
    let valid_config = Config::default();
    assert!(valid_config.validate().is_ok(), "Default config should be valid");
    
    // 测试无效配置 - 空的启用核心列表
    let mut invalid_config = Config::default();
    invalid_config.cores.enabled_cores.clear();
    assert!(invalid_config.validate().is_err(), "Config with empty enabled_cores should be invalid");
    
    // 测试无效配置 - 默认核心不在启用列表中
    let mut invalid_config = Config::default();
    invalid_config.cores.default_core = "nonexistent".to_string();
    assert!(invalid_config.validate().is_err(), "Config with invalid default_core should be invalid");
    
    // 测试软算法核心配置验证
    let mut invalid_config = Config::default();
    if let Some(ref mut software_config) = invalid_config.cores.software_core {
        software_config.device_count = 0; // 无效的设备数量
    }
    assert!(invalid_config.validate().is_err(), "Config with invalid software core config should be invalid");
}
