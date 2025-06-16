use crate::error::DeviceError;
use crate::device::traits::{ChainController, ChainStatus, HardwareInterface};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug};

/// ASIC 链控制器实现
pub struct AsicChainController {
    /// 链ID
    chain_id: u8,
    /// 硬件接口
    hardware: Arc<dyn HardwareInterface>,
    /// 链状态
    status: Arc<RwLock<ChainStatus>>,
    /// 芯片数量
    chip_count: Arc<RwLock<u32>>,
    /// PLL 频率
    pll_frequency: Arc<RwLock<u32>>,
    /// 电压设置
    voltage: Arc<RwLock<u32>>,
    /// 是否启用
    enabled: Arc<RwLock<bool>>,
    /// 最后活动时间
    last_activity: Arc<RwLock<SystemTime>>,
    /// 工作队列
    work_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    /// 结果队列
    result_queue: Arc<Mutex<Vec<(u32, u8)>>>,
    /// 错误计数
    error_count: Arc<RwLock<u32>>,
    /// 重置计数
    reset_count: Arc<RwLock<u32>>,
}

impl AsicChainController {
    /// 创建新的链控制器
    pub fn new(chain_id: u8, hardware: Arc<dyn HardwareInterface>) -> Self {
        Self {
            chain_id,
            hardware,
            status: Arc::new(RwLock::new(ChainStatus::Uninitialized)),
            chip_count: Arc::new(RwLock::new(0)),
            pll_frequency: Arc::new(RwLock::new(500)), // 默认 500MHz
            voltage: Arc::new(RwLock::new(850)), // 默认 850mV
            enabled: Arc::new(RwLock::new(true)),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            work_queue: Arc::new(Mutex::new(Vec::new())),
            result_queue: Arc::new(Mutex::new(Vec::new())),
            error_count: Arc::new(RwLock::new(0)),
            reset_count: Arc::new(RwLock::new(0)),
        }
    }

    /// 发送命令到链
    async fn send_command(&self, command: &[u8]) -> Result<Vec<u8>, DeviceError> {
        debug!("Sending command to chain {}: {:02x?}", self.chain_id, command);

        match timeout(Duration::from_secs(5), self.hardware.spi_transfer(self.chain_id, command)).await {
            Ok(result) => {
                match result {
                    Ok(response) => {
                        debug!("Received response from chain {}: {:02x?}", self.chain_id, response);
                        *self.last_activity.write().await = SystemTime::now();
                        Ok(response)
                    }
                    Err(e) => {
                        *self.error_count.write().await += 1;
                        Err(e)
                    }
                }
            }
            Err(_) => {
                *self.error_count.write().await += 1;
                Err(DeviceError::Timeout { device_id: self.chain_id as u32 })
            }
        }
    }

    /// 等待响应
    async fn wait_for_response(&self, timeout_ms: u64) -> Result<Vec<u8>, DeviceError> {
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout_duration {
            // 尝试读取响应
            match self.hardware.spi_transfer(self.chain_id, &[]).await {
                Ok(response) => {
                    if !response.is_empty() {
                        return Ok(response);
                    }
                }
                Err(_) => {
                    // 继续等待
                }
            }

            sleep(Duration::from_millis(1)).await;
        }

        Err(DeviceError::Timeout { device_id: self.chain_id as u32 })
    }

    /// 检测单个芯片
    async fn detect_chip(&self, chip_id: u8) -> Result<bool, DeviceError> {
        // 构造芯片检测命令
        let command = vec![
            0x55, 0xAA, // 同步头
            0x51, // 检测命令
            chip_id, // 芯片ID
            0x00, 0x00, 0x00, 0x00, // 数据
        ];

        match self.send_command(&command).await {
            Ok(response) => {
                // 检查响应是否有效
                if response.len() >= 4 && response[0] == 0x55 && response[1] == 0xAA {
                    debug!("Chip {} detected on chain {}", chip_id, self.chain_id);
                    Ok(true)
                } else {
                    debug!("Chip {} not responding on chain {}", chip_id, self.chain_id);
                    Ok(false)
                }
            }
            Err(e) => {
                debug!("Failed to detect chip {} on chain {}: {}", chip_id, self.chain_id, e);
                Ok(false)
            }
        }
    }

