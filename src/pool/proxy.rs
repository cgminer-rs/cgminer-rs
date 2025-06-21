//! 代理连接模块
//!
//! 支持SOCKS5和SOCKS5+TLS代理连接

use crate::config::ProxyConfig;
use crate::error::PoolError;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_native_tls::{TlsConnector, TlsStream};
use url::Url;
use tracing::{debug, info};

/// 代理连接类型
#[derive(Debug)]
pub enum ProxyConnection {
    /// 直接连接（无代理）
    Direct(TcpStream),
    /// SOCKS5代理连接
    Socks5(Socks5Stream<TcpStream>),
    /// SOCKS5+TLS代理连接
    Socks5Tls(TlsStream<Socks5Stream<TcpStream>>),
}

impl ProxyConnection {
    /// 分离为读写流
    pub fn into_split(self) -> (Box<dyn tokio::io::AsyncRead + Unpin + Send>, Box<dyn tokio::io::AsyncWrite + Unpin + Send>) {
        match self {
            ProxyConnection::Direct(stream) => {
                let (reader, writer) = stream.into_split();
                (Box::new(reader) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
                 Box::new(writer) as Box<dyn tokio::io::AsyncWrite + Unpin + Send>)
            }
            ProxyConnection::Socks5(stream) => {
                let (reader, writer) = tokio::io::split(stream);
                (Box::new(reader) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
                 Box::new(writer) as Box<dyn tokio::io::AsyncWrite + Unpin + Send>)
            }
            ProxyConnection::Socks5Tls(stream) => {
                let (reader, writer) = tokio::io::split(stream);
                (Box::new(reader) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
                 Box::new(writer) as Box<dyn tokio::io::AsyncWrite + Unpin + Send>)
            }
        }
    }
}

/// 代理连接器
pub struct ProxyConnector {
    proxy_config: Option<ProxyConfig>,
}

impl ProxyConnector {
    /// 创建新的代理连接器
    pub fn new(proxy_config: Option<ProxyConfig>) -> Self {
        Self { proxy_config }
    }

    /// 连接到目标地址
    pub async fn connect(&self, target_url: &str) -> Result<ProxyConnection, PoolError> {
        // 解析目标URL
        let parsed_url = self.parse_target_url(target_url)?;
        let target_host = parsed_url.host_str()
            .ok_or_else(|| PoolError::InvalidUrl { url: target_url.to_string() })?;
        let target_port = parsed_url.port()
            .ok_or_else(|| PoolError::InvalidUrl { url: target_url.to_string() })?;

        match &self.proxy_config {
            Some(proxy) => {
                match proxy.proxy_type.as_str() {
                    "socks5" => self.connect_socks5(proxy, target_host, target_port).await,
                    "socks5+tls" => self.connect_socks5_tls(proxy, target_host, target_port).await,
                    _ => Err(PoolError::ProtocolError {
                        url: target_url.to_string(),
                        error: format!("Unsupported proxy type: {}", proxy.proxy_type),
                    }),
                }
            }
            None => {
                // 直接连接
                self.connect_direct(target_host, target_port).await
            }
        }
    }

    /// 解析目标URL
    fn parse_target_url(&self, url: &str) -> Result<Url, PoolError> {
        // 处理stratum+tcp://协议
        let normalized_url = if url.starts_with("stratum+tcp://") {
            url.replace("stratum+tcp://", "tcp://")
        } else {
            url.to_string()
        };

        Url::parse(&normalized_url).map_err(|_e| PoolError::InvalidUrl {
            url: url.to_string(),
        })
    }

    /// 直接连接
    async fn connect_direct(&self, host: &str, port: u16) -> Result<ProxyConnection, PoolError> {
        debug!("🔗 建立直接连接到 {}:{}", host, port);

        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr).await.map_err(|e| PoolError::ConnectionFailed {
            url: addr.clone(),
            error: e.to_string(),
        })?;

