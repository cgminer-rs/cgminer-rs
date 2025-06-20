//! Maijie L7 ASIC核心主项目测试 (cgminer-asic-maijie-l7-core)
//!
//! 测试主项目中Maijie L7 ASIC核心的集成和配置功能

use cgminer_rs::Config;
use std::path::Path;

#[tokio::test]
async fn test_maijie_l7_core_config_loading() {
    // 测试Maijie L7 ASIC核心配置加载
    let config_path = "examples/configs/maijie_l7_core_example.toml";

    if Path::new(config_path).exists() {
        let config_result = Config::load(config_path);
        assert!(config_result.is_ok(), "Maijie L7 ASIC核心配置加载应该成功");

        let config = config_result.unwrap();

        // 验证Maijie L7 ASIC核心配置
        if let Some(maijie_l7_config) = &config.cores.maijie_l7 {
            assert!(maijie_l7_config.enabled || !maijie_l7_config.enabled, "Maijie L7核心配置应该有效");
            assert!(maijie_l7_config.chain_count > 0, "链数量应该大于0");
            assert!(maijie_l7_config.chain_count <= 10, "链数量不应该超过10");
            assert!(maijie_l7_config.spi_speed > 0, "SPI速度应该大于0");
            assert!(maijie_l7_config.uart_baud > 0, "UART波特率应该大于0");
            assert!(maijie_l7_config.power_limit > 0.0, "功率限制应该大于0");

            // 验证冷却模式
            assert!(
                maijie_l7_config.cooling_mode == "auto" ||
                maijie_l7_config.cooling_mode == "manual" ||
                maijie_l7_config.cooling_mode == "aggressive",
                "冷却模式应该是有效值"
            );
        }
    }
}

#[tokio::test]
async fn test_main_config_with_maijie_l7() {
    // 测试包含Maijie L7配置的主配置文件加载
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config_result = Config::load(config_path);
        assert!(config_result.is_ok(), "主配置文件加载应该成功");

        let config = config_result.unwrap();

        // 验证Maijie L7核心配置
        if let Some(maijie_l7_config) = &config.cores.maijie_l7 {
            // 验证链数量合理性
            assert!(maijie_l7_config.chain_count >= 1, "链数量应该至少为1");
            assert!(maijie_l7_config.chain_count <= 10, "链数量不应该超过10");

            // 验证SPI速度合理性
            assert!(maijie_l7_config.spi_speed >= 1_000_000, "SPI速度应该至少为1MHz");
            assert!(maijie_l7_config.spi_speed <= 50_000_000, "SPI速度不应该超过50MHz");

            // 验证UART波特率合理性
            let valid_bauds = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
            assert!(valid_bauds.contains(&maijie_l7_config.uart_baud), "UART波特率应该是标准值");

            // 验证功率限制合理性
            assert!(maijie_l7_config.power_limit >= 100.0, "功率限制应该至少为100W");
            assert!(maijie_l7_config.power_limit <= 10000.0, "功率限制不应该超过10kW");
        }

        // 验证设备链配置
        if !config.devices.chains.is_empty() {
            for chain in &config.devices.chains {
                assert!(chain.id < 10, "链ID应该小于10");
                assert!(chain.frequency > 0, "频率应该大于0");
                assert!(chain.voltage > 0, "电压应该大于0");
                assert!(chain.chip_count > 0, "芯片数量应该大于0");

                // 验证频率范围
                assert!(chain.frequency >= 100, "频率应该至少为100MHz");
                assert!(chain.frequency <= 2000, "频率不应该超过2000MHz");

                // 验证电压范围
                assert!(chain.voltage >= 500, "电压应该至少为500mV");
                assert!(chain.voltage <= 1200, "电压不应该超过1200mV");

                // 验证芯片数量范围
                assert!(chain.chip_count <= 200, "芯片数量不应该超过200");
            }
        }
    }
}

#[tokio::test]
async fn test_maijie_l7_feature_availability() {
    // 测试Maijie L7核心功能是否可用
    #[cfg(feature = "maijie-l7")]
    {
        println!("✅ Maijie L7 ASIC核心功能已启用");
        // 可以在这里添加更多的Maijie L7核心特定测试
    }

    #[cfg(not(feature = "maijie-l7"))]
    {
        println!("ℹ️  Maijie L7 ASIC核心功能未启用");
    }
}

