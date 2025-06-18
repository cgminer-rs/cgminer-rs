use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiningError {
    #[error("Device error: {0}")]
    Device(#[from] DeviceError),

    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),

    #[error("Work error: {0}")]
    Work(#[from] WorkError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Hardware error: {0}")]
    Hardware(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("Device not found: {device_id}")]
    NotFound { device_id: u32 },

    #[error("Device initialization failed: {device_id}, reason: {reason}")]
    InitializationFailed { device_id: u32, reason: String },

    #[error("Device communication error: {device_id}, error: {error}")]
    CommunicationError { device_id: u32, error: String },

    #[error("Device overheated: {device_id}, temperature: {temperature}°C")]
    Overheated { device_id: u32, temperature: f32 },

    #[error("Device hardware error: {device_id}, error_code: {error_code}")]
    HardwareError { device_id: u32, error_code: u32 },

    #[error("Chain error: {chain_id}, error: {error}")]
    ChainError { chain_id: u8, error: String },

    #[error("Chip error: {chain_id}, chip_id: {chip_id}, error: {error}")]
    ChipError { chain_id: u8, chip_id: u8, error: String },

    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Device timeout: {device_id}")]
    Timeout { device_id: u32 },
}

#[derive(Error, Debug, Clone)]
pub enum PoolError {
    #[error("Connection failed: {url}, error: {error}")]
    ConnectionFailed { url: String, error: String },

    #[error("Authentication failed: {url}")]
    AuthenticationFailed { url: String },

    #[error("Protocol error: {url}, error: {error}")]
    ProtocolError { url: String, error: String },

    #[error("No pools available")]
    NoPoolsAvailable,

    #[error("Pool timeout: {url}")]
    Timeout { url: String },

    #[error("Invalid pool URL: {url}")]
    InvalidUrl { url: String },

    #[error("Share rejected: {reason}")]
    ShareRejected { reason: String },

    #[error("Stratum error: {error_code}, message: {message}")]
    StratumError { error_code: i32, message: String },
}

#[derive(Error, Debug)]
pub enum WorkError {
    #[error("Work queue full")]
    QueueFull,

    #[error("Work queue empty")]
    QueueEmpty,

    #[error("Invalid work data: {reason}")]
    InvalidData { reason: String },

    #[error("Work expired: {work_id}")]
    Expired { work_id: String },

    #[error("Work not found: {work_id}")]
    NotFound { work_id: String },

    #[error("Duplicate work: {work_id}")]
    Duplicate { work_id: String },

    #[error("Work processing error: {error}")]
    ProcessingError { error: String },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Parse error: {error}")]
    ParseError { error: String },

    #[error("Validation error: {field}, reason: {reason}")]
    ValidationError { field: String, reason: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value: {field}, value: {value}, reason: {reason}")]
    InvalidValue { field: String, value: String, reason: String },
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection timeout: {address}")]
    Timeout { address: String },

    #[error("DNS resolution failed: {hostname}")]
    DnsResolutionFailed { hostname: String },

    #[error("TLS error: {error}")]
    TlsError { error: String },

    #[error("Socket error: {error}")]
    SocketError { error: String },

    #[error("HTTP error: {status_code}, message: {message}")]
    HttpError { status_code: u16, message: String },

    #[error("WebSocket error: {error}")]
    WebSocketError { error: String },
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Server start failed: {error}")]
    ServerStartFailed { error: String },

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("Resource not found: {resource}")]
    ResourceNotFound { resource: String },

    #[error("Method not allowed: {method}")]
    MethodNotAllowed { method: String },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error: {error}")]
    InternalError { error: String },
}

// 错误恢复策略
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 重试操作
    Retry { max_attempts: u32, delay_ms: u64 },
    /// 重启设备
    RestartDevice { device_id: u32 },
    /// 切换到备用池
    SwitchPool,
    /// 禁用设备
    DisableDevice { device_id: u32 },
    /// 优雅关闭
    GracefulShutdown,
    /// 立即停止
    ImmediateStop,
    /// 忽略错误
    Ignore,
}

impl DeviceError {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            DeviceError::CommunicationError { .. } => {
                RecoveryStrategy::Retry { max_attempts: 3, delay_ms: 1000 }
            }
            DeviceError::Timeout { .. } => {
                RecoveryStrategy::Retry { max_attempts: 2, delay_ms: 500 }
            }
            DeviceError::Overheated { device_id, .. } => {
                RecoveryStrategy::DisableDevice { device_id: *device_id }
            }
            DeviceError::HardwareError { device_id, .. } => {
                RecoveryStrategy::RestartDevice { device_id: *device_id }
            }
            DeviceError::InitializationFailed { .. } => {
                RecoveryStrategy::Retry { max_attempts: 5, delay_ms: 2000 }
            }
            _ => RecoveryStrategy::Ignore,
        }
    }
}

impl PoolError {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            PoolError::ConnectionFailed { .. } => {
                RecoveryStrategy::Retry { max_attempts: 3, delay_ms: 5000 }
            }
            PoolError::Timeout { .. } => RecoveryStrategy::SwitchPool,
            PoolError::AuthenticationFailed { .. } => RecoveryStrategy::SwitchPool,
            PoolError::NoPoolsAvailable => RecoveryStrategy::GracefulShutdown,
            _ => RecoveryStrategy::SwitchPool,
        }
    }
}

// 错误统计
#[derive(Debug, Default)]
pub struct ErrorStats {
    pub device_errors: u64,
    pub pool_errors: u64,
    pub work_errors: u64,
    pub network_errors: u64,
    pub total_errors: u64,
}

impl ErrorStats {
    pub fn record_error(&mut self, error: &MiningError) {
        self.total_errors += 1;

        match error {
            MiningError::Device(_) => self.device_errors += 1,
            MiningError::Pool(_) => self.pool_errors += 1,
            MiningError::Work(_) => self.work_errors += 1,
            MiningError::Network(_) => self.network_errors += 1,
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        *self = Default::default();
    }
}
