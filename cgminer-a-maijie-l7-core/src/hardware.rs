//! 硬件接口抽象层

use cgminer_core::DeviceError;
use async_trait::async_trait;
use std::time::Duration;

use std::path::Path;
use tracing::debug;

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

    /// 电压读取
    async fn read_voltage(&self, channel: u8) -> Result<f32, DeviceError>;

    /// 电流读取
    async fn read_current(&self, channel: u8) -> Result<f32, DeviceError>;

    /// 功率读取
    async fn read_power(&self, channel: u8) -> Result<f32, DeviceError>;

    /// 风扇控制
    async fn set_fan_speed(&self, fan_id: u8, speed: u32) -> Result<(), DeviceError>;
    async fn get_fan_speed(&self, fan_id: u8) -> Result<u32, DeviceError>;

    /// 复位控制
    async fn reset_chain(&self, chain_id: u8) -> Result<(), DeviceError>;
    async fn power_on_chain(&self, chain_id: u8) -> Result<(), DeviceError>;
    async fn power_off_chain(&self, chain_id: u8) -> Result<(), DeviceError>;

    /// 初始化硬件
    async fn initialize(&self) -> Result<(), DeviceError>;

    /// 关闭硬件
    async fn shutdown(&self) -> Result<(), DeviceError>;
}

/// 真实硬件接口实现
pub struct RealHardwareInterface {
    /// SPI设备路径
    spi_devices: Vec<String>,
    /// UART设备路径
    uart_devices: Vec<String>,
    /// GPIO基础路径
    gpio_base: String,
    /// PWM基础路径
    pwm_base: String,
    /// 是否已初始化
    initialized: std::sync::atomic::AtomicBool,
}

impl RealHardwareInterface {
    /// 创建新的真实硬件接口
    pub fn new() -> Self {
        Self {
            spi_devices: vec![
                "/dev/spidev0.0".to_string(),
                "/dev/spidev0.1".to_string(),
                "/dev/spidev1.0".to_string(),
            ],
            uart_devices: vec![
                "/dev/ttyS0".to_string(),
                "/dev/ttyS1".to_string(),
                "/dev/ttyS2".to_string(),
            ],
            gpio_base: "/sys/class/gpio".to_string(),
            pwm_base: "/sys/class/pwm".to_string(),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 检查SPI设备是否存在
    fn check_spi_device(&self, chain_id: u8) -> Result<String, DeviceError> {
        let device_index = chain_id as usize;
        if device_index >= self.spi_devices.len() {
            return Err(DeviceError::hardware_error(format!(
                "SPI设备索引 {} 超出范围", device_index
            )));
        }

        let device_path = &self.spi_devices[device_index];
        if !Path::new(device_path).exists() {
            return Err(DeviceError::hardware_error(format!(
                "SPI设备 {} 不存在", device_path
            )));
        }

        Ok(device_path.clone())
    }

    /// 写入GPIO值
    fn write_gpio(&self, pin: u32, value: bool) -> Result<(), DeviceError> {
        let gpio_path = format!("{}/gpio{}/value", self.gpio_base, pin);
        let value_str = if value { "1" } else { "0" };

        std::fs::write(&gpio_path, value_str).map_err(|e| {
            DeviceError::hardware_error(format!(
                "写入GPIO {} 失败: {}", pin, e
            ))
        })?;

        Ok(())
    }

    /// 读取GPIO值
    fn read_gpio(&self, pin: u32) -> Result<bool, DeviceError> {
        let gpio_path = format!("{}/gpio{}/value", self.gpio_base, pin);

        let value_str = std::fs::read_to_string(&gpio_path).map_err(|e| {
            DeviceError::hardware_error(format!(
                "读取GPIO {} 失败: {}", pin, e
            ))
        })?;

        let value = value_str.trim() == "1";
        Ok(value)
    }

    /// 导出GPIO
    fn export_gpio(&self, pin: u32) -> Result<(), DeviceError> {
        let export_path = format!("{}/export", self.gpio_base);
        let pin_str = pin.to_string();

        // 如果GPIO已经导出，忽略错误
        let _ = std::fs::write(&export_path, &pin_str);

        // 设置GPIO方向
        let direction_path = format!("{}/gpio{}/direction", self.gpio_base, pin);
        std::fs::write(&direction_path, "out").map_err(|e| {
            DeviceError::hardware_error(format!(
                "设置GPIO {} 方向失败: {}", pin, e
            ))
        })?;

        Ok(())
    }
}

impl Default for RealHardwareInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HardwareInterface for RealHardwareInterface {
    async fn spi_transfer(&self, chain_id: u8, data: &[u8]) -> Result<Vec<u8>, DeviceError> {
        debug!("SPI transfer on chain {}: {:02x?}", chain_id, data);

        // 检查SPI设备
        let device_path = self.check_spi_device(chain_id)?;

        // 根据操作系统选择实现方式
        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟SPI传输，因为没有真实的SPI设备
            debug!("Mac环境下模拟SPI传输到设备: {}", device_path);
            tokio::time::sleep(Duration::from_micros(100)).await;

            // 模拟Maijie L7的响应格式
            let mut response = vec![0x55, 0xAA]; // 响应头
            response.extend_from_slice(&[0x00, 0x01]); // 状态码
            response.extend_from_slice(&data[0..std::cmp::min(4, data.len())]); // 回显部分数据
            Ok(response)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux环境下的真实SPI实现
            use std::fs::OpenOptions;
            use std::io::{Read, Write};

            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&device_path)
                .map_err(|e| DeviceError::hardware_error(format!(
                    "打开SPI设备 {} 失败: {}", device_path, e
                )))?;

            // 写入数据
            file.write_all(data).map_err(|e| DeviceError::hardware_error(format!(
                "SPI写入失败: {}", e
            )))?;

            // 读取响应
            let mut response = vec![0u8; data.len() + 4]; // 预期响应长度
            let bytes_read = file.read(&mut response).map_err(|e| DeviceError::hardware_error(format!(
                "SPI读取失败: {}", e
            )))?;

            response.truncate(bytes_read);
            Ok(response)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(DeviceError::hardware_error("不支持的操作系统".to_string()))
        }
    }

