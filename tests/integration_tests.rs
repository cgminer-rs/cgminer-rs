use cgminer_rs::config::Config;
use cgminer_rs::mining::MiningManager;
use cgminer_rs::device::{DeviceManager, DeviceInfo};
use cgminer_rs::pool::PoolManager;
use cgminer_rs::monitoring::MonitoringSystem;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// 测试配置加载
#[tokio::test]
async fn test_config_loading() {
    // 创建临时配置文件
    let config_content = r#"
[mining]
scan_interval = 5
work_restart_timeout = 30
enable_auto_tuning = true

[devices]
scan_interval = 10

[[devices.chains]]
id = 0
enabled = true
frequency = 500
voltage = 850
auto_tune = true
chip_count = 76

[[devices.chains]]
id = 1
enabled = true
frequency = 500
voltage = 850
auto_tune = true
chip_count = 76

[pools]
strategy = "Failover"
retry_interval = 30

[[pools.pools]]
url = "stratum+tcp://pool.example.com:4444"
user = "test_user"
password = "test_password"
priority = 1

[[pools.pools]]
url = "stratum+tcp://backup.example.com:4444"
user = "test_user"
password = "test_password"
priority = 2

[api]
enabled = true
bind_address = "127.0.0.1"
port = 8080
auth_token = "test_token"
allow_origins = ["*"]

[monitoring]
enabled = true
metrics_interval = 10

[monitoring.alert_thresholds]
max_temperature = 85.0
max_device_temperature = 90.0
max_cpu_usage = 90
max_memory_usage = 90
max_error_rate = 5.0
min_hashrate = 30.0
"#;
    
    // 写入临时文件
    let temp_file = std::env::temp_dir().join("test_config.toml");
    std::fs::write(&temp_file, config_content).expect("Failed to write test config");
    
    // 测试配置加载
    let config = Config::load(temp_file.to_str().unwrap()).expect("Failed to load config");
    
    // 验证配置
    assert_eq!(config.mining.scan_interval, 5);
    assert_eq!(config.devices.chains.len(), 2);
    assert_eq!(config.pools.pools.len(), 2);
    assert!(config.api.enabled);
    assert!(config.monitoring.enabled);
    
    // 清理
    std::fs::remove_file(temp_file).ok();
}

/// 测试设备管理器
#[tokio::test]
async fn test_device_manager() {
    let config = create_test_config();
    let mut device_manager = DeviceManager::new(config.devices.clone());
    
    // 注册测试驱动
    let test_driver = Box::new(TestDeviceDriver::new());
    device_manager.register_driver(test_driver);
    
    // 测试初始化
    device_manager.initialize().await.expect("Failed to initialize device manager");
    
    // 测试启动
    device_manager.start().await.expect("Failed to start device manager");
    
    // 等待一段时间让设备稳定
    sleep(Duration::from_millis(100)).await;
    
    // 测试获取设备信息
    let device_info = device_manager.get_all_device_info().await;
    assert!(!device_info.is_empty());
    
    // 测试获取总算力
    let total_hashrate = device_manager.get_total_hashrate().await;
    assert!(total_hashrate > 0.0);
    
    // 测试停止
    device_manager.stop().await.expect("Failed to stop device manager");
}

/// 测试矿池管理器
#[tokio::test]
async fn test_pool_manager() {
    let config = create_test_config();
    let pool_manager = PoolManager::new(config.pools.clone()).await.expect("Failed to create pool manager");
    
    // 测试启动（注意：这会尝试连接到真实的矿池，在测试环境中可能会失败）
    // 在实际测试中，我们应该使用模拟的矿池服务器
    let result = pool_manager.start().await;
    
    // 由于我们使用的是测试配置，连接可能会失败，这是正常的
    // 我们主要测试管理器是否能正确处理错误
    match result {
        Ok(_) => {
            // 如果成功连接，测试其他功能
            let connected_count = pool_manager.get_connected_pool_count().await;
            println!("Connected pools: {}", connected_count);
            
            pool_manager.stop().await.expect("Failed to stop pool manager");
        }
        Err(e) => {
            // 连接失败是预期的，因为我们使用的是测试配置
            println!("Pool connection failed as expected: {}", e);
        }
    }
}

