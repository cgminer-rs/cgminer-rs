use crate::config::AlertThresholds;
use crate::error::MiningError;
use crate::monitoring::{SystemMetrics, DeviceMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{info, warn, debug};
use uuid::Uuid;

/// 告警类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertType {
    /// 系统告警
    System,
    /// 设备告警
    Device,
    /// 矿池告警
    Pool,
    /// 挖矿告警
    Mining,
}

/// 告警严重程度
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重
    Critical,
}

/// 告警状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertStatus {
    /// 活跃
    Active,
    /// 已解决
    Resolved,
    /// 已确认
    Acknowledged,
    /// 已抑制
    Suppressed,
}

/// 告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// 告警ID
    pub id: String,
    /// 告警类型
    pub alert_type: AlertType,
    /// 严重程度
    pub severity: AlertSeverity,
    /// 告警状态
    pub status: AlertStatus,
    /// 告警标题
    pub title: String,
    /// 告警描述
    pub description: String,
    /// 告警源
    pub source: String,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 触发时间
    pub triggered_at: SystemTime,
    /// 解决时间
    pub resolved_at: Option<SystemTime>,
    /// 确认时间
    pub acknowledged_at: Option<SystemTime>,
    /// 告警值
    pub value: Option<f64>,
    /// 阈值
    pub threshold: Option<f64>,
}

impl Alert {
    /// 创建新的告警
    pub fn new(
        alert_type: AlertType,
        severity: AlertSeverity,
        title: String,
        description: String,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            alert_type,
            severity,
            status: AlertStatus::Active,
            title,
            description,
            source,
            labels: HashMap::new(),
            triggered_at: SystemTime::now(),
            resolved_at: None,
            acknowledged_at: None,
            value: None,
            threshold: None,
        }
    }
    
    /// 添加标签
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }
    
    /// 设置值和阈值
    pub fn with_values(mut self, value: f64, threshold: f64) -> Self {
        self.value = Some(value);
        self.threshold = Some(threshold);
        self
    }
    
    /// 解决告警
    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(SystemTime::now());
    }
    
    /// 确认告警
    pub fn acknowledge(&mut self) {
        self.status = AlertStatus::Acknowledged;
        self.acknowledged_at = Some(SystemTime::now());
    }
    
    /// 抑制告警
    pub fn suppress(&mut self) {
        self.status = AlertStatus::Suppressed;
    }
    
    /// 获取告警持续时间
    pub fn get_duration(&self) -> Duration {
        let end_time = self.resolved_at.unwrap_or_else(SystemTime::now);
        end_time.duration_since(self.triggered_at).unwrap_or(Duration::from_secs(0))
    }
    
    /// 检查告警是否活跃
    pub fn is_active(&self) -> bool {
        self.status == AlertStatus::Active
    }
}

/// 告警规则
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// 规则名称
    pub name: String,
    /// 告警类型
    pub alert_type: AlertType,
    /// 严重程度
    pub severity: AlertSeverity,
    /// 条件表达式
    pub condition: AlertCondition,
    /// 持续时间
    pub duration: Duration,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 描述模板
    pub description_template: String,
}

/// 告警条件
#[derive(Debug, Clone)]
pub enum AlertCondition {
    /// 大于阈值
    GreaterThan(f64),
    /// 小于阈值
    LessThan(f64),
    /// 等于值
    Equals(f64),
    /// 不等于值
    NotEquals(f64),
    /// 在范围内
    InRange(f64, f64),
    /// 不在范围内
    OutOfRange(f64, f64),
}

impl AlertCondition {
    /// 检查条件是否满足
    pub fn check(&self, value: f64) -> bool {
        match self {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::Equals(target) => (value - target).abs() < f64::EPSILON,
            AlertCondition::NotEquals(target) => (value - target).abs() >= f64::EPSILON,
            AlertCondition::InRange(min, max) => value >= *min && value <= *max,
            AlertCondition::OutOfRange(min, max) => value < *min || value > *max,
        }
    }
}

/// 告警管理器
pub struct AlertManager {
    /// 活跃告警
    active_alerts: HashMap<String, Alert>,
    /// 告警历史
    alert_history: Vec<Alert>,
    /// 告警规则
    alert_rules: Vec<AlertRule>,
    /// 告警阈值配置
    thresholds: AlertThresholds,
    /// 最大历史记录数
    max_history: usize,
}

impl AlertManager {
    /// 创建新的告警管理器
    pub fn new(thresholds: AlertThresholds) -> Self {
        let mut manager = Self {
            active_alerts: HashMap::new(),
            alert_history: Vec::new(),
            alert_rules: Vec::new(),
            thresholds,
            max_history: 1000,
        };
        
        // 初始化默认告警规则
        manager.init_default_rules();
        
        manager
    }
    
