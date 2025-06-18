pub mod manager;
pub mod stratum;
pub mod connection;
pub mod scheduler;
pub mod switcher;

use crate::error::PoolError;
use crate::device::Work;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub use manager::PoolManager;
pub use stratum::{StratumClient, StratumMessage};
pub use connection::PoolConnection;
pub use scheduler::{PoolScheduler, PoolQuota, FailoverConfig, SchedulerStats};
pub use switcher::{PoolSwitcher, PoolMetrics, SwitchConfig, SwitchEvent, SwitchReason};

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
