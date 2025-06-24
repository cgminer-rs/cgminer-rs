use crate::error::PoolError;
use crate::device::Work;
use crate::pool::Share;
use crate::pool::proxy::ProxyConnector;
use crate::config::ProxyConfig;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use tokio::sync::{RwLock, Mutex};
use tokio::time::timeout;
use tracing::{info, error, debug, warn};

/// Stratum 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumMessage {
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<StratumError>,
}

/// Stratum 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Stratum 客户端
pub struct StratumClient {
    /// 矿池URL
    url: String,
    /// 用户名
    username: String,
    /// 密码
    password: String,
    /// 代理配置
    proxy_config: Option<ProxyConfig>,
    /// TCP连接 - 写入部分
    writer: Arc<Mutex<Option<Box<dyn tokio::io::AsyncWrite + Unpin + Send>>>>,
    /// TCP连接 - 读取部分
    reader: Arc<Mutex<Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>>>>,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 订阅ID
    subscription_id: Arc<RwLock<Option<String>>>,
    /// Extra nonce 1
    extra_nonce1: Arc<RwLock<Option<String>>>,
    /// Extra nonce 2 大小
    extra_nonce2_size: Arc<RwLock<usize>>,
    /// 当前难度
    difficulty: Arc<RwLock<f64>>,
    /// 当前作业
    current_job: Arc<RwLock<Option<StratumJob>>>,
    /// 消息ID计数器
    message_id: Arc<RwLock<u64>>,
    /// 待处理的请求
    pending_requests: Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<StratumMessage>>>>,
    /// 矿池ID
    pool_id: u32,

}

/// Stratum 作业
#[derive(Debug, Clone)]
pub struct StratumJob {
    pub job_id: String,
    pub previous_hash: String,
    pub coinbase1: String,
    pub coinbase2: String,
    pub merkle_branches: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
}

