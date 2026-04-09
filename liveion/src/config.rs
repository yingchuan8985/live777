use std::{env, net::SocketAddr, str::FromStr};

use iceserver::{IceServer, default_ice_servers};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub http: Http,
    #[serde(default = "default_ice_servers")]
    pub ice_servers: Vec<IceServer>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub strategy: api::strategy::Strategy,

    #[serde(default)]
    pub sdp: Sdp,

    #[cfg(feature = "net4mqtt")]
    #[serde(default)]
    pub net4mqtt: Option<Net4mqtt>,

    #[serde(default)]
    pub webhook: Webhook,

    #[serde(default)]
    pub ptz_udp: PtzUdp,

    #[cfg(feature = "recorder")]
    #[serde(default)]
    pub recorder: RecorderConfig,

    #[serde(default)]
    pub stream: StreamConfig,
}

#[cfg(feature = "net4mqtt")]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Net4mqtt {
    #[serde(default)]
    pub mqtt_url: String,
    #[serde(default)]
    pub alias: String,
}

#[cfg(feature = "net4mqtt")]
impl Net4mqtt {
    pub fn validate(&mut self) {
        self.mqtt_url = self.mqtt_url.replace("{alias}", &self.alias)
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Webhook {
    #[serde(default)]
    pub webhooks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Http {
    #[serde(default = "default_http_listen")]
    pub listen: SocketAddr,
    #[serde(default)]
    pub cors: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Auth {
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sdp {
    /// Disable specific codecs in SDP negotiation, e.g. ["VP8", "H264"]
    #[serde(default)]
    pub disable_codecs: Vec<String>,
}

fn default_http_listen() -> SocketAddr {
    SocketAddr::from_str(&format!(
        "0.0.0.0:{}",
        env::var("PORT").unwrap_or(String::from("7777"))
    ))
    .expect("invalid listen address")
}

impl Default for Http {
    fn default() -> Self {
        Self {
            listen: default_http_listen(),
            cors: Default::default(),
        }
    }
}

impl Default for Log {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PtzUdp {
    /// Per-stream UDP port configuration, keyed by stream name.
    #[serde(default)]
    pub streams: std::collections::HashMap<String, PtzUdpStream>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PtzUdpStream {
    /// Target port for all control messages (downstream parses message_type)
    pub udp_port: u16,
    /// UDP inbound listen port for replies from downstream
    pub listen_port: u16,
    /// Target host, default "127.0.0.1"
    #[serde(default = "default_target_host")]
    pub target_host: String,
}

fn default_target_host() -> String { "127.0.0.1".to_string() }

impl Default for PtzUdp {
    fn default() -> Self {
        Self { streams: std::collections::HashMap::new() }
    }
}


fn default_log_level() -> String {
    env::var("LOG_LEVEL").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "info".to_string()
        }
    })
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        for ice_server in self.ice_servers.iter() {
            ice_server
                .validate()
                .map_err(|e| anyhow::anyhow!(format!("ice_server error : {}", e)))?;
        }

        #[cfg(feature = "source")]
        for source in &self.stream.sources {
            source
                .validate()
                .map_err(|e| anyhow::anyhow!("source config error: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(feature = "recorder")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderConfig {
    /// List of stream names to automatically record
    #[serde(default)]
    pub auto_streams: Vec<String>,

    /// Storage backend configuration
    #[serde(default)]
    pub storage: storage::StorageConfig,

    /// Node alias for identification (optional)
    #[serde(default)]
    pub node_alias: Option<String>,

    /// Optional path for recorder index file (index.json)
    #[serde(default)]
    pub index_path: Option<String>,

    /// Maximum duration in seconds for a single recording before rotation (0 disables auto-rotation)
    #[serde(default = "default_max_recording_seconds")]
    pub max_recording_seconds: u64,

    /// Async upload configuration
    #[serde(default)]
    pub upload: UploadConfig,
}

#[cfg(feature = "recorder")]
fn default_max_recording_seconds() -> u64 {
    86_400
}

#[cfg(feature = "recorder")]
impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            auto_streams: vec![],
            storage: Default::default(),
            node_alias: None,
            index_path: None,
            max_recording_seconds: default_max_recording_seconds(),
            upload: Default::default(),
        }
    }
}

#[cfg(feature = "recorder")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    /// Enable async uploads via Liveman presigned URLs
    #[serde(default)]
    pub enabled: bool,
    /// Liveman base URL, e.g. http://127.0.0.1:8888
    #[serde(default)]
    pub liveman_url: String,
    /// Liveman bearer token for presign API
    #[serde(default)]
    pub liveman_token: String,
    /// Queue file path for pending uploads
    #[serde(default = "default_upload_queue_path")]
    pub queue_path: String,
    /// Local spool directory for recordings before upload
    #[serde(default = "default_upload_local_dir")]
    pub local_dir: String,
    /// Presigned URL TTL seconds
    #[serde(default = "default_presign_ttl_seconds")]
    pub presign_ttl_seconds: u64,
    /// Upload loop interval in milliseconds
    #[serde(default = "default_upload_interval_ms")]
    pub interval_ms: u64,
    /// Maximum concurrent uploads
    #[serde(default = "default_upload_concurrency")]
    pub concurrency: usize,
}

#[cfg(feature = "recorder")]
impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            liveman_url: String::new(),
            liveman_token: String::new(),
            queue_path: default_upload_queue_path(),
            local_dir: default_upload_local_dir(),
            presign_ttl_seconds: default_presign_ttl_seconds(),
            interval_ms: default_upload_interval_ms(),
            concurrency: default_upload_concurrency(),
        }
    }
}

#[cfg(feature = "recorder")]
fn default_upload_queue_path() -> String {
    "./recordings/upload_queue.jsonl".to_string()
}

#[cfg(feature = "recorder")]
fn default_upload_local_dir() -> String {
    "./recordings".to_string()
}

#[cfg(feature = "recorder")]
fn default_presign_ttl_seconds() -> u64 {
    300
}

#[cfg(feature = "recorder")]
fn default_upload_interval_ms() -> u64 {
    2_000
}

#[cfg(feature = "recorder")]
fn default_upload_concurrency() -> usize {
    2
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamConfig {
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Stream ID
    pub stream_id: String,

    /// Source URL
    /// - RTSP: rtsp://username:password@host:port/path
    /// - SDP file: file:///path/to/file.sdp or /path/to/file.sdp
    pub url: String,
}

impl SourceConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.stream_id.trim().is_empty() {
            anyhow::bail!("stream_id cannot be empty");
        }

        if self.url.trim().is_empty() {
            anyhow::bail!("url cannot be empty");
        }

        let url_lower = self.url.to_lowercase();
        if !url_lower.starts_with("rtsp://")
            && !url_lower.starts_with("rtsps://")
            && !url_lower.starts_with("file://")
            && !url_lower.ends_with(".sdp")
        {
            anyhow::bail!(
                "Invalid URL format: {}. Must be rtsp://, rtsps://, file://, or end with .sdp",
                self.url
            );
        }

        Ok(())
    }
}
