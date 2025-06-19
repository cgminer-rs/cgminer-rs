//! 数据验证和一致性检查模块
//!
//! 提供统一的数据验证机制，确保关键数据传递点的数据完整性

use crate::error::{MiningError, DeviceError, PoolError};
use cgminer_core::types::{Work, MiningResult};
use crate::pool::Share;
use std::time::{SystemTime, Duration};
use tracing::{warn, debug};
use uuid::Uuid;

/// 数据验证器
pub struct DataValidator;

impl DataValidator {
    /// 验证Work数据的完整性
    pub fn validate_work(work: &Work) -> Result<(), MiningError> {
        debug!("验证Work数据: job_id={}, work_id={}", work.job_id, work.work_id);

        // 1. 基本字段验证
        if work.job_id.is_empty() {
            return Err(MiningError::validation_error("Work job_id is empty"));
        }

        if work.work_id == 0 {
            return Err(MiningError::validation_error("Work work_id is zero"));
        }

        // 2. 时间验证
        let now = SystemTime::now();
        if work.expires_at <= now {
            return Err(MiningError::validation_error("Work has expired"));
        }

        if work.created_at > now {
            return Err(MiningError::validation_error("Work created_at is in the future"));
        }

        // 3. 目标难度验证
        if work.target == [0u8; 32] {
            return Err(MiningError::validation_error("Work target is zero"));
        }

        if work.difficulty <= 0.0 {
            return Err(MiningError::validation_error("Work difficulty must be positive"));
        }

        // 4. Extranonce验证
        if work.extranonce1.is_empty() {
            warn!("Work extranonce1 is empty, may cause mining issues");
        }

        if work.extranonce2.len() != work.extranonce2_size {
            return Err(MiningError::validation_error(
                format!("Extranonce2 length mismatch: expected {}, got {}",
                       work.extranonce2_size, work.extranonce2.len())
            ));
        }

        // 5. Coinbase验证
        if !work.coinbase1.is_empty() && !work.coinbase2.is_empty() {
            work.validate_coinbase().map_err(|e|
                MiningError::validation_error(format!("Coinbase validation failed: {}", e))
            )?;
        }

        debug!("Work数据验证通过");
        Ok(())
    }

    /// 验证挖矿结果的完整性
    pub fn validate_mining_result(result: &MiningResult) -> Result<(), MiningError> {
        debug!("验证挖矿结果: work_id={}, device_id={}, nonce={:08x}",
               result.work_id, result.device_id, result.nonce);

        // 1. 基本字段验证
        if result.work_id == Uuid::nil() {
            return Err(MiningError::validation_error("Mining result work_id is nil"));
        }

        if result.device_id == 0 {
            return Err(MiningError::validation_error("Mining result device_id is zero"));
        }

        // 2. 时间验证
        let now = SystemTime::now();
        if result.timestamp > now {
            return Err(MiningError::validation_error("Mining result timestamp is in the future"));
        }

        // 检查时间戳是否过于久远（超过1小时）
        if let Ok(duration) = now.duration_since(result.timestamp) {
            if duration > Duration::from_secs(3600) {
                warn!("Mining result timestamp is very old: {:?}", duration);
            }
        }

        // 3. 难度验证
        if result.share_difficulty <= 0.0 {
            return Err(MiningError::validation_error("Mining result share_difficulty must be positive"));
        }

        // 4. 哈希验证
        if result.hash == [0u8; 32] {
            return Err(MiningError::validation_error("Mining result hash is zero"));
        }

        // 5. Extranonce验证
        if result.extranonce2.is_empty() {
            return Err(MiningError::validation_error("Mining result extranonce2 is empty"));
        }

        debug!("挖矿结果验证通过");
        Ok(())
    }

