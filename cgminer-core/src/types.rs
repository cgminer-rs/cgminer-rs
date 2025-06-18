//! 基础类型定义

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// 工作单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    /// 工作ID
    pub id: u64,
    /// 区块头数据
    pub header: Vec<u8>,
    /// 目标难度
    pub target: Vec<u8>,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 额外随机数
    pub extranonce: Vec<u8>,
    /// 工作难度
    pub difficulty: f64,
}

impl Work {
    /// 创建新的工作单元
    pub fn new(id: u64, header: Vec<u8>, target: Vec<u8>, difficulty: f64) -> Self {
        Self {
            id,
            header,
            target,
            timestamp: SystemTime::now(),
            extranonce: Vec::new(),
            difficulty,
        }
    }

    /// 检查工作是否过期
    pub fn is_expired(&self, max_age: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|age| age > max_age)
            .unwrap_or(true)
    }
}

/// 挖矿结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    /// 工作ID
    pub work_id: u64,
    /// 设备ID
    pub device_id: u32,
    /// 随机数
    pub nonce: u32,
    /// 额外随机数
    pub extranonce: Vec<u8>,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 哈希值
    pub hash: Vec<u8>,
    /// 是否满足目标难度
    pub meets_target: bool,
}

impl MiningResult {
    /// 创建新的挖矿结果
    pub fn new(
        work_id: u64,
        device_id: u32,
        nonce: u32,
        hash: Vec<u8>,
        meets_target: bool,
    ) -> Self {
        Self {
            work_id,
            device_id,
            nonce,
            extranonce: Vec::new(),
            timestamp: SystemTime::now(),
            hash,
            meets_target,
        }
    }
}

/// 算力类型
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HashRate {
    /// 每秒哈希数
    pub hashes_per_second: f64,
}

impl HashRate {
    /// 创建新的算力
    pub fn new(hashes_per_second: f64) -> Self {
        Self { hashes_per_second }
    }

    /// 转换为 TH/s
    pub fn as_th_per_second(&self) -> f64 {
        self.hashes_per_second / 1_000_000_000_000.0
    }

    /// 转换为 GH/s
    pub fn as_gh_per_second(&self) -> f64 {
        self.hashes_per_second / 1_000_000_000.0
    }

    /// 转换为 MH/s
    pub fn as_mh_per_second(&self) -> f64 {
        self.hashes_per_second / 1_000_000.0
    }

    /// 从 TH/s 创建
    pub fn from_th_per_second(th_per_second: f64) -> Self {
        Self::new(th_per_second * 1_000_000_000_000.0)
    }

    /// 从 GH/s 创建
    pub fn from_gh_per_second(gh_per_second: f64) -> Self {
        Self::new(gh_per_second * 1_000_000_000.0)
    }

    /// 从 MH/s 创建
    pub fn from_mh_per_second(mh_per_second: f64) -> Self {
        Self::new(mh_per_second * 1_000_000.0)
    }
}

impl std::fmt::Display for HashRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.hashes_per_second >= 1_000_000_000_000.0 {
            write!(f, "{:.2} TH/s", self.as_th_per_second())
        } else if self.hashes_per_second >= 1_000_000_000.0 {
            write!(f, "{:.2} GH/s", self.as_gh_per_second())
        } else if self.hashes_per_second >= 1_000_000.0 {
            write!(f, "{:.2} MH/s", self.as_mh_per_second())
        } else {
            write!(f, "{:.2} H/s", self.hashes_per_second)
        }
    }
}

/// 温度类型
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Temperature {
    /// 摄氏度
    pub celsius: f32,
}

impl Temperature {
    /// 创建新的温度
    pub fn new(celsius: f32) -> Self {
        Self { celsius }
    }

    /// 转换为华氏度
    pub fn as_fahrenheit(&self) -> f32 {
        self.celsius * 9.0 / 5.0 + 32.0
    }

    /// 从华氏度创建
    pub fn from_fahrenheit(fahrenheit: f32) -> Self {
        Self::new((fahrenheit - 32.0) * 5.0 / 9.0)
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}°C", self.celsius)
    }
}

/// 电压类型
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Voltage {
    /// 毫伏
    pub millivolts: u32,
}

impl Voltage {
    /// 创建新的电压
    pub fn new(millivolts: u32) -> Self {
        Self { millivolts }
    }

    /// 转换为伏特
    pub fn as_volts(&self) -> f32 {
        self.millivolts as f32 / 1000.0
    }

    /// 从伏特创建
    pub fn from_volts(volts: f32) -> Self {
        Self::new((volts * 1000.0) as u32)
    }
}

impl std::fmt::Display for Voltage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}V", self.as_volts())
    }
}

/// 频率类型
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Frequency {
    /// 兆赫兹
    pub megahertz: u32,
}

impl Frequency {
    /// 创建新的频率
    pub fn new(megahertz: u32) -> Self {
        Self { megahertz }
    }

    /// 转换为赫兹
    pub fn as_hertz(&self) -> u64 {
        self.megahertz as u64 * 1_000_000
    }

    /// 从赫兹创建
    pub fn from_hertz(hertz: u64) -> Self {
        Self::new((hertz / 1_000_000) as u32)
    }
}

impl std::fmt::Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}MHz", self.megahertz)
    }
}