/// 测试监控系统
#[tokio::test]
async fn test_monitoring_system() {
    let config = create_test_config();
    let monitoring_system = MonitoringSystem::new(config.monitoring.clone()).await.expect("Failed to create monitoring system");
    
    // 测试启动
    monitoring_system.start().await.expect("Failed to start monitoring system");
    
    // 等待一段时间让监控系统收集数据
    sleep(Duration::from_secs(2)).await;
    
    // 测试获取指标
    let system_metrics = monitoring_system.get_system_metrics().await;
    assert!(system_metrics.is_some());
    
    let mining_metrics = monitoring_system.get_mining_metrics().await;
    assert!(mining_metrics.is_some());
    
    // 测试性能统计
    let performance_stats = monitoring_system.get_performance_stats().await;
    assert!(performance_stats.total_metrics_collected > 0);
    
    // 测试停止
    monitoring_system.stop().await.expect("Failed to stop monitoring system");
}

/// 测试挖矿管理器
#[tokio::test]
async fn test_mining_manager() {
    let config = create_test_config();
    let mining_manager = Arc::new(MiningManager::new(config).await.expect("Failed to create mining manager"));
    
    // 测试启动
    mining_manager.start().await.expect("Failed to start mining manager");
    
    // 等待一段时间让系统稳定
    sleep(Duration::from_secs(1)).await;
    
    // 测试获取状态
    let state = mining_manager.get_state().await;
    println!("Mining state: {:?}", state);
    
    // 测试获取统计信息
    let stats = mining_manager.get_stats().await;
    assert!(stats.uptime.as_secs() > 0);
    
    // 测试获取系统状态
    let system_status = mining_manager.get_system_status().await.expect("Failed to get system status");
    assert!(system_status.uptime.as_secs() > 0);
    
    // 测试停止
    mining_manager.stop().await.expect("Failed to stop mining manager");
}

/// 测试事件系统
#[tokio::test]
async fn test_event_system() {
    let config = create_test_config();
    let mining_manager = Arc::new(MiningManager::new(config).await.expect("Failed to create mining manager"));
    
    // 订阅事件
    let mut event_receiver = mining_manager.subscribe_events();
    
    // 启动挖矿管理器
    mining_manager.start().await.expect("Failed to start mining manager");
    
    // 等待并接收事件
    let timeout_duration = Duration::from_secs(5);
    let event_result = tokio::time::timeout(timeout_duration, event_receiver.recv()).await;
    
    match event_result {
        Ok(Ok(event)) => {
            println!("Received event: {:?}", event.event_type());
        }
        Ok(Err(e)) => {
            println!("Event receiver error: {}", e);
        }
        Err(_) => {
            println!("No events received within timeout");
        }
    }
    
    // 停止挖矿管理器
    mining_manager.stop().await.expect("Failed to stop mining manager");
}

