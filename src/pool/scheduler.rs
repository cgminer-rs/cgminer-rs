//! 矿池调度器 - 智能多矿池管理

use crate::pool::{Pool, PoolStrategy};
use crate::error::PoolError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, debug};

/// 矿池调度器
pub struct PoolScheduler {
    /// 矿池列表
    pools: Arc<RwLock<HashMap<u32, Arc<Mutex<Pool>>>>>,
    /// 调度策略
    strategy: PoolStrategy,
    /// 当前活跃矿池
    active_pools: Arc<RwLock<Vec<u32>>>,
    /// 矿池权重 (用于负载均衡)
    pool_weights: Arc<RwLock<HashMap<u32, f64>>>,
    /// 矿池配额 (用于配额策略)
    pool_quotas: Arc<RwLock<HashMap<u32, PoolQuota>>>,
    /// 轮询索引 (用于轮询策略)
    round_robin_index: Arc<Mutex<usize>>,
    /// 故障转移配置
    failover_config: FailoverConfig,
    /// 调度统计
    scheduler_stats: Arc<RwLock<SchedulerStats>>,
    /// 矿池健康检查
    health_checker: Arc<PoolHealthChecker>,
}

/// 矿池配额信息
#[derive(Debug, Clone)]
pub struct PoolQuota {
    /// 配额百分比
    pub percentage: f64,
    /// 已使用的份额数
    pub used_shares: u64,
    /// 总分配的份额数
    pub allocated_shares: u64,
    /// 重置时间
    pub reset_time: SystemTime,
}

/// 故障转移配置
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// 故障检测超时
    pub failure_timeout: Duration,
    /// 重试间隔
    pub retry_interval: Duration,
    /// 最大重试次数
    pub max_retries: u32,
    /// 自动恢复
    pub auto_recovery: bool,
}

/// 调度统计
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// 总调度次数
    pub total_schedules: u64,
    /// 故障转移次数
    pub failover_count: u64,
    /// 平均响应时间
    pub avg_response_time: Duration,
    /// 最后调度时间
    pub last_schedule_time: SystemTime,
    /// 矿池使用统计
    pub pool_usage: HashMap<u32, PoolUsageStats>,
}

/// 矿池使用统计
#[derive(Debug, Clone)]
pub struct PoolUsageStats {
    /// 使用次数
    pub usage_count: u64,
    /// 总工作时间
    pub total_work_time: Duration,
    /// 成功率
    pub success_rate: f64,
    /// 平均延迟
    pub avg_latency: Duration,
}

/// 矿池健康检查器
pub struct PoolHealthChecker {
    /// 健康状态
    health_status: Arc<RwLock<HashMap<u32, PoolHealth>>>,
    /// 检查间隔
    check_interval: Duration,
    /// 检查任务句柄
    check_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// 矿池健康状态
#[derive(Debug, Clone)]
pub struct PoolHealth {
    /// 是否健康
    pub is_healthy: bool,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最后检查时间
    pub last_check: SystemTime,
    /// 平均延迟
    pub avg_latency: Duration,
    /// 错误率
    pub error_rate: f64,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            failure_timeout: Duration::from_secs(30),
            retry_interval: Duration::from_secs(10),
            max_retries: 3,
            auto_recovery: true,
        }
    }
}