    async fn uart_write(&self, chain_id: u8, data: &[u8]) -> Result<(), DeviceError> {
        debug!("UART write on chain {}: {:02x?}", chain_id, data);

        // 在实际实现中，这里会写入UART设备
        tokio::time::sleep(Duration::from_micros(50)).await;
        Ok(())
    }

    async fn uart_read(&self, chain_id: u8, len: usize) -> Result<Vec<u8>, DeviceError> {
        debug!("UART read on chain {}, length: {}", chain_id, len);

        // 在实际实现中，这里会从UART设备读取
        tokio::time::sleep(Duration::from_micros(50)).await;
        Ok(vec![0; len])
    }

    async fn gpio_set(&self, pin: u32, value: bool) -> Result<(), DeviceError> {
        debug!("GPIO set pin {} to {}", pin, value);

        #[cfg(target_os = "linux")]
        {
            // 确保GPIO已导出
            self.export_gpio(pin)?;
            // 写入GPIO值
            self.write_gpio(pin, value)?;
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟GPIO操作
            debug!("Mac环境下模拟GPIO {} 设置为 {}", pin, value);
        }

        Ok(())
    }

    async fn gpio_get(&self, pin: u32) -> Result<bool, DeviceError> {
        debug!("GPIO get pin {}", pin);

        #[cfg(target_os = "linux")]
        {
            // 确保GPIO已导出
            self.export_gpio(pin)?;
            // 读取GPIO值
            return self.read_gpio(pin);
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟GPIO读取
            debug!("Mac环境下模拟GPIO {} 读取", pin);
            return Ok(false);
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(DeviceError::hardware_error("不支持的操作系统".to_string()))
        }
    }

    async fn pwm_set_duty(&self, channel: u32, duty: f32) -> Result<(), DeviceError> {
        debug!("PWM set channel {} duty to {:.2}%", channel, duty * 100.0);

        // 在实际实现中，这里会设置PWM占空比
        Ok(())
    }

    async fn read_temperature(&self, sensor_id: u8) -> Result<f32, DeviceError> {
        debug!("Read temperature from sensor {}", sensor_id);

        #[cfg(target_os = "linux")]
        {
            // Linux环境下读取真实温度传感器
            let temp_path = format!("/sys/class/thermal/thermal_zone{}/temp", sensor_id);
            if Path::new(&temp_path).exists() {
                match std::fs::read_to_string(&temp_path) {
                    Ok(temp_str) => {
                        if let Ok(temp_millidegrees) = temp_str.trim().parse::<i32>() {
                            return Ok(temp_millidegrees as f32 / 1000.0);
                        }
                    }
                    Err(e) => {
                        warn!("读取温度传感器 {} 失败: {}", sensor_id, e);
                    }
                }
            }

            // 如果无法读取真实传感器，返回模拟值
            Ok(45.0 + fastrand::f32() * 10.0)
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟温度读取，基于实际的温度范围
            let base_temp = 45.0;
            let variation = fastrand::f32() * 15.0; // 45-60度范围
            Ok(base_temp + variation)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Ok(45.0 + fastrand::f32() * 10.0)
        }
    }

