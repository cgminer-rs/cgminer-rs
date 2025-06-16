use crate::error::PoolError;
use crate::pool::{Pool, PoolStatus};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};

/// 矿池连接管理器
pub struct PoolConnection {
    /// 矿池信息
    pool: Arc<RwLock<Pool>>,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 最后活动时间
    last_activity: Arc<RwLock<SystemTime>>,
    /// 连接尝试次数
    connection_attempts: Arc<RwLock<u32>>,
    /// 最大重连次数
    max_reconnect_attempts: u32,
    /// 重连间隔
    reconnect_interval: Duration,
    /// 连接超时
    connection_timeout: Duration,
    /// 心跳间隔
    heartbeat_interval: Duration,
}

impl PoolConnection {
    /// 创建新的矿池连接
    pub fn new(pool: Pool) -> Self {
        Self {
            pool: Arc::new(RwLock::new(pool)),
            connected: Arc::new(RwLock::new(false)),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            connection_attempts: Arc::new(RwLock::new(0)),
            max_reconnect_attempts: 10,
            reconnect_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(60),
        }
    }
    
    /// 连接到矿池
    pub async fn connect(&self) -> Result<(), PoolError> {
        let mut pool = self.pool.write().await;
        
        info!("Connecting to pool: {}", pool.url);
        pool.status = PoolStatus::Connecting;
        
        // 增加连接尝试次数
        *self.connection_attempts.write().await += 1;
        
        // 模拟连接过程
        match self.attempt_connection().await {
            Ok(_) => {
                pool.status = PoolStatus::Connected;
                pool.connected_at = Some(SystemTime::now());
                *self.connected.write().await = true;
                *self.last_activity.write().await = SystemTime::now();
                
                info!("Successfully connected to pool: {}", pool.url);
                Ok(())
            }
            Err(e) => {
                pool.status = PoolStatus::Error(e.to_string());
                error!("Failed to connect to pool {}: {}", pool.url, e);
                Err(e)
            }
        }
    }
    
    /// 断开连接
    pub async fn disconnect(&self) -> Result<(), PoolError> {
        let mut pool = self.pool.write().await;
        
        info!("Disconnecting from pool: {}", pool.url);
        
        *self.connected.write().await = false;
        pool.status = PoolStatus::Disconnected;
        pool.connected_at = None;
        
        info!("Disconnected from pool: {}", pool.url);
        Ok(())
    }
    
    /// 检查连接状态
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
    
    /// 获取连接状态
    pub async fn get_status(&self) -> PoolStatus {
        let pool = self.pool.read().await;
        pool.status.clone()
    }
    
    /// 更新活动时间
    pub async fn update_activity(&self) {
        *self.last_activity.write().await = SystemTime::now();
    }
    
    /// 获取最后活动时间
    pub async fn get_last_activity(&self) -> SystemTime {
        *self.last_activity.read().await
    }
    
    /// 检查连接是否超时
    pub async fn is_timeout(&self, timeout: Duration) -> bool {
        let last_activity = *self.last_activity.read().await;
        SystemTime::now().duration_since(last_activity).unwrap_or(Duration::from_secs(0)) > timeout
    }
    
