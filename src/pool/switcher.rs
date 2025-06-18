//! 智能矿池切换器

use crate::error::PoolError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, error};

/// 矿池切换器
pub struct PoolSwitcher {
    /// 当前活跃矿池
    current_pool: Arc<RwLock<Option<u32>>>,
    /// 矿池性能指标
    pool_metrics: Arc<RwLock<HashMap<u32, PoolMetrics>>>,
    /// 切换配置
    switch_config: SwitchConfig,
    /// 切换历史
    switch_history: Arc<RwLock<Vec<SwitchEvent>>>,
    /// 自动切换任务句柄
    auto_switch_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 切换统计
    switch_stats: Arc<RwLock<SwitchStats>>,
}

/// 矿池性能指标
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    /// 矿池ID
    pub pool_id: u32,
    /// 平均延迟
    pub avg_latency: Duration,
    /// 接受率
    pub accept_rate: f64,
    /// 拒绝率
    pub reject_rate: f64,
    /// 过期率
    pub stale_rate: f64,
    /// 连接稳定性
    pub connection_stability: f64,
    /// 难度
    pub difficulty: f64,
    /// 最后更新时间
    pub last_update: SystemTime,
    /// 性能分数 (综合评分)
    pub performance_score: f64,
}

/// 切换配置
#[derive(Debug, Clone)]
pub struct SwitchConfig {
    /// 是否启用自动切换
    pub auto_switch_enabled: bool,
    /// 切换检查间隔
    pub check_interval: Duration,
    /// 延迟阈值 (ms)
    pub latency_threshold: u64,
    /// 接受率阈值 (%)
    pub accept_rate_threshold: f64,
    /// 稳定性阈值
    pub stability_threshold: f64,
    /// 切换冷却时间
    pub switch_cooldown: Duration,
    /// 最小性能差异 (切换前后性能差异阈值)
    pub min_performance_diff: f64,
}

/// 切换事件
#[derive(Debug, Clone)]
pub struct SwitchEvent {
    /// 事件时间
    pub timestamp: SystemTime,
    /// 从哪个矿池切换
    pub from_pool: Option<u32>,
    /// 切换到哪个矿池
    pub to_pool: u32,
    /// 切换原因
    pub reason: SwitchReason,
    /// 切换耗时
    pub switch_duration: Duration,
    /// 是否成功
    pub success: bool,
}

/// 切换原因
#[derive(Debug, Clone)]
pub enum SwitchReason {
    /// 手动切换
    Manual,
    /// 高延迟
    HighLatency,
    /// 低接受率
    LowAcceptRate,
    /// 连接不稳定
    ConnectionUnstable,
    /// 矿池离线
    PoolOffline,
    /// 性能优化
    PerformanceOptimization,
    /// 负载均衡
    LoadBalancing,
    /// 故障转移
    Failover,
}

/// 切换统计
#[derive(Debug, Clone)]
pub struct SwitchStats {
    /// 总切换次数
    pub total_switches: u64,
    /// 成功切换次数
    pub successful_switches: u64,
    /// 失败切换次数
    pub failed_switches: u64,
    /// 平均切换时间
    pub avg_switch_time: Duration,
    /// 最后切换时间
    pub last_switch_time: Option<SystemTime>,
    /// 按原因分类的切换次数
    pub switches_by_reason: HashMap<String, u64>,
}

impl Default for SwitchConfig {
    fn default() -> Self {
        Self {
            auto_switch_enabled: true,
            check_interval: Duration::from_secs(60), // 每分钟检查一次
            latency_threshold: 1000, // 1秒
            accept_rate_threshold: 95.0, // 95%
            stability_threshold: 0.9, // 90%
            switch_cooldown: Duration::from_secs(300), // 5分钟冷却
            min_performance_diff: 0.1, // 10%性能差异
        }
    }
}

