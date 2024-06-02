// Yeahbut May 2024

use tokio::net::TcpListener;
use std::error::Error;
use rsa::RsaPrivateKey;

#[derive(Copy, Clone)]
pub enum OnlineStatus {
    Online,
    Offline,
}

#[derive(Clone)]
pub struct ProxyInfo {
    pub proxy_addr: String,
    pub proxy_port: u16,
    pub online_status: OnlineStatus,
    pub backend_addr: String,
    pub backend_port: u16,
    pub private_key: RsaPrivateKey,
}

impl ProxyInfo {
    pub fn formatted_proxy_address(&self) -> String {
        format!("{}:{}", self.proxy_addr, self.proxy_port)
    }

    pub fn formatted_backend_address(&self) -> String {
        format!("{}:{}", self.backend_addr, self.backend_port)
    }
}

pub struct TcpListenerWrapper {
    pub listener: TcpListener,
    pub info: ProxyInfo,
}

impl TcpListenerWrapper {
    pub async fn bind(info: ProxyInfo) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            listener: TcpListener::bind(
                info.formatted_proxy_address()).await?,
            info: info,
        })
    }
}
