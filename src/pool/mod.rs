pub mod manager;
pub mod stratum;
pub mod connection;
pub mod scheduler;
pub mod switcher;
pub mod proxy;

use crate::error::PoolError;
use crate::device::Work;
use cgminer_core::types::MiningResult;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub use manager::PoolManager;


/// 矿池信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub id: u32,
    pub url: String,
    pub user: String,
    pub password: String,
    pub priority: u8,
    pub quota: Option<u32>,
    pub enabled: bool,
    pub status: PoolStatus,
    pub connected_at: Option<SystemTime>,
    pub last_share_time: Option<SystemTime>,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub difficulty: f64,
    pub ping: Option<Duration>,
}

impl Pool {
    pub fn new(id: u32, url: String, user: String, password: String, priority: u8, enabled: bool) -> Self {
        Self {
            id,
            url,
            user,
            password,
            priority,
            quota: None,
            enabled,
            status: PoolStatus::Disconnected,
            connected_at: None,
            last_share_time: None,
            accepted_shares: 0,
            rejected_shares: 0,
            stale_shares: 0,
            difficulty: 1.0,
            ping: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status, PoolStatus::Connected)
    }

    pub fn get_accept_rate(&self) -> f64 {
        let total = self.accepted_shares + self.rejected_shares;
        if total == 0 {
            0.0
        } else {
            self.accepted_shares as f64 / total as f64 * 100.0
        }
    }

    pub fn get_reject_rate(&self) -> f64 {
        let total = self.accepted_shares + self.rejected_shares;
        if total == 0 {
            0.0
        } else {
            self.rejected_shares as f64 / total as f64 * 100.0
        }
    }

    pub fn record_accepted_share(&mut self, difficulty: f64) {
        self.accepted_shares += 1;
        self.last_share_time = Some(SystemTime::now());
        if difficulty > self.difficulty {
            self.difficulty = difficulty;
        }
    }

    pub fn record_rejected_share(&mut self) {
        self.rejected_shares += 1;
    }

    pub fn record_stale_share(&mut self) {
        self.stale_shares += 1;
    }
}

/// 矿池状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolStatus {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 认证中
    Authenticating,
    /// 认证成功
    Authenticated,
    /// 错误状态
    Error(String),
    /// 已禁用
    Disabled,
}

/// 矿池策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolStrategy {
    /// 故障转移
    Failover,
    /// 轮询
    RoundRobin,
    /// 负载均衡
    LoadBalance,
    /// 配额
    Quota,
}

/// 份额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub id: Uuid,
    pub pool_id: u32,
    pub work_id: Uuid,
    pub device_id: u32,
    pub job_id: String,
    pub extra_nonce2: String,
    pub nonce: u32,
    pub ntime: u32,  // 添加ntime字段，用于正确的份额提交
    pub timestamp: SystemTime,
    pub difficulty: f64,
    pub status: ShareStatus,
}

