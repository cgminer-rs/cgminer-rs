//! 简化安全模块使用示例
//! 
//! 展示如何使用简化的安全功能来保护挖矿配置和敏感数据

use cgminer_rs::security::{SimpleSecurityManager, simple::OperationType};
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();

    println!("🔒 简化安全模块使用示例");
    println!("================================");

    // 1. 创建简化安全管理器
    let config_paths = vec![
        PathBuf::from("config/mining.toml"),
        PathBuf::from("config/pools.toml"),
        PathBuf::from("config/wallet.toml"),
    ];
    let backup_dir = PathBuf::from("backups");

    let mut security_manager = SimpleSecurityManager::new(config_paths, backup_dir)?;

    // 2. 初始化安全系统
    security_manager.initialize().await?;
    println!("✅ 安全系统初始化完成");

    // 3. 演示敏感数据加密
    println!("\n📝 敏感数据加密演示:");
    let wallet_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let api_key = "sk-1234567890abcdef";

    let encrypted_wallet = security_manager.encrypt_sensitive_data(wallet_address).await?;
    let encrypted_api_key = security_manager.encrypt_sensitive_data(api_key).await?;

    println!("原始钱包地址: {}", wallet_address);
    println!("加密后: {}", encrypted_wallet);

    // 4. 演示数据解密
    let decrypted_wallet = security_manager.decrypt_sensitive_data(&encrypted_wallet).await?;
    let decrypted_api_key = security_manager.decrypt_sensitive_data(&encrypted_api_key).await?;

    println!("解密后钱包地址: {}", decrypted_wallet);
    println!("解密后API密钥: {}", decrypted_api_key);

    assert_eq!(wallet_address, decrypted_wallet);
    assert_eq!(api_key, decrypted_api_key);
    println!("✅ 加密解密测试通过");

    // 5. 演示配置完整性检查
    println!("\n🔍 配置完整性检查:");
    let integrity_ok = security_manager.check_config_integrity().await?;
    if integrity_ok {
        println!("✅ 配置文件完整性正常");
    } else {
        println!("⚠️  配置文件可能被修改");
    }

    // 6. 演示操作确认
    println!("\n⚠️  操作确认演示:");
    let operations = vec![
        OperationType::StopMining,
        OperationType::ChangeWallet,
        OperationType::DeleteConfig,
        OperationType::ResetSettings,
    ];

    for operation in operations {
        let confirmed = security_manager.request_confirmation(operation);
        println!("操作确认结果: {}", if confirmed { "已确认" } else { "已取消" });
    }

    // 7. 演示配置备份和恢复
    println!("\n💾 配置备份演示:");
    
    // 创建一个示例配置文件
    let test_config_path = PathBuf::from("test_config.toml");
    std::fs::write(&test_config_path, "# 测试配置文件\n[mining]\nthreads = 4\n")?;

    // 创建备份
    let backup_path = security_manager.backup_config(&test_config_path).await?;
    println!("✅ 配置备份已创建: {:?}", backup_path);

    // 修改原文件
    std::fs::write(&test_config_path, "# 修改后的配置文件\n[mining]\nthreads = 8\n")?;
    println!("📝 配置文件已修改");

    // 恢复备份
    security_manager.restore_config(&test_config_path).await?;
    println!("✅ 配置已从备份恢复");

    // 验证恢复结果
    let restored_content = std::fs::read_to_string(&test_config_path)?;
    if restored_content.contains("threads = 4") {
        println!("✅ 备份恢复测试通过");
    } else {
        println!("❌ 备份恢复测试失败");
    }

    // 清理测试文件
    let _ = std::fs::remove_file(&test_config_path);

    // 8. 演示安全功能开关
    println!("\n🔧 安全功能控制:");
    println!("当前安全状态: {}", if security_manager.is_enabled() { "启用" } else { "禁用" });

    security_manager.set_enabled(false);
    println!("安全功能已禁用");

    security_manager.set_enabled(true);
    println!("安全功能已重新启用");

    println!("\n🎉 简化安全模块演示完成！");
    println!("================================");
    println!("简化安全模块的优势:");
    println!("✅ 轻量级设计，适合个人使用");
    println!("✅ 保留核心安全功能");
    println!("✅ 移除复杂的认证和权限管理");
    println!("✅ 专注于配置保护和数据加密");
    println!("✅ 简单易用的API接口");

    Ok(())
}

/// 演示如何在实际挖矿程序中集成简化安全模块
async fn integration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔗 集成示例:");

    // 在挖矿程序启动时初始化安全模块
    let config_paths = vec![
        PathBuf::from("config/mining.toml"),
        PathBuf::from("config/pools.toml"),
    ];
    let backup_dir = PathBuf::from("backups");

    let mut security = SimpleSecurityManager::new(config_paths, backup_dir)?;
    security.initialize().await?;

    // 在保存敏感配置时加密
    let pool_password = "worker_password_123";
    let encrypted_password = security.encrypt_sensitive_data(pool_password).await?;
    
    // 保存到配置文件（这里只是示例）
    println!("保存加密后的密码到配置文件");

    // 在读取配置时解密
    let decrypted_password = security.decrypt_sensitive_data(&encrypted_password).await?;
    println!("从配置文件读取并解密密码");

    // 在执行重要操作前请求确认
    if security.request_confirmation(OperationType::StopMining) {
        println!("用户确认停止挖矿");
        // 执行停止挖矿操作
    }

    // 定期检查配置完整性
    if !security.check_config_integrity().await? {
        println!("⚠️  配置文件可能被修改，建议检查");
    }

    // 在修改重要配置前创建备份
    let config_path = PathBuf::from("config/mining.toml");
    security.backup_config(&config_path).await?;
    println!("配置备份已创建");

    Ok(())
}
