//! 重命名后核心库的端到端集成测试
//!
//! 测试重命名后的核心库：
//! - cgminer-cpu-btc-core (Bitcoin软算法核心)
//! - cgminer-asic-maijie-l7-core (Maijie L7 ASIC核心)

use cgminer_rs::Config;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_end_to_end_mining_with_cpu_btc_core() {
    // 测试使用Bitcoin软算法核心进行完整挖矿流程
    #[cfg(feature = "cpu-btc")]
    {
        println!("🚀 开始Bitcoin软算法核心端到端测试");

        // 创建测试配置
        let config = create_cpu_btc_test_config();

        // 验证配置
        let validation_result = config.validate();
        assert!(validation_result.is_ok(), "配置验证应该成功: {:?}", validation_result.err());

        // 创建CGMiner实例
        let cgminer_result = cgminer_rs::CGMiner::new(config).await;
        assert!(cgminer_result.is_ok(), "CGMiner创建应该成功");

        let mut cgminer = cgminer_result.unwrap();

        // 启动挖矿
        let start_result = timeout(Duration::from_secs(10), cgminer.start()).await;
        assert!(start_result.is_ok(), "CGMiner启动不应该超时");
        assert!(start_result.unwrap().is_ok(), "CGMiner启动应该成功");

        // 等待一段时间让挖矿运行
        tokio::time::sleep(Duration::from_secs(5)).await;

        // 获取统计信息
        let stats_result = cgminer.get_stats().await;
        assert!(stats_result.is_ok(), "获取统计信息应该成功");

        let stats = stats_result.unwrap();
        assert!(stats.total_hashrate >= 0.0, "总算力应该大于等于0");
        assert!(stats.uptime.as_secs() > 0, "运行时间应该大于0");

        println!("✅ Bitcoin软算法核心统计信息:");
        println!("   总算力: {:.2} GH/s", stats.total_hashrate / 1_000_000_000.0);
        println!("   运行时间: {} 秒", stats.uptime.as_secs());
        println!("   设备数量: {}", stats.device_count);

        // 停止挖矿
        let stop_result = timeout(Duration::from_secs(10), cgminer.stop()).await;
        assert!(stop_result.is_ok(), "CGMiner停止不应该超时");
        assert!(stop_result.unwrap().is_ok(), "CGMiner停止应该成功");

        println!("✅ Bitcoin软算法核心端到端测试完成");
    }

    #[cfg(not(feature = "cpu-btc"))]
    {
        println!("ℹ️  Bitcoin软算法核心功能未启用，跳过端到端测试");
    }
}

#[tokio::test]
async fn test_end_to_end_mining_with_maijie_l7_core() {
    // 测试使用Maijie L7 ASIC核心进行完整挖矿流程
    #[cfg(feature = "maijie-l7")]
    {
        println!("🚀 开始Maijie L7 ASIC核心端到端测试");

        // 创建测试配置
        let config = create_maijie_l7_test_config();

        // 验证配置
        let validation_result = config.validate();
        assert!(validation_result.is_ok(), "配置验证应该成功: {:?}", validation_result.err());

        // 创建CGMiner实例
        let cgminer_result = cgminer_rs::CGMiner::new(config).await;

        match cgminer_result {
            Ok(mut cgminer) => {
                // 尝试启动挖矿（在没有实际硬件的情况下可能会失败）
                let start_result = timeout(Duration::from_secs(10), cgminer.start()).await;

                match start_result {
                    Ok(Ok(_)) => {
                        println!("✅ Maijie L7核心启动成功");

                        // 等待一段时间
                        tokio::time::sleep(Duration::from_secs(2)).await;

                        // 获取统计信息
                        if let Ok(stats) = cgminer.get_stats().await {
                            println!("✅ Maijie L7统计信息:");
                            println!("   总算力: {:.2} TH/s", stats.total_hashrate / 1_000_000_000_000.0);
                            println!("   设备数量: {}", stats.device_count);
                        }

                        // 停止挖矿
                        let _ = cgminer.stop().await;
                    }
                    Ok(Err(e)) => {
                        println!("ℹ️  Maijie L7核心启动失败（预期，因为没有实际硬件）: {}", e);
                    }
                    Err(_) => {
                        println!("ℹ️  Maijie L7核心启动超时（预期，因为没有实际硬件）");
                    }
                }
            }
            Err(e) => {
                println!("ℹ️  Maijie L7 CGMiner创建失败（预期，因为没有实际硬件）: {}", e);
            }
        }

        println!("✅ Maijie L7 ASIC核心端到端测试完成");
    }

    #[cfg(not(feature = "maijie-l7"))]
    {
        println!("ℹ️  Maijie L7 ASIC核心功能未启用，跳过端到端测试");
    }
}

