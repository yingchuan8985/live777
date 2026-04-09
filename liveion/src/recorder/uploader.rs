use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use http::header;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, warn};

use crate::config::UploadConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadEntry {
    id: String,
    object_key: String,
    local_path: String,
    retry_count: u32,
    next_retry_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PresignRequest {
    method: String,
    path: String,
    ttl_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PresignResponse {
    url: String,
    headers: HashMap<String, String>,
}

pub struct UploadManager {
    cfg: UploadConfig,
    client: Client,
    entries: RwLock<HashMap<String, UploadEntry>>,
    write_lock: Mutex<()>,
    semaphore: Arc<Semaphore>,
    last_ping_fail: Mutex<i64>,
}

impl UploadManager {
    pub async fn load(cfg: UploadConfig) -> Result<Self> {
        let client = Client::new();
        let mut entries = HashMap::new();
        let path = PathBuf::from(&cfg.queue_path);
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<UploadEntry>(line) {
                    entries.insert(entry.id.clone(), entry);
                }
            }
        }

        let concurrency = cfg.concurrency.max(1);
        Ok(Self {
            cfg,
            client,
            entries: RwLock::new(entries),
            write_lock: Mutex::new(()),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            last_ping_fail: Mutex::new(0),
        })
    }

    pub fn local_dir(&self) -> String {
        self.cfg.local_dir.clone()
    }

    pub async fn enqueue(&self, object_key: String, local_path: String) -> Result<()> {
        let entry = UploadEntry {
            id: format!("{}:{}", object_key, chrono::Utc::now().timestamp_millis()),
            object_key,
            local_path,
            retry_count: 0,
            next_retry_at: 0,
        };
        {
            let mut map = self.entries.write().await;
            map.insert(entry.id.clone(), entry);
        }
        self.persist_queue().await
    }

    pub async fn run(self: std::sync::Arc<Self>) {
        let interval = Duration::from_millis(self.cfg.interval_ms.max(500));
        loop {
            tokio::time::sleep(interval).await;
            if let Err(e) = self.clone().process_queue().await {
                warn!("[uploader] queue processing failed: {}", e);
            }
        }
    }

    async fn process_queue(self: std::sync::Arc<Self>) -> Result<()> {
        if !self.is_liveman_available().await? {
            return Ok(());
        }
        let now = chrono::Utc::now().timestamp_millis();
        let entries: Vec<UploadEntry> = {
            let map = self.entries.read().await;
            map.values()
                .filter(|entry| entry.next_retry_at <= now)
                .cloned()
                .collect()
        };

        if entries.is_empty() {
            return Ok(());
        }

        for entry in entries {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let this = self.clone();
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = this.try_upload(entry).await {
                    warn!("[uploader] upload failed: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn try_upload(&self, mut entry: UploadEntry) -> Result<()> {
        let presign = self.presign_put(&entry.object_key).await?;
        let body = tokio::fs::read(&entry.local_path)
            .await
            .with_context(|| format!("read local file {}", entry.local_path))?;

        let mut req = self.client.put(presign.url);
        for (k, v) in presign.headers {
            if let (Ok(name), Ok(value)) = (
                header::HeaderName::from_bytes(k.as_bytes()),
                header::HeaderValue::from_str(&v),
            ) {
                req = req.header(name, value);
            }
        }

        let resp = req.body(body).send().await?;
        if !resp.status().is_success() {
            entry.retry_count += 1;
            entry.next_retry_at = backoff_ts(entry.retry_count);
            self.update_entry(entry).await?;
            return Err(anyhow::anyhow!("upload failed: {}", resp.status()));
        }

        debug!("[uploader] uploaded {}", entry.object_key);
        let _ = tokio::fs::remove_file(&entry.local_path).await;
        self.remove_entry(&entry.id).await?;
        Ok(())
    }

    async fn presign_put(&self, object_key: &str) -> Result<PresignResponse> {
        let url = format!(
            "{}/api/storage/presign",
            self.cfg.liveman_url.trim_end_matches('/')
        );
        let req = PresignRequest {
            method: "PUT".to_string(),
            path: object_key.to_string(),
            ttl_seconds: self.cfg.presign_ttl_seconds.max(30),
        };
        let mut builder = self.client.post(url).json(&req);
        if !self.cfg.liveman_token.is_empty() {
            builder = builder.header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.cfg.liveman_token),
            );
        }
        let resp = builder.send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("presign failed: {}", resp.status()));
        }
        Ok(resp.json::<PresignResponse>().await?)
    }

    async fn is_liveman_available(&self) -> Result<bool> {
        if self.cfg.liveman_url.trim().is_empty() {
            return Ok(false);
        }
        let now = chrono::Utc::now().timestamp_millis();
        let mut last_fail = self.last_ping_fail.lock().await;
        if *last_fail != 0 && now - *last_fail < 5_000 {
            return Ok(false);
        }

        let url = format!(
            "{}/api/storage/ping",
            self.cfg.liveman_url.trim_end_matches('/')
        );
        let mut req = self.client.get(url);
        if !self.cfg.liveman_token.is_empty() {
            req = req.header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.cfg.liveman_token),
            );
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                *last_fail = 0;
                Ok(true)
            }
            Ok(resp) => {
                *last_fail = now;
                warn!("[uploader] liveman ping failed: {}", resp.status());
                Ok(false)
            }
            Err(e) => {
                *last_fail = now;
                warn!("[uploader] liveman ping error: {}", e);
                Ok(false)
            }
        }
    }

    async fn update_entry(&self, entry: UploadEntry) -> Result<()> {
        {
            let mut map = self.entries.write().await;
            map.insert(entry.id.clone(), entry);
        }
        self.persist_queue().await
    }

    async fn remove_entry(&self, id: &str) -> Result<()> {
        {
            let mut map = self.entries.write().await;
            map.remove(id);
        }
        self.persist_queue().await
    }

    async fn persist_queue(&self) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let entries: Vec<UploadEntry> = {
            let map = self.entries.read().await;
            map.values().cloned().collect()
        };

        let path = PathBuf::from(&self.cfg.queue_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let tmp_path = tmp_path_for(&path);
        let mut contents = String::new();
        for entry in entries {
            let line = serde_json::to_string(&entry)?;
            contents.push_str(&line);
            contents.push('\n');
        }
        tokio::fs::write(&tmp_path, contents).await?;
        if tokio::fs::metadata(&path).await.is_ok() {
            let _ = tokio::fs::remove_file(&path).await;
        }
        tokio::fs::rename(&tmp_path, &path)
            .await
            .with_context(|| format!("replace upload queue {}", path.display()))?;

        Ok(())
    }
}

fn backoff_ts(retry: u32) -> i64 {
    let base = 5_000i64;
    let max = 10 * 60 * 1000i64;
    let delay = (base * (1i64 << retry.min(10))).min(max).max(base);
    chrono::Utc::now().timestamp_millis() + delay
}

fn tmp_path_for(path: &Path) -> PathBuf {
    let mut tmp = path.to_path_buf();
    if let Some(ext) = path.extension() {
        let mut ext = ext.to_os_string();
        ext.push(".tmp");
        tmp.set_extension(ext);
    } else {
        tmp.set_extension("tmp");
    }
    tmp
}
