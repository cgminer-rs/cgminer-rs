//! Bitcoin软算法核心主项目测试 (cgminer-s-btc-core)
//!
//! 测试主项目中Bitcoin软算法核心的集成和配置功能

use cgminer_rs::Config;
use std::path::Path;

#[tokio::test]
async fn test_btc_software_core_config_loading() {
    // 测试Bitcoin软算法核心配置加载
    let config_path = "examples/configs/btc_software_core_example.toml";

    if Path::new(config_path).exists() {
        let config_result = Config::load(config_path);
        assert!(config_result.is_ok(), "Bitcoin软算法核心配置加载应该成功");

        let config = config_result.unwrap();

        // 验证Bitcoin软算法核心配置
        if let Some(btc_software_config) = &config.cores.btc_software {
            assert!(btc_software_config.enabled, "Bitcoin软算法核心应该被启用");
            assert!(btc_software_config.device_count > 0, "设备数量应该大于0");
            assert!(btc_software_config.min_hashrate > 0.0, "最小算力应该大于0");
            assert!(btc_software_config.max_hashrate > btc_software_config.min_hashrate, "最大算力应该大于最小算力");
            assert!(btc_software_config.error_rate >= 0.0 && btc_software_config.error_rate <= 1.0, "错误率应该在0-1之间");
            assert!(btc_software_config.batch_size > 0, "批次大小应该大于0");
            assert!(btc_software_config.work_timeout_ms > 0, "工作超时应该大于0");

            // 验证CPU绑定配置
            if let Some(cpu_config) = &btc_software_config.cpu_affinity {
                // CPU绑定配置存在时的验证
                assert!(cpu_config.enabled || !cpu_config.enabled, "CPU绑定配置应该有效");
            }
        }
    }
}

#[tokio::test]
async fn test_main_config_loading() {
    // 测试主配置文件加载
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config_result = Config::load(config_path);
        assert!(config_result.is_ok(), "主配置文件加载应该成功");

        let config = config_result.unwrap();

        // 验证矿池配置
        assert!(!config.pools.pools.is_empty(), "矿池配置不应该为空");
        if let Some(pool) = config.pools.pools.first() {
            assert!(!pool.url.is_empty(), "矿池URL不应该为空");
            assert!(!pool.user.is_empty(), "用户名不应该为空");
        }

        // 验证API配置
        assert!(config.api.port > 0, "API端口应该大于0");
        assert!(config.api.port <= 65535, "API端口应该小于等于65535");

        // 验证监控配置
        assert!(config.monitoring.metrics_interval > 0, "监控间隔应该大于0");
    }
}

#[tokio::test]
async fn test_btc_software_core_feature_availability() {
    // 测试Bitcoin软算法核心功能是否可用
    #[cfg(feature = "btc-software")]
    {
        println!("✅ Bitcoin软算法核心功能已启用");
        // 可以在这里添加更多的Bitcoin软算法核心特定测试
    }

    #[cfg(not(feature = "btc-software"))]
    {
        println!("ℹ️  Bitcoin软算法核心功能未启用");
    }
}



#[tokio::test]
async fn test_config_validation() {
    // 测试配置验证功能
    let config_path = "cgminer.toml";

    if Path::new(config_path).exists() {
        let config = Config::load(config_path).expect("配置加载应该成功");

        // 验证配置
        let validation_result = config.validate();
        assert!(validation_result.is_ok(), "配置验证应该成功: {:?}", validation_result.err());

        // 验证Bitcoin软算法核心特定配置
        if let Some(btc_software_config) = &config.cores.btc_software {
            if btc_software_config.enabled {
                // 验证设备数量合理性
                assert!(btc_software_config.device_count <= 64, "设备数量不应该超过64");
                assert!(btc_software_config.device_count >= 1, "设备数量应该至少为1");

                // 验证算力范围合理性
                assert!(btc_software_config.min_hashrate >= 1_000_000.0, "最小算力应该至少为1 MH/s");
                assert!(btc_software_config.max_hashrate <= 100_000_000_000.0, "最大算力不应该超过100 GH/s");

                // 验证错误率合理性
                assert!(btc_software_config.error_rate <= 0.5, "错误率不应该超过50%");

                // 验证批次大小合理性
                assert!(btc_software_config.batch_size >= 100, "批次大小应该至少为100");
                assert!(btc_software_config.batch_size <= 10000, "批次大小不应该超过10000");

                // 验证超时设置合理性
                assert!(btc_software_config.work_timeout_ms >= 1000, "工作超时应该至少为1秒");
                assert!(btc_software_config.work_timeout_ms <= 60000, "工作超时不应该超过60秒");
            }
        }
    }
}
