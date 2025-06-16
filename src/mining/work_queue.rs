use crate::device::{Work, MiningResult};
use crate::error::WorkError;
use crate::mining::{WorkItem, ResultItem, WorkDistributionStrategy, ValidationStatus};
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, Notify};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// 工作队列
pub struct WorkQueue {
    /// 工作队列
    queue: VecDeque<WorkItem>,
    /// 最大队列大小
    max_size: usize,
    /// 工作映射 (用于快速查找)
    work_map: HashMap<Uuid, WorkItem>,
    /// 统计信息
    stats: WorkQueueStats,
}

impl WorkQueue {
    /// 创建新的工作队列
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            work_map: HashMap::new(),
            stats: WorkQueueStats::new(),
        }
    }
    
    /// 添加工作
    pub fn push(&mut self, work_item: WorkItem) -> Result<(), WorkError> {
        if self.queue.len() >= self.max_size {
            return Err(WorkError::QueueFull);
        }
        
        // 检查是否重复
        if self.work_map.contains_key(&work_item.work.id) {
            return Err(WorkError::Duplicate { work_id: work_item.work.id.to_string() });
        }
        
        // 添加到队列和映射
        self.work_map.insert(work_item.work.id, work_item.clone());
        self.queue.push_back(work_item);
        self.stats.total_added += 1;
        
        Ok(())
    }
    
    /// 获取工作
    pub fn pop(&mut self) -> Option<WorkItem> {
        if let Some(work_item) = self.queue.pop_front() {
            self.work_map.remove(&work_item.work.id);
            self.stats.total_processed += 1;
            Some(work_item)
        } else {
            None
        }
    }
    
    /// 按优先级获取工作
    pub fn pop_by_priority(&mut self) -> Option<WorkItem> {
        if self.queue.is_empty() {
            return None;
        }
        
        // 找到最高优先级的工作
        let mut max_priority = 0;
        let mut max_index = 0;
        
        for (index, work_item) in self.queue.iter().enumerate() {
            if work_item.priority > max_priority {
                max_priority = work_item.priority;
                max_index = index;
            }
        }
        
        if let Some(work_item) = self.queue.remove(max_index) {
            self.work_map.remove(&work_item.work.id);
            self.stats.total_processed += 1;
            Some(work_item)
        } else {
            None
        }
    }
    
    /// 获取指定设备的工作
    pub fn pop_for_device(&mut self, device_id: u32) -> Option<WorkItem> {
        // 首先查找分配给特定设备的工作
        for i in 0..self.queue.len() {
            if let Some(assigned_device) = self.queue[i].assigned_device {
                if assigned_device == device_id {
                    if let Some(work_item) = self.queue.remove(i) {
                        self.work_map.remove(&work_item.work.id);
                        self.stats.total_processed += 1;
                        return Some(work_item);
                    }
                }
            }
        }
        
        // 如果没有专门分配的工作，返回第一个未分配的工作
        for i in 0..self.queue.len() {
            if self.queue[i].assigned_device.is_none() {
                if let Some(mut work_item) = self.queue.remove(i) {
                    work_item.assigned_device = Some(device_id);
                    self.work_map.remove(&work_item.work.id);
                    self.stats.total_processed += 1;
                    return Some(work_item);
                }
            }
        }
        
        None
    }
    
    /// 清理过期工作
    pub fn cleanup_expired(&mut self) -> usize {
        let mut removed_count = 0;
        let mut i = 0;
        
        while i < self.queue.len() {
            if self.queue[i].is_expired() {
                if let Some(work_item) = self.queue.remove(i) {
                    self.work_map.remove(&work_item.work.id);
                    removed_count += 1;
                    self.stats.total_expired += 1;
                }
            } else {
                i += 1;
            }
        }
        
        removed_count
    }
    
    /// 获取队列大小
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    
    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    
    /// 检查队列是否已满
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.max_size
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> &WorkQueueStats {
        &self.stats
    }
    
    /// 清空队列
    pub fn clear(&mut self) {
        self.queue.clear();
        self.work_map.clear();
        self.stats.total_cleared += self.queue.len() as u64;
    }
}

