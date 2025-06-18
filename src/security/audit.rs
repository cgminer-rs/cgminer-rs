//! 审计日志模块

use crate::error::MiningError;
use crate::security::config::AuditConfig;
use crate::security::{SecurityEvent, SecurityEventType, SecuritySeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write, BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 审计日志管理器
pub struct AuditLogger {
    /// 配置
    config: AuditConfig,
    /// 当前日志文件
    current_log_file: Option<BufWriter<File>>,
    /// 日志文件路径
    log_file_path: PathBuf,
    /// 当前日志文件大小
    current_file_size: u64,
    /// 日志文件计数器
    file_counter: u32,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// 条目ID
    pub id: String,
    /// 时间戳
    pub timestamp: u64,
    /// 事件类型
    pub event_type: String,
    /// 严重程度
    pub severity: String,
    /// 描述
    pub description: String,
    /// 用户ID
    pub user_id: Option<String>,
    /// 源IP地址
    pub source_ip: Option<String>,
    /// 资源
    pub resource: Option<String>,
    /// 操作
    pub action: Option<String>,
    /// 结果
    pub result: Option<String>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 审计查询条件
#[derive(Debug, Clone)]
pub struct AuditQuery {
    /// 开始时间
    pub start_time: Option<SystemTime>,
    /// 结束时间
    pub end_time: Option<SystemTime>,
    /// 事件类型过滤
    pub event_types: Vec<SecurityEventType>,
    /// 严重程度过滤
    pub severities: Vec<SecuritySeverity>,
    /// 用户ID过滤
    pub user_id: Option<String>,
    /// 源IP过滤
    pub source_ip: Option<String>,
    /// 最大结果数
    pub limit: Option<usize>,
}

/// 审计统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    /// 总事件数
    pub total_events: u64,
    /// 按类型分组的事件数
    pub events_by_type: HashMap<String, u64>,
    /// 按严重程度分组的事件数
    pub events_by_severity: HashMap<String, u64>,
    /// 按用户分组的事件数
    pub events_by_user: HashMap<String, u64>,
    /// 最近24小时事件数
    pub events_last_24h: u64,
    /// 最近7天事件数
    pub events_last_7d: u64,
}

impl AuditLogger {
    /// 创建新的审计日志管理器
    pub fn new(config: AuditConfig) -> Result<Self, MiningError> {
        let log_file_path = PathBuf::from(&config.log_path);

        // 确保日志目录存在
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MiningError::security(format!("创建日志目录失败: {}", e)))?;
        }

        Ok(Self {
            config,
            current_log_file: None,
            log_file_path,
            current_file_size: 0,
            file_counter: 0,
        })
    }

    /// 初始化审计日志管理器
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        info!("📝 初始化审计日志管理器");

        if self.config.enabled {
            // 打开日志文件
            self.open_log_file().await?;

            // 清理旧日志文件
            self.cleanup_old_logs().await?;

            info!("✅ 审计日志管理器初始化完成");
        } else {
            info!("📝 审计日志功能已禁用");
        }

        Ok(())
    }

    /// 记录安全事件
    pub async fn log_event(&mut self, event: SecurityEvent) -> Result<(), MiningError> {
        if !self.config.enabled {
            return Ok(());
        }

        // 检查事件过滤器
        if !self.should_log_event(&event) {
            return Ok(());
        }

        // 创建审计日志条目
        let log_entry = self.create_log_entry(event)?;

        // 写入日志
        self.write_log_entry(&log_entry).await?;

        // 检查是否需要轮转日志文件
        self.check_log_rotation().await?;

        Ok(())
    }

    /// 查询审计日志
    pub async fn query_logs(&self, query: AuditQuery) -> Result<Vec<AuditLogEntry>, MiningError> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let mut results = Vec::new();
        let mut count = 0;

        // 读取当前日志文件
        if let Err(e) = self.search_log_file(&self.log_file_path, &query, &mut results, &mut count) {
            warn!("搜索当前日志文件失败: {}", e);
        }

        // 如果需要更多结果，搜索历史日志文件
        if query.limit.is_none() || count < query.limit.unwrap() {
            self.search_historical_logs(&query, &mut results, &mut count).await?;
        }

        // 按时间戳排序
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 应用限制
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// 获取审计统计信息
    pub async fn get_stats(&self) -> Result<AuditStats, MiningError> {
        if !self.config.enabled {
            return Ok(AuditStats {
                total_events: 0,
                events_by_type: HashMap::new(),
                events_by_severity: HashMap::new(),
                events_by_user: HashMap::new(),
                events_last_24h: 0,
                events_last_7d: 0,
            });
        }

        let mut stats = AuditStats {
            total_events: 0,
            events_by_type: HashMap::new(),
            events_by_severity: HashMap::new(),
            events_by_user: HashMap::new(),
            events_last_24h: 0,
            events_last_7d: 0,
        };

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let day_ago = now - 86400;
        let week_ago = now - 86400 * 7;

        // 统计当前日志文件
        if let Err(e) = self.collect_stats_from_file(&self.log_file_path, &mut stats, day_ago, week_ago) {
            warn!("统计当前日志文件失败: {}", e);
        }

        // 统计历史日志文件
        self.collect_stats_from_historical_logs(&mut stats, day_ago, week_ago).await?;

        Ok(stats)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool, MiningError> {
        if !self.config.enabled {
            return Ok(true);
        }

        // 检查日志文件是否可写
        if !self.log_file_path.exists() {
            return Ok(false);
        }

        // 检查磁盘空间
        // 这里简化处理，实际应该检查磁盘可用空间

        Ok(true)
    }

    /// 打开日志文件
    async fn open_log_file(&mut self) -> Result<(), MiningError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .map_err(|e| MiningError::security(format!("打开日志文件失败: {}", e)))?;

        // 获取当前文件大小
        self.current_file_size = file.metadata()
            .map_err(|e| MiningError::security(format!("获取文件大小失败: {}", e)))?
            .len();

        self.current_log_file = Some(BufWriter::new(file));

        debug!("审计日志文件已打开: {:?}", self.log_file_path);
        Ok(())
    }

    /// 创建日志条目
    fn create_log_entry(&self, event: SecurityEvent) -> Result<AuditLogEntry, MiningError> {
        Ok(AuditLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: event.timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: format!("{:?}", event.event_type),
            severity: format!("{:?}", event.severity),
            description: event.description,
            user_id: event.user_id,
            source_ip: event.source_ip,
            resource: None, // 可以从元数据中提取
            action: None,   // 可以从元数据中提取
            result: None,   // 可以从元数据中提取
            metadata: event.metadata,
        })
    }

    /// 写入日志条目
    async fn write_log_entry(&mut self, entry: &AuditLogEntry) -> Result<(), MiningError> {
        let log_line = match self.config.log_format.as_str() {
            "json" => serde_json::to_string(entry)
                .map_err(|e| MiningError::security(format!("序列化日志条目失败: {}", e)))?,
            "text" => format!(
                "{} [{}] {} - {} (User: {}, IP: {})",
                entry.timestamp,
                entry.severity,
                entry.event_type,
                entry.description,
                entry.user_id.as_deref().unwrap_or("N/A"),
                entry.source_ip.as_deref().unwrap_or("N/A")
            ),
            _ => return Err(MiningError::security("不支持的日志格式".to_string())),
        };

        if let Some(ref mut writer) = self.current_log_file {
            writeln!(writer, "{}", log_line)
                .map_err(|e| MiningError::security(format!("写入日志失败: {}", e)))?;

            writer.flush()
                .map_err(|e| MiningError::security(format!("刷新日志缓冲区失败: {}", e)))?;

            self.current_file_size += log_line.len() as u64 + 1; // +1 for newline
        }

        Ok(())
    }

    /// 检查是否应该记录事件
    fn should_log_event(&self, event: &SecurityEvent) -> bool {
        // 检查事件类型过滤器
        if !self.config.event_filters.is_empty() {
            let event_type_str = format!("{:?}", event.event_type);
            if !self.config.event_filters.contains(&event_type_str) {
                return false;
            }
        }

        // 检查是否记录敏感数据
        if !self.config.log_sensitive_data {
            // 这里可以添加敏感数据检测逻辑
            // 简化处理，假设包含"password"的事件为敏感事件
            if event.description.to_lowercase().contains("password") {
                return false;
            }
        }

        true
    }

    /// 检查日志轮转
    async fn check_log_rotation(&mut self) -> Result<(), MiningError> {
        if self.current_file_size > self.config.max_log_size {
            self.rotate_log_file().await?;
        }
        Ok(())
    }

    /// 轮转日志文件
    async fn rotate_log_file(&mut self) -> Result<(), MiningError> {
        info!("🔄 开始轮转审计日志文件");

        // 关闭当前文件
        self.current_log_file = None;

        // 重命名当前文件
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let rotated_path = self.log_file_path.with_extension(format!("log.{}", timestamp));

        std::fs::rename(&self.log_file_path, &rotated_path)
            .map_err(|e| MiningError::security(format!("重命名日志文件失败: {}", e)))?;

        // 打开新的日志文件
        self.current_file_size = 0;
        self.file_counter += 1;
        self.open_log_file().await?;

        info!("✅ 日志文件轮转完成");
        Ok(())
    }

    /// 清理旧日志文件
    async fn cleanup_old_logs(&self) -> Result<(), MiningError> {
        // 获取日志目录中的所有日志文件
        let log_dir = self.log_file_path.parent().unwrap_or(Path::new("."));
        let log_name = self.log_file_path.file_stem().unwrap().to_str().unwrap();

        let mut log_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(log_name) && file_name.ends_with(".log") && file_name != self.log_file_path.file_name().unwrap().to_str().unwrap() {
                        if let Ok(metadata) = entry.metadata() {
                            log_files.push((path, metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }

        // 按修改时间排序
        log_files.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除超过限制的文件
        if log_files.len() > self.config.max_log_files as usize {
            for (path, _) in log_files.iter().skip(self.config.max_log_files as usize) {
                if let Err(e) = std::fs::remove_file(path) {
                    warn!("删除旧日志文件失败: {} - {}", path.display(), e);
                } else {
                    debug!("删除旧日志文件: {}", path.display());
                }
            }
        }

        Ok(())
    }

    /// 搜索日志文件
    fn search_log_file(&self, file_path: &Path, query: &AuditQuery, results: &mut Vec<AuditLogEntry>, count: &mut usize) -> Result<(), Box<dyn std::error::Error>> {
        if !file_path.exists() {
            return Ok(());
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;

            if let Ok(entry) = serde_json::from_str::<AuditLogEntry>(&line) {
                if self.matches_query(&entry, query) {
                    results.push(entry);
                    *count += 1;

                    if let Some(limit) = query.limit {
                        if *count >= limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 搜索历史日志文件
    async fn search_historical_logs(&self, _query: &AuditQuery, _results: &mut Vec<AuditLogEntry>, _count: &mut usize) -> Result<(), MiningError> {
        // 简化实现，实际应该搜索所有历史日志文件
        Ok(())
    }

    /// 检查条目是否匹配查询条件
    fn matches_query(&self, entry: &AuditLogEntry, query: &AuditQuery) -> bool {
        // 时间范围检查
        if let Some(start_time) = query.start_time {
            let start_timestamp = start_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
            if entry.timestamp < start_timestamp {
                return false;
            }
        }

        if let Some(end_time) = query.end_time {
            let end_timestamp = end_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
            if entry.timestamp > end_timestamp {
                return false;
            }
        }

        // 用户ID检查
        if let Some(ref user_id) = query.user_id {
            if entry.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        // 源IP检查
        if let Some(ref source_ip) = query.source_ip {
            if entry.source_ip.as_ref() != Some(source_ip) {
                return false;
            }
        }

        true
    }

    /// 从文件收集统计信息
    fn collect_stats_from_file(&self, file_path: &Path, stats: &mut AuditStats, day_ago: u64, week_ago: u64) -> Result<(), Box<dyn std::error::Error>> {
        if !file_path.exists() {
            return Ok(());
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;

            if let Ok(entry) = serde_json::from_str::<AuditLogEntry>(&line) {
                stats.total_events += 1;

                // 按类型统计
                *stats.events_by_type.entry(entry.event_type.clone()).or_insert(0) += 1;

                // 按严重程度统计
                *stats.events_by_severity.entry(entry.severity.clone()).or_insert(0) += 1;

                // 按用户统计
                if let Some(user_id) = &entry.user_id {
                    *stats.events_by_user.entry(user_id.clone()).or_insert(0) += 1;
                }

                // 时间范围统计
                if entry.timestamp >= day_ago {
                    stats.events_last_24h += 1;
                }
                if entry.timestamp >= week_ago {
                    stats.events_last_7d += 1;
                }
            }
        }

        Ok(())
    }

    /// 从历史日志文件收集统计信息
    async fn collect_stats_from_historical_logs(&self, _stats: &mut AuditStats, _day_ago: u64, _week_ago: u64) -> Result<(), MiningError> {
        // 简化实现，实际应该统计所有历史日志文件
        Ok(())
    }
}