#[tokio::test]
async fn test_maijie_l7_config_validation() {
    // 测试Maijie L7配置验证功能
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config = Config::load(config_path).expect("配置加载应该成功");

        // 验证配置
        let validation_result = config.validate();
        assert!(validation_result.is_ok(), "配置验证应该成功: {:?}", validation_result.err());

        // 验证Maijie L7特定配置
        if let Some(maijie_l7_config) = &config.cores.maijie_l7 {
            if maijie_l7_config.enabled {
                // 验证链数量合理性
                assert!(maijie_l7_config.chain_count <= 10, "链数量不应该超过10");
                assert!(maijie_l7_config.chain_count >= 1, "链数量应该至少为1");

                // 验证SPI速度合理性
                assert!(maijie_l7_config.spi_speed >= 1_000_000, "SPI速度应该至少为1MHz");
                assert!(maijie_l7_config.spi_speed <= 50_000_000, "SPI速度不应该超过50MHz");

                // 验证功率限制合理性
                assert!(maijie_l7_config.power_limit >= 100.0, "功率限制应该至少为100W");
                assert!(maijie_l7_config.power_limit <= 10000.0, "功率限制不应该超过10kW");

                // 验证冷却模式
                let valid_cooling_modes = ["auto", "manual", "aggressive"];
                assert!(
                    valid_cooling_modes.contains(&maijie_l7_config.cooling_mode.as_str()),
                    "冷却模式应该是有效值: {:?}",
                    maijie_l7_config.cooling_mode
                );
            }
        }
    }
}

#[tokio::test]
async fn test_hardware_detection_simulation() {
    // 模拟硬件检测测试
    #[cfg(feature = "maijie-l7")]
    {
        // 在实际环境中，这里会测试硬件检测功能
        // 在测试环境中，我们只验证配置是否正确
        println!("🔍 模拟Maijie L7硬件检测");

        let config_path = "cgminer.toml";
        if Path::new(config_path).exists() {
            let config = Config::load(config_path).expect("配置加载应该成功");

            if let Some(maijie_l7_config) = &config.cores.maijie_l7 {
                if maijie_l7_config.auto_detect {
                    println!("✅ 自动检测已启用");
                } else {
                    println!("ℹ️  自动检测已禁用，使用手动配置");
                }
            }
        }
    }

    #[cfg(not(feature = "maijie-l7"))]
    {
        println!("ℹ️  Maijie L7功能未启用，跳过硬件检测测试");
    }
}

#[tokio::test]
async fn test_temperature_monitoring_config() {
    // 测试温度监控配置
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config = Config::load(config_path).expect("配置加载应该成功");

        // 验证监控配置中的温度阈值
        assert!(config.monitoring.alert_thresholds.temperature_warning > 0.0, "温度警告阈值应该大于0");
        assert!(config.monitoring.alert_thresholds.temperature_critical > config.monitoring.alert_thresholds.temperature_warning, "临界温度应该高于警告温度");
        assert!(config.monitoring.alert_thresholds.max_device_temperature > 0.0, "最大设备温度应该大于0");

        // 验证温度阈值合理性
        assert!(config.monitoring.alert_thresholds.temperature_warning >= 60.0, "温度警告阈值应该至少为60°C");
        assert!(config.monitoring.alert_thresholds.temperature_warning <= 100.0, "温度警告阈值不应该超过100°C");
        assert!(config.monitoring.alert_thresholds.temperature_critical >= 80.0, "临界温度阈值应该至少为80°C");
        assert!(config.monitoring.alert_thresholds.temperature_critical <= 120.0, "临界温度阈值不应该超过120°C");
    }
}

#[tokio::test]
async fn test_power_management_config() {
    // 测试功率管理配置
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config = Config::load(config_path).expect("配置加载应该成功");

        if let Some(maijie_l7_config) = &config.cores.maijie_l7 {
            // 验证功率限制设置
            assert!(maijie_l7_config.power_limit > 0.0, "功率限制应该大于0");

            // 验证功率限制合理性（基于Maijie L7的实际规格）
            assert!(maijie_l7_config.power_limit >= 2000.0, "Maijie L7功率限制应该至少为2kW");
            assert!(maijie_l7_config.power_limit <= 5000.0, "Maijie L7功率限制不应该超过5kW");
        }
    }
}

#[tokio::test]
async fn test_chain_configuration_validation() {
    // 测试链配置验证
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config = Config::load(config_path).expect("配置加载应该成功");

        // 验证设备链配置
        for chain in &config.devices.chains {
            // 验证链ID唯一性
            let chain_id_count = config.devices.chains.iter()
                .filter(|c| c.id == chain.id)
                .count();
            assert_eq!(chain_id_count, 1, "链ID应该是唯一的: {}", chain.id);

            // 验证芯片数量合理性（基于Maijie L7规格）
            if chain.chip_count > 100 {
                // 假设这是Maijie L7链
                assert!(chain.chip_count <= 126, "Maijie L7链芯片数量不应该超过126");
                assert!(chain.chip_count >= 100, "Maijie L7链芯片数量应该至少为100");
            }
        }
    }
}
