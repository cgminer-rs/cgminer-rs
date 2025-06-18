//! 多矿池管理示例

use cgminer_rs::pool::{
    PoolManager, PoolScheduler, PoolSwitcher, PoolStrategy, 
    PoolMetrics, SwitchConfig, FailoverConfig
};
use cgminer_rs::config::{Config, PoolConfig, PoolInfo};
use cgminer_rs::error::PoolError;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("🚀 启动多矿池管理示例");

    // 创建多矿池配置
    let pool_config = create_multi_pool_config();
    
    // 演示不同的矿池策略
    demo_failover_strategy(&pool_config).await?;
    demo_load_balance_strategy(&pool_config).await?;
    demo_intelligent_switching(&pool_config).await?;

    info!("✅ 多矿池管理示例完成");
    Ok(())
}

/// 创建多矿池配置
fn create_multi_pool_config() -> PoolConfig {
    PoolConfig {
        strategy: PoolStrategy::Failover,
        failover_timeout: 30,
        retry_interval: 10,
        pools: vec![
            // F2Pool 主矿池
            PoolInfo {
                url: "stratum+tcp://btc.f2pool.com:1314".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "123".to_string(),
                priority: 1,
                quota: Some(50), // 50%配额
                enabled: true,
            },
            // F2Pool 备用矿池
            PoolInfo {
                url: "stratum+tcp://btc.f2pool.com:25".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "123".to_string(),
                priority: 2,
                quota: Some(30), // 30%配额
                enabled: true,
            },
            // AntPool 备用矿池
            PoolInfo {
                url: "stratum+tcp://pool.antpool.com:3333".to_string(),
                user: "kayuii.bbt".to_string(),
                password: "123".to_string(),
                priority: 3,
                quota: Some(20), // 20%配额
                enabled: true,
            },
        ],
    }
}

/// 演示故障转移策略
async fn demo_failover_strategy(pool_config: &PoolConfig) -> Result<(), PoolError> {
    info!("📋 演示故障转移策略");

    // 创建矿池管理器
    let pool_manager = PoolManager::new(pool_config.clone()).await?;
    
    // 创建调度器
    let failover_config = FailoverConfig {
        failure_timeout: Duration::from_secs(30),
        retry_interval: Duration::from_secs(10),
        max_retries: 3,
        auto_recovery: true,
    };
    
    let scheduler = PoolScheduler::new(PoolStrategy::Failover, Some(failover_config));

    // 添加矿池到调度器
    for (index, pool_info) in pool_config.pools.iter().enumerate() {
        let pool = cgminer_rs::pool::Pool::new(
            index as u32,
            pool_info.url.clone(),
            pool_info.user.clone(),
            pool_info.password.clone(),
            pool_info.priority,
            pool_info.enabled,
        );
        scheduler.add_pool(pool).await?;
    }

    // 启动健康检查
    scheduler.start_health_check().await?;

    // 模拟工作分配
    for i in 0..10 {
        match scheduler.select_pool_for_work().await {
            Ok(pool_id) => {
                info!("🎯 工作 {} 分配给矿池 {}", i, pool_id);
            }
            Err(e) => {
                error!("❌ 工作分配失败: {}", e);
            }
        }
        sleep(Duration::from_millis(500)).await;
    }

    // 模拟矿池故障
    info!("⚠️ 模拟主矿池故障");
    scheduler.handle_pool_failure(0, &PoolError::ConnectionFailed {
        url: "btc.f2pool.com:1314".to_string(),
        error: "Connection timeout".to_string(),
    }).await;

    // 继续工作分配
    for i in 10..15 {
        match scheduler.select_pool_for_work().await {
            Ok(pool_id) => {
                info!("🎯 故障转移后工作 {} 分配给矿池 {}", i, pool_id);
            }
            Err(e) => {
                error!("❌ 工作分配失败: {}", e);
            }
        }
        sleep(Duration::from_millis(500)).await;
    }

    // 获取统计信息
    let stats = scheduler.get_scheduler_stats().await;
    info!("📊 调度统计: 总调度次数={}, 故障转移次数={}", 
          stats.total_schedules, stats.failover_count);

    scheduler.stop_health_check().await?;
    info!("✅ 故障转移策略演示完成");
    Ok(())
}