    async fn read_voltage(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Read voltage from channel {}", channel);

        // 在实际实现中，这里会读取电压
        Ok(12.0 + fastrand::f32() * 0.5)
    }

    async fn read_current(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Read current from channel {}", channel);

        // 在实际实现中，这里会读取电流
        Ok(10.0 + fastrand::f32() * 2.0)
    }

    async fn read_power(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Read power from channel {}", channel);

        // 在实际实现中，这里会读取功率
        let voltage = self.read_voltage(channel).await?;
        let current = self.read_current(channel).await?;
        Ok(voltage * current)
    }

    async fn set_fan_speed(&self, fan_id: u8, speed: u32) -> Result<(), DeviceError> {
        debug!("Set fan {} speed to {}%", fan_id, speed);

        // 限制风扇速度范围
        let speed = speed.clamp(0, 100);

        #[cfg(target_os = "linux")]
        {
            // Linux环境下通过PWM控制风扇速度
            let pwm_path = format!("{}/pwmchip0/pwm{}/duty_cycle", self.pwm_base, fan_id);
            if Path::new(&pwm_path).exists() {
                // PWM占空比 = (速度百分比 / 100) * 最大占空比
                let duty_cycle = (speed as f32 / 100.0 * 255.0) as u32;
                if let Err(e) = std::fs::write(&pwm_path, duty_cycle.to_string()) {
                    warn!("设置风扇 {} 速度失败: {}", fan_id, e);
                }
            } else {
                debug!("PWM设备 {} 不存在，使用模拟控制", pwm_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟风扇控制
            debug!("Mac环境下模拟风扇 {} 速度设置为 {}%", fan_id, speed);
        }

        Ok(())
    }

    async fn get_fan_speed(&self, fan_id: u8) -> Result<u32, DeviceError> {
        debug!("Get fan {} speed", fan_id);

        #[cfg(target_os = "linux")]
        {
            // Linux环境下读取真实风扇速度
            let rpm_path = format!("/sys/class/hwmon/hwmon0/fan{}_input", fan_id + 1);
            if Path::new(&rpm_path).exists() {
                if let Ok(rpm_str) = std::fs::read_to_string(&rpm_path) {
                    if let Ok(rpm) = rpm_str.trim().parse::<u32>() {
                        // 将RPM转换为百分比（假设最大RPM为3000）
                        let percentage = (rpm * 100 / 3000).min(100);
                        return Ok(percentage);
                    }
                }
            }
        }

        // 如果无法读取真实风扇速度，返回模拟值
        Ok(70 + fastrand::u32(0..20))
    }

    async fn reset_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Reset chain {}", chain_id);

        // 在实际实现中，这里会复位ASIC链
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    async fn power_on_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Power on chain {}", chain_id);

        // 在实际实现中，这里会给ASIC链上电
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    async fn power_off_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Power off chain {}", chain_id);

        // 在实际实现中，这里会给ASIC链断电
        Ok(())
    }

    async fn initialize(&self) -> Result<(), DeviceError> {
        debug!("Initialize hardware interface");

        // 检查是否已经初始化
        if self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("硬件接口已经初始化");
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Linux环境下初始化硬件接口
            debug!("初始化Linux硬件接口");

            // 检查SPI设备
            for (i, device) in self.spi_devices.iter().enumerate() {
                if Path::new(device).exists() {
                    debug!("发现SPI设备 {}: {}", i, device);
                } else {
                    warn!("SPI设备 {} 不存在: {}", i, device);
                }
            }

            // 检查UART设备
            for (i, device) in self.uart_devices.iter().enumerate() {
                if Path::new(device).exists() {
                    debug!("发现UART设备 {}: {}", i, device);
                } else {
                    warn!("UART设备 {} 不存在: {}", i, device);
                }
            }

            // 初始化GPIO
            if Path::new(&self.gpio_base).exists() {
                debug!("GPIO基础路径存在: {}", self.gpio_base);
            } else {
                warn!("GPIO基础路径不存在: {}", self.gpio_base);
            }

            // 初始化PWM
            if Path::new(&self.pwm_base).exists() {
                debug!("PWM基础路径存在: {}", self.pwm_base);
            } else {
                warn!("PWM基础路径不存在: {}", self.pwm_base);
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟硬件初始化
            debug!("Mac环境下模拟硬件接口初始化");
            debug!("模拟SPI设备: {:?}", self.spi_devices);
            debug!("模拟UART设备: {:?}", self.uart_devices);
        }

        // 标记为已初始化
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        debug!("硬件接口初始化完成");
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), DeviceError> {
        debug!("Shutdown hardware interface");

        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("硬件接口未初始化，无需关闭");
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Linux环境下关闭硬件接口
            debug!("关闭Linux硬件接口");

            // 这里可以添加清理GPIO、关闭设备文件等操作
            // 例如：取消导出的GPIO
        }

        #[cfg(target_os = "macos")]
        {
            // Mac环境下模拟硬件关闭
            debug!("Mac环境下模拟硬件接口关闭");
        }

        // 标记为未初始化
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        debug!("硬件接口关闭完成");
        Ok(())
    }
}

