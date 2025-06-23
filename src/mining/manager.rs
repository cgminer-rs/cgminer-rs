use crate::config::Config;
use crate::error::MiningError;
use crate::device::{DeviceManager, DeviceCoreMapper};
use crate::pool::PoolManager;
use crate::monitoring::{MonitoringSystem, MiningMetrics};
use crate::mining::{MiningState, MiningStats, MiningConfig, MiningEvent, WorkItem, ResultItem, Hashmeter};
use cgminer_core::{CoreRegistry, CoreType, CoreConfig};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, broadcast};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

/// 挖矿管理器 - 协调所有子系统（集成协调器功能）
pub struct MiningManager {
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// 设备管理器
    device_manager: Arc<Mutex<DeviceManager>>,
    /// 设备-核心映射器（从协调器移入）
    device_core_mapper: Arc<DeviceCoreMapper>,
    /// 矿池管理器
    pool_manager: Arc<Mutex<PoolManager>>,
    /// 监控系统
    monitoring_system: Arc<Mutex<MonitoringSystem>>,
    /// 算力计量器
    hashmeter: Arc<Mutex<Option<Hashmeter>>>,
    /// 完整配置
    full_config: Config,
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
    /// 算力更新任务句柄
    hashmeter_update_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 核心结果收集任务句柄
    core_result_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl MiningManager {
    /// 创建新的挖矿管理器
    pub async fn new(config: Config, core_registry: Arc<CoreRegistry>) -> Result<Self, MiningError> {
        info!("Creating mining manager with core registry");

        // 创建设备管理器
        let mut device_manager = DeviceManager::new(config.devices.clone(), core_registry.clone());
        device_manager.set_full_config(config.clone());

        // 创建设备-核心映射器
        let device_core_mapper = DeviceCoreMapper::new(core_registry.clone());

        // 根据配置的核心类型注册相应的设备驱动
        Self::register_drivers_for_cores(&mut device_manager, &config.cores).await?;

        // 创建矿池管理器
        let pool_manager = PoolManager::new(config.pools.clone()).await?;

        // 创建监控系统
        let monitoring_system = MonitoringSystem::new(config.monitoring.clone()).await?;

        // 创建通道
        let (work_sender, work_receiver) = mpsc::unbounded_channel();
        let (result_sender, result_receiver) = mpsc::unbounded_channel();
        let (event_sender, _) = broadcast::channel(1000);

        let mining_config = MiningConfig::from(&config);

        // 创建算力计量器
        let hashmeter = if config.hashmeter.enabled && config.hashmeter.log_interval > 0 {
            Some(Hashmeter::new(config.hashmeter.clone()))
        } else {
            None
        };

        Ok(Self {
            core_registry,
            device_manager: Arc::new(Mutex::new(device_manager)),
            device_core_mapper: Arc::new(device_core_mapper),
            pool_manager: Arc::new(Mutex::new(pool_manager)),
            monitoring_system: Arc::new(Mutex::new(monitoring_system)),
            hashmeter: Arc::new(Mutex::new(hashmeter)),
            full_config: config,
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
            hashmeter_update_handle: Arc::new(Mutex::new(None)),
            core_result_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 根据配置的核心类型注册相应的设备驱动
    async fn register_drivers_for_cores(
        _device_manager: &mut DeviceManager,
        cores_config: &crate::config::CoresConfig
    ) -> Result<(), MiningError> {
        info!("根据配置注册设备驱动，启用的核心: {:?}", cores_config.enabled_cores);

        for core_type in &cores_config.enabled_cores {
            match core_type.as_str() {
                "software" | "cpu-btc" | "btc" | "cpu" => {
                    // 软算法核心不需要设备驱动，直接通过核心管理
                    info!("软算法核心已启用，将通过核心管理器直接管理");
                }
                "asic" | "maijie-l7" | "l7" => {
                    // ASIC核心现在通过工厂模式管理，不需要在这里注册设备驱动
                    info!("ASIC核心将通过统一设备工厂管理");
                }
                _ => {
                    warn!("未知的核心类型: {}", core_type);
                }
            }
        }

        Ok(())
    }

    /// 创建挖矿核心
    pub async fn create_core(&self, core_type: &str, config: CoreConfig) -> Result<String, MiningError> {
        info!("创建挖矿核心: {}", core_type);

        let core_id = self.core_registry.create_core(core_type, config).await
            .map_err(|e| MiningError::CoreError(format!("创建核心失败: {}", e)))?;

        info!("挖矿核心创建成功: {}", core_id);
        Ok(core_id)
    }

    /// 列出可用的核心类型
    pub async fn list_available_cores(&self) -> Result<Vec<cgminer_core::CoreInfo>, MiningError> {
        self.core_registry.list_factories().await
            .map_err(|e| MiningError::CoreError(format!("获取核心列表失败: {}", e)))
    }

    /// 根据类型获取核心
    pub async fn get_cores_by_type(&self, core_type: &CoreType) -> Result<Vec<cgminer_core::CoreInfo>, MiningError> {
        self.core_registry.get_factories_by_type(core_type).await
            .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))
    }

    /// 移除挖矿核心
    pub async fn remove_core(&self, core_id: &str) -> Result<(), MiningError> {
        info!("移除挖矿核心: {}", core_id);

        self.core_registry.remove_core(core_id).await
            .map_err(|e| MiningError::CoreError(format!("移除核心失败: {}", e)))?;

        info!("挖矿核心移除成功: {}", core_id);
        Ok(())
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

        // 先启动挖矿核心（创建核心实例）
        self.start_cores().await?;

        // 初始化设备管理器（使用协调器功能）
        self.initialize_device_manager().await?;

        // 启动矿池管理器
        {
            let pool_manager = self.pool_manager.lock().await;
            pool_manager.start().await?;
        }

        // 启动监控系统
        {
            let monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.start().await?;
        }

        // 启动算力计量器
        self.start_hashmeter().await?;

        // 启动各个任务
        self.start_main_loop().await?;
        self.start_work_dispatch().await?;
        self.start_result_processing().await?;
        self.start_core_result_collection().await?;
        self.start_hashmeter_updates().await?;

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
            let monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.stop().await?;
        }

        // 停止矿池管理器
        {
            let pool_manager = self.pool_manager.lock().await;
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
        let _monitoring_system = self.monitoring_system.clone();
        let _event_sender = self.event_sender.clone();
        let work_sender = self.work_sender.clone();
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
                if let Ok(_device_manager) = device_manager.try_lock() {
                    // 这里可以添加设备健康检查逻辑
                }

                // 检查矿池连接状态并获取工作
                if let Ok(pool_manager) = pool_manager.try_lock() {
                    // 获取工作并发送到工作分发器
                    if let Ok(work_sender_guard) = work_sender.try_lock() {
                        if let Some(sender) = work_sender_guard.as_ref() {
                            // 尝试从矿池获取工作
                            match pool_manager.get_work().await {
                                Ok(work) => {
                                    let work_item = WorkItem {
                                        work,
                                        assigned_device: None, // 让工作分发器决定分配给哪个设备
                                        created_at: SystemTime::now(),
                                        priority: 1,
                                        retry_count: 0,
                                    };

                                    if let Err(e) = sender.send(work_item) {
                                        warn!("Failed to send work to dispatcher: {}", e);
                                    } else {
                                        info!("Work sent to dispatcher");
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to get work from pool: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        *self.main_loop_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动统一工作分发器
    async fn start_work_dispatch(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let device_manager = self.device_manager.clone();
        let core_registry = self.core_registry.clone();
        let work_receiver = self.work_receiver.clone();

        let handle = tokio::spawn(async move {
            let receiver = work_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                info!("📡 工作分发器已启动，等待工作...");

                // 创建统一的工作分发器
                let work_dispatcher = UnifiedWorkDispatcher::new(
                    core_registry.clone(),
                    device_manager.clone(),
                );

                while *running.read().await {
                    match receiver.recv().await {
                        Some(work_item) => {
                            info!("📨 收到新工作，ID: {}", work_item.work.id);

                            // 使用统一的工作分发逻辑
                            match work_dispatcher.dispatch_work(work_item).await {
                                Ok(target) => {
                                    info!("✅ 工作成功分发到: {}", target);
                                }
                                Err(e) => {
                                    error!("❌ 工作分发失败: {}", e);
                                }
                            }
                        }
                        None => {
                            info!("📡 工作接收器关闭，退出分发循环");
                            break;
                        }
                    }
                }

                info!("📡 工作分发器已停止");
            } else {
                error!("❌ 无法获取工作接收器");
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
            let receiver = result_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                while *running.read().await {
                    match receiver.recv().await {
                        Some(result_item) => {
                            // 处理挖矿结果
                            if result_item.is_valid() {
                                // 提交到矿池
                                if let Ok(_pool_manager) = pool_manager.try_lock() {
                                    // 这里需要实现份额提交逻辑
                                }

                                // 更新统计
                                {
                                    let mut stats = stats.write().await;
                                    stats.record_accepted_share(result_item.result.share_difficulty);
                                }

                                // 发送事件
                                let _ = event_sender.send(MiningEvent::ShareAccepted {
                                    work_id: result_item.result.work_id,
                                    difficulty: result_item.result.share_difficulty,
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

    /// 启动核心结果收集
    async fn start_core_result_collection(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let core_registry = self.core_registry.clone();
        let _result_sender = self.result_sender.clone(); // 暂时不使用，因为我们不创建假的WorkItem
        let stats = self.stats.clone();
        let _pool_manager = self.pool_manager.clone(); // 暂时不使用，因为缺少工作数据
        let core_result_handle = self.core_result_handle.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100)); // 每100ms检查一次结果

            while *running.read().await {
                interval.tick().await;

                // 从核心注册表获取所有活跃核心并收集结果
                match core_registry.list_active_cores().await {
                    Ok(active_core_ids) => {
                        if !active_core_ids.is_empty() {
                            debug!("Collecting results from {} active cores", active_core_ids.len());
                        }
                        for core_id in active_core_ids {
                            // 从核心注册表收集结果
                            match core_registry.collect_results_from_core(&core_id).await {
                                Ok(results) => {
                                    for core_result in results {
                                        // 转换核心结果到本地格式（work_id已经是UUID）
                                        let mut mining_result = cgminer_core::types::MiningResult::new(
                                            core_result.work_id,
                                            core_result.device_id,
                                            core_result.nonce,
                                            core_result.hash,
                                            core_result.meets_target,
                                        );

                                        // 设置extranonce2
                                        if core_result.extranonce2.len() >= 4 {
                                            mining_result = mining_result.with_extranonce2(core_result.extranonce2);
                                        }

                                        // 计算份额难度
                                        if let Err(e) = mining_result.calculate_share_difficulty() {
                                            warn!("Failed to calculate share difficulty: {}", e);
                                        }

                                        // 直接处理挖矿结果，不创建假的WorkItem
                                        // 注意：由于我们无法获取原始的工作数据（job_id、ntime等），
                                        // 我们暂时跳过份额提交，只更新统计数据
                                        // 在完整的实现中，应该维护一个工作ID到工作数据的映射

                                        if core_result.meets_target {
                                            debug!("Valid result found from core {}, device {}, but skipping submission due to missing work data",
                                                   core_id, core_result.device_id);

                                            // 更新统计数据（记录为找到有效结果）
                                            {
                                                let mut stats_guard = stats.write().await;
                                                stats_guard.record_accepted_share(mining_result.share_difficulty);
                                            }
                                        } else {
                                            // 结果不满足目标难度，仅更新哈希计数
                                            debug!("Result from core {} does not meet target difficulty", core_id);
                                        }

                                        // 更新统计数据
                                        {
                                            let mut stats_guard = stats.write().await;
                                            if core_result.meets_target {
                                                stats_guard.record_accepted_share(1.0);
                                            } else {
                                                stats_guard.record_rejected_share();
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("No results from core {}: {}", core_id, e);
                                }
                            }

                            // 获取核心的算力统计
                            match core_registry.get_core_stats(&core_id).await {
                                Ok(core_stats) => {
                                    // 更新总体算力统计
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.current_hashrate = core_stats.total_hashrate;
                                    stats_guard.average_hashrate = core_stats.average_hashrate;
                                }
                                Err(e) => {
                                    debug!("Failed to get stats from core {}: {}", core_id, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to list active cores: {}", e);
                    }
                }
            }
        });

        // 存储任务句柄
        *core_result_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动挖矿核心
    async fn start_cores(&self) -> Result<(), MiningError> {
        info!("启动挖矿核心");

        // 首先检查是否已经有活跃的核心（由设备管理器创建）
        match self.core_registry.list_active_cores().await {
            Ok(active_cores) => {
                if !active_cores.is_empty() {
                    info!("发现已存在的活跃核心: {:?}", active_cores);

                    // 启动所有已存在的核心
                    for core_id in &active_cores {
                        match self.core_registry.start_core(core_id).await {
                            Ok(()) => {
                                info!("🚀 核心启动成功: {}", core_id);
                            }
                            Err(e) => {
                                warn!("⚠️ 核心启动失败: {}: {}", core_id, e);
                            }
                        }
                    }

                    info!("所有挖矿核心启动完成");
                    return Ok(());
                }
            }
            Err(e) => {
                warn!("获取活跃核心列表失败: {}", e);
            }
        }

        // 如果没有活跃的核心，则创建新的核心
        info!("没有发现活跃核心，开始创建新核心");

        // 获取启用的核心类型
        let enabled_cores = &self.full_config.cores.enabled_cores;

        for core_type in enabled_cores {
            match core_type.as_str() {
                "software" | "cpu-btc" | "btc" | "cpu" => {
                    info!("启动软算法核心");

                    // 创建软算法核心配置
                    let core_config = CoreConfig {
                        name: "software_core".to_string(),
                        enabled: true,
                        devices: vec![], // 设备配置将在核心内部创建
                        custom_params: {
                            let mut params = std::collections::HashMap::new();
                            if let Some(cpu_btc_config) = &self.full_config.cores.cpu_btc {
                                params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.device_count)));
                                params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.min_hashrate).unwrap()));
                                params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.max_hashrate).unwrap()));
                                params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.error_rate).unwrap()));
                                params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.batch_size)));
                                params.insert("work_timeout_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.work_timeout_ms)));
                            }
                            params
                        },
                    };

                    // 创建软算法核心
                    let core_id = self.create_core("cpu-btc", core_config).await?;

                    // 检查核心是否创建成功
                    if self.core_registry.get_core(&core_id).await
                        .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))?.is_some() {
                        info!("✅ 软算法核心创建成功: {}", core_id);

                        // 启动软算法核心
                        match self.core_registry.start_core(&core_id).await {
                            Ok(()) => {
                                info!("🚀 软算法核心启动成功: {}", core_id);
                                info!("软算法核心已在CoreRegistry中管理并运行: {}", core_id);
                            }
                            Err(e) => {
                                error!("❌ 软算法核心启动失败: {}: {}", core_id, e);
                                return Err(MiningError::CoreError(format!("启动核心失败: {}", e)));
                            }
                        }
                    }
                }
                "asic" | "maijie-l7" | "l7" => {
                    if let Some(maijie_l7_config) = &self.full_config.cores.maijie_l7 {
                        if maijie_l7_config.enabled {
                            info!("启动Maijie L7 ASIC核心");

                            let core_config = CoreConfig {
                                name: "maijie_l7_core".to_string(),
                                enabled: true,
                                devices: vec![], // 设备配置将在核心内部创建
                                custom_params: {
                                    let mut params = std::collections::HashMap::new();
                                    params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.chain_count)));
                                    params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.spi_speed)));
                                    params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.uart_baud)));
                                    params.insert("auto_detect".to_string(), serde_json::Value::Bool(maijie_l7_config.auto_detect));
                                    params.insert("power_limit".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(maijie_l7_config.power_limit).unwrap()));
                                    params.insert("cooling_mode".to_string(), serde_json::Value::String(maijie_l7_config.cooling_mode.clone()));
                                    params
                                },
                            };

                            let core_id = self.create_core("maijie-l7", core_config).await?;

                            if self.core_registry.get_core(&core_id).await
                                .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))?.is_some() {
                                info!("✅ ASIC核心创建成功: {}", core_id);
                            }
                        }
                    }
                }
                _ => {
                    warn!("未知的核心类型: {}", core_type);
                }
            }
        }

        info!("所有挖矿核心启动完成");
        Ok(())
    }

    /// 启动算力计量器
    async fn start_hashmeter(&self) -> Result<(), MiningError> {
        let hashmeter_guard = self.hashmeter.lock().await;
        if let Some(hashmeter) = hashmeter_guard.as_ref() {
            hashmeter.start().await?;
            info!("📊 Hashmeter started successfully");
        }
        Ok(())
    }

    /// 启动算力数据更新任务
    async fn start_hashmeter_updates(&self) -> Result<(), MiningError> {
        let hashmeter = self.hashmeter.clone();
        let stats = self.stats.clone();
        let _device_manager = self.device_manager.clone();
        let _monitoring_system = self.monitoring_system.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // 每5秒更新一次数据

            while *running.read().await {
                interval.tick().await;

                // 检查是否有hashmeter
                let hashmeter_guard = hashmeter.lock().await;
                if let Some(hashmeter) = hashmeter_guard.as_ref() {
                    // 获取挖矿统计数据
                    let stats_guard = stats.read().await;
                    let mining_metrics = MiningMetrics {
                        timestamp: SystemTime::now(),
                        total_hashrate: stats_guard.current_hashrate,
                        accepted_shares: stats_guard.accepted_shares,
                        rejected_shares: stats_guard.rejected_shares,
                        hardware_errors: stats_guard.hardware_errors,
                        stale_shares: stats_guard.stale_shares,
                        best_share: stats_guard.best_share,
                        current_difficulty: stats_guard.current_difficulty,
                        network_difficulty: stats_guard.network_difficulty,
                        blocks_found: stats_guard.blocks_found,
                        efficiency: stats_guard.efficiency,
                        active_devices: 0, // 需要从设备管理器获取
                        connected_pools: 0, // 需要从矿池管理器获取
                    };

                    // 更新总体统计
                    if let Err(e) = hashmeter.update_total_stats(&mining_metrics).await {
                        warn!("Failed to update hashmeter total stats: {}", e);
                    }

                    // TODO: 更新设备级统计数据
                    // 这里需要从设备管理器获取设备统计数据
                }
            }
        });

        *self.hashmeter_update_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止所有任务
    async fn stop_tasks(&self) {
        // 停止算力计量器
        {
            let hashmeter_guard = self.hashmeter.lock().await;
            if let Some(hashmeter) = hashmeter_guard.as_ref() {
                if let Err(e) = hashmeter.stop().await {
                    warn!("Failed to stop hashmeter: {}", e);
                }
            }
        }

        // 停止算力更新任务
        if let Some(handle) = self.hashmeter_update_handle.lock().await.take() {
            handle.abort();
        }

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

        // 停止核心结果收集
        if let Some(handle) = self.core_result_handle.lock().await.take() {
            handle.abort();
        }
    }

    /// 初始化设备管理器（从协调器移植）
    async fn initialize_device_manager(&self) -> Result<(), MiningError> {
        info!("初始化设备管理器");

        let active_core_ids = self.core_registry.list_active_cores().await
            .map_err(|e| MiningError::CoreError(format!("获取活跃核心列表失败: {}", e)))?;

        let mut device_manager = self.device_manager.lock().await;
        device_manager.set_active_cores(active_core_ids).await;
        device_manager.initialize().await?;
        device_manager.start().await?;

        // 验证设备映射
        device_manager.validate_device_mappings().await?;

        info!("设备管理器初始化成功");
        Ok(())
    }

    /// 提交工作（从协调器移植）
    pub async fn submit_work(&self, work: crate::device::Work) -> Result<(), MiningError> {
        let work_item = WorkItem {
            work,
            assigned_device: None,
            created_at: SystemTime::now(),
            priority: 1,
            retry_count: 0,
        };

        if let Ok(work_sender_guard) = self.work_sender.try_lock() {
            if let Some(sender) = work_sender_guard.as_ref() {
                sender.send(work_item)
                    .map_err(|e| MiningError::WorkError(format!("提交工作失败: {}", e)))?;
            }
        }

        Ok(())
    }

    /// 获取设备-核心映射器
    pub fn get_device_core_mapper(&self) -> Arc<DeviceCoreMapper> {
        self.device_core_mapper.clone()
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

/// 统一工作分发器
/// 负责将工作统一分发到核心或设备，避免分发逻辑的重复和不一致
pub struct UnifiedWorkDispatcher {
    core_registry: Arc<CoreRegistry>,
    device_manager: Arc<Mutex<DeviceManager>>,
}

impl UnifiedWorkDispatcher {
    /// 创建新的统一工作分发器
    pub fn new(
        core_registry: Arc<CoreRegistry>,
        device_manager: Arc<Mutex<DeviceManager>>,
    ) -> Self {
        Self {
            core_registry,
            device_manager,
        }
    }

    /// 分发工作
    /// 优先级：活跃核心 > 指定设备 > 任意可用设备
    pub async fn dispatch_work(&self, work_item: WorkItem) -> Result<String, String> {
        info!("🚀 开始统一工作分发，工作ID: {}", work_item.work.id);

        // 1. 优先尝试分发到活跃的核心
        info!("🎯 第一步：尝试分发到活跃核心...");
        match self.dispatch_to_cores(&work_item).await {
            Ok(target) => {
                info!("✅ 工作成功分发到: {}", target);
                return Ok(target);
            }
            Err(e) => {
                warn!("⚠️  核心分发失败: {}", e);
            }
        }

        // 2. 如果核心分发失败，尝试分发到设备
        info!("🎯 第二步：尝试分发到设备...");
        match self.dispatch_to_devices(&work_item).await {
            Ok(target) => {
                info!("✅ 工作成功分发到: {}", target);
                return Ok(target);
            }
            Err(e) => {
                warn!("⚠️  设备分发失败: {}", e);
            }
        }

        error!("💥 工作分发完全失败：没有可用的核心或设备");
        Err("No available cores or devices for work dispatch".to_string())
    }

    /// 分发工作到核心
    async fn dispatch_to_cores(&self, work_item: &WorkItem) -> Result<String, String> {
        info!("🔍 开始分发工作到核心...");

        let active_core_ids = self.core_registry.list_active_cores().await
            .map_err(|e| format!("Failed to list active cores: {}", e))?;

        info!("📋 发现 {} 个活跃核心", active_core_ids.len());

        if active_core_ids.is_empty() {
            warn!("⚠️  没有活跃的核心可用于工作分发");
            return Err("No active cores available".to_string());
        }

        info!("🎯 活跃核心列表: {:?}", active_core_ids);

        // 使用轮询策略分发到核心
        for core_id in &active_core_ids {
            info!("📤 尝试向核心 {} 提交工作...", core_id);
            match self.core_registry.submit_work_to_core(core_id, work_item.work.clone()).await {
                Ok(()) => {
                    info!("✅ 工作成功分发到核心: {}", core_id);
                    return Ok(format!("core:{}", core_id));
                }
                Err(e) => {
                    warn!("❌ 向核心 {} 提交工作失败: {}", core_id, e);
                    continue;
                }
            }
        }

        warn!("💥 所有核心都拒绝了工作");
        Err("All cores rejected the work".to_string())
    }

    /// 分发工作到设备
    async fn dispatch_to_devices(&self, work_item: &WorkItem) -> Result<String, String> {
        let device_manager = self.device_manager.try_lock()
            .map_err(|_| "Device manager is busy".to_string())?;

        // 如果指定了设备，优先分发到该设备
        if let Some(device_id) = work_item.assigned_device {
            match device_manager.submit_work(device_id, work_item.work.clone()).await {
                Ok(()) => {
                    return Ok(format!("device:{}", device_id));
                }
                Err(e) => {
                    debug!("Failed to submit work to assigned device {}: {}", device_id, e);
                }
            }
        }

        // 如果没有指定设备或指定设备失败，尝试分发到任意可用设备
        // 这里需要从设备管理器获取可用设备列表
        // 由于当前DeviceManager没有提供获取所有设备的方法，我们暂时返回错误
        Err("No available devices for work dispatch".to_string())
    }
}
