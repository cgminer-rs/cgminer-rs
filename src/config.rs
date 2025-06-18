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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub cores: CoresConfig,
    pub devices: DeviceConfig,
    pub pools: PoolConfig,
    pub api: ApiConfig,
    pub monitoring: MonitoringConfig,
    pub web: WebConfig,
    pub hashmeter: HashmeterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub log_file: Option<PathBuf>,
    pub pid_file: Option<PathBuf>,
    pub work_restart_timeout: u64,
    pub scan_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoresConfig {
    pub enabled_cores: Vec<String>,
    pub default_core: String,
    pub btc_software: Option<BtcSoftwareCoreConfig>,
    pub maijie_l7: Option<MaijieL7CoreConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// 绑定策略: "round_robin", "manual", "performance_first", "physical_only"
    pub strategy: String,
    /// 手动核心映射 (设备ID -> CPU核心索引)
    pub manual_mapping: Option<std::collections::HashMap<u32, usize>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub url: String,
    pub user: String,
    pub password: String,
    pub priority: u8,
    pub quota: Option<u32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub allow_origins: Vec<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_interval: u64,
    pub web_port: Option<u16>,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                log_level: "info".to_string(),
                log_file: None,
                pid_file: Some(PathBuf::from("/tmp/cgminer-rs.pid")),
                work_restart_timeout: 60,
                scan_time: 30,
            },
            cores: CoresConfig {
                enabled_cores: vec!["btc-software".to_string()],
                default_core: "btc-software".to_string(),
                btc_software: Some(BtcSoftwareCoreConfig {
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
                    }),
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
                        url: "stratum+tcp://pool.example.com:4444".to_string(),
                        user: "username".to_string(),
                        password: "password".to_string(),
                        priority: 1,
                        quota: None,
                        enabled: true,
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

    pub fn save(&self, path: &str) -> Result<()> {
        let config_content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, config_content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // 验证核心配置
        if self.cores.enabled_cores.is_empty() {
            anyhow::bail!("At least one core must be enabled");
        }

        if !self.cores.enabled_cores.contains(&self.cores.default_core) {
            anyhow::bail!("Default core '{}' must be in enabled cores list", self.cores.default_core);
        }

        // 验证Bitcoin软算法核心配置
        if let Some(btc_software_config) = &self.cores.btc_software {
            if btc_software_config.enabled {
                if btc_software_config.device_count == 0 {
                    anyhow::bail!("Bitcoin software core device count must be greater than 0");
                }
                if btc_software_config.device_count > 100 {
                    anyhow::bail!("Bitcoin software core device count cannot exceed 100");
                }
                if btc_software_config.min_hashrate >= btc_software_config.max_hashrate {
                    anyhow::bail!("Bitcoin software core min_hashrate must be less than max_hashrate");
                }
                if btc_software_config.error_rate < 0.0 || btc_software_config.error_rate > 1.0 {
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