    /// 初始化默认告警规则
    fn init_default_rules(&mut self) {
        // 系统温度告警
        self.alert_rules.push(AlertRule {
            name: "high_system_temperature".to_string(),
            alert_type: AlertType::System,
            severity: AlertSeverity::Warning,
            condition: AlertCondition::GreaterThan(self.thresholds.max_temperature as f64),
            duration: Duration::from_secs(60),
            labels: HashMap::new(),
            description_template: "System temperature is {value}°C, exceeding threshold of {threshold}°C".to_string(),
        });
        
        // CPU使用率告警
        self.alert_rules.push(AlertRule {
            name: "high_cpu_usage".to_string(),
            alert_type: AlertType::System,
            severity: AlertSeverity::Warning,
            condition: AlertCondition::GreaterThan(self.thresholds.max_cpu_usage as f64),
            duration: Duration::from_secs(300),
            labels: HashMap::new(),
            description_template: "CPU usage is {value}%, exceeding threshold of {threshold}%".to_string(),
        });
        
        // 内存使用率告警
        self.alert_rules.push(AlertRule {
            name: "high_memory_usage".to_string(),
            alert_type: AlertType::System,
            severity: AlertSeverity::Warning,
            condition: AlertCondition::GreaterThan(self.thresholds.max_memory_usage as f64),
            duration: Duration::from_secs(300),
            labels: HashMap::new(),
            description_template: "Memory usage is {value}%, exceeding threshold of {threshold}%".to_string(),
        });
        
        // 设备温度告警
        self.alert_rules.push(AlertRule {
            name: "high_device_temperature".to_string(),
            alert_type: AlertType::Device,
            severity: AlertSeverity::Error,
            condition: AlertCondition::GreaterThan(self.thresholds.max_device_temperature as f64),
            duration: Duration::from_secs(30),
            labels: HashMap::new(),
            description_template: "Device temperature is {value}°C, exceeding threshold of {threshold}°C".to_string(),
        });
        
        // 设备错误率告警
        self.alert_rules.push(AlertRule {
            name: "high_device_error_rate".to_string(),
            alert_type: AlertType::Device,
            severity: AlertSeverity::Warning,
            condition: AlertCondition::GreaterThan(self.thresholds.max_error_rate as f64),
            duration: Duration::from_secs(120),
            labels: HashMap::new(),
            description_template: "Device error rate is {value}%, exceeding threshold of {threshold}%".to_string(),
        });
    }
    
    /// 检查系统告警
    pub async fn check_system_alerts(&mut self, metrics: &SystemMetrics) -> Result<Vec<Alert>, MiningError> {
        let mut alerts = Vec::new();
        
        // 检查系统温度
        if metrics.temperature > self.thresholds.max_temperature {
            let alert = Alert::new(
                AlertType::System,
                AlertSeverity::Warning,
                "High System Temperature".to_string(),
                format!("System temperature is {:.1}°C, exceeding threshold of {}°C", 
                       metrics.temperature, self.thresholds.max_temperature),
                "system".to_string(),
            )
            .with_label("metric".to_string(), "temperature".to_string())
            .with_values(metrics.temperature as f64, self.thresholds.max_temperature as f64);
            
            alerts.push(alert);
        }
        
        // 检查CPU使用率
        if metrics.cpu_usage > self.thresholds.max_cpu_usage as f64 {
            let alert = Alert::new(
                AlertType::System,
                AlertSeverity::Warning,
                "High CPU Usage".to_string(),
                format!("CPU usage is {:.1}%, exceeding threshold of {}%", 
                       metrics.cpu_usage, self.thresholds.max_cpu_usage),
                "system".to_string(),
            )
            .with_label("metric".to_string(), "cpu_usage".to_string())
            .with_values(metrics.cpu_usage, self.thresholds.max_cpu_usage as f64);
            
            alerts.push(alert);
        }
        
        // 检查内存使用率
        if metrics.memory_usage > self.thresholds.max_memory_usage as f64 {
            let alert = Alert::new(
                AlertType::System,
                AlertSeverity::Warning,
                "High Memory Usage".to_string(),
                format!("Memory usage is {:.1}%, exceeding threshold of {}%", 
                       metrics.memory_usage, self.thresholds.max_memory_usage),
                "system".to_string(),
            )
            .with_label("metric".to_string(), "memory_usage".to_string())
            .with_values(metrics.memory_usage, self.thresholds.max_memory_usage as f64);
            
            alerts.push(alert);
        }
        
        // 处理新告警
        for alert in &alerts {
            self.process_alert(alert.clone()).await?;
        }
        
        Ok(alerts)
    }
    