impl PoolSwitcher {
    /// 创建新的矿池切换器
    pub fn new(config: Option<SwitchConfig>) -> Self {
        Self {
            current_pool: Arc::new(RwLock::new(None)),
            pool_metrics: Arc::new(RwLock::new(HashMap::new())),
            switch_config: config.unwrap_or_default(),
            switch_history: Arc::new(RwLock::new(Vec::new())),
            auto_switch_handle: Arc::new(Mutex::new(None)),
            switch_stats: Arc::new(RwLock::new(SwitchStats::new())),
        }
    }

    /// 手动切换到指定矿池
    pub async fn switch_to_pool(&self, pool_id: u32) -> Result<(), PoolError> {
        let start_time = Instant::now();
        let current_pool = *self.current_pool.read().await;

        info!("Manually switching to pool {}", pool_id);

        // 执行切换
        match self.perform_switch(current_pool, pool_id, SwitchReason::Manual).await {
            Ok(_) => {
                let switch_duration = start_time.elapsed();
                self.record_switch_event(current_pool, pool_id, SwitchReason::Manual, switch_duration, true).await;
                info!("Successfully switched to pool {} in {:?}", pool_id, switch_duration);
                Ok(())
            }
            Err(e) => {
                let switch_duration = start_time.elapsed();
                self.record_switch_event(current_pool, pool_id, SwitchReason::Manual, switch_duration, false).await;
                error!("Failed to switch to pool {}: {}", pool_id, e);
                Err(e)
            }
        }
    }

    /// 更新矿池指标
    pub async fn update_pool_metrics(&self, pool_id: u32, metrics: PoolMetrics) {
        self.pool_metrics.write().await.insert(pool_id, metrics);
        debug!("Updated metrics for pool {}", pool_id);
    }

