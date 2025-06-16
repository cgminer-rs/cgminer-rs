use async_trait::async_trait;
use crate::error::DeviceError;
use super::{DeviceInfo, DeviceConfig, DeviceStats, Work, MiningResult};
use std::time::Duration;

/// 挖矿设备特征
#[async_trait]
pub trait MiningDevice: Send + Sync {
    /// 获取设备ID
    fn device_id(&self) -> u32;
    
    /// 获取设备信息
    async fn get_info(&self) -> Result<DeviceInfo, DeviceError>;
    
    /// 初始化设备
    async fn initialize(&mut self, config: DeviceConfig) -> Result<(), DeviceError>;
    
    /// 启动设备
    async fn start(&mut self) -> Result<(), DeviceError>;
    
    /// 停止设备
    async fn stop(&mut self) -> Result<(), DeviceError>;
    
    /// 重启设备
    async fn restart(&mut self) -> Result<(), DeviceError>;
    
    /// 提交工作
    async fn submit_work(&mut self, work: Work) -> Result<(), DeviceError>;
    
    /// 获取挖矿结果
    async fn get_result(&mut self) -> Result<Option<MiningResult>, DeviceError>;
    
    /// 获取设备状态
    async fn get_status(&self) -> Result<super::DeviceStatus, DeviceError>;
    
    /// 获取温度
    async fn get_temperature(&self) -> Result<f32, DeviceError>;
    
    /// 获取算力
    async fn get_hashrate(&self) -> Result<f64, DeviceError>;
    
    /// 获取统计信息
    async fn get_stats(&self) -> Result<DeviceStats, DeviceError>;
    
    /// 设置频率
    async fn set_frequency(&mut self, frequency: u32) -> Result<(), DeviceError>;
    
    /// 设置电压
    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError>;
    
    /// 设置风扇速度
    async fn set_fan_speed(&mut self, speed: u32) -> Result<(), DeviceError>;
    
    /// 检查设备健康状态
    async fn health_check(&self) -> Result<bool, DeviceError>;
    
    /// 重置统计信息
    async fn reset_stats(&mut self) -> Result<(), DeviceError>;
}

/// 设备驱动特征
#[async_trait]
pub trait DeviceDriver: Send + Sync {
    /// 驱动名称
    fn driver_name(&self) -> &'static str;
    
    /// 支持的设备类型
    fn supported_devices(&self) -> Vec<&'static str>;
    
    /// 扫描设备
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, DeviceError>;
    
    /// 创建设备实例
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, DeviceError>;
    
    /// 验证设备配置
    fn validate_config(&self, config: &DeviceConfig) -> Result<(), DeviceError>;
    
    /// 获取默认配置
    fn default_config(&self) -> DeviceConfig;
    
    /// 获取驱动版本
    fn version(&self) -> &'static str;
}

/// 链控制器特征
#[async_trait]
pub trait ChainController: Send + Sync {
    /// 获取链ID
    fn chain_id(&self) -> u8;
    
    /// 初始化链
    async fn initialize(&mut self) -> Result<(), DeviceError>;
    
    /// 检测芯片数量
    async fn detect_chips(&self) -> Result<u32, DeviceError>;
    
    /// 设置PLL频率
    async fn set_pll_frequency(&mut self, frequency: u32) -> Result<(), DeviceError>;
    
    /// 设置电压
    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError>;
    
    /// 发送作业到链
    async fn send_job(&mut self, job_data: &[u8]) -> Result<(), DeviceError>;
    
    /// 从链读取结果
    async fn read_result(&mut self) -> Result<Option<(u32, u8)>, DeviceError>; // (nonce, work_id)
    
    /// 获取链状态
    async fn get_status(&self) -> Result<ChainStatus, DeviceError>;
    
    /// 获取链温度
    async fn get_temperature(&self) -> Result<f32, DeviceError>;
    
    /// 重置链
    async fn reset(&mut self) -> Result<(), DeviceError>;
    
    /// 启用/禁用链
    async fn set_enabled(&mut self, enabled: bool) -> Result<(), DeviceError>;
}

/// 链状态
#[derive(Debug, Clone, PartialEq)]
pub enum ChainStatus {
    /// 未初始化
    Uninitialized,
    /// 正在初始化
    Initializing,
    /// 空闲
    Idle,
    /// 工作中
    Working,
    /// 错误
    Error(String),
    /// 禁用
    Disabled,
}