/// 演示负载均衡策略
async fn demo_load_balance_strategy(pool_config: &PoolConfig) -> Result<(), PoolError> {
    info!("⚖️ 演示负载均衡策略");

    let scheduler = PoolScheduler::new(PoolStrategy::LoadBalance, None);

    // 添加矿池并设置权重
    for (index, pool_info) in pool_config.pools.iter().enumerate() {
        let pool = cgminer_rs::pool::Pool::new(
            index as u32,
            pool_info.url.clone(),
            pool_info.user.clone(),
            pool_info.password.clone(),
            pool_info.priority,
            pool_info.enabled,
        );
        scheduler.add_pool(pool).await?;
        
        // 设置不同的权重
        let weight = match index {
            0 => 2.0, // 主矿池权重更高
            1 => 1.5,
            2 => 1.0,
            _ => 1.0,
        };
        scheduler.set_pool_weight(index as u32, weight).await;
    }

    // 模拟负载均衡工作分配
    let mut pool_usage = std::collections::HashMap::new();
    
    for i in 0..20 {
        match scheduler.select_pool_for_work().await {
            Ok(pool_id) => {
                *pool_usage.entry(pool_id).or_insert(0) += 1;
                info!("🎯 负载均衡工作 {} 分配给矿池 {}", i, pool_id);
            }
            Err(e) => {
                error!("❌ 工作分配失败: {}", e);
            }
        }
        sleep(Duration::from_millis(200)).await;
    }

    // 显示负载分布
    info!("📊 负载分布:");
    for (pool_id, count) in pool_usage {
        info!("   矿池 {}: {} 次工作 ({:.1}%)", 
              pool_id, count, count as f64 / 20.0 * 100.0);
    }

    info!("✅ 负载均衡策略演示完成");
    Ok(())
}

/// 演示智能切换
async fn demo_intelligent_switching(pool_config: &PoolConfig) -> Result<(), PoolError> {
    info!("🧠 演示智能矿池切换");

    // 创建切换配置
    let switch_config = SwitchConfig {
        auto_switch_enabled: true,
        check_interval: Duration::from_secs(5),
        latency_threshold: 1000, // 1秒
        accept_rate_threshold: 95.0, // 95%
        stability_threshold: 0.9,
        switch_cooldown: Duration::from_secs(10),
        min_performance_diff: 0.1,
    };

    let switcher = PoolSwitcher::new(Some(switch_config));

    // 启动自动切换
    switcher.start_auto_switch().await?;

    // 模拟不同矿池的性能指标
    let pool_metrics = vec![
        PoolMetrics {
            pool_id: 0,
            avg_latency: Duration::from_millis(200),
            accept_rate: 98.5,
            reject_rate: 1.5,
            stale_rate: 0.5,
            connection_stability: 0.95,
            difficulty: 1000.0,
            last_update: std::time::SystemTime::now(),
            performance_score: 0.0, // 将被计算
        },
        PoolMetrics {
            pool_id: 1,
            avg_latency: Duration::from_millis(500),
            accept_rate: 96.0,
            reject_rate: 4.0,
            stale_rate: 1.0,
            connection_stability: 0.90,
            difficulty: 1000.0,
            last_update: std::time::SystemTime::now(),
            performance_score: 0.0,
        },
        PoolMetrics {
            pool_id: 2,
            avg_latency: Duration::from_millis(800),
            accept_rate: 94.0,
            reject_rate: 6.0,
            stale_rate: 2.0,
            connection_stability: 0.85,
            difficulty: 1000.0,
            last_update: std::time::SystemTime::now(),
            performance_score: 0.0,
        },
    ];

    // 更新矿池指标
    for mut metrics in pool_metrics {
        metrics.calculate_performance_score();
        info!("📊 矿池 {} 性能分数: {:.3}", metrics.pool_id, metrics.performance_score);
        switcher.update_pool_metrics(metrics.pool_id, metrics).await;
    }

    // 手动切换到最佳矿池
    switcher.switch_to_pool(0).await?;
    info!("🔄 手动切换到矿池 0");

    // 模拟性能变化
    sleep(Duration::from_secs(2)).await;
    
    // 模拟矿池0性能下降
    let mut degraded_metrics = PoolMetrics {
        pool_id: 0,
        avg_latency: Duration::from_millis(2000), // 延迟增加
        accept_rate: 90.0, // 接受率下降
        reject_rate: 10.0,
        stale_rate: 3.0,
        connection_stability: 0.70, // 稳定性下降
        difficulty: 1000.0,
        last_update: std::time::SystemTime::now(),
        performance_score: 0.0,
    };
    degraded_metrics.calculate_performance_score();
    
    info!("⚠️ 矿池 0 性能下降，新分数: {:.3}", degraded_metrics.performance_score);
    switcher.update_pool_metrics(0, degraded_metrics).await;

    // 等待自动切换
    sleep(Duration::from_secs(10)).await;

    // 获取切换统计
    let switch_stats = switcher.get_switch_stats().await;
    info!("📊 切换统计:");
    info!("   总切换次数: {}", switch_stats.total_switches);
    info!("   成功率: {:.1}%", switch_stats.success_rate());
    info!("   平均切换时间: {:?}", switch_stats.avg_switch_time);

    // 获取切换历史
    let history = switcher.get_switch_history(Some(5)).await;
    info!("📜 最近切换历史:");
    for event in history {
        info!("   {:?} -> {} (原因: {:?}, 成功: {})", 
              event.from_pool, event.to_pool, event.reason, event.success);
    }

    switcher.stop_auto_switch().await?;
    info!("✅ 智能切换演示完成");
    Ok(())
}
