use crate::config::Config;
use crate::error::MiningError;
use crate::device::{DeviceManager, Work, MiningResult};
use crate::pool::PoolManager;
use crate::monitoring::MonitoringSystem;
use crate::mining::{MiningState, MiningStats, MiningConfig, MiningEvent, WorkItem, ResultItem};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, broadcast};
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// 挖矿管理器 - 协调所有子系统
pub struct MiningManager {
    /// 设备管理器
    device_manager: Arc<Mutex<DeviceManager>>,
    /// 矿池管理器
    pool_manager: Arc<Mutex<PoolManager>>,
    /// 监控系统
    monitoring_system: Arc<Mutex<MonitoringSystem>>,
    /// 挖矿配置
    config: MiningConfig,
    /// 挖矿状态
    state: Arc<RwLock<MiningState>>,
    /// 挖矿统计
    stats: Arc<RwLock<MiningStats>>,
    /// 工作分发通道
    work_sender: Arc<Mutex<Option<mpsc::UnboundedSender<WorkItem>>>>,
    work_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<WorkItem>>>>,
    /// 结果收集通道
    result_sender: Arc<Mutex<Option<mpsc::UnboundedSender<ResultItem>>>>,
    result_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<ResultItem>>>>,
    /// 事件广播
    event_sender: broadcast::Sender<MiningEvent>,
    /// 主循环任务句柄
    main_loop_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 工作分发任务句柄
    work_dispatch_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 结果处理任务句柄
    result_process_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl MiningManager {
    /// 创建新的挖矿管理器
    pub async fn new(config: Config) -> Result<Self, MiningError> {
        info!("Creating mining manager");
        
        // 创建设备管理器
        let mut device_manager = DeviceManager::new(config.devices.clone());
        
        // 注册 Maijie L7 驱动
        let maijie_driver = Box::new(crate::device::maijie_l7::MaijieL7Driver::new());
        device_manager.register_driver(maijie_driver);
        
        // 创建矿池管理器
        let pool_manager = PoolManager::new(config.pools.clone()).await?;
        
        // 创建监控系统
        let monitoring_system = MonitoringSystem::new(config.monitoring.clone()).await?;
        
        // 创建通道
        let (work_sender, work_receiver) = mpsc::unbounded_channel();
        let (result_sender, result_receiver) = mpsc::unbounded_channel();
        let (event_sender, _) = broadcast::channel(1000);
        
        let mining_config = MiningConfig::from(&config);
        
        Ok(Self {
            device_manager: Arc::new(Mutex::new(device_manager)),
            pool_manager: Arc::new(Mutex::new(pool_manager)),
            monitoring_system: Arc::new(Mutex::new(monitoring_system)),
            config: mining_config,
            state: Arc::new(RwLock::new(MiningState::Stopped)),
            stats: Arc::new(RwLock::new(MiningStats::new())),
            work_sender: Arc::new(Mutex::new(Some(work_sender))),
            work_receiver: Arc::new(Mutex::new(Some(work_receiver))),
            result_sender: Arc::new(Mutex::new(Some(result_sender))),
            result_receiver: Arc::new(Mutex::new(Some(result_receiver))),
            event_sender,
            main_loop_handle: Arc::new(Mutex::new(None)),
            work_dispatch_handle: Arc::new(Mutex::new(None)),
            result_process_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }
    
    /// 启动挖矿
    pub async fn start(&self) -> Result<(), MiningError> {
        info!("Starting mining manager");
        
        // 检查是否已经在运行
        if *self.running.read().await {
            warn!("Mining manager is already running");
            return Ok(());
        }
        
        // 更新状态
        *self.state.write().await = MiningState::Starting;
        *self.running.write().await = true;
        
        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Stopped,
            new_state: MiningState::Starting,
            timestamp: SystemTime::now(),
        }).await;
        
        // 初始化设备管理器
        {
            let mut device_manager = self.device_manager.lock().await;
            device_manager.initialize().await?;
            device_manager.start().await?;
        }
        
        // 启动矿池管理器
        {
            let mut pool_manager = self.pool_manager.lock().await;
            pool_manager.start().await?;
        }
        
        // 启动监控系统
        {
            let mut monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.start().await?;
        }
        
        // 启动各个任务
        self.start_main_loop().await?;
        self.start_work_dispatch().await?;
        self.start_result_processing().await?;
        
        // 更新状态和统计
        *self.state.write().await = MiningState::Running;
        self.stats.write().await.start();
        
        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Starting,
            new_state: MiningState::Running,
            timestamp: SystemTime::now(),
        }).await;
        
