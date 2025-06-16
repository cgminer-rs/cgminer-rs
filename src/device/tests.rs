#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult};
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;

    #[test]
    fn test_device_info_creation() {
        let device_info = DeviceInfo::new(
            0,
            "Test Device".to_string(),
            "test-device".to_string(),
            0,
        );
        
        assert_eq!(device_info.id, 0);
        assert_eq!(device_info.name, "Test Device");
        assert_eq!(device_info.device_type, "test-device");
        assert_eq!(device_info.chain_id, 0);
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
        assert!(result.is_valid());
    }

    #[test]
    fn test_mining_result_validation() {
        let work_id = Uuid::new_v4();
        let mut result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);
        
        // 测试有效结果
        assert!(result.is_valid());
        
        // 测试无效结果（nonce为0通常表示无效）
        result.nonce = 0;
        assert!(!result.is_valid());
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
        
        // 测试有效配置
        assert!(config.is_valid());
        
        // 测试无效频率
        let mut invalid_config = config.clone();
        invalid_config.frequency = 50; // 太低
        assert!(!invalid_config.is_valid());
        
        // 测试无效电压
        invalid_config = config.clone();
        invalid_config.voltage = 2000; // 太高
        assert!(!invalid_config.is_valid());
        
        // 测试无效温度限制
        invalid_config = config.clone();
        invalid_config.temperature_limit = 150.0; // 太高
        assert!(!invalid_config.is_valid());
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
        assert!(config.is_valid());
    }

    #[test]
    fn test_device_status_display() {
        let status = DeviceStatus::Mining;
        assert_eq!(format!("{:?}", status), "Mining");
        
        let status = DeviceStatus::Error("Test error".to_string());
        assert_eq!(format!("{:?}", status), "Error(\"Test error\")");
    }

    #[test]
    fn test_device_stats_uptime() {
        let stats = DeviceStats::new();
        
        // 运行时间应该大于0
        let uptime = stats.get_uptime();
        assert!(uptime.as_secs() >= 0);
    }

    #[test]
    fn test_device_stats_average_temperature() {
        let mut stats = DeviceStats::new();
        
        // 初始平均温度应该为None
        assert_eq!(stats.get_average_temperature(), None);
        
        // 添加温度读数
        stats.add_temperature_reading(65.0);
        stats.add_temperature_reading(70.0);
        stats.add_temperature_reading(68.0);
        
        // 计算平均温度
        let avg_temp = stats.get_average_temperature().unwrap();
        assert!((avg_temp - 67.67).abs() < 0.1); // 允许小的浮点误差
    }

    #[test]
    fn test_device_stats_average_hashrate() {
        let mut stats = DeviceStats::new();
        
        // 初始平均算力应该为None
        assert_eq!(stats.get_average_hashrate(), None);
        
        // 添加算力读数
        stats.add_hashrate_reading(35.0);
        stats.add_hashrate_reading(38.0);
        stats.add_hashrate_reading(37.0);
        
        // 计算平均算力
        let avg_hashrate = stats.get_average_hashrate().unwrap();
        assert!((avg_hashrate - 36.67).abs() < 0.1); // 允许小的浮点误差
    }

    #[test]
    fn test_work_item_creation() {
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);
        let work_item = crate::mining::WorkItem::new(work);
        
        assert_eq!(work_item.priority, 0);
        assert_eq!(work_item.assigned_device, None);
        assert!(!work_item.is_expired());
    }

    #[test]
    fn test_work_item_assignment() {
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);
        let mut work_item = crate::mining::WorkItem::new(work);
        
        // 分配给设备
        work_item.assign_to_device(0);
        assert_eq!(work_item.assigned_device, Some(0));
        
        // 设置优先级
        work_item.set_priority(5);
        assert_eq!(work_item.priority, 5);
    }

    #[test]
    fn test_result_item_creation() {
        let work_id = Uuid::new_v4();
        let result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);
        let work_item = crate::mining::WorkItem::new(work);
        let result_item = crate::mining::ResultItem::new(result, work_item);
        
        assert!(result_item.is_valid());
        assert_eq!(result_item.result.device_id, 0);
    }

    #[test]
    fn test_result_item_validation() {
        let work_id = Uuid::new_v4();
        let mut result = MiningResult::new(work_id, 0, 0x12345678, 1024.0);
        let target = [0u8; 32];
        let header = [0u8; 80];
        let work = Work::new("test_job".to_string(), target, header, 1024.0);
        let work_item = crate::mining::WorkItem::new(work);
        
        // 测试有效结果
        let result_item = crate::mining::ResultItem::new(result.clone(), work_item.clone());
        assert!(result_item.is_valid());
        
        // 测试无效结果
        result.nonce = 0;
        let invalid_result_item = crate::mining::ResultItem::new(result, work_item);
        assert!(!invalid_result_item.is_valid());
    }
}
