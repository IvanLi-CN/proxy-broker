use std::{
    collections::HashMap,
    fs,
    io::Read,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use flate2::read::GzDecoder;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{process::Child, process::Command, sync::Mutex, time::sleep};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone)]
pub struct MihomoRuntimeOptions {
    pub binary_path: Option<PathBuf>,
    pub auto_download: bool,
    pub work_dir: PathBuf,
    pub startup_timeout_sec: u64,
    pub secret: Option<String>,
}

impl Default for MihomoRuntimeOptions {
    fn default() -> Self {
        Self {
            binary_path: None,
            auto_download: true,
            work_dir: PathBuf::from(".proxy-broker/runtime"),
            startup_timeout_sec: 15,
            secret: None,
        }
    }
}

#[async_trait]
pub trait MihomoRuntime: Send + Sync {
    async fn ensure_started(&self, profile_id: &str) -> anyhow::Result<()>;
    async fn shutdown_profile(&self, profile_id: &str) -> anyhow::Result<()>;
    async fn controller_meta(&self, profile_id: &str) -> anyhow::Result<(String, Option<String>)>;
    async fn controller_addr(&self, profile_id: &str) -> anyhow::Result<String>;
    async fn apply_config(&self, profile_id: &str, payload: &str) -> anyhow::Result<()>;
    async fn measure_proxy_delay(
        &self,
        profile_id: &str,
        proxy_name: &str,
        url: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<Option<u64>>;
}

#[derive(Debug)]
struct RuntimeInstance {
    controller_addr: String,
    secret: Option<String>,
    _home_dir: PathBuf,
    _config_path: PathBuf,
    child: Child,
}

#[derive(Clone)]
pub struct ManagedMihomoRuntime {
    options: MihomoRuntimeOptions,
    http: reqwest::Client,
    instances: Arc<Mutex<HashMap<String, RuntimeInstance>>>,
    binary_cache: Arc<Mutex<Option<PathBuf>>>,
    ensure_lock: Arc<Mutex<()>>,
}

impl ManagedMihomoRuntime {
    pub fn new(options: MihomoRuntimeOptions) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            options,
            http,
            instances: Arc::new(Mutex::new(HashMap::new())),
            binary_cache: Arc::new(Mutex::new(None)),
            ensure_lock: Arc::new(Mutex::new(())),
        }
    }

    fn profile_safe_name(profile_id: &str) -> String {
        let mut readable: String = profile_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        readable.truncate(24);
        if readable.is_empty() {
            readable = "profile".to_string();
        }
        let stable_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, profile_id.as_bytes())
            .simple()
            .to_string();
        format!("{readable}-{stable_id}")
    }

    fn pick_controller_port(seed: &str) -> anyhow::Result<u16> {
        let mut hash: u64 = 1469598103934665603;
        for b in seed.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        let start = 19090 + (hash % 1000) as u16;
        for offset in 0..500 {
            let port = start + offset;
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(port);
            }
        }
        let socket = TcpListener::bind(("127.0.0.1", 0))?;
        Ok(socket.local_addr()?.port())
    }

    fn platform_tokens() -> anyhow::Result<(&'static str, &'static str)> {
        let os_token = match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            other => return Err(anyhow!("unsupported os for auto download: {other}")),
        };
        let arch_token = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => return Err(anyhow!("unsupported arch for auto download: {other}")),
        };
        Ok((os_token, arch_token))
    }

    fn discover_cached_binary(
        bin_dir: &Path,
        os_token: &str,
        arch_token: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        if !bin_dir.exists() {
            return Ok(None);
        }

        let mut latest: Option<(SystemTime, PathBuf)> = None;
        for entry in fs::read_dir(bin_dir)
            .with_context(|| format!("failed to read bin dir: {}", bin_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !file_name.contains("mihomo")
                || !file_name.contains(os_token)
                || !file_name.contains(arch_token)
            {
                continue;
            }

            #[cfg(unix)]
            {
                let mode = entry.metadata()?.permissions().mode();
                if mode & 0o111 == 0 {
                    continue;
                }
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            match &latest {
                Some((latest_modified, _)) if *latest_modified >= modified => {}
                _ => latest = Some((modified, path)),
            }
        }

        Ok(latest.map(|(_, path)| path))
    }

    async fn resolve_binary(&self) -> anyhow::Result<PathBuf> {
        if let Some(path) = &self.options.binary_path {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(anyhow!(
                "configured mihomo binary does not exist: {}",
                path.display()
            ));
        }
        let (os_token, arch_token) = Self::platform_tokens()?;

        let mut guard = self.binary_cache.lock().await;
        if let Some(path) = &*guard
            && path.exists()
        {
            return Ok(path.clone());
        }

        let cached =
            Self::discover_cached_binary(&self.options.work_dir.join("bin"), os_token, arch_token)?;
        if let Some(path) = cached {
            *guard = Some(path.clone());
            return Ok(path);
        }

        if !self.options.auto_download {
            return Err(anyhow!(
                "mihomo binary path is missing, no cached binary found, and auto_download is disabled"
            ));
        }

        let downloaded = self.download_latest_binary().await?;
        *guard = Some(downloaded.clone());
        Ok(downloaded)
    }

    fn verify_sha256_digest(bytes: &[u8], expected_digest: &str) -> anyhow::Result<()> {
        let expected = expected_digest
            .trim()
            .strip_prefix("sha256:")
            .unwrap_or(expected_digest)
            .trim();
        if expected.is_empty() {
            return Err(anyhow!("release asset digest is empty"));
        }

        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(anyhow!(
                "sha256 mismatch: expected sha256:{expected}, got sha256:{actual}"
            ));
        }
        Ok(())
    }

    async fn download_latest_binary(&self) -> anyhow::Result<PathBuf> {
        #[derive(Debug, Deserialize)]
        struct ReleaseAsset {
            name: String,
            browser_download_url: String,
            digest: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct Release {
            tag_name: String,
            assets: Vec<ReleaseAsset>,
        }

        let release: Release = self
            .http
            .get("https://api.github.com/repos/MetaCubeX/mihomo/releases/latest")
            .header(USER_AGENT, "proxy-broker")
            .send()
            .await
            .context("failed to query mihomo release")?
            .error_for_status()
            .context("mihomo release api failed")?
            .json()
            .await
            .context("failed to decode mihomo release response")?;

        let (os_token, arch_token) = Self::platform_tokens()?;

        let asset = release
            .assets
            .into_iter()
            .find(|a| {
                let name = a.name.to_ascii_lowercase();
                name.contains(os_token) && name.contains(arch_token) && name.ends_with(".gz")
            })
            .ok_or_else(|| anyhow!("failed to find matching mihomo release asset"))?;

        let bytes = self
            .http
            .get(&asset.browser_download_url)
            .header(USER_AGENT, "proxy-broker")
            .send()
            .await
            .context("failed to download mihomo binary")?
            .error_for_status()
            .context("mihomo binary download returned non-2xx")?
            .bytes()
            .await
            .context("failed to read mihomo binary bytes")?;
        if let Some(expected_digest) = asset.digest.as_deref() {
            Self::verify_sha256_digest(&bytes, expected_digest)
                .with_context(|| format!("digest verification failed for {}", asset.name))?;
        } else {
            tracing::warn!(
                asset_name = %asset.name,
                "mihomo release asset has no digest, skipping verification"
            );
        }

        let bin_dir = self.options.work_dir.join("bin");
        tokio::fs::create_dir_all(&bin_dir)
            .await
            .with_context(|| format!("failed to create bin dir: {}", bin_dir.display()))?;

        let out = bin_dir.join(format!(
            "mihomo-{}-{}-{}",
            release.tag_name, os_token, arch_token
        ));

        let bytes_vec = bytes.to_vec();
        let out_clone = out.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut decoder = GzDecoder::new(&bytes_vec[..]);
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .context("failed to unzip mihomo asset")?;
            fs::write(&out_clone, decoded)
                .with_context(|| format!("failed to write binary: {}", out_clone.display()))?;
            Ok(())
        })
        .await
        .context("download task join failed")??;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&out)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&out, perms)?;
        }

        Ok(out)
    }

    fn make_bootstrap_yaml(
        controller_addr: &str,
        secret: &Option<String>,
    ) -> anyhow::Result<String> {
        let mut root = serde_json::json!({
            "mode": "rule",
            "log-level": "warning",
            "allow-lan": false,
            "external-controller": controller_addr,
            "proxies": [],
            "rules": ["MATCH,DIRECT"],
        });
        if let Some(secret) = secret {
            root["secret"] = serde_json::Value::String(secret.clone());
        }
        serde_yaml::to_string(&root).map_err(|e| anyhow!(e))
    }

    async fn start_profile(&self, profile_id: &str) -> anyhow::Result<RuntimeInstance> {
        let binary = self.resolve_binary().await?;

        let safe_profile = Self::profile_safe_name(profile_id);
        let home_dir = self.options.work_dir.join("profiles").join(&safe_profile);
        tokio::fs::create_dir_all(&home_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create profile runtime dir: {}",
                    home_dir.display()
                )
            })?;

        let config_path = home_dir.join("config.yaml");
        let startup_retries = 3usize;
        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..startup_retries {
            let port_seed = format!("{profile_id}-{attempt}");
            let port = Self::pick_controller_port(&port_seed)?;
            let controller_addr = format!("127.0.0.1:{port}");
            let bootstrap = Self::make_bootstrap_yaml(&controller_addr, &self.options.secret)?;
            tokio::fs::write(&config_path, bootstrap)
                .await
                .with_context(|| {
                    format!(
                        "failed to write bootstrap config: {}",
                        config_path.display()
                    )
                })?;

            let child = Command::new(&binary)
                .arg("-d")
                .arg(&home_dir)
                .arg("-f")
                .arg(&config_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .with_context(|| format!("failed to start mihomo: {}", binary.display()))?;

            let mut instance = RuntimeInstance {
                controller_addr: controller_addr.clone(),
                secret: self.options.secret.clone(),
                _home_dir: home_dir.clone(),
                _config_path: config_path.clone(),
                child,
            };

            if let Err(err) = self
                .wait_controller_ready(&controller_addr, instance.secret.as_ref())
                .await
            {
                let _ = instance.child.start_kill();
                let _ = instance.child.wait().await;
                last_err = Some(err.context(format!(
                    "mihomo controller is not ready for profile={profile_id}, addr={controller_addr}, attempt={}",
                    attempt + 1
                )));
                if attempt + 1 < startup_retries {
                    continue;
                }
            } else {
                return Ok(instance);
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("failed to start mihomo runtime")))
    }

    async fn wait_controller_ready(
        &self,
        controller_addr: &str,
        secret: Option<&String>,
    ) -> anyhow::Result<()> {
        let endpoint = format!("http://{controller_addr}/version");
        let deadline =
            std::time::Instant::now() + Duration::from_secs(self.options.startup_timeout_sec);

        loop {
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(anyhow!("controller readiness timeout"));
            }
            let remaining = deadline.saturating_duration_since(now);
            let mut req = self
                .http
                .get(&endpoint)
                .timeout(remaining.min(Duration::from_millis(800)))
                .header(USER_AGENT, "proxy-broker");
            if let Some(secret) = secret {
                req = req.header(AUTHORIZATION, format!("Bearer {secret}"));
            }

            if let Ok(resp) = req.send().await
                && resp.status().is_success()
            {
                return Ok(());
            }

            if std::time::Instant::now() >= deadline {
                return Err(anyhow!("controller readiness timeout"));
            }
            sleep(Duration::from_millis(250)).await;
        }
    }

    async fn ensure_instance(&self, profile_id: &str) -> anyhow::Result<()> {
        let _ensure_guard = self.ensure_lock.lock().await;

        {
            let mut guard = self.instances.lock().await;
            if let Some(instance) = guard.get_mut(profile_id) {
                if let Some(status) = instance.child.try_wait()? {
                    tracing::warn!(
                        profile_id,
                        status = ?status,
                        "existing mihomo child exited, restarting"
                    );
                    guard.remove(profile_id);
                } else {
                    return Ok(());
                }
            }
        }

        let instance = self.start_profile(profile_id).await?;
        let mut guard = self.instances.lock().await;
        guard.insert(profile_id.to_string(), instance);
        Ok(())
    }

    async fn instance_meta(&self, profile_id: &str) -> anyhow::Result<(String, Option<String>)> {
        self.ensure_instance(profile_id).await?;
        let guard = self.instances.lock().await;
        let instance = guard
            .get(profile_id)
            .ok_or_else(|| anyhow!("runtime instance missing after start"))?;
        Ok((instance.controller_addr.clone(), instance.secret.clone()))
    }

    async fn shutdown_instance(&self, profile_id: &str) -> anyhow::Result<()> {
        let removed = {
            let mut guard = self.instances.lock().await;
            guard.remove(profile_id)
        };
        if let Some(mut instance) = removed {
            let _ = instance.child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(3), instance.child.wait()).await;
        }
        Ok(())
    }
}

