#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        
        // 测试默认值
        assert_eq!(config.mining.scan_interval, Duration::from_secs(5));
        assert_eq!(config.mining.work_restart_timeout, Duration::from_secs(60));
        assert!(config.mining.enable_auto_tuning);
        
        assert_eq!(config.devices.scan_interval, 10);
        assert_eq!(config.devices.chains.len(), 2);
        
        assert_eq!(config.pools.strategy, crate::pool::PoolStrategy::Failover);
        assert_eq!(config.pools.retry_interval, 30);
        assert_eq!(config.pools.pools.len(), 1);
        
        assert!(config.api.enabled);
        assert_eq!(config.api.bind_address, "127.0.0.1");
        assert_eq!(config.api.port, 8080);
        
        assert!(config.monitoring.enabled);
        assert_eq!(config.monitoring.metrics_interval, 30);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // 测试有效配置
        assert!(config.is_valid());
        
        // 测试无效API端口
        config.api.port = 0;
        assert!(!config.is_valid());
        
        // 恢复有效端口
        config.api.port = 8080;
        assert!(config.is_valid());
        
        // 测试无效扫描间隔
        config.devices.scan_interval = 0;
        assert!(!config.is_valid());
    }

    #[test]
    fn test_mining_config() {
        let config = MiningConfig {
            scan_interval: Duration::from_secs(10),
            work_restart_timeout: Duration::from_secs(30),
            enable_auto_tuning: false,
        };
        
        assert_eq!(config.scan_interval, Duration::from_secs(10));
        assert_eq!(config.work_restart_timeout, Duration::from_secs(30));
        assert!(!config.enable_auto_tuning);
    }

    #[test]
    fn test_device_config() {
        let config = DeviceConfig {
            scan_interval: 5,
            chains: vec![
                ChainConfig {
                    id: 0,
                    enabled: true,
                    frequency: 600,
                    voltage: 900,
                    auto_tune: false,
                    chip_count: 80,
                },
            ],
        };
        
        assert_eq!(config.scan_interval, 5);
        assert_eq!(config.chains.len(), 1);
        assert_eq!(config.chains[0].frequency, 600);
        assert_eq!(config.chains[0].voltage, 900);
        assert!(!config.chains[0].auto_tune);
    }

    #[test]
    fn test_chain_config_validation() {
        let mut config = ChainConfig {
            id: 0,
            enabled: true,
            frequency: 500,
            voltage: 850,
            auto_tune: true,
            chip_count: 76,
        };
        
        // 测试有效配置
        assert!(config.is_valid());
        
        // 测试无效频率
        config.frequency = 50; // 太低
        assert!(!config.is_valid());
        
        config.frequency = 1000; // 太高
        assert!(!config.is_valid());
        
        // 恢复有效频率
        config.frequency = 500;
        assert!(config.is_valid());
        
        // 测试无效电压
        config.voltage = 500; // 太低
        assert!(!config.is_valid());
        
        config.voltage = 1200; // 太高
        assert!(!config.is_valid());
        
        // 恢复有效电压
        config.voltage = 850;
        assert!(config.is_valid());
        
        // 测试无效芯片数量
        config.chip_count = 0;
        assert!(!config.is_valid());
        
        config.chip_count = 200; // 太多
        assert!(!config.is_valid());
    }

    #[test]
    fn test_pool_config() {
        let config = PoolConfig {
            strategy: crate::pool::PoolStrategy::LoadBalance,
            retry_interval: 60,
            pools: vec![
                PoolInfo {
                    url: "stratum+tcp://pool1.example.com:4444".to_string(),
                    user: "user1".to_string(),
                    password: "pass1".to_string(),
                    priority: 1,
                },
                PoolInfo {
                    url: "stratum+tcp://pool2.example.com:4444".to_string(),
                    user: "user2".to_string(),
                    password: "pass2".to_string(),
                    priority: 2,
                },
            ],
        };
        
        assert_eq!(config.strategy, crate::pool::PoolStrategy::LoadBalance);
        assert_eq!(config.retry_interval, 60);
        assert_eq!(config.pools.len(), 2);
        assert_eq!(config.pools[0].priority, 1);
        assert_eq!(config.pools[1].priority, 2);
    }

    #[test]
    fn test_pool_info_validation() {
        let mut pool_info = PoolInfo {
            url: "stratum+tcp://pool.example.com:4444".to_string(),
            user: "test_user".to_string(),
            password: "test_password".to_string(),
            priority: 1,
        };
        
        // 测试有效配置
        assert!(pool_info.is_valid());
        
        // 测试无效URL
        pool_info.url = "invalid_url".to_string();
        assert!(!pool_info.is_valid());
        
        // 恢复有效URL
        pool_info.url = "stratum+tcp://pool.example.com:4444".to_string();
        assert!(pool_info.is_valid());
        
        // 测试空用户名
        pool_info.user = "".to_string();
        assert!(!pool_info.is_valid());
        
        // 恢复有效用户名
        pool_info.user = "test_user".to_string();
        assert!(pool_info.is_valid());
        
        // 测试无效优先级
        pool_info.priority = 0;
        assert!(!pool_info.is_valid());
        
        pool_info.priority = 11; // 太高
        assert!(!pool_info.is_valid());
    }

    #[test]
    fn test_api_config() {
        let config = ApiConfig {
            enabled: false,
            bind_address: "0.0.0.0".to_string(),
            port: 9090,
            auth_token: Some("secret_token".to_string()),
            allow_origins: vec!["https://example.com".to_string()],
        };
        
        assert!(!config.enabled);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 9090);
        assert_eq!(config.auth_token, Some("secret_token".to_string()));
        assert_eq!(config.allow_origins.len(), 1);
    }

    #[test]
    fn test_api_config_validation() {
        let mut config = ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: None,
            allow_origins: vec!["*".to_string()],
        };
        
        // 测试有效配置
        assert!(config.is_valid());
        
        // 测试无效端口
        config.port = 0;
        assert!(!config.is_valid());
        
        config.port = 70000; // 太大
        assert!(!config.is_valid());
        
        // 恢复有效端口
        config.port = 8080;
        assert!(config.is_valid());
        
        // 测试无效绑定地址
        config.bind_address = "invalid_ip".to_string();
        assert!(!config.is_valid());
    }

    #[test]
    fn test_monitoring_config() {
        let config = MonitoringConfig {
            enabled: false,
            metrics_interval: 60,
            alert_thresholds: AlertThresholds {
                max_temperature: 90.0,
                max_device_temperature: 95.0,
                max_cpu_usage: 95,
                max_memory_usage: 95,
                max_error_rate: 10.0,
                min_hashrate: 25.0,
            },
        };
        
        assert!(!config.enabled);
        assert_eq!(config.metrics_interval, 60);
        assert_eq!(config.alert_thresholds.max_temperature, 90.0);
        assert_eq!(config.alert_thresholds.max_device_temperature, 95.0);
    }

    #[test]
    fn test_alert_thresholds_validation() {
        let mut thresholds = AlertThresholds {
            max_temperature: 85.0,
            max_device_temperature: 90.0,
            max_cpu_usage: 90,
            max_memory_usage: 90,
            max_error_rate: 5.0,
            min_hashrate: 30.0,
        };
        
        // 测试有效阈值
        assert!(thresholds.is_valid());
        
        // 测试无效温度阈值
        thresholds.max_temperature = 150.0; // 太高
        assert!(!thresholds.is_valid());
        
        thresholds.max_temperature = -10.0; // 太低
        assert!(!thresholds.is_valid());
        
        // 恢复有效温度
        thresholds.max_temperature = 85.0;
        assert!(thresholds.is_valid());
        
        // 测试无效CPU使用率
        thresholds.max_cpu_usage = 150; // 超过100%
        assert!(!thresholds.is_valid());
        
        // 测试无效错误率
        thresholds.max_error_rate = -1.0; // 负数
        assert!(!thresholds.is_valid());
        
        // 测试无效最小算力
        thresholds.min_hashrate = -5.0; // 负数
        assert!(!thresholds.is_valid());
    }

    #[test]
    fn test_config_toml_serialization() {
        let config = Config::default();
        
        // 测试序列化
        let toml_string = toml::to_string(&config).expect("Failed to serialize config");
        assert!(!toml_string.is_empty());
        
        // 测试反序列化
        let deserialized_config: Config = toml::from_str(&toml_string).expect("Failed to deserialize config");
        
        // 验证关键字段
        assert_eq!(config.mining.scan_interval, deserialized_config.mining.scan_interval);
        assert_eq!(config.devices.chains.len(), deserialized_config.devices.chains.len());
        assert_eq!(config.api.port, deserialized_config.api.port);
    }

    #[test]
    fn test_config_from_args() {
        let args = Args {
            config: Some("test_config.toml".to_string()),
            verbose: 2,
            daemon: true,
            log_file: Some("test.log".to_string()),
        };
        
        // 测试从参数创建配置
        let config = Config::from(&args);
        
        // 验证默认值被正确设置
        assert!(config.is_valid());
    }

    #[test]
    fn test_args_default() {
        let args = Args::default();
        
        assert_eq!(args.config, None);
        assert_eq!(args.verbose, 0);
        assert!(!args.daemon);
        assert_eq!(args.log_file, None);
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        let override_config = Config {
            mining: MiningConfig {
                scan_interval: Duration::from_secs(10),
                work_restart_timeout: Duration::from_secs(120),
                enable_auto_tuning: false,
            },
            ..Config::default()
        };
        
        // 合并配置
        base_config.merge(override_config);
        
        // 验证合并结果
        assert_eq!(base_config.mining.scan_interval, Duration::from_secs(10));
        assert_eq!(base_config.mining.work_restart_timeout, Duration::from_secs(120));
        assert!(!base_config.mining.enable_auto_tuning);
    }

    #[test]
    fn test_config_environment_variables() {
        // 设置环境变量
        std::env::set_var("CGMINER_API_PORT", "9090");
        std::env::set_var("CGMINER_API_BIND_ADDRESS", "0.0.0.0");
        
        let mut config = Config::default();
        config.apply_environment_overrides();
        
        // 验证环境变量覆盖
        assert_eq!(config.api.port, 9090);
        assert_eq!(config.api.bind_address, "0.0.0.0");
        
        // 清理环境变量
        std::env::remove_var("CGMINER_API_PORT");
        std::env::remove_var("CGMINER_API_BIND_ADDRESS");
    }
}