#[tokio::test]
async fn test_core_feature_detection() {
    // 测试核心功能检测
    println!("🔍 检测可用的核心功能:");

    #[cfg(feature = "cpu-btc")]
    {
        println!("✅ Bitcoin软算法核心 (cgminer-cpu-btc-core) 已启用");
    }

    #[cfg(not(feature = "cpu-btc"))]
    {
        println!("❌ Bitcoin软算法核心 (cgminer-cpu-btc-core) 未启用");
    }

    #[cfg(feature = "maijie-l7")]
    {
        println!("✅ Maijie L7 ASIC核心 (cgminer-asic-maijie-l7-core) 已启用");
    }

    #[cfg(not(feature = "maijie-l7"))]
    {
        println!("❌ Maijie L7 ASIC核心 (cgminer-asic-maijie-l7-core) 未启用");
    }

    #[cfg(feature = "all-cores")]
    {
        println!("✅ 所有核心功能已启用");
    }

    #[cfg(not(feature = "all-cores"))]
    {
        println!("ℹ️  部分核心功能启用");
    }
}

#[tokio::test]
async fn test_config_compatibility_with_renamed_cores() {
    // 测试配置与重命名后核心的兼容性
    println!("🔧 测试配置兼容性");

    // 测试Bitcoin软算法核心配置
    let btc_config = create_cpu_btc_test_config();
    let btc_validation = btc_config.validate();
    assert!(btc_validation.is_ok(), "Bitcoin软算法核心配置应该有效");
    println!("✅ Bitcoin软算法核心配置验证通过");

    // 测试Maijie L7核心配置
    let l7_config = create_maijie_l7_test_config();
    let l7_validation = l7_config.validate();
    assert!(l7_validation.is_ok(), "Maijie L7核心配置应该有效");
    println!("✅ Maijie L7核心配置验证通过");

    // 测试混合配置
    let mixed_config = create_mixed_cores_test_config();
    let mixed_validation = mixed_config.validate();
    assert!(mixed_validation.is_ok(), "混合核心配置应该有效");
    println!("✅ 混合核心配置验证通过");
}

/// 创建Bitcoin软算法核心测试配置
fn create_cpu_btc_test_config() -> Config {
    Config {
        general: cgminer_rs::config::GeneralConfig {
            log_level: "info".to_string(),
            log_file: None,
            pid_file: None,
            work_restart_timeout: 30,
            scan_time: 5,
        },
        cores: cgminer_rs::config::CoresConfig {
            enabled_cores: vec!["cpu-btc".to_string()],
            default_core: "cpu-btc".to_string(),
            cpu_btc: Some(cgminer_rs::config::BtcSoftwareCoreConfig {
                enabled: true,
                device_count: 2,
                min_hashrate: 1_000_000_000.0,
                max_hashrate: 4_000_000_000.0,
                error_rate: 0.01,
                batch_size: 1000,
                work_timeout_ms: 3000,
                cpu_affinity: None,
            }),
            maijie_l7: None,
        },
        devices: cgminer_rs::config::DeviceConfig {
            auto_detect: true,
            scan_interval: 10,
            chains: vec![],
        },
        pools: cgminer_rs::config::PoolConfig {
            strategy: cgminer_rs::config::PoolStrategy::Failover,
            failover_timeout: 60,
            retry_interval: 30,
            pools: vec![
                cgminer_rs::config::PoolInfo {
                    url: "stratum+tcp://test.pool.com:4444".to_string(),
                    user: "test_user".to_string(),
                    password: "test_password".to_string(),
                    priority: 1,
                    quota: None,
                    enabled: true,
                },
            ],
        },
        api: cgminer_rs::config::ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: None,
            allow_origins: vec!["*".to_string()],
        },
        monitoring: cgminer_rs::config::MonitoringConfig {
            enabled: true,
            metrics_interval: 30,
            prometheus_port: None,
            alert_thresholds: cgminer_rs::config::AlertThresholds {
                temperature_warning: 80.0,
                temperature_critical: 90.0,
                hashrate_drop_percent: 20.0,
                error_rate_percent: 5.0,
                max_temperature: 85.0,
                max_cpu_usage: 90.0,
                max_memory_usage: 90.0,
                max_device_temperature: 90.0,
                max_error_rate: 5.0,
                min_hashrate: 1.0,
            },
        },
    }
}