/// 结果队列
pub struct ResultQueue {
    /// 结果队列
    queue: VecDeque<ResultItem>,
    /// 最大队列大小
    max_size: usize,
    /// 结果映射
    result_map: HashMap<Uuid, ResultItem>,
    /// 统计信息
    stats: ResultQueueStats,
}

impl ResultQueue {
    /// 创建新的结果队列
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            result_map: HashMap::new(),
            stats: ResultQueueStats::new(),
        }
    }
    
    /// 添加结果
    pub fn push(&mut self, result_item: ResultItem) -> Result<(), WorkError> {
        if self.queue.len() >= self.max_size {
            return Err(WorkError::QueueFull);
        }
        
        // 检查是否重复
        if self.result_map.contains_key(&result_item.result.work_id) {
            return Err(WorkError::Duplicate { work_id: result_item.result.work_id.to_string() });
        }
        
        // 添加到队列和映射
        self.result_map.insert(result_item.result.work_id, result_item.clone());
        self.queue.push_back(result_item);
        self.stats.total_added += 1;
        
        Ok(())
    }
    
    /// 获取结果
    pub fn pop(&mut self) -> Option<ResultItem> {
        if let Some(result_item) = self.queue.pop_front() {
            self.result_map.remove(&result_item.result.work_id);
            self.stats.total_processed += 1;
            Some(result_item)
        } else {
            None
        }
    }
    
    /// 获取有效结果
    pub fn pop_valid(&mut self) -> Option<ResultItem> {
        for i in 0..self.queue.len() {
            if self.queue[i].is_valid() {
                if let Some(result_item) = self.queue.remove(i) {
                    self.result_map.remove(&result_item.result.work_id);
                    self.stats.total_processed += 1;
                    return Some(result_item);
                }
            }
        }
        None
    }
    
    /// 获取队列大小
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    
    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> &ResultQueueStats {
        &self.stats
    }
    
    /// 清空队列
    pub fn clear(&mut self) {
        self.queue.clear();
        self.result_map.clear();
        self.stats.total_cleared += self.queue.len() as u64;
    }
}

