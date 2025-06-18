//! 核心注册和发现系统

use crate::core::{MiningCore, CoreInfo, CoreConfig};
use crate::error::CoreError;
use crate::CoreType;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn, error};

/// 核心工厂特征
#[async_trait]
pub trait CoreFactory: Send + Sync {
    /// 获取核心类型
    fn core_type(&self) -> CoreType;

    /// 获取核心信息
    fn core_info(&self) -> CoreInfo;

    /// 创建核心实例
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>, CoreError>;

    /// 验证配置
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError>;

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig;
}

/// 核心注册表
pub struct CoreRegistry {
    /// 注册的核心工厂
    factories: Arc<RwLock<HashMap<String, Box<dyn CoreFactory>>>>,
    /// 活跃的核心实例
    active_cores: Arc<RwLock<HashMap<String, Box<dyn MiningCore>>>>,
}

impl CoreRegistry {
    /// 创建新的核心注册表
    pub fn new() -> Self {
        Self {
            factories: Arc::new(RwLock::new(HashMap::new())),
            active_cores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册核心工厂
    pub fn register_factory(&self, name: String, factory: Box<dyn CoreFactory>) -> Result<(), CoreError> {
        let mut factories = self.factories.write().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire write lock: {}", e))
        })?;

        if factories.contains_key(&name) {
            warn!("核心工厂 '{}' 已存在，将被覆盖", name);
        }

        info!("注册核心工厂: {} (类型: {})", name, factory.core_type());
        factories.insert(name, factory);
        Ok(())
    }

    /// 取消注册核心工厂
    pub fn unregister_factory(&self, name: &str) -> Result<(), CoreError> {
        let mut factories = self.factories.write().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire write lock: {}", e))
        })?;

        if factories.remove(name).is_some() {
            info!("取消注册核心工厂: {}", name);
            Ok(())
        } else {
            Err(CoreError::runtime(format!("核心工厂 '{}' 不存在", name)))
        }
    }

    /// 获取所有注册的核心工厂
    pub fn list_factories(&self) -> Result<Vec<CoreInfo>, CoreError> {
        let factories = self.factories.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(factories.values().map(|factory| factory.core_info()).collect())
    }

    /// 根据名称获取核心工厂
    pub fn get_factory(&self, name: &str) -> Result<Option<CoreInfo>, CoreError> {
        let factories = self.factories.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(factories.get(name).map(|factory| factory.core_info()))
    }

    /// 根据类型获取核心工厂
    pub fn get_factories_by_type(&self, core_type: &CoreType) -> Result<Vec<CoreInfo>, CoreError> {
        let factories = self.factories.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(factories
            .values()
            .filter(|factory| &factory.core_type() == core_type)
            .map(|factory| factory.core_info())
            .collect())
    }

    /// 创建核心实例
    pub async fn create_core(&self, factory_name: &str, config: CoreConfig) -> Result<String, CoreError> {
        // 获取工厂
        let _factory = {
            let factories = self.factories.read().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire read lock: {}", e))
            })?;

            factories.get(factory_name).ok_or_else(|| {
                CoreError::runtime(format!("核心工厂 '{}' 不存在", factory_name))
            })?.core_info()
        };

        // 验证配置
        {
            let factories = self.factories.read().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire read lock: {}", e))
            })?;

            if let Some(factory) = factories.get(factory_name) {
                factory.validate_config(&config)?;
            }
        }

        // 创建核心实例
        let core = {
            let factories = self.factories.read().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire read lock: {}", e))
            })?;

            if let Some(factory) = factories.get(factory_name) {
                factory.create_core(config.clone()).await?
            } else {
                return Err(CoreError::runtime(format!("核心工厂 '{}' 不存在", factory_name)));
            }
        };

        // 生成核心实例ID
        let core_id = format!("{}_{}", factory_name, uuid::Uuid::new_v4());

        // 存储核心实例
        {
            let mut active_cores = self.active_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;

            active_cores.insert(core_id.clone(), core);
        }

        info!("创建核心实例: {} (工厂: {})", core_id, factory_name);
        Ok(core_id)
    }

    /// 获取活跃的核心实例
    pub fn get_core(&self, core_id: &str) -> Result<Option<()>, CoreError> {
        let active_cores = self.active_cores.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(if active_cores.contains_key(core_id) {
            Some(())
        } else {
            None
        })
    }

    /// 列出所有活跃的核心实例
    pub fn list_active_cores(&self) -> Result<Vec<String>, CoreError> {
        let active_cores = self.active_cores.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(active_cores.keys().cloned().collect())
    }

    /// 移除核心实例
    pub async fn remove_core(&self, core_id: &str) -> Result<(), CoreError> {
        let mut core = {
            let mut active_cores = self.active_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;

            active_cores.remove(core_id).ok_or_else(|| {
                CoreError::runtime(format!("核心实例 '{}' 不存在", core_id))
            })?
        };

        // 关闭核心
        if let Err(e) = core.shutdown().await {
            error!("关闭核心实例 '{}' 时出错: {}", core_id, e);
        }

        info!("移除核心实例: {}", core_id);
        Ok(())
    }

    /// 关闭所有核心实例
    pub async fn shutdown_all(&self) -> Result<(), CoreError> {
        let core_ids: Vec<String> = {
            let active_cores = self.active_cores.read().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire read lock: {}", e))
            })?;
            active_cores.keys().cloned().collect()
        };

        for core_id in core_ids {
            if let Err(e) = self.remove_core(&core_id).await {
                error!("关闭核心实例 '{}' 时出错: {}", core_id, e);
            }
        }

        info!("所有核心实例已关闭");
        Ok(())
    }

    /// 获取注册表统计信息
    pub fn get_stats(&self) -> Result<RegistryStats, CoreError> {
        let factories = self.factories.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        let active_cores = self.active_cores.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(RegistryStats {
            registered_factories: factories.len(),
            active_cores: active_cores.len(),
        })
    }
}

impl Default for CoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 注册表统计信息
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// 注册的工厂数量
    pub registered_factories: usize,
    /// 活跃的核心数量
    pub active_cores: usize,
}
