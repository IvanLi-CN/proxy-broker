use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, anyhow};
use futures_util::{StreamExt, TryStreamExt, stream};
use maxminddb::{Reader, geoip2};
use serde::Deserialize;
use tokio::sync::Mutex as TokioMutex;

use crate::{
    auth::{Principal, constant_time_eq, hash_secret, issue_api_key, parse_api_key_secret},
    config_render::{dedicated_ip_proxy_name, render_payload},
    constants::{
        DEFAULT_GEO_ONLINE_CONCURRENCY, DEFAULT_GEO_TTL_SEC, DEFAULT_MMDB_URL,
        DEFAULT_ONLINE_GEO_BASE, DEFAULT_PROBE_CONCURRENCY, DEFAULT_PROBE_TARGETS,
        DEFAULT_PROBE_TIMEOUT_MS, DEFAULT_PROBE_TTL_SEC, DEFAULT_SESSION_LISTEN_IP,
    },
    error::{BrokerError, BrokerResult},
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProfileResponse, ExtractIpItem,
        ExtractIpRequest, ExtractIpResponse, IpRecord, ListApiKeysResponse, ListProfilesResponse,
        ListSessionsResponse, LoadSubscriptionResponse, OpenBatchRequest, OpenBatchResponse,
        OpenSessionRequest, OpenSessionResponse, ProbeRecord, ProxyNode, RefreshRequest,
        RefreshResponse, SessionRecord, now_epoch_sec,
    },
    runtime::MihomoRuntime,
    store::BrokerStore,
    subscription,
};

#[derive(Debug, Clone)]
pub struct BrokerServiceOptions {
    pub probe_targets: Vec<String>,
    pub probe_timeout_ms: u64,
    pub probe_concurrency: usize,
    pub probe_ttl_sec: u64,
    pub geo_online_concurrency: usize,
    pub geo_ttl_sec: u64,
    pub online_geo_base: String,
    pub mmdb_url: String,
    pub data_dir: PathBuf,
    pub session_listen_ip: IpAddr,
}

impl Default for BrokerServiceOptions {
    fn default() -> Self {
        Self {
            probe_targets: DEFAULT_PROBE_TARGETS.map(ToString::to_string).to_vec(),
            probe_timeout_ms: DEFAULT_PROBE_TIMEOUT_MS,
            probe_concurrency: DEFAULT_PROBE_CONCURRENCY,
            probe_ttl_sec: DEFAULT_PROBE_TTL_SEC,
            geo_online_concurrency: DEFAULT_GEO_ONLINE_CONCURRENCY,
            geo_ttl_sec: DEFAULT_GEO_TTL_SEC,
            online_geo_base: DEFAULT_ONLINE_GEO_BASE.to_string(),
            mmdb_url: DEFAULT_MMDB_URL.to_string(),
            data_dir: PathBuf::from(".proxy-broker/data"),
            session_listen_ip: DEFAULT_SESSION_LISTEN_IP
                .parse()
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        }
    }
}

#[derive(Clone)]
pub struct BrokerService {
    store: Arc<dyn BrokerStore>,
    runtime: Arc<dyn MihomoRuntime>,
    http: reqwest::Client,
    options: BrokerServiceOptions,
    profile_locks: Vec<Arc<TokioMutex<()>>>,
}