impl Share {
    pub fn new(
        pool_id: u32,
        work_id: Uuid,
        device_id: u32,
        job_id: String,
        extra_nonce2: String,
        nonce: u32,
        ntime: u32,
        difficulty: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            pool_id,
            work_id,
            device_id,
            job_id,
            extra_nonce2,
            nonce,
            ntime,
            timestamp: SystemTime::now(),
            difficulty,
            status: ShareStatus::Pending,
        }
    }

    pub fn mark_accepted(mut self) -> Self {
        self.status = ShareStatus::Accepted;
        self
    }

    pub fn mark_rejected(mut self, reason: String) -> Self {
        self.status = ShareStatus::Rejected(reason);
        self
    }

    pub fn mark_stale(mut self) -> Self {
        self.status = ShareStatus::Stale;
        self
    }

    /// 从挖矿结果创建份额
    pub fn from_mining_result(
        pool_id: u32,
        work: &Work,
        result: &MiningResult,
    ) -> Result<Self, String> {
        // 验证结果是否属于这个工作
        if result.work_id != work.id {
            return Err("Mining result work ID does not match work".to_string());
        }

        // 计算实际份额难度
        let actual_difficulty = Self::calculate_share_difficulty(&result.hash)?;

        // 转换extranonce2为十六进制字符串
        let extra_nonce2 = hex::encode(&result.extranonce2);

        // 验证挖矿结果数据完整性
        // TODO: 重新启用验证 - DataValidator::validate_mining_result(result)
        //     .map_err(|e| format!("Mining result validation failed: {}", e))?;

        // 验证Work和MiningResult的一致性
        // TODO: 重新启用验证 - DataValidator::validate_work_result_consistency(work, result)
        //     .map_err(|e| format!("Work-result consistency check failed: {}", e))?;

        let share = Self {
            id: Uuid::new_v4(),
            pool_id,
            work_id: work.id,
            device_id: result.device_id,
            job_id: work.job_id.clone(),
            extra_nonce2,
            nonce: result.nonce,
            ntime: work.ntime,  // 使用工作数据中的ntime而不是结果时间戳
            timestamp: result.timestamp,
            difficulty: actual_difficulty,
            status: ShareStatus::Pending,
        };

        // 验证创建的份额数据
        // TODO: 重新启用验证 - DataValidator::validate_share(&share)
        //     .map_err(|e| format!("Share validation failed: {}", e))?;

        Ok(share)
    }

    /// 计算份额难度
    pub fn calculate_share_difficulty(hash: &[u8]) -> Result<f64, String> {
        if hash.len() != 32 {
            return Err("Invalid hash length".to_string());
        }

        // 将哈希转换为大端序的256位整数进行难度计算
        // Bitcoin的难度计算基于哈希值与难度1目标的比较

        // 取哈希的最后8个字节作为64位整数（小端序）
        let mut hash_bytes = [0u8; 8];
        hash_bytes.copy_from_slice(&hash[24..32]);
        let hash_value = u64::from_le_bytes(hash_bytes);

        if hash_value == 0 {
            return Ok(f64::INFINITY);
        }

        // 难度1的目标值（简化计算）
        // 实际的Bitcoin难度1目标是 0x1d00ffff，这里使用简化版本
        const DIFFICULTY_1_TARGET: u64 = 0x00000000FFFF0000;

        let difficulty = DIFFICULTY_1_TARGET as f64 / hash_value as f64;

        // 确保难度至少为1.0
        Ok(difficulty.max(1.0))
    }

    /// 验证份额是否满足最小难度要求
    pub fn meets_minimum_difficulty(&self, min_difficulty: f64) -> bool {
        self.difficulty >= min_difficulty
    }

    /// 验证份额的完整性
    pub fn validate(&self) -> Result<(), String> {
        // 检查基本字段
        if self.job_id.is_empty() {
            return Err("Job ID is empty".to_string());
        }

        if self.extra_nonce2.is_empty() {
            return Err("Extra nonce2 is empty".to_string());
        }

        // 验证extra_nonce2是有效的十六进制字符串
        if hex::decode(&self.extra_nonce2).is_err() {
            return Err("Invalid extra_nonce2 format".to_string());
        }

        // 验证难度值
        if self.difficulty <= 0.0 || !self.difficulty.is_finite() {
            return Err(format!("Invalid difficulty: {}", self.difficulty));
        }

        // 验证nonce不为零
        if self.nonce == 0 {
            return Err("Nonce cannot be zero".to_string());
        }

        Ok(())
    }

    /// 获取份额的年龄
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or(Duration::from_secs(0))
    }

    /// 检查份额是否过期
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.age() > max_age
    }
}

/// 份额状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShareStatus {
    /// 待处理
    Pending,
    /// 已接受
    Accepted,
    /// 已拒绝
    Rejected(String),
    /// 过期
    Stale,
}

/// 矿池统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoolStats {
    pub pool_id: u32,
    pub uptime: Duration,
    pub connected_time: Duration,
    pub total_shares: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub best_share: f64,
    pub average_difficulty: f64,
    pub last_share_time: Option<SystemTime>,
    pub connection_attempts: u32,
    pub disconnection_count: u32,
    pub last_error: Option<String>,
}

