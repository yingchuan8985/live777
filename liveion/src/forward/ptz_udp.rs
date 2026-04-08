/// DataChannel ↔ UDP bidirectional forwarding
///
/// Each stream maps to one independent UDP port.
/// All message types are forwarded to the same port;
/// the downstream is responsible for parsing the message_type field.
///
/// Configuration example (conf/live777.toml):
///   [ptz_udp.streams.camera]
///   udp_port    = 8890   # target port for all control messages
///   listen_port = 8891   # UDP inbound listen port (for replies)
///   target_host = "127.0.0.1"
///
///   [ptz_udp.streams.camera2]
///   udp_port    = 8990
///   listen_port = 8991
///   target_host = "127.0.0.1"
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::{Mutex, broadcast};
use tracing::{debug, info, warn};

use crate::config::PtzUdpStream;

/// Spawn bidirectional forwarding tasks.
/// If `stream_cfg` is None (stream not configured), this is a no-op.
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

    let target = format!("{}:{}", cfg.target_host, cfg.udp_port);
    let listen_addr = format!("0.0.0.0:{}", cfg.listen_port);

    info!("ptz_udp [{}]: udp_port={} listen_port={}", stream, cfg.udp_port, cfg.listen_port);

    let send_socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => Arc::new(s),
        Err(e) => { warn!("ptz_udp [{}]: bind send socket failed: {}", stream, e); return; }
    };

    let recv_socket = match UdpSocket::bind(&listen_addr).await {
        Ok(s) => { info!("ptz_udp [{}]: listening on {}", stream, listen_addr); Arc::new(s) }
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
