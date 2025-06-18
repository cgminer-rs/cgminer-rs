# CGMiner-RS 通用核心使用指南

CGMiner-RS 提供了一套完整的通用核心管理系统，允许统一管理和使用不同类型的挖矿核心。

## 核心架构概述

```
┌─────────────────────────────────────────────────────────────┐
│                    CGMiner-RS 主程序                        │
├─────────────────────────────────────────────────────────────┤
│                   CoreRegistry (核心注册表)                  │
│  ┌─────────────────┬─────────────────┬─────────────────┐    │
│  │  SoftwareCore   │    AsicCore     │   CustomCore    │    │
│  │     Factory     │     Factory     │     Factory     │    │
│  └─────────────────┴─────────────────┴─────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    MiningCore Trait                        │
│  ┌─────────────────┬─────────────────┬─────────────────┐    │
│  │  SoftwareCore   │    AsicCore     │   CustomCore    │    │
│  │   Implementation│  Implementation │  Implementation │    │
│  └─────────────────┴─────────────────┴─────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## 1. 核心特征系统

### CoreFactory Trait
所有核心都必须实现 `CoreFactory` 特征：

```rust
#[async_trait]
pub trait CoreFactory: Send + Sync {
    fn core_type(&self) -> CoreType;
    fn core_info(&self) -> CoreInfo;
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>, CoreError>;
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError>;
    fn default_config(&self) -> CoreConfig;
}
```

### MiningCore Trait
所有核心实例都必须实现 `MiningCore` 特征：

```rust
#[async_trait]
pub trait MiningCore: Send + Sync {
    async fn initialize(&mut self, config: CoreConfig) -> Result<(), CoreError>;
    async fn start(&mut self) -> Result<(), CoreError>;
    async fn stop(&mut self) -> Result<(), CoreError>;
    async fn submit_work(&mut self, work: Work) -> Result<(), CoreError>;
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>, CoreError>;
    async fn get_stats(&self) -> Result<CoreStats, CoreError>;
    // ... 更多方法
}
```

## 2. 通用核心使用方法

### 方法一：通过配置文件使用

在 `cgminer.toml` 中配置：

```toml
[cores]
enabled_cores = ["software", "asic"]  # 启用多种核心
default_core = "software"

[cores.software_core]
enabled = true
device_count = 4
target_hashrate = 2.0
error_rate = 0.01
cpu_affinity = true

[cores.asic_core]
enabled = true
chain_count = 3
auto_detect = true
power_limit = 3000.0
```

### 方法二：通过API动态管理

```rust
// 列出所有可用的核心类型
let available_cores = mining_manager.list_available_cores().await?;
for core_info in available_cores {
    println!("可用核心: {} (类型: {})", core_info.name, core_info.core_type);
}

// 动态创建核心
let config = CoreConfig {
    name: "my_software_core".to_string(),
    enabled: true,
    devices: vec![],
    custom_params: HashMap::new(),
};

let core_id = mining_manager.create_core("software", config).await?;
println!("创建核心成功: {}", core_id);
```

### 方法三：通过核心注册表直接管理

```rust
use cgminer_core::{CoreRegistry, CoreConfig};

// 创建核心注册表
let registry = CoreRegistry::new();

// 注册核心工厂
registry.register_factory(
    "software".to_string(), 
    Box::new(SoftwareCoreFactory::new())
)?;

// 创建核心实例
let core_id = registry.create_core("software", config).await?;

// 获取核心实例
if let Some(core) = registry.get_core(&core_id)? {
    core.start().await?;
}
```

## 3. 核心类型支持

### 内置核心类型

1. **Software Core** (`software`)
   - 软算法挖矿核心
   - 使用CPU进行真实SHA256计算
   - 适用于测试和开发

2. **ASIC Core** (`asic`)
   - ASIC硬件挖矿核心
   - 支持真实ASIC矿机
   - 高性能挖矿

### 自定义核心类型

可以通过实现 `CoreFactory` 和 `MiningCore` 特征来添加自定义核心：

```rust
pub struct CustomCoreFactory {
    core_info: CoreInfo,
}

#[async_trait]
impl CoreFactory for CustomCoreFactory {
    fn core_type(&self) -> CoreType {
        CoreType::Custom("my_custom_core".to_string())
    }
    
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>, CoreError> {
        Ok(Box::new(MyCustomCore::new(config)?))
    }
    
    // ... 实现其他方法
}
```

## 4. 动态核心加载

CGMiner-RS 支持动态加载核心库：

```rust
// 加载所有可用核心
let core_loader = CoreLoader::new(registry);
core_loader.load_all_cores().await?;

// 动态加载特定核心
core_loader.load_dynamic_core("/path/to/custom_core.so").await?;
```

## 5. 核心管理最佳实践

### 统一配置管理
```rust
// 使用统一的配置结构
let config = CoreConfig {
    name: "production_core".to_string(),
    enabled: true,
    devices: device_configs,
    custom_params: {
        let mut params = HashMap::new();
        params.insert("hashrate_target".to_string(), "5.0".to_string());
        params.insert("power_limit".to_string(), "3000".to_string());
        params
    },
};
```

### 错误处理和监控
```rust
// 统一的错误处理
match core.health_check().await {
    Ok(true) => info!("核心 {} 运行正常", core_id),
    Ok(false) => warn!("核心 {} 状态异常", core_id),
    Err(e) => error!("核心 {} 健康检查失败: {}", core_id, e),
}

// 统一的统计信息收集
let stats = core.get_stats().await?;
println!("核心统计: 算力={} GH/s, 设备数={}", 
         stats.total_hashrate / 1e9, stats.device_count);
```

### 生命周期管理
```rust
// 优雅关闭
for (core_id, mut core) in active_cores {
    if let Err(e) = core.stop().await {
        error!("停止核心 {} 失败: {}", core_id, e);
    }
    
    if let Err(e) = core.shutdown().await {
        error!("关闭核心 {} 失败: {}", core_id, e);
    }
}
```

## 6. 总结

CGMiner-RS 的通用核心管理系统提供了：

✅ **统一接口** - 所有核心都实现相同的特征  
✅ **动态加载** - 支持运行时加载新核心  
✅ **配置管理** - 统一的配置格式和验证  
✅ **生命周期管理** - 完整的启动、停止、监控流程  
✅ **扩展性** - 易于添加新的核心类型  
✅ **类型安全** - Rust的类型系统保证安全性  

这个架构使得用户可以：
- 同时使用多种不同类型的挖矿核心
- 通过配置文件或API动态管理核心
- 轻松添加自定义核心实现
- 享受统一的监控和管理体验
