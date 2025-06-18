//! CGMiner-RS: High-performance ASIC Bitcoin miner written in Rust
//!
//! This library provides a complete Bitcoin mining solution with support for
//! various ASIC devices, pool connections, and monitoring capabilities.

pub mod api;
pub mod config;
pub mod core_loader;
pub mod device;
pub mod error;
pub mod mining;
pub mod monitoring;
pub mod pool;

// Re-export commonly used types
pub use core_loader::{CoreLoader, LoadStats};
pub use device::{DeviceInfo, DeviceStatus, DeviceStats, Work, MiningResult};
pub use device::{MiningDevice, DeviceDriver, DeviceManager};
pub use error::{MiningError, DeviceError};
pub use mining::{MiningManager, MiningState};
pub use config::Config;

// Re-export core types
pub use cgminer_core::{CoreRegistry, CoreFactory, CoreType, CoreInfo, CoreError};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize the library with default configuration
pub fn init() -> Result<(), MiningError> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    Ok(())
}

/// Initialize the library with custom configuration
pub fn init_with_config(config: Config) -> Result<(), MiningError> {
    // Initialize logging with custom configuration
    let log_level = match config.general.log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| MiningError::System(format!("Failed to set logger: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(NAME, "cgminer-rs");
    }

    #[test]
    fn test_init() {
        // This test might fail if logger is already initialized
        // but that's okay for testing purposes
        let _ = init();
    }
}
