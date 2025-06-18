//! ASIC设备基础实现

use cgminer_core::{
    MiningDevice, DeviceInfo, DeviceConfig, DeviceStatus, DeviceStats,
    Work, MiningResult, DeviceError, HashRate, Temperature, Voltage, Frequency
};
use crate::hardware::HardwareInterface;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, info, warn};

/// ASIC设备
pub struct AsicDevice {
    /// 设备信息
    device_info: Arc<RwLock<DeviceInfo>>,
    /// 设备配置
    config: Arc<RwLock<DeviceConfig>>,
    /// 设备状态
    status: Arc<RwLock<DeviceStatus>>,
    /// 设备统计信息
    stats: Arc<RwLock<DeviceStats>>,
    /// 当前工作
    current_work: Arc<Mutex<Option<Work>>>,
    /// 硬件接口
    hardware: Arc<dyn HardwareInterface>,
    /// 启动时间
    start_time: Option<Instant>,
    /// 最后一次挖矿时间
    last_mining_time: Arc<RwLock<Option<Instant>>>,
}

impl AsicDevice {
    /// 创建新的ASIC设备
    pub async fn new(
        device_info: DeviceInfo,
        config: DeviceConfig,
        hardware: Arc<dyn HardwareInterface>,
    ) -> Result<Self, DeviceError> {
        let device_id = device_info.id;
        let stats = DeviceStats::new(device_id);

        Ok(Self {
            device_info: Arc::new(RwLock::new(device_info)),
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(DeviceStatus::Uninitialized)),
            stats: Arc::new(RwLock::new(stats)),
            current_work: Arc::new(Mutex::new(None)),
            hardware,
            start_time: None,
            last_mining_time: Arc::new(RwLock::new(None)),
        })
    }

    /// 发送工作到ASIC芯片
    async fn send_work_to_asic(&self, work: &Work) -> Result<(), DeviceError> {
        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        debug!("向ASIC链 {} 发送工作 {}", chain_id, work.id);

        // 构建ASIC工作数据包
        let mut work_data = Vec::new();
        work_data.extend_from_slice(&work.header);
        work_data.extend_from_slice(&work.target);

        // 通过SPI发送工作数据
        let response = self.hardware.spi_transfer(chain_id, &work_data).await?;

        if response.len() < 4 || response[0] != 0x55 || response[1] != 0xAA {
            return Err(DeviceError::communication_error("ASIC响应无效"));
        }

        debug!("ASIC链 {} 接受工作成功", chain_id);
        Ok(())
    }

    /// 从ASIC芯片读取结果
    async fn read_result_from_asic(&self) -> Result<Option<MiningResult>, DeviceError> {
        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        let work_id = {
            let work = self.current_work.lock().await;
            work.as_ref().map(|w| w.id).unwrap_or(0)
        };

        // 通过SPI读取结果
        let read_cmd = vec![0xAA, 0x55, 0x01, 0x00]; // 读取命令
        let response = self.hardware.spi_transfer(chain_id, &read_cmd).await?;

        if response.len() < 8 {
            return Ok(None); // 没有结果
        }

        // 检查是否有有效结果
        if response[0] == 0x55 && response[1] == 0xAA && response[2] == 0x02 {
            let nonce = u32::from_le_bytes([response[4], response[5], response[6], response[7]]);

            debug!("ASIC链 {} 找到nonce: {:08x}", chain_id, nonce);

            // 构建挖矿结果
            let result = MiningResult::new(
                work_id,
                self.device_id(),
                nonce,
                response[8..].to_vec(), // 哈希值
                true, // ASIC已验证满足目标
            );

            // 更新统计信息
            {
                let mut stats = self.stats.write().map_err(|e| {
                    DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
                })?;
                stats.accepted_work += 1;
                stats.last_updated = SystemTime::now();
            }

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// 更新设备温度和其他传感器数据
    async fn update_sensors(&self) -> Result<(), DeviceError> {
        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 读取温度
        let temperature = self.hardware.read_temperature(chain_id).await?;

        // 读取电压
        let voltage = self.hardware.read_voltage(chain_id).await?;

        // 读取电流
        let _current = self.hardware.read_current(chain_id).await?;

        // 读取功率
        let power = self.hardware.read_power(chain_id).await?;

        // 更新设备信息
        {
            let mut info = self.device_info.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            info.update_temperature(temperature);
            info.updated_at = SystemTime::now();
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;

            stats.temperature = Some(Temperature::new(temperature));
            stats.voltage = Some(Voltage::from_volts(voltage));
            stats.power_consumption = Some(power as f64);
            stats.last_updated = SystemTime::now();

            // 更新频率信息
            let config = self.config.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            stats.frequency = Some(Frequency::new(config.frequency));
        }

        // 检查温度是否过高
        let config = self.config.read().map_err(|e| {
            DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
        })?;

        if temperature > config.temperature_limit {
            warn!("ASIC设备 {} 温度过高: {:.1}°C (限制: {:.1}°C)",
                  self.device_id(), temperature, config.temperature_limit);

            // 设置设备状态为错误
            let mut status = self.status.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            *status = DeviceStatus::Error(format!("温度过高: {:.1}°C", temperature));
        }

        Ok(())
    }

    /// 计算当前算力
    async fn calculate_hashrate(&self) -> Result<f64, DeviceError> {
        let config = self.config.read().map_err(|e| {
            DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
        })?;

        // 基于频率和芯片数量计算理论算力
        // 这是一个简化的计算，实际算力会根据ASIC芯片的具体规格而定
        let base_hashrate_per_chip = 1_000_000_000.0; // 1 GH/s per chip at base frequency
        let frequency_factor = config.frequency as f64 / 600.0; // 基准频率600MHz
        let total_hashrate = base_hashrate_per_chip * config.chip_count as f64 * frequency_factor;

        Ok(total_hashrate)
    }
}

#[async_trait]
impl MiningDevice for AsicDevice {
    /// 获取设备ID
    fn device_id(&self) -> u32 {
        // 使用 tokio::task::block_in_place 来在异步上下文中调用同步代码
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.device_info.read().unwrap().id
            })
        })
    }

    /// 获取设备信息
    async fn get_info(&self) -> Result<DeviceInfo, DeviceError> {
        let info = self.device_info.read().map_err(|e| {
            DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(info.clone())
    }

    /// 初始化设备
    async fn initialize(&mut self, config: DeviceConfig) -> Result<(), DeviceError> {
        info!("初始化ASIC设备 {}", self.device_id());

        let chain_id = config.chain_id;

        // 更新配置
        {
            let mut device_config = self.config.write().map_err(|e| {
                DeviceError::initialization_failed(format!("Failed to acquire write lock: {}", e))
            })?;
            *device_config = config.clone();
        }

        // 给链上电
        self.hardware.power_on_chain(chain_id).await?;

        // 等待上电稳定
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 复位链
        self.hardware.reset_chain(chain_id).await?;

        // 等待复位完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 设置频率和电压
        // 这里需要根据具体的ASIC芯片协议来实现
        let freq_cmd = vec![0xAA, 0x55, 0x10, config.frequency as u8];
        self.hardware.spi_transfer(chain_id, &freq_cmd).await?;

        let volt_cmd = vec![0xAA, 0x55, 0x11, (config.voltage / 10) as u8];
        self.hardware.spi_transfer(chain_id, &volt_cmd).await?;

        // 更新状态
        {
            let mut status = self.status.write().map_err(|e| {
                DeviceError::initialization_failed(format!("Failed to acquire write lock: {}", e))
            })?;
            *status = DeviceStatus::Idle;
        }

        // 更新传感器数据
        self.update_sensors().await?;

        info!("ASIC设备 {} 初始化完成", self.device_id());
        Ok(())
    }

    /// 启动设备
    async fn start(&mut self) -> Result<(), DeviceError> {
        info!("启动ASIC设备 {}", self.device_id());

        {
            let mut status = self.status.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            *status = DeviceStatus::Running;
        }

        self.start_time = Some(Instant::now());

        // 更新算力统计
        let hashrate = self.calculate_hashrate().await?;
        {
            let mut stats = self.stats.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            stats.current_hashrate = HashRate::new(hashrate);
            stats.average_hashrate = HashRate::new(hashrate);
        }

        info!("ASIC设备 {} 启动完成，预期算力: {:.2} TH/s",
              self.device_id(), hashrate / 1_000_000_000_000.0);
        Ok(())
    }

    /// 停止设备
    async fn stop(&mut self) -> Result<(), DeviceError> {
        info!("停止ASIC设备 {}", self.device_id());

        {
            let mut status = self.status.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            *status = DeviceStatus::Idle;
        }

        // 清除当前工作
        {
            let mut work = self.current_work.lock().await;
            *work = None;
        }

        // 可选：给链断电以节省功耗
        let _chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 注释掉断电，因为可能需要保持设备在线
        // self.hardware.power_off_chain(chain_id).await?;

        info!("ASIC设备 {} 已停止", self.device_id());
        Ok(())
    }

    /// 重启设备
    async fn restart(&mut self) -> Result<(), DeviceError> {
        info!("重启ASIC设备 {}", self.device_id());
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 重新初始化
        let config = {
            let config = self.config.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            config.clone()
        };

        self.initialize(config).await?;
        self.start().await?;
        Ok(())
    }

    /// 提交工作
    async fn submit_work(&mut self, work: Work) -> Result<(), DeviceError> {
        debug!("向ASIC设备 {} 提交工作 {}", self.device_id(), work.id);

        // 发送工作到ASIC
        self.send_work_to_asic(&work).await?;

        // 存储当前工作
        {
            let mut current_work = self.current_work.lock().await;
            *current_work = Some(work);
        }

        Ok(())
    }

    /// 获取挖矿结果
    async fn get_result(&mut self) -> Result<Option<MiningResult>, DeviceError> {
        // 更新传感器数据
        self.update_sensors().await?;

        // 读取ASIC结果
        self.read_result_from_asic().await
    }

    /// 获取设备状态
    async fn get_status(&self) -> Result<DeviceStatus, DeviceError> {
        let status = self.status.read().map_err(|e| {
            DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(status.clone())
    }

    /// 获取设备统计信息
    async fn get_stats(&self) -> Result<DeviceStats, DeviceError> {
        // 更新运行时间
        if let Some(start_time) = self.start_time {
            let mut stats = self.stats.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            stats.uptime = start_time.elapsed();
        }

        let stats = self.stats.read().map_err(|e| {
            DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(stats.clone())
    }

    /// 设置频率
    async fn set_frequency(&mut self, frequency: u32) -> Result<(), DeviceError> {
        info!("设置ASIC设备 {} 频率为 {} MHz", self.device_id(), frequency);

        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 发送频率设置命令到ASIC
        let freq_cmd = vec![0xAA, 0x55, 0x10, frequency as u8];
        self.hardware.spi_transfer(chain_id, &freq_cmd).await?;

        // 更新配置
        {
            let mut config = self.config.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            config.frequency = frequency;
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            info.update_frequency(frequency);
        }

        // 重新计算算力
        let hashrate = self.calculate_hashrate().await?;
        {
            let mut stats = self.stats.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            stats.current_hashrate = HashRate::new(hashrate);
        }

        Ok(())
    }

    /// 设置电压
    async fn set_voltage(&mut self, voltage: u32) -> Result<(), DeviceError> {
        info!("设置ASIC设备 {} 电压为 {} mV", self.device_id(), voltage);

        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 发送电压设置命令到ASIC
        let volt_cmd = vec![0xAA, 0x55, 0x11, (voltage / 10) as u8];
        self.hardware.spi_transfer(chain_id, &volt_cmd).await?;

        // 更新配置
        {
            let mut config = self.config.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            config.voltage = voltage;
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            info.update_voltage(voltage);
        }

        Ok(())
    }

    /// 设置风扇速度
    async fn set_fan_speed(&mut self, speed: u32) -> Result<(), DeviceError> {
        info!("设置ASIC设备 {} 风扇速度为 {}%", self.device_id(), speed);

        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 设置风扇速度
        self.hardware.set_fan_speed(chain_id, speed).await?;

        // 更新配置
        {
            let mut config = self.config.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            config.fan_speed = Some(speed);
        }

        // 更新设备信息
        {
            let mut info = self.device_info.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            info.fan_speed = Some(speed);
            info.updated_at = SystemTime::now();
        }

        Ok(())
    }

    /// 重置设备
    async fn reset(&mut self) -> Result<(), DeviceError> {
        info!("重置ASIC设备 {}", self.device_id());

        let chain_id = {
            let info = self.device_info.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            info.chain_id
        };

        // 硬件复位
        self.hardware.reset_chain(chain_id).await?;

        // 等待复位完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 重置统计信息
        {
            let mut stats = self.stats.write().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire write lock: {}", e))
            })?;
            *stats = DeviceStats::new(self.device_id());
        }

        // 清除当前工作
        {
            let mut work = self.current_work.lock().await;
            *work = None;
        }

        // 重置时间
        self.start_time = Some(Instant::now());

        info!("ASIC设备 {} 重置完成", self.device_id());
        Ok(())
    }

    /// 获取设备健康状态
    async fn health_check(&self) -> Result<bool, DeviceError> {
        let status = self.get_status().await?;
        let stats = self.get_stats().await?;

        // 检查设备状态
        let status_ok = matches!(status, DeviceStatus::Running | DeviceStatus::Idle);

        // 检查温度
        let temp_ok = if let Some(temp) = stats.temperature {
            let config = self.config.read().map_err(|e| {
                DeviceError::hardware_error(format!("Failed to acquire read lock: {}", e))
            })?;
            temp.celsius < config.temperature_limit
        } else {
            true
        };

        // 检查算力
        let hashrate_ok = stats.current_hashrate.hashes_per_second > 0.0;

        // 检查错误率
        let error_rate_ok = stats.error_rate() < 0.05; // 错误率不超过5%

        Ok(status_ok && temp_ok && hashrate_ok && error_rate_ok)
    }
}
