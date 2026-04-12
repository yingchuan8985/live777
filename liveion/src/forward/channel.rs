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
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::config::ChannelStream;

/// Buffer size for incoming UDP packets.
/// Control messages (e.g. PTZ commands) are typically small JSON payloads,
/// well within 1KB. Using a fixed 1KB buffer avoids unnecessary memory usage.
const UDP_BUF_SIZE: usize = 1024;

/// Spawn bidirectional forwarding tasks.
/// If `stream_cfg` is None or the URL is invalid, this is a no-op.
pub async fn spawn_channel(
    stream: String,
    mut dc_rx: broadcast::Receiver<Vec<u8>>,
    dc_tx: broadcast::Sender<Vec<u8>>,
    stream_cfg: Option<ChannelStream>,
) -> anyhow::Result<()> {
    let cfg = match stream_cfg {
        Some(c) => c,
        None => {
            debug!("channel [{}]: no config, skipping", stream);
            return Ok(());
        }
    };

    let (listen_host, listen_port, target_host, target_port) = match cfg.parse() {
        Some(v) => v,
        None => {
            warn!("channel [{}]: invalid url: {}", stream, cfg.url);
            return Ok(());
        }
    };

    // Format socket addresses using url::Host to correctly handle IPv6 brackets
    let target = format!("{}:{}", target_host, target_port);
    let bind_addr = format!("{}:{}", listen_host, listen_port);

    let socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => {
            info!("channel [{}]: listen={} target={}", stream, bind_addr, target);
            s
        }
        Err(e) => {
            warn!("channel [{}]: bind socket failed on {}: {}", stream, bind_addr, e);
            return Err(anyhow::anyhow!("channel [{}]: bind {} failed: {}", stream, bind_addr, e));
        }
    };

    // Bidirectional forwarding using tokio::select! to handle both directions concurrently
    let stream_dc = stream.clone();
    tokio::spawn(async move {
        let mut buf = vec![0u8; UDP_BUF_SIZE];
        loop {
            tokio::select! {
                // DataChannel -> UDP
                result = dc_rx.recv() => match result {
                    Ok(data) => {
                        if let Err(e) = socket.send_to(&data, &target).await {
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
                },
                // UDP -> DataChannel (passthrough, no wrapping)
                result = socket.recv_from(&mut buf) => match result {
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
                },
            }
        }
    });

    Ok(())
}
