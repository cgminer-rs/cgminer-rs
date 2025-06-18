#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult, DeviceConfig};
    use crate::device::virtual_device::{VirtualDeviceDriver, VirtualDevice};
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

    // 虚拟设备测试
    #[tokio::test]
    async fn test_virtual_device_driver_creation() {
        let driver = VirtualDeviceDriver::new();

        assert_eq!(driver.driver_name(), "virtual-device");
        assert_eq!(driver.supported_devices(), vec!["virtual"]);
        assert_eq!(driver.version(), "1.0.0");
    }

    #[tokio::test]
    async fn test_virtual_device_driver_scan() {
        let driver = VirtualDeviceDriver::new();
        let devices = driver.scan_devices().await.unwrap();

        assert_eq!(devices.len(), 4); // 应该扫描到4个虚拟设备

        for (i, device) in devices.iter().enumerate() {
            assert_eq!(device.id, 1000 + i as u32);
            assert_eq!(device.name, format!("Virtual Device {}", i));
            assert_eq!(device.device_type, "virtual");
            assert_eq!(device.chain_id, i as u8);
        }
    }

    #[tokio::test]
    async fn test_virtual_device_creation() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let device = VirtualDevice::new(device_info).await.unwrap();
        assert_eq!(device.device_id(), 1000);
    }

    #[tokio::test]
    async fn test_virtual_device_initialization() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig {
            chain_id: 0,
            enabled: true,
            frequency: 700,
            voltage: 950,
            auto_tune: false,
            chip_count: 64,
            temperature_limit: 80.0,
            fan_speed: Some(60),
        };

        let result = device.initialize(config.clone()).await;
        assert!(result.is_ok());

        let info = device.get_info().await.unwrap();
        assert_eq!(info.status, DeviceStatus::Idle);
        assert_eq!(info.frequency, Some(700));
        assert_eq!(info.voltage, Some(950));
        assert_eq!(info.chip_count, 64);
        assert_eq!(info.fan_speed, Some(60));
    }

    #[tokio::test]
    async fn test_virtual_device_start_stop() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 测试启动
        let result = device.start().await;
        assert!(result.is_ok());

        let status = device.get_status().await.unwrap();
        assert_eq!(status, DeviceStatus::Mining);

        // 等待一小段时间让挖矿模拟运行
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 测试停止
        let result = device.stop().await;
        assert!(result.is_ok());

        let status = device.get_status().await.unwrap();
        assert_eq!(status, DeviceStatus::Idle);
    }

    #[tokio::test]
    async fn test_virtual_device_work_submission() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 创建工作
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);

        // 提交工作
        let result = device.submit_work(work).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_virtual_device_expired_work() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 创建过期的工作
        let target = [0u8; 32];
        let header = [0u8; 80];
        let mut work = Work::new("test_job".to_string(), target, header, 1024.0);
        work.expires_at = SystemTime::now() - Duration::from_secs(10);

        // 提交过期工作应该失败
        let result = device.submit_work(work).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_virtual_device_frequency_setting() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 设置频率
        let result = device.set_frequency(800).await;
        assert!(result.is_ok());

        let info = device.get_info().await.unwrap();
        assert_eq!(info.frequency, Some(800));
    }

    #[tokio::test]
    async fn test_virtual_device_voltage_setting() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 设置电压
        let result = device.set_voltage(1000).await;
        assert!(result.is_ok());

        let info = device.get_info().await.unwrap();
        assert_eq!(info.voltage, Some(1000));
    }

    #[tokio::test]
    async fn test_virtual_device_fan_speed_setting() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 设置风扇速度
        let result = device.set_fan_speed(80).await;
        assert!(result.is_ok());

        let info = device.get_info().await.unwrap();
        assert_eq!(info.fan_speed, Some(80));
    }

    #[tokio::test]
    async fn test_virtual_device_health_check() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 健康检查
        let is_healthy = device.health_check().await.unwrap();
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_virtual_device_stats_reset() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 重置统计信息
        let result = device.reset_stats().await;
        assert!(result.is_ok());

        let info = device.get_info().await.unwrap();
        assert_eq!(info.accepted_shares, 0);
        assert_eq!(info.rejected_shares, 0);
        assert_eq!(info.hardware_errors, 0);
    }

    #[tokio::test]
    async fn test_virtual_device_bitcoin_mining() {
        let device_info = DeviceInfo::new(
            1000,
            "Test Bitcoin Virtual Device".to_string(),
            "virtual".to_string(),
            0,
        );

        let mut device = VirtualDevice::new(device_info).await.unwrap();
        let config = DeviceConfig::default();
        device.initialize(config).await.unwrap();

        // 创建一个简单的工作，使用低难度以便快速找到解
        let target = [0u8; 32];
        let mut header = [0u8; 80];

        // 设置一个简单的 header
        header[0] = 0x01; // 版本
        header[4] = 0x00; // 前一个区块哈希
        header[36] = 0x00; // Merkle root
        header[68] = 0x00; // 时间戳
        header[72] = 0x00; // 难度目标

        let work = Work::new("test_bitcoin_job".to_string(), target, header, 1.0); // 最低难度

        // 提交工作
        let result = device.submit_work(work).await;
        assert!(result.is_ok());

        // 启动设备进行短时间挖矿
        device.start().await.unwrap();

        // 等待一段时间让挖矿算法运行
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 停止设备
        device.stop().await.unwrap();

        // 检查是否有统计更新
        let stats = device.get_stats().await.unwrap();
        assert!(stats.total_hashes > 0); // 应该处理了一些哈希
    }

    #[test]
    fn test_bitcoin_hash_meets_target() {
        // 测试哈希目标检查函数
        let hash_low = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ];

        let hash_high = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let target = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
        ];

        // 低哈希应该满足目标（0x01 < 0x10）
        assert!(VirtualDevice::hash_meets_target(&hash_low, &target));

        // 高哈希不应该满足目标
        assert!(!VirtualDevice::hash_meets_target(&hash_high, &target));
    }

    #[test]
    fn test_difficulty_to_target() {
        // 测试难度到目标的转换
        let target_1 = VirtualDevice::difficulty_to_target(1.0);
        let target_high = VirtualDevice::difficulty_to_target(1000.0);

        // 难度1应该给出最高目标值（最容易）
        assert_eq!(target_1[0], 0xFF);

        // 高难度应该给出更低的目标值（更难）
        assert!(target_high[31] < target_1[31] || target_high[30] < target_1[30]);
    }

    #[test]
    fn test_bitcoin_mining_algorithm() {
        // 测试 Bitcoin 挖矿算法
        let mut header = [0u8; 80];

        // 设置一个简单的 header
        header[0] = 0x01; // 版本

        // 使用非常低的难度目标（很容易满足）
        let target = [0xFFu8; 32];

        // 尝试挖矿
        let result = VirtualDevice::mine_bitcoin_block(&header, &target, 0, 1000);

        // 应该能找到一个有效的 nonce
        assert!(result.is_some());

        if let Some(nonce) = result {
            // 验证找到的 nonce 确实有效
            let mut test_header = header;
            test_header[76..80].copy_from_slice(&nonce.to_le_bytes());

            // 计算哈希
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&test_header);
            let hash1 = hasher.finalize();

            let mut hasher = Sha256::new();
            hasher.update(&hash1);
            let hash2 = hasher.finalize();

            // 验证哈希满足目标
            assert!(VirtualDevice::hash_meets_target(&hash2, &target));
        }
    }
}
