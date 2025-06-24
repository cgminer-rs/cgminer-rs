#[cfg(test)]
mod tests {

    use crate::device::{DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult, DeviceConfig};
    // VirtualDevice removed - using cgminer-cpu-btc-core instead
    use crate::device::traits::{MiningDevice, DeviceDriver};
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;
    use tokio;

    #[test]
    fn test_device_info_creation() {
        let mut device_info = DeviceInfo::new(
            0,
            "Test Device".to_string(),
            "test-device".to_string(),
            0,
        );

        assert_eq!(device_info.id, 0);
        assert_eq!(device_info.name, "Test Device");
        assert_eq!(device_info.device_type, "test-device");
        assert_eq!(device_info.chain_id, 0);

        // 新创建的设备状态是 Uninitialized，不是健康状态
        assert!(!device_info.is_healthy());

        // 设置为 Idle 状态后应该是健康的
        device_info.update_status(DeviceStatus::Idle);
        assert!(device_info.is_healthy());
    }

    #[test]
    fn test_device_info_status_update() {
        let mut device_info = DeviceInfo::new(
            0,
            "Test Device".to_string(),
            "test-device".to_string(),
            0,
        );

        // 测试状态更新
        device_info.update_status(DeviceStatus::Error("Test error".to_string()));
        assert!(!device_info.is_healthy());

        device_info.update_status(DeviceStatus::Mining);
        assert!(device_info.is_healthy());
    }

    #[test]
    fn test_device_info_temperature_update() {
        let mut device_info = DeviceInfo::new(
            0,
            "Test Device".to_string(),
            "test-device".to_string(),
            0,
        );

        // 测试温度更新
        device_info.update_temperature(75.5);
        assert_eq!(device_info.temperature, Some(75.5));

        // 测试过热检测
        device_info.update_temperature(95.0);
        assert!(!device_info.is_healthy());
    }

    #[test]
    fn test_device_info_hashrate_update() {
        let mut device_info = DeviceInfo::new(
            0,
            "Test Device".to_string(),
            "test-device".to_string(),
            0,
        );

        // 测试算力更新
        device_info.update_hashrate(38.5);
        assert_eq!(device_info.hashrate, 38.5);
    }

    #[test]
    fn test_device_stats() {
        let mut stats = DeviceStats::new();

        // 测试初始状态
        assert_eq!(stats.total_hashes, 0);
        assert_eq!(stats.valid_nonces, 0);
        assert_eq!(stats.hardware_errors, 0);

        // 测试记录有效nonce
        stats.record_valid_nonce();
        assert_eq!(stats.valid_nonces, 1);

        // 测试记录无效nonce
        stats.record_invalid_nonce();
        assert_eq!(stats.invalid_nonces, 1);

        // 测试记录硬件错误
        stats.record_hardware_error();
        assert_eq!(stats.hardware_errors, 1);

        // 测试记录重启
        stats.record_restart();
        assert_eq!(stats.restart_count, 1);

        // 测试错误率计算
        let error_rate = stats.get_error_rate();
        assert!(error_rate > 0.0);
    }

    #[test]
    fn test_work_creation() {
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);