/// 工作队列管理器
pub struct WorkQueueManager {
    /// 工作队列
    work_queue: Arc<Mutex<WorkQueue>>,
    /// 结果队列
    result_queue: Arc<Mutex<ResultQueue>>,
    /// 分发策略
    distribution_strategy: WorkDistributionStrategy,
    /// 工作通知
    work_notify: Arc<Notify>,
    /// 结果通知
    result_notify: Arc<Notify>,
    /// 清理任务句柄
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl WorkQueueManager {
    /// 创建新的工作队列管理器
    pub fn new(
        max_work_queue_size: usize,
        max_result_queue_size: usize,
        distribution_strategy: WorkDistributionStrategy,
    ) -> Self {
        Self {
            work_queue: Arc::new(Mutex::new(WorkQueue::new(max_work_queue_size))),
            result_queue: Arc::new(Mutex::new(ResultQueue::new(max_result_queue_size))),
            distribution_strategy,
            work_notify: Arc::new(Notify::new()),
            result_notify: Arc::new(Notify::new()),
            cleanup_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 启动队列管理器
    pub async fn start(&self) -> Result<(), WorkError> {
        info!("Starting work queue manager");
        
        *self.running.write().await = true;
        
        // 启动清理任务
        self.start_cleanup_task().await;
        
        info!("Work queue manager started");
        Ok(())
    }
    
    /// 停止队列管理器
    pub async fn stop(&self) -> Result<(), WorkError> {
        info!("Stopping work queue manager");
        
        *self.running.write().await = false;
        
        // 停止清理任务
        if let Some(handle) = self.cleanup_handle.lock().await.take() {
            handle.abort();
        }
        
        info!("Work queue manager stopped");
        Ok(())
    }
    
    /// 提交工作
    pub async fn submit_work(&self, work: Work) -> Result<(), WorkError> {
        let work_item = WorkItem::new(work);
        
        {
            let mut queue = self.work_queue.lock().await;
            queue.push(work_item)?;
        }
        
        // 通知有新工作
        self.work_notify.notify_waiters();
        
        Ok(())
    }
    
    /// 获取工作
    pub async fn get_work(&self, device_id: Option<u32>) -> Option<WorkItem> {
        let mut queue = self.work_queue.lock().await;
        
        match self.distribution_strategy {
            WorkDistributionStrategy::RoundRobin => queue.pop(),
            WorkDistributionStrategy::Priority => queue.pop_by_priority(),
            WorkDistributionStrategy::LoadBalance => {
                if let Some(device_id) = device_id {
                    queue.pop_for_device(device_id)
                } else {
                    queue.pop()
                }
            }
            WorkDistributionStrategy::Random => {
                // 简单实现：随机选择
                if queue.is_empty() {
                    None
                } else {
                    let index = fastrand::usize(0..queue.len());
                    queue.queue.remove(index).map(|work_item| {
                        queue.work_map.remove(&work_item.work.id);
                        queue.stats.total_processed += 1;
                        work_item
                    })
                }
            }
        }
    }
    
    /// 等待工作
    pub async fn wait_for_work(&self) {
        self.work_notify.notified().await;
    }
    
    /// 提交结果
    pub async fn submit_result(&self, result: MiningResult, work_item: WorkItem) -> Result<(), WorkError> {
        let result_item = ResultItem::new(result, work_item);
        
        {
            let mut queue = self.result_queue.lock().await;
            queue.push(result_item)?;
        }
        
        // 通知有新结果
        self.result_notify.notify_waiters();
        
        Ok(())
    }
    
    /// 获取结果
    pub async fn get_result(&self) -> Option<ResultItem> {
        let mut queue = self.result_queue.lock().await;
        queue.pop()
    }
    
    /// 获取有效结果
    pub async fn get_valid_result(&self) -> Option<ResultItem> {
        let mut queue = self.result_queue.lock().await;
        queue.pop_valid()
    }
    
    /// 等待结果
    pub async fn wait_for_result(&self) {
        self.result_notify.notified().await;
    }
    
    /// 获取工作队列统计
    pub async fn get_work_queue_stats(&self) -> WorkQueueStats {
        let queue = self.work_queue.lock().await;
        queue.get_stats().clone()
    }
    
    /// 获取结果队列统计
    pub async fn get_result_queue_stats(&self) -> ResultQueueStats {
        let queue = self.result_queue.lock().await;
        queue.get_stats().clone()
    }
    
    /// 启动清理任务
    async fn start_cleanup_task(&self) {
        let work_queue = self.work_queue.clone();
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            while *running.read().await {
                interval.tick().await;
                
                // 清理过期工作
                let mut queue = work_queue.lock().await;
                let removed = queue.cleanup_expired();
                if removed > 0 {
                    debug!("Cleaned up {} expired work items", removed);
                }
            }
        });
        
        *self.cleanup_handle.lock().await = Some(handle);
    }
}

/// 工作队列统计
#[derive(Debug, Clone, Default)]
pub struct WorkQueueStats {
    pub total_added: u64,
    pub total_processed: u64,
    pub total_expired: u64,
    pub total_cleared: u64,
}

impl WorkQueueStats {
    pub fn new() -> Self {
        Default::default()
    }
}

/// 结果队列统计
#[derive(Debug, Clone, Default)]
pub struct ResultQueueStats {
    pub total_added: u64,
    pub total_processed: u64,
    pub total_cleared: u64,
}

impl ResultQueueStats {
    pub fn new() -> Self {
        Default::default()
    }
}
