//! 代理连接模块
//!
//! 支持SOCKS5和SOCKS5+TLS代理连接
//! 改进的TLS支持，参考gost项目实现

use crate::config::ProxyConfig;
use crate::error::PoolError;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_native_tls::{TlsConnector, TlsStream};
use url::Url;
use tracing::{debug, info, warn};

/// 代理连接类型
#[derive(Debug)]
pub enum ProxyConnection {
    /// 直接连接（无代理）
    Direct(TcpStream),
    /// SOCKS5代理连接
    Socks5(Socks5Stream<TcpStream>),
    /// SOCKS5+TLS代理连接（TLS到代理服务器）
    Socks5Tls(TlsStream<TcpStream>),
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

/// TLS配置选项
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// 是否跳过证书验证
    pub skip_verify: bool,
    /// 服务器名称指示（SNI）
    pub server_name: Option<String>,
    /// 自定义CA证书路径
    pub ca_cert_path: Option<String>,
    /// 客户端证书路径
    pub client_cert_path: Option<String>,
    /// 客户端私钥路径
    pub client_key_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            skip_verify: false,
            server_name: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }
}

/// 代理连接器
pub struct ProxyConnector {
    proxy_config: Option<ProxyConfig>,
    tls_config: TlsConfig,
}

impl ProxyConnector {
    /// 创建新的代理连接器
    pub fn new(proxy_config: Option<ProxyConfig>) -> Self {
        Self {
            proxy_config,
            tls_config: TlsConfig::default(),
        }
    }

    /// 创建带TLS配置的代理连接器
    pub fn new_with_tls(proxy_config: Option<ProxyConfig>, tls_config: TlsConfig) -> Self {
        Self {
            proxy_config,
            tls_config,
        }
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

    /// SOCKS5+TLS代理连接（改进版本，参考gost）
    /// 架构：TCP -> TLS -> SOCKS5 (正确的层次)
    async fn connect_socks5_tls(&self, proxy: &ProxyConfig, target_host: &str, target_port: u16) -> Result<ProxyConnection, PoolError> {
        debug!("🔗 通过SOCKS5+TLS代理连接: {}:{} -> {}:{}",
               proxy.host, proxy.port, target_host, target_port);

        // 第一步：建立到代理服务器的TCP连接
        debug!("🔗 第一步：建立TCP连接到代理服务器: {}:{}", proxy.host, proxy.port);
        let proxy_addr = SocketAddr::new(
            proxy.host.parse().map_err(|_e| PoolError::InvalidUrl {
                url: format!("{}:{}", proxy.host, proxy.port),
            })?,
            proxy.port,
        );

        let tcp_stream = TcpStream::connect(proxy_addr).await.map_err(|e| {
            let error_msg = format!("TCP连接到代理服务器失败: {}", e);
            debug!("❌ {}", error_msg);
            PoolError::ConnectionFailed {
                url: format!("socks5+tls://{}:{}", proxy.host, proxy.port),
                error: error_msg,
            }
        })?;
        debug!("✅ TCP连接到代理服务器建立成功");

        // 第二步：在TCP连接上建立TLS连接
        debug!("🔐 第二步：在TCP连接上建立TLS连接到代理服务器");
        let tls_connector = self.create_tls_connector()?;

        // 确定TLS连接的服务器名称
        let server_name = self.tls_config.server_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&proxy.host);

        debug!("🏷️ TLS服务器名称: {}", server_name);
        debug!("🔒 TLS配置: skip_verify={}", self.tls_config.skip_verify);

        let tls_stream = tls_connector.connect(server_name, tcp_stream).await.map_err(|e| {
            let error_msg = format!("TLS握手到代理服务器失败: {}", e);
            debug!("❌ {}", error_msg);
            warn!("🔐 TLS连接失败，可能的原因:");
            warn!("   • 证书问题（尝试添加 ?skip_verify=true）");
            warn!("   • 服务器名称不匹配（尝试添加 ?server_name=正确名称）");
            warn!("   • 代理服务器不支持TLS");
            PoolError::ConnectionFailed {
                url: format!("socks5+tls://{}:{}", proxy.host, proxy.port),
                error: error_msg,
            }
        })?;
        debug!("✅ TLS连接到代理服务器建立成功");

        // 第三步：在TLS连接上进行SOCKS5协商
        debug!("🌐 第三步：在TLS连接上进行SOCKS5协商到目标: {}:{}", target_host, target_port);
        let negotiated_stream = self.perform_socks5_over_tls_and_return_stream(tls_stream, proxy, target_host, target_port).await?;

        info!("✅ SOCKS5+TLS代理连接建立成功: {}:{} -> {}:{}",
              proxy.host, proxy.port, target_host, target_port);
        info!("🔒 使用TLS加密传输，安全级别提升");

        // 返回协商完成的TLS流，它现在可以直接用于数据传输
        Ok(ProxyConnection::Socks5Tls(negotiated_stream))
    }

