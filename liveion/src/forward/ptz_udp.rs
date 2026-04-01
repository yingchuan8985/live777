/// DataChannel ↔ UDP 双向转发
/// DataChannel → UDP 消息路由（按 message_type 字段）:
///   "ptz_control"     → target_host:8890
///   "media_control"   → target_host:8888
///   "general_control" → target_host:8892
///   其他 / 非 JSON    → target_host:8888
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::{Mutex, broadcast};
use tracing::{debug, info, warn};

/// UDP 转发配置，对应独立方案的 UdpConfig + BridgeConfig
#[derive(Debug, Clone)]
pub struct PtzUdpConfig {
    /// UDP 监听地址（用于接收回传消息），默认 "0.0.0.0:8888"
    pub listen: String,
    /// DataChannel → UDP 的目标主机，默认 "127.0.0.1"
    pub target_host: String,
    /// 无已知客户端时的默认目标地址列表
    pub target_addresses: Vec<String>,
    /// 是否开启详细日志
    pub enable_logging: bool,
}

impl Default for PtzUdpConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:8888".to_string(),
            target_host: "127.0.0.1".to_string(),
            target_addresses: vec![], // 空列表，走 route_port() 路由到对应端口
            enable_logging: true,
        }
    }
}

/// 按 message_type 路由到对应 UDP 端口
fn route_port(data: &[u8]) -> u16 {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
            return match v.get("message_type").and_then(|t| t.as_str()) {
                Some("ptz_control")     => 8890,
                Some("media_control")   => 8888,
                Some("general_control") => 8892,
                _                       => 8888,
            };
        }
    }
    8888
}

/// 启动双向转发任务：
/// - DataChannel → UDP：收到 DC 消息后按 message_type 路由发送
/// - UDP → DataChannel：监听 UDP 端口，收到消息后写回 DataChannel
///
/// `dc_rx`      — 订阅 PeerForwardInternal.data_channel_forward.publish
/// `dc_tx`      — PeerForwardInternal.data_channel_forward.subscribe（写回 DC）
/// `config`     — UDP 配置
pub async fn spawn_ptz_udp(
    mut dc_rx: broadcast::Receiver<Vec<u8>>,
    dc_tx: broadcast::Sender<Vec<u8>>,
    config: PtzUdpConfig,
) {
    // 绑定发送用 socket（临时端口）
    let send_socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            warn!("ptz_udp: failed to bind send socket: {}", e);
            return;
        }
    };

    // 绑定监听 socket（接收 UDP 回传）
    let recv_socket = match UdpSocket::bind(&config.listen).await {
        Ok(s) => {
            info!("ptz_udp: listening on {}", config.listen);
            Arc::new(s)
        }
        Err(e) => {
            warn!("ptz_udp: failed to bind recv socket on {}: {}", config.listen, e);
            return;
        }
    };

    // 记录已知 UDP 客户端
    let udp_clients: Arc<Mutex<HashMap<String, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let target_host = config.target_host.clone();
    let target_addresses = config.target_addresses.clone();
    let enable_logging = config.enable_logging;

    // --- DataChannel → UDP ---
    let send_socket_dc = send_socket.clone();
    let udp_clients_dc = udp_clients.clone();
    let target_addresses_dc = target_addresses.clone();
    tokio::spawn(async move {
        loop {
            match dc_rx.recv().await {
                Ok(data) => {
                    // 解析 message_type 决定目标端口
                    let port = route_port(&data);
                    let target = format!("{}:{}", target_host, port);

                    if enable_logging {
                        debug!("ptz_udp: DC→UDP {} bytes → {}", data.len(), target);
                    }

                    // 优先发给已知 UDP 客户端
                    let clients = udp_clients_dc.lock().await;
                    let mut sent = 0usize;
                    for (_, addr) in clients.iter() {
                        if let Err(e) = send_socket_dc.send_to(&data, addr).await {
                            warn!("ptz_udp: send to {} failed: {}", addr, e);
                        } else {
                            sent += 1;
                        }
                    }
                    drop(clients);

                    // 无已知客户端时发到默认目标
                    if sent == 0 {
                        if target_addresses_dc.is_empty() {
                            // 直接发到路由端口
                            if let Err(e) = send_socket_dc.send_to(&data, &target).await {
                                warn!("ptz_udp: send to {} failed: {}", target, e);
                            } else {
                                info!("ptz_udp: DC→UDP {} bytes → {}", data.len(), target);
                            }
                        } else {
                            for addr_str in &target_addresses_dc {
                                if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                                    if let Err(e) = send_socket_dc.send_to(&data, addr).await {
                                        warn!("ptz_udp: send to {} failed: {}", addr, e);
                                    } else {
                                        info!("ptz_udp: DC→UDP {} bytes → {}", data.len(), addr);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("ptz_udp: DC channel lagged, dropped {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("ptz_udp: DC channel closed, dc→udp task exit");
                    break;
                }
            }
        }
    });

    // --- UDP → DataChannel ---
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1024 * 16];
        loop {
            match recv_socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    let data = buf[..n].to_vec();
                    let client_id = format!("{}:{}", addr.ip(), addr.port());

                    // 记录客户端地址
                    udp_clients.lock().await.insert(client_id.clone(), addr);

                    if enable_logging {
                        debug!("ptz_udp: UDP→DC {} bytes from {}", n, addr);
                    }

                    // 包装成与独立方案一致的 JSON 格式
                    let msg = serde_json::json!({
                        "type": "udp_to_datachannel",
                        "client_id": client_id,
                        "data": String::from_utf8_lossy(&data),
                    });

                    if let Err(e) = dc_tx.send(msg.to_string().into_bytes()) {
                        warn!("ptz_udp: forward to DC failed: {}", e);
                    }
                }
                Err(e) => {
                    warn!("ptz_udp: recv_from failed: {}", e);
                }
            }
        }
    });
}
