use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use serde_json::{json, Value};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "测试 Stratum 连接和协议", long_about = None)]
struct Args {
    /// Stratum 服务器 URL
    #[arg(long, default_value = "stratum+tcp://192.168.18.240:10203")]
    url: String,

    /// 测试用户名
    #[arg(short = 'u', long, default_value = "test.worker")]
    username: String,

    /// 测试密码
    #[arg(short, long, default_value = "x")]
    password: String,

    /// 连接超时时间（秒）
    #[arg(short, long, default_value = "10")]
    timeout: u64,

    /// 是否显示详细输出
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🔍 Stratum 协议测试工具");
    println!("═══════════════════════════════════════════════════════════");
    println!("📋 测试配置:");
    println!("   URL: {}", args.url);
    println!("   用户名: {}", args.username);
    println!("   密码: {}", args.password);
    println!("   超时: {} 秒", args.timeout);
    println!();

    // 验证 URL 格式
    if !args.url.starts_with("stratum+tcp://") {
        eprintln!("❌ 错误: URL 必须以 'stratum+tcp://' 开头");
        std::process::exit(1);
    }

    // 解析地址
    let address = args.url.strip_prefix("stratum+tcp://").unwrap();
    println!("🔗 正在连接到: {}", address);

    // 测试 TCP 连接
    match test_tcp_connection(address, args.timeout, args.verbose).await {
        Ok(()) => println!("✅ TCP 连接成功"),
        Err(e) => {
            eprintln!("❌ TCP 连接失败: {}", e);
            std::process::exit(1);
        }
    }

    // 测试 Stratum 协议
    match test_stratum_protocol(address, &args.username, &args.password, args.timeout, args.verbose).await {
        Ok(pool_info) => {
            println!("✅ Stratum 协议测试成功");
            println!("📊 矿池信息:");
            if let Some(info) = pool_info {
                println!("   {}", info);
            }
        }
        Err(e) => {
            eprintln!("❌ Stratum 协议测试失败: {}", e);
            std::process::exit(1);
        }
    }

    println!();
    println!("🎉 所有测试完成！");
    Ok(())
}

async fn test_tcp_connection(address: &str, timeout_secs: u64, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("   正在尝试 TCP 连接...");
    }

    let stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(address)
    ).await??;

    if verbose {
        if let Ok(peer_addr) = stream.peer_addr() {
            println!("   已连接到: {}", peer_addr);
        }
        if let Ok(local_addr) = stream.local_addr() {
            println!("   本地地址: {}", local_addr);
        }
    }

    drop(stream);
    Ok(())
}

async fn test_stratum_protocol(
    address: &str,
    username: &str,
    password: &str,
    timeout_secs: u64,
    verbose: bool
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if verbose {
        println!("   正在测试 Stratum 协议...");
    }

    let stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(address)
    ).await??;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // 发送订阅请求
    let subscribe_msg = json!({
        "id": 1,
        "method": "mining.subscribe",
        "params": ["cgminer-rs/1.0.0", null]
    });

    let subscribe_str = format!("{}\n", subscribe_msg.to_string());
    if verbose {
        println!("   发送订阅请求: {}", subscribe_str.trim());
    }

    writer.write_all(subscribe_str.as_bytes()).await?;

    // 读取响应
    let mut response = String::new();
    let response_result = timeout(
        Duration::from_secs(timeout_secs),
        reader.read_line(&mut response)
    ).await?;

    if response_result? == 0 {
        return Err("连接被服务器关闭".into());
    }

    if verbose {
        println!("   收到响应: {}", response.trim());
    }

    // 解析响应
    let parsed_response: Value = serde_json::from_str(&response)?;

    // 检查是否是有效的 Stratum 响应
    if parsed_response.get("id").is_some() &&
       (parsed_response.get("result").is_some() || parsed_response.get("error").is_some()) {

        if let Some(error) = parsed_response.get("error") {
            if !error.is_null() {
                return Err(format!("服务器返回错误: {}", error).into());
            }
        }

        if let Some(result) = parsed_response.get("result") {
            if verbose {
                println!("   订阅结果: {}", result);
            }

            // 尝试发送认证请求
            let auth_msg = json!({
                "id": 2,
                "method": "mining.authorize",
                "params": [username, password]
            });

            let auth_str = format!("{}\n", auth_msg.to_string());
            if verbose {
                println!("   发送认证请求: {}", auth_str.trim());
            }

            writer.write_all(auth_str.as_bytes()).await?;

            // 读取认证响应
            let mut auth_response = String::new();
            let auth_result = timeout(
                Duration::from_secs(timeout_secs),
                reader.read_line(&mut auth_response)
            ).await?;

            if auth_result? > 0 {
                if verbose {
                    println!("   认证响应: {}", auth_response.trim());
                }

                let auth_parsed: Value = serde_json::from_str(&auth_response)?;

                // 分析响应以确定矿池类型
                let pool_info = analyze_pool_response(&parsed_response, &auth_parsed);
                return Ok(Some(pool_info));
            }
        }

        Ok(Some("检测到有效的 Stratum 协议，但无法确定具体矿池类型".to_string()))
    } else {
        Err("服务器响应不符合 Stratum 协议格式".into())
    }
}

fn analyze_pool_response(subscribe_response: &Value, auth_response: &Value) -> String {
    let mut info = Vec::new();

    // 检查订阅响应中的信息
    if let Some(result) = subscribe_response.get("result") {
        if let Some(array) = result.as_array() {
            if array.len() >= 2 {
                if let Some(subscription_details) = array[0].as_array() {
                    for detail in subscription_details {
                        if let Some(method) = detail.as_array() {
                            if method.len() >= 1 {
                                if let Some(method_name) = method[0].as_str() {
                                    info.push(format!("支持方法: {}", method_name));
                                }
                            }
                        }
                    }
                }

                if let Some(extra_nonce1) = array[1].as_str() {
                    info.push(format!("Extra Nonce 1: {}", extra_nonce1));
                }

                if array.len() >= 3 {
                    if let Some(extra_nonce2_size) = array[2].as_u64() {
                        info.push(format!("Extra Nonce 2 大小: {}", extra_nonce2_size));
                    }
                }
            }
        }
    }

    // 检查认证响应
    if let Some(result) = auth_response.get("result") {
        if result.as_bool() == Some(true) {
            info.push("认证成功".to_string());
        } else {
            info.push("认证失败".to_string());
        }
    }

    // 尝试识别是否为 F2Pool
    let response_str = format!("{} {}", subscribe_response.to_string(), auth_response.to_string());
    if response_str.contains("f2pool") || response_str.contains("F2Pool") {
        info.push("🐟 检测到 F2Pool 矿池特征".to_string());
    }

    if info.is_empty() {
        "检测到标准 Stratum 协议".to_string()
    } else {
        info.join(", ")
    }
}