        info!("✅ 直接连接建立成功: {}", addr);
        Ok(ProxyConnection::Direct(stream))
    }

    /// SOCKS5代理连接
    async fn connect_socks5(&self, proxy: &ProxyConfig, target_host: &str, target_port: u16) -> Result<ProxyConnection, PoolError> {
        debug!("🔗 通过SOCKS5代理连接: {}:{} -> {}:{}",
               proxy.host, proxy.port, target_host, target_port);

        let proxy_addr = SocketAddr::new(
            proxy.host.parse().map_err(|_e| PoolError::InvalidUrl {
                url: format!("{}:{}", proxy.host, proxy.port),
            })?,
            proxy.port,
        );

        let stream = if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
            // 带认证的SOCKS5连接
            debug!("🔐 使用用户名密码认证连接SOCKS5代理");
            Socks5Stream::connect_with_password(
                proxy_addr,
                (target_host, target_port),
                username,
                password,
            ).await.map_err(|e| PoolError::ConnectionFailed {
                url: format!("socks5://{}:{}", proxy.host, proxy.port),
                error: e.to_string(),
            })?
        } else {
            // 无认证的SOCKS5连接
            debug!("🔓 无认证连接SOCKS5代理");
            Socks5Stream::connect(proxy_addr, (target_host, target_port)).await.map_err(|e| PoolError::ConnectionFailed {
                url: format!("socks5://{}:{}", proxy.host, proxy.port),
                error: e.to_string(),
            })?
        };

        info!("✅ SOCKS5代理连接建立成功: {}:{} -> {}:{}",
              proxy.host, proxy.port, target_host, target_port);
        Ok(ProxyConnection::Socks5(stream))
    }

    /// SOCKS5+TLS代理连接
    async fn connect_socks5_tls(&self, proxy: &ProxyConfig, target_host: &str, target_port: u16) -> Result<ProxyConnection, PoolError> {
        debug!("🔗 通过SOCKS5+TLS代理连接: {}:{} -> {}:{}",
               proxy.host, proxy.port, target_host, target_port);

        // 首先建立SOCKS5连接
        debug!("🔗 第一步：建立SOCKS5连接到代理服务器");
        let socks5_stream = self.connect_socks5_stream(proxy, target_host, target_port).await.map_err(|e| {
            debug!("❌ SOCKS5连接失败: {}", e);
            e
        })?;
        debug!("✅ SOCKS5连接建立成功");

        // 在SOCKS5连接上建立TLS
        debug!("🔐 第二步：在SOCKS5连接上建立TLS连接到目标主机: {}", target_host);
        let tls_connector = TlsConnector::from(native_tls::TlsConnector::new().map_err(|e| {
            let error_msg = format!("TLS connector creation failed: {}", e);
            debug!("❌ {}", error_msg);
            PoolError::ConnectionFailed {
                url: format!("socks5+tls://{}:{}", proxy.host, proxy.port),
                error: error_msg,
            }
        })?);

        let tls_stream = tls_connector.connect(target_host, socks5_stream).await.map_err(|e| {
            let error_msg = format!("TLS handshake failed: {}", e);
            debug!("❌ {}", error_msg);
            PoolError::ConnectionFailed {
                url: format!("socks5+tls://{}:{}", proxy.host, proxy.port),
                error: error_msg,
            }
        })?;

        info!("✅ SOCKS5+TLS代理连接建立成功: {}:{} -> {}:{}",
              proxy.host, proxy.port, target_host, target_port);
        Ok(ProxyConnection::Socks5Tls(tls_stream))
    }

    /// 建立SOCKS5流（用于TLS连接）
    async fn connect_socks5_stream(&self, proxy: &ProxyConfig, target_host: &str, target_port: u16) -> Result<Socks5Stream<TcpStream>, PoolError> {
        let proxy_addr = SocketAddr::new(
            proxy.host.parse().map_err(|_e| PoolError::InvalidUrl {
                url: format!("{}:{}", proxy.host, proxy.port),
            })?,
            proxy.port,
        );

        debug!("🔗 连接到SOCKS5代理服务器: {}", proxy_addr);

        if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
            debug!("🔐 使用用户名密码认证连接SOCKS5代理");
            Socks5Stream::connect_with_password(
                proxy_addr,
                (target_host, target_port),
                username,
                password,
            ).await.map_err(|e| {
                let error_msg = format!("SOCKS5认证连接失败: {}", e);
                debug!("❌ {}", error_msg);
                PoolError::ConnectionFailed {
                    url: format!("socks5://{}:{}", proxy.host, proxy.port),
                    error: error_msg,
                }
            })
        } else {
            debug!("🔗 无认证连接SOCKS5代理");
            Socks5Stream::connect(proxy_addr, (target_host, target_port)).await.map_err(|e| {
                let error_msg = format!("SOCKS5连接失败: {}", e);
                debug!("❌ {}", error_msg);
                PoolError::ConnectionFailed {
                    url: format!("socks5://{}:{}", proxy.host, proxy.port),
                    error: error_msg,
                }
            })
        }
    }
}

/// 从URL解析代理配置
pub fn parse_proxy_from_url(url: &str) -> Result<Option<ProxyConfig>, PoolError> {
    if url.starts_with("socks5://") || url.starts_with("socks5+tls://") {
        let parsed = Url::parse(url).map_err(|_e| PoolError::InvalidUrl {
            url: url.to_string(),
        })?;

        let proxy_type = if url.starts_with("socks5+tls://") {
            "socks5+tls".to_string()
        } else {
            "socks5".to_string()
        };

        let host = parsed.host_str()
            .ok_or_else(|| PoolError::InvalidUrl { url: url.to_string() })?
            .to_string();

        let port = parsed.port()
            .ok_or_else(|| PoolError::InvalidUrl { url: url.to_string() })?;

        let username = if parsed.username().is_empty() {
            None
        } else {
            Some(parsed.username().to_string())
        };

        let password = parsed.password().map(|p| p.to_string());

        Ok(Some(ProxyConfig {
            proxy_type,
            host,
            port,
            username,
            password,
        }))
    } else {
        Ok(None)
    }
}