    /// 创建TLS连接器（改进版本，支持更多配置）
    fn create_tls_connector(&self) -> Result<TlsConnector, PoolError> {
        let mut builder = native_tls::TlsConnector::builder();

        // 配置证书验证
        if self.tls_config.skip_verify {
            warn!("⚠️ TLS证书验证已禁用，连接可能不安全");
            builder.danger_accept_invalid_certs(true);
            builder.danger_accept_invalid_hostnames(true);
        }

        // 配置最小TLS版本（安全性考虑）
        builder.min_protocol_version(Some(native_tls::Protocol::Tlsv12));

        // TODO: 添加自定义CA证书支持
        if let Some(_ca_path) = &self.tls_config.ca_cert_path {
            debug!("📋 自定义CA证书功能待实现");
        }

        // TODO: 添加客户端证书支持
        if let Some(_cert_path) = &self.tls_config.client_cert_path {
            debug!("🔑 客户端证书功能待实现");
        }

        let native_connector = builder.build().map_err(|e| {
            let error_msg = format!("TLS连接器创建失败: {}", e);
            debug!("❌ {}", error_msg);
            PoolError::ConnectionFailed {
                url: "tls://".to_string(),
                error: error_msg,
            }
        })?;

        Ok(TlsConnector::from(native_connector))
    }

    /// 在TLS流上进行SOCKS5协商（改进版本，参考gost）
    async fn perform_socks5_over_tls(
        &self,
        mut tls_stream: TlsStream<TcpStream>,
        proxy: &ProxyConfig,
        target_host: &str,
        target_port: u16
    ) -> Result<(), PoolError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        debug!("🔧 开始在TLS流上进行SOCKS5协商");

        // 第一步：发送认证方法协商
        let auth_methods = if proxy.username.is_some() && proxy.password.is_some() {
            vec![0x00, 0x02] // NO AUTH, USERNAME/PASSWORD
        } else {
            vec![0x00] // NO AUTH only
        };

        let mut request = vec![0x05]; // SOCKS5 version
        request.push(auth_methods.len() as u8); // 方法数量
        request.extend(&auth_methods);

