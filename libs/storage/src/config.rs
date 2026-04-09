use serde::{Deserialize, Serialize};

/// Unified storage configuration for Live777 components
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    /// Local filesystem storage (basic maintenance; no presign support)
    Fs {
        /// Root directory for storing recordings
        root: String,
    },
    /// AWS S3 compatible storage
    S3 {
        /// S3 bucket name
        bucket: String,
        /// Root path within bucket
        #[serde(default = "default_s3_root")]
        root: String,
        /// AWS region
        #[serde(default)]
        region: Option<String>,
        /// Custom endpoint for S3-compatible services
        #[serde(default)]
        endpoint: Option<String>,
        /// Access key ID
        #[serde(default)]
        access_key_id: Option<String>,
        /// Secret access key
        #[serde(default)]
        secret_access_key: Option<String>,
        /// Session token for temporary credentials
        #[serde(default)]
        session_token: Option<String>,
        /// Disable config/credential auto-loading
        #[serde(default)]
        disable_config_load: bool,
        /// Enable virtual host style addressing
        #[serde(default)]
        enable_virtual_host_style: bool,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::Fs {
            root: "./storage".to_string(),
        }
    }
}

fn default_s3_root() -> String {
    "/".to_string()
}