    /// 验证份额数据的完整性
    pub fn validate_share(share: &Share) -> Result<(), PoolError> {
        debug!("验证份额数据: job_id={}, device_id={}, nonce={:08x}",
               share.job_id, share.device_id, share.nonce);

        // 1. 基本字段验证
        if share.job_id.is_empty() {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share job_id is empty".to_string(),
            });
        }

        if share.work_id == Uuid::nil() {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share work_id is nil".to_string(),
            });
        }

        if share.device_id == 0 {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share device_id is zero".to_string(),
            });
        }

        // 2. Extranonce2验证
        if share.extra_nonce2.is_empty() {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share extranonce2 is empty".to_string(),
            });
        }

        // 验证extranonce2是否为有效的十六进制字符串
        if hex::decode(&share.extra_nonce2).is_err() {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share extranonce2 is not valid hex".to_string(),
            });
        }

        // 3. 时间验证
        if share.ntime == 0 {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share ntime is zero".to_string(),
            });
        }

        // 4. 难度验证
        if share.difficulty <= 0.0 {
            return Err(PoolError::ProtocolError {
                url: "unknown".to_string(),
                error: "Share difficulty must be positive".to_string(),
            });
        }

        debug!("份额数据验证通过");
        Ok(())
    }

    /// 验证Work和MiningResult的一致性
    pub fn validate_work_result_consistency(work: &Work, result: &MiningResult) -> Result<(), MiningError> {
        debug!("验证Work和MiningResult一致性");

        // 1. 工作ID一致性
        if work.id != result.work_id {
            return Err(MiningError::validation_error(
                format!("Work ID mismatch: work.id={}, result.work_id={}", work.id, result.work_id)
            ));
        }

        // 2. 时间一致性检查
        if result.timestamp < work.created_at {
            return Err(MiningError::validation_error(
                "Mining result timestamp is before work creation time"
            ));
        }

        if result.timestamp > work.expires_at {
            return Err(MiningError::validation_error(
                "Mining result timestamp is after work expiration time"
            ));
        }

        // 3. Extranonce2长度一致性
        if result.extranonce2.len() != work.extranonce2_size {
            return Err(MiningError::validation_error(
                format!("Extranonce2 length mismatch: work expects {}, result has {}",
                       work.extranonce2_size, result.extranonce2.len())
            ));
        }

        debug!("Work和MiningResult一致性验证通过");
        Ok(())
    }

    /// 验证设备ID的有效性
    pub fn validate_device_id(device_id: u32) -> Result<(), DeviceError> {
        if device_id == 0 {
            return Err(DeviceError::InvalidConfig {
                reason: "Device ID cannot be zero".to_string(),
            });
        }

        // 检查设备ID是否在合理范围内
        if device_id > 10000 {
            return Err(DeviceError::InvalidConfig {
                reason: format!("Device ID {} is out of reasonable range", device_id),
            });
        }

        Ok(())
    }

    /// 批量验证数据一致性
    pub fn batch_validate_consistency(
        works: &[Work],
        results: &[MiningResult]
    ) -> Result<Vec<String>, MiningError> {
        let mut warnings = Vec::new();

        // 检查是否有孤立的结果（没有对应的工作）
        for result in results {
            let has_matching_work = works.iter().any(|work| work.id == result.work_id);
            if !has_matching_work {
                warnings.push(format!(
                    "Orphaned mining result: work_id={}, device_id={}",
                    result.work_id, result.device_id
                ));
            }
        }

        // 检查是否有过期的工作
        let now = SystemTime::now();
        for work in works {
            if work.expires_at <= now {
                warnings.push(format!(
                    "Expired work found: job_id={}, expired_at={:?}",
                    work.job_id, work.expires_at
                ));
            }
        }

        if !warnings.is_empty() {
            warn!("数据一致性检查发现 {} 个警告", warnings.len());
            for warning in &warnings {
                warn!("  - {}", warning);
            }
        }

        Ok(warnings)
    }
}

/// 数据完整性统计
#[derive(Debug, Default)]
pub struct ValidationStats {
    pub work_validations: u64,
    pub work_validation_failures: u64,
    pub result_validations: u64,
    pub result_validation_failures: u64,
    pub share_validations: u64,
    pub share_validation_failures: u64,
    pub consistency_checks: u64,
    pub consistency_failures: u64,
}

impl ValidationStats {
    pub fn record_work_validation(&mut self, success: bool) {
        self.work_validations += 1;
        if !success {
            self.work_validation_failures += 1;
        }
    }

    pub fn record_result_validation(&mut self, success: bool) {
        self.result_validations += 1;
        if !success {
            self.result_validation_failures += 1;
        }
    }

    pub fn record_share_validation(&mut self, success: bool) {
        self.share_validations += 1;
        if !success {
            self.share_validation_failures += 1;
        }
    }

    pub fn record_consistency_check(&mut self, success: bool) {
        self.consistency_checks += 1;
        if !success {
            self.consistency_failures += 1;
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.work_validations + self.result_validations +
                   self.share_validations + self.consistency_checks;
        let failures = self.work_validation_failures + self.result_validation_failures +
                      self.share_validation_failures + self.consistency_failures;

        if total == 0 {
            100.0
        } else {
            ((total - failures) as f64 / total as f64) * 100.0
        }
    }
}