/// 硬件接口特征
#[async_trait]
pub trait HardwareInterface: Send + Sync {
    /// SPI 读写
    async fn spi_transfer(&self, chain_id: u8, data: &[u8]) -> Result<Vec<u8>, DeviceError>;
    
    /// UART 读写
    async fn uart_write(&self, chain_id: u8, data: &[u8]) -> Result<(), DeviceError>;
    async fn uart_read(&self, chain_id: u8, len: usize) -> Result<Vec<u8>, DeviceError>;
    
    /// GPIO 控制
    async fn gpio_set(&self, pin: u32, value: bool) -> Result<(), DeviceError>;
    async fn gpio_get(&self, pin: u32) -> Result<bool, DeviceError>;
    
    /// PWM 控制
    async fn pwm_set_duty(&self, channel: u32, duty: f32) -> Result<(), DeviceError>;
    
    /// 温度读取
    async fn read_temperature(&self, sensor_id: u8) -> Result<f32, DeviceError>;
    
    /// 电压设置
    async fn set_voltage(&self, chain_id: u8, voltage: u32) -> Result<(), DeviceError>;
    
    /// 频率设置
    async fn set_frequency(&self, chain_id: u8, frequency: u32) -> Result<(), DeviceError>;
}

/// 自动调优特征
#[async_trait]
pub trait AutoTuning: Send + Sync {
    /// 开始自动调优
    async fn start_tuning(&mut self, device_id: u32) -> Result<(), DeviceError>;
    
    /// 停止自动调优
    async fn stop_tuning(&mut self, device_id: u32) -> Result<(), DeviceError>;
    
    /// 获取调优状态
    async fn get_tuning_status(&self, device_id: u32) -> Result<TuningStatus, DeviceError>;
    
    /// 获取最优参数
    async fn get_optimal_params(&self, device_id: u32) -> Result<OptimalParams, DeviceError>;
    
    /// 应用调优结果
    async fn apply_tuning_result(&mut self, device_id: u32, params: OptimalParams) -> Result<(), DeviceError>;
}

/// 调优状态
#[derive(Debug, Clone, PartialEq)]
pub enum TuningStatus {
    /// 未开始
    NotStarted,
    /// 进行中
    InProgress { progress: f32 },
    /// 已完成
    Completed,
    /// 失败
    Failed(String),
}

/// 最优参数
#[derive(Debug, Clone)]
pub struct OptimalParams {
    pub frequency: u32,
    pub voltage: u32,
    pub expected_hashrate: f64,
    pub power_consumption: f64,
    pub efficiency: f64, // MH/J
}

/// 监控特征
#[async_trait]
pub trait DeviceMonitor: Send + Sync {
    /// 开始监控
    async fn start_monitoring(&mut self, interval: Duration) -> Result<(), DeviceError>;
    
    /// 停止监控
    async fn stop_monitoring(&mut self) -> Result<(), DeviceError>;
    
    /// 获取实时指标
    async fn get_metrics(&self, device_id: u32) -> Result<DeviceMetrics, DeviceError>;
    
    /// 设置告警阈值
    async fn set_alert_thresholds(&mut self, thresholds: AlertThresholds) -> Result<(), DeviceError>;
    
    /// 检查告警
    async fn check_alerts(&self, device_id: u32) -> Result<Vec<Alert>, DeviceError>;
}

/// 设备指标
#[derive(Debug, Clone)]
pub struct DeviceMetrics {
    pub device_id: u32,
    pub timestamp: std::time::SystemTime,
    pub temperature: f32,
    pub hashrate: f64,
    pub power_consumption: f64,
    pub fan_speed: u32,
    pub voltage: u32,
    pub frequency: u32,
    pub error_rate: f64,
    pub uptime: Duration,
}

/// 告警阈值
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub temperature_warning: f32,
    pub temperature_critical: f32,
    pub hashrate_drop_percent: f32,
    pub error_rate_percent: f32,
    pub power_limit_watts: f64,
}

/// 告警信息
#[derive(Debug, Clone)]
pub struct Alert {
    pub device_id: u32,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: std::time::SystemTime,
}

/// 告警类型
#[derive(Debug, Clone, PartialEq)]
pub enum AlertType {
    Temperature,
    Hashrate,
    ErrorRate,
    PowerConsumption,
    DeviceOffline,
    HardwareError,
}

/// 告警严重程度
#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}