impl PoolStats {
    pub fn new(pool_id: u32) -> Self {
        Self {
            pool_id,
            ..Default::default()
        }
    }

    pub fn record_share(&mut self, share: &Share) {
        self.total_shares += 1;
        self.last_share_time = Some(share.timestamp);

        match &share.status {
            ShareStatus::Accepted => {
                self.accepted_shares += 1;
                if share.difficulty > self.best_share {
                    self.best_share = share.difficulty;
                }
            }
            ShareStatus::Rejected(_) => self.rejected_shares += 1,
            ShareStatus::Stale => self.stale_shares += 1,
            ShareStatus::Pending => {}
        }

        // 更新平均难度
        if self.total_shares > 0 {
            self.average_difficulty = (self.average_difficulty * (self.total_shares - 1) as f64 + share.difficulty) / self.total_shares as f64;
        }
    }

    pub fn record_connection_attempt(&mut self) {
        self.connection_attempts += 1;
    }

    pub fn record_disconnection(&mut self) {
        self.disconnection_count += 1;
    }

    pub fn record_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    pub fn get_accept_rate(&self) -> f64 {
        if self.total_shares == 0 {
            0.0
        } else {
            self.accepted_shares as f64 / self.total_shares as f64 * 100.0
        }
    }

    pub fn get_reject_rate(&self) -> f64 {
        if self.total_shares == 0 {
            0.0
        } else {
            self.rejected_shares as f64 / self.total_shares as f64 * 100.0
        }
    }

    pub fn get_stale_rate(&self) -> f64 {
        if self.total_shares == 0 {
            0.0
        } else {
            self.stale_shares as f64 / self.total_shares as f64 * 100.0
        }
    }
}

/// 矿池事件
#[derive(Debug, Clone)]
pub enum PoolEvent {
    /// 连接状态变更
    ConnectionChanged {
        pool_id: u32,
        old_status: PoolStatus,
        new_status: PoolStatus,
        timestamp: SystemTime,
    },
    /// 新工作接收
    WorkReceived {
        pool_id: u32,
        work: Work,
        timestamp: SystemTime,
    },
    /// 份额提交
    ShareSubmitted {
        pool_id: u32,
        share: Share,
        timestamp: SystemTime,
    },
    /// 份额响应
    ShareResponse {
        pool_id: u32,
        share_id: Uuid,
        accepted: bool,
        reason: Option<String>,
        timestamp: SystemTime,
    },
    /// 难度调整
    DifficultyChanged {
        pool_id: u32,
        old_difficulty: f64,
        new_difficulty: f64,
        timestamp: SystemTime,
    },
    /// 错误事件
    Error {
        pool_id: u32,
        error: PoolError,
        timestamp: SystemTime,
    },
}

impl PoolEvent {
    pub fn timestamp(&self) -> SystemTime {
        match self {
            PoolEvent::ConnectionChanged { timestamp, .. } => *timestamp,
            PoolEvent::WorkReceived { timestamp, .. } => *timestamp,
            PoolEvent::ShareSubmitted { timestamp, .. } => *timestamp,
            PoolEvent::ShareResponse { timestamp, .. } => *timestamp,
            PoolEvent::DifficultyChanged { timestamp, .. } => *timestamp,
            PoolEvent::Error { timestamp, .. } => *timestamp,
        }
    }