#[async_trait]
impl MihomoRuntime for ManagedMihomoRuntime {
    async fn ensure_started(&self, profile_id: &str) -> anyhow::Result<()> {
        self.ensure_instance(profile_id).await
    }

    async fn shutdown_profile(&self, profile_id: &str) -> anyhow::Result<()> {
        self.shutdown_instance(profile_id).await
    }

    async fn controller_meta(&self, profile_id: &str) -> anyhow::Result<(String, Option<String>)> {
        self.instance_meta(profile_id).await
    }

    async fn controller_addr(&self, profile_id: &str) -> anyhow::Result<String> {
        let (addr, _secret) = self.controller_meta(profile_id).await?;
        Ok(addr)
    }

    async fn apply_config(&self, profile_id: &str, payload: &str) -> anyhow::Result<()> {
        let (controller_addr, secret) = self.controller_meta(profile_id).await?;
        let endpoint = format!("http://{}/configs?force=true", controller_addr);
        let mut req = self
            .http
            .put(endpoint)
            .header(USER_AGENT, "proxy-broker")
            .json(&serde_json::json!({ "path": "", "payload": payload }));
        if let Some(secret) = &secret {
            req = req.header(AUTHORIZATION, format!("Bearer {secret}"));
        }
        let resp = req.send().await.context("failed to call /configs")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("/configs failed: status={status}, body={body}"));
        }
        Ok(())
    }

    async fn measure_proxy_delay(
        &self,
        profile_id: &str,
        proxy_name: &str,
        url: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<Option<u64>> {
        #[derive(Deserialize)]
        struct DelayResponse {
            delay: Option<u64>,
        }

        let (controller_addr, secret) = self.controller_meta(profile_id).await?;
        let encoded = urlencoding::encode(proxy_name);
        let endpoint = format!(
            "http://{}/proxies/{}/delay?url={}&timeout={}",
            controller_addr,
            encoded,
            urlencoding::encode(url),
            timeout_ms
        );
        let mut req = self.http.get(endpoint).header(USER_AGENT, "proxy-broker");
        if let Some(secret) = &secret {
            req = req.header(AUTHORIZATION, format!("Bearer {secret}"));
        }
        let resp = req.send().await.context("failed to call delay api")?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: DelayResponse = resp
            .json()
            .await
            .context("failed to decode delay response")?;
        Ok(parsed.delay)
    }
}

impl Drop for ManagedMihomoRuntime {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.instances.try_lock() {
            for (_profile, instance) in guard.iter_mut() {
                let _ = instance.child.start_kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ManagedMihomoRuntime;

    #[test]
    fn sha256_digest_verification_accepts_matching_digest() {
        let bytes = b"proxy-broker";
        let digest = "sha256:36f6994cc3abef9332e9975a7df77cdc0ad7cf1f5fc7a88f8f797f1065c423e5";
        ManagedMihomoRuntime::verify_sha256_digest(bytes, digest)
            .expect("matching digest should pass");
    }

    #[test]
    fn sha256_digest_verification_rejects_mismatch() {
        let bytes = b"proxy-broker";
        let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let err = ManagedMihomoRuntime::verify_sha256_digest(bytes, digest)
            .expect_err("mismatched digest should fail");
        assert!(err.to_string().contains("sha256 mismatch"));
    }
}