    /// 配置芯片
    async fn configure_chip(&self, chip_id: u8) -> Result<(), DeviceError> {
        debug!("Configuring chip {} on chain {}", chip_id, self.chain_id);

        let frequency = *self.pll_frequency.read().await;
        let voltage = *self.voltage.read().await;

        // 设置PLL频率
        let pll_command = vec![
            0x55, 0xAA, // 同步头
            0x52, // PLL设置命令
            chip_id, // 芯片ID
            (frequency >> 8) as u8, // 频率高字节
            frequency as u8, // 频率低字节
            0x00, 0x00, // 保留
        ];

        self.send_command(&pll_command).await?;
        sleep(Duration::from_millis(10)).await;

        // 设置电压（如果支持）
        let voltage_command = vec![
            0x55, 0xAA, // 同步头
            0x53, // 电压设置命令
            chip_id, // 芯片ID
            (voltage >> 8) as u8, // 电压高字节
            voltage as u8, // 电压低字节
            0x00, 0x00, // 保留
        ];

        self.send_command(&voltage_command).await?;
        sleep(Duration::from_millis(10)).await;

        info!("Chip {} configured: frequency={}MHz, voltage={}mV", chip_id, frequency, voltage);
        Ok(())
    }

    /// 发送工作到芯片
    async fn send_work_to_chip(&self, chip_id: u8, work_data: &[u8]) -> Result<(), DeviceError> {
        if work_data.len() != 32 {
            return Err(DeviceError::InvalidConfig {
                reason: "Work data must be 32 bytes".to_string(),
            });
        }

        let mut command = vec![
            0x55, 0xAA, // 同步头
            0x54, // 工作命令
            chip_id, // 芯片ID
        ];
        command.extend_from_slice(work_data);

        self.send_command(&command).await?;
        debug!("Work sent to chip {} on chain {}", chip_id, self.chain_id);
        Ok(())
    }

    /// 从芯片读取结果
    async fn read_result_from_chip(&self, chip_id: u8) -> Result<Option<(u32, u8)>, DeviceError> {
        let command = vec![
            0x55, 0xAA, // 同步头
            0x55, // 读取结果命令
            chip_id, // 芯片ID
            0x00, 0x00, 0x00, 0x00, // 保留
        ];

        match self.send_command(&command).await {
            Ok(response) => {
                if response.len() >= 8 && response[0] == 0x55 && response[1] == 0xAA {
                    // 解析nonce和工作ID
                    let nonce = u32::from_be_bytes([response[4], response[5], response[6], response[7]]);
                    let work_id = response[3];

                    if nonce != 0 {
                        debug!("Result from chip {} on chain {}: nonce={:08x}, work_id={}",
                               chip_id, self.chain_id, nonce, work_id);
                        return Ok(Some((nonce, work_id)));
                    }
                }
                Ok(None)
            }
            Err(e) => {
                debug!("Failed to read result from chip {} on chain {}: {}", chip_id, self.chain_id, e);
                Ok(None)
            }
        }
    }

    /// 重置芯片
    async fn reset_chip(&self, chip_id: u8) -> Result<(), DeviceError> {
        debug!("Resetting chip {} on chain {}", chip_id, self.chain_id);

        let command = vec![
            0x55, 0xAA, // 同步头
            0x56, // 重置命令
            chip_id, // 芯片ID
            0x00, 0x00, 0x00, 0x00, // 保留
        ];

        self.send_command(&command).await?;
        sleep(Duration::from_millis(100)).await; // 等待重置完成

        info!("Chip {} reset on chain {}", chip_id, self.chain_id);
        Ok(())
    }

