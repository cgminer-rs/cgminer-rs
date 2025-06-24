use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::web::WebConfig;
use crate::mining::HashmeterConfig;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "cgminer.toml")]
    pub config: String,

    /// Enable debug mode
    #[arg(short, long)]
    pub debug: bool,

    /// API server port
    #[arg(long, default_value = "4028")]
    pub api_port: u16,

    /// Disable API server
    #[arg(long)]
    pub no_api: bool,

    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// SOCKS5 proxy URL (e.g., socks5://127.0.0.1:1080 or socks5+tls://proxy.example.com:1080)
    #[arg(long, help = "SOCKS5 proxy URL for pool connections")]
    pub proxy: Option<String>,

    /// SOCKS5 proxy username for authentication
    #[arg(long, help = "Username for SOCKS5 proxy authentication")]
    pub proxy_user: Option<String>,

    /// SOCKS5 proxy password for authentication
    #[arg(long, help = "Password for SOCKS5 proxy authentication")]
    pub proxy_pass: Option<String>,

    /// Pool URL to connect to (overrides config file)
    #[arg(short = 'o', long, help = "Mining pool URL (stratum+tcp://pool:port)")]
    pub pool: Option<String>,

    /// Pool username/worker name (overrides config file)
    #[arg(short = 'u', long, help = "Pool username or worker name")]
    pub user: Option<String>,

    /// Pool password (overrides config file)
    #[arg(short = 'p', long, help = "Pool password")]
    pub pass: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub cores: CoresConfig,
    pub devices: DeviceConfig,
    pub pools: PoolConfig,
    pub api: ApiConfig,
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub hashmeter: HashmeterConfig,
    pub performance: Option<PerformanceConfig>,
    pub limits: Option<LimitsConfig>,
    pub logging: Option<LoggingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GeneralConfig {
    pub log_level: String,
    pub log_file: Option<PathBuf>,
    pub pid_file: Option<PathBuf>,
    pub work_restart_timeout: u64,
    pub scan_time: u64,
    /// 结果收集间隔 (毫秒) - 参考原版cgminer的ASIC轮询延迟
    pub result_collection_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoresConfig {
    pub enabled_cores: Vec<String>,
    pub default_core: String,
    pub cpu_btc: Option<BtcSoftwareCoreConfig>,
    pub gpu_btc: Option<GpuBtcCoreConfig>,
    pub maijie_l7: Option<MaijieL7CoreConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BtcSoftwareCoreConfig {
    pub enabled: bool,
    pub device_count: u32,
    pub min_hashrate: f64,
    pub max_hashrate: f64,
    pub error_rate: f64,
    pub batch_size: u32,
    pub work_timeout_ms: u64,
    /// CPU绑定配置
    pub cpu_affinity: Option<CpuAffinityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuAffinityConfig {
    /// 是否启用CPU绑定
    pub enabled: bool,
    /// 绑定策略: "round_robin", "manual", "performance_first", "physical_only", "intelligent"
    pub strategy: String,
    /// 手动核心映射 (设备ID -> CPU核心索引)
    pub manual_mapping: Option<std::collections::HashMap<u32, usize>>,
    /// 是否避免超线程
    pub avoid_hyperthreading: Option<bool>,
    /// 是否优先使用性能核心
    pub prefer_performance_cores: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GpuBtcCoreConfig {
    pub enabled: bool,
    pub device_count: u32,
    pub max_hashrate: f64,
    pub work_size: u32,
    pub work_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaijieL7CoreConfig {
    pub enabled: bool,
    pub chain_count: u32,
    pub spi_speed: u32,
    pub uart_baud: u32,
    pub auto_detect: bool,
    pub power_limit: f64,
    pub cooling_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceConfig {
    pub auto_detect: bool,
    pub scan_interval: u64,
    pub chains: Vec<ChainConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub id: u8,
    pub enabled: bool,
    pub frequency: u32,
    pub voltage: u32,
    pub auto_tune: bool,
    pub chip_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PoolConfig {
    pub strategy: PoolStrategy,
    pub failover_timeout: u64,
    pub retry_interval: u64,
    pub pools: Vec<PoolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PoolStrategy {
    Failover,
    RoundRobin,
    LoadBalance,
    Quota,
}

impl Default for PoolStrategy {
    fn default() -> Self {
        PoolStrategy::Failover  // 默认使用故障转移策略
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub name: Option<String>,
    pub url: String,
    #[serde(alias = "user")]
    pub username: String,
    pub password: String,
    pub priority: u8,
    pub quota: Option<u32>,
    pub enabled: bool,
    /// 代理配置
    pub proxy: Option<ProxyConfig>,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理类型：socks5, socks5+tls
    pub proxy_type: String,
    /// 代理服务器地址
    pub host: String,
    /// 代理服务器端口
    pub port: u16,
    /// 代理认证用户名（可选）
    pub username: Option<String>,
    /// 代理认证密码（可选）
    pub password: Option<String>,
    /// TLS配置：是否跳过证书验证
    pub skip_verify: Option<bool>,
    /// TLS配置：服务器名称
    pub server_name: Option<String>,
    /// TLS配置：CA证书路径
    pub ca_cert: Option<String>,
    /// TLS配置：客户端证书路径
    pub client_cert: Option<String>,
    /// TLS配置：客户端私钥路径
    pub client_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ApiConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub allow_origins: Vec<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_interval: u64,
    pub web_port: Option<u16>,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertThresholds {
    pub temperature_warning: f32,
    pub temperature_critical: f32,
    pub hashrate_drop_percent: f32,
    pub error_rate_percent: f32,
    pub max_temperature: f32,
    pub max_cpu_usage: f32,
    pub max_memory_usage: f32,
    pub max_device_temperature: f32,
    pub max_error_rate: f32,
    pub min_hashrate: f64,
}

/// 性能优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 算力优化配置
    pub hashrate_optimization: Option<HashrateOptimizationConfig>,
    /// 内存优化配置
    pub memory_optimization: Option<MemoryOptimizationConfig>,
    /// 线程优化配置
    pub thread_optimization: Option<ThreadOptimizationConfig>,
    /// 批处理优化配置
    pub batch_optimization: Option<BatchOptimizationConfig>,
    /// 网络优化配置
    pub network_optimization: Option<NetworkOptimizationConfig>,
}

/// 算力优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashrateOptimizationConfig {
    /// 基础算力 (H/s)
    pub base_hashrate: f64,
    /// 算力变化范围 (0.0-1.0)
    pub hashrate_variance: f64,
    /// 频率-算力因子
    pub frequency_hashrate_factor: f64,
    /// 电压-算力因子
    pub voltage_hashrate_factor: f64,
    /// 温度影响因子
    pub temperature_impact_factor: f64,
    /// 自适应调整
    pub adaptive_adjustment: bool,
}

/// 内存优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimizationConfig {
    /// 工作缓存大小
    pub work_cache_size: u32,
    /// 结果缓存大小
    pub result_cache_size: u32,
    /// 统计保留时间 (秒)
    pub stats_retention_seconds: u64,
    /// 启用内存池
    pub enable_memory_pool: bool,
    /// 预分配内存 (MB)
    pub preallocated_memory_mb: u32,
}

/// 线程优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadOptimizationConfig {
    /// 每设备工作线程数
    pub worker_threads_per_device: u32,
    /// 线程优先级
    pub thread_priority: String,
    /// 线程栈大小 (KB)
    pub thread_stack_size_kb: u32,
    /// 启用线程池
    pub enable_thread_pool: bool,
}

/// 批处理优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptimizationConfig {
    /// 默认批次大小
    pub default_batch_size: u32,
    /// 最小批次大小
    pub min_batch_size: u32,
    /// 最大批次大小
    pub max_batch_size: u32,
    /// 自适应批次大小
    pub adaptive_batch_size: bool,
    /// 批次超时 (毫秒)
    pub batch_timeout_ms: u64,
}

/// 网络优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOptimizationConfig {
    /// 连接池大小
    pub connection_pool_size: u32,
    /// 请求超时 (毫秒)
    pub request_timeout_ms: u64,
    /// 最大并发请求数
    pub max_concurrent_requests: u32,
    /// 保活间隔 (秒)
    pub keepalive_interval: u64,
}

/// 系统资源限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// 最大内存使用 (MB)
    pub max_memory_mb: u64,
    /// 最大CPU使用率 (%)
    pub max_cpu_percent: f64,
    /// 最大打开文件数
    pub max_open_files: u32,
    /// 最大网络连接数
    pub max_network_connections: u32,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: String,
    /// 日志文件路径
    pub file: String,
    /// 最大文件大小
    pub max_size: String,
    /// 最大文件数量
    pub max_files: u32,
    /// 控制台输出
    pub console: bool,
    /// JSON格式
    pub json_format: bool,
    /// 日志轮转
    pub rotation: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                log_level: "info".to_string(),
                log_file: None,
                pid_file: Some(PathBuf::from("/tmp/cgminer-rs.pid")),
                work_restart_timeout: 60,
                scan_time: 30,
                result_collection_interval_ms: 20,
            },
            cores: CoresConfig {
                enabled_cores: vec!["cpu-btc".to_string()],
                default_core: "cpu-btc".to_string(),
                cpu_btc: Some(BtcSoftwareCoreConfig {
                    enabled: true,
                    device_count: 4,
                    min_hashrate: 1_000_000_000.0, // 1 GH/s
                    max_hashrate: 5_000_000_000.0, // 5 GH/s
                    error_rate: 0.01, // 1%
                    batch_size: 1000,
                    work_timeout_ms: 5000,
                    cpu_affinity: Some(CpuAffinityConfig {
                        enabled: true,
                        strategy: "round_robin".to_string(),
                        manual_mapping: None,
                        avoid_hyperthreading: Some(false),
                        prefer_performance_cores: Some(true),
                    }),
                }),
                gpu_btc: Some(GpuBtcCoreConfig {
                    enabled: false, // 默认禁用，需要用户手动启用
                    device_count: 1,
                    max_hashrate: 1_000_000_000_000.0, // 1 TH/s
                    work_size: 32768, // 32K 工作项
                    work_timeout_ms: 2000,
                }),
                maijie_l7: Some(MaijieL7CoreConfig {
                    enabled: false,
                    chain_count: 3,
                    spi_speed: 6_000_000, // 6MHz
                    uart_baud: 115200,
                    auto_detect: true,
                    power_limit: 3000.0, // 3kW
                    cooling_mode: "auto".to_string(),
                }),
            },
            devices: DeviceConfig {
                auto_detect: true,
                scan_interval: 5,
                chains: vec![
                    ChainConfig {
                        id: 0,
                        enabled: true,
                        frequency: 500,
                        voltage: 850,
                        auto_tune: true,
                        chip_count: 76,
                    },
                    ChainConfig {
                        id: 1,
                        enabled: true,
                        frequency: 500,
                        voltage: 850,
                        auto_tune: true,
                        chip_count: 76,
                    },
                ],
            },
            pools: PoolConfig {
                strategy: PoolStrategy::Failover,
                failover_timeout: 30,
                retry_interval: 10,
                pools: vec![
                    PoolInfo {
                        name: Some("example-pool".to_string()),
                        url: "stratum+tcp://pool.example.com:4444".to_string(),
                        username: "username".to_string(),
                        password: "password".to_string(),
                        priority: 1,
                        quota: None,
                        enabled: true,
                        proxy: None,
                    },
                ],
            },
            api: ApiConfig {
                enabled: true,
                bind_address: "127.0.0.1".to_string(),
                port: 4028,
                allow_origins: vec!["*".to_string()],
                auth_token: None,
            },
            monitoring: MonitoringConfig {
                enabled: true,
                metrics_interval: 30,
                web_port: Some(8888),
                alert_thresholds: AlertThresholds {
                    temperature_warning: 80.0,
                    temperature_critical: 90.0,
                    hashrate_drop_percent: 20.0,
                    error_rate_percent: 5.0,
                    max_temperature: 85.0,
                    max_cpu_usage: 80.0,
                    max_memory_usage: 90.0,
                    max_device_temperature: 85.0,
                    max_error_rate: 5.0,
                    min_hashrate: 50.0,
                },
            },
            web: WebConfig::default(),
            hashmeter: HashmeterConfig::default(),
            performance: None,
            limits: None,
            logging: None,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let config_content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        config.validate()?;

        Ok(config)
    }

    /// 应用CLI参数覆盖配置
    pub fn apply_cli_args(&mut self, args: &Args) -> Result<()> {
        // 应用API端口覆盖
        if args.api_port != 4028 {
            self.api.port = args.api_port;
        }

        // 应用API禁用选项
        if args.no_api {
            self.api.enabled = false;
        }

        // 应用日志级别覆盖
        if args.log_level != "info" {
            self.general.log_level = args.log_level.clone();
        }

        // 处理代理和矿池相关的CLI参数
        if args.proxy.is_some() || args.pool.is_some() || args.user.is_some() || args.pass.is_some() {
            self.apply_pool_cli_args(args)?;
        }

        Ok(())
    }

    /// 应用矿池相关的CLI参数
    fn apply_pool_cli_args(&mut self, args: &Args) -> Result<()> {
        // 如果指定了矿池URL，创建或修改第一个矿池配置
        if let Some(pool_url) = &args.pool {
            // 确保至少有一个矿池配置
            if self.pools.pools.is_empty() {
                self.pools.pools.push(PoolInfo {
                    name: Some("cli-pool".to_string()),
                    url: pool_url.clone(),
                    username: args.user.clone().unwrap_or_else(|| "worker".to_string()),
                    password: args.pass.clone().unwrap_or_else(|| "x".to_string()),
                    priority: 1,
                    quota: None,
                    enabled: true,
                    proxy: None,
                });
            } else {
                // 修改第一个矿池配置
                self.pools.pools[0].url = pool_url.clone();
                if let Some(user) = &args.user {
                    self.pools.pools[0].username = user.clone();
                }
                if let Some(pass) = &args.pass {
                    self.pools.pools[0].password = pass.clone();
                }
            }
        } else {
            // 如果没有指定矿池URL但指定了用户名或密码，应用到第一个矿池
            if !self.pools.pools.is_empty() {
                if let Some(user) = &args.user {
                    self.pools.pools[0].username = user.clone();
                }
                if let Some(pass) = &args.pass {
                    self.pools.pools[0].password = pass.clone();
                }
            }
        }

        // 处理代理配置
        if let Some(proxy_url) = &args.proxy {
            let proxy_config = self.parse_proxy_url(proxy_url, args)?;

            // 应用代理配置到所有启用的矿池
            for pool in &mut self.pools.pools {
                if pool.enabled {
                    pool.proxy = Some(proxy_config.clone());
                }
            }
        }

        Ok(())
    }

    /// 解析代理URL并创建代理配置
    fn parse_proxy_url(&self, proxy_url: &str, args: &Args) -> Result<ProxyConfig> {
        use url::Url;

        let parsed_url = Url::parse(proxy_url)
            .with_context(|| format!("Invalid proxy URL: {}", proxy_url))?;

        let proxy_type = match parsed_url.scheme() {
            "socks5" => "socks5".to_string(),
            "socks5+tls" => "socks5+tls".to_string(),
            scheme => {
                anyhow::bail!("Unsupported proxy scheme: {}. Use 'socks5' or 'socks5+tls'", scheme);
            }
        };

        let host = parsed_url.host_str()
            .ok_or_else(|| anyhow::anyhow!("Proxy URL must include a host"))?
            .to_string();

        let port = parsed_url.port()
            .ok_or_else(|| anyhow::anyhow!("Proxy URL must include a port"))?;

        // 优先使用CLI参数中的认证信息，其次使用URL中的认证信息
        let username = args.proxy_user.clone()
            .or_else(|| {
                if parsed_url.username().is_empty() {
                    None
                } else {
                    Some(parsed_url.username().to_string())
                }
            });

        let password = args.proxy_pass.clone()
            .or_else(|| parsed_url.password().map(|p| p.to_string()));

        // 解析URL查询参数中的TLS配置
        let mut skip_verify = None;
        let mut server_name = None;
        let mut ca_cert = None;
        let mut client_cert = None;
        let mut client_key = None;

        for (key, value) in parsed_url.query_pairs() {
            match key.as_ref() {
                "skip_verify" => {
                    skip_verify = Some(value.parse::<bool>()
                        .with_context(|| format!("Invalid skip_verify value: {}", value))?);
                },
                "server_name" => {
                    server_name = Some(value.to_string());
                },
                "ca_cert" => {
                    ca_cert = Some(value.to_string());
                },
                "client_cert" => {
                    client_cert = Some(value.to_string());
                },
                "client_key" => {
                    client_key = Some(value.to_string());
                },
                _ => {
                    // 忽略未知参数，但记录警告
                    eprintln!("Warning: Unknown proxy URL parameter: {}", key);
                }
            }
        }

        Ok(ProxyConfig {
            proxy_type,
            host,
            port,
            username,
            password,
            skip_verify,
            server_name,
            ca_cert,
            client_cert,
            client_key,
        })
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &str) -> Result<()> {
        let config_content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, config_content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // 注意：核心配置现在完全由编译特性和系统优先级逻辑控制
        // enabled_cores和default_core都不再需要配置验证

        // 验证Bitcoin软算法核心配置
        if let Some(cpu_btc_config) = &self.cores.cpu_btc {
            if cpu_btc_config.enabled {
                if cpu_btc_config.device_count == 0 {
                    anyhow::bail!("Bitcoin software core device count must be greater than 0");
                }
                if cpu_btc_config.device_count > 100 {
                    anyhow::bail!("Bitcoin software core device count cannot exceed 100");
                }
                if cpu_btc_config.min_hashrate >= cpu_btc_config.max_hashrate {
                    anyhow::bail!("Bitcoin software core min_hashrate must be less than max_hashrate");
                }
                if cpu_btc_config.error_rate < 0.0 || cpu_btc_config.error_rate > 1.0 {
                    anyhow::bail!("Bitcoin software core error_rate must be between 0.0 and 1.0");
                }
            }
        }

        // 验证Maijie L7 ASIC核心配置
        if let Some(maijie_l7_config) = &self.cores.maijie_l7 {
            if maijie_l7_config.enabled {
                if maijie_l7_config.chain_count == 0 {
                    anyhow::bail!("Maijie L7 core chain count must be greater than 0");
                }
                if maijie_l7_config.chain_count > 16 {
                    anyhow::bail!("Maijie L7 core chain count cannot exceed 16");
                }
                if maijie_l7_config.spi_speed == 0 || maijie_l7_config.spi_speed > 50_000_000 {
                    anyhow::bail!("Maijie L7 core SPI speed must be between 1 and 50,000,000 Hz");
                }
                let valid_bauds = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
                if !valid_bauds.contains(&maijie_l7_config.uart_baud) {
                    anyhow::bail!("Maijie L7 core UART baud rate must be a standard value");
                }
            }
        }

        // 验证矿池配置
        if self.pools.pools.is_empty() {
            anyhow::bail!("At least one pool must be configured");
        }

        // 验证设备配置
        if self.devices.chains.is_empty() {
            anyhow::bail!("At least one chain must be configured");
        }

        // 验证频率和电压范围
        for chain in &self.devices.chains {
            if chain.frequency < 100 || chain.frequency > 1000 {
                anyhow::bail!("Chain {} frequency {} is out of range (100-1000)",
                    chain.id, chain.frequency);
            }

            if chain.voltage < 600 || chain.voltage > 1000 {
                anyhow::bail!("Chain {} voltage {} is out of range (600-1000)",
                    chain.id, chain.voltage);
            }
        }

        // 验证API配置
        if self.api.port < 1024 {
            anyhow::bail!("API port {} is out of range (1024-65535)", self.api.port);
        }

        Ok(())
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}