    /// 启动自动重连
    pub async fn start_auto_reconnect(&self) -> Result<(), PoolError> {
        let connected = self.connected.clone();
        let pool = self.pool.clone();
        let connection_attempts = self.connection_attempts.clone();
        let max_attempts = self.max_reconnect_attempts;
        let interval_duration = self.reconnect_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(interval_duration);
            
            loop {
                interval.tick().await;
                
                if !*connected.read().await {
                    let attempts = *connection_attempts.read().await;
                    
                    if attempts < max_attempts {
                        let pool_url = {
                            let pool = pool.read().await;
                            pool.url.clone()
                        };
                        
                        info!("Attempting to reconnect to pool: {} (attempt {})", pool_url, attempts + 1);
                        
                        // 这里应该调用实际的连接逻辑
                        // 为了简化，我们只是模拟重连
                        match Self::simulate_reconnect().await {
                            Ok(_) => {
                                *connected.write().await = true;
                                let mut pool = pool.write().await;
                                pool.status = PoolStatus::Connected;
                                pool.connected_at = Some(SystemTime::now());
                                
                                info!("Successfully reconnected to pool: {}", pool_url);
                                *connection_attempts.write().await = 0; // 重置计数器
                            }
                            Err(e) => {
                                warn!("Reconnection failed for pool {}: {}", pool_url, e);
                                *connection_attempts.write().await += 1;
                            }
                        }
                    } else {
                        error!("Max reconnection attempts reached for pool, giving up");
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 启动心跳检测
    pub async fn start_heartbeat(&self) -> Result<(), PoolError> {
        let connected = self.connected.clone();
        let last_activity = self.last_activity.clone();
        let pool = self.pool.clone();
        let heartbeat_interval = self.heartbeat_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);
            
            loop {
                interval.tick().await;
                
                if *connected.read().await {
                    // 检查是否需要发送心跳
                    let last_activity_time = *last_activity.read().await;
                    let elapsed = SystemTime::now()
                        .duration_since(last_activity_time)
                        .unwrap_or(Duration::from_secs(0));
                    
                    if elapsed > heartbeat_interval {
                        let pool_url = {
                            let pool = pool.read().await;
                            pool.url.clone()
                        };
                        
                        debug!("Sending heartbeat to pool: {}", pool_url);
                        
                        // 这里应该发送实际的心跳消息
                        match Self::send_heartbeat().await {
                            Ok(_) => {
                                *last_activity.write().await = SystemTime::now();
                                debug!("Heartbeat sent successfully to pool: {}", pool_url);
                            }
                            Err(e) => {
                                warn!("Heartbeat failed for pool {}: {}", pool_url, e);
                                *connected.write().await = false;
                                
                                let mut pool = pool.write().await;
                                pool.status = PoolStatus::Error(format!("Heartbeat failed: {}", e));
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 获取连接统计信息
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let pool = self.pool.read().await;
        let connected = *self.connected.read().await;
        let last_activity = *self.last_activity.read().await;
        let connection_attempts = *self.connection_attempts.read().await;
        
        let uptime = if connected {
            pool.connected_at
                .map(|connected_at| {
                    SystemTime::now()
                        .duration_since(connected_at)
                        .unwrap_or(Duration::from_secs(0))
                })
                .unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        };
        
        ConnectionStats {
            pool_id: pool.id,
            url: pool.url.clone(),
            connected,
            status: pool.status.clone(),
            uptime,
            last_activity,
            connection_attempts,
            ping: pool.ping,
        }
    }
    
    /// 尝试连接（模拟）
    async fn attempt_connection(&self) -> Result<(), PoolError> {
        // 模拟连接延迟
        sleep(Duration::from_millis(100)).await;
        
        // 模拟连接成功/失败
        if fastrand::f32() > 0.1 { // 90% 成功率
            Ok(())
        } else {
            Err(PoolError::ConnectionFailed {
                url: "simulated".to_string(),
                error: "Connection timeout".to_string(),
            })
        }
    }
    
    /// 模拟重连
    async fn simulate_reconnect() -> Result<(), PoolError> {
        sleep(Duration::from_millis(50)).await;
        
        if fastrand::f32() > 0.3 { // 70% 成功率
            Ok(())
        } else {
            Err(PoolError::ConnectionFailed {
                url: "simulated".to_string(),
                error: "Reconnection failed".to_string(),
            })
        }
    }
    
    /// 发送心跳（模拟）
    async fn send_heartbeat() -> Result<(), PoolError> {
        sleep(Duration::from_millis(10)).await;
        
        if fastrand::f32() > 0.05 { // 95% 成功率
            Ok(())
        } else {
            Err(PoolError::ConnectionFailed {
                url: "simulated".to_string(),
                error: "Heartbeat timeout".to_string(),
            })
        }
    }
    
    /// 设置连接参数
    pub fn set_connection_params(
        &mut self,
        max_reconnect_attempts: u32,
        reconnect_interval: Duration,
        connection_timeout: Duration,
        heartbeat_interval: Duration,
    ) {
        self.max_reconnect_attempts = max_reconnect_attempts;
        self.reconnect_interval = reconnect_interval;
        self.connection_timeout = connection_timeout;
        self.heartbeat_interval = heartbeat_interval;
    }
    
    /// 重置连接尝试计数器
    pub async fn reset_connection_attempts(&self) {
        *self.connection_attempts.write().await = 0;
    }
    
    /// 获取连接尝试次数
    pub async fn get_connection_attempts(&self) -> u32 {
        *self.connection_attempts.read().await
    }
    
    /// 强制重连
    pub async fn force_reconnect(&self) -> Result<(), PoolError> {
        info!("Forcing reconnection");
        
        // 先断开
        self.disconnect().await?;
        
        // 等待一小段时间
        sleep(Duration::from_millis(100)).await;
        
        // 重新连接
        self.connect().await
    }
}

/// 连接统计信息
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub pool_id: u32,
    pub url: String,
    pub connected: bool,
    pub status: PoolStatus,
    pub uptime: Duration,
    pub last_activity: SystemTime,
    pub connection_attempts: u32,
    pub ping: Option<Duration>,
}

impl ConnectionStats {
    /// 获取连接稳定性评分 (0-100)
    pub fn get_stability_score(&self) -> u8 {
        let mut score = 100u8;
        
        // 根据连接尝试次数扣分
        if self.connection_attempts > 0 {
            score = score.saturating_sub(self.connection_attempts as u8 * 10);
        }
        
        // 根据ping延迟扣分
        if let Some(ping) = self.ping {
            if ping > Duration::from_millis(1000) {
                score = score.saturating_sub(20);
            } else if ping > Duration::from_millis(500) {
                score = score.saturating_sub(10);
            }
        }
        
        // 如果未连接，分数为0
        if !self.connected {
            score = 0;
        }
        
        score
    }
    
    /// 检查连接是否健康
    pub fn is_healthy(&self) -> bool {
        self.connected && self.get_stability_score() > 50
    }
}