impl PoolScheduler {
    /// 创建新的矿池调度器
    pub fn new(strategy: PoolStrategy, failover_config: Option<FailoverConfig>) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            active_pools: Arc::new(RwLock::new(Vec::new())),
            pool_weights: Arc::new(RwLock::new(HashMap::new())),
            pool_quotas: Arc::new(RwLock::new(HashMap::new())),
            round_robin_index: Arc::new(Mutex::new(0)),
            failover_config: failover_config.unwrap_or_default(),
            scheduler_stats: Arc::new(RwLock::new(SchedulerStats::new())),
            health_checker: Arc::new(PoolHealthChecker::new(Duration::from_secs(30))),
        }
    }

    /// 添加矿池
    pub async fn add_pool(&self, pool: Pool) -> Result<(), PoolError> {
        let pool_id = pool.id;
        info!("Adding pool {} to scheduler", pool_id);

        // 添加到矿池列表
        self.pools.write().await.insert(pool_id, Arc::new(Mutex::new(pool)));

        // 初始化权重
        self.pool_weights.write().await.insert(pool_id, 1.0);

        // 初始化健康状态
        self.health_checker.add_pool(pool_id).await;

        // 初始化使用统计
        {
            let mut stats = self.scheduler_stats.write().await;
            stats.pool_usage.insert(pool_id, PoolUsageStats::new());
        }

        debug!("Pool {} added to scheduler successfully", pool_id);
        Ok(())
    }

    /// 移除矿池
    pub async fn remove_pool(&self, pool_id: u32) -> Result<(), PoolError> {
        info!("Removing pool {} from scheduler", pool_id);

        // 从活跃列表中移除
        {
            let mut active = self.active_pools.write().await;
            active.retain(|&id| id != pool_id);
        }

        // 从矿池列表中移除
        self.pools.write().await.remove(&pool_id);
        self.pool_weights.write().await.remove(&pool_id);
        self.pool_quotas.write().await.remove(&pool_id);

        // 从健康检查中移除
        self.health_checker.remove_pool(pool_id).await;

        debug!("Pool {} removed from scheduler successfully", pool_id);
        Ok(())
    }

    /// 选择下一个矿池进行工作分配
    pub async fn select_pool_for_work(&self) -> Result<u32, PoolError> {
        let start_time = Instant::now();

        let selected_pool = match self.strategy {
            PoolStrategy::Failover => self.select_failover_pool().await?,
            PoolStrategy::RoundRobin => self.select_round_robin_pool().await?,
            PoolStrategy::LoadBalance => self.select_load_balanced_pool().await?,
            PoolStrategy::Quota => self.select_quota_pool().await?,
        };

        // 更新统计
        {
            let mut stats = self.scheduler_stats.write().await;
            stats.total_schedules += 1;
            stats.last_schedule_time = SystemTime::now();

            if let Some(pool_stats) = stats.pool_usage.get_mut(&selected_pool) {
                pool_stats.usage_count += 1;
            }
        }

        let elapsed = start_time.elapsed();
        debug!("Pool {} selected for work in {:?}", selected_pool, elapsed);

        Ok(selected_pool)
    }

    /// 故障转移池选择
    async fn select_failover_pool(&self) -> Result<u32, PoolError> {
        let pools = self.pools.read().await;
        let health_status = self.health_checker.health_status.read().await;

        // 按优先级排序，选择第一个健康的矿池
        let mut pool_priorities: Vec<(u32, u8)> = Vec::new();

        for (id, pool) in pools.iter() {
            let pool_guard = pool.lock().await;
            if pool_guard.enabled && pool_guard.is_connected() {
                if let Some(health) = health_status.get(id) {
                    if health.is_healthy {
                        pool_priorities.push((*id, pool_guard.priority));
                    }
                }
            }
        }

        if pool_priorities.is_empty() {
            return Err(PoolError::NoPoolsAvailable);
        }

        pool_priorities.sort_by_key(|(_, priority)| *priority);
        Ok(pool_priorities[0].0)
    }

    /// 轮询池选择
    async fn select_round_robin_pool(&self) -> Result<u32, PoolError> {
        let active_pools = self.active_pools.read().await;

        if active_pools.is_empty() {
            return Err(PoolError::NoPoolsAvailable);
        }

        let mut index = self.round_robin_index.lock().await;
        let selected_index = *index % active_pools.len();
        *index = (*index + 1) % active_pools.len();

        Ok(active_pools[selected_index])
    }

    /// 负载均衡池选择
    async fn select_load_balanced_pool(&self) -> Result<u32, PoolError> {
        let active_pools = self.active_pools.read().await;
        let weights = self.pool_weights.read().await;
        let stats = self.scheduler_stats.read().await;

        if active_pools.is_empty() {
            return Err(PoolError::NoPoolsAvailable);
        }

        // 基于权重和当前负载选择矿池
        let mut best_pool = active_pools[0];
        let mut best_score = f64::MIN;

        for &pool_id in active_pools.iter() {
            let weight = weights.get(&pool_id).copied().unwrap_or(1.0);
            let usage_stats = stats.pool_usage.get(&pool_id);

            // 计算负载分数 (权重 / 使用次数)
            let load_factor = if let Some(usage) = usage_stats {
                if usage.usage_count > 0 {
                    weight / (usage.usage_count as f64)
                } else {
                    weight
                }
            } else {
                weight
            };

            if load_factor > best_score {
                best_score = load_factor;
                best_pool = pool_id;
            }
        }

        Ok(best_pool)
    }

    /// 配额池选择
    async fn select_quota_pool(&self) -> Result<u32, PoolError> {
        let active_pools = self.active_pools.read().await;
        let quotas = self.pool_quotas.read().await;

        if active_pools.is_empty() {
            return Err(PoolError::NoPoolsAvailable);
        }

        // 选择配额未用完的矿池
        for &pool_id in active_pools.iter() {
            if let Some(quota) = quotas.get(&pool_id) {
                if quota.used_shares < quota.allocated_shares {
                    return Ok(pool_id);
                }
            }
        }

        // 如果所有配额都用完，选择第一个活跃矿池
        Ok(active_pools[0])
    }

    /// 处理矿池故障
    pub async fn handle_pool_failure(&self, pool_id: u32, error: &PoolError) {
        warn!("Pool {} failed: {}", pool_id, error);

        // 更新健康状态
        self.health_checker.mark_failure(pool_id).await;

        // 从活跃列表中移除
        {
            let mut active = self.active_pools.write().await;
            active.retain(|&id| id != pool_id);
        }

        // 更新统计
        {
            let mut stats = self.scheduler_stats.write().await;
            stats.failover_count += 1;
        }

        // 如果启用自动恢复，尝试故障转移
        if self.failover_config.auto_recovery {
            self.attempt_failover().await;
        }
    }

    /// 尝试故障转移
    async fn attempt_failover(&self) {
        info!("Attempting failover to backup pools");

        let pools = self.pools.read().await;
        let health_status = self.health_checker.health_status.read().await;

        // 寻找健康的备用矿池
        for (pool_id, pool) in pools.iter() {
            let pool_guard = pool.lock().await;
            if pool_guard.enabled && !pool_guard.is_connected() {
                if let Some(health) = health_status.get(pool_id) {
                    if health.is_healthy || health.consecutive_failures < self.failover_config.max_retries {
                        info!("Attempting to activate backup pool {}", pool_id);
                        // 这里应该触发矿池连接逻辑
                        break;
                    }
                }
            }
        }
    }

    /// 设置矿池权重
    pub async fn set_pool_weight(&self, pool_id: u32, weight: f64) {
        self.pool_weights.write().await.insert(pool_id, weight);
        debug!("Set pool {} weight to {}", pool_id, weight);
    }

    /// 设置矿池配额
    pub async fn set_pool_quota(&self, pool_id: u32, percentage: f64) {
        let quota = PoolQuota {
            percentage,
            used_shares: 0,
            allocated_shares: (percentage * 1000.0) as u64, // 假设总共1000份额
            reset_time: SystemTime::now(),
        };

        self.pool_quotas.write().await.insert(pool_id, quota);
        debug!("Set pool {} quota to {}%", pool_id, percentage);
    }

    /// 获取调度统计
    pub async fn get_scheduler_stats(&self) -> SchedulerStats {
        self.scheduler_stats.read().await.clone()
    }

    /// 启动健康检查
    pub async fn start_health_check(&self) -> Result<(), PoolError> {
        self.health_checker.start().await
    }

    /// 停止健康检查
    pub async fn stop_health_check(&self) -> Result<(), PoolError> {
        self.health_checker.stop().await
    }
}