/// 模拟硬件接口实现（用于测试）
pub struct MockHardwareInterface {
    /// 模拟温度
    temperature: f32,
    /// 模拟电压
    voltage: f32,
    /// 模拟电流
    current: f32,
    /// 模拟风扇速度
    fan_speeds: Vec<u32>,
}

impl MockHardwareInterface {
    /// 创建新的模拟硬件接口
    pub fn new() -> Self {
        Self {
            temperature: 45.0,
            voltage: 12.0,
            current: 10.0,
            fan_speeds: vec![70, 75, 80],
        }
    }
}

impl Default for MockHardwareInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HardwareInterface for MockHardwareInterface {
    async fn spi_transfer(&self, chain_id: u8, data: &[u8]) -> Result<Vec<u8>, DeviceError> {
        debug!("Mock SPI transfer on chain {}: {:02x?}", chain_id, data);
        tokio::time::sleep(Duration::from_micros(10)).await;
        Ok(vec![0x55, 0xAA, 0x00, 0x00])
    }

    async fn uart_write(&self, chain_id: u8, data: &[u8]) -> Result<(), DeviceError> {
        debug!("Mock UART write on chain {}: {:02x?}", chain_id, data);
        tokio::time::sleep(Duration::from_micros(5)).await;
        Ok(())
    }

    async fn uart_read(&self, chain_id: u8, len: usize) -> Result<Vec<u8>, DeviceError> {
        debug!("Mock UART read on chain {}, length: {}", chain_id, len);
        tokio::time::sleep(Duration::from_micros(5)).await;
        Ok(vec![0; len])
    }

    async fn gpio_set(&self, pin: u32, value: bool) -> Result<(), DeviceError> {
        debug!("Mock GPIO set pin {} to {}", pin, value);
        Ok(())
    }

    async fn gpio_get(&self, pin: u32) -> Result<bool, DeviceError> {
        debug!("Mock GPIO get pin {}", pin);
        Ok(false)
    }

    async fn pwm_set_duty(&self, channel: u32, duty: f32) -> Result<(), DeviceError> {
        debug!("Mock PWM set channel {} duty to {:.2}%", channel, duty * 100.0);
        Ok(())
    }

    async fn read_temperature(&self, sensor_id: u8) -> Result<f32, DeviceError> {
        debug!("Mock read temperature from sensor {}", sensor_id);
        Ok(self.temperature + fastrand::f32() * 5.0)
    }

    async fn read_voltage(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Mock read voltage from channel {}", channel);
        Ok(self.voltage + fastrand::f32() * 0.2)
    }

    async fn read_current(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Mock read current from channel {}", channel);
        Ok(self.current + fastrand::f32() * 1.0)
    }

    async fn read_power(&self, channel: u8) -> Result<f32, DeviceError> {
        debug!("Mock read power from channel {}", channel);
        let voltage = self.read_voltage(channel).await?;
        let current = self.read_current(channel).await?;
        Ok(voltage * current)
    }

    async fn set_fan_speed(&self, fan_id: u8, speed: u32) -> Result<(), DeviceError> {
        debug!("Mock set fan {} speed to {}%", fan_id, speed);
        Ok(())
    }

    async fn get_fan_speed(&self, fan_id: u8) -> Result<u32, DeviceError> {
        debug!("Mock get fan {} speed", fan_id);
        let index = fan_id as usize % self.fan_speeds.len();
        Ok(self.fan_speeds[index])
    }

    async fn reset_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Mock reset chain {}", chain_id);
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    async fn power_on_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Mock power on chain {}", chain_id);
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    async fn power_off_chain(&self, chain_id: u8) -> Result<(), DeviceError> {
        debug!("Mock power off chain {}", chain_id);
        Ok(())
    }

    async fn initialize(&self) -> Result<(), DeviceError> {
        debug!("Mock initialize hardware interface");
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), DeviceError> {
        debug!("Mock shutdown hardware interface");
        Ok(())
    }
}