        assert_eq!(work.job_id, "test_job");
        assert_eq!(work.difficulty, 1024.0);
        assert!(!work.is_expired());
    }

    #[test]
    fn test_work_expiration() {
        let target = [0u8; 32];
        let header = [0u8; 80];
        let mut work = Work::new("test_job".to_string(), target, header, 1024.0);

        // 设置过期时间为过去
        work.expires_at = SystemTime::now() - Duration::from_secs(10);
        assert!(work.is_expired());
    }

    #[test]
    fn test_mining_result_creation() {
        let work_id = Uuid::new_v4();
        let result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);

        assert_eq!(result.work_id, work_id);
        assert_eq!(result.device_id, 0);
        assert_eq!(result.nonce, 0x12345678);
        assert_eq!(result.difficulty, 1024.0);
        // 新创建的结果默认是无效的，需要验证后标记为有效
        assert!(!result.is_valid);

        // 标记为有效后应该是有效的
        let valid_result = result.mark_valid();
        assert!(valid_result.is_valid);
    }

    #[test]
    fn test_mining_result_validation() {
        let work_id = Uuid::new_v4();
        let result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);

        // 测试结果创建
        assert_eq!(result.work_id, work_id);
        assert_eq!(result.device_id, 0);
        assert_eq!(result.nonce, 0x12345678);
        assert_eq!(result.difficulty, 1024.0);
        assert!(!result.is_valid); // 默认为false，需要验证后设置

        // 测试标记为有效
        let valid_result = result.mark_valid();
        assert!(valid_result.is_valid);
    }

    #[test]
    fn test_device_config_validation() {
        let config = DeviceConfig {
            chain_id: 0,
            enabled: true,
            frequency: 500,
            voltage: 850,
            auto_tune: true,
            chip_count: 76,
            temperature_limit: 85.0,
            fan_speed: Some(50),
        };

        // 测试配置字段
        assert_eq!(config.chain_id, 0);
        assert!(config.enabled);
        assert_eq!(config.frequency, 500);
        assert_eq!(config.voltage, 850);
        assert!(config.auto_tune);
        assert_eq!(config.chip_count, 76);
        assert_eq!(config.temperature_limit, 85.0);
        assert_eq!(config.fan_speed, Some(50));

        // 测试频率范围
        let high_freq_config = DeviceConfig {
            frequency: 1000,
            ..config.clone()
        };
        assert_eq!(high_freq_config.frequency, 1000);

        // 测试电压范围
        let high_voltage_config = DeviceConfig {
            voltage: 1200,
            ..config.clone()
        };
        assert_eq!(high_voltage_config.voltage, 1200);
    }

    #[test]
    fn test_device_config_default() {
        let config = DeviceConfig::default();

        assert_eq!(config.chain_id, 0);
        assert!(config.enabled);
        assert_eq!(config.frequency, 500);
        assert_eq!(config.voltage, 850);
        assert!(config.auto_tune);
        assert_eq!(config.chip_count, 76);
        assert_eq!(config.temperature_limit, 85.0);
        assert_eq!(config.fan_speed, None);
    }

    #[test]
    fn test_device_status_display() {
        let status = DeviceStatus::Mining;
        assert_eq!(format!("{:?}", status), "Mining");

        let status = DeviceStatus::Error("Test error".to_string());
        assert_eq!(format!("{:?}", status), "Error(\"Test error\")");
    }

    #[test]
    fn test_device_stats_temperature_recording() {
        let mut stats = DeviceStats::new();

        // 初始平均温度应该为None
        assert_eq!(stats.get_average_temperature(), None);

        // 添加温度读数
        stats.record_temperature(65.0);
        stats.record_temperature(70.0);
        stats.record_temperature(68.0);

        // 计算平均温度
        let avg_temp = stats.get_average_temperature().unwrap();
        assert!((avg_temp - 67.67).abs() < 0.1); // 允许小的浮点误差
    }

    #[test]
    fn test_device_stats_hashrate_recording() {
        let mut stats = DeviceStats::new();

        // 初始平均算力应该为None
        assert_eq!(stats.get_average_hashrate(), None);

        // 添加算力读数
        stats.record_hashrate(35.0);
        stats.record_hashrate(38.0);
        stats.record_hashrate(37.0);

        // 计算平均算力
        let avg_hashrate = stats.get_average_hashrate().unwrap();
        assert!((avg_hashrate - 36.67).abs() < 0.1); // 允许小的浮点误差
    }

    // VirtualDevice tests removed - using cgminer-cpu-btc-core instead

    /*
    VirtualDevice tests removed - using cgminer-cpu-btc-core instead
    The real device tests are now handled by the core system
    */


    // 保留有用的辅助函数测试

    #[test]
    fn test_bitcoin_hash_meets_target() {
        // 测试简单的目标检查
        let hash_low = [0x00, 0x00, 0x00, 0x01]; // 很小的哈希
        let hash_high = [0xFF, 0xFF, 0xFF, 0xFF]; // 很大的哈希
        let target = [0x00, 0x00, 0x00, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF]; // 中等目标

        // 使用 cgminer-core 的函数进行测试
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&hash_low);
        let hash1 = hasher.finalize();

        let mut hasher = Sha256::new();
        hasher.update(&hash1);
        let hash2 = hasher.finalize();

        // 验证 cgminer-core 的 meets_target 函数
        assert!(cgminer_core::meets_target(&hash2, &target));
    }

    #[test]
    fn test_device_config_and_info_integration() {
        // 测试设备配置和信息的集成
        let device_info = DeviceInfo::new(
            2000,
            "Test Integration Device".to_string(),
            "cpu-btc".to_string(), // 使用真正的核心类型
            0,
        );

        let config = DeviceConfig {
            chain_id: 0,
            enabled: true,
            frequency: 600,
            voltage: 900,
            auto_tune: false,
            chip_count: 64,
            temperature_limit: 80.0,
            fan_speed: Some(50),
        };

        // 验证配置有效性
        assert!(config.enabled);
        assert_eq!(config.frequency, 600);
        assert_eq!(config.voltage, 900);
        assert_eq!(config.chip_count, 64);

        // 验证设备信息
        assert_eq!(device_info.id, 2000);
        assert_eq!(device_info.device_type, "cpu-btc");
        assert_eq!(device_info.chain_id, 0);
    }
}