        tls_stream.write_all(&request).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("发送认证方法失败: {}", e),
            }
        })?;

        // 读取服务器选择的认证方法
        let mut response = [0u8; 2];
        tls_stream.read_exact(&mut response).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取认证方法响应失败: {}", e),
            }
        })?;

        if response[0] != 0x05 {
            return Err(PoolError::ProtocolError {
                url: "socks5+tls".to_string(),
                error: "无效的SOCKS5响应版本".to_string(),
            });
        }

        // 处理认证
        match response[1] {
            0x00 => {
                debug!("✅ 服务器选择无认证方法");
            }
            0x02 => {
                debug!("🔐 服务器要求用户名密码认证");
                if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
                    self.socks5_username_password_auth(&mut tls_stream, username, password).await?;
                } else {
                    return Err(PoolError::ProtocolError {
                        url: "socks5+tls".to_string(),
                        error: "服务器要求认证但未提供凭据".to_string(),
                    });
                }
            }
            0xFF => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: "服务器拒绝所有认证方法".to_string(),
                });
            }
            _ => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: format!("不支持的认证方法: {}", response[1]),
                });
            }
        }

        // 第二步：发送连接请求
        debug!("🎯 准备连接到目标: {}:{}", target_host, target_port);
        let mut connect_request = vec![0x05, 0x01, 0x00]; // VER, CMD=CONNECT, RSV

        // 添加目标地址
        if target_host.parse::<std::net::IpAddr>().is_ok() {
            // IP地址
            if let Ok(ip) = target_host.parse::<std::net::Ipv4Addr>() {
                debug!("📍 使用IPv4地址: {}", ip);
                connect_request.push(0x01); // IPv4
                connect_request.extend(&ip.octets());
            } else if let Ok(ip) = target_host.parse::<std::net::Ipv6Addr>() {
                debug!("📍 使用IPv6地址: {}", ip);
                connect_request.push(0x04); // IPv6
                connect_request.extend(&ip.octets());
            }
        } else {
            // 域名
            debug!("📍 使用域名: {} (长度: {})", target_host, target_host.len());
            connect_request.push(0x03); // DOMAINNAME
            connect_request.push(target_host.len() as u8);
            connect_request.extend(target_host.as_bytes());
        }

        // 添加端口
        connect_request.extend(&target_port.to_be_bytes());
        debug!("📦 SOCKS5连接请求大小: {} 字节", connect_request.len());

        tls_stream.write_all(&connect_request).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("发送连接请求失败: {}", e),
            }
        })?;

        // 读取连接响应
        let mut connect_response = [0u8; 4];
        tls_stream.read_exact(&mut connect_response).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取连接响应失败: {}", e),
            }
        })?;

        if connect_response[0] != 0x05 {
            return Err(PoolError::ProtocolError {
                url: "socks5+tls".to_string(),
                error: "无效的SOCKS5连接响应版本".to_string(),
            });
        }

        if connect_response[1] != 0x00 {
            let error_description = match connect_response[1] {
                0x01 => "一般SOCKS服务器失败",
                0x02 => "连接规则不允许",
                0x03 => "网络不可达",
                0x04 => "主机不可达",
                0x05 => "连接被拒绝",
                0x06 => "TTL过期",
                0x07 => "不支持的命令",
                0x08 => "不支持的地址类型",
                _ => "未知错误",
            };

            warn!("❌ SOCKS5代理连接失败: {} (错误代码: {})", error_description, connect_response[1]);
            warn!("🎯 目标地址: {}:{}", target_host, target_port);
            warn!("🌐 代理服务器: {}:{}", proxy.host, proxy.port);

            return Err(PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("SOCKS5连接失败 - {}: {} (错误代码: {})",
                              error_description, target_host, connect_response[1]),
            });
        }

        // 读取绑定地址（跳过，我们不需要它）
        let addr_type = connect_response[3];
        let skip_bytes = match addr_type {
            0x01 => 4 + 2, // IPv4 + port
            0x03 => {
                let mut len_buf = [0u8; 1];
                tls_stream.read_exact(&mut len_buf).await.map_err(|e| {
                    PoolError::ConnectionFailed {
                        url: "socks5+tls".to_string(),
                        error: format!("读取域名长度失败: {}", e),
                    }
                })?;
                len_buf[0] as usize + 2 // domain length + port
            }
            0x04 => 16 + 2, // IPv6 + port
            _ => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: format!("不支持的地址类型: {}", addr_type),
                });
            }
        };

        let mut skip_buf = vec![0u8; skip_bytes];
        tls_stream.read_exact(&mut skip_buf).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取绑定地址失败: {}", e),
            }
        })?;

        debug!("✅ SOCKS5协商完成，连接已建立");

        // SOCKS5协商成功，TLS流现在已经准备好进行数据传输
        // 在真实的实现中，这个TLS流将被包装并返回给调用者
        Ok(())
    }

    /// 在TLS流上进行SOCKS5协商并返回协商完成的流
    async fn perform_socks5_over_tls_and_return_stream(
        &self,
        mut tls_stream: TlsStream<TcpStream>,
        proxy: &ProxyConfig,
        target_host: &str,
        target_port: u16
    ) -> Result<TlsStream<TcpStream>, PoolError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        debug!("🔧 开始在TLS流上进行SOCKS5协商");

        // 第一步：发送认证方法协商
        let auth_methods = if proxy.username.is_some() && proxy.password.is_some() {
            vec![0x00, 0x02] // NO AUTH, USERNAME/PASSWORD
        } else {
            vec![0x00] // NO AUTH only
        };

        let mut request = vec![0x05]; // SOCKS5 version
        request.push(auth_methods.len() as u8); // 方法数量
        request.extend(&auth_methods);

        tls_stream.write_all(&request).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("发送认证方法失败: {}", e),
            }
        })?;

        // 读取服务器选择的认证方法
        let mut response = [0u8; 2];
        tls_stream.read_exact(&mut response).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取认证方法响应失败: {}", e),
            }
        })?;

        if response[0] != 0x05 {
            return Err(PoolError::ProtocolError {
                url: "socks5+tls".to_string(),
                error: "无效的SOCKS5响应版本".to_string(),
            });
        }

        // 处理认证
        match response[1] {
            0x00 => {
                debug!("✅ 服务器选择无认证方法");
            }
            0x02 => {
                debug!("🔐 服务器要求用户名密码认证");
                if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
                    self.socks5_username_password_auth(&mut tls_stream, username, password).await?;
                } else {
                    return Err(PoolError::ProtocolError {
                        url: "socks5+tls".to_string(),
                        error: "服务器要求认证但未提供凭据".to_string(),
                    });
                }
            }
            0xFF => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: "服务器拒绝所有认证方法".to_string(),
                });
            }
            _ => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: format!("不支持的认证方法: {}", response[1]),
                });
            }
        }

        // 第二步：发送连接请求
        debug!("🎯 准备连接到目标: {}:{}", target_host, target_port);
        let mut connect_request = vec![0x05, 0x01, 0x00]; // VER, CMD=CONNECT, RSV

        // 添加目标地址
        if target_host.parse::<std::net::IpAddr>().is_ok() {
            // IP地址
            if let Ok(ip) = target_host.parse::<std::net::Ipv4Addr>() {
                debug!("📍 使用IPv4地址: {}", ip);
                connect_request.push(0x01); // IPv4
                connect_request.extend(&ip.octets());
            } else if let Ok(ip) = target_host.parse::<std::net::Ipv6Addr>() {
                debug!("📍 使用IPv6地址: {}", ip);
                connect_request.push(0x04); // IPv6
                connect_request.extend(&ip.octets());
            }
        } else {
            // 域名
            debug!("📍 使用域名: {} (长度: {})", target_host, target_host.len());
            connect_request.push(0x03); // DOMAINNAME
            connect_request.push(target_host.len() as u8);
            connect_request.extend(target_host.as_bytes());
        }

        // 添加端口
        connect_request.extend(&target_port.to_be_bytes());
        debug!("📦 SOCKS5连接请求大小: {} 字节", connect_request.len());

        tls_stream.write_all(&connect_request).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("发送连接请求失败: {}", e),
            }
        })?;

        // 读取连接响应
        let mut connect_response = [0u8; 4];
        tls_stream.read_exact(&mut connect_response).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取连接响应失败: {}", e),
            }
        })?;

        if connect_response[0] != 0x05 {
            return Err(PoolError::ProtocolError {
                url: "socks5+tls".to_string(),
                error: "无效的SOCKS5连接响应版本".to_string(),
            });
        }

        if connect_response[1] != 0x00 {
            let error_description = match connect_response[1] {
                0x01 => "一般SOCKS服务器失败",
                0x02 => "连接规则不允许",
                0x03 => "网络不可达",
                0x04 => "主机不可达",
                0x05 => "连接被拒绝",
                0x06 => "TTL过期",
                0x07 => "不支持的命令",
                0x08 => "不支持的地址类型",
                _ => "未知错误",
            };

            warn!("❌ SOCKS5代理连接失败: {} (错误代码: {})", error_description, connect_response[1]);
            warn!("🎯 目标地址: {}:{}", target_host, target_port);
            warn!("🌐 代理服务器: {}:{}", proxy.host, proxy.port);

            return Err(PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("SOCKS5连接失败 - {}: {} (错误代码: {})",
                              error_description, target_host, connect_response[1]),
            });
        }

        // 读取绑定地址（跳过，我们不需要它）
        let addr_type = connect_response[3];
        let skip_bytes = match addr_type {
            0x01 => 4 + 2, // IPv4 + port
            0x03 => {
                let mut len_buf = [0u8; 1];
                tls_stream.read_exact(&mut len_buf).await.map_err(|e| {
                    PoolError::ConnectionFailed {
                        url: "socks5+tls".to_string(),
                        error: format!("读取域名长度失败: {}", e),
                    }
                })?;
                len_buf[0] as usize + 2 // domain length + port
            }
            0x04 => 16 + 2, // IPv6 + port
            _ => {
                return Err(PoolError::ProtocolError {
                    url: "socks5+tls".to_string(),
                    error: format!("不支持的地址类型: {}", addr_type),
                });
            }
        };

        let mut skip_buf = vec![0u8; skip_bytes];
        tls_stream.read_exact(&mut skip_buf).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取绑定地址失败: {}", e),
            }
        })?;

        debug!("✅ SOCKS5协商完成，连接已建立");

        // 返回协商完成的TLS流
        Ok(tls_stream)
    }

    /// SOCKS5用户名密码认证
    async fn socks5_username_password_auth(
        &self,
        stream: &mut TlsStream<TcpStream>,
        username: &str,
        password: &str
    ) -> Result<(), PoolError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        debug!("🔑 执行SOCKS5用户名密码认证");

        let mut auth_request = vec![0x01]; // 认证版本
        auth_request.push(username.len() as u8);
        auth_request.extend(username.as_bytes());
        auth_request.push(password.len() as u8);
        auth_request.extend(password.as_bytes());

        stream.write_all(&auth_request).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("发送认证请求失败: {}", e),
            }
        })?;

        let mut auth_response = [0u8; 2];
        stream.read_exact(&mut auth_response).await.map_err(|e| {
            PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: format!("读取认证响应失败: {}", e),
            }
        })?;

        if auth_response[0] != 0x01 {
            return Err(PoolError::ProtocolError {
                url: "socks5+tls".to_string(),
                error: "无效的认证响应版本".to_string(),
            });
        }

        if auth_response[1] != 0x00 {
            return Err(PoolError::ConnectionFailed {
                url: "socks5+tls".to_string(),
                error: "用户名密码认证失败".to_string(),
            });
        }

        debug!("✅ 用户名密码认证成功");
        Ok(())
    }
}