/// 创建Maijie L7 ASIC核心测试配置
fn create_maijie_l7_test_config() -> Config {
    Config {
        general: cgminer_rs::config::GeneralConfig {
            log_level: "info".to_string(),
            log_file: None,
            pid_file: None,
            work_restart_timeout: 30,
            scan_time: 5,
        },
        cores: cgminer_rs::config::CoresConfig {
            enabled_cores: vec!["maijie-l7".to_string()],
            default_core: "maijie-l7".to_string(),
            cpu_btc: None,
            maijie_l7: Some(cgminer_rs::config::MaijieL7CoreConfig {
                enabled: true,
                chain_count: 3,
                spi_speed: 6000000,
                uart_baud: 115200,
                auto_detect: true,
                power_limit: 3000.0,
                cooling_mode: "auto".to_string(),
            }),
        },
        devices: cgminer_rs::config::DeviceConfig {
            auto_detect: true,
            scan_interval: 10,
            chains: vec![
                cgminer_rs::config::ChainConfig {
                    id: 0,
                    enabled: true,
                    frequency: 1000,
                    voltage: 900,
                    auto_tune: false,
                    chip_count: 126,
                },
                cgminer_rs::config::ChainConfig {
                    id: 1,
                    enabled: true,
                    frequency: 1000,
                    voltage: 900,
                    auto_tune: false,
                    chip_count: 126,
                },
                cgminer_rs::config::ChainConfig {
                    id: 2,
                    enabled: true,
                    frequency: 1000,
                    voltage: 900,
                    auto_tune: false,
                    chip_count: 126,
                },
            ],
        },
        pools: cgminer_rs::config::PoolConfig {
            strategy: cgminer_rs::config::PoolStrategy::Failover,
            failover_timeout: 60,
            retry_interval: 30,
            pools: vec![
                cgminer_rs::config::PoolInfo {
                    url: "stratum+tcp://test.pool.com:4444".to_string(),
                    user: "test_user".to_string(),
                    password: "test_password".to_string(),
                    priority: 1,
                    quota: None,
                    enabled: true,
                },
            ],
        },
        api: cgminer_rs::config::ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: None,
            allow_origins: vec!["*".to_string()],
        },
        monitoring: cgminer_rs::config::MonitoringConfig {
            enabled: true,
            metrics_interval: 30,
            prometheus_port: None,
            alert_thresholds: cgminer_rs::config::AlertThresholds {
                temperature_warning: 80.0,
                temperature_critical: 90.0,
                hashrate_drop_percent: 20.0,
                error_rate_percent: 5.0,
                max_temperature: 85.0,
                max_cpu_usage: 90.0,
                max_memory_usage: 90.0,
                max_device_temperature: 90.0,
                max_error_rate: 5.0,
                min_hashrate: 50.0, // 更高的最小算力适合ASIC
            },
        },
    }
}

/// 创建混合核心测试配置
fn create_mixed_cores_test_config() -> Config {
    Config {
        general: cgminer_rs::config::GeneralConfig {
            log_level: "info".to_string(),
            log_file: None,
            pid_file: None,
            work_restart_timeout: 30,
            scan_time: 5,
        },
        cores: cgminer_rs::config::CoresConfig {
            enabled_cores: vec!["cpu-btc".to_string(), "maijie-l7".to_string()],
            default_core: "cpu-btc".to_string(),
            cpu_btc: Some(cgminer_rs::config::BtcSoftwareCoreConfig {
                enabled: true,
                device_count: 2,
                min_hashrate: 1_000_000_000.0,
                max_hashrate: 4_000_000_000.0,
                error_rate: 0.01,
                batch_size: 1000,
                work_timeout_ms: 3000,
                cpu_affinity: None,
            }),
            maijie_l7: Some(cgminer_rs::config::MaijieL7CoreConfig {
                enabled: false, // 默认禁用，避免在没有硬件时出错
                chain_count: 3,
                spi_speed: 6000000,
                uart_baud: 115200,
                auto_detect: true,
                power_limit: 3000.0,
                cooling_mode: "auto".to_string(),
            }),
        },
        devices: cgminer_rs::config::DeviceConfig {
            auto_detect: true,
            scan_interval: 10,
            chains: vec![],
        },
        pools: cgminer_rs::config::PoolConfig {
            strategy: cgminer_rs::config::PoolStrategy::Failover,
            failover_timeout: 60,
            retry_interval: 30,
            pools: vec![
                cgminer_rs::config::PoolInfo {
                    url: "stratum+tcp://test.pool.com:4444".to_string(),
                    user: "test_user".to_string(),
                    password: "test_password".to_string(),
                    priority: 1,
                    quota: None,
                    enabled: true,
                },
            ],
        },
        api: cgminer_rs::config::ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: None,
            allow_origins: vec!["*".to_string()],
        },
        monitoring: cgminer_rs::config::MonitoringConfig {
            enabled: true,
            metrics_interval: 30,
            prometheus_port: None,
            alert_thresholds: cgminer_rs::config::AlertThresholds {
                temperature_warning: 80.0,
                temperature_critical: 90.0,
                hashrate_drop_percent: 20.0,
                error_rate_percent: 5.0,
                max_temperature: 85.0,
                max_cpu_usage: 90.0,
                max_memory_usage: 90.0,
                max_device_temperature: 90.0,
                max_error_rate: 5.0,
                min_hashrate: 1.0,
            },
        },
    }
}