    /// 获取芯片温度
    async fn get_chip_temperature(&self, chip_id: u8) -> Result<f32, DeviceError> {
        let command = vec![
            0x55, 0xAA, // 同步头
            0x57, // 温度读取命令
            chip_id, // 芯片ID
            0x00, 0x00, 0x00, 0x00, // 保留
        ];

        match self.send_command(&command).await {
            Ok(response) => {
                if response.len() >= 6 && response[0] == 0x55 && response[1] == 0xAA {
                    // 解析温度数据（假设为16位整数，单位0.1°C）
                    let temp_raw = u16::from_be_bytes([response[4], response[5]]);
                    let temperature = temp_raw as f32 / 10.0;

                    debug!("Temperature from chip {} on chain {}: {:.1}°C", chip_id, self.chain_id, temperature);
                    Ok(temperature)
                } else {
                    Err(DeviceError::CommunicationError {
                        device_id: self.chain_id as u32,
                        error: "Invalid temperature response".to_string(),
                    })
                }
            }
            Err(e) => {
                debug!("Failed to read temperature from chip {} on chain {}: {}", chip_id, self.chain_id, e);
                Err(e)
            }
        }
    }

    /// 检查链是否健康
    pub async fn is_healthy(&self) -> bool {
        let status = self.status.read().await;
        matches!(*status, ChainStatus::Idle | ChainStatus::Working)
    }

    /// 获取错误计数
    pub async fn get_error_count(&self) -> u32 {
        *self.error_count.read().await
    }

    /// 获取重置计数
    pub async fn get_reset_count(&self) -> u32 {
        *self.reset_count.read().await
    }

    /// 清除错误计数
    pub async fn clear_error_count(&self) {
        *self.error_count.write().await = 0;
    }

    /// 获取最后活动时间
    pub async fn get_last_activity(&self) -> SystemTime {
        *self.last_activity.read().await
    }
}

#[async_trait]
impl ChainController for AsicChainController {
    /// 获取链ID
    fn chain_id(&self) -> u8 {
        self.chain_id
    }

    /// 初始化链
    async fn initialize(&mut self) -> Result<(), DeviceError> {
        info!("Initializing chain {}", self.chain_id);

        *self.status.write().await = ChainStatus::Initializing;

        // 重置链
        self.reset().await?;

        // 检测芯片
        let chip_count = self.detect_chips().await?;
        *self.chip_count.write().await = chip_count;

        if chip_count == 0 {
            *self.status.write().await = ChainStatus::Error("No chips detected".to_string());
            return Err(DeviceError::ChainError {
                chain_id: self.chain_id,
                error: "No chips detected".to_string(),
            });
        }

        // 配置所有芯片
        for chip_id in 0..chip_count as u8 {
            if let Err(e) = self.configure_chip(chip_id).await {
                warn!("Failed to configure chip {} on chain {}: {}", chip_id, self.chain_id, e);
            }
        }

        *self.status.write().await = ChainStatus::Idle;
        info!("Chain {} initialized with {} chips", self.chain_id, chip_count);
        Ok(())
    }

    /// 检测芯片数量
    async fn detect_chips(&self) -> Result<u32, DeviceError> {
        info!("Detecting chips on chain {}", self.chain_id);

        let mut chip_count = 0;
        const MAX_CHIPS: u8 = 128; // 最大芯片数量

        for chip_id in 0..MAX_CHIPS {
            match self.detect_chip(chip_id).await {
                Ok(true) => {
                    chip_count += 1;
                }
                Ok(false) => {
                    // 连续3个芯片未检测到则停止
                    if chip_id >= 3 && chip_count == 0 {
                        break;
                    }
                    if chip_id > chip_count + 3 {
                        break;
                    }
                }
                Err(_) => {
                    // 检测错误，继续下一个
                    continue;
                }
            }
        }

        info!("Detected {} chips on chain {}", chip_count, self.chain_id);
        Ok(chip_count)
    }

    /// 设置PLL频率
    async fn set_pll_frequency(&mut self, frequency: u32) -> Result<(), DeviceError> {
        info!("Setting PLL frequency to {} MHz on chain {}", frequency, self.chain_id);

        *self.pll_frequency.write().await = frequency;

        // 应用到所有芯片
        let chip_count = *self.chip_count.read().await;
        for chip_id in 0..chip_count as u8 {
            if let Err(e) = self.configure_chip(chip_id).await {
                warn!("Failed to set frequency for chip {} on chain {}: {}", chip_id, self.chain_id, e);
            }
        }

        Ok(())
    }

