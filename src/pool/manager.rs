use crate::config::PoolConfig;
use crate::error::PoolError;
use crate::pool::{Pool, PoolStatus, Share, PoolStats, PoolEvent};
use crate::pool::stratum::StratumClient;
use crate::device::Work;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, broadcast};
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// 矿池管理器
pub struct PoolManager {
    /// 矿池列表
    pools: Arc<RwLock<HashMap<u32, Arc<Mutex<Pool>>>>>,
    /// Stratum 客户端
    stratum_clients: Arc<RwLock<HashMap<u32, Arc<Mutex<StratumClient>>>>>,
    /// 矿池统计
    pool_stats: Arc<RwLock<HashMap<u32, PoolStats>>>,
    /// 当前活跃矿池
    active_pool: Arc<RwLock<Option<u32>>>,
    /// 配置
    config: PoolConfig,
    /// 工作接收通道
    work_sender: Arc<Mutex<Option<mpsc::UnboundedSender<Work>>>>,
    /// 份额提交通道
    share_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<Share>>>>,
    /// 事件广播
    event_sender: broadcast::Sender<PoolEvent>,
    /// 连接管理任务句柄
    connection_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 心跳任务句柄
    heartbeat_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl PoolManager {
    /// 创建新的矿池管理器
    pub async fn new(config: PoolConfig) -> Result<Self, PoolError> {
        info!("Creating pool manager with {} pools", config.pools.len());

        let mut pools = HashMap::new();
        let mut stratum_clients = HashMap::new();
        let mut pool_stats = HashMap::new();

        // 初始化矿池
        for (index, pool_info) in config.pools.iter().enumerate() {
            let pool_id = index as u32;
            let pool = Pool::new(
                pool_id,
                pool_info.url.clone(),
                pool_info.user.clone(),
                pool_info.password.clone(),
                pool_info.priority,
            );

            // 创建 Stratum 客户端
            let stratum_client = StratumClient::new(
                pool_info.url.clone(),
                pool_info.user.clone(),
                pool_info.password.clone(),
            ).await?;

            pools.insert(pool_id, Arc::new(Mutex::new(pool)));
            stratum_clients.insert(pool_id, Arc::new(Mutex::new(stratum_client)));
            pool_stats.insert(pool_id, PoolStats::new(pool_id));
        }

        let (work_sender, _) = mpsc::unbounded_channel();
        let (_, share_receiver) = mpsc::unbounded_channel();
        let (event_sender, _) = broadcast::channel(1000);

        Ok(Self {
            pools: Arc::new(RwLock::new(pools)),
            stratum_clients: Arc::new(RwLock::new(stratum_clients)),
            pool_stats: Arc::new(RwLock::new(pool_stats)),
            active_pool: Arc::new(RwLock::new(None)),
            config,
            work_sender: Arc::new(Mutex::new(Some(work_sender))),
            share_receiver: Arc::new(Mutex::new(Some(share_receiver))),
            event_sender,
            connection_handle: Arc::new(Mutex::new(None)),
            heartbeat_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动矿池管理器
    pub async fn start(&self) -> Result<(), PoolError> {
        info!("Starting pool manager");

        *self.running.write().await = true;

        // 连接到矿池
        self.connect_to_pools().await?;

        // 启动连接管理任务
        self.start_connection_management().await?;

        // 启动心跳任务
        self.start_heartbeat().await?;

        info!("Pool manager started successfully");
        Ok(())
    }

    /// 停止矿池管理器
    pub async fn stop(&self) -> Result<(), PoolError> {
        info!("Stopping pool manager");

        *self.running.write().await = false;

        // 停止任务
        if let Some(handle) = self.connection_handle.lock().await.take() {
            handle.abort();
        }

        if let Some(handle) = self.heartbeat_handle.lock().await.take() {
            handle.abort();
        }

        // 断开所有连接
        self.disconnect_all_pools().await?;

        info!("Pool manager stopped successfully");
        Ok(())
    }

    /// 连接到矿池
    pub async fn connect_to_pools(&self) -> Result<(), PoolError> {
        info!("Connecting to pools");

        let pools = self.pools.read().await;
        let stratum_clients = self.stratum_clients.read().await;

        // 根据策略连接矿池
        match self.config.strategy {
            crate::config::PoolStrategy::Failover => {
                // 故障转移：按优先级连接
                self.connect_failover_pools(&pools, &stratum_clients).await?;
            }
            crate::config::PoolStrategy::RoundRobin => {
                // 轮询：连接所有启用的矿池
                self.connect_all_enabled_pools(&pools, &stratum_clients).await?;
            }
            crate::config::PoolStrategy::LoadBalance => {
                // 负载均衡：连接所有启用的矿池
                self.connect_all_enabled_pools(&pools, &stratum_clients).await?;
            }
            crate::config::PoolStrategy::Quota => {
                // 配额：连接所有启用的矿池
                self.connect_all_enabled_pools(&pools, &stratum_clients).await?;
            }
        }

        Ok(())
    }

    /// 故障转移连接
    async fn connect_failover_pools(
        &self,
        pools: &HashMap<u32, Arc<Mutex<Pool>>>,
        stratum_clients: &HashMap<u32, Arc<Mutex<StratumClient>>>,
    ) -> Result<(), PoolError> {
        // 按优先级排序
        let mut pool_priorities: Vec<(u32, u8)> = pools
            .iter()
            .map(|(id, pool)| {
                let priority = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        pool.lock().await.priority
                    })
                });
                (*id, priority)
            })
            .collect();

        pool_priorities.sort_by_key(|(_, priority)| *priority);

        // 尝试连接最高优先级的矿池
        for (pool_id, _) in pool_priorities {
            if let Some(stratum_client) = stratum_clients.get(&pool_id) {
                match self.connect_single_pool(pool_id, stratum_client.clone()).await {
                    Ok(_) => {
                        *self.active_pool.write().await = Some(pool_id);
                        info!("Connected to primary pool {}", pool_id);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to connect to pool {}: {}", pool_id, e);
                        continue;
                    }
                }
            }
        }

        Err(PoolError::NoPoolsAvailable)
    }

