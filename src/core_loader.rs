//! 核心加载器 - 动态加载和管理挖矿核心

use cgminer_core::{CoreRegistry, CoreFactory, CoreType, CoreInfo, CoreError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn, error, debug};

#[cfg(feature = "software-core")]
use cgminer_software_core;

#[cfg(feature = "asic-core")]
use cgminer_asic_core;

/// 核心加载器
pub struct CoreLoader {
    /// 核心注册表
    registry: Arc<CoreRegistry>,
    /// 已加载的核心库
    loaded_cores: Arc<RwLock<HashMap<String, CoreType>>>,
}

impl CoreLoader {
    /// 创建新的核心加载器
    pub fn new() -> Self {
        Self {
            registry: Arc::new(CoreRegistry::new()),
            loaded_cores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取核心注册表
    pub fn registry(&self) -> Arc<CoreRegistry> {
        self.registry.clone()
    }

    /// 加载所有可用的核心
    pub async fn load_all_cores(&self) -> Result<(), CoreError> {
        info!("开始加载所有可用的挖矿核心");

        // 加载软算法核心
        #[cfg(feature = "software-core")]
        {
            if let Err(e) = self.load_software_core().await {
                error!("加载软算法核心失败: {}", e);
            }
        }

        // 加载ASIC核心
        #[cfg(feature = "asic-core")]
        {
            if let Err(e) = self.load_asic_core().await {
                error!("加载ASIC核心失败: {}", e);
            }
        }

        // 尝试动态加载其他核心
        if let Err(e) = self.load_dynamic_cores().await {
            warn!("动态加载核心失败: {}", e);
        }

        let stats = self.registry.get_stats()?;
        info!("核心加载完成，共加载 {} 个工厂，{} 个活跃核心", 
              stats.registered_factories, stats.active_cores);

        Ok(())
    }

    /// 加载软算法核心
    #[cfg(feature = "software-core")]
    async fn load_software_core(&self) -> Result<(), CoreError> {
        info!("加载软算法核心");

        let factory = cgminer_software_core::create_factory();
        let core_info = factory.core_info();
        
        self.registry.register_factory("software".to_string(), factory)?;
        
        {
            let mut loaded = self.loaded_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            loaded.insert("software".to_string(), core_info.core_type);
        }

        info!("软算法核心加载成功: {}", core_info.name);
        Ok(())
    }

    /// 加载ASIC核心
    #[cfg(feature = "asic-core")]
    async fn load_asic_core(&self) -> Result<(), CoreError> {
        info!("加载ASIC核心");

        let factory = cgminer_asic_core::create_factory();
        let core_info = factory.core_info();
        
        self.registry.register_factory("asic".to_string(), factory)?;
        
        {
            let mut loaded = self.loaded_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            loaded.insert("asic".to_string(), core_info.core_type);
        }

        info!("ASIC核心加载成功: {}", core_info.name);
        Ok(())
    }

    /// 动态加载核心库
    async fn load_dynamic_cores(&self) -> Result<(), CoreError> {
        debug!("尝试动态加载核心库");

        // 在实际实现中，这里会扫描指定目录下的动态库文件
        // 并尝试加载它们。由于这需要复杂的动态库加载逻辑，
        // 这里只是一个占位符实现。

        #[cfg(feature = "dynamic-loading")]
        {
            use libloading::{Library, Symbol};
            use std::path::Path;

            let core_dirs = vec![
                "./cores",
                "/usr/local/lib/cgminer-cores",
                "/opt/cgminer/cores",
            ];

            for dir in core_dirs {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("so") {
                            if let Err(e) = self.load_dynamic_core(&path).await {
                                warn!("加载动态核心 {:?} 失败: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 加载单个动态核心
    #[cfg(feature = "dynamic-loading")]
    async fn load_dynamic_core(&self, path: &Path) -> Result<(), CoreError> {
        use libloading::{Library, Symbol};
        use std::ffi::CStr;

        debug!("加载动态核心: {:?}", path);

        unsafe {
            let lib = Library::new(path).map_err(|e| {
                CoreError::runtime(format!("无法加载动态库: {}", e))
            })?;

            // 获取核心信息
            let get_info: Symbol<unsafe extern fn() -> *const std::os::raw::c_char> = 
                lib.get(b"cgminer_core_info").map_err(|e| {
                    CoreError::runtime(format!("找不到 cgminer_core_info 函数: {}", e))
                })?;

            let info_ptr = get_info();
            if info_ptr.is_null() {
                return Err(CoreError::runtime("核心信息为空"));
            }

            let info_str = CStr::from_ptr(info_ptr).to_str().map_err(|e| {
                CoreError::runtime(format!("核心信息字符串无效: {}", e))
            })?;

            let core_info: CoreInfo = serde_json::from_str(info_str).map_err(|e| {
                CoreError::runtime(format!("解析核心信息失败: {}", e))
            })?;

            // 创建工厂
            let create_factory: Symbol<unsafe extern fn() -> *mut std::os::raw::c_void> = 
                lib.get(b"cgminer_create_factory").map_err(|e| {
                    CoreError::runtime(format!("找不到 cgminer_create_factory 函数: {}", e))
                })?;

            let factory_ptr = create_factory();
            if factory_ptr.is_null() {
                return Err(CoreError::runtime("工厂创建失败"));
            }

            // 这里需要将C指针转换为Rust的CoreFactory trait对象
            // 这是一个复杂的过程，需要仔细处理内存安全
            // 在实际实现中，可能需要使用更复杂的FFI包装

            info!("动态核心加载成功: {}", core_info.name);
        }

        Ok(())
    }

    /// 列出所有已加载的核心
    pub fn list_loaded_cores(&self) -> Result<Vec<CoreInfo>, CoreError> {
        self.registry.list_factories()
    }

    /// 根据类型获取核心
    pub fn get_cores_by_type(&self, core_type: &CoreType) -> Result<Vec<CoreInfo>, CoreError> {
        self.registry.get_factories_by_type(core_type)
    }

    /// 检查核心是否已加载
    pub fn is_core_loaded(&self, name: &str) -> bool {
        let loaded = self.loaded_cores.read().unwrap();
        loaded.contains_key(name)
    }

    /// 卸载核心
    pub async fn unload_core(&self, name: &str) -> Result<(), CoreError> {
        info!("卸载核心: {}", name);

        // 从注册表中移除
        self.registry.unregister_factory(name)?;

        // 从已加载列表中移除
        {
            let mut loaded = self.loaded_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            loaded.remove(name);
        }

        info!("核心 {} 卸载成功", name);
        Ok(())
    }

    /// 重新加载核心
    pub async fn reload_core(&self, name: &str) -> Result<(), CoreError> {
        info!("重新加载核心: {}", name);

        // 先卸载
        if self.is_core_loaded(name) {
            self.unload_core(name).await?;
        }

        // 重新加载
        match name {
            #[cfg(feature = "software-core")]
            "software" => self.load_software_core().await?,
            #[cfg(feature = "asic-core")]
            "asic" => self.load_asic_core().await?,
            _ => {
                return Err(CoreError::runtime(format!("未知的核心类型: {}", name)));
            }
        }

        info!("核心 {} 重新加载成功", name);
        Ok(())
    }

    /// 获取加载统计信息
    pub fn get_load_stats(&self) -> Result<LoadStats, CoreError> {
        let registry_stats = self.registry.get_stats()?;
        let loaded = self.loaded_cores.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(LoadStats {
            total_loaded: loaded.len(),
            registered_factories: registry_stats.registered_factories,
            active_cores: registry_stats.active_cores,
            core_types: loaded.values().cloned().collect(),
        })
    }

    /// 关闭所有核心
    pub async fn shutdown(&self) -> Result<(), CoreError> {
        info!("关闭所有核心");

        // 关闭所有活跃的核心实例
        self.registry.shutdown_all().await?;

        // 清空已加载列表
        {
            let mut loaded = self.loaded_cores.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            loaded.clear();
        }

        info!("所有核心已关闭");
        Ok(())
    }
}

impl Default for CoreLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// 加载统计信息
#[derive(Debug, Clone)]
pub struct LoadStats {
    /// 已加载的核心数量
    pub total_loaded: usize,
    /// 注册的工厂数量
    pub registered_factories: usize,
    /// 活跃的核心数量
    pub active_cores: usize,
    /// 核心类型列表
    pub core_types: Vec<CoreType>,
}

impl std::fmt::Display for LoadStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "已加载核心: {}, 注册工厂: {}, 活跃核心: {}", 
               self.total_loaded, self.registered_factories, self.active_cores)
    }
}