        info!("Mining manager started successfully");
        Ok(())
    }
    
    /// 停止挖矿
    pub async fn stop(&self) -> Result<(), MiningError> {
        info!("Stopping mining manager");
        
        // 检查是否已经停止
        if !*self.running.read().await {
            warn!("Mining manager is already stopped");
            return Ok(());
        }
        
        // 更新状态
        *self.state.write().await = MiningState::Stopping;
        *self.running.write().await = false;
        
        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Running,
            new_state: MiningState::Stopping,
            timestamp: SystemTime::now(),
        }).await;
        
        // 停止各个任务
        self.stop_tasks().await;
        
        // 停止监控系统
        {
            let mut monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.stop().await?;
        }
        
        // 停止矿池管理器
        {
            let mut pool_manager = self.pool_manager.lock().await;
            pool_manager.stop().await?;
        }
        
        // 停止设备管理器
        {
            let mut device_manager = self.device_manager.lock().await;
            device_manager.stop().await?;
        }
        
        // 更新状态
        *self.state.write().await = MiningState::Stopped;
        
        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Stopping,
            new_state: MiningState::Stopped,
            timestamp: SystemTime::now(),
        }).await;
        
        info!("Mining manager stopped successfully");
        Ok(())
    }
    
    /// 获取挖矿状态
    pub async fn get_state(&self) -> MiningState {
        self.state.read().await.clone()
    }
    
    /// 获取挖矿统计
    pub async fn get_stats(&self) -> MiningStats {
        let mut stats = self.stats.write().await;
        stats.update_uptime();
        
        // 更新当前算力
        if let Ok(device_manager) = self.device_manager.try_lock() {
            let hashrate = device_manager.get_total_hashrate().await;
            stats.update_hashrate(hashrate);
        }
        
        stats.clone()
    }
    
    /// 获取系统状态
    pub async fn get_system_status(&self) -> SystemStatus {
        let stats = self.get_stats().await;
        let device_manager = self.device_manager.lock().await;
        let pool_manager = self.pool_manager.lock().await;
        
        SystemStatus {
            state: self.get_state().await,
            uptime: stats.uptime,
            total_hashrate: stats.current_hashrate,
            accepted_shares: stats.accepted_shares,
            rejected_shares: stats.rejected_shares,
            hardware_errors: stats.hardware_errors,
            active_devices: device_manager.get_active_device_count().await,
            connected_pools: pool_manager.get_connected_pool_count().await,
            current_difficulty: stats.current_difficulty,
            best_share: stats.best_share,
            efficiency: stats.efficiency,
            power_consumption: stats.power_consumption,
        }
    }
    
    /// 订阅事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<MiningEvent> {
        self.event_sender.subscribe()
    }
    
    /// 发送事件
    async fn send_event(&self, event: MiningEvent) {
        if let Err(e) = self.event_sender.send(event) {
            debug!("Failed to send mining event: {}", e);
        }
    }
    
    /// 启动主循环
    async fn start_main_loop(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let stats = self.stats.clone();
        let device_manager = self.device_manager.clone();
        let pool_manager = self.pool_manager.clone();
        let monitoring_system = self.monitoring_system.clone();
        let event_sender = self.event_sender.clone();
        let scan_interval = self.config.scan_interval;
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(scan_interval);
            
            while *running.read().await {
                interval.tick().await;
                
                // 更新统计信息
                {
                    let mut stats = stats.write().await;
                    stats.update_uptime();
                    
                    // 获取设备算力
                    if let Ok(device_manager) = device_manager.try_lock() {
                        let hashrate = device_manager.get_total_hashrate().await;
                        stats.update_hashrate(hashrate);
                    }
                }
                
                // 检查设备健康状态
                if let Ok(device_manager) = device_manager.try_lock() {
                    // 这里可以添加设备健康检查逻辑
                }
                
                // 检查矿池连接状态
                if let Ok(pool_manager) = pool_manager.try_lock() {
                    // 这里可以添加矿池连接检查逻辑
                }
            }
        });
        
        *self.main_loop_handle.lock().await = Some(handle);
        Ok(())
    }
    
    /// 启动工作分发
    async fn start_work_dispatch(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let device_manager = self.device_manager.clone();
        let work_receiver = self.work_receiver.clone();
        
        let handle = tokio::spawn(async move {
            let mut receiver = work_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                while *running.read().await {
                    match receiver.recv().await {
                        Some(work_item) => {
                            // 分发工作到设备
                            if let Ok(device_manager) = device_manager.try_lock() {
                                if let Some(device_id) = work_item.assigned_device {
                                    if let Err(e) = device_manager.submit_work(device_id, work_item.work).await {
                                        error!("Failed to submit work to device {}: {}", device_id, e);
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }
            }
        });
        
        *self.work_dispatch_handle.lock().await = Some(handle);
        Ok(())
    }
    
    /// 启动结果处理
    async fn start_result_processing(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let pool_manager = self.pool_manager.clone();
        let stats = self.stats.clone();
        let result_receiver = self.result_receiver.clone();
        let event_sender = self.event_sender.clone();
        
        let handle = tokio::spawn(async move {
            let mut receiver = result_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                while *running.read().await {
                    match receiver.recv().await {
                        Some(result_item) => {
                            // 处理挖矿结果
                            if result_item.is_valid() {
                                // 提交到矿池
                                if let Ok(pool_manager) = pool_manager.try_lock() {
                                    // 这里需要实现份额提交逻辑
                                }
                                
                                // 更新统计
                                {
                                    let mut stats = stats.write().await;
                                    stats.record_accepted_share(result_item.result.difficulty);
                                }
                                
                                // 发送事件
                                let _ = event_sender.send(MiningEvent::ShareAccepted {
                                    work_id: result_item.result.work_id,
                                    difficulty: result_item.result.difficulty,
                                    timestamp: SystemTime::now(),
                                });
                            }
                        }
                        None => break,
                    }
                }
            }
        });
        
        *self.result_process_handle.lock().await = Some(handle);
        Ok(())
    }
    
    /// 停止所有任务
    async fn stop_tasks(&self) {
        // 停止主循环
        if let Some(handle) = self.main_loop_handle.lock().await.take() {
            handle.abort();
        }
        
        // 停止工作分发
        if let Some(handle) = self.work_dispatch_handle.lock().await.take() {
            handle.abort();
        }
        
        // 停止结果处理
        if let Some(handle) = self.result_process_handle.lock().await.take() {
            handle.abort();
        }
    }
}

/// 系统状态
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub state: MiningState,
    pub uptime: Duration,
    pub total_hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub active_devices: u32,
    pub connected_pools: u32,
    pub current_difficulty: f64,
    pub best_share: f64,
    pub efficiency: f64,
    pub power_consumption: f64,
}
