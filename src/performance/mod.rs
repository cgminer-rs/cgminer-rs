//! 性能优化模块 - 简化版本
//!
//! 基础性能监控功能

// 注意：复杂的性能优化功能已移除，只保留基础监控

use std::time::{Duration, Instant};

/// 简化的性能监控器
pub struct PerformanceMonitor {
    /// 启动时间
    start_time: Instant,
    /// 是否启用
    enabled: bool,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            enabled: true,
        }
    }

    /// 获取运行时间
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 启用/禁用监控
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