    /// 设置电压
    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError> {
        info!("Setting voltage to {} mV on chain {}", voltage, self.chain_id);

        *self.voltage.write().await = voltage;

        // 应用到所有芯片
        let chip_count = *self.chip_count.read().await;
        for chip_id in 0..chip_count as u8 {
            if let Err(e) = self.configure_chip(chip_id).await {
                warn!("Failed to set voltage for chip {} on chain {}: {}", chip_id, self.chain_id, e);
            }
        }

        Ok(())
    }

    /// 发送作业到链
    async fn send_job(&mut self, job_data: &[u8]) -> Result<(), DeviceError> {
        if !*self.enabled.read().await {
            return Err(DeviceError::ChainError {
                chain_id: self.chain_id,
                error: "Chain is disabled".to_string(),
            });
        }

        // 将作业添加到队列
        let mut work_queue = self.work_queue.lock().await;
        work_queue.push(job_data.to_vec());

        // 分发作业到芯片
        let chip_count = *self.chip_count.read().await;
        if chip_count > 0 {
            let chip_id = (work_queue.len() % chip_count as usize) as u8;
            if let Err(e) = self.send_work_to_chip(chip_id, job_data).await {
                warn!("Failed to send work to chip {} on chain {}: {}", chip_id, self.chain_id, e);
                return Err(e);
            }

            *self.status.write().await = ChainStatus::Working;
        }

        Ok(())
    }

    /// 从链读取结果
    async fn read_result(&mut self) -> Result<Option<(u32, u8)>, DeviceError> {
        if !*self.enabled.read().await {
            return Ok(None);
        }

        // 检查结果队列
        let mut result_queue = self.result_queue.lock().await;
        if let Some(result) = result_queue.pop() {
            return Ok(Some(result));
        }

        // 从所有芯片读取结果
        let chip_count = *self.chip_count.read().await;
        for chip_id in 0..chip_count as u8 {
            match self.read_result_from_chip(chip_id).await {
                Ok(Some(result)) => {
                    return Ok(Some(result));
                }
                Ok(None) => {
                    // 继续检查下一个芯片
                }
                Err(e) => {
                    debug!("Error reading from chip {} on chain {}: {}", chip_id, self.chain_id, e);
                }
            }
        }

        Ok(None)
    }

    /// 获取链状态
    async fn get_status(&self) -> Result<ChainStatus, DeviceError> {
        Ok(self.status.read().await.clone())
    }

    /// 获取链温度
    async fn get_temperature(&self) -> Result<f32, DeviceError> {
        let chip_count = *self.chip_count.read().await;
        if chip_count == 0 {
            return Ok(0.0);
        }

        let mut total_temp = 0.0;
        let mut valid_readings = 0;

        // 读取所有芯片温度并计算平均值
        for chip_id in 0..chip_count as u8 {
            match self.get_chip_temperature(chip_id).await {
                Ok(temp) => {
                    total_temp += temp;
                    valid_readings += 1;
                }
                Err(_) => {
                    // 忽略读取失败的芯片
                }
            }
        }

        if valid_readings > 0 {
            Ok(total_temp / valid_readings as f32)
        } else {
            Ok(0.0)
        }
    }

    /// 重置链
    async fn reset(&mut self) -> Result<(), DeviceError> {
        info!("Resetting chain {}", self.chain_id);

        *self.status.write().await = ChainStatus::Initializing;
        *self.reset_count.write().await += 1;

        // 清空队列
        self.work_queue.lock().await.clear();
        self.result_queue.lock().await.clear();

        // 重置所有芯片
        let chip_count = *self.chip_count.read().await;
        for chip_id in 0..chip_count as u8 {
            if let Err(e) = self.reset_chip(chip_id).await {
                warn!("Failed to reset chip {} on chain {}: {}", chip_id, self.chain_id, e);
            }
        }

        // 等待重置完成
        sleep(Duration::from_millis(500)).await;

        *self.status.write().await = ChainStatus::Idle;
        info!("Chain {} reset completed", self.chain_id);
        Ok(())
    }

    /// 启用/禁用链
    async fn set_enabled(&mut self, enabled: bool) -> Result<(), DeviceError> {
        info!("Setting chain {} enabled: {}", self.chain_id, enabled);

        *self.enabled.write().await = enabled;

        if enabled {
            *self.status.write().await = ChainStatus::Idle;
        } else {
            *self.status.write().await = ChainStatus::Disabled;
        }

        Ok(())
    }
}