impl BrokerService {
    pub fn new(
        store: Arc<dyn BrokerStore>,
        runtime: Arc<dyn MihomoRuntime>,
        options: BrokerServiceOptions,
    ) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            store,
            runtime,
            http,
            options,
            profile_locks: (0..64).map(|_| Arc::new(TokioMutex::new(()))).collect(),
        }
    }

    fn profile_lock_index(&self, profile_id: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        profile_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.profile_locks.len()
    }

    async fn lock_profile(&self, profile_id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self.profile_locks[self.profile_lock_index(profile_id)].clone();
        lock.lock_owned().await
    }

    async fn profile_exists(&self, profile_id: &str) -> BrokerResult<bool> {
        let profiles = self
            .store
            .list_profiles()
            .await
            .map_err(BrokerError::from)?;
        Ok(profiles.into_iter().any(|item| item == profile_id))
    }

    async fn cleanup_profile_runtime_if_idle(&self, profile_id: &str, sessions: &[SessionRecord]) {
        if !sessions.is_empty() {
            return;
        }

        if let Err(err) = self.runtime.shutdown_profile(profile_id).await {
            tracing::warn!(
                profile_id,
                error = %err,
                "failed to shutdown idle profile runtime"
            );
        }
    }

    pub async fn reconcile_startup_sessions(&self) -> BrokerResult<()> {
        let profiles = self
            .store
            .list_profiles()
            .await
            .map_err(BrokerError::from)?;
        for profile_id in profiles {
            if let Err(err) = self.reconcile_profile_sessions(&profile_id).await {
                tracing::warn!(
                    profile_id,
                    error = %err,
                    "startup session reconciliation failed"
                );
            }
        }
        Ok(())
    }

    async fn reconcile_profile_sessions(&self, profile_id: &str) -> BrokerResult<()> {
        let _profile_guard = self.lock_profile(profile_id).await;

        let nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let existing_sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        if existing_sessions.is_empty() {
            self.cleanup_profile_runtime_if_idle(profile_id, &existing_sessions)
                .await;
            return Ok(());
        }

        if nodes.is_empty() {
            self.runtime
                .shutdown_profile(profile_id)
                .await
                .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
            for session in &existing_sessions {
                self.store
                    .delete_session(profile_id, &session.session_id)
                    .await
                    .map_err(BrokerError::from)?;
            }
            return Ok(());
        }

        let valid_proxy_ip_pairs: HashSet<(String, String)> = nodes
            .iter()
            .flat_map(|node| {
                node.resolved_ips
                    .iter()
                    .map(move |ip| (node.proxy_name.clone(), ip.clone()))
            })
            .collect();

        let reconciled_sessions: Vec<SessionRecord> = existing_sessions
            .iter()
            .filter(|session| {
                valid_proxy_ip_pairs
                    .contains(&(session.proxy_name.clone(), session.selected_ip.clone()))
            })
            .cloned()
            .collect();

        let reconciled_ids: HashSet<&str> = reconciled_sessions
            .iter()
            .map(|session| session.session_id.as_str())
            .collect();
        let stale_ids: Vec<String> = existing_sessions
            .iter()
            .filter(|session| !reconciled_ids.contains(session.session_id.as_str()))
            .map(|session| session.session_id.clone())
            .collect();

        if reconciled_sessions.is_empty() {
            self.runtime
                .shutdown_profile(profile_id)
                .await
                .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
            for session_id in stale_ids {
                self.store
                    .delete_session(profile_id, &session_id)
                    .await
                    .map_err(BrokerError::from)?;
            }
            return Ok(());
        }

        self.runtime
            .ensure_started(profile_id)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
        self.apply_sessions_config(profile_id, &nodes, &reconciled_sessions)
            .await?;
        for session_id in stale_ids {
            self.store
                .delete_session(profile_id, &session_id)
                .await
                .map_err(BrokerError::from)?;
        }

        Ok(())
    }

    pub async fn load_subscription(
        &self,
        profile_id: &str,
        source: &crate::models::SubscriptionSource,
    ) -> BrokerResult<LoadSubscriptionResponse> {
        let _profile_guard = self.lock_profile(profile_id).await;

        let (mut nodes, mut warnings) = subscription::load_from_source(&self.http, source)
            .await
            .map_err(|err| match err {
                subscription::SubscriptionLoadError::SourceRead(message) => {
                    BrokerError::SubscriptionFetch(message)
                }
                subscription::SubscriptionLoadError::InvalidPayload(_) => {
                    BrokerError::SubscriptionInvalid
                }
            })?;

        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }
        if has_duplicate_proxy_names(&nodes) {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let existing_nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let existing_ips_by_proxy: HashMap<(String, String), Vec<String>> = existing_nodes
            .iter()
            .filter_map(|node| {
                if node.resolved_ips.is_empty() {
                    None
                } else {
                    Some((
                        (node.proxy_name.clone(), node.server.clone()),
                        node.resolved_ips.clone(),
                    ))
                }
            })
            .collect();
        for node in &mut nodes {
            if !node.resolved_ips.is_empty() {
                continue;
            }
            if let Some(cached_ips) =
                existing_ips_by_proxy.get(&(node.proxy_name.clone(), node.server.clone()))
            {
                node.resolved_ips = cached_ips.clone();
                warnings.push(format!(
                    "proxy `{}` dns resolve failed, reused {} cached ip(s)",
                    node.proxy_name,
                    node.resolved_ips.len()
                ));
            }
        }

        let existing_ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let existing_ip_map: HashMap<String, IpRecord> = existing_ip_records
            .into_iter()
            .map(|record| (record.ip.clone(), record))
            .collect();

        let mut ip_map: HashMap<String, IpRecord> = HashMap::new();
        for node in &nodes {
            for ip in &node.resolved_ips {
                ip_map.entry(ip.clone()).or_insert_with(|| {
                    if let Some(existing) = existing_ip_map.get(ip) {
                        existing.clone()
                    } else {
                        IpRecord {
                            ip: ip.clone(),
                            country_code: None,
                            country_name: None,
                            region_name: None,
                            city: None,
                            geo_source: None,
                            probe_updated_at: None,
                            geo_updated_at: None,
                            last_used_at: None,
                        }
                    }
                });
            }
        }
        let valid_ips: HashSet<String> = ip_map.keys().cloned().collect();
        if valid_ips.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }
        let valid_proxy_ip_pairs: HashSet<(String, String)> = nodes
            .iter()
            .flat_map(|node| {
                node.resolved_ips
                    .iter()
                    .map(move |ip| (node.proxy_name.clone(), ip.clone()))
            })
            .collect();
        let existing_sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let active_sessions: Vec<SessionRecord> = existing_sessions
            .iter()
            .filter(|session| {
                valid_proxy_ip_pairs
                    .contains(&(session.proxy_name.clone(), session.selected_ip.clone()))
            })
            .cloned()
            .collect();
        let stale_session_ids: Vec<String> = existing_sessions
            .iter()
            .filter(|session| {
                !valid_proxy_ip_pairs
                    .contains(&(session.proxy_name.clone(), session.selected_ip.clone()))
            })
            .map(|session| session.session_id.clone())
            .collect();
        let fresh_probe_records = filter_probe_records_by_pair(
            self.store
                .list_probe_records(profile_id)
                .await
                .map_err(BrokerError::from)?,
            &valid_proxy_ip_pairs,
        );
        let mut next_ip_records = ip_map.values().cloned().collect::<Vec<_>>();
        clear_stale_probe_timestamps(&mut next_ip_records, &fresh_probe_records);

        let runtime_applied = !active_sessions.is_empty();
        if runtime_applied {
            self.apply_sessions_config(profile_id, &nodes, &active_sessions)
                .await?;
        }

        if let Err(err) = self
            .store
            .apply_subscription_snapshot(
                profile_id,
                &nodes,
                &next_ip_records,
                &fresh_probe_records,
                &stale_session_ids,
            )
            .await
            .map_err(BrokerError::from)
        {
            if runtime_applied
                && let Err(rollback_err) = self
                    .rollback_runtime_sessions(profile_id, &existing_nodes, &existing_sessions)
                    .await
            {
                tracing::error!(
                    profile_id,
                    error = %rollback_err,
                    "runtime rollback failed after subscription snapshot persistence error"
                );
                self.recover_runtime_desync(profile_id, &existing_nodes, &existing_sessions)
                    .await;
            }
            return Err(err);
        }

        self.cleanup_profile_runtime_if_idle(profile_id, &active_sessions)
            .await;

        let distinct_ips = valid_ips.len();

        Ok(LoadSubscriptionResponse {
            loaded_proxies: nodes.len(),
            distinct_ips,
            warnings,
        })
    }

    pub async fn refresh(
        &self,
        profile_id: &str,
        request: &RefreshRequest,
    ) -> BrokerResult<RefreshResponse> {
        let _profile_guard = self.lock_profile(profile_id).await;

        let nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let mut ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let stored_probe_records = self
            .store
            .list_probe_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_cache_complete =
            has_complete_probe_records(&nodes, &self.options.probe_targets, &stored_probe_records);

        let now = now_epoch_sec();
        let should_probe = request.force
            || !probe_cache_complete
            || ip_records.iter().any(|r| {
                r.probe_updated_at
                    .map(|ts| ts + (self.options.probe_ttl_sec as i64) < now)
                    .unwrap_or(true)
            });

        let mut probe_records = if should_probe {
            self.runtime
                .ensure_started(profile_id)
                .await
                .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
            let sessions = self
                .store
                .list_sessions(profile_id)
                .await
                .map_err(BrokerError::from)?;
            self.apply_sessions_config(profile_id, &nodes, &sessions)
                .await?;
            self.refresh_probe_records(profile_id, now, &nodes).await?
        } else {
            stored_probe_records
        };

        if should_probe {
            for record in &mut ip_records {
                if probe_records.iter().any(|p| p.ip == record.ip) {
                    record.probe_updated_at = Some(now);
                }
            }
            self.store
                .upsert_probe_records(profile_id, &probe_records)
                .await
                .map_err(BrokerError::from)?;
        }

        let geo_updated = self
            .refresh_geo_records(profile_id, request.force, now, &mut ip_records)
            .await?;
        self.store
            .upsert_ip_records(profile_id, &ip_records)
            .await
            .map_err(BrokerError::from)?;

        if !should_probe {
            probe_records = self
                .store
                .list_probe_records(profile_id)
                .await
                .map_err(BrokerError::from)?;
        }

        let sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        self.cleanup_profile_runtime_if_idle(profile_id, &sessions)
            .await;

        let probed_ips: HashSet<String> = probe_records.into_iter().map(|r| r.ip).collect();

        Ok(RefreshResponse {
            probed_ips: probed_ips.len(),
            geo_updated,
            skipped_cached: if should_probe { 0 } else { ip_records.len() },
        })
    }

    async fn refresh_probe_records(
        &self,
        profile_id: &str,
        now: i64,
        nodes: &[ProxyNode],
    ) -> BrokerResult<Vec<ProbeRecord>> {
        let mut tasks = Vec::new();
        for node in nodes {
            for ip in &node.resolved_ips {
                let probe_proxy_name = dedicated_ip_proxy_name(&node.proxy_name, ip);
                for target in &self.options.probe_targets {
                    tasks.push((
                        node.proxy_name.clone(),
                        ip.clone(),
                        target.clone(),
                        probe_proxy_name.clone(),
                    ));
                }
            }
        }

        let profile_id = profile_id.to_string();
        let timeout_ms = self.options.probe_timeout_ms;
        let concurrency = self.options.probe_concurrency.max(1);

        let probed: Vec<((String, String, String), ProbeRecord)> = stream::iter(tasks)
            .map(|(proxy_name, ip, target, probe_proxy_name)| {
                let profile_id = profile_id.clone();
                async move {
                    let delay = self
                        .runtime
                        .measure_proxy_delay(&profile_id, &probe_proxy_name, &target, timeout_ms)
                        .await
                        .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
                    let key = (proxy_name.clone(), ip.clone(), target.clone());
                    let record = ProbeRecord {
                        proxy_name,
                        ip,
                        target_url: target,
                        ok: delay.is_some(),
                        latency_ms: delay,
                        updated_at: now,
                    };
                    Ok::<_, BrokerError>((key, record))
                }
            })
            .buffer_unordered(concurrency)
            .try_collect()
            .await?;

        let mut by_key: HashMap<(String, String, String), ProbeRecord> = HashMap::new();
        for (key, candidate) in probed {
            match by_key.get(&key) {
                Some(existing) => {
                    if is_better_probe(&candidate, existing) {
                        by_key.insert(key, candidate);
                    }
                }
                None => {
                    by_key.insert(key, candidate);
                }
            }
        }

        Ok(by_key.into_values().collect())
    }

    async fn ensure_mmdb_file(&self) -> anyhow::Result<PathBuf> {
        let geo_dir = self.options.data_dir.join("geo");
        tokio::fs::create_dir_all(&geo_dir)
            .await
            .with_context(|| format!("failed to create geo dir: {}", geo_dir.display()))?;
        let mmdb = geo_dir.join("country.mmdb");
        if mmdb.exists() {
            if Reader::open_readfile(&mmdb).is_ok() {
                return Ok(mmdb);
            }
            tracing::warn!(
                path = %mmdb.display(),
                "existing mmdb file is invalid, redownloading"
            );
            let _ = tokio::fs::remove_file(&mmdb).await;
        }

        let bytes = self
            .http
            .get(&self.options.mmdb_url)
            .send()
            .await
            .context("failed to download mmdb")?
            .error_for_status()
            .context("mmdb download status is not success")?
            .bytes()
            .await
            .context("failed to read mmdb body")?;

        let temp_file = geo_dir.join(format!("country.mmdb.tmp-{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_file, bytes)
            .await
            .with_context(|| format!("failed to write temp mmdb: {}", temp_file.display()))?;
        if Reader::open_readfile(&temp_file).is_err() {
            let _ = tokio::fs::remove_file(&temp_file).await;
            return Err(anyhow!(
                "downloaded mmdb file is invalid: {}",
                temp_file.display()
            ));
        }
        tokio::fs::rename(&temp_file, &mmdb)
            .await
            .with_context(|| {
                format!(
                    "failed to atomically replace mmdb {} -> {}",
                    temp_file.display(),
                    mmdb.display()
                )
            })?;
        Ok(mmdb)
    }

    async fn refresh_geo_records(
        &self,
        _profile_id: &str,
        force: bool,
        now: i64,
        ip_records: &mut [IpRecord],
    ) -> BrokerResult<usize> {
        let mmdb_path = self.ensure_mmdb_file().await.ok();
        let mmdb_reader = if let Some(path) = mmdb_path {
            Reader::open_readfile(path).ok()
        } else {
            None
        };

        let candidate_ips: HashSet<String> = ip_records
            .iter()
            .filter_map(|record| {
                let stale = record
                    .geo_updated_at
                    .map(|ts| ts + (self.options.geo_ttl_sec as i64) < now)
                    .unwrap_or(true);
                if !force && !stale {
                    return None;
                }
                IpAddr::from_str(&record.ip).ok()?;
                Some(record.ip.clone())
            })
            .collect();
        let online_lookup = self.lookup_online_geo_batch(candidate_ips).await;

        let mut changed = 0usize;
        for record in ip_records.iter_mut() {
            let stale = record
                .geo_updated_at
                .map(|ts| ts + (self.options.geo_ttl_sec as i64) < now)
                .unwrap_or(true);
            if !force && !stale {
                continue;
            }

            let ip = match IpAddr::from_str(&record.ip) {
                Ok(ip) => ip,
                Err(_) => continue,
            };

            let mut country_code = record.country_code.clone();
            let mut country_name = record.country_name.clone();
            let mut region_name = record.region_name.clone();
            let mut city = record.city.clone();
            let mut source = None;
            let mut mmdb_hit = false;
            let mut online_hit = false;
            let mut mmdb_lookup_succeeded = false;
            let online_state = online_lookup
                .get(&record.ip)
                .cloned()
                .unwrap_or_else(OnlineGeoLookupState::default);

            if let Some(reader) = &mmdb_reader
                && let Ok(country) = reader.lookup::<geoip2::Country<'_>>(ip)
            {
                mmdb_lookup_succeeded = true;
                if let Some(country) = country
                    && let Some(info) = country.country
                {
                    let mmdb_country_code = info.iso_code.map(ToString::to_string);
                    let mmdb_country_name =
                        info.names.and_then(|m| m.get("en").map(|x| x.to_string()));
                    if mmdb_country_code.is_some() {
                        country_code = mmdb_country_code.clone();
                    }
                    if mmdb_country_name.is_some() {
                        country_name = mmdb_country_name.clone();
                    }
                    mmdb_hit = mmdb_country_code.is_some() || mmdb_country_name.is_some();
                }
                if mmdb_hit {
                    source = Some("mmdb".to_string());
                }
            }

            if let Some(online) = online_state.result {
                let online_has_geo = online.country_code.is_some()
                    || online.country.is_some()
                    || online.region.is_some()
                    || online.city.is_some();
                if let Some(value) = online.country_code {
                    country_code = Some(value);
                }
                if let Some(value) = online.country {
                    country_name = Some(value);
                }
                if let Some(value) = online.region {
                    region_name = Some(value);
                }
                if let Some(value) = online.city {
                    city = Some(value);
                }
                if online_has_geo {
                    online_hit = true;
                    source = Some(match source {
                        Some(_) => "mmdb+online".to_string(),
                        None => "online".to_string(),
                    });
                }
            }

            if !(mmdb_hit || online_hit) {
                if mmdb_lookup_succeeded || online_state.lookup_succeeded {
                    record.country_code = None;
                    record.country_name = None;
                    record.region_name = None;
                    record.city = None;
                    record.geo_source = Some("none".to_string());
                    record.geo_updated_at = Some(now);
                    changed += 1;
                }
                continue;
            }

            record.country_code = country_code;
            record.country_name = country_name;
            record.region_name = region_name;
            record.city = city;
            record.geo_source = source;
            record.geo_updated_at = Some(now);
            changed += 1;
        }

        Ok(changed)
    }

    async fn lookup_online_geo(&self, ip: &str) -> anyhow::Result<OnlineGeoResult> {
        #[derive(Debug, Deserialize)]
        struct OnlineGeoResp {
            success: Option<bool>,
            country_code: Option<String>,
            country: Option<String>,
            region: Option<String>,
            city: Option<String>,
        }

        let url = format!(
            "{}/{}",
            self.options.online_geo_base.trim_end_matches('/'),
            ip
        );
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("online geo request failed")?
            .error_for_status()
            .context("online geo status is non-success")?
            .json::<OnlineGeoResp>()
            .await
            .context("failed to decode online geo response")?;

        if response.success == Some(false) {
            return Err(anyhow!("online geo lookup unsuccessful"));
        }

        Ok(OnlineGeoResult {
            country_code: response.country_code,
            country: response.country,
            region: response.region,
            city: response.city,
        })
    }

    async fn lookup_online_geo_batch(
        &self,
        ips: HashSet<String>,
    ) -> HashMap<String, OnlineGeoLookupState> {
        if ips.is_empty() {
            return HashMap::new();
        }

        let concurrency = self.options.geo_online_concurrency.max(1);
        stream::iter(ips)
            .map(|ip| async move {
                let state = match self.lookup_online_geo(&ip).await {
                    Ok(result) => OnlineGeoLookupState {
                        result: Some(result),
                        lookup_succeeded: true,
                    },
                    Err(err) => {
                        tracing::debug!(ip = %ip, error = %err, "online geo lookup failed");
                        OnlineGeoLookupState::default()
                    }
                };
                (ip, state)
            })
            .buffer_unordered(concurrency)
            .collect()
            .await
    }

    async fn apply_sessions_config(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
        sessions: &[SessionRecord],
    ) -> BrokerResult<()> {
        let (controller, secret) = self
            .runtime
            .controller_meta(profile_id)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
        let payload = render_payload(&controller, secret.as_deref(), nodes, sessions)
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
        self.runtime
            .apply_config(profile_id, &payload)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))
    }

    async fn rollback_runtime_sessions(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
        sessions: &[SessionRecord],
    ) -> anyhow::Result<()> {
        self.apply_sessions_config(profile_id, nodes, sessions)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn recover_runtime_desync(
        &self,
        profile_id: &str,
        nodes: &[ProxyNode],
        sessions: &[SessionRecord],
    ) {
        tracing::warn!(
            profile_id,
            "attempting runtime recovery after rollback failure"
        );
        if let Err(err) = self.runtime.shutdown_profile(profile_id).await {
            tracing::warn!(
                profile_id,
                error = %err,
                "failed to shutdown runtime during rollback recovery"
            );
        }

        if sessions.is_empty() {
            return;
        }

        if let Err(err) = self
            .runtime
            .ensure_started(profile_id)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))
        {
            tracing::warn!(
                profile_id,
                error = %err,
                "failed to restart runtime during rollback recovery"
            );
            return;
        }

        if let Err(err) = self
            .apply_sessions_config(profile_id, nodes, sessions)
            .await
        {
            tracing::warn!(
                profile_id,
                error = %err,
                "failed to reapply sessions during rollback recovery"
            );
        }
    }

    pub async fn extract_ips(
        &self,
        profile_id: &str,
        request: &ExtractIpRequest,
    ) -> BrokerResult<ExtractIpResponse> {
        validate_conflict(request)?;
        let _profile_guard = self.lock_profile(profile_id).await;

        let ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let mut items = filter_ip_records(ip_records, &probe_records, request)?;
        if let Some(limit) = request.limit {
            items.truncate(limit);
        }
        Ok(ExtractIpResponse { items })
    }

    pub async fn open_session(
        &self,
        profile_id: &str,
        request: &OpenSessionRequest,
    ) -> BrokerResult<OpenSessionResponse> {
        let _profile_guard = self.lock_profile(profile_id).await;

        let nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let existing = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let retryable = request.desired_port.is_none();
        let max_attempts = if retryable { 3usize } else { 1usize };

        for attempt in 1..=max_attempts {
            let prepared = match prepare_session(
                request,
                &nodes,
                &ip_records,
                &probe_records,
                &existing,
                self.options.session_listen_ip,
            ) {
                Ok(prepared) => prepared,
                Err(err)
                    if retryable
                        && attempt < max_attempts
                        && matches!(&err, BrokerError::PortInUse) =>
                {
                    continue;
                }
                Err(err) => return Err(err),
            };

            let mut merged = existing.clone();
            merged.push(prepared.clone());

            if let Err(err) = self
                .apply_sessions_config(profile_id, &nodes, &merged)
                .await
            {
                tracing::warn!(
                    profile_id,
                    attempt,
                    error = %err,
                    "session apply config failed"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions(profile_id, &nodes, &existing)
                    .await
                {
                    tracing::error!(
                        profile_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after session apply failure"
                    );
                    self.recover_runtime_desync(profile_id, &nodes, &existing)
                        .await;
                }
                if retryable && attempt < max_attempts {
                    continue;
                }
                return Err(err);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(profile_id, std::slice::from_ref(&prepared), now)
                .await
            {
                tracing::error!(
                    profile_id,
                    session_id = %prepared.session_id,
                    error = %err,
                    "persist session failed after runtime apply, rolling back runtime"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions(profile_id, &nodes, &existing)
                    .await
                {
                    tracing::error!(
                        profile_id,
                        session_id = %prepared.session_id,
                        error = %rollback_err,
                        "runtime rollback failed after session insert failure"
                    );
                    self.recover_runtime_desync(profile_id, &nodes, &existing)
                        .await;
                }
                return Err(BrokerError::from(err));
            }

            return Ok(OpenSessionResponse {
                session_id: prepared.session_id,
                listen: prepared.listen,
                port: prepared.port,
                selected_ip: prepared.selected_ip,
                proxy_name: prepared.proxy_name,
            });
        }

        Err(BrokerError::PortInUse)
    }

    pub async fn open_batch(
        &self,
        profile_id: &str,
        request: &OpenBatchRequest,
    ) -> BrokerResult<OpenBatchResponse> {
        if request.requests.is_empty() {
            return Ok(OpenBatchResponse { sessions: vec![] });
        }

        let _profile_guard = self.lock_profile(profile_id).await;

        let nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let existing = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let retryable = request.requests.iter().all(|r| r.desired_port.is_none());
        let max_attempts = if retryable { 3usize } else { 1usize };

        for attempt in 1..=max_attempts {
            let staged = match stage_batch_sessions(
                &request.requests,
                &nodes,
                &ip_records,
                &probe_records,
                &existing,
                self.options.session_listen_ip,
            ) {
                Ok(staged) => staged,
                Err(err)
                    if retryable
                        && attempt < max_attempts
                        && matches!(&err, BrokerError::PortInUse) =>
                {
                    continue;
                }
                Err(err) => return Err(err),
            };

            let mut merged = existing.clone();
            merged.extend(staged.clone());

            if let Err(err) = self
                .apply_sessions_config(profile_id, &nodes, &merged)
                .await
            {
                tracing::warn!(
                    profile_id,
                    attempt,
                    error = %err,
                    "batch apply config failed before persisting sessions"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions(profile_id, &nodes, &existing)
                    .await
                {
                    tracing::error!(
                        profile_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after batch apply failure"
                    );
                    self.recover_runtime_desync(profile_id, &nodes, &existing)
                        .await;
                }
                if retryable && attempt < max_attempts {
                    continue;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(profile_id, &staged, now)
                .await
            {
                tracing::error!(
                    profile_id,
                    error = %err,
                    "batch persist failed after runtime apply, rolling back"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions(profile_id, &nodes, &existing)
                    .await
                {
                    tracing::error!(
                        profile_id,
                        error = %rollback_err,
                        "runtime rollback failed after batch persist failure"
                    );
                    self.recover_runtime_desync(profile_id, &nodes, &existing)
                        .await;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            return Ok(OpenBatchResponse {
                sessions: staged
                    .into_iter()
                    .map(|s| OpenSessionResponse {
                        session_id: s.session_id,
                        listen: s.listen,
                        port: s.port,
                        selected_ip: s.selected_ip,
                        proxy_name: s.proxy_name,
                    })
                    .collect(),
            });
        }

        Err(BrokerError::BatchOpenFailed)
    }

    pub async fn list_profiles(&self) -> BrokerResult<ListProfilesResponse> {
        let profiles = self
            .store
            .list_profiles()
            .await
            .map_err(BrokerError::from)?;
        Ok(ListProfilesResponse { profiles })
    }

    pub async fn create_profile(&self, profile_id: &str) -> BrokerResult<CreateProfileResponse> {
        let normalized = profile_id.trim();
        if normalized.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "profile_id must not be empty".to_string(),
            ));
        }

        let _profile_guard = self.lock_profile(normalized).await;
        let exists = self
            .store
            .list_profiles()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .any(|item| item == normalized);
        if exists {
            return Err(BrokerError::ProfileExists);
        }

        self.store
            .create_profile(normalized, now_epoch_sec())
            .await
            .map_err(BrokerError::from)?;

        Ok(CreateProfileResponse {
            profile_id: normalized.to_string(),
        })
    }

    pub async fn list_api_keys(&self, profile_id: &str) -> BrokerResult<ListApiKeysResponse> {
        if !self.profile_exists(profile_id).await? {
            return Err(BrokerError::ProfileNotFound);
        }

        let api_keys = self
            .store
            .list_api_keys(profile_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| record.as_summary())
            .collect();

        Ok(ListApiKeysResponse { api_keys })
    }

    pub async fn create_api_key(
        &self,
        profile_id: &str,
        request: &CreateApiKeyRequest,
        created_by_subject: &str,
    ) -> BrokerResult<CreateApiKeyResponse> {
        if !self.profile_exists(profile_id).await? {
            return Err(BrokerError::ProfileNotFound);
        }

        let name = request.name.trim();
        if name.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "api key name must not be empty".to_string(),
            ));
        }

        let issued = issue_api_key(profile_id, name, created_by_subject);
        self.store
            .insert_api_key(&issued.record)
            .await
            .map_err(BrokerError::from)?;
        Ok(issued.into_response())
    }

    pub async fn revoke_api_key(&self, profile_id: &str, key_id: &str) -> BrokerResult<()> {
        if !self.profile_exists(profile_id).await? {
            return Err(BrokerError::ProfileNotFound);
        }

        let revoked = self
            .store
            .revoke_api_key(profile_id, key_id, now_epoch_sec())
            .await
            .map_err(BrokerError::from)?;
        if revoked {
            Ok(())
        } else {
            Err(BrokerError::ApiKeyNotFound)
        }
    }

    pub async fn authenticate_api_key(&self, secret: &str) -> BrokerResult<Principal> {
        let (key_id, normalized_secret) = parse_api_key_secret(secret)?;
        let api_key = self
            .store
            .get_api_key(key_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ApiKeyInvalid)?;

        if api_key.revoked_at.is_some() {
            return Err(BrokerError::ApiKeyRevoked);
        }

        let computed_hash = hash_secret(&api_key.secret_salt, normalized_secret);
        if !constant_time_eq(&computed_hash, &api_key.secret_hash) {
            return Err(BrokerError::ApiKeyInvalid);
        }

        self.store
            .touch_api_key_last_used(&api_key.key_id, now_epoch_sec())
            .await
            .map_err(BrokerError::from)?;

        Ok(Principal::api_key(api_key.profile_id, api_key.key_id))
    }

    pub async fn list_sessions(&self, profile_id: &str) -> BrokerResult<ListSessionsResponse> {
        let sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        Ok(ListSessionsResponse { sessions })
    }

    pub async fn close_session(&self, profile_id: &str, session_id: &str) -> BrokerResult<()> {
        let _profile_guard = self.lock_profile(profile_id).await;

        let nodes = self
            .store
            .list_subscription(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let mut sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let previous_sessions = sessions.clone();

        let old_len = sessions.len();
        sessions.retain(|s| s.session_id != session_id);
        if sessions.len() == old_len {
            return Err(BrokerError::SessionNotFound);
        }

        if sessions.is_empty() {
            self.store
                .delete_session(profile_id, session_id)
                .await
                .map_err(BrokerError::from)?;
            self.cleanup_profile_runtime_if_idle(profile_id, &sessions)
                .await;
            return Ok(());
        }

        self.apply_sessions_config(profile_id, &nodes, &sessions)
            .await?;

        if let Err(err) = self.store.delete_session(profile_id, session_id).await {
            tracing::error!(
                profile_id,
                session_id,
                error = %err,
                "persist close-session failed, rolling back runtime"
            );
            if let Err(rollback_err) = self
                .rollback_runtime_sessions(profile_id, &nodes, &previous_sessions)
                .await
            {
                tracing::error!(
                    profile_id,
                    session_id,
                    error = %rollback_err,
                    "runtime rollback failed after close-session persistence error"
                );
                self.recover_runtime_desync(profile_id, &nodes, &previous_sessions)
                    .await;
            }
            return Err(BrokerError::from(err));
        }

        self.cleanup_profile_runtime_if_idle(profile_id, &sessions)
            .await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct OnlineGeoResult {
    country_code: Option<String>,
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct OnlineGeoLookupState {
    result: Option<OnlineGeoResult>,
    lookup_succeeded: bool,
}

fn normalize_ip_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    IpAddr::from_str(trimmed)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| trimmed.to_string())
}

fn validate_conflict(request: &ExtractIpRequest) -> BrokerResult<()> {
    let specified: HashSet<String> = request
        .specified_ips
        .iter()
        .map(|ip| normalize_ip_text(ip))
        .filter(|ip| !ip.is_empty())
        .collect();
    let blacklist: HashSet<String> = request
        .blacklist_ips
        .iter()
        .map(|ip| normalize_ip_text(ip))
        .filter(|ip| !ip.is_empty())
        .collect();

    let conflicts: Vec<String> = specified.intersection(&blacklist).cloned().collect();
    if !conflicts.is_empty() {
        return Err(BrokerError::IpConflictBlacklist(conflicts));
    }
    Ok(())
}

fn filter_probe_records_by_pair(
    probe_records: Vec<ProbeRecord>,
    valid_proxy_ip_pairs: &HashSet<(String, String)>,
) -> Vec<ProbeRecord> {
    probe_records
        .into_iter()
        .filter(|record| {
            valid_proxy_ip_pairs.contains(&(record.proxy_name.clone(), record.ip.clone()))
        })
        .collect()
}

fn clear_stale_probe_timestamps(ip_records: &mut [IpRecord], probe_records: &[ProbeRecord]) {
    let valid_probe_ips: HashSet<&str> = probe_records
        .iter()
        .map(|record| record.ip.as_str())
        .collect();
    for record in ip_records {
        if !valid_probe_ips.contains(record.ip.as_str()) {
            record.probe_updated_at = None;
        }
    }
}

fn expected_probe_keys(
    nodes: &[ProxyNode],
    probe_targets: &[String],
) -> HashSet<(String, String, String)> {
    nodes
        .iter()
        .flat_map(|node| {
            node.resolved_ips.iter().flat_map(move |ip| {
                probe_targets
                    .iter()
                    .map(move |target| (node.proxy_name.clone(), ip.clone(), target.clone()))
            })
        })
        .collect()
}

fn has_complete_probe_records(
    nodes: &[ProxyNode],
    probe_targets: &[String],
    probe_records: &[ProbeRecord],
) -> bool {
    let expected = expected_probe_keys(nodes, probe_targets);
    if expected.is_empty() {
        return true;
    }
    let actual: HashSet<(String, String, String)> = probe_records
        .iter()
        .map(|record| {
            (
                record.proxy_name.clone(),
                record.ip.clone(),
                record.target_url.clone(),
            )
        })
        .collect();
    expected.is_subset(&actual)
}

fn probe_summary(probes: &[ProbeRecord]) -> HashMap<String, (bool, Option<u64>)> {
    let mut map: HashMap<String, (bool, Option<u64>)> = HashMap::new();
    for probe in probes {
        let entry = map.entry(probe.ip.clone()).or_insert((false, None));
        if probe.ok {
            entry.0 = true;
            match (entry.1, probe.latency_ms) {
                (Some(current), Some(new_val)) if new_val < current => entry.1 = Some(new_val),
                (None, Some(new_val)) => entry.1 = Some(new_val),
                _ => {}
            }
        }
    }
    map
}

fn filter_ip_records(
    ip_records: Vec<IpRecord>,
    probe_records: &[ProbeRecord],
    request: &ExtractIpRequest,
) -> BrokerResult<Vec<ExtractIpItem>> {
    validate_conflict(request)?;

    let mut items = Vec::new();
    let probe_index = probe_summary(probe_records);

    let specified: HashSet<String> = request
        .specified_ips
        .iter()
        .map(|s| normalize_ip_text(s))
        .filter(|s| !s.is_empty())
        .collect();
    let blacklist: HashSet<String> = request
        .blacklist_ips
        .iter()
        .map(|s| normalize_ip_text(s))
        .filter(|s| !s.is_empty())
        .collect();

    let country_set: HashSet<String> = request
        .country_codes
        .iter()
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let city_set: HashSet<String> = request
        .cities
        .iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();

    for record in ip_records {
        let record_ip_key = normalize_ip_text(&record.ip);

        if blacklist.contains(&record_ip_key) {
            continue;
        }

        let include = if !specified.is_empty() {
            specified.contains(&record_ip_key)
        } else {
            let country_pass = if country_set.is_empty() {
                true
            } else {
                record
                    .country_code
                    .as_ref()
                    .map(|c| country_set.contains(&c.to_ascii_uppercase()))
                    .unwrap_or(false)
            };
            let city_pass = if city_set.is_empty() {
                true
            } else {
                record
                    .city
                    .as_ref()
                    .map(|c| city_set.contains(&c.to_ascii_lowercase()))
                    .unwrap_or(false)
            };
            country_pass && city_pass
        };

        if !include {
            continue;
        }

        let (probe_ok, best_latency_ms) = probe_index
            .get(&record.ip)
            .cloned()
            .unwrap_or((false, None));

        items.push(ExtractIpItem {
            ip: record.ip,
            country_code: record.country_code,
            country_name: record.country_name,
            region_name: record.region_name,
            city: record.city,
            probe_ok,
            best_latency_ms,
            last_used_at: record.last_used_at,
        });
    }

    match request.sort_mode {
        crate::models::SortMode::Mru => {
            items.sort_by(|a, b| {
                b.last_used_at
                    .cmp(&a.last_used_at)
                    .then_with(|| a.ip.cmp(&b.ip))
            });
        }
        crate::models::SortMode::Lru => {
            items.sort_by(|a, b| {
                a.last_used_at
                    .cmp(&b.last_used_at)
                    .then_with(|| a.ip.cmp(&b.ip))
            });
        }
    }

    if items.is_empty() {
        return Err(BrokerError::IpNotFound);
    }

    Ok(items)
}

fn choose_ip_for_open(
    request: &OpenSessionRequest,
    ip_records: &[IpRecord],
    probes: &[ProbeRecord],
) -> BrokerResult<String> {
    let using_custom_selector = request.selector.is_some();

    if let Some(ip) = &request.specified_ip {
        let normalized_ip = normalize_ip_text(ip);
        if let Some(selector) = &request.selector {
            let mut req = selector.clone();
            req.specified_ips = vec![normalized_ip.clone()];
            let mut items = filter_ip_records(ip_records.to_vec(), probes, &req)?;
            if let Some(limit) = req.limit {
                items.truncate(limit);
            }
            if items.is_empty() {
                return Err(BrokerError::IpNotFound);
            }
            return Ok(items[0].ip.clone());
        }

        if let Some(record) = ip_records
            .iter()
            .find(|record| normalize_ip_text(&record.ip) == normalized_ip)
        {
            return Ok(record.ip.clone());
        }
        return Err(BrokerError::IpNotFound);
    }

    let selector = request.selector.clone().unwrap_or_default();
    validate_conflict(&selector)?;
    let mut items = filter_ip_records(ip_records.to_vec(), probes, &selector)?;
    if let Some(limit) = selector.limit {
        items.truncate(limit);
    }
    if items.is_empty() {
        return Err(BrokerError::IpNotFound);
    }

    if !using_custom_selector {
        // Default auto-pick policy: health first + low latency + LRU.
        items.sort_by(|a, b| {
            b.probe_ok
                .cmp(&a.probe_ok)
                .then_with(|| a.best_latency_ms.cmp(&b.best_latency_ms))
                .then_with(|| a.last_used_at.cmp(&b.last_used_at))
                .then_with(|| a.ip.cmp(&b.ip))
        });
    }

    items
        .first()
        .map(|i| i.ip.clone())
        .ok_or(BrokerError::IpNotFound)
}

fn choose_proxy_for_ip(
    ip: &str,
    nodes: &[ProxyNode],
    probes: &[ProbeRecord],
) -> BrokerResult<String> {
    let candidates: Vec<&ProxyNode> = nodes
        .iter()
        .filter(|node| node.resolved_ips.iter().any(|item| item == ip))
        .collect();
    if candidates.is_empty() {
        return Err(BrokerError::IpNotFound);
    }
    if candidates.len() == 1 {
        return Ok(candidates[0].proxy_name.clone());
    }

    let mut probe_by_proxy: HashMap<String, (bool, Option<u64>)> = HashMap::new();
    for probe in probes {
        if probe.ip != ip {
            continue;
        }
        let entry = probe_by_proxy
            .entry(probe.proxy_name.clone())
            .or_insert((false, None));
        if probe.ok {
            entry.0 = true;
            match (entry.1, probe.latency_ms) {
                (Some(current), Some(new_val)) if new_val < current => entry.1 = Some(new_val),
                (None, Some(new_val)) => entry.1 = Some(new_val),
                _ => {}
            }
        }
    }

    candidates
        .into_iter()
        .min_by(|a, b| {
            let a_probe = probe_by_proxy
                .get(&a.proxy_name)
                .cloned()
                .unwrap_or((false, None));
            let b_probe = probe_by_proxy
                .get(&b.proxy_name)
                .cloned()
                .unwrap_or((false, None));

            b_probe
                .0
                .cmp(&a_probe.0)
                .then_with(|| {
                    a_probe
                        .1
                        .unwrap_or(u64::MAX)
                        .cmp(&b_probe.1.unwrap_or(u64::MAX))
                })
                .then_with(|| a.proxy_name.cmp(&b.proxy_name))
        })
        .map(|node| node.proxy_name.clone())
        .ok_or(BrokerError::IpNotFound)
}

fn allocate_port(
    existing: &[SessionRecord],
    desired: Option<u16>,
    listen_ip: IpAddr,
) -> BrokerResult<u16> {
    let used: HashSet<u16> = existing.iter().map(|s| s.port).collect();
    if let Some(port) = desired {
        if port == 0 {
            return Err(BrokerError::InvalidPort);
        }
        if used.contains(&port) {
            return Err(BrokerError::PortInUse);
        }
        if std::net::TcpListener::bind((listen_ip, port)).is_err() {
            return Err(BrokerError::PortInUse);
        }
        return Ok(port);
    }

    for _ in 0..32 {
        let socket =
            std::net::TcpListener::bind((listen_ip, 0)).map_err(|_| BrokerError::PortInUse)?;
        let port = socket
            .local_addr()
            .map_err(|_| BrokerError::PortInUse)?
            .port();
        if !used.contains(&port) {
            return Ok(port);
        }
    }
    Err(BrokerError::PortInUse)
}

fn prepare_session(
    request: &OpenSessionRequest,
    nodes: &[ProxyNode],
    ip_records: &[IpRecord],
    probes: &[ProbeRecord],
    existing: &[SessionRecord],
    listen_ip: IpAddr,
) -> BrokerResult<SessionRecord> {
    let ip = choose_ip_for_open(request, ip_records, probes)?;
    let proxy_name = choose_proxy_for_ip(&ip, nodes, probes)?;
    let port = allocate_port(existing, request.desired_port, listen_ip)?;
    let now = now_epoch_sec();

    Ok(SessionRecord {
        session_id: uuid::Uuid::new_v4().to_string(),
        listen: listen_ip.to_string(),
        port,
        selected_ip: ip,
        proxy_name,
        created_at: now,
    })
}

fn stage_batch_sessions(
    requests: &[OpenSessionRequest],
    nodes: &[ProxyNode],
    ip_records: &[IpRecord],
    probe_records: &[ProbeRecord],
    existing: &[SessionRecord],
    listen_ip: IpAddr,
) -> BrokerResult<Vec<SessionRecord>> {
    let mut staged = Vec::new();
    for request in requests {
        let mut all_sessions = existing.to_vec();
        all_sessions.extend(staged.clone());
        let prepared = prepare_session(
            request,
            nodes,
            ip_records,
            probe_records,
            &all_sessions,
            listen_ip,
        )?;
        staged.push(prepared);
    }
    Ok(staged)
}

fn is_better_probe(candidate: &ProbeRecord, existing: &ProbeRecord) -> bool {
    match (candidate.ok, existing.ok) {
        (true, false) => true,
        (false, true) => false,
        _ => match (candidate.latency_ms, existing.latency_ms) {
            (Some(a), Some(b)) => a < b,
            (Some(_), None) => true,
            _ => false,
        },
    }
}

fn has_duplicate_proxy_names(nodes: &[ProxyNode]) -> bool {
    let mut seen = HashSet::new();
    for node in nodes {
        if !seen.insert(node.proxy_name.as_str()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{SortMode, SubscriptionSource},
        runtime::MihomoRuntime,
        store::{BrokerStore, MemoryStore},
    };
    use anyhow::anyhow;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct TestRuntime {
        fail_controller_meta: bool,
        fail_shutdown: bool,
        apply_calls: AtomicUsize,
        shutdown_calls: AtomicUsize,
    }

    impl TestRuntime {
        fn with_failures(fail_controller_meta: bool, fail_shutdown: bool) -> Self {
            Self {
                fail_controller_meta,
                fail_shutdown,
                apply_calls: AtomicUsize::new(0),
                shutdown_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl MihomoRuntime for TestRuntime {
        async fn ensure_started(&self, _profile_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_profile(&self, _profile_id: &str) -> anyhow::Result<()> {
            self.shutdown_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_shutdown {
                return Err(anyhow!("shutdown unavailable"));
            }
            Ok(())
        }

        async fn controller_meta(
            &self,
            _profile_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            if self.fail_controller_meta {
                return Err(anyhow!("controller unavailable"));
            }
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, profile_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(profile_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _profile_id: &str, _payload: &str) -> anyhow::Result<()> {
            self.apply_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_controller_meta {
                return Err(anyhow!("apply unavailable"));
            }
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _profile_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    async fn write_subscription_file(content: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "proxy-broker-subscription-{}.yaml",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::write(&path, content)
            .await
            .expect("subscription file should be written");
        path.to_string_lossy().to_string()
    }

    fn make_node(proxy_name: &str, ip: &str) -> ProxyNode {
        ProxyNode {
            proxy_name: proxy_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy: serde_json::json!({
                "name": proxy_name,
                "type": "socks5",
                "server": ip
            }),
        }
    }

    fn make_session(
        session_id: &str,
        proxy_name: &str,
        ip: &str,
        created_at: i64,
    ) -> SessionRecord {
        SessionRecord {
            session_id: session_id.to_string(),
            listen: "127.0.0.1".to_string(),
            port: 18080,
            selected_ip: ip.to_string(),
            proxy_name: proxy_name.to_string(),
            created_at,
        }
    }

    #[tokio::test]
    async fn load_subscription_skips_runtime_apply_when_no_session_survives() {
        let profile_id = "p-load";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(profile_id, &[make_node("old", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(profile_id, &make_session("s1", "old", "1.1.1.1", 1))
            .await
            .expect("seed session should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;

        let result = service
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await;

        let _ = tokio::fs::remove_file(&source_path).await;

        assert!(
            result.is_ok(),
            "stale sessions should not block subscription load"
        );
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 1);
        assert!(
            store
                .list_sessions(profile_id)
                .await
                .expect("list sessions should succeed")
                .is_empty(),
            "stale sessions should be cleaned from store"
        );
    }

    #[tokio::test]
    async fn close_session_allows_last_session_cleanup_without_runtime() {
        let profile_id = "p-close";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(profile_id, &[make_node("old", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(profile_id, &make_session("s1", "old", "1.1.1.1", 1))
            .await
            .expect("seed session should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        service
            .close_session(profile_id, "s1")
            .await
            .expect("closing last session should still succeed");

        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 1);
        assert!(
            store
                .list_sessions(profile_id)
                .await
                .expect("list sessions should succeed")
                .is_empty(),
            "last session should be removed from store"
        );
    }

    #[tokio::test]
    async fn load_subscription_rejects_when_no_resolved_ips() {
        let profile_id = "p-no-ip";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: unresolved
    type: socks5
    server: does-not-exist.invalid
"#,
        )
        .await;

        let result = service
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await;
        let _ = tokio::fs::remove_file(&source_path).await;

        assert!(matches!(result, Err(BrokerError::SubscriptionInvalid)));
    }

    #[tokio::test]
    async fn reconcile_startup_keeps_session_when_port_is_occupied() {
        let profile_id = "p-reconcile";
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0))
            .expect("should reserve a local port for test");
        let occupied_port = occupied
            .local_addr()
            .expect("listener should expose local addr")
            .port();

        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(profile_id, &[make_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(
                profile_id,
                &SessionRecord {
                    session_id: "s1".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: occupied_port,
                    selected_ip: "1.1.1.1".to_string(),
                    proxy_name: "node-a".to_string(),
                    created_at: 1,
                },
            )
            .await
            .expect("seed session should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, false));
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should complete");

        let sessions = store
            .list_sessions(profile_id)
            .await
            .expect("list sessions should succeed");
        assert_eq!(
            sessions.len(),
            1,
            "session should not be dropped on port probe"
        );
        assert_eq!(sessions[0].session_id, "s1");
    }

    #[tokio::test]
    async fn open_batch_empty_requests_is_noop_without_runtime() {
        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(
            Arc::new(MemoryStore::new()),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        let response = service
            .open_batch("p-empty", &OpenBatchRequest { requests: vec![] })
            .await
            .expect("empty batch should be a no-op");

        assert!(response.sessions.is_empty());
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn open_batch_surfaces_invalid_request_errors() {
        let profile_id = "p-batch-invalid";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(profile_id, &[make_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .replace_ip_records(profile_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let err = service
            .open_batch(
                profile_id,
                &OpenBatchRequest {
                    requests: vec![OpenSessionRequest {
                        specified_ip: Some("1.1.1.1".to_string()),
                        selector: None,
                        desired_port: Some(0),
                    }],
                },
            )
            .await
            .expect_err("invalid batch request should fail with explicit error");

        assert!(
            matches!(err, BrokerError::InvalidPort),
            "unexpected error: {err:?}"
        );
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn create_profile_trims_and_lists_empty_profile() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let created = service
            .create_profile("  fresh-lab  ")
            .await
            .expect("create should succeed");
        assert_eq!(created.profile_id, "fresh-lab");

        let profiles = service
            .list_profiles()
            .await
            .expect("list should succeed")
            .profiles;
        assert_eq!(profiles, vec!["fresh-lab"]);
    }

    #[tokio::test]
    async fn create_profile_rejects_duplicates() {
        let store = Arc::new(MemoryStore::new());
        store
            .create_profile("default", 1)
            .await
            .expect("seed create should succeed");
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let err = service
            .create_profile("default")
            .await
            .expect_err("duplicate create should fail");
        assert!(matches!(err, BrokerError::ProfileExists));
    }

    fn sample_ip(ip: &str, last_used_at: Option<i64>) -> IpRecord {
        IpRecord {
            ip: ip.to_string(),
            country_code: Some("US".to_string()),
            country_name: Some("United States".to_string()),
            region_name: Some("CA".to_string()),
            city: Some("San Jose".to_string()),
            geo_source: Some("test".to_string()),
            probe_updated_at: None,
            geo_updated_at: None,
            last_used_at,
        }
    }

    fn sample_probe(proxy_name: &str, ip: &str) -> ProbeRecord {
        ProbeRecord {
            proxy_name: proxy_name.to_string(),
            ip: ip.to_string(),
            target_url: "https://www.gstatic.com/generate_204".to_string(),
            ok: true,
            latency_ms: Some(100),
            updated_at: 1,
        }
    }

    fn sample_node(proxy_name: &str, ip: &str) -> ProxyNode {
        ProxyNode {
            proxy_name: proxy_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy: serde_json::json!({
                "name": proxy_name,
                "type": "socks5",
                "server": ip
            }),
        }
    }

    #[test]
    fn conflict_detected() {
        let req = ExtractIpRequest {
            specified_ips: vec!["1.1.1.1".to_string()],
            blacklist_ips: vec!["1.1.1.1".to_string()],
            ..Default::default()
        };
        assert!(matches!(
            validate_conflict(&req),
            Err(BrokerError::IpConflictBlacklist(_))
        ));
    }

    #[test]
    fn conflict_detected_for_ipv6_equivalent_forms() {
        let req = ExtractIpRequest {
            specified_ips: vec!["2001:DB8::1".to_string()],
            blacklist_ips: vec!["2001:db8:0:0:0:0:0:1".to_string()],
            ..Default::default()
        };
        assert!(matches!(
            validate_conflict(&req),
            Err(BrokerError::IpConflictBlacklist(_))
        ));
    }

    #[test]
    fn lru_puts_unseen_first() {
        let req = ExtractIpRequest {
            sort_mode: SortMode::Lru,
            ..Default::default()
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", None),
            sample_ip("3.3.3.3", Some(10)),
        ];
        let probes = vec![];
        let result = filter_ip_records(ips, &probes, &req).expect("should filter");
        let ordered: Vec<String> = result.into_iter().map(|x| x.ip).collect();
        assert_eq!(ordered, vec!["2.2.2.2", "3.3.3.3", "1.1.1.1"]);
    }

    #[test]
    fn mru_puts_recent_first() {
        let req = ExtractIpRequest {
            sort_mode: SortMode::Mru,
            ..Default::default()
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", None),
            sample_ip("3.3.3.3", Some(10)),
        ];
        let probes = vec![];
        let result = filter_ip_records(ips, &probes, &req).expect("should filter");
        let ordered: Vec<String> = result.into_iter().map(|x| x.ip).collect();
        assert_eq!(ordered, vec!["1.1.1.1", "3.3.3.3", "2.2.2.2"]);
    }

    #[test]
    fn blank_specified_ips_are_ignored() {
        let req = ExtractIpRequest {
            specified_ips: vec!["   ".to_string()],
            sort_mode: SortMode::Lru,
            ..Default::default()
        };
        let ips = vec![sample_ip("1.1.1.1", Some(100)), sample_ip("2.2.2.2", None)];
        let probes = vec![];
        let result = filter_ip_records(ips, &probes, &req).expect("should filter");
        let ordered: Vec<String> = result.into_iter().map(|x| x.ip).collect();
        assert_eq!(ordered, vec!["2.2.2.2", "1.1.1.1"]);
    }

    #[test]
    fn probe_records_keep_only_valid_proxy_ip_pairs() {
        let valid_pairs = HashSet::from([("proxy-a".to_string(), "1.1.1.1".to_string())]);
        let probes = vec![
            sample_probe("proxy-a", "1.1.1.1"),
            sample_probe("proxy-a", "2.2.2.2"),
            sample_probe("proxy-b", "1.1.1.1"),
        ];
        let filtered = filter_probe_records_by_pair(probes, &valid_pairs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].proxy_name, "proxy-a");
        assert_eq!(filtered[0].ip, "1.1.1.1");
    }

    #[test]
    fn stale_probe_timestamp_cleared_without_probe_records() {
        let mut ips = vec![sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)];
        ips[0].probe_updated_at = Some(10);
        ips[1].probe_updated_at = Some(20);
        let probes = vec![sample_probe("proxy-a", "2.2.2.2")];

        clear_stale_probe_timestamps(&mut ips, &probes);

        assert_eq!(ips[0].probe_updated_at, None);
        assert_eq!(ips[1].probe_updated_at, Some(20));
    }

    #[test]
    fn probe_cache_requires_complete_proxy_ip_target_matrix() {
        let nodes = vec![
            sample_node("proxy-a", "1.1.1.1"),
            sample_node("proxy-b", "1.1.1.1"),
        ];
        let targets = vec![
            "https://www.gstatic.com/generate_204".to_string(),
            "https://cp.cloudflare.com".to_string(),
        ];
        let probes = vec![
            ProbeRecord {
                proxy_name: "proxy-a".to_string(),
                ip: "1.1.1.1".to_string(),
                target_url: "https://www.gstatic.com/generate_204".to_string(),
                ok: true,
                latency_ms: Some(10),
                updated_at: 1,
            },
            ProbeRecord {
                proxy_name: "proxy-a".to_string(),
                ip: "1.1.1.1".to_string(),
                target_url: "https://cp.cloudflare.com".to_string(),
                ok: true,
                latency_ms: Some(20),
                updated_at: 1,
            },
            ProbeRecord {
                proxy_name: "proxy-b".to_string(),
                ip: "1.1.1.1".to_string(),
                target_url: "https://www.gstatic.com/generate_204".to_string(),
                ok: true,
                latency_ms: Some(30),
                updated_at: 1,
            },
        ];
        assert!(!has_complete_probe_records(&nodes, &targets, &probes));
    }

    #[test]
    fn probe_cache_complete_when_all_proxy_ip_target_pairs_exist() {
        let nodes = vec![sample_node("proxy-a", "1.1.1.1")];
        let targets = vec![
            "https://www.gstatic.com/generate_204".to_string(),
            "https://cp.cloudflare.com".to_string(),
        ];
        let probes = vec![
            ProbeRecord {
                proxy_name: "proxy-a".to_string(),
                ip: "1.1.1.1".to_string(),
                target_url: "https://www.gstatic.com/generate_204".to_string(),
                ok: true,
                latency_ms: Some(10),
                updated_at: 1,
            },
            ProbeRecord {
                proxy_name: "proxy-a".to_string(),
                ip: "1.1.1.1".to_string(),
                target_url: "https://cp.cloudflare.com".to_string(),
                ok: false,
                latency_ms: None,
                updated_at: 1,
            },
        ];
        assert!(has_complete_probe_records(&nodes, &targets, &probes));
    }

    #[test]
    fn duplicate_proxy_name_is_detected() {
        let nodes = vec![sample_node("dup", "1.1.1.1"), sample_node("dup", "2.2.2.2")];
        assert!(has_duplicate_proxy_names(&nodes));
    }

    #[test]
    fn distinct_proxy_names_are_accepted() {
        let nodes = vec![
            sample_node("proxy-a", "1.1.1.1"),
            sample_node("proxy-b", "2.2.2.2"),
        ];
        assert!(!has_duplicate_proxy_names(&nodes));
    }

    #[test]
    fn batch_stage_failure_returns_underlying_error() {
        let requests = vec![OpenSessionRequest {
            specified_ip: Some("9.9.9.9".to_string()),
            selector: None,
            desired_port: None,
        }];
        let nodes = vec![sample_node("proxy-a", "1.1.1.1")];
        let ips = vec![sample_ip("1.1.1.1", None)];
        let err = stage_batch_sessions(
            &requests,
            &nodes,
            &ips,
            &[],
            &[],
            Ipv4Addr::LOCALHOST.into(),
        )
        .expect_err("non-existent specified ip should fail");
        assert!(matches!(err, BrokerError::IpNotFound));
    }

    #[test]
    fn choose_ip_honors_selector_sort_mode_when_selector_provided() {
        let request = OpenSessionRequest {
            specified_ip: None,
            selector: Some(ExtractIpRequest {
                sort_mode: SortMode::Mru,
                ..Default::default()
            }),
            desired_port: None,
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", Some(10)),
        ];
        let chosen = choose_ip_for_open(&request, &ips, &[]).expect("should pick with mru");
        assert_eq!(chosen, "1.1.1.1");
    }

    #[test]
    fn choose_ip_respects_selector_limit() {
        let request = OpenSessionRequest {
            specified_ip: None,
            selector: Some(ExtractIpRequest {
                limit: Some(0),
                ..Default::default()
            }),
            desired_port: None,
        };
        let ips = vec![sample_ip("1.1.1.1", Some(100))];
        let err = choose_ip_for_open(&request, &ips, &[])
            .expect_err("selector limit=0 should produce no candidate");
        assert!(matches!(err, BrokerError::IpNotFound));
    }

    #[test]
    fn desired_port_zero_is_invalid() {
        let err = allocate_port(&[], Some(0), Ipv4Addr::LOCALHOST.into())
            .expect_err("port 0 should be rejected");
        assert!(matches!(err, BrokerError::InvalidPort));
    }

    #[test]
    fn allocate_port_respects_configured_listen_ip() {
        let occupied = std::net::TcpListener::bind(("0.0.0.0", 0))
            .expect("should reserve an externally visible port");
        let occupied_port = occupied
            .local_addr()
            .expect("listener should expose local addr")
            .port();

        let err = allocate_port(&[], Some(occupied_port), Ipv4Addr::UNSPECIFIED.into())
            .expect_err("occupied wildcard port should be rejected");
        assert!(matches!(err, BrokerError::PortInUse));
    }

    #[test]
    fn prepare_session_uses_configured_listen_ip() {
        let request = OpenSessionRequest {
            specified_ip: Some("1.1.1.1".to_string()),
            selector: None,
            desired_port: None,
        };
        let nodes = vec![sample_node("proxy-a", "1.1.1.1")];
        let ips = vec![sample_ip("1.1.1.1", None)];

        let session = prepare_session(
            &request,
            &nodes,
            &ips,
            &[],
            &[],
            Ipv4Addr::UNSPECIFIED.into(),
        )
        .expect("session should be prepared");

        assert_eq!(session.listen, "0.0.0.0");
    }
}