    /// 连接所有启用的矿池
    async fn connect_all_enabled_pools(
        &self,
        pools: &HashMap<u32, Arc<Mutex<Pool>>>,
        stratum_clients: &HashMap<u32, Arc<Mutex<StratumClient>>>,
    ) -> Result<(), PoolError> {
        let mut connected_count = 0;

        for (pool_id, pool) in pools.iter() {
            let enabled = pool.lock().await.enabled;
            if enabled {
                if let Some(stratum_client) = stratum_clients.get(pool_id) {
                    match self.connect_single_pool(*pool_id, stratum_client.clone()).await {
                        Ok(_) => {
                            connected_count += 1;
                            if self.active_pool.read().await.is_none() {
                                *self.active_pool.write().await = Some(*pool_id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to connect to pool {}: {}", pool_id, e);
                        }
                    }
                }
            }
        }

        if connected_count == 0 {
            Err(PoolError::NoPoolsAvailable)
        } else {
            info!("Connected to {} pools", connected_count);
            Ok(())
        }
    }

    /// 连接单个矿池
    async fn connect_single_pool(
        &self,
        pool_id: u32,
        stratum_client: Arc<Mutex<StratumClient>>,
    ) -> Result<(), PoolError> {
        info!("Connecting to pool {}", pool_id);

        // 更新矿池状态
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&pool_id) {
                let mut pool = pool.lock().await;
                pool.status = PoolStatus::Connecting;
            }
        }

        // 发送连接事件
        self.send_event(PoolEvent::ConnectionChanged {
            pool_id,
            old_status: PoolStatus::Disconnected,
            new_status: PoolStatus::Connecting,
            timestamp: SystemTime::now(),
        }).await;

        // 连接到矿池
        {
            let mut client = stratum_client.lock().await;
            client.connect().await?;
        }

        // 更新矿池状态
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&pool_id) {
                let mut pool = pool.lock().await;
                pool.status = PoolStatus::Connected;
                pool.connected_at = Some(SystemTime::now());
            }
        }

        // 发送连接事件
        self.send_event(PoolEvent::ConnectionChanged {
            pool_id,
            old_status: PoolStatus::Connecting,
            new_status: PoolStatus::Connected,
            timestamp: SystemTime::now(),
        }).await;

        // 更新统计
        {
            let mut stats = self.pool_stats.write().await;
            if let Some(pool_stats) = stats.get_mut(&pool_id) {
                pool_stats.record_connection_attempt();
            }
        }

        info!("Successfully connected to pool {}", pool_id);
        Ok(())
    }

    /// 断开所有矿池连接
    async fn disconnect_all_pools(&self) -> Result<(), PoolError> {
        info!("Disconnecting from all pools");

        let stratum_clients = self.stratum_clients.read().await;

        for (pool_id, stratum_client) in stratum_clients.iter() {
            if let Err(e) = self.disconnect_single_pool(*pool_id, stratum_client.clone()).await {
                warn!("Failed to disconnect from pool {}: {}", pool_id, e);
            }
        }

        *self.active_pool.write().await = None;
        Ok(())
    }

    /// 断开单个矿池连接
    async fn disconnect_single_pool(
        &self,
        pool_id: u32,
        stratum_client: Arc<Mutex<StratumClient>>,
    ) -> Result<(), PoolError> {
        info!("Disconnecting from pool {}", pool_id);

        // 断开连接
        {
            let mut client = stratum_client.lock().await;
            client.disconnect().await?;
        }

        // 更新矿池状态
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&pool_id) {
                let mut pool = pool.lock().await;
                pool.status = PoolStatus::Disconnected;
                pool.connected_at = None;
            }
        }

        // 发送断开事件
        self.send_event(PoolEvent::ConnectionChanged {
            pool_id,
            old_status: PoolStatus::Connected,
            new_status: PoolStatus::Disconnected,
            timestamp: SystemTime::now(),
        }).await;

        // 更新统计
        {
            let mut stats = self.pool_stats.write().await;
            if let Some(pool_stats) = stats.get_mut(&pool_id) {
                pool_stats.record_disconnection();
            }
        }

        Ok(())
    }

    /// 提交份额
    pub async fn submit_share(&self, share: Share) -> Result<(), PoolError> {
        let active_pool_id = self.active_pool.read().await;

        if let Some(pool_id) = *active_pool_id {
            let stratum_clients = self.stratum_clients.read().await;
            if let Some(stratum_client) = stratum_clients.get(&pool_id) {
                let mut client = stratum_client.lock().await;

                // 发送份额提交事件
                self.send_event(PoolEvent::ShareSubmitted {
                    pool_id,
                    share: share.clone(),
                    timestamp: SystemTime::now(),
                }).await;

                // 提交份额
                match client.submit_share(&share).await {
                    Ok(accepted) => {
                        // 更新矿池统计
                        {
                            let pools = self.pools.read().await;
                            if let Some(pool) = pools.get(&pool_id) {
                                let mut pool = pool.lock().await;
                                if accepted {
                                    pool.record_accepted_share(share.difficulty);
                                } else {
                                    pool.record_rejected_share();
                                }
                            }
                        }

                        // 发送份额响应事件
                        self.send_event(PoolEvent::ShareResponse {
                            pool_id,
                            share_id: share.id,
                            accepted,
                            reason: if accepted { None } else { Some("Rejected".to_string()) },
                            timestamp: SystemTime::now(),
                        }).await;

                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to submit share to pool {}: {}", pool_id, e);
                        Err(e)
                    }
                }
            } else {
                Err(PoolError::NoPoolsAvailable)
            }
        } else {
            Err(PoolError::NoPoolsAvailable)
        }
    }

    /// 获取工作
    pub async fn get_work(&self) -> Result<Work, PoolError> {
        let active_pool_id = self.active_pool.read().await;

        if let Some(pool_id) = *active_pool_id {
            let stratum_clients = self.stratum_clients.read().await;
            if let Some(stratum_client) = stratum_clients.get(&pool_id) {
                let mut client = stratum_client.lock().await;

                match client.get_work().await {
                    Ok(work) => {
                        // 发送工作接收事件
                        self.send_event(PoolEvent::WorkReceived {
                            pool_id,
                            work: work.clone(),
                            timestamp: SystemTime::now(),
                        }).await;

                        Ok(work)
                    }
                    Err(e) => {
                        error!("Failed to get work from pool {}: {}", pool_id, e);
                        Err(e)
                    }
                }
            } else {
                Err(PoolError::NoPoolsAvailable)
            }
        } else {
            Err(PoolError::NoPoolsAvailable)
        }
    }

    /// 获取连接的矿池数量
    pub async fn get_connected_pool_count(&self) -> u32 {
        let pools = self.pools.read().await;
        let mut count = 0;

        for pool in pools.values() {
            if pool.lock().await.is_connected() {
                count += 1;
            }
        }

        count
    }

    /// 获取矿池统计
    pub async fn get_pool_stats(&self, pool_id: u32) -> Option<PoolStats> {
        let stats = self.pool_stats.read().await;
        stats.get(&pool_id).cloned()
    }

    /// 订阅事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<PoolEvent> {
        self.event_sender.subscribe()
    }

    /// 发送事件
    async fn send_event(&self, event: PoolEvent) {
        if let Err(e) = self.event_sender.send(event) {
            debug!("Failed to send pool event: {}", e);
        }
    }

    /// 启动连接管理任务
    async fn start_connection_management(&self) -> Result<(), PoolError> {
        let running = self.running.clone();
        let pools = self.pools.clone();
        let stratum_clients = self.stratum_clients.clone();
        let active_pool = self.active_pool.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.retry_interval));

            while *running.read().await {
                interval.tick().await;

                // 检查连接状态并重连
                // 这里可以添加连接检查和重连逻辑
            }
        });

        *self.connection_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动心跳任务
    async fn start_heartbeat(&self) -> Result<(), PoolError> {
        let running = self.running.clone();
        let stratum_clients = self.stratum_clients.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            while *running.read().await {
                interval.tick().await;

                // 发送心跳到所有连接的矿池
                let clients = stratum_clients.read().await;
                for (pool_id, client) in clients.iter() {
                    if let Ok(mut client) = client.try_lock() {
                        if let Err(e) = client.ping().await {
                            warn!("Heartbeat failed for pool {}: {}", pool_id, e);
                        }
                    }
                }
            }
        });

        *self.heartbeat_handle.lock().await = Some(handle);
        Ok(())
    }
}
