/// DataChannel ↔ UDP bidirectional forwarding
///
/// Each stream maps to one independent UDP endpoint configured via URL:
///   udp://<listen_addr>:<listen_port>?host=<target_host>&port=<target_port>
///
/// - listen_addr:listen_port  — liveion binds here to receive replies from downstream
/// - target_host:target_port  — liveion sends DataChannel messages to this address
///
/// Configuration example (conf/live777.toml):
///   [ptz_udp.streams.camera]
///   url = "udp://127.0.0.1:7774?host=127.0.0.1&port=1234"
///
///   [ptz_udp.streams.camera2]
///   url = "udp://127.0.0.1:7775?host=127.0.0.1&port=1235"
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::{Mutex, broadcast};
use tracing::{debug, info, warn};

use crate::config::PtzUdpStream;

/// Spawn bidirectional forwarding tasks.
/// If `stream_cfg` is None or the URL is invalid, this is a no-op.
pub async fn spawn_ptz_udp(
    stream: String,
    mut dc_rx: broadcast::Receiver<Vec<u8>>,
    dc_tx: broadcast::Sender<Vec<u8>>,
    stream_cfg: Option<PtzUdpStream>,
) {
    let cfg = match stream_cfg {
        Some(c) => c,
        None => {
            debug!("ptz_udp [{}]: no config, skipping", stream);
            return;
        }
    };

    let (listen_addr, listen_port, target_host, target_port) = match cfg.parse() {
        Some(v) => v,
        None => {
            warn!("ptz_udp [{}]: invalid url: {}", stream, cfg.url);
            return;
        }
    };

    let target = format!("{}:{}", target_host, target_port);
    let bind_addr = format!("{}:{}", listen_addr, listen_port);

    info!("ptz_udp [{}]: listen={} target={}", stream, bind_addr, target);

    let send_socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => Arc::new(s),
        Err(e) => { warn!("ptz_udp [{}]: bind send socket failed: {}", stream, e); return; }
    };

    let recv_socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => { info!("ptz_udp [{}]: listening on {}", stream, bind_addr); Arc::new(s) }
        Err(e) => { warn!("ptz_udp [{}]: bind recv socket failed: {}", stream, e); return; }
    };

    let udp_clients: Arc<Mutex<HashMap<String, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // --- DataChannel → UDP ---
    let send_socket_dc = send_socket.clone();
    let udp_clients_dc = udp_clients.clone();
    let stream_dc = stream.clone();
    tokio::spawn(async move {
        loop {
            match dc_rx.recv().await {
                Ok(data) => {
                    debug!("ptz_udp [{}]: DC→UDP {} bytes → {}", stream_dc, data.len(), target);

                    // Prefer sending to known UDP clients (recorded from inbound replies)
                    let clients = udp_clients_dc.lock().await;
                    let mut sent = 0usize;
                    for (_, addr) in clients.iter() {
                        if send_socket_dc.send_to(&data, addr).await.is_ok() {
                            sent += 1;
                        }
                    }
                    drop(clients);

                    // Fall back to configured target address if no known clients
                    if sent == 0 {
                        if let Err(e) = send_socket_dc.send_to(&data, &target).await {
                            warn!("ptz_udp [{}]: send to {} failed: {}", stream_dc, target, e);
                        } else {
                            info!("ptz_udp [{}]: DC→UDP {} bytes → {}", stream_dc, data.len(), target);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("ptz_udp [{}]: lagged, dropped {} messages", stream_dc, n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("ptz_udp [{}]: channel closed", stream_dc); break;
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
                    // Record client address for future DC→UDP replies
                    udp_clients.lock().await.insert(client_id.clone(), addr);
                    debug!("ptz_udp [{}]: UDP→DC {} bytes from {}", stream, n, addr);

                    let msg = serde_json::json!({
                        "type": "udp_to_datachannel",
                        "client_id": client_id,
                        "data": String::from_utf8_lossy(&data),
                    });
                    if let Err(e) = dc_tx.send(msg.to_string().into_bytes()) {
                        warn!("ptz_udp [{}]: forward to DC failed: {}", stream, e);
                    }
                }
                Err(e) => { warn!("ptz_udp [{}]: recv_from failed: {}", stream, e); }
            }
        }
    });
}