    /// 启动自动切换
    pub async fn start_auto_switch(&self) -> Result<(), PoolError> {
        if !self.switch_config.auto_switch_enabled {
            debug!("Auto switch is disabled");
            return Ok(());
        }

        let pool_metrics = self.pool_metrics.clone();
        let current_pool = self.current_pool.clone();
        let switch_config = self.switch_config.clone();
        let switch_history = self.switch_history.clone();
        let _switch_stats = self.switch_stats.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(switch_config.check_interval);

            loop {
                interval.tick().await;

                // 检查是否需要切换
                if let Some(best_pool) = Self::find_best_pool(&pool_metrics, &switch_config).await {
                    let current = *current_pool.read().await;

                    if let Some(current_id) = current {
                        if current_id != best_pool {
                            // 检查冷却时间
                            if Self::is_switch_allowed(&switch_history, &switch_config).await {
                                info!("Auto switching from pool {} to pool {}", current_id, best_pool);
                                // 这里应该触发实际的切换逻辑
                            }
                        }
                    } else {
                        // 没有当前矿池，直接切换到最佳矿池
                        info!("Auto switching to pool {}", best_pool);
                        *current_pool.write().await = Some(best_pool);
                    }
                }
            }
        });

        *self.auto_switch_handle.lock().await = Some(handle);
        info!("Auto switch started");
        Ok(())
    }

    /// 停止自动切换
    pub async fn stop_auto_switch(&self) -> Result<(), PoolError> {
        if let Some(handle) = self.auto_switch_handle.lock().await.take() {
            handle.abort();
        }
        info!("Auto switch stopped");
        Ok(())
    }

    /// 寻找最佳矿池
    async fn find_best_pool(
        pool_metrics: &Arc<RwLock<HashMap<u32, PoolMetrics>>>,
        config: &SwitchConfig,
    ) -> Option<u32> {
        let metrics = pool_metrics.read().await;
        let mut best_pool = None;
        let mut best_score = 0.0;

        for (pool_id, pool_metrics) in metrics.iter() {
            // 检查矿池是否满足基本要求
            if pool_metrics.avg_latency.as_millis() > config.latency_threshold as u128 {
                continue;
            }
            if pool_metrics.accept_rate < config.accept_rate_threshold {
                continue;
            }
            if pool_metrics.connection_stability < config.stability_threshold {
                continue;
            }

            // 计算综合性能分数
            let score = pool_metrics.performance_score;
            if score > best_score {
                best_score = score;
                best_pool = Some(*pool_id);
            }
        }

        best_pool
    }

    /// 检查是否允许切换 (冷却时间)
    async fn is_switch_allowed(
        switch_history: &Arc<RwLock<Vec<SwitchEvent>>>,
        config: &SwitchConfig,
    ) -> bool {
        let history = switch_history.read().await;

        if let Some(last_switch) = history.last() {
            let elapsed = SystemTime::now()
                .duration_since(last_switch.timestamp)
                .unwrap_or(Duration::from_secs(0));

            elapsed >= config.switch_cooldown
        } else {
            true
        }
    }

    /// 执行切换
    async fn perform_switch(
        &self,
        from_pool: Option<u32>,
        to_pool: u32,
        reason: SwitchReason,
    ) -> Result<(), PoolError> {
        // 更新当前矿池
        *self.current_pool.write().await = Some(to_pool);

        // 这里应该实现实际的矿池切换逻辑
        // 例如：断开当前连接，连接新矿池，更新工作分配等

        debug!("Pool switch completed: {:?} -> {}, reason: {:?}", from_pool, to_pool, reason);
        Ok(())
    }

    /// 记录切换事件
    async fn record_switch_event(
        &self,
        from_pool: Option<u32>,
        to_pool: u32,
        reason: SwitchReason,
        duration: Duration,
        success: bool,
    ) {
        let event = SwitchEvent {
            timestamp: SystemTime::now(),
            from_pool,
            to_pool,
            reason: reason.clone(),
            switch_duration: duration,
            success,
        };

        // 添加到历史记录
        self.switch_history.write().await.push(event);

        // 更新统计
        {
            let mut stats = self.switch_stats.write().await;
            stats.total_switches += 1;
            if success {
                stats.successful_switches += 1;
            } else {
                stats.failed_switches += 1;
            }
            stats.last_switch_time = Some(SystemTime::now());

            // 更新平均切换时间
            let total_time = stats.avg_switch_time.as_millis() as u64 * (stats.total_switches - 1) + duration.as_millis() as u64;
            stats.avg_switch_time = Duration::from_millis(total_time / stats.total_switches);

            // 按原因统计
            let reason_key = format!("{:?}", reason);
            *stats.switches_by_reason.entry(reason_key).or_insert(0) += 1;
        }
    }

    /// 获取当前矿池
    pub async fn get_current_pool(&self) -> Option<u32> {
        *self.current_pool.read().await
    }

    /// 获取切换统计
    pub async fn get_switch_stats(&self) -> SwitchStats {
        self.switch_stats.read().await.clone()
    }

    /// 获取切换历史
    pub async fn get_switch_history(&self, limit: Option<usize>) -> Vec<SwitchEvent> {
        let history = self.switch_history.read().await;
        if let Some(limit) = limit {
            history.iter().rev().take(limit).cloned().collect()
        } else {
            history.clone()
        }
    }
}

impl PoolMetrics {
    /// 计算性能分数
    pub fn calculate_performance_score(&mut self) {
        // 综合评分算法
        let latency_score = 1.0 - (self.avg_latency.as_millis() as f64 / 5000.0).min(1.0); // 5秒为最差
        let accept_score = self.accept_rate / 100.0;
        let stability_score = self.connection_stability;
        let stale_penalty = 1.0 - (self.stale_rate / 100.0);

        // 加权平均
        self.performance_score = (latency_score * 0.3 + accept_score * 0.4 + stability_score * 0.2 + stale_penalty * 0.1).max(0.0).min(1.0);
    }
}

impl SwitchStats {
    pub fn new() -> Self {
        Self {
            total_switches: 0,
            successful_switches: 0,
            failed_switches: 0,
            avg_switch_time: Duration::from_millis(0),
            last_switch_time: None,
            switches_by_reason: HashMap::new(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_switches == 0 {
            0.0
        } else {
            self.successful_switches as f64 / self.total_switches as f64 * 100.0
        }
    }
}