impl StratumClient {
    /// 创建新的 Stratum 客户端
    pub async fn new(url: String, username: String, password: String, pool_id: u32, _verbose: bool, proxy_config: Option<ProxyConfig>) -> Result<Self, PoolError> {
        Ok(Self {
            url,
            username,
            password,
            proxy_config,
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            connected: Arc::new(RwLock::new(false)),
            subscription_id: Arc::new(RwLock::new(None)),
            extra_nonce1: Arc::new(RwLock::new(None)),
            extra_nonce2_size: Arc::new(RwLock::new(4)),
            difficulty: Arc::new(RwLock::new(1.0)),
            current_job: Arc::new(RwLock::new(None)),
            message_id: Arc::new(RwLock::new(1)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            pool_id,

        })
    }

    /// 连接到矿池
    pub async fn connect(&mut self) -> Result<(), PoolError> {
        info!("Connecting to Stratum pool: {}", self.url);
        debug!("🔗 [Pool {}] 开始连接到矿池: {}", self.pool_id, self.url);

        // 创建代理连接器（支持TLS配置）
        let connector = if let Some(ref proxy_config) = self.proxy_config {
            // 检查是否是SOCKS5+TLS代理，如果是则设置TLS配置
            if proxy_config.proxy_type == "socks5+tls" {
                // 直接使用代理配置中的TLS设置，不硬编码任何服务器
                let tls_config = crate::pool::proxy::TlsConfig {
                    skip_verify: proxy_config.skip_verify.unwrap_or(false),
                    server_name: proxy_config.server_name.clone(),
                    ca_cert_path: proxy_config.ca_cert.clone(),
                    client_cert_path: proxy_config.client_cert.clone(),
                    client_key_path: proxy_config.client_key.clone(),
                };

                debug!("🔐 [Pool {}] 使用TLS代理配置: skip_verify={:?}, server_name={:?}",
                       self.pool_id, tls_config.skip_verify, tls_config.server_name);

                if tls_config.skip_verify {
                    warn!("⚠️ [Pool {}] TLS证书验证已禁用 (skip_verify=true)", self.pool_id);
                }

                ProxyConnector::new_with_tls(Some(proxy_config.clone()), tls_config)
            } else {
                ProxyConnector::new(self.proxy_config.clone())
            }
        } else {
            ProxyConnector::new(self.proxy_config.clone())
        };

        // 建立连接（可能通过代理）
        debug!("🔗 [Pool {}] 尝试建立连接，超时时间: 10秒", self.pool_id);
        let connection = match timeout(Duration::from_secs(10), connector.connect(&self.url)).await {
            Ok(Ok(connection)) => {
                debug!("🔗 [Pool {}] 连接建立成功", self.pool_id);
                connection
            },
            Ok(Err(e)) => {
                debug!("🔗 [Pool {}] 连接失败: {}", self.pool_id, e);
                warn!("Pool {} connection failed: {}", self.pool_id, e);
                return Err(e);
            }
            Err(_) => {
                debug!("🔗 [Pool {}] 连接超时", self.pool_id);
                warn!("Pool {} connection timeout", self.pool_id);
                return Err(PoolError::Timeout { url: self.url.clone() });
            }
        };

        // 分离读写流
        debug!("🔗 [Pool {}] 分离连接为读写流", self.pool_id);
        let (reader, writer) = connection.into_split();
        *self.reader.lock().await = Some(reader);
        *self.writer.lock().await = Some(writer);
        *self.connected.write().await = true;

        // 启动消息处理循环
        debug!("🔗 [Pool {}] 启动消息处理循环", self.pool_id);
        self.start_message_loop().await?;

        // 发送订阅请求
        debug!("🔗 [Pool {}] 发送订阅请求", self.pool_id);
        self.subscribe().await?;

        // 发送认证请求
        debug!("🔗 [Pool {}] 发送认证请求", self.pool_id);
        self.authorize().await?;

        info!("Pool {} connected successfully", self.pool_id);
        info!("Successfully connected to Stratum pool");
        debug!("🔗 [Pool {}] 完整连接流程完成", self.pool_id);
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<(), PoolError> {
        info!("Disconnecting from Stratum pool");

        *self.connected.write().await = false;

        // 关闭读写流
        if let Some(reader) = self.reader.lock().await.take() {
            drop(reader);
        }
        if let Some(writer) = self.writer.lock().await.take() {
            drop(writer);
        }

        // 清理状态
        *self.subscription_id.write().await = None;
        *self.extra_nonce1.write().await = None;
        *self.current_job.write().await = None;
        self.pending_requests.write().await.clear();

        info!("Pool {} disconnected", self.pool_id);
        info!("Disconnected from Stratum pool");
        Ok(())
    }

    /// 订阅挖矿通知
    async fn subscribe(&self) -> Result<(), PoolError> {
        debug!("📤 [Pool {}] 发送 mining.subscribe 请求", self.pool_id);

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.subscribe".to_string()),
            params: Some(json!(["cgminer-rs/1.0.0"])),
            result: None,
            error: None,
        };

        debug!("📤 [Pool {}] mining.subscribe 消息内容: {:?}", self.pool_id, message);
        let response = self.send_request(message).await?;

        debug!("📥 [Pool {}] 收到 mining.subscribe 响应: {:?}", self.pool_id, response);

        if let Some(result) = response.result {
            if let Some(array) = result.as_array() {
                debug!("📥 [Pool {}] 响应数组长度: {}, 内容: {:?}", self.pool_id, array.len(), array);

                if array.len() < 2 {
                    debug!("❌ [Pool {}] 响应数组长度不足: {} < 2", self.pool_id, array.len());
                    return Err(PoolError::ProtocolError {
                        url: self.url.clone(),
                        error: format!("Invalid subscribe response: insufficient parameters (got {}, need at least 2)", array.len()),
                    });
                }

                // 第一个元素通常是订阅信息数组，我们暂时跳过详细解析
                if let Some(subscriptions) = array.get(0) {
                    debug!("📥 [Pool {}] 订阅信息: {:?}", self.pool_id, subscriptions);
                }

                // 第二个元素是extranonce1
                if let Some(extra_nonce1) = array.get(1).and_then(|v| v.as_str()) {
                    debug!("📥 [Pool {}] 获取到 extranonce1: '{}'", self.pool_id, extra_nonce1);

                    // 验证extranonce1格式
                    if extra_nonce1.is_empty() {
                        debug!("❌ [Pool {}] extranonce1 为空", self.pool_id);
                        return Err(PoolError::ProtocolError {
                            url: self.url.clone(),
                            error: "Empty extranonce1".to_string(),
                        });
                    }

                    // 验证是否为有效的十六进制字符串
                    if hex::decode(extra_nonce1).is_err() {
                        debug!("❌ [Pool {}] extranonce1 不是有效的十六进制: '{}'", self.pool_id, extra_nonce1);
                        return Err(PoolError::ProtocolError {
                            url: self.url.clone(),
                            error: format!("Invalid extranonce1 format (not hex): '{}'", extra_nonce1),
                        });
                    }

                    *self.extra_nonce1.write().await = Some(extra_nonce1.to_string());
                    debug!("✅ [Pool {}] extranonce1 设置成功: {}", self.pool_id, extra_nonce1);
                } else {
                    debug!("❌ [Pool {}] 无法从响应中获取 extranonce1，第二个元素: {:?}", self.pool_id, array.get(1));
                    return Err(PoolError::ProtocolError {
                        url: self.url.clone(),
                        error: "Missing or invalid extranonce1".to_string(),
                    });
                }

                // 第三个元素是extranonce2_size
                if array.len() >= 3 {
                    if let Some(extra_nonce2_size) = array.get(2).and_then(|v| v.as_u64()) {
                        debug!("📥 [Pool {}] 获取到 extranonce2_size: {}", self.pool_id, extra_nonce2_size);

                        // 验证extranonce2_size的合理范围
                        if extra_nonce2_size == 0 || extra_nonce2_size > 16 {
                            debug!("❌ [Pool {}] extranonce2_size 超出合理范围: {} (应该在1-16之间)", self.pool_id, extra_nonce2_size);
                            return Err(PoolError::ProtocolError {
                                url: self.url.clone(),
                                error: format!("Invalid extranonce2_size: {} (should be 1-16)", extra_nonce2_size),
                            });
                        }

                        *self.extra_nonce2_size.write().await = extra_nonce2_size as usize;
                        debug!("✅ [Pool {}] extranonce2_size 设置成功: {}", self.pool_id, extra_nonce2_size);
                    } else {
                        debug!("⚠️ [Pool {}] 无法获取 extranonce2_size，使用默认值 4，第三个元素: {:?}", self.pool_id, array.get(2));
                        // 使用默认值而不是报错，因为有些矿池可能不提供这个参数
                        *self.extra_nonce2_size.write().await = 4;
                    }
                } else {
                    debug!("⚠️ [Pool {}] 响应中没有 extranonce2_size，使用默认值 4", self.pool_id);
                    // 使用默认值
                    *self.extra_nonce2_size.write().await = 4;
                }
            } else {
                debug!("❌ [Pool {}] 响应结果不是数组格式: {:?}", self.pool_id, result);
                return Err(PoolError::ProtocolError {
                    url: self.url.clone(),
                    error: "Invalid subscribe response format (result is not an array)".to_string(),
                });
            }
        } else if let Some(error) = response.error {
            debug!("❌ [Pool {}] 订阅请求返回错误: 代码={}, 消息={}", self.pool_id, error.code, error.message);
            return Err(PoolError::StratumError {
                error_code: error.code,
                message: error.message,
            });
        } else {
            debug!("❌ [Pool {}] 响应中既没有结果也没有错误", self.pool_id);
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "No result or error in subscribe response".to_string(),
            });
        }

