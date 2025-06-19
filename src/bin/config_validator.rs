use std::collections::HashSet;
use std::path::Path;
use cgminer_rs::config::Config;
use serde_json::Value;
use tracing::{info, warn, error};

/// 配置验证器 - 检测虚假配置和未使用的配置项
#[derive(Debug)]
pub struct ConfigValidator {
    /// 已知的有效配置路径
    valid_config_paths: HashSet<String>,
    /// 检测到的问题
    issues: Vec<ConfigIssue>,
}

/// 配置问题类型
#[derive(Debug, Clone)]
pub enum ConfigIssue {
    /// 虚假配置 - 定义了但代码中未使用
    FakeConfig {
        path: String,
        description: String,
    },
    /// 配置位置错误
    WrongLocation {
        path: String,
        correct_location: String,
        description: String,
    },
    /// 字段名不匹配
    FieldMismatch {
        config_field: String,
        code_field: String,
        description: String,
    },
    /// 缺失必要配置
    MissingConfig {
        path: String,
        description: String,
    },
}

impl ConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        let mut valid_paths = HashSet::new();

        // 添加所有真正被代码使用的配置路径
        Self::add_valid_paths(&mut valid_paths);

        Self {
            valid_config_paths: valid_paths,
            issues: Vec::new(),
        }
    }

    /// 添加有效的配置路径
    fn add_valid_paths(paths: &mut HashSet<String>) {
        // [general] 配置
        paths.insert("general.log_level".to_string());
        paths.insert("general.log_file".to_string());
        paths.insert("general.pid_file".to_string());
        paths.insert("general.work_restart_timeout".to_string());
        paths.insert("general.scan_time".to_string());

        // [cores] 配置
        paths.insert("cores.enabled_cores".to_string());
        paths.insert("cores.default_core".to_string());

        // [cores.btc_software] 配置
        paths.insert("cores.btc_software.enabled".to_string());
        paths.insert("cores.btc_software.device_count".to_string());
        paths.insert("cores.btc_software.min_hashrate".to_string());
        paths.insert("cores.btc_software.max_hashrate".to_string());
        paths.insert("cores.btc_software.error_rate".to_string());
        paths.insert("cores.btc_software.batch_size".to_string());
        paths.insert("cores.btc_software.work_timeout_ms".to_string());

        // [cores.btc_software.cpu_affinity] 配置
        paths.insert("cores.btc_software.cpu_affinity.enabled".to_string());
        paths.insert("cores.btc_software.cpu_affinity.strategy".to_string());
        paths.insert("cores.btc_software.cpu_affinity.manual_mapping".to_string());
        paths.insert("cores.btc_software.cpu_affinity.avoid_hyperthreading".to_string());
        paths.insert("cores.btc_software.cpu_affinity.prefer_performance_cores".to_string());

        // [cores.maijie_l7] 配置
        paths.insert("cores.maijie_l7.enabled".to_string());
        paths.insert("cores.maijie_l7.chain_count".to_string());
        paths.insert("cores.maijie_l7.spi_speed".to_string());
        paths.insert("cores.maijie_l7.uart_baud".to_string());
        paths.insert("cores.maijie_l7.auto_detect".to_string());
        paths.insert("cores.maijie_l7.power_limit".to_string());
        paths.insert("cores.maijie_l7.cooling_mode".to_string());

        // [pools] 配置
        paths.insert("pools.strategy".to_string());
        paths.insert("pools.failover_timeout".to_string());
        paths.insert("pools.retry_interval".to_string());
        paths.insert("pools.pools".to_string());

        // [devices] 配置
        paths.insert("devices.auto_detect".to_string());
        paths.insert("devices.scan_interval".to_string());

        // [api] 配置
        paths.insert("api.enabled".to_string());
        paths.insert("api.bind_address".to_string());
        paths.insert("api.port".to_string());

        // [monitoring] 配置
        paths.insert("monitoring.enabled".to_string());
        paths.insert("monitoring.metrics_interval".to_string());
        paths.insert("monitoring.web_port".to_string());
        paths.insert("monitoring.alert_thresholds".to_string());

        // [hashmeter] 配置
        paths.insert("hashmeter.enabled".to_string());
        paths.insert("hashmeter.log_interval".to_string());
        paths.insert("hashmeter.per_device_stats".to_string());
        paths.insert("hashmeter.console_output".to_string());
        paths.insert("hashmeter.beautiful_output".to_string());
        paths.insert("hashmeter.hashrate_unit".to_string());

        // [web] 配置
        paths.insert("web.enabled".to_string());
        paths.insert("web.port".to_string());
        paths.insert("web.bind_address".to_string());
        paths.insert("web.static_path".to_string());
        paths.insert("web.static_files_dir".to_string()); // 别名支持
        paths.insert("web.template_dir".to_string());
    }

    /// 验证配置文件
    pub fn validate_config_file(&mut self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔍 开始验证配置文件: {}", config_path);

        // 读取配置文件
        let config_content = std::fs::read_to_string(config_path)?;
        let config_value: Value = toml::from_str(&config_content)?;

        // 尝试解析为Config结构 (忽略解析错误，只检查配置使用情况)
        match toml::from_str::<Config>(&config_content) {
            Ok(_config) => {
                info!("✅ 配置文件结构解析成功");
            }
            Err(e) => {
                warn!("⚠️  配置文件解析警告: {} (继续检查配置使用情况)", e);
            }
        }

        // 递归检查所有配置项
        self.check_config_value("", &config_value);

        // 检查特定的已知问题
        self.check_known_issues(&config_value);

        Ok(())
    }

    /// 递归检查配置值
    fn check_config_value(&mut self, prefix: &str, value: &Value) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    // 检查是否是有效配置路径
                    if !val.is_object() && !self.valid_config_paths.contains(&path) {
                        // 检查是否是已知的虚假配置
                        if self.is_known_fake_config(&path) {
                            self.issues.push(ConfigIssue::FakeConfig {
                                path: path.clone(),
                                description: self.get_fake_config_description(&path),
                            });
                        }
                    }

                    self.check_config_value(&path, val);
                }
            }
            _ => {}
        }
    }

    /// 检查已知问题
    fn check_known_issues(&mut self, config: &Value) {
        // 检查API配置位置错误
        if let Some(general) = config.get("general") {
            if general.get("api_port").is_some() || general.get("api_bind").is_some() {
                self.issues.push(ConfigIssue::WrongLocation {
                    path: "general.api_port/api_bind".to_string(),
                    correct_location: "api.port/bind_address".to_string(),
                    description: "API配置应该在[api]部分，不是[general]部分".to_string(),
                });
            }
        }

        // 检查字段名不匹配
        if let Some(pools) = config.get("pools") {
            if let Some(pools_array) = pools.get("pools") {
                if let Some(array) = pools_array.as_array() {
                    for pool in array {
                        if pool.get("user").is_some() && pool.get("username").is_none() {
                            self.issues.push(ConfigIssue::FieldMismatch {
                                config_field: "user".to_string(),
                                code_field: "username".to_string(),
                                description: "矿池配置应使用'username'字段，不是'user'".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 检查Web配置字段
        if let Some(web) = config.get("web") {
            if web.get("static_files_dir").is_some() && web.get("static_path").is_none() {
                info!("✅ Web配置使用别名'static_files_dir'，这是支持的");
            }
        }
    }

    /// 检查是否是已知的虚假配置
    fn is_known_fake_config(&self, path: &str) -> bool {
        // 已知的虚假配置路径
        let fake_configs = [
            "performance",
            "limits",
            "logging",
            "performance.hashrate_optimization",
            "performance.memory_optimization",
            "performance.thread_optimization",
            "performance.batch_optimization",
            "performance.network_optimization",
        ];

        for fake_config in &fake_configs {
            if path.starts_with(fake_config) {
                return true;
            }
        }

        false
    }

    /// 获取虚假配置的描述
    fn get_fake_config_description(&self, path: &str) -> String {
        match path {
            p if p.starts_with("performance") => {
                "性能配置块已定义但在代码中完全未使用，这是虚假配置".to_string()
            }
            p if p.starts_with("limits") => {
                "资源限制配置已定义但在代码中完全未使用，这是虚假配置".to_string()
            }
            p if p.starts_with("logging") => {
                "日志配置块已定义但在代码中完全未使用，这是虚假配置".to_string()
            }
            _ => "此配置项在代码中未被使用".to_string()
        }
    }

    /// 生成验证报告
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# CGMiner-RS 配置验证报告\n\n");

        if self.issues.is_empty() {
            report.push_str("✅ **配置验证通过** - 未发现问题\n\n");
            return report;
        }

        report.push_str(&format!("⚠️  **发现 {} 个配置问题**\n\n", self.issues.len()));

        let mut fake_configs = Vec::new();
        let mut wrong_locations = Vec::new();
        let mut field_mismatches = Vec::new();
        let mut missing_configs = Vec::new();

        for issue in &self.issues {
            match issue {
                ConfigIssue::FakeConfig { .. } => fake_configs.push(issue),
                ConfigIssue::WrongLocation { .. } => wrong_locations.push(issue),
                ConfigIssue::FieldMismatch { .. } => field_mismatches.push(issue),
                ConfigIssue::MissingConfig { .. } => missing_configs.push(issue),
            }
        }

        if !fake_configs.is_empty() {
            report.push_str("## 🚨 虚假配置 (定义但未使用)\n\n");
            for issue in fake_configs {
                if let ConfigIssue::FakeConfig { path, description } = issue {
                    report.push_str(&format!("- **{}**: {}\n", path, description));
                }
            }
            report.push_str("\n");
        }

        if !wrong_locations.is_empty() {
            report.push_str("## ⚠️  配置位置错误\n\n");
            for issue in wrong_locations {
                if let ConfigIssue::WrongLocation { path, correct_location, description } = issue {
                    report.push_str(&format!("- **{}** → **{}**: {}\n", path, correct_location, description));
                }
            }
            report.push_str("\n");
        }

        if !field_mismatches.is_empty() {
            report.push_str("## 🔄 字段名不匹配\n\n");
            for issue in field_mismatches {
                if let ConfigIssue::FieldMismatch { config_field, code_field, description } = issue {
                    report.push_str(&format!("- **{}** → **{}**: {}\n", config_field, code_field, description));
                }
            }
            report.push_str("\n");
        }

        report.push_str("## 💡 修复建议\n\n");
        report.push_str("1. **移除所有虚假配置** - 这些配置不会产生任何效果\n");
        report.push_str("2. **修复配置位置** - 将配置移动到正确的部分\n");
        report.push_str("3. **更新字段名** - 使用代码中实际支持的字段名\n");
        report.push_str("4. **使用修复后的配置文件** - 参考 `software_core_max_cpu_fixed.toml`\n\n");

        report
    }

    /// 打印验证结果
    pub fn print_results(&self) {
        if self.issues.is_empty() {
            info!("✅ 配置验证通过 - 未发现问题");
            return;
        }

        error!("⚠️  发现 {} 个配置问题:", self.issues.len());

        for issue in &self.issues {
            match issue {
                ConfigIssue::FakeConfig { path, description } => {
                    error!("🚨 虚假配置: {} - {}", path, description);
                }
                ConfigIssue::WrongLocation { path, correct_location, description } => {
                    warn!("⚠️  位置错误: {} → {} - {}", path, correct_location, description);
                }
                ConfigIssue::FieldMismatch { config_field, code_field, description } => {
                    warn!("🔄 字段不匹配: {} → {} - {}", config_field, code_field, description);
                }
                ConfigIssue::MissingConfig { path, description } => {
                    warn!("❓ 缺失配置: {} - {}", path, description);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("使用方法: {} <配置文件路径> [输出报告路径]", args[0]);
        println!("示例: {} examples/configs/software_core_max_cpu.toml", args[0]);
        return Ok(());
    }

    let config_path = &args[1];
    let report_path = args.get(2);

    if !Path::new(config_path).exists() {
        error!("配置文件不存在: {}", config_path);
        return Ok(());
    }

    let mut validator = ConfigValidator::new();

    match validator.validate_config_file(config_path) {
        Ok(()) => {
            info!("✅ 配置文件解析成功");
            validator.print_results();

            // 生成报告
            let report = validator.generate_report();

            if let Some(output_path) = report_path {
                std::fs::write(output_path, &report)?;
                info!("📄 验证报告已保存到: {}", output_path);
            } else {
                println!("\n{}", report);
            }
        }
        Err(e) => {
            error!("❌ 配置文件验证失败: {}", e);
        }
    }

    Ok(())
}