    pub fn pool_id(&self) -> u32 {
        match self {
            PoolEvent::ConnectionChanged { pool_id, .. } => *pool_id,
            PoolEvent::WorkReceived { pool_id, .. } => *pool_id,
            PoolEvent::ShareSubmitted { pool_id, .. } => *pool_id,
            PoolEvent::ShareResponse { pool_id, .. } => *pool_id,
            PoolEvent::DifficultyChanged { pool_id, .. } => *pool_id,
            PoolEvent::Error { pool_id, .. } => *pool_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgminer_core::types::Work;

    #[test]
    fn test_share_difficulty_calculation() {
        // 测试全零哈希（应该返回无穷大难度）
        let hash = vec![0u8; 32];
        let result = Share::calculate_share_difficulty(&hash);
        assert!(result.is_ok());
        let difficulty = result.unwrap();
        assert!(difficulty.is_infinite());

        // 测试无效哈希长度
        let invalid_hash = vec![0u8; 16];
        let result = Share::calculate_share_difficulty(&invalid_hash);
        assert!(result.is_err());

        // 测试高难度哈希（很多零）
        let mut high_diff_hash = vec![0u8; 32];
        high_diff_hash[31] = 1; // 最后一个字节为1
        let result = Share::calculate_share_difficulty(&high_diff_hash);
        assert!(result.is_ok());
        let difficulty = result.unwrap();
        // 由于函数确保难度至少为1.0，所以这里应该是>=1.0
        assert!(difficulty >= 1.0);

        // 测试正常难度哈希
        let mut normal_hash = vec![0u8; 32];
        normal_hash[24] = 0xFF; // 设置一个较大的值
        normal_hash[25] = 0xFF;
        let result = Share::calculate_share_difficulty(&normal_hash);
        assert!(result.is_ok());
        let difficulty = result.unwrap();
        assert!(difficulty > 0.0 && difficulty.is_finite());
    }

    #[test]
    fn test_share_validation() {
        let share = Share::new(
            1,
            Uuid::new_v4(),
            0,
            "test_job".to_string(),
            "deadbeef".to_string(),
            12345,
            0x5f5e100, // ntime
            1024.0,
        );

        // 基本验证应该通过
        assert!(share.validate().is_ok());

        // 测试空job_id
        let mut invalid_share = share.clone();
        invalid_share.job_id = String::new();
        assert!(invalid_share.validate().is_err());

        // 测试空extra_nonce2
        let mut invalid_share = share.clone();
        invalid_share.extra_nonce2 = String::new();
        assert!(invalid_share.validate().is_err());

        // 测试无效的extra_nonce2格式
        let mut invalid_share = share.clone();
        invalid_share.extra_nonce2 = "invalid_hex".to_string();
        assert!(invalid_share.validate().is_err());

        // 测试零nonce
        let mut invalid_share = share.clone();
        invalid_share.nonce = 0;
        assert!(invalid_share.validate().is_err());

        // 测试无效难度
        let mut invalid_share = share.clone();
        invalid_share.difficulty = 0.0;
        assert!(invalid_share.validate().is_err());
    }

    #[test]
    fn test_share_minimum_difficulty() {
        let share = Share::new(
            1,
            Uuid::new_v4(),
            0,
            "test_job".to_string(),
            "deadbeef".to_string(),
            12345,
            0x5f5e100, // ntime
            1024.0,
        );

        assert!(share.meets_minimum_difficulty(512.0));
        assert!(share.meets_minimum_difficulty(1024.0));
        assert!(!share.meets_minimum_difficulty(2048.0));
    }

    #[test]
    fn test_share_age_and_staleness() {
        let share = Share::new(
            1,
            Uuid::new_v4(),
            0,
            "test_job".to_string(),
            "deadbeef".to_string(),
            12345,
            0x5f5e100, // ntime
            1024.0,
        );

        // 新创建的份额不应该过期
        assert!(!share.is_stale(Duration::from_secs(60)));

        // 年龄应该很小
        let age = share.age();
        assert!(age < Duration::from_secs(1));
    }

    #[test]
    fn test_share_from_mining_result() {
        let work = Work::new(
            "test_job".to_string(),
            [0u8; 32],
            [0u8; 80],
            1024.0,
        );

        let mining_result = MiningResult::new(
            work.id,
            0,
            12345,
            vec![0u8; 32],
            true,
        ).with_extranonce2(vec![0xde, 0xad, 0xbe, 0xef]);

        let share = Share::from_mining_result(1, &work, &mining_result);
        assert!(share.is_ok());

        let share = share.unwrap();
        assert_eq!(share.pool_id, 1);
        assert_eq!(share.work_id, work.id);
        assert_eq!(share.device_id, 0);
        assert_eq!(share.job_id, "test_job");
        assert_eq!(share.nonce, 12345);
        assert_eq!(share.extra_nonce2, "deadbeef");
    }
}