/// 从URL解析代理配置（扩展版本，支持TLS参数）
pub fn parse_proxy_from_url(url: &str) -> Result<Option<(ProxyConfig, TlsConfig)>, PoolError> {
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

        // 解析TLS相关查询参数
        let mut tls_config = TlsConfig::default();
        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "skip_verify" | "insecure" => {
                    tls_config.skip_verify = value.parse().unwrap_or(false);
                }
                "server_name" | "sni" => {
                    tls_config.server_name = Some(value.into_owned());
                }
                "ca" | "ca_cert" => {
                    tls_config.ca_cert_path = Some(value.into_owned());
                }
                "cert" | "client_cert" => {
                    tls_config.client_cert_path = Some(value.into_owned());
                }
                "key" | "client_key" => {
                    tls_config.client_key_path = Some(value.into_owned());
                }
                _ => {}
            }
        }

        let proxy_config = ProxyConfig {
            proxy_type,
            host,
            port,
            username,
            password,
            skip_verify: Some(tls_config.skip_verify),
            server_name: tls_config.server_name.clone(),
            ca_cert: tls_config.ca_cert_path.clone(),
            client_cert: tls_config.client_cert_path.clone(),
            client_key: tls_config.client_key_path.clone(),
        };

        Ok(Some((proxy_config, tls_config)))
    } else {
        Ok(None)
    }
}