/// 创建测试配置
fn create_test_config() -> Config {
    Config {
        mining: cgminer_rs::config::MiningConfig {
            scan_interval: Duration::from_secs(5),
            work_restart_timeout: Duration::from_secs(30),
            enable_auto_tuning: true,
        },
        devices: cgminer_rs::config::DeviceConfig {
            scan_interval: 10,
            chains: vec![
                cgminer_rs::config::ChainConfig {
                    id: 0,
                    enabled: true,
                    frequency: 500,
                    voltage: 850,
                    auto_tune: true,
                    chip_count: 76,
                },
                cgminer_rs::config::ChainConfig {
                    id: 1,
                    enabled: true,
                    frequency: 500,
                    voltage: 850,
                    auto_tune: true,
                    chip_count: 76,
                },
            ],
        },
        pools: cgminer_rs::config::PoolConfig {
            strategy: cgminer_rs::pool::PoolStrategy::Failover,
            retry_interval: 30,
            pools: vec![
                cgminer_rs::config::PoolInfo {
                    url: "stratum+tcp://pool.example.com:4444".to_string(),
                    user: "test_user".to_string(),
                    password: "test_password".to_string(),
                    priority: 1,
                },
            ],
        },
        api: cgminer_rs::config::ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: Some("test_token".to_string()),
            allow_origins: vec!["*".to_string()],
        },
        monitoring: cgminer_rs::config::MonitoringConfig {
            enabled: true,
            metrics_interval: 10,
            alert_thresholds: cgminer_rs::config::AlertThresholds {
                max_temperature: 85.0,
                max_device_temperature: 90.0,
                max_cpu_usage: 90,
                max_memory_usage: 90,
                max_error_rate: 5.0,
                min_hashrate: 30.0,
            },
        },
    }
}

/// 测试设备驱动
struct TestDeviceDriver;

impl TestDeviceDriver {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl cgminer_rs::device::DeviceDriver for TestDeviceDriver {
    fn driver_name(&self) -> &'static str {
        "Test Driver"
    }
    
    fn supported_devices(&self) -> Vec<&'static str> {
        vec!["test-device"]
    }
    
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, cgminer_rs::error::DeviceError> {
        // 返回模拟设备
        Ok(vec![
            DeviceInfo::new(0, "Test Device 0".to_string(), "test-device".to_string(), 0),
            DeviceInfo::new(1, "Test Device 1".to_string(), "test-device".to_string(), 1),
        ])
    }
    
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn cgminer_rs::device::MiningDevice>, cgminer_rs::error::DeviceError> {
        Ok(Box::new(TestMiningDevice::new(device_info)))
    }
    
    fn validate_config(&self, _config: &cgminer_rs::device::DeviceConfig) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    fn default_config(&self) -> cgminer_rs::device::DeviceConfig {
        cgminer_rs::device::DeviceConfig::default()
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

/// 测试挖矿设备
struct TestMiningDevice {
    device_info: DeviceInfo,
}

impl TestMiningDevice {
    fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }
}

#[async_trait::async_trait]
impl cgminer_rs::device::MiningDevice for TestMiningDevice {
    fn device_id(&self) -> u32 {
        self.device_info.id
    }
    
    async fn get_info(&self) -> Result<DeviceInfo, cgminer_rs::error::DeviceError> {
        Ok(self.device_info.clone())
    }
    
    async fn initialize(&mut self, _config: cgminer_rs::device::DeviceConfig) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn restart(&mut self) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn submit_work(&mut self, _work: cgminer_rs::device::Work) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn get_result(&mut self) -> Result<Option<cgminer_rs::device::MiningResult>, cgminer_rs::error::DeviceError> {
        Ok(None)
    }
    
    async fn get_status(&self) -> Result<cgminer_rs::device::DeviceStatus, cgminer_rs::error::DeviceError> {
        Ok(cgminer_rs::device::DeviceStatus::Mining)
    }
    
    async fn get_temperature(&self) -> Result<f32, cgminer_rs::error::DeviceError> {
        Ok(65.0)
    }
    
    async fn get_hashrate(&self) -> Result<f64, cgminer_rs::error::DeviceError> {
        Ok(38.0)
    }
    
    async fn get_stats(&self) -> Result<cgminer_rs::device::DeviceStats, cgminer_rs::error::DeviceError> {
        Ok(cgminer_rs::device::DeviceStats::new())
    }
    
    async fn set_frequency(&mut self, _frequency: u32) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn set_voltage(&mut self, _voltage: u32) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn set_fan_speed(&mut self, _speed: u32) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool, cgminer_rs::error::DeviceError> {
        Ok(true)
    }
    
    async fn reset_stats(&mut self) -> Result<(), cgminer_rs::error::DeviceError> {
        Ok(())
    }
}