    /// 检查设备告警
    pub async fn check_device_alerts(&mut self, metrics: &DeviceMetrics) -> Result<Vec<Alert>, MiningError> {
        let mut alerts = Vec::new();
        
        // 检查设备温度
        if metrics.temperature > self.thresholds.max_device_temperature {
            let alert = Alert::new(
                AlertType::Device,
                AlertSeverity::Error,
                "High Device Temperature".to_string(),
                format!("Device {} temperature is {:.1}°C, exceeding threshold of {}°C", 
                       metrics.device_id, metrics.temperature, self.thresholds.max_device_temperature),
                format!("device_{}", metrics.device_id),
            )
            .with_label("device_id".to_string(), metrics.device_id.to_string())
            .with_label("metric".to_string(), "temperature".to_string())
            .with_values(metrics.temperature as f64, self.thresholds.max_device_temperature as f64);
            
            alerts.push(alert);
        }
        
        // 检查设备错误率
        if metrics.error_rate > self.thresholds.max_error_rate {
            let alert = Alert::new(
                AlertType::Device,
                AlertSeverity::Warning,
                "High Device Error Rate".to_string(),
                format!("Device {} error rate is {:.2}%, exceeding threshold of {}%", 
                       metrics.device_id, metrics.error_rate, self.thresholds.max_error_rate),
                format!("device_{}", metrics.device_id),
            )
            .with_label("device_id".to_string(), metrics.device_id.to_string())
            .with_label("metric".to_string(), "error_rate".to_string())
            .with_values(metrics.error_rate, self.thresholds.max_error_rate);
            
            alerts.push(alert);
        }
        
        // 检查设备算力
        if metrics.hashrate < self.thresholds.min_hashrate {
            let alert = Alert::new(
                AlertType::Device,
                AlertSeverity::Warning,
                "Low Device Hashrate".to_string(),
                format!("Device {} hashrate is {:.1} GH/s, below threshold of {} GH/s", 
                       metrics.device_id, metrics.hashrate, self.thresholds.min_hashrate),
                format!("device_{}", metrics.device_id),
            )
            .with_label("device_id".to_string(), metrics.device_id.to_string())
            .with_label("metric".to_string(), "hashrate".to_string())
            .with_values(metrics.hashrate, self.thresholds.min_hashrate);
            
            alerts.push(alert);
        }
        
        // 处理新告警
        for alert in &alerts {
            self.process_alert(alert.clone()).await?;
        }
        
        Ok(alerts)
    }
    
    /// 处理告警
    async fn process_alert(&mut self, alert: Alert) -> Result<(), MiningError> {
        let alert_key = format!("{}_{}", alert.source, alert.title.replace(" ", "_").to_lowercase());
        
        // 检查是否已存在相同的活跃告警
        if let Some(existing_alert) = self.active_alerts.get_mut(&alert_key) {
            // 更新现有告警
            existing_alert.value = alert.value;
            existing_alert.triggered_at = alert.triggered_at;
            debug!("Updated existing alert: {}", alert_key);
        } else {
            // 添加新告警
            info!("New alert triggered: {} - {}", alert.title, alert.description);
            self.active_alerts.insert(alert_key, alert.clone());
        }
        
        // 添加到历史记录
        self.add_to_history(alert);
        
        Ok(())
    }
    
    /// 解决告警
    pub async fn resolve_alert(&mut self, alert_id: &str) -> Result<(), MiningError> {
        if let Some(mut alert) = self.active_alerts.remove(alert_id) {
            alert.resolve();
            self.add_to_history(alert);
            info!("Alert resolved: {}", alert_id);
        }
        
        Ok(())
    }
    
    /// 确认告警
    pub async fn acknowledge_alert(&mut self, alert_id: &str) -> Result<(), MiningError> {
        if let Some(alert) = self.active_alerts.get_mut(alert_id) {
            alert.acknowledge();
            info!("Alert acknowledged: {}", alert_id);
        }
        
        Ok(())
    }
    
    /// 获取活跃告警
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts.values().collect()
    }
    
    /// 获取告警历史
    pub fn get_alert_history(&self) -> &Vec<Alert> {
        &self.alert_history
    }
    
    /// 获取告警统计
    pub fn get_alert_stats(&self) -> AlertStats {
        let active_count = self.active_alerts.len();
        let total_count = self.alert_history.len();
        
        let mut severity_counts = HashMap::new();
        for alert in self.active_alerts.values() {
            *severity_counts.entry(alert.severity.clone()).or_insert(0) += 1;
        }
        
        AlertStats {
            active_alerts: active_count,
            total_alerts: total_count,
            severity_counts,
        }
    }
    
    /// 添加到历史记录
    fn add_to_history(&mut self, alert: Alert) {
        self.alert_history.push(alert);
        
        // 限制历史记录大小
        if self.alert_history.len() > self.max_history {
            self.alert_history.remove(0);
        }
    }
    
    /// 清理已解决的告警
    pub fn cleanup_resolved_alerts(&mut self) {
        let resolved_keys: Vec<String> = self.active_alerts
            .iter()
            .filter(|(_, alert)| !alert.is_active())
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in resolved_keys {
            if let Some(mut alert) = self.active_alerts.remove(&key) {
                alert.resolve();
                self.add_to_history(alert);
            }
        }
    }
}

/// 告警统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertStats {
    pub active_alerts: usize,
    pub total_alerts: usize,
    pub severity_counts: HashMap<AlertSeverity, u32>,
}
