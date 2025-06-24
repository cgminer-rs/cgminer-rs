//! 静态核心注册系统 - 编译时注册所有启用的挖矿核心

use cgminer_core::{CoreRegistry, CoreType, CoreInfo, CoreError};
use std::sync::Arc;
use tracing::info;

#[cfg(feature = "cpu-btc")]
use cgminer_cpu_btc_core;

#[cfg(feature = "maijie-l7")]
use cgminer_asic_maijie_l7_core;

#[cfg(feature = "gpu-btc")]
use cgminer_gpu_btc_core;

/// 静态核心注册器 - 在编译时注册所有启用的核心
pub struct StaticCoreRegistry {
    /// 核心注册表
    registry: Arc<CoreRegistry>,
}

impl StaticCoreRegistry {
    /// 创建新的静态核心注册器并注册所有启用的核心
    pub async fn new() -> Result<Self, CoreError> {
        let registry = Arc::new(CoreRegistry::new());
        let instance = Self { registry };

        // 静态注册所有启用的核心
        instance.register_all_cores().await?;

        Ok(instance)
    }

    /// 获取核心注册表
    pub fn registry(&self) -> Arc<CoreRegistry> {
        self.registry.clone()
    }

    /// 静态注册所有启用的核心
    async fn register_all_cores(&self) -> Result<(), CoreError> {
        info!("🔧 开始静态注册所有启用的挖矿核心");

        let mut registered_count = 0;

        // 注册Bitcoin软算法核心
        #[cfg(feature = "cpu-btc")]
        {
            if let Err(e) = self.register_cpu_btc_core().await {
                return Err(CoreError::runtime(format!("❌ 注册Bitcoin软算法核心失败: {}", e)));
            }
            registered_count += 1;
        }

        // 注册Maijie L7 ASIC核心
        #[cfg(feature = "maijie-l7")]
        {
            if let Err(e) = self.register_maijie_l7_core().await {
                return Err(CoreError::runtime(format!("❌ 注册Maijie L7 ASIC核心失败: {}", e)));
            }
            registered_count += 1;
        }

        // 注册GPU Bitcoin核心
        #[cfg(feature = "gpu-btc")]
        {
            if let Err(e) = self.register_gpu_btc_core().await {
                return Err(CoreError::runtime(format!("❌ 注册GPU Bitcoin核心失败: {}", e)));
            }
            registered_count += 1;
        }

        let _stats = self.registry.get_stats().await?;
        info!("✅ 静态核心注册完成，共注册 {} 个核心工厂",
              registered_count);

        Ok(())
    }

    /// 注册Bitcoin软算法核心
    #[cfg(feature = "cpu-btc")]
    async fn register_cpu_btc_core(&self) -> Result<(), CoreError> {
        info!("🔧 注册Bitcoin软算法核心");

        let factory = cgminer_cpu_btc_core::create_factory();
        let core_info = factory.core_info();

        self.registry.register_factory("cpu-btc".to_string(), factory).await?;

        info!("✅ Bitcoin软算法核心注册成功: {} ({})",
              core_info.name, core_info.core_type);
        Ok(())
    }

    /// 注册Maijie L7 ASIC核心
    #[cfg(feature = "maijie-l7")]
    async fn register_maijie_l7_core(&self) -> Result<(), CoreError> {
        info!("🔧 注册Maijie L7 ASIC核心");

        let factory = cgminer_asic_maijie_l7_core::create_factory();
        let core_info = factory.core_info();

        self.registry.register_factory("maijie-l7".to_string(), factory).await?;

        info!("✅ Maijie L7 ASIC核心注册成功: {} ({})",
              core_info.name, core_info.core_type);
        Ok(())
    }

    /// 注册GPU Bitcoin核心
    #[cfg(feature = "gpu-btc")]
    async fn register_gpu_btc_core(&self) -> Result<(), CoreError> {
        info!("🔧 注册GPU Bitcoin核心");

        let factory = cgminer_gpu_btc_core::create_factory();
        let core_info = factory.core_info();

        self.registry.register_factory("gpu-btc".to_string(), factory).await?;

        info!("✅ GPU Bitcoin核心注册成功: {} ({})",
              core_info.name, core_info.core_type);
        Ok(())
    }



    /// 列出所有已注册的核心
    pub async fn list_registered_cores(&self) -> Result<Vec<CoreInfo>, CoreError> {
        self.registry.list_factories().await
    }

    /// 根据类型获取核心
    pub async fn get_cores_by_type(&self, core_type: &CoreType) -> Result<Vec<CoreInfo>, CoreError> {
        self.registry.get_factories_by_type(core_type).await
    }

    /// 获取注册统计信息
    pub async fn get_registry_stats(&self) -> Result<RegistryStats, CoreError> {
        let registry_stats = self.registry.get_stats().await?;

        Ok(RegistryStats {
            registered_factories: registry_stats.registered_factories,
            active_cores: registry_stats.active_cores,
        })
    }

    /// 关闭所有核心
    pub async fn shutdown(&self) -> Result<(), CoreError> {
        info!("🔧 关闭所有核心");

        // 关闭所有活跃的核心实例
        self.registry.shutdown_all().await?;

        info!("✅ 所有核心已关闭");
        Ok(())
    }
}

/// 注册统计信息
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// 注册的工厂数量
    pub registered_factories: usize,
    /// 活跃的核心数量
    pub active_cores: usize,
}

impl std::fmt::Display for RegistryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.active_cores == 0 {
            // 在静态注册阶段，不显示活跃核心数量（总是0）
            write!(f, "注册工厂: {}", self.registered_factories)
        } else {
            // 在运行时显示完整信息
            write!(f, "注册工厂: {}, 活跃核心: {}",
                   self.registered_factories, self.active_cores)
        }
    }
}
