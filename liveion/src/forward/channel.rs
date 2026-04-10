/// DataChannel <-> UDP bidirectional forwarding
///
/// Each stream maps to one independent UDP endpoint configured via URL:
///   udp://<listen_addr>:<listen_port>?host=<target_host>&port=<target_port>
///
/// - listen_addr:listen_port  -- liveion binds here to receive replies from downstream
/// - target_host:target_port  -- liveion sends DataChannel messages to this address
///
/// Configuration example (conf/live777.toml):
///   [channel.streams.camera]
///   url = "udp://0.0.0.0:7774?host=127.0.0.1&port=1234"
///
///   [channel.streams.camera2]
///   url = "udp://0.0.0.0:7775?host=127.0.0.1&port=1235"
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::config::ChannelStream;

/// Spawn bidirectional forwarding tasks.
/// If `stream_cfg` is None or the URL is invalid, this is a no-op.
pub async fn spawn_channel(
    stream: String,
    mut dc_rx: broadcast::Receiver<Vec<u8>>,
    dc_tx: broadcast::Sender<Vec<u8>>,
    stream_cfg: Option<ChannelStream>,
) {
    let cfg = match stream_cfg {
        Some(c) => c,
        None => {
            debug!("channel [{}]: no config, skipping", stream);
            return;
        }
    };

    let (listen_addr, listen_port, target_host, target_port) = match cfg.parse() {
        Some(v) => v,
        None => {
            warn!("channel [{}]: invalid url: {}", stream, cfg.url);
            return;
        }
    };

    // Format addresses, wrapping IPv6 in brackets
    let target = if target_host.contains(':') {
        format!("[{}]:{}", target_host, target_port)
    } else {
        format!("{}:{}", target_host, target_port)
    };
    let bind_addr = if listen_addr.contains(':') {
        format!("[{}]:{}", listen_addr, listen_port)
    } else {
        format!("{}:{}", listen_addr, listen_port)
    };

    let socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => {
            info!("channel [{}]: listen={} target={}", stream, bind_addr, target);
            Arc::new(s)
        }
        Err(e) => {
            warn!("channel [{}]: bind socket failed: {}", stream, e);
            return;
        }
    };

    // --- DataChannel -> UDP ---
    let socket_dc = socket.clone();
    let stream_dc = stream.clone();
    tokio::spawn(async move {
        loop {
            match dc_rx.recv().await {
                Ok(data) => {
                    if let Err(e) = socket_dc.send_to(&data, &target).await {
                        warn!("channel [{}]: send to {} failed: {}", stream_dc, target, e);
                    } else {
                        debug!("channel [{}]: DC->UDP {} bytes -> {}", stream_dc, data.len(), target);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("channel [{}]: lagged, dropped {} messages", stream_dc, n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("channel [{}]: channel closed", stream_dc);
                    break;
                }
            }
        }
    });

    // --- UDP -> DataChannel (passthrough, no wrapping) ---
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    let data = buf[..n].to_vec();
                    debug!("channel [{}]: UDP->DC {} bytes from {}", stream, n, addr);
                    if let Err(e) = dc_tx.send(data) {
                        warn!("channel [{}]: forward to DC failed: {}", stream, e);
                    }
                }
                Err(e) => {
                    warn!("channel [{}]: recv_from failed: {}", stream, e);
                }
            }
        }
    });
}