        debug!("✅ [Pool {}] 挖矿订阅成功完成", self.pool_id);
        Ok(())
    }

    /// 认证
    async fn authorize(&self) -> Result<(), PoolError> {
        debug!("Sending mining.authorize");

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.authorize".to_string()),
            params: Some(json!([self.username, self.password])),
            result: None,
            error: None,
        };

        let response = self.send_request(message).await?;

        if let Some(result) = response.result {
            if result.as_bool() == Some(true) {
                debug!("Mining authorization successful");
                Ok(())
            } else {
                Err(PoolError::AuthenticationFailed { url: self.url.clone() })
            }
        } else if let Some(error) = response.error {
            Err(PoolError::StratumError {
                error_code: error.code,
                message: error.message,
            })
        } else {
            Err(PoolError::AuthenticationFailed { url: self.url.clone() })
        }
    }

    /// 提交份额
    pub async fn submit_share(&self, share: &Share) -> Result<bool, PoolError> {
        // 记录份额提交详情
        debug!("Pool {} submitting share from device {}", self.pool_id, share.device_id);

        debug!("Submitting share: job_id={}, nonce={:08x}, ntime={:08x}",
               share.job_id, share.nonce, share.ntime);

        // 验证份额数据完整性
        // TODO: 重新启用验证 - DataValidator::validate_share(share)?;

        // 确保extranonce2格式正确（应该已经是十六进制字符串）
        let extranonce2_hex = if share.extra_nonce2.is_empty() {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Extranonce2 is empty".to_string(),
            });
        } else {
            share.extra_nonce2.clone()
        };

        // 按照Stratum协议格式提交份额
        // 参数顺序：[username, job_id, extranonce2, ntime, nonce]
        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.submit".to_string()),
            params: Some(json!([
                self.username,
                share.job_id,
                extranonce2_hex,
                format!("{:08x}", share.ntime),  // 使用工作数据中的ntime
                format!("{:08x}", share.nonce)
            ])),
            result: None,
            error: None,
        };

        let response = self.send_request(message).await?;

        if let Some(result) = response.result {
            let accepted = result.as_bool().unwrap_or(false);

            // 记录份额提交结果
            if accepted {
                info!("Accepted share from device {}", share.device_id);
            } else {
                info!("Rejected share from device {}", share.device_id);
            }

            if accepted {
                debug!("Share accepted by pool");
            } else {
                debug!("Share rejected by pool");
            }
            Ok(accepted)
        } else if let Some(error) = response.error {
            // 记录拒绝的份额
            warn!("Rejected share from device {}: {}", share.device_id, error.message);

            warn!("Share rejected: {}", error.message);
            Err(PoolError::ShareRejected { reason: error.message })
        } else {
            // 记录未知响应
            warn!("Unknown response format for share submission from device {}", share.device_id);

            warn!("Unknown response format for share submission");
            Ok(false)
        }
    }

    /// 获取工作
    pub async fn get_work(&self) -> Result<Work, PoolError> {
        let job = self.current_job.read().await;

        if let Some(job) = job.as_ref() {
            // 构造工作数据
            let work = self.build_work_from_job(job)?;
            Ok(work)
        } else {
            Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "No current job available".to_string(),
            })
        }
    }

    /// 从作业构造工作
    fn build_work_from_job(&self, job: &StratumJob) -> Result<Work, PoolError> {
        // 验证extranonce配置
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.validate_extranonce_config().await
            })
        })?;

        // 获取extranonce信息
        let extranonce1 = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.extra_nonce1.read().await.clone()
            })
        }).ok_or_else(|| PoolError::ProtocolError {
            url: self.url.clone(),
            error: "Extranonce1 not available".to_string(),
        })?;

        let extranonce2_size = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.extra_nonce2_size.read().await
            })
        });

        if extranonce2_size == 0 {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Extranonce2 size not set".to_string(),
            });
        }

        let difficulty = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.difficulty.read().await
            })
        });

        // 解析版本、nBits、nTime
        let version = u32::from_str_radix(&job.version, 16)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid version format".to_string(),
            })?;

        let nbits = u32::from_str_radix(&job.nbits, 16)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid nBits format".to_string(),
            })?;

        let ntime = u32::from_str_radix(&job.ntime, 16)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid nTime format".to_string(),
            })?;

        // 解析coinbase和merkle分支
        let coinbase1 = hex::decode(&job.coinbase1)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid coinbase1 format".to_string(),
            })?;

        let coinbase2 = hex::decode(&job.coinbase2)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid coinbase2 format".to_string(),
            })?;

        let merkle_branches: Result<Vec<Vec<u8>>, _> = job.merkle_branches
            .iter()
            .map(|branch| hex::decode(branch))
            .collect();

        let merkle_branches = merkle_branches
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid merkle branch format".to_string(),
            })?;

        // 解析extranonce1
        let extranonce1_bytes = hex::decode(&extranonce1)
            .map_err(|_| PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Invalid extranonce1 format".to_string(),
            })?;

        // 使用Work::from_stratum_job创建工作
        let mut work = Work::from_stratum_job(
            job.job_id.clone(),
            &job.previous_hash,
            coinbase1,
            coinbase2,
            merkle_branches,
            version,
            nbits,
            ntime,
            extranonce1_bytes,
            extranonce2_size,
            difficulty,
            job.clean_jobs,
        ).map_err(|e| PoolError::ProtocolError {
            url: self.url.clone(),
            error: format!("Failed to create work from job: {}", e),
        })?;

        // 生成extranonce2并计算merkle root
        let extranonce2 = self.generate_extranonce2(extranonce2_size);
        work.set_extranonce2(extranonce2);

        // 验证coinbase交易
        work.validate_coinbase().map_err(|e| PoolError::ProtocolError {
            url: self.url.clone(),
            error: format!("Invalid coinbase transaction: {}", e),
        })?;

        // 计算merkle root
        work.calculate_merkle_root().map_err(|e| PoolError::ProtocolError {
            url: self.url.clone(),
            error: format!("Failed to calculate merkle root: {}", e),
        })?;

        // 验证Work数据完整性
        // TODO: 重新启用验证 - DataValidator::validate_work(&work).map_err(|e| PoolError::ProtocolError {
        //     url: self.url.clone(),
        //     error: format!("Work validation failed: {}", e),
        // })?;

        Ok(work)
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// 发送心跳检测
    pub async fn ping(&self) -> Result<(), PoolError> {
        // 首先检查连接状态
        if !*self.connected.read().await {
            return Err(PoolError::ConnectionFailed {
                url: self.url.clone(),
                error: "StratumClient not connected".to_string(),
            });
        }

        // 检查writer是否可用
        {
            let writer_guard = self.writer.lock().await;
            if writer_guard.is_none() {
                // 连接状态和writer不一致，更新连接状态
                *self.connected.write().await = false;
                return Err(PoolError::ConnectionFailed {
                    url: self.url.clone(),
                    error: "TCP writer not available".to_string(),
                });
            }
        }

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.ping".to_string()),
            params: None,
            result: None,
            error: None,
        };

        match self.send_request(message).await {
            Ok(_response) => {
                debug!("💗 [Pool {}] 心跳响应成功", self.pool_id);
                Ok(())
            }
            Err(e) => {
                // 心跳失败时，检查是否是连接问题
                match &e {
                    PoolError::ConnectionFailed { .. } | PoolError::Timeout { .. } => {
                        // 连接问题，更新连接状态
                        warn!("💔 [Pool {}] 心跳失败，连接可能断开: {}", self.pool_id, e);
                        *self.connected.write().await = false;
                    }
                    _ => {
                        debug!("💔 [Pool {}] 心跳失败: {}", self.pool_id, e);
                    }
                }
                Err(e)
            }
        }
    }

    /// 发送请求并等待响应
    async fn send_request(&self, message: StratumMessage) -> Result<StratumMessage, PoolError> {
        let message_id = message.id.unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();

        // 注册待处理请求
        self.pending_requests.write().await.insert(message_id, tx);

        // 发送消息
        self.send_message(message).await?;

        // 等待响应
        match timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Request cancelled".to_string(),
            }),
            Err(_) => {
                // 清理待处理请求
                self.pending_requests.write().await.remove(&message_id);
                Err(PoolError::Timeout { url: self.url.clone() })
            }
        }
    }

    /// 发送消息
    async fn send_message(&self, message: StratumMessage) -> Result<(), PoolError> {
        debug!("📤 [Pool {}] 准备发送消息: {:?}", self.pool_id, message);

        let json_str = serde_json::to_string(&message)
            .map_err(|e| {
                debug!("❌ [Pool {}] JSON序列化失败: {}", self.pool_id, e);
                PoolError::ProtocolError {
                    url: self.url.clone(),
                    error: format!("JSON serialization error: {}", e),
                }
            })?;

        debug!("📤 [Pool {}] 发送JSON: {}", self.pool_id, json_str);

        let mut writer_guard = self.writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            debug!("📤 [Pool {}] 写入JSON数据到TCP流", self.pool_id);
            writer.write_all(json_str.as_bytes()).await
                .map_err(|e| {
                    debug!("❌ [Pool {}] TCP写入JSON失败: {}", self.pool_id, e);
                    PoolError::ConnectionFailed {
                        url: self.url.clone(),
                        error: e.to_string(),
                    }
                })?;

            debug!("📤 [Pool {}] 写入换行符", self.pool_id);
            writer.write_all(b"\n").await
                .map_err(|e| {
                    debug!("❌ [Pool {}] TCP写入换行符失败: {}", self.pool_id, e);
                    PoolError::ConnectionFailed {
                        url: self.url.clone(),
                        error: e.to_string(),
                    }
                })?;

            debug!("📤 [Pool {}] 刷新TCP缓冲区", self.pool_id);
            writer.flush().await
                .map_err(|e| {
                    debug!("❌ [Pool {}] TCP刷新失败: {}", self.pool_id, e);
                    PoolError::ConnectionFailed {
                        url: self.url.clone(),
                        error: e.to_string(),
                    }
                })?;
        } else {
            debug!("❌ [Pool {}] 无法发送消息：TCP连接未建立", self.pool_id);
            return Err(PoolError::ConnectionFailed {
                url: self.url.clone(),
                error: "Not connected".to_string(),
            });
        }

        debug!("✅ [Pool {}] 消息发送完成: {}", self.pool_id, json_str);
        Ok(())
    }

    /// 启动消息处理循环
    async fn start_message_loop(&self) -> Result<(), PoolError> {
        let reader = self.reader.clone();
        let connected = self.connected.clone();
        let pending_requests = self.pending_requests.clone();
        let current_job = self.current_job.clone();
        let difficulty = self.difficulty.clone();

        let pool_id = self.pool_id;

        tokio::spawn(async move {
            // 获取读取流
            let reader_stream = {
                let mut reader_guard = reader.lock().await;
                reader_guard.take()
            };

            if let Some(reader_stream) = reader_stream {
                let mut buf_reader = BufReader::new(reader_stream);
                let mut line = String::new();

                while *connected.read().await {
                    line.clear();

                    match buf_reader.read_line(&mut line).await {
                        Ok(0) => {
                            debug!("📥 [Pool {}] TCP连接已关闭 (EOF)", pool_id);
                            break; // EOF
                        },
                        Ok(bytes_read) => {
                            debug!("📥 [Pool {}] 接收到 {} 字节数据: {}", pool_id, bytes_read, line.trim());
                            if let Ok(message) = serde_json::from_str::<StratumMessage>(&line.trim()) {
                                debug!("📥 [Pool {}] 解析消息成功: {:?}", pool_id, message);

                                // 处理响应
                                if let Some(id) = message.id {
                                    debug!("📥 [Pool {}] 处理响应消息，ID: {}", pool_id, id);
                                    let mut pending = pending_requests.write().await;
                                    if let Some(tx) = pending.remove(&id) {
                                        debug!("📥 [Pool {}] 找到对应的待处理请求，发送响应", pool_id);
                                        let _ = tx.send(message);
                                        continue;
                                    } else {
                                        debug!("⚠️ [Pool {}] 未找到ID为 {} 的待处理请求", pool_id, id);
                                    }
                                }

                                // 处理通知
                                if let Some(method) = &message.method {
                                    debug!("📥 [Pool {}] 处理通知消息，方法: {}", pool_id, method);
                                    match method.as_str() {
                                        "mining.notify" => {
                                            // 处理新作业通知
                                            if let Some(params) = &message.params {
                                                if let Some(job) = Self::parse_job_notification(params) {
                                                    // 记录新工作接收
                                                    let _current_difficulty = *difficulty.read().await;
                                                    info!("Pool {} new job: {}", pool_id, job.job_id);

                                                    *current_job.write().await = Some(job);
                                                }
                                            }
                                        }
                                        "mining.set_difficulty" => {
                                            // 处理难度设置
                                            if let Some(params) = &message.params {
                                                if let Some(array) = params.as_array() {
                                                    if let Some(diff) = array.get(0).and_then(|v| v.as_f64()) {
                                                        // 验证难度值的合理性
                                                        if diff > 0.0 && diff.is_finite() {
                                                            let old_difficulty = *difficulty.read().await;
                                                            *difficulty.write().await = diff;

                                                            // 记录难度变化
                                                            if old_difficulty != diff {
                                                                info!("Pool {} difficulty changed from {} to {}", pool_id, old_difficulty, diff);
                                                            }

                                                            debug!("Difficulty updated to: {}", diff);
                                                        } else {
                                                            warn!("Invalid difficulty value received: {}", diff);
                                                        }
                                                    } else {
                                                        warn!("Failed to parse difficulty from mining.set_difficulty");
                                                    }
                                                } else {
                                                    warn!("Invalid parameters format for mining.set_difficulty");
                                                }
                                            } else {
                                                warn!("No parameters in mining.set_difficulty message");
                                            }
                                        }
                                        _ => {
                                            debug!("📥 [Pool {}] 未知方法: {}", pool_id, method);
                                        }
                                    }
                                } else {
                                    debug!("📥 [Pool {}] 收到无方法的消息: {:?}", pool_id, message);
                                }
                            } else {
                                debug!("❌ [Pool {}] JSON解析失败: {}", pool_id, line.trim());
                            }
                        }
                        Err(e) => {
                            debug!("❌ [Pool {}] TCP读取错误: {}", pool_id, e);
                            error!("Error reading from stream: {}", e);
                            break;
                        }
                    }
                }

                warn!("📥 [Pool {}] 消息处理循环结束，更新连接状态", pool_id);
                *connected.write().await = false;
            } else {
                warn!("📥 [Pool {}] 无法获取读取流，连接可能未建立", pool_id);
                *connected.write().await = false;
            }
        });

        Ok(())
    }

    /// 解析作业通知
    fn parse_job_notification(params: &Value) -> Option<StratumJob> {
        if let Some(array) = params.as_array() {
            if array.len() >= 9 {
                return Some(StratumJob {
                    job_id: array[0].as_str()?.to_string(),
                    previous_hash: array[1].as_str()?.to_string(),
                    coinbase1: array[2].as_str()?.to_string(),
                    coinbase2: array[3].as_str()?.to_string(),
                    merkle_branches: array[4].as_array()?
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    version: array[5].as_str()?.to_string(),
                    nbits: array[6].as_str()?.to_string(),
                    ntime: array[7].as_str()?.to_string(),
                    clean_jobs: array[8].as_bool().unwrap_or(false),
                });
            }
        }
        None
    }

    /// 获取下一个消息ID
    async fn next_message_id(&self) -> u64 {
        let mut id = self.message_id.write().await;
        *id += 1;
        *id
    }

    /// 生成extranonce2
    fn generate_extranonce2(&self, size: usize) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..size).map(|_| rng.gen::<u8>()).collect()
    }

    /// 检查extranonce是否已正确设置
    pub async fn is_extranonce_ready(&self) -> bool {
        let extranonce1 = self.extra_nonce1.read().await;
        let extranonce2_size = *self.extra_nonce2_size.read().await;

        extranonce1.is_some() && extranonce2_size > 0
    }

    /// 获取extranonce信息
    pub async fn get_extranonce_info(&self) -> (Option<String>, usize) {
        let extranonce1 = self.extra_nonce1.read().await.clone();
        let extranonce2_size = *self.extra_nonce2_size.read().await;

        (extranonce1, extranonce2_size)
    }

    /// 验证extranonce配置
    pub async fn validate_extranonce_config(&self) -> Result<(), PoolError> {
        let (extranonce1, extranonce2_size) = self.get_extranonce_info().await;

        // 检查extranonce1
        if let Some(ref en1) = extranonce1 {
            if en1.is_empty() {
                return Err(PoolError::ProtocolError {
                    url: self.url.clone(),
                    error: "Extranonce1 is empty".to_string(),
                });
            }

            // 验证十六进制格式
            if hex::decode(en1).is_err() {
                return Err(PoolError::ProtocolError {
                    url: self.url.clone(),
                    error: "Extranonce1 is not valid hex".to_string(),
                });
            }
        } else {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Extranonce1 not set".to_string(),
            });
        }

        // 检查extranonce2_size
        if extranonce2_size == 0 {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Extranonce2 size not set".to_string(),
            });
        }

        if extranonce2_size > 16 {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: format!("Extranonce2 size too large: {}", extranonce2_size),
            });
        }

        Ok(())
    }

    /// 获取当前难度
    pub async fn get_current_difficulty(&self) -> f64 {
        *self.difficulty.read().await
    }

    /// 验证难度值是否有效
    pub fn is_valid_difficulty(difficulty: f64) -> bool {
        difficulty > 0.0 && difficulty.is_finite() && difficulty <= 1e12
    }

    /// 设置难度值（带验证）
    pub async fn set_difficulty(&self, difficulty: f64) -> Result<(), PoolError> {
        if !Self::is_valid_difficulty(difficulty) {
            return Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: format!("Invalid difficulty value: {}", difficulty),
            });
        }

        *self.difficulty.write().await = difficulty;
        debug!("Difficulty set to: {}", difficulty);
        Ok(())
    }
}