impl SchedulerStats {
    pub fn new() -> Self {
        Self {
            total_schedules: 0,
            failover_count: 0,
            avg_response_time: Duration::from_millis(0),
            last_schedule_time: SystemTime::now(),
            pool_usage: HashMap::new(),
        }
    }
}

impl PoolUsageStats {
    pub fn new() -> Self {
        Self {
            usage_count: 0,
            total_work_time: Duration::from_secs(0),
            success_rate: 100.0,
            avg_latency: Duration::from_millis(0),
        }
    }
}

impl PoolHealthChecker {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            check_interval,
            check_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn add_pool(&self, pool_id: u32) {
        let health = PoolHealth {
            is_healthy: true,
            consecutive_failures: 0,
            last_check: SystemTime::now(),
            avg_latency: Duration::from_millis(0),
            error_rate: 0.0,
        };

        self.health_status.write().await.insert(pool_id, health);
        debug!("Added pool {} to health checker", pool_id);
    }

    pub async fn remove_pool(&self, pool_id: u32) {
        self.health_status.write().await.remove(&pool_id);
        debug!("Removed pool {} from health checker", pool_id);
    }

    pub async fn mark_failure(&self, pool_id: u32) {
        let mut health_status = self.health_status.write().await;
        if let Some(health) = health_status.get_mut(&pool_id) {
            health.consecutive_failures += 1;
            health.is_healthy = health.consecutive_failures < 3; // 连续3次失败后标记为不健康
            health.last_check = SystemTime::now();
        }
    }

    pub async fn mark_success(&self, pool_id: u32, latency: Duration) {
        let mut health_status = self.health_status.write().await;
        if let Some(health) = health_status.get_mut(&pool_id) {
            health.consecutive_failures = 0;
            health.is_healthy = true;
            health.avg_latency = latency;
            health.last_check = SystemTime::now();
        }
    }

    pub async fn start(&self) -> Result<(), PoolError> {
        let health_status = self.health_status.clone();
        let check_interval = self.check_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                // 执行健康检查
                let pools: Vec<u32> = {
                    health_status.read().await.keys().copied().collect()
                };

                for pool_id in pools {
                    // 这里应该实现实际的健康检查逻辑
                    // 例如发送ping请求或检查连接状态
                    debug!("Health check for pool {}", pool_id);
                }
            }
        });

        *self.check_handle.lock().await = Some(handle);
        info!("Pool health checker started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), PoolError> {
        if let Some(handle) = self.check_handle.lock().await.take() {
            handle.abort();
        }
        info!("Pool health checker stopped");
        Ok(())
    }
}
