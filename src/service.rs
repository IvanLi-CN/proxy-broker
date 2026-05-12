use std::{
    cmp::Ordering as CmpOrdering,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, anyhow};
use futures_util::{StreamExt, TryStreamExt, stream};
use maxminddb::{Reader, geoip2};
use serde::Deserialize;
use tokio::sync::{Mutex as TokioMutex, broadcast};

use crate::{
    auth::{Principal, constant_time_eq, hash_secret, issue_api_key, parse_api_key_secret},
    config_render::{dedicated_ip_proxy_name, render_payload},
    constants::{
        DEFAULT_GEO_ONLINE_CONCURRENCY, DEFAULT_GEO_TTL_SEC, DEFAULT_MMDB_URL,
        DEFAULT_ONLINE_GEO_BASE, DEFAULT_PROBE_CONCURRENCY, DEFAULT_PROBE_TARGETS,
        DEFAULT_PROBE_TIMEOUT_MS, DEFAULT_PROBE_TTL_SEC, DEFAULT_PROXY_PROBE_INTERVAL_SEC,
        DEFAULT_SESSION_LISTEN_IP,
    },
    error::{BrokerError, BrokerResult},
    ids,
    models::{
        ApiKeyProjectScope, ApiKeyProjectScopeKind, CreateApiKeyRequest, CreateApiKeyResponse,
        CreateProjectResponse, ExtractIpItem, ExtractIpRequest, ExtractIpResponse, IpRecord,
        ListApiKeysResponse, ListProjectsResponse, ListProxyImportResponse,
        ListProxyInventoryResponse, ListSessionsResponse, LoadSubscriptionRequest,
        LoadSubscriptionResponse, OpenBatchByIpRequest, OpenBatchByNodeRequest, OpenBatchRequest,
        OpenBatchResponse, OpenSessionByIpRequest, OpenSessionByNodeRequest, OpenSessionRequest,
        OpenSessionResponse, ProbeRecord, ProjectProxySettings, ProxyCatalogGroupItem,
        ProxyCatalogNodeItem, ProxyCatalogQuery, ProxyCatalogResponse, ProxyImportItem,
        ProxyImportKind, ProxyImportRecord, ProxyImportSourceIdentity, ProxyImportSyncConfig,
        ProxyInventoryItem, ProxyInventoryRecord, ProxyNode, ProxyNodeMetadataRecord,
        ProxyNodeProbeSampleRecord, ProxyOperationAcceptedResponse, ProxyOperationRequest,
        ProxyScope, RefreshRequest, RefreshResponse, ResolvedImportNameSource,
        SearchSessionIpNodeOptionsRequest, SearchSessionIpNodeOptionsResponse,
        SearchSessionNodeOptionsRequest, SearchSessionNodeOptionsResponse,
        SearchSessionOptionsRequest, SearchSessionOptionsResponse, SessionIpNodeGroupBy,
        SessionIpNodeOptionGroupItem, SessionIpNodeOptionIpItem, SessionIpNodeOptionNodeItem,
        SessionListItem, SessionNodeOptionItem, SessionNodeSortMode, SessionOptionItem,
        SessionOptionKind, SessionRecord, SessionSelectionMode, SubscriptionMetadata,
        SubscriptionSource, SuggestedPortResponse, SystemSettings, TaskEventLevel, TaskListQuery,
        TaskListResponse, TaskRunDetail, TaskRunEventRecord, TaskRunKind, TaskRunRecord,
        TaskRunScope, TaskRunStage, TaskRunStatus, TaskRunSummary, TaskRunTrigger,
        UpdateSessionNodeRequest, now_epoch_sec,
    },
    proxy_node_validation::{filter_malformed_proxy_nodes, malformed_proxy_reason},
    runtime::MihomoRuntime,
    store::BrokerStore,
    subscription,
    tasks::{TaskBusEvent, build_task_list_response, to_detail},
};

const DEFAULT_AUTO_SYNC_EVERY_SEC: u64 = 600;
const DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC: u64 = 86_400;
const TASK_SCHEDULE_SCAN_SEC: u64 = 30;
const TASK_DISPATCH_POLL_SEC: u64 = 1;
const DEFAULT_SESSION_OPTIONS_LIMIT: usize = 25;
const DEFAULT_SESSION_NODE_OPTIONS_LIMIT: usize = 80;
const GLOBAL_RUNTIME_PROJECT_ID: &str = "__global__";
const PROXY_PROBE_ROUNDS: usize = 5;
const AUTO_PORT_BIND_ATTEMPTS: usize = 3;

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
    pub session_public_host: Option<String>,
    pub session_port_range: Option<(u16, u16)>,
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
            session_public_host: None,
            session_port_range: None,
        }
    }
}

#[derive(Clone)]
pub struct BrokerService {
    store: Arc<dyn BrokerStore>,
    runtime: Arc<dyn MihomoRuntime>,
    http: reqwest::Client,
    options: BrokerServiceOptions,
    project_locks: Vec<Arc<TokioMutex<()>>>,
    shared_runtime_lock: Arc<TokioMutex<()>>,
    proxy_probe_queue_lock: Arc<TokioMutex<()>>,
    task_events: broadcast::Sender<TaskBusEvent>,
    task_active_projects: Arc<TokioMutex<HashSet<String>>>,
    task_supervisor_started: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct LoadSubscriptionOutcome {
    response: LoadSubscriptionResponse,
    new_ips: Vec<String>,
    import_id: String,
}

#[derive(Debug, Clone)]
struct ImportedInventoryOutcome {
    response: LoadSubscriptionResponse,
    import_id: String,
}

#[derive(Debug, Clone)]
struct ResolvedImportName {
    value: Option<String>,
    source: Option<ResolvedImportNameSource>,
}

#[derive(Debug, Clone, Default)]
struct LegacyProjectMetadata {
    ip_records: HashMap<String, IpRecord>,
    probe_records: HashMap<(String, String), Vec<ProbeRecord>>,
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
        let (task_events, _) = broadcast::channel(256);
        Self {
            store,
            runtime,
            http,
            options,
            project_locks: (0..64).map(|_| Arc::new(TokioMutex::new(()))).collect(),
            shared_runtime_lock: Arc::new(TokioMutex::new(())),
            proxy_probe_queue_lock: Arc::new(TokioMutex::new(())),
            task_events,
            task_active_projects: Arc::new(TokioMutex::new(HashSet::new())),
            task_supervisor_started: Arc::new(AtomicBool::new(false)),
        }
    }

    fn project_lock_index(&self, project_id: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        project_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.project_locks.len()
    }

    async fn lock_project(&self, project_id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self.project_locks[self.project_lock_index(project_id)].clone();
        lock.lock_owned().await
    }

    fn resolve_session_display_host(
        &self,
        bind_host: &str,
        request_display_host: Option<&str>,
    ) -> String {
        if is_wildcard_session_host(bind_host) {
            return normalize_session_host(self.options.session_public_host.as_deref())
                .or_else(|| normalize_session_host(request_display_host))
                .unwrap_or_else(|| bind_host.trim().to_string());
        }

        normalize_session_host(Some(bind_host)).unwrap_or_else(|| bind_host.trim().to_string())
    }

    async fn project_exists(&self, project_id: &str) -> BrokerResult<bool> {
        let projects = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?;
        Ok(projects.into_iter().any(|item| item == project_id))
    }

    pub async fn require_project_exists(&self, project_id: &str) -> BrokerResult<()> {
        if self.project_exists(project_id).await? {
            Ok(())
        } else {
            Err(BrokerError::ProjectNotFound)
        }
    }

    fn default_project_proxy_settings(&self, project_id: &str) -> ProjectProxySettings {
        ProjectProxySettings {
            project_id: project_id.to_string(),
            use_global_proxies: true,
        }
    }

    async fn get_project_proxy_settings_effective(
        &self,
        project_id: &str,
    ) -> BrokerResult<ProjectProxySettings> {
        Ok(self
            .store
            .get_project_proxy_settings(project_id)
            .await
            .map_err(BrokerError::from)?
            .unwrap_or_else(|| self.default_project_proxy_settings(project_id)))
    }

    async fn list_project_ids_with_settings(
        &self,
    ) -> BrokerResult<(Vec<String>, HashMap<String, ProjectProxySettings>)> {
        let projects = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?;
        let mut settings = HashMap::new();
        for project_id in &projects {
            settings.insert(
                project_id.clone(),
                self.get_project_proxy_settings_effective(project_id)
                    .await?,
            );
        }
        Ok((projects, settings))
    }

    fn effective_project_ids_for_record(
        &self,
        record: &ProxyInventoryRecord,
        projects: &[String],
        settings: &HashMap<String, ProjectProxySettings>,
    ) -> Vec<String> {
        match &record.allocation_scope {
            ProxyScope::Global => projects
                .iter()
                .filter(|project_id| {
                    settings
                        .get(project_id.as_str())
                        .map(|item| item.use_global_proxies)
                        .unwrap_or(true)
                })
                .cloned()
                .collect(),
            ProxyScope::Project { project_id } => vec![project_id.clone()],
        }
    }

    async fn normalize_api_key_scope(
        &self,
        scope: &ApiKeyProjectScope,
    ) -> BrokerResult<ApiKeyProjectScope> {
        match scope.kind {
            ApiKeyProjectScopeKind::AllProjects => {
                if !scope.project_ids.is_empty() {
                    return Err(BrokerError::InvalidRequest(
                        "project ids must be omitted when project_scope.kind=all_projects"
                            .to_string(),
                    ));
                }
                Ok(ApiKeyProjectScope::all_projects())
            }
            ApiKeyProjectScopeKind::SelectedProjects => {
                let mut project_ids = scope
                    .project_ids
                    .iter()
                    .map(|item| item.trim())
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                project_ids.sort();
                project_ids.dedup();
                if project_ids.is_empty() {
                    return Err(BrokerError::InvalidRequest(
                        "project_scope.project_ids must not be empty when kind=selected_projects"
                            .to_string(),
                    ));
                }

                let known_projects = self
                    .store
                    .list_projects()
                    .await
                    .map_err(BrokerError::from)?
                    .into_iter()
                    .collect::<HashSet<_>>();
                let unknown_projects = project_ids
                    .iter()
                    .filter(|project_id| !known_projects.contains(*project_id))
                    .cloned()
                    .collect::<Vec<_>>();
                if !unknown_projects.is_empty() {
                    return Err(BrokerError::InvalidRequest(format!(
                        "unknown projects in project_scope.project_ids: {}",
                        unknown_projects.join(", ")
                    )));
                }

                Ok(ApiKeyProjectScope::selected(project_ids))
            }
        }
    }

    async fn collect_all_sessions(
        &self,
        override_project_id: Option<&str>,
        override_sessions: Option<&[SessionRecord]>,
    ) -> BrokerResult<Vec<SessionRecord>> {
        let mut sessions = Vec::new();
        for project_id in self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?
        {
            if override_project_id == Some(project_id.as_str()) {
                let override_sessions = override_sessions.unwrap_or(&[]);
                let restorable_sessions = self
                    .filter_restorable_sessions_for_project(
                        &project_id,
                        override_sessions,
                        "shared runtime apply kept persisted override session but left it out of runtime restore",
                    )
                    .await?;
                sessions.extend(restorable_sessions);
                continue;
            }
            sessions.extend(self.list_sessions_backfilled(&project_id).await?);
        }
        sessions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        Ok(sessions)
    }

    async fn collect_restorable_runtime_sessions(
        &self,
        override_project_id: Option<&str>,
        override_sessions: Option<&[SessionRecord]>,
    ) -> BrokerResult<Vec<SessionRecord>> {
        let mut sessions = Vec::new();
        for project_id in self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?
        {
            if override_project_id == Some(project_id.as_str()) {
                let override_sessions = override_sessions.unwrap_or(&[]);
                let restorable_sessions = self
                    .filter_restorable_sessions_for_project(
                        &project_id,
                        override_sessions,
                        "shared runtime apply kept persisted override session but left it out of runtime restore",
                    )
                    .await?;
                sessions.extend(restorable_sessions);
                continue;
            }
            let existing_sessions = self.list_sessions_backfilled(&project_id).await?;
            let restorable_sessions = self
                .filter_restorable_sessions_for_project(
                    &project_id,
                    &existing_sessions,
                    "shared runtime apply kept persisted session but left it out of runtime restore",
                )
                .await?;
            sessions.extend(restorable_sessions);
        }
        sessions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        Ok(sessions)
    }

    fn filter_malformed_inventory_records(
        &self,
        project_id: &str,
        records: Vec<ProxyInventoryRecord>,
        log_message: &'static str,
    ) -> Vec<ProxyInventoryRecord> {
        records
            .into_iter()
            .filter(|item| {
                if let Some(reason) = malformed_proxy_reason(&item.proxy_type, &item.raw_proxy) {
                    tracing::warn!(
                        project_id,
                        import_id = %item.import_id,
                        node_id = %item.node_id,
                        source_scope = %item.source_scope.key(),
                        allocation_scope = %item.allocation_scope.key(),
                        proxy_name = %item.proxy_name,
                        proxy_type = %item.proxy_type,
                        error = %reason,
                        "{log_message}"
                    );
                    false
                } else {
                    true
                }
            })
            .collect()
    }

    async fn collect_all_runtime_nodes(&self) -> BrokerResult<Vec<ProxyNode>> {
        let mut nodes = self
            .filter_malformed_inventory_records(
                GLOBAL_RUNTIME_PROJECT_ID,
                self.store
                    .list_proxy_inventory()
                    .await
                    .map_err(BrokerError::from)?,
                "malformed proxy inventory node skipped from shared runtime payload",
            )
            .into_iter()
            .map(|item| ProxyNode {
                node_id: Some(item.node_id),
                proxy_name: item.proxy_name,
                proxy_type: item.proxy_type,
                server: item.server,
                resolved_ips: item.resolved_ips,
                raw_proxy: item.raw_proxy,
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| {
            left.node_id
                .cmp(&right.node_id)
                .then_with(|| left.proxy_name.cmp(&right.proxy_name))
        });
        Ok(nodes)
    }

    async fn apply_shared_runtime_config(
        &self,
        override_project_id: Option<&str>,
        override_sessions: Option<&[SessionRecord]>,
        start_without_sessions: bool,
    ) -> BrokerResult<()> {
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        self.apply_shared_runtime_config_locked(
            override_project_id,
            override_sessions,
            start_without_sessions,
        )
        .await
    }

    async fn apply_shared_runtime_config_locked(
        &self,
        override_project_id: Option<&str>,
        override_sessions: Option<&[SessionRecord]>,
        start_without_sessions: bool,
    ) -> BrokerResult<()> {
        let sessions = self
            .collect_restorable_runtime_sessions(override_project_id, override_sessions)
            .await?;
        self.apply_exact_shared_runtime_config_locked(&sessions, start_without_sessions, false)
            .await
    }

    async fn apply_exact_shared_runtime_config_locked(
        &self,
        sessions: &[SessionRecord],
        start_without_sessions: bool,
        shutdown_when_empty: bool,
    ) -> BrokerResult<()> {
        let nodes = self.collect_all_runtime_nodes().await?;
        if nodes.is_empty() || (sessions.is_empty() && !start_without_sessions) {
            if shutdown_when_empty {
                if let Err(err) = self
                    .runtime
                    .shutdown_project(GLOBAL_RUNTIME_PROJECT_ID)
                    .await
                {
                    tracing::warn!(
                        error = %err,
                        "failed to shutdown shared runtime after exact runtime session set became empty"
                    );
                }
            } else {
                self.cleanup_shared_runtime_if_idle_locked().await;
            }
            return Ok(());
        }

        let (controller, secret) = self
            .runtime
            .controller_meta(GLOBAL_RUNTIME_PROJECT_ID)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
        let payload = render_payload(&controller, secret.as_deref(), &nodes, sessions)
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
        self.runtime
            .apply_config(GLOBAL_RUNTIME_PROJECT_ID, &payload)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))
    }

    async fn apply_project_restorable_sessions_locked(
        &self,
        project_id: &str,
        restorable_sessions: &[SessionRecord],
    ) -> BrokerResult<()> {
        let mut sessions = Vec::new();
        let project_ids = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?;
        for current_project_id in project_ids {
            if current_project_id == project_id {
                sessions.extend(restorable_sessions.iter().cloned());
            } else {
                let existing_sessions = self.list_sessions_backfilled(&current_project_id).await?;
                let restorable_sessions = self
                    .filter_restorable_sessions_for_project(
                        &current_project_id,
                        &existing_sessions,
                        "shared runtime rebuild kept persisted session but left it out of runtime restore",
                    )
                    .await?;
                sessions.extend(restorable_sessions);
            }
        }
        sessions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        self.apply_exact_shared_runtime_config_locked(&sessions, false, true)
            .await
    }

    async fn filter_restorable_sessions_for_project(
        &self,
        project_id: &str,
        existing_sessions: &[SessionRecord],
        unrestored_log_message: &'static str,
    ) -> BrokerResult<Vec<SessionRecord>> {
        if existing_sessions.is_empty() {
            return Ok(vec![]);
        }

        let inventory_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let restorable_sessions = if inventory_nodes.is_empty() {
            let nodes = self.compose_effective_session_nodes(project_id).await?;
            let valid_proxy_ip_pairs = nodes
                .iter()
                .flat_map(valid_proxy_ip_pairs_for_node)
                .collect::<HashSet<_>>();

            if valid_proxy_ip_pairs.is_empty() {
                log_unrestored_sessions(project_id, existing_sessions, &[], unrestored_log_message);
                return Ok(vec![]);
            }

            existing_sessions
                .iter()
                .filter(|session| {
                    valid_proxy_ip_pairs.contains(&(
                        session_runtime_key(session).to_string(),
                        session.selected_ip.clone(),
                    ))
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
            existing_sessions
                .iter()
                .filter_map(|session| {
                    reselect_session_from_inventory(session, &inventory_nodes, &metadata_by_pair)
                })
                .collect::<Vec<_>>()
        };
        log_unrestored_sessions(
            project_id,
            existing_sessions,
            &restorable_sessions,
            unrestored_log_message,
        );
        Ok(restorable_sessions)
    }

    async fn cleanup_shared_runtime_if_idle(&self) {
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        self.cleanup_shared_runtime_if_idle_locked().await;
    }

    async fn cleanup_shared_runtime_if_idle_locked(&self) {
        let sessions = match self.collect_all_sessions(None, None).await {
            Ok(sessions) => sessions,
            Err(err) => {
                tracing::warn!(error = %err, "failed to inspect shared runtime idleness");
                return;
            }
        };
        if !sessions.is_empty() {
            return;
        }

        if let Err(err) = self
            .runtime
            .shutdown_project(GLOBAL_RUNTIME_PROJECT_ID)
            .await
        {
            tracing::warn!(
                error = %err,
                "failed to shutdown idle shared runtime"
            );
        }
    }

    pub async fn reconcile_startup_sessions(&self) -> BrokerResult<()> {
        let projects = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?;
        for project_id in projects {
            if let Err(err) = self.reconcile_project_sessions(&project_id).await {
                tracing::warn!(
                    project_id,
                    error = %err,
                    "startup session reconciliation failed"
                );
            }
        }
        Ok(())
    }

    async fn reconcile_project_sessions(&self, project_id: &str) -> BrokerResult<()> {
        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;

        let existing_sessions = self.list_sessions_backfilled(project_id).await?;
        if existing_sessions.is_empty() {
            self.cleanup_shared_runtime_if_idle_locked().await;
            return Ok(());
        }

        let inventory_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let reconciled_sessions: Vec<SessionRecord> = if inventory_nodes.is_empty() {
            let nodes = self
                .store
                .list_subscription(project_id)
                .await
                .map_err(BrokerError::from)?;
            let valid_proxy_ip_pairs = nodes
                .iter()
                .flat_map(valid_proxy_ip_pairs_for_node)
                .collect::<HashSet<_>>();
            if valid_proxy_ip_pairs.is_empty() {
                tracing::warn!(
                    project_id,
                    session_count = existing_sessions.len(),
                    "startup session reconciliation skipped pruning because no authoritative proxy/IP pairs were available"
                );
                return Ok(());
            }
            existing_sessions
                .iter()
                .filter(|session| {
                    valid_proxy_ip_pairs.contains(&(
                        session_runtime_key(session).to_string(),
                        session.selected_ip.clone(),
                    ))
                })
                .cloned()
                .collect()
        } else {
            let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
            existing_sessions
                .iter()
                .filter_map(|session| {
                    reselect_session_from_inventory(session, &inventory_nodes, &metadata_by_pair)
                })
                .collect()
        };

        log_unrestored_sessions(
            project_id,
            &existing_sessions,
            &reconciled_sessions,
            "startup session reconciliation left persisted session out of runtime restore",
        );

        if reconciled_sessions.is_empty() {
            self.apply_project_restorable_sessions_locked(project_id, &[])
                .await?;
            return Ok(());
        }

        self.apply_project_restorable_sessions_locked(project_id, &reconciled_sessions)
            .await?;
        self.store
            .insert_sessions(project_id, &reconciled_sessions)
            .await
            .map_err(BrokerError::from)?;

        Ok(())
    }

    pub fn start_background_workers(self: &Arc<Self>) {
        if self.task_supervisor_started.swap(true, Ordering::SeqCst) {
            return;
        }

        let service = Arc::clone(self);
        tokio::spawn(async move {
            service.task_supervisor_loop().await;
        });
    }

    async fn task_supervisor_loop(self: Arc<Self>) {
        if let Err(err) = self.recover_interrupted_task_runs().await {
            tracing::warn!(error = %err, "task supervisor failed to recover interrupted runs");
        }

        let mut schedule_tick = tokio::time::interval(Duration::from_secs(TASK_SCHEDULE_SCAN_SEC));
        schedule_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut dispatch_tick = tokio::time::interval(Duration::from_secs(TASK_DISPATCH_POLL_SEC));
        dispatch_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = schedule_tick.tick() => {
                    if let Err(err) = self.enqueue_due_tasks().await {
                        tracing::warn!(error = %err, "task supervisor failed to enqueue due tasks");
                    }
                }
                _ = dispatch_tick.tick() => {
                    if let Err(err) = self.dispatch_queued_tasks().await {
                        tracing::warn!(error = %err, "task supervisor failed to dispatch queued tasks");
                    }
                }
            }
        }
    }

    async fn recover_interrupted_task_runs(&self) -> BrokerResult<()> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery::default())
            .await
            .map_err(BrokerError::from)?;
        let now = now_epoch_sec();

        for mut run in runs
            .into_iter()
            .filter(|run| run.status == TaskRunStatus::Running)
        {
            run.status = TaskRunStatus::Failed;
            run.stage = TaskRunStage::Completed;
            run.finished_at = Some(now);
            run.error_code = Some("interrupted_on_restart".to_string());
            run.error_message =
                Some("task run interrupted while service was restarting".to_string());
            self.update_task_run_and_emit(&run).await?;
            self.append_task_event(
                &run,
                TaskEventLevel::Error,
                TaskRunStage::Completed,
                "Task run was interrupted by service restart.",
                None,
            )
            .await?;
        }

        Ok(())
    }

    async fn enqueue_due_tasks(&self) -> BrokerResult<()> {
        let configs = self
            .store
            .list_proxy_import_sync_configs()
            .await
            .map_err(BrokerError::from)?;
        let now = now_epoch_sec();

        let mut configs_by_project = HashMap::<String, Vec<ProxyImportSyncConfig>>::new();
        for config in configs {
            configs_by_project
                .entry(config.project_id.clone())
                .or_default()
                .push(config);
        }

        for (project_id, configs) in configs_by_project {
            if self.has_pending_or_running_tasks(&project_id).await? {
                continue;
            }

            let sync_due = configs.iter().any(|config| {
                config.enabled && config.last_sync_due_at.map(|ts| ts <= now).unwrap_or(false)
            });
            let full_due = configs.iter().any(|config| {
                config.enabled
                    && config
                        .last_full_refresh_due_at
                        .map(|ts| ts <= now)
                        .unwrap_or(false)
            });

            if !sync_due && !full_due {
                continue;
            }

            if sync_due {
                self.enqueue_task_run(
                    &project_id,
                    TaskRunKind::SubscriptionSync,
                    TaskRunTrigger::Schedule,
                    TaskRunScope::All,
                )
                .await?;
            }

            if full_due {
                self.enqueue_task_run(
                    &project_id,
                    TaskRunKind::MetadataRefreshFull,
                    TaskRunTrigger::Schedule,
                    TaskRunScope::All,
                )
                .await?;
            }
        }

        self.enqueue_due_proxy_probe_task(now).await?;

        Ok(())
    }

    async fn enqueue_due_proxy_probe_task(&self, now: i64) -> BrokerResult<()> {
        let _probe_queue_guard = self.proxy_probe_queue_lock.lock().await;
        let settings = self.get_system_settings().await?;
        if self
            .has_pending_or_running_tasks(GLOBAL_RUNTIME_PROJECT_ID)
            .await?
        {
            return Ok(());
        }

        let records = self
            .store
            .list_proxy_inventory()
            .await
            .map_err(BrokerError::from)?;
        let subscription_import_ids = self.subscription_import_ids().await?;
        if records.iter().all(|record| {
            !subscription_import_ids.contains(&record.import_id) || record.resolved_ips.is_empty()
        }) {
            return Ok(());
        }

        let latest_scheduled_probe = self
            .store
            .list_task_runs(&TaskListQuery {
                project_id: Some(GLOBAL_RUNTIME_PROJECT_ID.to_string()),
                kind: Some(TaskRunKind::ProxyLatencyProbe),
                trigger: Some(TaskRunTrigger::Schedule),
                ..TaskListQuery::default()
            })
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|run| run.finished_at.or(run.started_at).unwrap_or(run.created_at))
            .max();
        if latest_scheduled_probe
            .map(|ts| {
                i64::try_from(settings.proxy_probe_interval_sec)
                    .ok()
                    .and_then(|interval| ts.checked_add(interval))
                    .unwrap_or(i64::MAX)
                    > now
            })
            .unwrap_or(false)
        {
            return Ok(());
        }

        let active_node_ids = self.active_proxy_probe_node_ids().await?;
        let target_node_ids = records
            .into_iter()
            .filter(|record| subscription_import_ids.contains(&record.import_id))
            .filter(|record| !record.resolved_ips.is_empty())
            .filter(|record| !active_node_ids.contains(&record.node_id))
            .map(|record| record.node_id)
            .collect::<Vec<_>>();
        if target_node_ids.is_empty() {
            return Ok(());
        }
        let scope = if active_node_ids.is_empty() {
            TaskRunScope::All
        } else {
            TaskRunScope::Nodes {
                node_ids: target_node_ids,
            }
        };

        self.enqueue_task_run(
            GLOBAL_RUNTIME_PROJECT_ID,
            TaskRunKind::ProxyLatencyProbe,
            TaskRunTrigger::Schedule,
            scope,
        )
        .await?;
        Ok(())
    }

    async fn subscription_import_ids(&self) -> BrokerResult<HashSet<String>> {
        Ok(self
            .store
            .list_proxy_imports()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .filter(|record| record.import_kind == ProxyImportKind::Subscription)
            .map(|record| record.import_id)
            .collect())
    }

    async fn dispatch_queued_tasks(self: &Arc<Self>) -> BrokerResult<()> {
        let mut runs = self
            .store
            .list_task_runs(&TaskListQuery::default())
            .await
            .map_err(BrokerError::from)?;
        runs.retain(|run| run.status == TaskRunStatus::Queued);
        sort_queued_runs_for_dispatch(&mut runs);

        for run in runs {
            if !self.claim_task_project(&run.project_id).await {
                continue;
            }

            let service = Arc::clone(self);
            tokio::spawn(async move {
                service.run_task(run).await;
            });
        }

        Ok(())
    }

    async fn claim_task_project(&self, project_id: &str) -> bool {
        let mut active = self.task_active_projects.lock().await;
        active.insert(project_id.to_string())
    }

    async fn release_task_project(&self, project_id: &str) {
        let mut active = self.task_active_projects.lock().await;
        active.remove(project_id);
    }

    async fn has_pending_or_running_tasks(&self, project_id: &str) -> BrokerResult<bool> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .map_err(BrokerError::from)?;
        Ok(runs
            .into_iter()
            .any(|run| matches!(run.status, TaskRunStatus::Queued | TaskRunStatus::Running)))
    }

    async fn active_proxy_probe_node_ids(&self) -> BrokerResult<HashSet<String>> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery {
                kind: Some(TaskRunKind::ProxyLatencyProbe),
                ..TaskListQuery::default()
            })
            .await
            .map_err(BrokerError::from)?;
        let mut active = HashSet::new();
        for run in runs
            .into_iter()
            .filter(|run| matches!(run.status, TaskRunStatus::Queued | TaskRunStatus::Running))
        {
            match self.resolve_proxy_operation_nodes_for_run(&run).await {
                Ok(nodes) => {
                    active.extend(nodes.into_iter().map(|node| node.node_id));
                }
                Err(err) => {
                    tracing::warn!(
                        run_id = %run.run_id,
                        error = %err,
                        "failed to resolve active proxy latency probe targets"
                    );
                }
            }
        }
        Ok(active)
    }

    async fn queued_or_running_task_runs(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<TaskRunRecord>> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .map_err(BrokerError::from)?;
        Ok(runs
            .into_iter()
            .filter(|run| matches!(run.status, TaskRunStatus::Queued | TaskRunStatus::Running))
            .collect())
    }

    async fn run_task(self: Arc<Self>, mut run: TaskRunRecord) {
        let result = match run.kind {
            TaskRunKind::SubscriptionSync => self.execute_subscription_sync_task(&mut run).await,
            TaskRunKind::MetadataRefreshIncremental => {
                self.execute_incremental_refresh_task(&mut run).await
            }
            TaskRunKind::MetadataRefreshFull => self.execute_full_refresh_task(&mut run).await,
            TaskRunKind::ProxyMetadataRefresh => {
                self.execute_proxy_metadata_refresh_task(&mut run).await
            }
            TaskRunKind::ProxyLatencyProbe => self.execute_proxy_latency_probe_task(&mut run).await,
        };

        if let Err(err) = result {
            tracing::warn!(
                run_id = %run.run_id,
                project_id = %run.project_id,
                error = %err,
                "task run failed"
            );
            let _ = self.fail_task_run(&mut run, err).await;
        }

        self.release_task_project(&run.project_id).await;
    }

    async fn execute_subscription_sync_task(&self, run: &mut TaskRunRecord) -> BrokerResult<()> {
        self.mark_task_running(run, TaskRunStage::LoadingSubscription, None, None)
            .await?;
        self.append_task_event(
            run,
            TaskEventLevel::Info,
            TaskRunStage::LoadingSubscription,
            "Refreshing subscription feed for project.",
            None,
        )
        .await?;

        let now = now_epoch_sec();
        let configs = self
            .store
            .list_proxy_import_sync_configs_for_project(&run.project_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .filter(|config| {
                config.enabled && config.last_sync_due_at.map(|ts| ts <= now).unwrap_or(false)
            })
            .collect::<Vec<_>>();
        if configs.is_empty() {
            return Err(BrokerError::InvalidRequest(format!(
                "project `{}` has no due persisted subscription source",
                run.project_id
            )));
        }
        let due_import_ids = configs
            .iter()
            .map(|config| config.import_id.clone())
            .collect::<Vec<_>>();
        self.mark_sync_started_for_imports(&due_import_ids).await?;

        let mut total_loaded_proxies = 0usize;
        let mut warnings = Vec::<String>::new();
        let mut new_ips = Vec::<String>::new();
        let mut distinct_ips = HashSet::<String>::new();
        let mut completed_import_ids = Vec::<String>::new();
        for config in &configs {
            let outcome = match self
                .load_subscription_internal(&run.project_id, &config.source, None)
                .await
            {
                Ok(outcome) => outcome,
                Err(err) => {
                    let failed_at = now_epoch_sec();
                    if !completed_import_ids.is_empty() {
                        self.mark_sync_finished_for_imports(&completed_import_ids, failed_at)
                            .await?;
                    }
                    self.mark_sync_failed_for_imports(
                        std::slice::from_ref(&config.import_id),
                        failed_at,
                    )
                    .await?;
                    return Err(Self::with_subscription_sync_context(err, &config.import_id));
                }
            };
            total_loaded_proxies += outcome.response.loaded_proxies;
            warnings.extend(outcome.response.warnings);
            new_ips.extend(outcome.new_ips);
            completed_import_ids.push(config.import_id.clone());
            for node in self
                .store
                .list_proxy_inventory_for_import(&outcome.import_id)
                .await
                .map_err(BrokerError::from)?
            {
                distinct_ips.extend(node.resolved_ips);
            }
        }
        new_ips.sort();
        new_ips.dedup();
        let targeted_ips = new_ips.len() as u64;
        run.progress_total = Some(targeted_ips);
        self.update_task_run_and_emit(run).await?;
        self.append_task_event(
            run,
            TaskEventLevel::Info,
            TaskRunStage::DiffingInventory,
            format!("Subscription sync finished with {} new IP(s).", new_ips.len()),
            Some(serde_json::json!({
                "loaded_proxies": total_loaded_proxies,
                "distinct_ips": distinct_ips.len(),
                "warnings": warnings,
                "new_ips": new_ips,
                "import_ids": configs.iter().map(|config| config.import_id.clone()).collect::<Vec<_>>(),
            })),
        )
        .await?;

        if new_ips.is_empty() {
            self.mark_sync_finished_for_imports(&due_import_ids, now_epoch_sec())
                .await?;
            self.complete_task_run(
                run,
                TaskRunStatus::Succeeded,
                Some(serde_json::json!({
                    "loaded_proxies": total_loaded_proxies,
                    "distinct_ips": distinct_ips.len(),
                    "warnings": warnings,
                    "new_ips": 0,
                    "probed_ips": 0,
                    "geo_updated": 0,
                    "skipped_cached": 0,
                })),
                None,
                None,
            )
            .await?;
            return Ok(());
        }

        if self
            .queued_or_running_task_runs(&run.project_id)
            .await?
            .into_iter()
            .any(|queued_run| {
                queued_run.run_id != run.run_id
                    && queued_run.kind == TaskRunKind::MetadataRefreshFull
            })
        {
            self.mark_sync_finished_for_imports(&due_import_ids, now_epoch_sec())
                .await?;
            self.complete_task_run(
                run,
                TaskRunStatus::Succeeded,
                Some(serde_json::json!({
                    "loaded_proxies": total_loaded_proxies,
                    "distinct_ips": distinct_ips.len(),
                    "warnings": warnings,
                    "new_ips": targeted_ips,
                    "probed_ips": 0,
                    "geo_updated": 0,
                    "skipped_cached": 0,
                    "deferred_to_full_refresh": true,
                })),
                None,
                None,
            )
            .await?;
            return Ok(());
        }

        let target_ip_set = new_ips.iter().cloned().collect::<HashSet<_>>();
        let refresh = match self
            .refresh_metadata_internal(
                &run.project_id,
                false,
                Some(&target_ip_set),
                Some(&run.run_id),
            )
            .await
        {
            Ok(refresh) => refresh,
            Err(err) => {
                self.mark_sync_failed_for_imports(&due_import_ids, now_epoch_sec())
                    .await?;
                return Err(err);
            }
        };

        self.mark_sync_finished_for_imports(&due_import_ids, now_epoch_sec())
            .await?;
        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "loaded_proxies": total_loaded_proxies,
                "distinct_ips": distinct_ips.len(),
                "warnings": warnings,
                "new_ips": targeted_ips,
                "probed_ips": refresh.probed_ips,
                "geo_updated": refresh.geo_updated,
                "skipped_cached": refresh.skipped_cached,
            })),
            None,
            None,
        )
        .await
    }

    async fn execute_incremental_refresh_task(&self, run: &mut TaskRunRecord) -> BrokerResult<()> {
        if let Some(latest_run) = self
            .store
            .get_task_run(&run.run_id)
            .await
            .map_err(BrokerError::from)?
        {
            run.scope = latest_run.scope;
        }

        let target_ips = match &run.scope {
            TaskRunScope::Ips { ips } => ips.clone(),
            TaskRunScope::Nodes { .. } => {
                return Err(BrokerError::InvalidRequest(
                    "incremental refresh does not accept node scope".to_string(),
                ));
            }
            TaskRunScope::All => self
                .store
                .list_ip_records(&run.project_id)
                .await
                .map_err(BrokerError::from)?
                .into_iter()
                .map(|record| record.ip)
                .collect(),
        };

        if target_ips.is_empty() {
            self.complete_task_run(
                run,
                TaskRunStatus::Skipped,
                Some(serde_json::json!({ "reason": "no_target_ips" })),
                None,
                None,
            )
            .await?;
            return Ok(());
        }

        let target_ip_set = target_ips.iter().cloned().collect::<HashSet<_>>();
        let refresh = self
            .refresh_metadata_internal(
                &run.project_id,
                false,
                Some(&target_ip_set),
                Some(&run.run_id),
            )
            .await?;

        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "targeted_ips": target_ips.len(),
                "probed_ips": refresh.probed_ips,
                "geo_updated": refresh.geo_updated,
                "skipped_cached": refresh.skipped_cached,
            })),
            None,
            None,
        )
        .await
    }

    async fn execute_full_refresh_task(&self, run: &mut TaskRunRecord) -> BrokerResult<()> {
        self.mark_full_refresh_started(&run.project_id).await?;
        let refresh = self
            .refresh_metadata_internal(&run.project_id, true, None, Some(&run.run_id))
            .await;
        let refresh = refresh?;
        self.mark_full_refresh_finished(&run.project_id, now_epoch_sec())
            .await?;

        let targeted_ips = self
            .store
            .list_ip_records(&run.project_id)
            .await
            .map_err(BrokerError::from)?
            .len();

        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "targeted_ips": targeted_ips,
                "probed_ips": refresh.probed_ips,
                "geo_updated": refresh.geo_updated,
                "skipped_cached": refresh.skipped_cached,
            })),
            None,
            None,
        )
        .await
    }

    async fn enqueue_task_run(
        &self,
        project_id: &str,
        kind: TaskRunKind,
        trigger: TaskRunTrigger,
        scope: TaskRunScope,
    ) -> BrokerResult<TaskRunRecord> {
        let run = TaskRunRecord {
            run_id: ids::random_task_run_id(),
            project_id: project_id.to_string(),
            kind,
            trigger,
            status: TaskRunStatus::Queued,
            stage: TaskRunStage::Queued,
            progress_current: Some(0),
            progress_total: None,
            created_at: now_epoch_sec(),
            started_at: None,
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
            scope,
        };
        self.insert_task_run_and_emit(&run).await?;
        self.append_task_event(
            &run,
            TaskEventLevel::Info,
            TaskRunStage::Queued,
            "Task run queued.",
            None,
        )
        .await?;
        Ok(run)
    }

    async fn insert_skipped_task_run(
        &self,
        project_id: &str,
        kind: TaskRunKind,
        trigger: TaskRunTrigger,
        scope: TaskRunScope,
        summary_json: Option<serde_json::Value>,
    ) -> BrokerResult<TaskRunRecord> {
        let now = now_epoch_sec();
        let run = TaskRunRecord {
            run_id: ids::random_task_run_id(),
            project_id: project_id.to_string(),
            kind,
            trigger,
            status: TaskRunStatus::Skipped,
            stage: TaskRunStage::Completed,
            progress_current: Some(0),
            progress_total: Some(0),
            created_at: now,
            started_at: None,
            finished_at: Some(now),
            summary_json: summary_json.clone(),
            error_code: None,
            error_message: None,
            scope,
        };
        self.insert_task_run_and_emit(&run).await?;
        self.append_task_event(
            &run,
            TaskEventLevel::Warning,
            TaskRunStage::Completed,
            "Task run skipped.",
            summary_json,
        )
        .await?;
        Ok(run)
    }

    async fn insert_task_run_and_emit(&self, run: &TaskRunRecord) -> BrokerResult<()> {
        self.store
            .insert_task_run(run)
            .await
            .map_err(BrokerError::from)?;
        let _ = self
            .task_events
            .send(TaskBusEvent::RunUpsert(run.as_summary()));
        Ok(())
    }

    async fn update_task_run_and_emit(&self, run: &TaskRunRecord) -> BrokerResult<()> {
        self.store
            .update_task_run(run)
            .await
            .map_err(BrokerError::from)?;
        let _ = self
            .task_events
            .send(TaskBusEvent::RunUpsert(run.as_summary()));
        Ok(())
    }

    async fn append_task_event(
        &self,
        run: &TaskRunRecord,
        level: TaskEventLevel,
        stage: TaskRunStage,
        message: impl Into<String>,
        payload_json: Option<serde_json::Value>,
    ) -> BrokerResult<()> {
        let event = TaskRunEventRecord {
            event_id: ids::random_task_event_id(),
            run_id: run.run_id.clone(),
            project_id: run.project_id.clone(),
            at: now_epoch_sec(),
            level,
            stage,
            message: message.into(),
            payload_json,
        };
        self.store
            .insert_task_run_event(&event)
            .await
            .map_err(BrokerError::from)?;
        let _ = self.task_events.send(TaskBusEvent::RunEvent(event));
        Ok(())
    }

    async fn mark_task_running(
        &self,
        run: &mut TaskRunRecord,
        stage: TaskRunStage,
        progress_current: Option<u64>,
        progress_total: Option<u64>,
    ) -> BrokerResult<()> {
        run.status = TaskRunStatus::Running;
        run.stage = stage;
        run.progress_current = progress_current;
        run.progress_total = progress_total;
        if run.started_at.is_none() {
            run.started_at = Some(now_epoch_sec());
        }
        self.update_task_run_and_emit(run).await
    }

    async fn complete_task_run(
        &self,
        run: &mut TaskRunRecord,
        status: TaskRunStatus,
        summary_json: Option<serde_json::Value>,
        error_code: Option<String>,
        error_message: Option<String>,
    ) -> BrokerResult<()> {
        run.status = status;
        run.stage = TaskRunStage::Completed;
        run.progress_current = run.progress_total.or(run.progress_current);
        run.finished_at = Some(now_epoch_sec());
        run.summary_json = summary_json.clone();
        run.error_code = error_code;
        run.error_message = error_message;
        self.update_task_run_and_emit(run).await?;

        let level = match status {
            TaskRunStatus::Failed => TaskEventLevel::Error,
            TaskRunStatus::Skipped => TaskEventLevel::Warning,
            _ => TaskEventLevel::Info,
        };
        let message = match status {
            TaskRunStatus::Succeeded => "Task run completed successfully.",
            TaskRunStatus::Skipped => "Task run skipped.",
            TaskRunStatus::Failed => "Task run failed.",
            TaskRunStatus::Queued => "Task run queued.",
            TaskRunStatus::Running => "Task run is running.",
        };
        self.append_task_event(run, level, TaskRunStage::Completed, message, summary_json)
            .await
    }

    async fn fail_task_run(&self, run: &mut TaskRunRecord, error: BrokerError) -> BrokerResult<()> {
        let failed_at = now_epoch_sec();
        if run.trigger == TaskRunTrigger::Schedule {
            match run.kind {
                TaskRunKind::SubscriptionSync => {}
                TaskRunKind::MetadataRefreshFull => {
                    self.mark_full_refresh_failed(&run.project_id, failed_at)
                        .await?;
                }
                TaskRunKind::MetadataRefreshIncremental
                | TaskRunKind::ProxyMetadataRefresh
                | TaskRunKind::ProxyLatencyProbe => {}
            }
        }
        let error_code = error.code().to_string();
        let error_message = error.to_string();
        let error_details = error.details();
        self.complete_task_run(
            run,
            TaskRunStatus::Failed,
            Some(serde_json::json!({
                "error": {
                    "code": error_code,
                    "message": error_message,
                    "details": error_details,
                },
                "task": {
                    "run_id": run.run_id.clone(),
                    "project_id": run.project_id.clone(),
                    "kind": run.kind,
                    "trigger": run.trigger,
                    "scope": run.scope.clone(),
                },
            })),
            Some(error_code),
            Some(error_message),
        )
        .await
    }

    async fn update_task_stage_by_id(
        &self,
        run_id: &str,
        stage: TaskRunStage,
        progress_current: Option<u64>,
        progress_total: Option<u64>,
        message: &str,
        payload_json: Option<serde_json::Value>,
    ) -> BrokerResult<()> {
        let mut run = self
            .store
            .get_task_run(run_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::TaskRunNotFound)?;
        self.mark_task_running(&mut run, stage, progress_current, progress_total)
            .await?;
        self.append_task_event(&run, TaskEventLevel::Info, stage, message, payload_json)
            .await
    }

    async fn register_project_sync_source(
        &self,
        import_id: &str,
        project_id: &str,
        source: &SubscriptionSource,
    ) -> BrokerResult<()> {
        let now = now_epoch_sec();
        let mut config = self
            .store
            .get_proxy_import_sync_config(import_id)
            .await
            .map_err(BrokerError::from)?
            .unwrap_or(ProxyImportSyncConfig {
                import_id: import_id.to_string(),
                project_id: project_id.to_string(),
                source: source.clone(),
                enabled: true,
                sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                last_sync_due_at: None,
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: None,
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            });
        config.project_id = project_id.to_string();
        config.source = source.clone();
        config.enabled = true;
        config.sync_every_sec = DEFAULT_AUTO_SYNC_EVERY_SEC;
        config.full_refresh_every_sec = DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC;
        config.last_sync_due_at = Some(preserve_or_advance_due_at(
            config.last_sync_due_at,
            now,
            DEFAULT_AUTO_SYNC_EVERY_SEC,
        ));
        config.last_full_refresh_due_at = Some(seed_due_at_if_missing(
            config.last_full_refresh_due_at,
            now,
            DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
        ));
        config.updated_at = now;
        self.store
            .upsert_proxy_import_sync_config(&config)
            .await
            .map_err(BrokerError::from)
    }

    async fn sync_configs_for_project(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<ProxyImportSyncConfig>> {
        self.store
            .list_proxy_import_sync_configs_for_project(project_id)
            .await
            .map_err(BrokerError::from)
    }

    async fn mark_sync_started_for_imports(&self, import_ids: &[String]) -> BrokerResult<()> {
        let now = now_epoch_sec();
        for import_id in import_ids {
            let Some(mut config) = self
                .store
                .get_proxy_import_sync_config(import_id)
                .await
                .map_err(BrokerError::from)?
            else {
                continue;
            };
            config.last_sync_started_at = Some(now);
            config.updated_at = now;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_sync_finished_for_imports(
        &self,
        import_ids: &[String],
        finished_at: i64,
    ) -> BrokerResult<()> {
        for import_id in import_ids {
            let Some(mut config) = self
                .store
                .get_proxy_import_sync_config(import_id)
                .await
                .map_err(BrokerError::from)?
            else {
                continue;
            };
            config.last_sync_finished_at = Some(finished_at);
            config.last_sync_due_at = Some(finished_at + config.sync_every_sec as i64);
            config.updated_at = finished_at;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_sync_failed_for_imports(
        &self,
        import_ids: &[String],
        failed_at: i64,
    ) -> BrokerResult<()> {
        for import_id in import_ids {
            let Some(mut config) = self
                .store
                .get_proxy_import_sync_config(import_id)
                .await
                .map_err(BrokerError::from)?
            else {
                continue;
            };
            config.last_sync_due_at = Some(preserve_or_advance_due_at(
                config.last_sync_due_at,
                failed_at,
                config.sync_every_sec,
            ));
            config.updated_at = failed_at;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_started(&self, project_id: &str) -> BrokerResult<()> {
        let now = now_epoch_sec();
        for mut config in self.sync_configs_for_project(project_id).await? {
            config.last_full_refresh_started_at = Some(now);
            config.updated_at = now;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_failed(&self, project_id: &str, failed_at: i64) -> BrokerResult<()> {
        for mut config in self.sync_configs_for_project(project_id).await? {
            config.last_full_refresh_due_at = Some(preserve_or_advance_due_at(
                config.last_full_refresh_due_at,
                failed_at,
                config.full_refresh_every_sec,
            ));
            config.updated_at = failed_at;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_finished(
        &self,
        project_id: &str,
        finished_at: i64,
    ) -> BrokerResult<()> {
        for mut config in self.sync_configs_for_project(project_id).await? {
            config.last_full_refresh_finished_at = Some(finished_at);
            config.last_full_refresh_due_at =
                Some(finished_at + config.full_refresh_every_sec as i64);
            config.updated_at = finished_at;
            self.store
                .upsert_proxy_import_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    fn proxy_import_source_identity(
        &self,
        source: &SubscriptionSource,
    ) -> ProxyImportSourceIdentity {
        ProxyImportSourceIdentity::from_source(source)
    }

    fn proxy_import_id(
        &self,
        source_scope: &ProxyScope,
        source_identity: &ProxyImportSourceIdentity,
    ) -> String {
        ids::stable_import_id(&source_scope.key(), &source_identity.key())
    }

    fn proxy_inventory_node_id(&self, import_id: &str, node: &ProxyNode) -> String {
        ids::stable_proxy_inventory_node_id_for_proxy(
            import_id,
            &node.proxy_name,
            &node.proxy_type,
            &node.server,
            &node.raw_proxy,
        )
    }

    fn generated_manual_import_name(&self, nodes: &[ProxyNode]) -> Option<String> {
        let first = nodes.first()?.proxy_name.trim();
        if first.is_empty() {
            return None;
        }
        if nodes.len() == 1 {
            return Some(first.to_string());
        }
        Some(format!("{first} +{}", nodes.len() - 1))
    }

    fn resolve_import_name(
        &self,
        requested_name: Option<&str>,
        existing_import: Option<&ProxyImportRecord>,
        parsed_name: Option<&str>,
        generated_name: Option<&str>,
    ) -> ResolvedImportName {
        if let Some(name) = requested_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return ResolvedImportName {
                value: Some(name),
                source: Some(ResolvedImportNameSource::ExplicitInput),
            };
        }

        if let Some(name) = existing_import
            .and_then(|item| item.name.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return ResolvedImportName {
                value: Some(name),
                source: Some(ResolvedImportNameSource::ExistingImport),
            };
        }

        if let Some(name) = parsed_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return ResolvedImportName {
                value: Some(name),
                source: Some(ResolvedImportNameSource::ParsedSource),
            };
        }

        if let Some(name) = generated_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            return ResolvedImportName {
                value: Some(name),
                source: Some(ResolvedImportNameSource::Generated),
            };
        }

        ResolvedImportName {
            value: None,
            source: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn persist_imported_inventory(
        &self,
        source_scope: &ProxyScope,
        import_id: String,
        source_identity: ProxyImportSourceIdentity,
        import_kind: ProxyImportKind,
        requested_name: Option<&str>,
        parsed_name: Option<String>,
        subscription_metadata: Option<SubscriptionMetadata>,
        mut nodes: Vec<ProxyNode>,
        mut warnings: Vec<String>,
    ) -> BrokerResult<ImportedInventoryOutcome> {
        nodes = filter_malformed_proxy_nodes(nodes, &mut warnings);
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let existing_import = self
            .store
            .get_proxy_import(&import_id)
            .await
            .map_err(BrokerError::from)?;
        let existing_import_nodes = self
            .store
            .list_proxy_inventory_for_import(&import_id)
            .await
            .map_err(BrokerError::from)?;
        let existing_ips_by_proxy: HashMap<(String, String), Vec<String>> = existing_import_nodes
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
        let existing_by_node_id = existing_import_nodes
            .iter()
            .map(|item| (item.node_id.clone(), item.clone()))
            .collect::<HashMap<_, _>>();

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

        let distinct_ips = nodes
            .iter()
            .flat_map(|node| node.resolved_ips.iter().cloned())
            .collect::<HashSet<_>>();
        if distinct_ips.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let now = now_epoch_sec();
        let derived_name = if import_kind == ProxyImportKind::SingleNode {
            self.generated_manual_import_name(&nodes)
        } else {
            None
        };
        let resolved_name = self.resolve_import_name(
            requested_name,
            existing_import.as_ref(),
            parsed_name.as_deref(),
            derived_name.as_deref(),
        );
        let import_record = ProxyImportRecord {
            import_id: import_id.clone(),
            name: resolved_name.value.clone(),
            import_kind,
            source_scope: source_scope.clone(),
            source_identity,
            allocation_scope: existing_import
                .as_ref()
                .map(|item| item.allocation_scope.clone())
                .unwrap_or_else(|| source_scope.clone()),
            subscription_metadata,
            created_at: existing_import
                .as_ref()
                .map(|item| item.created_at)
                .unwrap_or(now),
            updated_at: now,
        };
        let inventory_nodes = nodes
            .into_iter()
            .map(|node| {
                let node_id = self.proxy_inventory_node_id(&import_id, &node);
                let created_at = existing_by_node_id
                    .get(&node_id)
                    .map(|item| item.created_at)
                    .unwrap_or(now);
                ProxyInventoryRecord {
                    import_id: import_id.clone(),
                    node_id,
                    source_scope: source_scope.clone(),
                    allocation_scope: import_record.allocation_scope.clone(),
                    proxy_name: node.proxy_name,
                    proxy_type: node.proxy_type,
                    server: node.server,
                    resolved_ips: node.resolved_ips,
                    raw_proxy: node.raw_proxy,
                    created_at,
                    updated_at: now,
                }
            })
            .collect::<Vec<_>>();

        self.store
            .replace_proxy_inventory_import(&import_record, &inventory_nodes)
            .await
            .map_err(BrokerError::from)?;

        Ok(ImportedInventoryOutcome {
            import_id,
            response: LoadSubscriptionResponse {
                loaded_proxies: inventory_nodes.len(),
                distinct_ips: distinct_ips.len(),
                resolved_name: import_record.name.clone(),
                resolved_name_source: resolved_name.source,
                subscription_metadata: import_record.subscription_metadata.clone(),
                warnings,
            },
        })
    }

    async fn import_inventory_scope_from_source(
        &self,
        source_scope: &ProxyScope,
        source: &SubscriptionSource,
        requested_name: Option<&str>,
    ) -> BrokerResult<ImportedInventoryOutcome> {
        let loaded = subscription::load_from_source(&self.http, source)
            .await
            .map_err(|err| match err {
                subscription::SubscriptionLoadError::SourceRead(message) => {
                    BrokerError::SubscriptionFetch(message)
                }
                subscription::SubscriptionLoadError::InvalidPayload(message) => {
                    BrokerError::SubscriptionInvalidDetail(message)
                }
            })?;

        if loaded.nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let source_identity = self.proxy_import_source_identity(source);
        let import_id = self.proxy_import_id(source_scope, &source_identity);
        self.persist_imported_inventory(
            source_scope,
            import_id,
            source_identity,
            ProxyImportKind::Subscription,
            requested_name,
            loaded.parsed_name,
            loaded.metadata,
            loaded.nodes,
            loaded.warnings,
        )
        .await
    }

    async fn import_inventory_scope_from_content(
        &self,
        source_scope: &ProxyScope,
        content: &str,
        requested_name: Option<&str>,
    ) -> BrokerResult<ImportedInventoryOutcome> {
        let loaded = subscription::load_from_content(content)
            .await
            .map_err(|_| BrokerError::SubscriptionInvalid)?;
        let import_id = ids::random_import_id();
        self.persist_imported_inventory(
            source_scope,
            import_id.clone(),
            ProxyImportSourceIdentity::manual(import_id),
            ProxyImportKind::SingleNode,
            requested_name,
            None,
            None,
            loaded.nodes,
            loaded.warnings,
        )
        .await
    }

    fn with_subscription_sync_context(error: BrokerError, import_id: &str) -> BrokerError {
        match error {
            BrokerError::SubscriptionInvalidDetail(message) => {
                BrokerError::SubscriptionInvalidDetail(format!(
                    "import_id `{import_id}` failed: {message}"
                ))
            }
            BrokerError::SubscriptionFetch(message) => {
                BrokerError::SubscriptionFetch(format!("import_id `{import_id}` failed: {message}"))
            }
            other => other,
        }
    }

    async fn compose_effective_proxy_nodes(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<ProxyNode>> {
        let records = self
            .compose_effective_proxy_runtime_records(project_id)
            .await?;
        let mut nodes = records
            .into_iter()
            .map(|item| ProxyNode {
                node_id: Some(item.node_id),
                proxy_name: item.proxy_name,
                proxy_type: item.proxy_type,
                server: item.server,
                resolved_ips: item.resolved_ips,
                raw_proxy: item.raw_proxy,
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.proxy_name.cmp(&right.proxy_name));
        Ok(nodes)
    }

    async fn compose_effective_proxy_inventory_records(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<ProxyInventoryRecord>> {
        let settings = self
            .get_project_proxy_settings_effective(project_id)
            .await?;
        let candidates = self
            .store
            .list_proxy_inventory()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .filter(|item| match &item.allocation_scope {
                ProxyScope::Global => settings.use_global_proxies,
                ProxyScope::Project {
                    project_id: allocated_project_id,
                } => allocated_project_id == project_id,
            })
            .collect::<Vec<_>>();
        let mut candidates = self.filter_malformed_inventory_records(
            project_id,
            candidates,
            "malformed proxy inventory node skipped from effective project inventory",
        );
        candidates.sort_by(|left, right| {
            left.proxy_name
                .cmp(&right.proxy_name)
                .then_with(|| compare_inventory_preference(project_id, left, right))
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        Ok(candidates)
    }

    async fn compose_effective_proxy_runtime_records(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<ProxyInventoryRecord>> {
        let mut candidates = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        candidates.sort_by(|left, right| compare_inventory_preference(project_id, left, right));

        let mut by_name = HashMap::new();
        for candidate in candidates {
            by_name
                .entry(candidate.proxy_name.clone())
                .or_insert(candidate);
        }

        let mut items = by_name.into_values().collect::<Vec<_>>();
        items.sort_by(|left, right| left.proxy_name.cmp(&right.proxy_name));
        Ok(items)
    }

    async fn compose_effective_session_nodes(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<ProxyNode>> {
        let effective_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?
            .into_iter()
            .map(|record| ProxyNode {
                node_id: Some(record.node_id),
                proxy_name: record.proxy_name,
                proxy_type: record.proxy_type,
                server: record.server,
                resolved_ips: record.resolved_ips,
                raw_proxy: record.raw_proxy,
            })
            .collect::<Vec<_>>();
        if !effective_nodes.is_empty() {
            return Ok(effective_nodes);
        }

        self.store
            .list_subscription(project_id)
            .await
            .map_err(BrokerError::from)
    }

    async fn rebuild_effective_project_locked(
        &self,
        project_id: &str,
    ) -> BrokerResult<Vec<String>> {
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        let inventory_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let nodes = self.compose_effective_proxy_nodes(project_id).await?;
        let existing_ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let existing_ip_map: HashMap<String, IpRecord> = existing_ip_records
            .into_iter()
            .map(|record| (record.ip.clone(), record))
            .collect();
        let existing_ip_keys: HashSet<String> = existing_ip_map.keys().cloned().collect();
        let existing_sessions = self.list_sessions_backfilled(project_id).await?;

        if nodes.is_empty() {
            log_unrestored_sessions(
                project_id,
                &existing_sessions,
                &[],
                "effective project rebuild kept persisted session but could not restore it because no effective nodes were available",
            );
            self.store
                .apply_subscription_snapshot(project_id, &[], &[], &[])
                .await
                .map_err(BrokerError::from)?;
            self.apply_project_restorable_sessions_locked(project_id, &[])
                .await?;
            return Ok(vec![]);
        }

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
        let valid_proxy_ip_pairs: HashSet<(String, String)> = nodes
            .iter()
            .flat_map(valid_proxy_ip_pairs_for_node)
            .collect();
        let active_sessions: Vec<SessionRecord> = if inventory_nodes.is_empty() {
            existing_sessions
                .iter()
                .filter(|session| {
                    valid_proxy_ip_pairs.contains(&(
                        session_runtime_key(session).to_string(),
                        session.selected_ip.clone(),
                    ))
                })
                .cloned()
                .collect()
        } else {
            existing_sessions
                .iter()
                .filter_map(|session| {
                    reselect_session_from_inventory(session, &inventory_nodes, &metadata_by_pair)
                })
                .collect()
        };
        log_unrestored_sessions(
            project_id,
            &existing_sessions,
            &active_sessions,
            "effective project rebuild kept persisted session but left it out of runtime restore",
        );
        let fresh_probe_records = filter_probe_records_by_pair(
            self.store
                .list_probe_records(project_id)
                .await
                .map_err(BrokerError::from)?,
            &valid_proxy_ip_pairs,
        );
        let mut next_ip_records = ip_map.values().cloned().collect::<Vec<_>>();
        clear_stale_probe_timestamps(&mut next_ip_records, &fresh_probe_records);

        self.apply_project_restorable_sessions_locked(project_id, &active_sessions)
            .await?;

        if let Err(err) = self
            .store
            .apply_subscription_snapshot(project_id, &nodes, &next_ip_records, &fresh_probe_records)
            .await
            .map_err(BrokerError::from)
        {
            if let Err(rollback_err) = self
                .apply_project_restorable_sessions_locked(project_id, &existing_sessions)
                .await
            {
                tracing::error!(
                    project_id,
                    error = %rollback_err,
                    "runtime rollback failed after subscription snapshot persistence error"
                );
                self.recover_runtime_desync_locked(project_id, &existing_sessions)
                    .await;
            }
            return Err(err);
        }

        if !active_sessions.is_empty() {
            self.store
                .insert_sessions(project_id, &active_sessions)
                .await
                .map_err(BrokerError::from)?;
        }
        self.cleanup_shared_runtime_if_idle_locked().await;

        let mut new_ips = valid_ips
            .difference(&existing_ip_keys)
            .cloned()
            .collect::<Vec<_>>();
        new_ips.sort();
        Ok(new_ips)
    }

    async fn rebuild_projects(&self, project_ids: &HashSet<String>) -> BrokerResult<()> {
        let mut project_ids = project_ids.iter().cloned().collect::<Vec<_>>();
        project_ids.sort();
        for project_id in project_ids {
            let _project_guard = self.lock_project(&project_id).await;
            self.rebuild_effective_project_locked(&project_id).await?;
        }
        Ok(())
    }

    async fn inventory_item_from_record(
        &self,
        record: ProxyInventoryRecord,
    ) -> BrokerResult<ProxyInventoryItem> {
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let effective_project_ids =
            self.effective_project_ids_for_record(&record, &projects, &settings);
        Ok(ProxyInventoryItem {
            import_id: record.import_id,
            node_id: record.node_id,
            proxy_name: record.proxy_name,
            proxy_type: record.proxy_type,
            server: record.server,
            resolved_ips: record.resolved_ips,
            source_scope: record.source_scope,
            allocation_scope: record.allocation_scope,
            effective_project_ids,
        })
    }

    async fn proxy_import_item_from_record(
        &self,
        record: ProxyImportRecord,
    ) -> BrokerResult<ProxyImportItem> {
        let summary_project_id = match &record.allocation_scope {
            ProxyScope::Global => GLOBAL_RUNTIME_PROJECT_ID,
            ProxyScope::Project { project_id } => project_id.as_str(),
        };
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let effective_project_ids = match &record.allocation_scope {
            ProxyScope::Global => projects
                .into_iter()
                .filter(|project_id| {
                    settings
                        .get(project_id.as_str())
                        .map(|item| item.use_global_proxies)
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>(),
            ProxyScope::Project { project_id } => vec![project_id.clone()],
        };
        let nodes = self.filter_malformed_inventory_records(
            summary_project_id,
            self.store
                .list_proxy_inventory_for_import(&record.import_id)
                .await
                .map_err(BrokerError::from)?,
            "malformed proxy inventory node skipped from proxy import summary",
        );
        let distinct_ip_count = nodes
            .iter()
            .flat_map(|item| item.resolved_ips.iter().cloned())
            .collect::<HashSet<_>>()
            .len();
        Ok(ProxyImportItem {
            import_id: record.import_id,
            name: record.name,
            import_kind: record.import_kind,
            source_scope: record.source_scope,
            source_identity: record.source_identity,
            allocation_scope: record.allocation_scope,
            proxy_count: nodes.len(),
            distinct_ip_count,
            effective_project_ids,
            subscription_metadata: record.subscription_metadata,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    async fn build_proxy_catalog_response(
        &self,
        view: &str,
        project_id: Option<&str>,
        records: Vec<ProxyInventoryRecord>,
    ) -> BrokerResult<ProxyCatalogResponse> {
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let legacy_metadata = self.load_legacy_project_metadata(&projects).await?;
        let import_records = self
            .store
            .list_proxy_imports()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.import_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let samples_by_node_ip = self
            .store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<(String, String), Vec<ProxyNodeProbeSampleRecord>>::new(),
                |mut acc, record| {
                    acc.entry((record.node_id.clone(), record.ip.clone()))
                        .or_default()
                        .push(record);
                    acc
                },
            );
        let metadata_by_node = self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<String, Vec<ProxyNodeMetadataRecord>>::new(),
                |mut acc, record| {
                    let record = attach_recent_probe_samples(
                        sanitize_proxy_node_metadata_record(record),
                        &samples_by_node_ip,
                    );
                    acc.entry(record.node_id.clone()).or_default().push(record);
                    acc
                },
            );

        let mut records_by_import = HashMap::<String, Vec<ProxyInventoryRecord>>::new();
        for record in records {
            records_by_import
                .entry(record.import_id.clone())
                .or_default()
                .push(record);
        }

        let mut groups = Vec::new();
        let mut import_ids = records_by_import.keys().cloned().collect::<Vec<_>>();
        import_ids.sort();
        for import_id in import_ids {
            let mut nodes = records_by_import.remove(&import_id).unwrap_or_default();
            nodes.sort_by(|left, right| left.proxy_name.cmp(&right.proxy_name));

            let Some(import_record) = import_records.get(&import_id).cloned() else {
                continue;
            };
            let import_item = self.proxy_import_item_from_record(import_record).await?;
            let min_probe_updated_at =
                now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64);
            let node_items = nodes
                .into_iter()
                .map(|record| {
                    let effective_project_ids =
                        self.effective_project_ids_for_record(&record, &projects, &settings);
                    let existing_ip_metadata = metadata_by_node
                        .get(&record.node_id)
                        .cloned()
                        .unwrap_or_default();
                    let mut ip_metadata = Vec::new();
                    for ip in &record.resolved_ips {
                        let existing = existing_ip_metadata
                            .iter()
                            .find(|item| item.ip == *ip)
                            .cloned();
                        if let Some(metadata) = self.merge_proxy_node_metadata_with_legacy(
                            &record,
                            ip,
                            &effective_project_ids,
                            &legacy_metadata,
                            existing,
                        ) {
                            ip_metadata.push(metadata);
                        }
                    }
                    ip_metadata.sort_by(|left, right| left.ip.cmp(&right.ip));
                    let can_open_session = view == "project"
                        && ip_metadata.iter().any(|item| {
                            proxy_node_metadata_is_fresh_healthy(item, min_probe_updated_at)
                        });
                    ProxyCatalogNodeItem {
                        import_id: record.import_id,
                        node_id: record.node_id,
                        proxy_name: record.proxy_name,
                        proxy_type: record.proxy_type,
                        server: record.server,
                        resolved_ips: record.resolved_ips.clone(),
                        source_scope: record.source_scope,
                        allocation_scope: record.allocation_scope,
                        effective_project_ids,
                        primary_ip: record.resolved_ips.first().cloned(),
                        ip_metadata,
                        can_open_session,
                    }
                })
                .collect::<Vec<_>>();
            groups.push(ProxyCatalogGroupItem {
                import: import_item,
                nodes: node_items,
            });
        }

        Ok(ProxyCatalogResponse {
            view: view.to_string(),
            project_id: project_id.map(ToOwned::to_owned),
            groups,
        })
    }

    async fn load_legacy_project_metadata(
        &self,
        projects: &[String],
    ) -> BrokerResult<HashMap<String, LegacyProjectMetadata>> {
        let mut metadata = HashMap::new();
        for project_id in projects {
            let ip_records = self
                .store
                .list_ip_records(project_id)
                .await
                .map_err(BrokerError::from)?
                .into_iter()
                .map(sanitize_ip_record)
                .map(|record| (record.ip.clone(), record))
                .collect::<HashMap<_, _>>();
            let mut probe_records = HashMap::<(String, String), Vec<ProbeRecord>>::new();
            for record in self
                .store
                .list_probe_records(project_id)
                .await
                .map_err(BrokerError::from)?
            {
                probe_records
                    .entry((record.proxy_name.clone(), record.ip.clone()))
                    .or_default()
                    .push(record);
            }
            metadata.insert(
                project_id.clone(),
                LegacyProjectMetadata {
                    ip_records,
                    probe_records,
                },
            );
        }
        Ok(metadata)
    }

    fn backfill_proxy_node_metadata(
        &self,
        record: &ProxyInventoryRecord,
        ip: &str,
        effective_project_ids: &[String],
        legacy_metadata: &HashMap<String, LegacyProjectMetadata>,
    ) -> Option<ProxyNodeMetadataRecord> {
        for project_id in effective_project_ids {
            let Some(project_metadata) = legacy_metadata.get(project_id) else {
                continue;
            };
            let ip_record = project_metadata.ip_records.get(ip);
            let probe_records = project_metadata
                .probe_records
                .get(&(record.proxy_name.clone(), ip.to_string()));
            if ip_record.is_none() && probe_records.is_none() {
                continue;
            }

            let mut sorted_probe_records = probe_records.cloned().unwrap_or_default();
            sorted_probe_records.sort_by(|left, right| {
                left.updated_at
                    .cmp(&right.updated_at)
                    .then_with(|| left.target_url.cmp(&right.target_url))
            });

            let successes = sorted_probe_records
                .iter()
                .filter_map(|probe| probe.ok.then_some(probe.latency_ms).flatten())
                .collect::<Vec<_>>();
            let last_probe_samples = if sorted_probe_records.is_empty() {
                Vec::new()
            } else {
                sorted_probe_records
                    .iter()
                    .map(|probe| if probe.ok { probe.latency_ms } else { None })
                    .collect::<Vec<_>>()
            };
            let probe_updated_at = sorted_probe_records.last().map(|probe| probe.updated_at);
            let updated_at = [
                ip_record.and_then(|item| item.geo_updated_at),
                ip_record.and_then(|item| item.probe_updated_at),
                probe_updated_at,
            ]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or(0);

            return Some(ProxyNodeMetadataRecord {
                node_id: record.node_id.clone(),
                ip: ip.to_string(),
                country_code: ip_record
                    .and_then(|item| normalize_country_code(item.country_code.as_deref())),
                country_name: ip_record.and_then(|item| item.country_name.clone()),
                region_name: ip_record.and_then(|item| item.region_name.clone()),
                city: ip_record.and_then(|item| item.city.clone()),
                geo_source: ip_record.and_then(|item| item.geo_source.clone()),
                probe_updated_at,
                geo_updated_at: ip_record.and_then(|item| item.geo_updated_at),
                last_probe_ok: sorted_probe_records.last().map(|probe| probe.ok),
                last_latency_ms: sorted_probe_records
                    .last()
                    .and_then(|probe| probe.latency_ms),
                median_latency_ms: median_success_latency(&successes),
                last_probe_samples,
                recent_probe_samples: Vec::new(),
                updated_at,
            });
        }
        None
    }

    fn merge_proxy_node_metadata_with_legacy(
        &self,
        record: &ProxyInventoryRecord,
        ip: &str,
        effective_project_ids: &[String],
        legacy_metadata: &HashMap<String, LegacyProjectMetadata>,
        existing: Option<ProxyNodeMetadataRecord>,
    ) -> Option<ProxyNodeMetadataRecord> {
        match (
            existing,
            self.backfill_proxy_node_metadata(record, ip, effective_project_ids, legacy_metadata),
        ) {
            (Some(existing), Some(backfilled)) => Some(sanitize_proxy_node_metadata_record(
                merge_backfilled_proxy_node_metadata(backfilled, Some(&existing), None),
            )),
            (Some(existing), None) => Some(existing),
            (None, Some(backfilled)) => Some(sanitize_proxy_node_metadata_record(backfilled)),
            (None, None) => None,
        }
    }

    async fn upsert_project_proxy_node_metadata_from_legacy_records(
        &self,
        project_id: &str,
        target_ips: Option<&HashSet<String>>,
    ) -> BrokerResult<usize> {
        let effective_records = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        if effective_records.is_empty() {
            return Ok(0);
        }

        let effective_project_ids = vec![project_id.to_string()];
        let legacy_metadata = self
            .load_legacy_project_metadata(&effective_project_ids)
            .await?;
        let existing_by_pair = self.proxy_node_metadata_by_pair().await?;
        let recent_samples_by_pair = self.recent_proxy_node_probe_samples_by_pair().await?;
        let mut updates = Vec::new();

        for record in &effective_records {
            for ip in &record.resolved_ips {
                if !ip_in_scope(ip, target_ips) {
                    continue;
                }
                let Some(backfilled) = self.backfill_proxy_node_metadata(
                    record,
                    ip,
                    &effective_project_ids,
                    &legacy_metadata,
                ) else {
                    continue;
                };
                let key = (record.node_id.clone(), ip.clone());
                let existing = existing_by_pair.get(&key);
                let merged = merge_backfilled_proxy_node_metadata(
                    backfilled,
                    existing,
                    recent_samples_by_pair.get(&key).cloned(),
                );
                if proxy_node_metadata_has_observation(&merged) {
                    updates.push(sanitize_proxy_node_metadata_record(merged));
                }
            }
        }

        if updates.is_empty() {
            return Ok(0);
        }
        let updated = updates.len();
        self.store
            .upsert_proxy_node_metadata(&updates)
            .await
            .map_err(BrokerError::from)?;
        Ok(updated)
    }

    pub async fn list_proxy_catalog(
        &self,
        query: &ProxyCatalogQuery,
    ) -> BrokerResult<ProxyCatalogResponse> {
        let view = query.view.as_deref().unwrap_or("global");
        match view {
            "global" => {
                let records = self.filter_malformed_inventory_records(
                    GLOBAL_RUNTIME_PROJECT_ID,
                    self.store
                        .list_proxy_inventory()
                        .await
                        .map_err(BrokerError::from)?,
                    "malformed proxy inventory node skipped from global proxy catalog",
                );
                self.build_proxy_catalog_response("global", None, records)
                    .await
            }
            "project" => {
                let Some(project_id) = query.project_id.as_deref() else {
                    return Err(BrokerError::InvalidRequest(
                        "project_id is required when view=project".to_string(),
                    ));
                };
                self.require_project_exists(project_id).await?;
                let records = self
                    .compose_effective_proxy_inventory_records(project_id)
                    .await?;
                self.build_proxy_catalog_response("project", Some(project_id), records)
                    .await
            }
            other => Err(BrokerError::InvalidRequest(format!(
                "unsupported proxy catalog view `{other}`"
            ))),
        }
    }

    async fn resolve_proxy_operation_nodes(
        &self,
        view: &str,
        project_id: Option<&str>,
        node_ids: &[String],
    ) -> BrokerResult<Vec<ProxyInventoryRecord>> {
        if node_ids.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "node_ids must not be empty".to_string(),
            ));
        }

        let mut unique_node_ids = node_ids
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        unique_node_ids.sort();
        unique_node_ids.dedup();
        if unique_node_ids.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "node_ids must not be empty".to_string(),
            ));
        }

        let records = match view {
            "global" => self.filter_malformed_inventory_records(
                GLOBAL_RUNTIME_PROJECT_ID,
                self.store
                    .list_proxy_inventory()
                    .await
                    .map_err(BrokerError::from)?,
                "malformed proxy inventory node skipped from proxy operation target resolution",
            ),
            "project" => {
                let Some(project_id) = project_id else {
                    return Err(BrokerError::InvalidRequest(
                        "project_id is required when view=project".to_string(),
                    ));
                };
                self.require_project_exists(project_id).await?;
                self.compose_effective_proxy_inventory_records(project_id)
                    .await?
            }
            other => {
                return Err(BrokerError::InvalidRequest(format!(
                    "unsupported proxy operation view `{other}`"
                )));
            }
        };

        let record_by_id = records
            .into_iter()
            .map(|record| (record.node_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let mut resolved = Vec::with_capacity(unique_node_ids.len());
        for node_id in unique_node_ids {
            let Some(record) = record_by_id.get(&node_id) else {
                return Err(BrokerError::ProxyInventoryNodeNotFound);
            };
            resolved.push(record.clone());
        }
        Ok(resolved)
    }

    fn proxy_operation_project_id(view: &str, project_id: Option<&str>) -> BrokerResult<String> {
        match view {
            "global" => Ok(GLOBAL_RUNTIME_PROJECT_ID.to_string()),
            "project" => project_id.map(ToOwned::to_owned).ok_or_else(|| {
                BrokerError::InvalidRequest("project_id is required when view=project".to_string())
            }),
            other => Err(BrokerError::InvalidRequest(format!(
                "unsupported proxy operation view `{other}`"
            ))),
        }
    }

    pub async fn queue_proxy_metadata_refresh(
        &self,
        request: &ProxyOperationRequest,
    ) -> BrokerResult<ProxyOperationAcceptedResponse> {
        let view = request.view.trim();
        let records = self
            .resolve_proxy_operation_nodes(view, request.project_id.as_deref(), &request.node_ids)
            .await?;
        let run = self
            .enqueue_task_run(
                &Self::proxy_operation_project_id(view, request.project_id.as_deref())?,
                TaskRunKind::ProxyMetadataRefresh,
                TaskRunTrigger::Operator,
                TaskRunScope::Nodes {
                    node_ids: records.into_iter().map(|record| record.node_id).collect(),
                },
            )
            .await?;
        Ok(ProxyOperationAcceptedResponse { run_id: run.run_id })
    }

    pub async fn queue_proxy_latency_probe(
        &self,
        request: &ProxyOperationRequest,
    ) -> BrokerResult<ProxyOperationAcceptedResponse> {
        let _probe_queue_guard = self.proxy_probe_queue_lock.lock().await;
        let view = request.view.trim();
        let records = self
            .resolve_proxy_operation_nodes(view, request.project_id.as_deref(), &request.node_ids)
            .await?;
        let active_node_ids = self.active_proxy_probe_node_ids().await?;
        let requested_nodes = records.len();
        let (ignored_records, target_records): (Vec<_>, Vec<_>) = records
            .into_iter()
            .partition(|record| active_node_ids.contains(&record.node_id));
        let ignored_node_ids = ignored_records
            .iter()
            .map(|record| record.node_id.clone())
            .collect::<Vec<_>>();
        if target_records
            .iter()
            .any(|record| record.resolved_ips.is_empty())
        {
            return Err(BrokerError::SubscriptionInvalid);
        }
        let project_id = Self::proxy_operation_project_id(view, request.project_id.as_deref())?;
        if target_records.is_empty() {
            let run = self
                .insert_skipped_task_run(
                    &project_id,
                    TaskRunKind::ProxyLatencyProbe,
                    TaskRunTrigger::Operator,
                    TaskRunScope::Nodes {
                        node_ids: ignored_node_ids.clone(),
                    },
                    Some(serde_json::json!({
                        "reason": "all_nodes_already_probing",
                        "requested_nodes": requested_nodes,
                        "ignored_nodes": ignored_node_ids.len(),
                        "ignored_node_ids": ignored_node_ids,
                    })),
                )
                .await?;
            return Ok(ProxyOperationAcceptedResponse { run_id: run.run_id });
        }
        let run = self
            .enqueue_task_run(
                &project_id,
                TaskRunKind::ProxyLatencyProbe,
                TaskRunTrigger::Operator,
                TaskRunScope::Nodes {
                    node_ids: target_records
                        .iter()
                        .map(|record| record.node_id.clone())
                        .collect(),
                },
            )
            .await?;
        if !ignored_node_ids.is_empty() {
            self.append_task_event(
                &run,
                TaskEventLevel::Warning,
                TaskRunStage::Queued,
                "Ignored proxy nodes that are already being probed.",
                Some(serde_json::json!({
                    "ignored_nodes": ignored_node_ids.len(),
                    "ignored_node_ids": ignored_node_ids,
                    "targeted_nodes": target_records.len(),
                    "requested_nodes": requested_nodes,
                })),
            )
            .await?;
        }
        Ok(ProxyOperationAcceptedResponse { run_id: run.run_id })
    }

    async fn resolve_proxy_operation_nodes_for_run(
        &self,
        run: &TaskRunRecord,
    ) -> BrokerResult<Vec<ProxyInventoryRecord>> {
        let view = if run.project_id == GLOBAL_RUNTIME_PROJECT_ID {
            "global"
        } else {
            "project"
        };
        match &run.scope {
            TaskRunScope::Nodes { node_ids } => {
                self.resolve_proxy_operation_nodes(
                    view,
                    (view == "project").then_some(run.project_id.as_str()),
                    node_ids,
                )
                .await
            }
            TaskRunScope::All => match view {
                "global" => {
                    let subscription_import_ids = if run.kind == TaskRunKind::ProxyLatencyProbe
                        && run.trigger == TaskRunTrigger::Schedule
                    {
                        Some(self.subscription_import_ids().await?)
                    } else {
                        None
                    };
                    Ok(self
                        .filter_malformed_inventory_records(
                            GLOBAL_RUNTIME_PROJECT_ID,
                            self.store
                                .list_proxy_inventory()
                                .await
                                .map_err(BrokerError::from)?,
                            "malformed proxy inventory node skipped from proxy operation all-node scope",
                        )
                        .into_iter()
                        .filter(|record| {
                            subscription_import_ids
                                .as_ref()
                                .map(|ids| ids.contains(&record.import_id))
                                .unwrap_or(true)
                        })
                        .filter(|record| !record.resolved_ips.is_empty())
                        .collect())
                }
                "project" => Ok(self
                    .compose_effective_proxy_inventory_records(&run.project_id)
                    .await?
                    .into_iter()
                    .filter(|record| !record.resolved_ips.is_empty())
                    .collect()),
                _ => unreachable!("proxy run view should be known"),
            },
            TaskRunScope::Ips { .. } => Err(BrokerError::InvalidRequest(
                "proxy operation does not accept ip scope".to_string(),
            )),
        }
    }

    async fn execute_proxy_metadata_refresh_task(
        &self,
        run: &mut TaskRunRecord,
    ) -> BrokerResult<()> {
        let nodes = self.resolve_proxy_operation_nodes_for_run(run).await?;
        if nodes.is_empty() {
            self.complete_task_run(
                run,
                TaskRunStatus::Skipped,
                Some(serde_json::json!({ "reason": "no_target_nodes" })),
                None,
                None,
            )
            .await?;
            return Ok(());
        }

        self.mark_task_running(
            run,
            TaskRunStage::GeoEnrichment,
            Some(0),
            Some(nodes.len() as u64),
        )
        .await?;

        let existing = self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| ((record.node_id.clone(), record.ip.clone()), record))
            .collect::<HashMap<_, _>>();
        let now = now_epoch_sec();

        let mut ip_records = nodes
            .iter()
            .filter_map(|node| {
                let ip = node.resolved_ips.first()?.clone();
                let existing = existing.get(&(node.node_id.clone(), ip.clone()));
                Some(IpRecord {
                    ip,
                    country_code: existing
                        .and_then(|record| normalize_country_code(record.country_code.as_deref())),
                    country_name: existing.and_then(|record| record.country_name.clone()),
                    region_name: existing.and_then(|record| record.region_name.clone()),
                    city: existing.and_then(|record| record.city.clone()),
                    geo_source: existing.and_then(|record| record.geo_source.clone()),
                    probe_updated_at: existing.and_then(|record| record.probe_updated_at),
                    geo_updated_at: existing.and_then(|record| record.geo_updated_at),
                    last_used_at: None,
                })
            })
            .collect::<Vec<_>>();
        let geo_updated = self
            .refresh_geo_records(GLOBAL_RUNTIME_PROJECT_ID, true, now, &mut ip_records, None)
            .await?;

        let by_ip = ip_records
            .into_iter()
            .map(|record| (record.ip.clone(), record))
            .collect::<HashMap<_, _>>();
        let mut updated_records = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let Some(primary_ip) = node.resolved_ips.first() else {
                continue;
            };
            let geo = by_ip.get(primary_ip);
            let previous = existing.get(&(node.node_id.clone(), primary_ip.clone()));
            updated_records.push(ProxyNodeMetadataRecord {
                node_id: node.node_id.clone(),
                ip: primary_ip.clone(),
                country_code: geo
                    .and_then(|record| normalize_country_code(record.country_code.as_deref())),
                country_name: geo.and_then(|record| record.country_name.clone()),
                region_name: geo.and_then(|record| record.region_name.clone()),
                city: geo.and_then(|record| record.city.clone()),
                geo_source: geo.and_then(|record| record.geo_source.clone()),
                probe_updated_at: previous.and_then(|record| record.probe_updated_at),
                geo_updated_at: geo.and_then(|record| record.geo_updated_at),
                last_probe_ok: previous.and_then(|record| record.last_probe_ok),
                last_latency_ms: previous.and_then(|record| record.last_latency_ms),
                median_latency_ms: previous.and_then(|record| record.median_latency_ms),
                last_probe_samples: previous
                    .map(|record| record.last_probe_samples.clone())
                    .unwrap_or_default(),
                recent_probe_samples: previous
                    .map(|record| record.recent_probe_samples.clone())
                    .unwrap_or_default(),
                updated_at: now,
            });
            run.progress_current = Some((index + 1) as u64);
            self.update_task_run_and_emit(run).await?;
            self.append_task_event(
                run,
                TaskEventLevel::Info,
                TaskRunStage::GeoEnrichment,
                format!("Refreshed node metadata for {}.", node.proxy_name),
                Some(serde_json::json!({
                    "node_id": node.node_id,
                    "proxy_name": node.proxy_name,
                    "ip": primary_ip,
                })),
            )
            .await?;
        }
        self.store
            .upsert_proxy_node_metadata(&updated_records)
            .await
            .map_err(BrokerError::from)?;
        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "targeted_nodes": updated_records.len(),
                "geo_updated": geo_updated,
            })),
            None,
            None,
        )
        .await
    }

    async fn execute_proxy_latency_probe_task(&self, run: &mut TaskRunRecord) -> BrokerResult<()> {
        let nodes = self.resolve_proxy_operation_nodes_for_run(run).await?;
        if nodes.is_empty() {
            self.complete_task_run(
                run,
                TaskRunStatus::Skipped,
                Some(serde_json::json!({ "reason": "no_target_nodes" })),
                None,
                None,
            )
            .await?;
            return Ok(());
        }
        if nodes.iter().any(|node| node.resolved_ips.is_empty()) {
            return Err(BrokerError::SubscriptionInvalid);
        }

        self.apply_shared_runtime_config(None, None, true).await?;
        let probe_pairs = nodes
            .iter()
            .flat_map(|node| {
                node.resolved_ips
                    .iter()
                    .cloned()
                    .map(|ip| (node.clone(), ip))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let total_samples = probe_pairs.len() * PROXY_PROBE_ROUNDS;
        self.mark_task_running(
            run,
            TaskRunStage::Probing,
            Some(0),
            Some(total_samples as u64),
        )
        .await?;

        let existing = self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| ((record.node_id.clone(), record.ip.clone()), record))
            .collect::<HashMap<_, _>>();
        let probe_target = self.options.probe_targets.first().cloned().ok_or_else(|| {
            BrokerError::InvalidRequest("probe target is not configured".to_string())
        })?;
        let timeout_ms = self.options.probe_timeout_ms;
        let mut progress = 0u64;
        let mut failed_samples = 0usize;
        let mut samples_by_pair = HashMap::<(String, String), Vec<Option<u64>>>::new();

        for round in 0..PROXY_PROBE_ROUNDS {
            let mut probe_stream = stream::iter(probe_pairs.clone())
                .map(|(node, ip)| {
                    let probe_target = probe_target.clone();
                    async move {
                        let runtime_alias = dedicated_ip_proxy_name(&node.node_id, &ip);
                        let sample = self
                            .runtime
                            .measure_proxy_delay(
                                GLOBAL_RUNTIME_PROJECT_ID,
                                &runtime_alias,
                                &probe_target,
                                timeout_ms,
                            )
                            .await
                            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))?;
                        Ok::<_, BrokerError>((node, ip, sample))
                    }
                })
                .buffer_unordered(self.options.probe_concurrency.max(1));

            while let Some(result) = probe_stream.next().await {
                let (node, ip, sample) = result?;
                let sampled_at = now_epoch_sec();
                if sample.is_none() {
                    failed_samples += 1;
                }
                samples_by_pair
                    .entry((node.node_id.clone(), ip.clone()))
                    .or_default()
                    .push(sample);
                progress += 1;
                self.persist_proxy_probe_sample(
                    &node,
                    &ip,
                    &probe_target,
                    sample,
                    sampled_at,
                    existing.get(&(node.node_id.clone(), ip.clone())),
                )
                .await?;
                run.progress_current = Some(progress);
                self.update_task_run_and_emit(run).await?;
                self.append_task_event(
                    run,
                    TaskEventLevel::Info,
                    TaskRunStage::Probing,
                    format!("Probe sample finished for {}.", node.proxy_name),
                    Some(serde_json::json!({
                        "node_id": node.node_id,
                        "proxy_name": node.proxy_name,
                        "ip": ip,
                        "round": round + 1,
                        "sample_ms": sample,
                        "progress_current": progress,
                        "progress_total": total_samples,
                    })),
                )
                .await?;
            }
        }

        let mut failed_nodes = 0usize;
        for node in &nodes {
            let node_failed = node.resolved_ips.iter().all(|ip| {
                samples_by_pair
                    .get(&(node.node_id.clone(), ip.clone()))
                    .map(|samples| samples.iter().all(Option::is_none))
                    .unwrap_or(true)
            });
            if node_failed {
                failed_nodes += 1;
            }
        }
        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "targeted_nodes": nodes.len(),
                "targeted_ips": probe_pairs.len(),
                "samples": total_samples,
                "failed_samples": failed_samples,
                "failed_nodes": failed_nodes,
                "rounds": PROXY_PROBE_ROUNDS,
            })),
            None,
            None,
        )
        .await?;
        self.cleanup_shared_runtime_if_idle().await;
        Ok(())
    }

    async fn persist_proxy_probe_sample(
        &self,
        node: &ProxyInventoryRecord,
        primary_ip: &str,
        probe_target: &str,
        sample: Option<u64>,
        sampled_at: i64,
        previous: Option<&ProxyNodeMetadataRecord>,
    ) -> BrokerResult<()> {
        let sample_record = ProxyNodeProbeSampleRecord {
            node_id: node.node_id.clone(),
            ip: primary_ip.to_string(),
            target_url: probe_target.to_string(),
            ok: sample.is_some(),
            latency_ms: sample,
            sampled_at,
        };
        self.store
            .insert_proxy_node_probe_samples(std::slice::from_ref(&sample_record))
            .await
            .map_err(BrokerError::from)?;

        let recent_samples = self
            .store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .filter(|record| record.node_id == node.node_id && record.ip == primary_ip)
            .collect::<Vec<_>>();
        let successes = recent_samples
            .iter()
            .filter_map(|record| record.ok.then_some(record.latency_ms).flatten())
            .collect::<Vec<_>>();
        let last_probe_samples = recent_samples
            .iter()
            .rev()
            .map(|record| record.ok.then_some(record.latency_ms).flatten())
            .collect::<Vec<_>>();
        let latest = recent_samples.first().cloned().unwrap_or(sample_record);
        let metadata = ProxyNodeMetadataRecord {
            node_id: node.node_id.clone(),
            ip: primary_ip.to_string(),
            country_code: previous
                .and_then(|record| normalize_country_code(record.country_code.as_deref())),
            country_name: previous.and_then(|record| record.country_name.clone()),
            region_name: previous.and_then(|record| record.region_name.clone()),
            city: previous.and_then(|record| record.city.clone()),
            geo_source: previous.and_then(|record| record.geo_source.clone()),
            probe_updated_at: Some(latest.sampled_at),
            geo_updated_at: previous.and_then(|record| record.geo_updated_at),
            last_probe_ok: Some(latest.ok),
            last_latency_ms: latest.latency_ms,
            median_latency_ms: median_success_latency(&successes),
            last_probe_samples,
            recent_probe_samples: recent_samples,
            updated_at: latest.sampled_at,
        };
        self.store
            .upsert_proxy_node_metadata(&[metadata])
            .await
            .map_err(BrokerError::from)?;
        Ok(())
    }

    pub async fn load_subscription(
        &self,
        project_id: &str,
        source: &crate::models::SubscriptionSource,
    ) -> BrokerResult<LoadSubscriptionResponse> {
        self.load_subscription_request(
            project_id,
            &crate::models::LoadSubscriptionRequest {
                name: None,
                source: Some(source.clone()),
                content: None,
            },
        )
        .await
    }

    pub async fn load_subscription_request(
        &self,
        project_id: &str,
        request: &crate::models::LoadSubscriptionRequest,
    ) -> BrokerResult<LoadSubscriptionResponse> {
        let (outcome, source) = self
            .load_subscription_request_internal(project_id, request)
            .await?;
        let mut response = outcome.response;
        if let Some(source) = source
            && let Err(err) = self
                .register_post_load_bookkeeping(
                    project_id,
                    &outcome.import_id,
                    &source,
                    &outcome.new_ips,
                )
                .await
        {
            tracing::warn!(
                project_id,
                error = %err,
                "post-load task bookkeeping failed after successful subscription import"
            );
            response.warnings.push(format!(
                "Imported subscription, but automatic task bookkeeping failed: {err}"
            ));
        }
        Ok(response)
    }

    pub async fn refresh(
        &self,
        project_id: &str,
        request: &RefreshRequest,
    ) -> BrokerResult<RefreshResponse> {
        self.refresh_metadata_internal(project_id, request.force, None, None)
            .await
    }

    async fn load_subscription_request_internal(
        &self,
        project_id: &str,
        request: &LoadSubscriptionRequest,
    ) -> BrokerResult<(LoadSubscriptionOutcome, Option<SubscriptionSource>)> {
        match (&request.source, request.content.as_deref()) {
            (Some(source), None) => Ok((
                self.load_subscription_internal(project_id, source, request.name.as_deref())
                    .await?,
                Some(source.clone()),
            )),
            (None, Some(content)) => Ok((
                self.load_manual_proxy_group_internal(project_id, content, request.name.as_deref())
                    .await?,
                None,
            )),
            (Some(_), Some(_)) => Err(BrokerError::InvalidRequest(
                "provide either `source` or `content`, not both".to_string(),
            )),
            (None, None) => Err(BrokerError::InvalidRequest(
                "either `source` or `content` is required".to_string(),
            )),
        }
    }

    async fn load_subscription_internal(
        &self,
        project_id: &str,
        source: &SubscriptionSource,
        requested_name: Option<&str>,
    ) -> BrokerResult<LoadSubscriptionOutcome> {
        let _project_guard = self.lock_project(project_id).await;
        let imported = self
            .import_inventory_scope_from_source(
                &ProxyScope::project(project_id),
                source,
                requested_name,
            )
            .await?;
        let new_ips = self.rebuild_effective_project_locked(project_id).await?;

        Ok(LoadSubscriptionOutcome {
            response: imported.response,
            new_ips,
            import_id: imported.import_id,
        })
    }

    async fn load_manual_proxy_group_internal(
        &self,
        project_id: &str,
        content: &str,
        requested_name: Option<&str>,
    ) -> BrokerResult<LoadSubscriptionOutcome> {
        let _project_guard = self.lock_project(project_id).await;
        let imported = self
            .import_inventory_scope_from_content(
                &ProxyScope::project(project_id),
                content,
                requested_name,
            )
            .await?;
        let new_ips = self.rebuild_effective_project_locked(project_id).await?;

        Ok(LoadSubscriptionOutcome {
            response: imported.response,
            new_ips,
            import_id: imported.import_id,
        })
    }

    async fn register_post_load_bookkeeping(
        &self,
        project_id: &str,
        import_id: &str,
        source: &SubscriptionSource,
        new_ips: &[String],
    ) -> BrokerResult<()> {
        self.register_project_sync_source(import_id, project_id, source)
            .await?;

        let queued_or_running = self.queued_or_running_task_runs(project_id).await?;
        if queued_or_running
            .iter()
            .any(|run| run.kind == TaskRunKind::MetadataRefreshFull)
        {
            return Ok(());
        }
        // Only queued incremental runs can safely absorb new IPs. Running runs may have already
        // snapshotted their scope, so later loads must queue a follow-up task instead.
        let mut existing_incremental = queued_or_running
            .iter()
            .find(|run| {
                run.status == TaskRunStatus::Queued
                    && run.kind == TaskRunKind::MetadataRefreshIncremental
            })
            .cloned();

        if let Some(mut existing_run) = existing_incremental.take() {
            if let Some(targeted_ips) = expand_incremental_task_scope(&mut existing_run, new_ips) {
                self.update_task_run_and_emit(&existing_run).await?;
                self.append_task_event(
                    &existing_run,
                    TaskEventLevel::Info,
                    existing_run.stage,
                    "Incremental task scope expanded to include newly loaded IPs.",
                    Some(serde_json::json!({ "targeted_ips": targeted_ips })),
                )
                .await?;
            }
        } else {
            self.enqueue_task_run(
                project_id,
                TaskRunKind::MetadataRefreshIncremental,
                TaskRunTrigger::PostLoad,
                TaskRunScope::Ips {
                    ips: new_ips.to_vec(),
                },
            )
            .await?;
        }

        Ok(())
    }

    async fn refresh_metadata_internal(
        &self,
        project_id: &str,
        force: bool,
        target_ips: Option<&HashSet<String>>,
        run_id: Option<&str>,
    ) -> BrokerResult<RefreshResponse> {
        let _project_guard = self.lock_project(project_id).await;

        let nodes = self.compose_effective_session_nodes(project_id).await?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let mut ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let scoped_ip_set = scoped_ip_records(&ip_records, target_ips);
        if target_ips.is_some() && scoped_ip_set.is_empty() {
            return Ok(RefreshResponse {
                probed_ips: 0,
                geo_updated: 0,
                skipped_cached: 0,
            });
        }
        let scoped_nodes = scope_nodes_for_ips(&nodes, Some(&scoped_ip_set));

        let stored_probe_records = self
            .store
            .list_probe_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let scoped_probe_records =
            filter_probe_records_to_ips(&stored_probe_records, &scoped_ip_set);
        let probe_cache_complete = has_complete_probe_records(
            &scoped_nodes,
            &self.options.probe_targets,
            &scoped_probe_records,
        );

        let now = now_epoch_sec();
        let should_probe = force
            || !probe_cache_complete
            || ip_records.iter().any(|record| {
                scoped_ip_set.contains(&record.ip)
                    && record
                        .probe_updated_at
                        .map(|ts| ts + (self.options.probe_ttl_sec as i64) < now)
                        .unwrap_or(true)
            });

        if let Some(run_id) = run_id {
            self.update_task_stage_by_id(
                run_id,
                TaskRunStage::DiffingInventory,
                Some(0),
                Some(scoped_ip_set.len() as u64),
                "Preparing metadata refresh scope.",
                Some(serde_json::json!({
                    "targeted_ips": scoped_ip_set.len(),
                    "force": force,
                })),
            )
            .await?;
        }

        let mut probe_records = if should_probe {
            if let Some(run_id) = run_id {
                self.update_task_stage_by_id(
                    run_id,
                    TaskRunStage::Probing,
                    Some(0),
                    Some(scoped_ip_set.len() as u64),
                    "Refreshing probe metadata.",
                    Some(serde_json::json!({
                        "targeted_ips": scoped_ip_set.len(),
                    })),
                )
                .await?;
            }
            let sessions = self
                .store
                .list_sessions(project_id)
                .await
                .map_err(BrokerError::from)?;
            self.apply_shared_runtime_config(Some(project_id), Some(&sessions), true)
                .await?;
            self.refresh_probe_records(project_id, now, &nodes, Some(&scoped_ip_set))
                .await?
        } else {
            scoped_probe_records
        };

        if should_probe {
            for record in &mut ip_records {
                if scoped_ip_set.contains(&record.ip)
                    && probe_records.iter().any(|probe| probe.ip == record.ip)
                {
                    record.probe_updated_at = Some(now);
                }
            }
            self.store
                .upsert_probe_records(project_id, &probe_records)
                .await
                .map_err(BrokerError::from)?;
        }

        if let Some(run_id) = run_id {
            self.update_task_stage_by_id(
                run_id,
                TaskRunStage::GeoEnrichment,
                Some(scoped_ip_set.len() as u64),
                Some(scoped_ip_set.len() as u64),
                "Refreshing geo metadata.",
                Some(serde_json::json!({
                    "targeted_ips": scoped_ip_set.len(),
                })),
            )
            .await?;
        }

        let geo_updated = self
            .refresh_geo_records(
                project_id,
                force,
                now,
                &mut ip_records,
                Some(&scoped_ip_set),
            )
            .await?;

        if let Some(run_id) = run_id {
            self.update_task_stage_by_id(
                run_id,
                TaskRunStage::Persisting,
                Some(scoped_ip_set.len() as u64),
                Some(scoped_ip_set.len() as u64),
                "Persisting refreshed metadata.",
                Some(serde_json::json!({
                    "targeted_ips": scoped_ip_set.len(),
                    "geo_updated": geo_updated,
                })),
            )
            .await?;
        }

        self.store
            .upsert_ip_records(project_id, &ip_records)
            .await
            .map_err(BrokerError::from)?;
        self.upsert_project_proxy_node_metadata_from_legacy_records(
            project_id,
            Some(&scoped_ip_set),
        )
        .await?;

        if !should_probe {
            probe_records = filter_probe_records_to_ips(
                &self
                    .store
                    .list_probe_records(project_id)
                    .await
                    .map_err(BrokerError::from)?,
                &scoped_ip_set,
            );
        }

        self.cleanup_shared_runtime_if_idle().await;

        let probed_ips: HashSet<String> =
            probe_records.into_iter().map(|record| record.ip).collect();

        Ok(RefreshResponse {
            probed_ips: probed_ips.len(),
            geo_updated,
            skipped_cached: if should_probe { 0 } else { scoped_ip_set.len() },
        })
    }

    async fn refresh_probe_records(
        &self,
        _project_id: &str,
        now: i64,
        nodes: &[ProxyNode],
        target_ips: Option<&HashSet<String>>,
    ) -> BrokerResult<Vec<ProbeRecord>> {
        let mut tasks = Vec::new();
        for node in nodes {
            for ip in &node.resolved_ips {
                if let Some(target_ips) = target_ips
                    && !target_ips.contains(ip)
                {
                    continue;
                }
                let runtime_name = node.node_id.as_deref().unwrap_or(&node.proxy_name);
                let probe_proxy_name = dedicated_ip_proxy_name(runtime_name, ip);
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

        let timeout_ms = self.options.probe_timeout_ms;
        let concurrency = self.options.probe_concurrency.max(1);

        let probed: Vec<((String, String, String), ProbeRecord)> = stream::iter(tasks)
            .map(|(proxy_name, ip, target, probe_proxy_name)| async move {
                let delay = self
                    .runtime
                    .measure_proxy_delay(
                        GLOBAL_RUNTIME_PROJECT_ID,
                        &probe_proxy_name,
                        &target,
                        timeout_ms,
                    )
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

        let temp_file = geo_dir.join(format!("country.mmdb.tmp-{}", ids::random_temp_suffix()));
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
        _project_id: &str,
        force: bool,
        now: i64,
        ip_records: &mut [IpRecord],
        target_ips: Option<&HashSet<String>>,
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
                if !ip_in_scope(&record.ip, target_ips) {
                    return None;
                }
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
            if !ip_in_scope(&record.ip, target_ips) {
                continue;
            }
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

            let mut country_code = normalize_country_code(record.country_code.as_deref());
            let mut country_name = record.country_name.clone();
            let mut region_name = record.region_name.clone();
            let mut city = record.city.clone();
            let mut source = None;
            let mut mmdb_hit = false;
            let mut online_hit = false;
            let mut lookup_country_code_hit = false;
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
                    let mmdb_country_code = normalize_country_code(info.iso_code);
                    let mmdb_country_name =
                        info.names.and_then(|m| m.get("en").map(|x| x.to_string()));
                    if mmdb_country_code.is_some() {
                        country_code = mmdb_country_code.clone();
                        lookup_country_code_hit = true;
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
                let normalized_online_country_code =
                    normalize_country_code(online.country_code.as_deref());
                let online_has_geo = normalized_online_country_code.is_some()
                    || online.country.as_ref().is_some()
                    || online.region.as_ref().is_some()
                    || online.city.as_ref().is_some();
                country_code = resolve_online_geo_country_code(
                    country_code,
                    lookup_country_code_hit,
                    online.country_code.as_deref(),
                    online_has_geo,
                );
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

    async fn apply_sessions_config_locked(
        &self,
        project_id: &str,
        sessions: &[SessionRecord],
    ) -> BrokerResult<()> {
        self.apply_shared_runtime_config_locked(Some(project_id), Some(sessions), false)
            .await
    }

    async fn rollback_runtime_sessions_locked(
        &self,
        project_id: &str,
        sessions: &[SessionRecord],
    ) -> anyhow::Result<()> {
        self.apply_sessions_config_locked(project_id, sessions)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn recover_runtime_desync_locked(&self, project_id: &str, sessions: &[SessionRecord]) {
        tracing::warn!(
            project_id,
            "attempting runtime recovery after rollback failure"
        );
        if let Err(err) = self
            .runtime
            .shutdown_project(GLOBAL_RUNTIME_PROJECT_ID)
            .await
        {
            tracing::warn!(
                project_id,
                error = %err,
                "failed to shutdown runtime during rollback recovery"
            );
        }

        if sessions.is_empty() {
            return;
        }

        if let Err(err) = self
            .runtime
            .ensure_started(GLOBAL_RUNTIME_PROJECT_ID)
            .await
            .map_err(|e| BrokerError::MihomoUnavailable(e.to_string()))
        {
            tracing::warn!(
                project_id,
                error = %err,
                "failed to restart runtime during rollback recovery"
            );
            return;
        }

        if let Err(err) = self
            .apply_sessions_config_locked(project_id, sessions)
            .await
        {
            tracing::warn!(
                project_id,
                error = %err,
                "failed to reapply sessions during rollback recovery"
            );
        }
    }

    pub async fn extract_ips(
        &self,
        project_id: &str,
        request: &ExtractIpRequest,
    ) -> BrokerResult<ExtractIpResponse> {
        validate_conflict(request)?;
        let _project_guard = self.lock_project(project_id).await;

        let ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(project_id)
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
        project_id: &str,
        request: &OpenSessionRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenSessionResponse> {
        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;

        let nodes = self.compose_effective_session_nodes(project_id).await?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let existing = self.list_sessions_backfilled(project_id).await?;

        let retryable = request.desired_port.is_none();
        let min_probe_updated_at =
            Some(now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64));
        let max_attempts = if retryable {
            nodes
                .iter()
                .flat_map(|node| {
                    node.resolved_ips
                        .iter()
                        .map(move |ip| (runtime_node_id(node), ip))
                })
                .collect::<HashSet<_>>()
                .len()
                .max(1)
        } else {
            1usize
        };
        let mut request_with_exclusions = request.clone();
        let mut excluded_node_ids = HashSet::new();
        let mut excluded_ips = HashSet::new();
        let mut last_runtime_apply_error = None;
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &probe_records,
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at,
        };
        let ports = SessionPortConfig {
            listen_ip: self.options.session_listen_ip,
            port_range: self.options.session_port_range,
        };

        for attempt in 1..=max_attempts {
            let prepared = match prepare_session(
                &request_with_exclusions,
                &ip_records,
                &existing,
                &candidates,
                &ports,
                &excluded_node_ids,
            ) {
                Ok(prepared) => prepared,
                Err(err)
                    if retryable
                        && attempt < max_attempts
                        && matches!(&err, BrokerError::PortInUse) =>
                {
                    continue;
                }
                Err(BrokerError::IpNotFound | BrokerError::NoHealthyProxyNodes)
                    if last_runtime_apply_error.is_some() =>
                {
                    return Err(last_runtime_apply_error.expect("checked above"));
                }
                Err(err) => return Err(err),
            };

            let mut merged = existing.clone();
            merged.push(prepared.clone());

            if let Err(err) = self.apply_sessions_config_locked(project_id, &merged).await {
                tracing::warn!(
                    project_id,
                    attempt,
                    error = %err,
                    "session apply config failed"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after session apply failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                let apply_error = BrokerError::ProxyRuntimeApplyFailed(err.to_string());
                if retryable && attempt < max_attempts {
                    excluded_node_ids.insert(prepared.node_id.clone());
                    let same_ip_has_available_candidate = nodes.iter().any(|node| {
                        !excluded_node_ids.contains(&runtime_node_id(node))
                            && node
                                .resolved_ips
                                .iter()
                                .any(|ip| ip == &prepared.selected_ip)
                    });
                    if !same_ip_has_available_candidate
                        && excluded_ips.insert(prepared.selected_ip.clone())
                    {
                        if request_with_exclusions.selection_mode == SessionSelectionMode::Ip {
                            let failed_ip = normalize_ip_text(&prepared.selected_ip);
                            request_with_exclusions
                                .specified_ips
                                .retain(|ip| normalize_ip_text(ip) != failed_ip);
                        } else {
                            request_with_exclusions
                                .excluded_ips
                                .push(prepared.selected_ip.clone());
                            request_with_exclusions.excluded_ips.sort();
                            request_with_exclusions.excluded_ips.dedup();
                        }
                    }
                    last_runtime_apply_error = Some(apply_error);
                    continue;
                }
                return Err(apply_error);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(project_id, std::slice::from_ref(&prepared), now)
                .await
            {
                tracing::error!(
                    project_id,
                    session_id = %prepared.session_id,
                    error = %err,
                    "persist session failed after runtime apply, rolling back runtime"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        session_id = %prepared.session_id,
                        error = %rollback_err,
                        "runtime rollback failed after session insert failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                return Err(BrokerError::from(err));
            }

            return Ok(self.build_open_session_response(prepared, request_display_host));
        }

        Err(last_runtime_apply_error.unwrap_or(BrokerError::PortInUse))
    }

    pub async fn open_batch(
        &self,
        project_id: &str,
        request: &OpenBatchRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenBatchResponse> {
        if request.requests.is_empty() {
            return Ok(OpenBatchResponse { sessions: vec![] });
        }

        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;

        let nodes = self
            .store
            .list_subscription(project_id)
            .await
            .map_err(BrokerError::from)?;
        if nodes.is_empty() {
            return Err(BrokerError::SubscriptionInvalid);
        }

        let ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let probe_records = self
            .store
            .list_probe_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let existing = self.list_sessions_backfilled(project_id).await?;

        let retryable = request.requests.iter().all(|r| r.desired_port.is_none());
        let min_probe_updated_at =
            Some(now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64));
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &probe_records,
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at,
        };
        let ports = SessionPortConfig {
            listen_ip: self.options.session_listen_ip,
            port_range: self.options.session_port_range,
        };
        let candidate_ips = request
            .requests
            .iter()
            .map(|request| {
                candidate_ips_for_open(
                    request,
                    &ip_records,
                    &probe_records,
                    &metadata_by_pair,
                    min_probe_updated_at,
                )
            })
            .collect::<BrokerResult<Vec<_>>>()?;
        let mut candidate_indexes = vec![0usize; request.requests.len()];

        let mut attempt = 0usize;
        let mut port_bind_attempts = 0usize;
        loop {
            attempt += 1;
            let staged = match stage_batch_sessions(
                &request.requests,
                &candidate_ips,
                &mut candidate_indexes,
                &existing,
                &candidates,
                &ports,
            ) {
                Ok(staged) => staged,
                Err(err) if retryable && matches!(err, BrokerError::PortInUse) => {
                    port_bind_attempts += 1;
                    if port_bind_attempts < AUTO_PORT_BIND_ATTEMPTS {
                        continue;
                    }
                    if advance_candidate_indexes(&mut candidate_indexes, &candidate_ips) {
                        port_bind_attempts = 0;
                        continue;
                    }
                    return Err(BrokerError::PortInUse);
                }
                Err(err) => return Err(err),
            };

            let mut merged = existing.clone();
            merged.extend(staged.clone());

            if let Err(err) = self.apply_sessions_config_locked(project_id, &merged).await {
                tracing::warn!(
                    project_id,
                    attempt,
                    error = %err,
                    "batch apply config failed before persisting sessions"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after batch apply failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                if retryable && advance_candidate_indexes(&mut candidate_indexes, &candidate_ips) {
                    port_bind_attempts = 0;
                    continue;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(project_id, &staged, now)
                .await
            {
                tracing::error!(
                    project_id,
                    error = %err,
                    "batch persist failed after runtime apply, rolling back"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        error = %rollback_err,
                        "runtime rollback failed after batch persist failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            return Ok(OpenBatchResponse {
                sessions: staged
                    .into_iter()
                    .map(|s| self.build_open_session_response(s, request_display_host))
                    .collect(),
            });
        }
    }

    async fn list_sessions_backfilled(&self, project_id: &str) -> BrokerResult<Vec<SessionRecord>> {
        let mut sessions = self
            .store
            .list_sessions(project_id)
            .await
            .map_err(BrokerError::from)?;
        let effective_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await
            .unwrap_or_default();
        let mut repaired = Vec::new();
        for session in &mut sessions {
            let mut changed = false;
            if session.node_id.trim().is_empty() {
                let matches = effective_nodes
                    .iter()
                    .filter(|node| {
                        node.proxy_name == session.proxy_name
                            && node
                                .resolved_ips
                                .iter()
                                .any(|ip| ip == &session.selected_ip)
                    })
                    .collect::<Vec<_>>();
                if matches.len() == 1 {
                    session.node_id = matches[0].node_id.clone();
                    changed = true;
                }
            }
            let normalized =
                normalized_candidate_node_ids(&session.node_id, &session.candidate_node_ids);
            if normalized != session.candidate_node_ids {
                session.candidate_node_ids = normalized;
                changed = true;
            }
            if changed {
                repaired.push(session.clone());
            }
        }
        if !repaired.is_empty() {
            self.store
                .insert_sessions(project_id, &repaired)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(sessions)
    }

    async fn build_session_list_items(
        &self,
        project_id: &str,
        sessions: Vec<SessionRecord>,
        request_display_host: Option<&str>,
    ) -> BrokerResult<Vec<SessionListItem>> {
        if sessions.is_empty() {
            return Ok(Vec::new());
        }

        let effective_project_ids = vec![project_id.to_string()];
        let effective_nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|record| (record.node_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let metadata_by_node = self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<String, Vec<ProxyNodeMetadataRecord>>::new(),
                |mut acc, record| {
                    let record = sanitize_proxy_node_metadata_record(record);
                    acc.entry(record.node_id.clone()).or_default().push(record);
                    acc
                },
            );
        let legacy_metadata = self
            .load_legacy_project_metadata(&effective_project_ids)
            .await?;

        Ok(sessions
            .into_iter()
            .map(|session| {
                let bind_host = session.listen.clone();
                let display_host =
                    self.resolve_session_display_host(&bind_host, request_display_host);
                let display_address = format_listen_endpoint(&display_host, session.port);
                let existing_metadata = metadata_by_node
                    .get(&session.node_id)
                    .and_then(|items| items.iter().find(|item| item.ip == session.selected_ip))
                    .cloned();
                let selected_metadata = effective_nodes
                    .get(&session.node_id)
                    .and_then(|record| {
                        self.merge_proxy_node_metadata_with_legacy(
                            record,
                            &session.selected_ip,
                            &effective_project_ids,
                            &legacy_metadata,
                            existing_metadata.clone(),
                        )
                    })
                    .or(existing_metadata);

                SessionListItem {
                    session_id: session.session_id,
                    listen: format_listen_endpoint(&bind_host, session.port),
                    bind_host,
                    display_host,
                    display_address,
                    port: session.port,
                    selected_ip: session.selected_ip,
                    proxy_name: session.proxy_name,
                    candidate_node_ids: normalized_candidate_node_ids(
                        &session.node_id,
                        &session.candidate_node_ids,
                    ),
                    node_id: session.node_id,
                    created_at: session.created_at,
                    country_code: selected_metadata
                        .as_ref()
                        .and_then(|item| item.country_code.clone()),
                    country_name: selected_metadata
                        .as_ref()
                        .and_then(|item| item.country_name.clone()),
                    region_name: selected_metadata
                        .as_ref()
                        .and_then(|item| item.region_name.clone()),
                    city: selected_metadata
                        .as_ref()
                        .and_then(|item| item.city.clone()),
                }
            })
            .collect())
    }

    fn build_open_session_response(
        &self,
        session: SessionRecord,
        request_display_host: Option<&str>,
    ) -> OpenSessionResponse {
        let bind_host = session.listen.clone();
        let display_host = self.resolve_session_display_host(&bind_host, request_display_host);
        let display_address = format_listen_endpoint(&display_host, session.port);
        OpenSessionResponse {
            session_id: session.session_id,
            listen: format_listen_endpoint(&bind_host, session.port),
            bind_host,
            display_host,
            display_address,
            port: session.port,
            selected_ip: session.selected_ip,
            proxy_name: session.proxy_name,
            candidate_node_ids: normalized_candidate_node_ids(
                &session.node_id,
                &session.candidate_node_ids,
            ),
            node_id: session.node_id,
        }
    }

    pub async fn open_session_by_node(
        &self,
        project_id: &str,
        request: &OpenSessionByNodeRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenSessionResponse> {
        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        let nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let Some(node) = nodes
            .into_iter()
            .find(|candidate| candidate.node_id == request.node_id)
        else {
            return Err(BrokerError::ProxyInventoryNodeNotFound);
        };
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let probe_records = self
            .store
            .list_probe_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let min_probe_updated_at =
            now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64);
        let fresh_ips = fresh_healthy_ips_for_inventory_node_health(
            &node,
            &metadata_by_pair,
            &probe_records,
            min_probe_updated_at,
        )?;
        let candidate_node_ids = vec![node.node_id.clone()];
        let existing = self.list_sessions_backfilled(project_id).await?;
        let retryable = request.desired_port.is_none();
        let max_attempts = if retryable { fresh_ips.len() } else { 1usize };
        let mut last_runtime_apply_error = None;

        for (attempt_index, selected_ip) in fresh_ips.into_iter().take(max_attempts).enumerate() {
            let attempt = attempt_index + 1;
            let port = match allocate_port(
                &existing,
                request.desired_port,
                self.options.session_listen_ip,
                self.options.session_port_range,
            ) {
                Ok(port) => port,
                Err(err)
                    if retryable
                        && attempt < max_attempts
                        && matches!(&err, BrokerError::PortInUse) =>
                {
                    continue;
                }
                Err(err) => return Err(err),
            };
            let prepared = SessionRecord {
                session_id: ids::random_session_id(),
                listen: self.options.session_listen_ip.to_string(),
                port,
                selected_ip: selected_ip.clone(),
                proxy_name: node.proxy_name.clone(),
                node_id: node.node_id.clone(),
                candidate_node_ids: candidate_node_ids.clone(),
                created_at: now_epoch_sec(),
            };
            let mut merged = existing.clone();
            merged.push(prepared.clone());

            if let Err(err) = self.apply_sessions_config_locked(project_id, &merged).await {
                tracing::warn!(
                    project_id,
                    attempt,
                    error = %err,
                    "node-pinned session apply config failed"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after node-pinned session apply failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                let apply_error = BrokerError::ProxyRuntimeApplyFailed(err.to_string());
                if retryable && attempt < max_attempts {
                    last_runtime_apply_error = Some(apply_error);
                    continue;
                }
                return Err(apply_error);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(project_id, std::slice::from_ref(&prepared), now)
                .await
            {
                tracing::error!(
                    project_id,
                    session_id = %prepared.session_id,
                    error = %err,
                    "persist node-pinned session failed after runtime apply"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        session_id = %prepared.session_id,
                        error = %rollback_err,
                        "runtime rollback failed after node-pinned session insert failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                return Err(BrokerError::from(err));
            }

            return Ok(self.build_open_session_response(prepared, request_display_host));
        }

        Err(last_runtime_apply_error.unwrap_or(BrokerError::PortInUse))
    }

    pub async fn open_batch_by_node(
        &self,
        project_id: &str,
        request: &OpenBatchByNodeRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenBatchResponse> {
        let requests = if !request.requests.is_empty() {
            request.requests.clone()
        } else {
            request
                .node_ids
                .iter()
                .cloned()
                .map(|node_id| OpenSessionByNodeRequest {
                    node_id,
                    desired_port: None,
                })
                .collect::<Vec<_>>()
        };
        if requests.is_empty() {
            return Ok(OpenBatchResponse {
                sessions: Vec::new(),
            });
        }
        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        let node_map = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?
            .into_iter()
            .map(|record| (record.node_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let probe_records = self
            .store
            .list_probe_records(project_id)
            .await
            .map_err(BrokerError::from)?;
        let min_probe_updated_at =
            now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64);
        let existing = self.list_sessions_backfilled(project_id).await?;
        let retryable = requests.iter().all(|item| item.desired_port.is_none());
        let mut fresh_ips_by_request = Vec::new();
        for request in &requests {
            let Some(node) = node_map.get(&request.node_id) else {
                return Err(BrokerError::ProxyInventoryNodeNotFound);
            };
            let fresh_ips = fresh_healthy_ips_for_inventory_node_health(
                node,
                &metadata_by_pair,
                &probe_records,
                min_probe_updated_at,
            )?;
            fresh_ips_by_request.push(fresh_ips);
        }
        let mut candidate_indexes = vec![0usize; requests.len()];

        let mut attempt = 0usize;
        let mut port_bind_attempts = 0usize;
        'attempts: loop {
            attempt += 1;
            let mut merged = existing.clone();
            let mut staged = Vec::new();
            for (request_index, request) in requests.iter().enumerate() {
                let Some(node) = node_map.get(&request.node_id) else {
                    return Err(BrokerError::ProxyInventoryNodeNotFound);
                };
                let selected_ip = fresh_ips_by_request
                    .get(request_index)
                    .and_then(|items| items.get(candidate_indexes[request_index]))
                    .cloned()
                    .ok_or(BrokerError::NoHealthyProxyNodes)?;
                let port = match allocate_port(
                    &merged,
                    request.desired_port,
                    self.options.session_listen_ip,
                    self.options.session_port_range,
                ) {
                    Ok(port) => port,
                    Err(err) if retryable && matches!(err, BrokerError::PortInUse) => {
                        port_bind_attempts += 1;
                        if port_bind_attempts < AUTO_PORT_BIND_ATTEMPTS {
                            continue 'attempts;
                        }
                        if advance_candidate_indexes(&mut candidate_indexes, &fresh_ips_by_request)
                        {
                            port_bind_attempts = 0;
                            continue 'attempts;
                        }
                        return Err(BrokerError::PortInUse);
                    }
                    Err(err) => return Err(err),
                };
                let session = SessionRecord {
                    session_id: ids::random_session_id(),
                    listen: self.options.session_listen_ip.to_string(),
                    port,
                    selected_ip,
                    proxy_name: node.proxy_name.clone(),
                    node_id: node.node_id.clone(),
                    candidate_node_ids: vec![node.node_id.clone()],
                    created_at: now_epoch_sec(),
                };
                merged.push(session.clone());
                staged.push(session);
            }
            if staged.len() != requests.len() {
                continue;
            }

            if let Err(err) = self.apply_sessions_config_locked(project_id, &merged).await {
                tracing::warn!(
                    project_id,
                    attempt,
                    error = %err,
                    "node-pinned batch apply config failed before persisting sessions"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after node-pinned batch apply failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                if retryable
                    && advance_candidate_indexes(&mut candidate_indexes, &fresh_ips_by_request)
                {
                    port_bind_attempts = 0;
                    continue;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(project_id, &staged, now)
                .await
            {
                tracing::error!(
                    project_id,
                    attempt,
                    error = %err,
                    "persist node-pinned batch failed after runtime apply"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after node-pinned batch insert failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            return Ok(OpenBatchResponse {
                sessions: staged
                    .into_iter()
                    .map(|session| self.build_open_session_response(session, request_display_host))
                    .collect(),
            });
        }
    }

    pub async fn open_session_by_ip(
        &self,
        project_id: &str,
        request: &OpenSessionByIpRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenSessionResponse> {
        let batch = self
            .open_batch_by_ip(
                project_id,
                &OpenBatchByIpRequest {
                    requests: vec![request.clone()],
                },
                request_display_host,
            )
            .await?;
        batch
            .sessions
            .into_iter()
            .next()
            .ok_or(BrokerError::IpNotFound)
    }

    pub async fn open_batch_by_ip(
        &self,
        project_id: &str,
        request: &OpenBatchByIpRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenBatchResponse> {
        if request.requests.is_empty() {
            return Ok(OpenBatchResponse {
                sessions: Vec::new(),
            });
        }

        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        let nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let min_probe_updated_at =
            now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64);
        let existing = self.list_sessions_backfilled(project_id).await?;
        let retryable = request
            .requests
            .iter()
            .all(|item| item.desired_port.is_none());
        let max_attempts = if retryable {
            request
                .requests
                .iter()
                .map(|item| {
                    let requested = normalized_candidate_node_ids("", &item.candidate_node_ids);
                    requested.len().max(1)
                })
                .product::<usize>()
                .max(1)
        } else {
            1usize
        };
        let mut excluded_node_ids_by_request =
            vec![HashSet::<String>::new(); request.requests.len()];

        for attempt in 1..=max_attempts {
            let mut merged = existing.clone();
            let mut staged = Vec::new();
            for (request_index, item) in request.requests.iter().enumerate() {
                let selected_ip = item.selected_ip.trim();
                if selected_ip.is_empty() {
                    return Err(BrokerError::InvalidRequest(
                        "selected_ip must not be empty".to_string(),
                    ));
                }
                let candidate_node_ids =
                    normalized_candidate_node_ids("", &item.candidate_node_ids);
                let (node, candidate_node_ids) = choose_best_inventory_node_for_ip_excluding(
                    selected_ip,
                    &candidate_node_ids,
                    &excluded_node_ids_by_request[request_index],
                    &nodes,
                    &metadata_by_pair,
                    Some(min_probe_updated_at),
                )?;
                let port = match allocate_port(
                    &merged,
                    item.desired_port,
                    self.options.session_listen_ip,
                    self.options.session_port_range,
                ) {
                    Ok(port) => port,
                    Err(err)
                        if retryable
                            && attempt < max_attempts
                            && matches!(&err, BrokerError::PortInUse) =>
                    {
                        staged.clear();
                        merged = existing.clone();
                        continue;
                    }
                    Err(err) => return Err(err),
                };
                let session = SessionRecord {
                    session_id: ids::random_session_id(),
                    listen: self.options.session_listen_ip.to_string(),
                    port,
                    selected_ip: selected_ip.to_string(),
                    proxy_name: node.proxy_name.clone(),
                    node_id: node.node_id.clone(),
                    candidate_node_ids,
                    created_at: now_epoch_sec(),
                };
                merged.push(session.clone());
                staged.push(session);
            }
            if staged.len() != request.requests.len() {
                continue;
            }

            if let Err(err) = self.apply_sessions_config_locked(project_id, &merged).await {
                tracing::warn!(
                    project_id,
                    attempt,
                    error = %err,
                    "ip-candidate batch apply config failed before persisting sessions"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after ip-candidate batch apply failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                if retryable && attempt < max_attempts {
                    let multi_candidate_request_indexes = request
                        .requests
                        .iter()
                        .enumerate()
                        .filter_map(|(request_index, item)| {
                            let selected_ip = item.selected_ip.trim();
                            let requested =
                                normalized_candidate_node_ids("", &item.candidate_node_ids);
                            let remaining = requested
                                .iter()
                                .filter(|node_id| {
                                    !excluded_node_ids_by_request[request_index].contains(*node_id)
                                        && nodes.iter().any(|node| {
                                            node.node_id == **node_id
                                                && node
                                                    .resolved_ips
                                                    .iter()
                                                    .any(|ip| ip == selected_ip)
                                        })
                                })
                                .count();
                            (remaining > 1).then_some(request_index)
                        })
                        .collect::<Vec<_>>();
                    for request_index in multi_candidate_request_indexes {
                        if let Some(session) = staged.get(request_index) {
                            excluded_node_ids_by_request[request_index]
                                .insert(session.node_id.clone());
                        }
                    }
                    continue;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            let now = now_epoch_sec();
            if let Err(err) = self
                .store
                .insert_sessions_with_touch(project_id, &staged, now)
                .await
            {
                tracing::error!(
                    project_id,
                    attempt,
                    error = %err,
                    "persist ip-candidate batch failed after runtime apply"
                );
                if let Err(rollback_err) = self
                    .rollback_runtime_sessions_locked(project_id, &existing)
                    .await
                {
                    tracing::error!(
                        project_id,
                        attempt,
                        error = %rollback_err,
                        "runtime rollback failed after ip-candidate batch insert failure"
                    );
                    self.recover_runtime_desync_locked(project_id, &existing)
                        .await;
                }
                return Err(BrokerError::BatchOpenFailed);
            }

            return Ok(OpenBatchResponse {
                sessions: staged
                    .into_iter()
                    .map(|session| self.build_open_session_response(session, request_display_host))
                    .collect(),
            });
        }

        Err(BrokerError::BatchOpenFailed)
    }

    pub async fn suggested_port(&self, project_id: &str) -> BrokerResult<SuggestedPortResponse> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }

        let _project_guard = self.lock_project(project_id).await;
        let existing = self.list_sessions_backfilled(project_id).await?;
        let port = allocate_port(
            &existing,
            None,
            self.options.session_listen_ip,
            self.options.session_port_range,
        )?;
        Ok(SuggestedPortResponse { port })
    }

    pub async fn search_session_options(
        &self,
        project_id: &str,
        request: &SearchSessionOptionsRequest,
    ) -> BrokerResult<SearchSessionOptionsResponse> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }

        let ip_records = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?;

        let items = search_session_options(&ip_records, request)?;
        Ok(SearchSessionOptionsResponse { items })
    }

    pub async fn search_session_node_options(
        &self,
        project_id: &str,
        session_id: &str,
        request: &SearchSessionNodeOptionsRequest,
    ) -> BrokerResult<SearchSessionNodeOptionsResponse> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }

        let sessions = self.list_sessions_backfilled(project_id).await?;
        if !sessions
            .iter()
            .any(|session| session.session_id == session_id)
        {
            return Err(BrokerError::SessionNotFound);
        }

        let query = request
            .query
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let session_usage = self
            .store
            .list_session_node_usages(project_id, session_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.node_id, record.last_used_at))
            .collect::<HashMap<_, _>>();
        let project_usage = self
            .store
            .list_project_node_usages(project_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.node_id, record.last_used_at))
            .collect::<HashMap<_, _>>();
        let import_records = self
            .store
            .list_proxy_imports()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.import_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let samples_by_node_ip = self
            .store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<(String, String), Vec<ProxyNodeProbeSampleRecord>>::new(),
                |mut acc, record| {
                    acc.entry((record.node_id.clone(), record.ip.clone()))
                        .or_default()
                        .push(record);
                    acc
                },
            );
        let metadata_by_node = self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<String, Vec<ProxyNodeMetadataRecord>>::new(),
                |mut acc, record| {
                    let record = sanitize_proxy_node_metadata_record(record);
                    acc.entry(record.node_id.clone()).or_default().push(record);
                    acc
                },
            );
        let effective_project_ids = vec![project_id.to_string()];
        let legacy_metadata = self
            .load_legacy_project_metadata(&effective_project_ids)
            .await?;

        let mut items = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?
            .into_iter()
            .filter_map(|record| {
                let import_record = import_records.get(&record.import_id);
                let import_name =
                    import_record
                        .and_then(|item| item.name.clone())
                        .and_then(|value| {
                            let trimmed = value.trim();
                            (!trimmed.is_empty()).then(|| trimmed.to_string())
                        });
                let source_label = import_record.map(format_proxy_import_source_label);
                let primary_ip = record.resolved_ips.first().cloned();
                let node_metadata = metadata_by_node.get(&record.node_id);
                let primary_metadata = primary_ip
                    .as_ref()
                    .and_then(|ip| {
                        let existing = node_metadata
                            .and_then(|items| items.iter().find(|item| item.ip == *ip))
                            .cloned();
                        self.merge_proxy_node_metadata_with_legacy(
                            &record,
                            ip,
                            &effective_project_ids,
                            &legacy_metadata,
                            existing,
                        )
                    })
                    .or_else(|| node_metadata.and_then(|items| items.first().cloned()));
                let recent_probe_samples = primary_metadata
                    .as_ref()
                    .and_then(|item| {
                        samples_by_node_ip
                            .get(&(item.node_id.clone(), item.ip.clone()))
                            .cloned()
                    })
                    .or_else(|| {
                        primary_metadata
                            .as_ref()
                            .map(legacy_probe_samples_as_recent)
                    })
                    .unwrap_or_default();
                let item = SessionNodeOptionItem {
                    node_id: record.node_id.clone(),
                    proxy_name: record.proxy_name,
                    import_name,
                    source_label,
                    primary_ip,
                    country_code: primary_metadata
                        .as_ref()
                        .and_then(|item| item.country_code.clone()),
                    country_name: primary_metadata
                        .as_ref()
                        .and_then(|item| item.country_name.clone()),
                    region_name: primary_metadata
                        .as_ref()
                        .and_then(|item| item.region_name.clone()),
                    city: primary_metadata.as_ref().and_then(|item| item.city.clone()),
                    last_probe_ok: primary_metadata
                        .as_ref()
                        .and_then(|item| item.last_probe_ok),
                    median_latency_ms: primary_metadata
                        .as_ref()
                        .and_then(|item| item.median_latency_ms),
                    recent_probe_samples,
                    session_last_used_at: session_usage.get(&record.node_id).copied(),
                    project_last_used_at: project_usage.get(&record.node_id).copied(),
                };
                matches_session_node_query(&item, &query).then_some(item)
            })
            .collect::<Vec<_>>();

        items.sort_by(|left, right| compare_session_node_options(left, right, request.sort_mode));
        if let Some(limit) = request.limit {
            items.truncate(limit);
        }
        Ok(SearchSessionNodeOptionsResponse { items })
    }

    pub async fn search_session_ip_node_options(
        &self,
        project_id: &str,
        request: &SearchSessionIpNodeOptionsRequest,
    ) -> BrokerResult<SearchSessionIpNodeOptionsResponse> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }

        let query = request
            .query
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let session_id = request.session_id.as_deref();
        let session_usage = if let Some(session_id) = session_id {
            self.store
                .list_session_node_usages(project_id, session_id)
                .await
                .map_err(BrokerError::from)?
                .into_iter()
                .map(|record| (record.node_id, record.last_used_at))
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
        };
        let project_usage = self
            .store
            .list_project_node_usages(project_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.node_id, record.last_used_at))
            .collect::<HashMap<_, _>>();
        let ip_usage = self
            .store
            .list_ip_records(project_id)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.ip, record.last_used_at))
            .collect::<HashMap<_, _>>();
        let import_records = self
            .store
            .list_proxy_imports()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| (record.import_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let effective_project_ids = vec![project_id.to_string()];
        let legacy_metadata = self
            .load_legacy_project_metadata(&effective_project_ids)
            .await?;

        let mut items_by_group_ip = HashMap::<(String, String), SessionIpNodeOptionIpItem>::new();
        for record in self
            .compose_effective_proxy_inventory_records(project_id)
            .await?
        {
            let import_record = import_records.get(&record.import_id);
            let import_name = import_record
                .and_then(|item| item.name.clone())
                .and_then(|value| {
                    let trimmed = value.trim();
                    (!trimmed.is_empty()).then(|| trimmed.to_string())
                });
            let source_label = import_record.map(format_proxy_import_source_label);
            let subscription_name = import_name
                .clone()
                .or_else(|| source_label.clone())
                .unwrap_or_else(|| record.import_id.clone());

            for ip in &record.resolved_ips {
                let existing = metadata_by_pair
                    .get(&(record.node_id.clone(), ip.clone()))
                    .cloned();
                let metadata = self.merge_proxy_node_metadata_with_legacy(
                    &record,
                    ip,
                    &effective_project_ids,
                    &legacy_metadata,
                    existing,
                );
                let node_item = SessionIpNodeOptionNodeItem {
                    node_id: record.node_id.clone(),
                    proxy_name: record.proxy_name.clone(),
                    import_name: import_name.clone(),
                    source_label: source_label.clone(),
                    country_code: metadata.as_ref().and_then(|item| item.country_code.clone()),
                    country_name: metadata.as_ref().and_then(|item| item.country_name.clone()),
                    region_name: metadata.as_ref().and_then(|item| item.region_name.clone()),
                    city: metadata.as_ref().and_then(|item| item.city.clone()),
                    last_probe_ok: metadata.as_ref().and_then(|item| item.last_probe_ok),
                    median_latency_ms: metadata.as_ref().and_then(|item| item.median_latency_ms),
                    recent_probe_samples: metadata
                        .as_ref()
                        .map(recent_probe_samples_from_metadata)
                        .unwrap_or_default(),
                    project_last_used_at: project_usage.get(&record.node_id).copied(),
                    session_last_used_at: session_usage.get(&record.node_id).copied(),
                };
                if !matches_session_ip_node_query(ip, &subscription_name, &node_item, &query) {
                    continue;
                }

                let group_label = match request.group_by {
                    SessionIpNodeGroupBy::Subscription => subscription_name.clone(),
                    SessionIpNodeGroupBy::City => node_item
                        .city
                        .clone()
                        .or_else(|| node_item.region_name.clone())
                        .or_else(|| node_item.country_name.clone())
                        .unwrap_or_else(|| "Unknown location".to_string()),
                };
                let group_key = match request.group_by {
                    SessionIpNodeGroupBy::Subscription => {
                        format!("subscription:{}", record.import_id)
                    }
                    SessionIpNodeGroupBy::City => format!(
                        "city:{}:{}:{}",
                        node_item.country_code.as_deref().unwrap_or_default(),
                        node_item.region_name.as_deref().unwrap_or_default(),
                        node_item.city.as_deref().unwrap_or_default(),
                    )
                    .to_ascii_lowercase(),
                };
                let entry = items_by_group_ip
                    .entry((group_key.clone(), ip.clone()))
                    .or_insert_with(|| SessionIpNodeOptionIpItem {
                        ip: ip.clone(),
                        group_key: group_key.clone(),
                        group_label: group_label.clone(),
                        subscription_name: Some(subscription_name.clone()),
                        country_code: node_item.country_code.clone(),
                        country_name: node_item.country_name.clone(),
                        region_name: node_item.region_name.clone(),
                        city: node_item.city.clone(),
                        last_used_at: ip_usage.get(ip).copied().flatten(),
                        best_latency_ms: None,
                        nodes: Vec::new(),
                    });
                entry.best_latency_ms =
                    best_latency(entry.best_latency_ms, node_item.median_latency_ms);
                entry.nodes.push(node_item);
            }
        }

        let mut items = items_by_group_ip.into_values().collect::<Vec<_>>();
        for item in &mut items {
            item.nodes.sort_by(compare_session_ip_node_nodes);
        }
        items.sort_by(compare_session_ip_node_items);
        items.truncate(request.limit.unwrap_or(DEFAULT_SESSION_NODE_OPTIONS_LIMIT));

        let mut grouped = Vec::<SessionIpNodeOptionGroupItem>::new();
        for item in items {
            if let Some(group) = grouped.iter_mut().find(|group| group.key == item.group_key) {
                group.items.push(item);
            } else {
                grouped.push(SessionIpNodeOptionGroupItem {
                    key: item.group_key.clone(),
                    label: item.group_label.clone(),
                    items: vec![item],
                });
            }
        }

        Ok(SearchSessionIpNodeOptionsResponse { groups: grouped })
    }

    async fn proxy_node_metadata_by_pair(
        &self,
    ) -> BrokerResult<HashMap<(String, String), ProxyNodeMetadataRecord>> {
        let samples_by_node_ip = self.recent_proxy_node_probe_samples_by_pair().await?;
        Ok(self
            .store
            .list_proxy_node_metadata()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(sanitize_proxy_node_metadata_record)
            .map(|record| attach_recent_probe_samples(record, &samples_by_node_ip))
            .map(|record| ((record.node_id.clone(), record.ip.clone()), record))
            .collect())
    }

    async fn recent_proxy_node_probe_samples_by_pair(
        &self,
    ) -> BrokerResult<HashMap<(String, String), Vec<ProxyNodeProbeSampleRecord>>> {
        Ok(self
            .store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .fold(
                HashMap::<(String, String), Vec<ProxyNodeProbeSampleRecord>>::new(),
                |mut acc, record| {
                    acc.entry((record.node_id.clone(), record.ip.clone()))
                        .or_default()
                        .push(record);
                    acc
                },
            ))
    }

    pub async fn update_session_node(
        &self,
        project_id: &str,
        session_id: &str,
        request: &UpdateSessionNodeRequest,
        request_display_host: Option<&str>,
    ) -> BrokerResult<OpenSessionResponse> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }

        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;
        let nodes = self
            .compose_effective_proxy_inventory_records(project_id)
            .await?;
        let metadata_by_pair = self.proxy_node_metadata_by_pair().await?;
        let min_probe_updated_at =
            now_epoch_sec().saturating_sub(self.options.probe_ttl_sec as i64);
        let requested_candidates =
            normalized_candidate_node_ids(&request.node_id, &request.candidate_node_ids);
        let (node, selected_ip, candidate_node_ids) = if let Some(selected_ip) = request
            .selected_ip
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let (node, candidate_node_ids) = choose_best_inventory_node_for_ip(
                selected_ip,
                &requested_candidates,
                &nodes,
                &metadata_by_pair,
                Some(min_probe_updated_at),
            )?;
            (node, selected_ip.to_string(), candidate_node_ids)
        } else {
            let Some(node) = nodes
                .into_iter()
                .find(|candidate| candidate.node_id == request.node_id)
            else {
                return Err(BrokerError::ProxyInventoryNodeNotFound);
            };
            let selected_ip = fresh_healthy_ips_for_inventory_node_metadata(
                &node,
                &metadata_by_pair,
                min_probe_updated_at,
            )?
            .into_iter()
            .next()
            .ok_or(BrokerError::NoHealthyProxyNodes)?;
            let candidate_node_ids = vec![node.node_id.clone()];
            (node, selected_ip, candidate_node_ids)
        };

        let mut sessions = self.list_sessions_backfilled(project_id).await?;
        let Some(session_index) = sessions
            .iter()
            .position(|session| session.session_id == session_id)
        else {
            return Err(BrokerError::SessionNotFound);
        };
        let previous_sessions = sessions.clone();
        let mut updated = sessions[session_index].clone();
        let touch_time = now_epoch_sec();

        if updated.node_id == node.node_id
            && updated.proxy_name == node.proxy_name
            && updated.selected_ip == selected_ip
            && normalized_candidate_node_ids(&updated.node_id, &updated.candidate_node_ids)
                == candidate_node_ids
        {
            updated.candidate_node_ids = candidate_node_ids;
            self.store
                .insert_sessions_with_touch(project_id, std::slice::from_ref(&updated), touch_time)
                .await
                .map_err(BrokerError::from)?;
            return Ok(self.build_open_session_response(updated, request_display_host));
        }

        updated.selected_ip = selected_ip;
        updated.proxy_name = node.proxy_name;
        updated.node_id = node.node_id;
        updated.candidate_node_ids = candidate_node_ids;
        sessions[session_index] = updated.clone();

        if let Err(err) = self
            .apply_sessions_config_locked(project_id, &sessions)
            .await
        {
            tracing::warn!(
                project_id,
                session_id,
                node_id = %updated.node_id,
                error = %err,
                "session node switch apply config failed"
            );
            if let Err(rollback_err) = self
                .rollback_runtime_sessions_locked(project_id, &previous_sessions)
                .await
            {
                tracing::error!(
                    project_id,
                    session_id,
                    error = %rollback_err,
                    "runtime rollback failed after session node switch apply failure"
                );
                self.recover_runtime_desync_locked(project_id, &previous_sessions)
                    .await;
            }
            return Err(err);
        }

        if let Err(err) = self
            .store
            .insert_sessions_with_touch(project_id, std::slice::from_ref(&updated), touch_time)
            .await
        {
            tracing::error!(
                project_id,
                session_id,
                node_id = %updated.node_id,
                error = %err,
                "persist session node switch failed after runtime apply"
            );
            if let Err(rollback_err) = self
                .rollback_runtime_sessions_locked(project_id, &previous_sessions)
                .await
            {
                tracing::error!(
                    project_id,
                    session_id,
                    error = %rollback_err,
                    "runtime rollback failed after session node switch persist failure"
                );
                self.recover_runtime_desync_locked(project_id, &previous_sessions)
                    .await;
            }
            return Err(BrokerError::from(err));
        }

        Ok(self.build_open_session_response(updated, request_display_host))
    }

    pub async fn list_projects(&self) -> BrokerResult<ListProjectsResponse> {
        let projects = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?;
        Ok(ListProjectsResponse { projects })
    }

    pub async fn create_project(&self, project_id: &str) -> BrokerResult<CreateProjectResponse> {
        let normalized = project_id.trim();
        if normalized.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "project_id must not be empty".to_string(),
            ));
        }

        let _project_guard = self.lock_project(normalized).await;
        let exists = self
            .store
            .list_projects()
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .any(|item| item == normalized);
        if exists {
            return Err(BrokerError::ProjectExists);
        }

        self.store
            .create_project(normalized, now_epoch_sec())
            .await
            .map_err(BrokerError::from)?;

        Ok(CreateProjectResponse {
            project_id: normalized.to_string(),
        })
    }

    pub async fn load_global_subscription(
        &self,
        source: &SubscriptionSource,
    ) -> BrokerResult<LoadSubscriptionResponse> {
        self.load_global_subscription_request(&LoadSubscriptionRequest {
            name: None,
            source: Some(source.clone()),
            content: None,
        })
        .await
    }

    pub async fn load_global_subscription_request(
        &self,
        request: &LoadSubscriptionRequest,
    ) -> BrokerResult<LoadSubscriptionResponse> {
        let _global_guard = self.lock_project("__global_proxy_scope__").await;
        let imported = match (&request.source, request.content.as_deref()) {
            (Some(source), None) => {
                self.import_inventory_scope_from_source(
                    &ProxyScope::global(),
                    source,
                    request.name.as_deref(),
                )
                .await?
            }
            (None, Some(content)) => {
                self.import_inventory_scope_from_content(
                    &ProxyScope::global(),
                    content,
                    request.name.as_deref(),
                )
                .await?
            }
            (Some(_), Some(_)) => {
                return Err(BrokerError::InvalidRequest(
                    "provide either `source` or `content`, not both".to_string(),
                ));
            }
            (None, None) => {
                return Err(BrokerError::InvalidRequest(
                    "either `source` or `content` is required".to_string(),
                ));
            }
        };
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let affected_projects = projects
            .into_iter()
            .filter(|project_id| {
                settings
                    .get(project_id.as_str())
                    .map(|item| item.use_global_proxies)
                    .unwrap_or(true)
            })
            .collect::<HashSet<_>>();
        self.rebuild_projects(&affected_projects).await?;
        Ok(imported.response)
    }

    pub async fn list_proxy_imports(
        &self,
        scope: Option<&str>,
        project_id: Option<&str>,
    ) -> BrokerResult<ListProxyImportResponse> {
        let mut items = self
            .store
            .list_proxy_imports()
            .await
            .map_err(BrokerError::from)?;
        match scope.unwrap_or("all") {
            "all" => {}
            "global" => items.retain(|item| matches!(&item.allocation_scope, ProxyScope::Global)),
            "project" => {
                let project_id = project_id.ok_or_else(|| {
                    BrokerError::InvalidRequest(
                        "project_id is required when scope=project".to_string(),
                    )
                })?;
                items.retain(|item| {
                    matches!(
                        &item.allocation_scope,
                        ProxyScope::Project {
                            project_id: allocated_project_id,
                        } if allocated_project_id == project_id
                    )
                });
            }
            _ => {
                return Err(BrokerError::InvalidRequest(
                    "scope must be one of all|global|project".to_string(),
                ));
            }
        }
        let mut items = futures_util::future::try_join_all(
            items
                .into_iter()
                .map(|record| self.proxy_import_item_from_record(record)),
        )
        .await?;
        items.sort_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .reverse()
                .then_with(|| left.import_id.cmp(&right.import_id))
        });
        Ok(ListProxyImportResponse { items })
    }

    pub async fn list_proxy_inventory(
        &self,
        scope: Option<&str>,
        project_id: Option<&str>,
    ) -> BrokerResult<ListProxyInventoryResponse> {
        let scope = scope.unwrap_or("all");
        let mut items = self
            .store
            .list_proxy_inventory()
            .await
            .map_err(BrokerError::from)?;
        match scope {
            "all" => {}
            "global" => items.retain(|item| matches!(&item.allocation_scope, ProxyScope::Global)),
            "project" => {
                let project_id = project_id.ok_or_else(|| {
                    BrokerError::InvalidRequest(
                        "project_id is required when scope=project".to_string(),
                    )
                })?;
                items.retain(|item| {
                    matches!(
                        &item.allocation_scope,
                        ProxyScope::Project {
                            project_id: allocated_project_id,
                        } if allocated_project_id == project_id
                    )
                });
            }
            _ => {
                return Err(BrokerError::InvalidRequest(
                    "scope must be one of all|global|project".to_string(),
                ));
            }
        }
        let listing_project_id = if scope == "project" {
            project_id.unwrap_or(GLOBAL_RUNTIME_PROJECT_ID)
        } else {
            GLOBAL_RUNTIME_PROJECT_ID
        };
        items = self.filter_malformed_inventory_records(
            listing_project_id,
            items,
            "malformed proxy inventory node skipped from inventory listing",
        );

        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let items = items
            .into_iter()
            .map(|item| {
                let effective_project_ids =
                    self.effective_project_ids_for_record(&item, &projects, &settings);
                ProxyInventoryItem {
                    import_id: item.import_id,
                    node_id: item.node_id,
                    proxy_name: item.proxy_name,
                    proxy_type: item.proxy_type,
                    server: item.server,
                    resolved_ips: item.resolved_ips,
                    source_scope: item.source_scope,
                    allocation_scope: item.allocation_scope,
                    effective_project_ids,
                }
            })
            .collect::<Vec<_>>();
        let mut items = items;
        items.sort_by(|left, right| {
            left.proxy_name
                .cmp(&right.proxy_name)
                .then_with(|| left.server.cmp(&right.server))
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        Ok(ListProxyInventoryResponse { items })
    }

    pub async fn update_proxy_allocation(
        &self,
        node_id: &str,
        allocation_scope: &ProxyScope,
    ) -> BrokerResult<ProxyInventoryItem> {
        if let ProxyScope::Project { project_id } = allocation_scope
            && !self.project_exists(project_id).await?
        {
            return Err(BrokerError::ProjectNotFound);
        }

        let before = self
            .store
            .get_proxy_inventory_node(node_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        let updated_import = self
            .update_proxy_import_allocation(&before.import_id, allocation_scope)
            .await?;
        let updated = self
            .store
            .get_proxy_inventory_node(node_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        let _ = updated_import;
        self.inventory_item_from_record(updated).await
    }

    pub async fn update_proxy_import_allocation(
        &self,
        import_id: &str,
        allocation_scope: &ProxyScope,
    ) -> BrokerResult<ProxyImportItem> {
        if let ProxyScope::Project { project_id } = allocation_scope
            && !self.project_exists(project_id).await?
        {
            return Err(BrokerError::ProjectNotFound);
        }

        let before = self
            .store
            .get_proxy_import(import_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let before_effective = match &before.allocation_scope {
            ProxyScope::Global => projects
                .iter()
                .filter(|project_id| {
                    settings
                        .get(project_id.as_str())
                        .map(|item| item.use_global_proxies)
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
            ProxyScope::Project { project_id } => vec![project_id.clone()],
        };
        let updated = self
            .store
            .update_proxy_import_allocation(import_id, allocation_scope, now_epoch_sec())
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        let after_effective = match &updated.allocation_scope {
            ProxyScope::Global => projects
                .iter()
                .filter(|project_id| {
                    settings
                        .get(project_id.as_str())
                        .map(|item| item.use_global_proxies)
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
            ProxyScope::Project { project_id } => vec![project_id.clone()],
        };
        let affected_projects = before_effective
            .into_iter()
            .chain(after_effective)
            .collect::<HashSet<_>>();
        self.rebuild_projects(&affected_projects).await?;
        self.proxy_import_item_from_record(updated).await
    }

    pub async fn delete_proxy_inventory_node(&self, node_id: &str) -> BrokerResult<()> {
        let before = self
            .store
            .get_proxy_inventory_node(node_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        self.delete_proxy_import(&before.import_id).await
    }

    pub async fn get_proxy_import(&self, import_id: &str) -> BrokerResult<ProxyImportRecord> {
        self.store
            .get_proxy_import(import_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)
    }

    pub async fn delete_proxy_import(&self, import_id: &str) -> BrokerResult<()> {
        let before = self
            .store
            .delete_proxy_import(import_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::ProxyInventoryNodeNotFound)?;
        let (projects, settings) = self.list_project_ids_with_settings().await?;
        let affected_projects = match &before.allocation_scope {
            ProxyScope::Global => projects
                .into_iter()
                .filter(|project_id| {
                    settings
                        .get(project_id.as_str())
                        .map(|item| item.use_global_proxies)
                        .unwrap_or(true)
                })
                .collect::<HashSet<_>>(),
            ProxyScope::Project { project_id } => std::iter::once(project_id.clone()).collect(),
        };
        self.rebuild_projects(&affected_projects).await?;
        Ok(())
    }

    pub async fn get_project_proxy_settings(
        &self,
        project_id: &str,
    ) -> BrokerResult<ProjectProxySettings> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }
        self.get_project_proxy_settings_effective(project_id).await
    }

    pub async fn update_project_proxy_settings(
        &self,
        project_id: &str,
        use_global_proxies: bool,
    ) -> BrokerResult<ProjectProxySettings> {
        if !self.project_exists(project_id).await? {
            return Err(BrokerError::ProjectNotFound);
        }
        let _project_guard = self.lock_project(project_id).await;
        let settings = ProjectProxySettings {
            project_id: project_id.to_string(),
            use_global_proxies,
        };
        self.store
            .upsert_project_proxy_settings(&settings)
            .await
            .map_err(BrokerError::from)?;
        self.rebuild_effective_project_locked(project_id).await?;
        Ok(settings)
    }

    pub async fn get_system_settings(&self) -> BrokerResult<SystemSettings> {
        Ok(self
            .store
            .get_system_settings()
            .await
            .map_err(BrokerError::from)?
            .unwrap_or_else(default_system_settings))
    }

    pub async fn update_system_settings(
        &self,
        proxy_probe_interval_sec: u64,
    ) -> BrokerResult<SystemSettings> {
        if proxy_probe_interval_sec < 60 {
            return Err(BrokerError::InvalidRequest(
                "proxy_probe_interval_sec must be at least 60".to_string(),
            ));
        }
        let settings = SystemSettings {
            proxy_probe_interval_sec,
            updated_at: now_epoch_sec(),
        };
        self.store
            .upsert_system_settings(&settings)
            .await
            .map_err(BrokerError::from)?;
        Ok(settings)
    }

    pub async fn list_tasks(&self, query: &TaskListQuery) -> BrokerResult<TaskListResponse> {
        let all_summaries = self.list_task_run_summaries(query).await?;
        Ok(build_task_list_response(query, all_summaries))
    }

    pub async fn list_task_run_summaries(
        &self,
        query: &TaskListQuery,
    ) -> BrokerResult<Vec<TaskRunSummary>> {
        let mut full_query = query.clone();
        full_query.limit = None;
        full_query.cursor = None;

        let all_runs = self
            .store
            .list_task_runs(&full_query)
            .await
            .map_err(BrokerError::from)?;
        Ok(all_runs
            .into_iter()
            .map(|run| run.as_summary())
            .collect::<Vec<_>>())
    }

    pub async fn get_task_run_detail(&self, run_id: &str) -> BrokerResult<TaskRunDetail> {
        let run = self
            .store
            .get_task_run(run_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or(BrokerError::TaskRunNotFound)?;
        let events = self
            .store
            .list_task_run_events(run_id)
            .await
            .map_err(BrokerError::from)?;
        Ok(to_detail(run.as_summary(), events))
    }

    pub async fn get_task_run_summary(&self, run_id: &str) -> BrokerResult<Option<TaskRunSummary>> {
        Ok(self
            .store
            .get_task_run(run_id)
            .await
            .map_err(BrokerError::from)?
            .map(|run| run.as_summary()))
    }

    pub fn subscribe_task_events(&self) -> broadcast::Receiver<TaskBusEvent> {
        self.task_events.subscribe()
    }

    pub async fn list_api_keys(&self, owner_subject: &str) -> BrokerResult<ListApiKeysResponse> {
        let api_keys = self
            .store
            .list_api_keys(owner_subject)
            .await
            .map_err(BrokerError::from)?
            .into_iter()
            .map(|record| record.as_summary())
            .collect();

        Ok(ListApiKeysResponse { api_keys })
    }

    pub async fn create_api_key(
        &self,
        request: &CreateApiKeyRequest,
        created_by_subject: &str,
    ) -> BrokerResult<CreateApiKeyResponse> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(BrokerError::InvalidRequest(
                "api key name must not be empty".to_string(),
            ));
        }

        let project_scope = self.normalize_api_key_scope(&request.project_scope).await?;
        let issued = issue_api_key(name, created_by_subject, project_scope);
        self.store
            .insert_api_key(&issued.record)
            .await
            .map_err(BrokerError::from)?;
        Ok(issued.into_response())
    }

    pub async fn revoke_api_key(&self, owner_subject: &str, key_id: &str) -> BrokerResult<()> {
        let revoked = self
            .store
            .revoke_api_key(owner_subject, key_id, now_epoch_sec())
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

        Ok(Principal::api_key(
            api_key.key_id,
            api_key.created_by_subject,
            api_key.project_scope,
        ))
    }

    pub async fn list_sessions(
        &self,
        project_id: &str,
        request_display_host: Option<&str>,
    ) -> BrokerResult<ListSessionsResponse> {
        let sessions = self.list_sessions_backfilled(project_id).await?;
        let sessions = self
            .build_session_list_items(project_id, sessions, request_display_host)
            .await?;
        Ok(ListSessionsResponse { sessions })
    }

    pub async fn close_session(&self, project_id: &str, session_id: &str) -> BrokerResult<()> {
        let _project_guard = self.lock_project(project_id).await;
        let _shared_runtime_guard = self.shared_runtime_lock.lock().await;

        let mut sessions = self.list_sessions_backfilled(project_id).await?;
        let previous_sessions = sessions.clone();

        let old_len = sessions.len();
        sessions.retain(|s| s.session_id != session_id);
        if sessions.len() == old_len {
            return Err(BrokerError::SessionNotFound);
        }

        if sessions.is_empty() {
            self.store
                .delete_session(project_id, session_id)
                .await
                .map_err(BrokerError::from)?;
            self.cleanup_shared_runtime_if_idle_locked().await;
            return Ok(());
        }

        self.apply_sessions_config_locked(project_id, &sessions)
            .await?;

        if let Err(err) = self.store.delete_session(project_id, session_id).await {
            tracing::error!(
                project_id,
                session_id,
                error = %err,
                "persist close-session failed, rolling back runtime"
            );
            if let Err(rollback_err) = self
                .rollback_runtime_sessions_locked(project_id, &previous_sessions)
                .await
            {
                tracing::error!(
                    project_id,
                    session_id,
                    error = %rollback_err,
                    "runtime rollback failed after close-session persistence error"
                );
                self.recover_runtime_desync_locked(project_id, &previous_sessions)
                    .await;
            }
            return Err(BrokerError::from(err));
        }

        self.cleanup_shared_runtime_if_idle_locked().await;

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

fn normalize_country_code(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.len() != 2 {
        return None;
    }
    let normalized = trimmed.to_ascii_uppercase();
    if normalized
        .as_bytes()
        .iter()
        .all(|byte| byte.is_ascii_alphabetic())
        || matches!(normalized.as_str(), "A1" | "A2" | "O1")
    {
        return Some(normalized);
    }
    None
}

fn resolve_online_geo_country_code(
    current_country_code: Option<String>,
    has_lookup_country_code: bool,
    online_country_code: Option<&str>,
    online_has_geo: bool,
) -> Option<String> {
    let normalized_online_country_code = normalize_country_code(online_country_code);
    let malformed_online_country_code =
        online_country_code.is_some() && normalized_online_country_code.is_none();
    if let Some(value) = normalized_online_country_code {
        return Some(value);
    }
    if online_has_geo && malformed_online_country_code && !has_lookup_country_code {
        return None;
    }
    current_country_code
}

fn normalize_country_filter_token(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    normalize_country_code(Some(trimmed)).or_else(|| Some(trimmed.to_ascii_uppercase()))
}

fn normalize_city_country_token(value: Option<&str>) -> Option<String> {
    normalize_country_filter_token(value)
}

fn sanitize_ip_record(mut record: IpRecord) -> IpRecord {
    record.country_code = normalize_country_code(record.country_code.as_deref());
    record
}

fn sanitize_proxy_node_metadata_record(
    mut record: ProxyNodeMetadataRecord,
) -> ProxyNodeMetadataRecord {
    record.country_code = normalize_country_code(record.country_code.as_deref());
    record
}

fn attach_recent_probe_samples(
    mut record: ProxyNodeMetadataRecord,
    samples_by_node_ip: &HashMap<(String, String), Vec<ProxyNodeProbeSampleRecord>>,
) -> ProxyNodeMetadataRecord {
    record.recent_probe_samples = samples_by_node_ip
        .get(&(record.node_id.clone(), record.ip.clone()))
        .cloned()
        .unwrap_or_else(|| legacy_probe_samples_as_recent(&record));
    record
}

fn recent_probe_samples_from_metadata(
    record: &ProxyNodeMetadataRecord,
) -> Vec<ProxyNodeProbeSampleRecord> {
    if record.recent_probe_samples.is_empty() {
        legacy_probe_samples_as_recent(record)
    } else {
        record.recent_probe_samples.clone()
    }
}

fn legacy_probe_samples_as_recent(
    record: &ProxyNodeMetadataRecord,
) -> Vec<ProxyNodeProbeSampleRecord> {
    if record.last_probe_samples.is_empty() {
        return Vec::new();
    }
    let base_at = record.probe_updated_at.unwrap_or(record.updated_at);
    record
        .last_probe_samples
        .iter()
        .rev()
        .take(10)
        .enumerate()
        .map(|(index, latency_ms)| ProxyNodeProbeSampleRecord {
            node_id: record.node_id.clone(),
            ip: record.ip.clone(),
            target_url: "legacy".to_string(),
            ok: latency_ms.is_some(),
            latency_ms: *latency_ms,
            sampled_at: base_at.saturating_sub(index as i64),
        })
        .collect()
}

fn proxy_node_metadata_has_geo_observation(record: &ProxyNodeMetadataRecord) -> bool {
    record.country_code.is_some()
        || record.country_name.is_some()
        || record.region_name.is_some()
        || record.city.is_some()
        || record
            .geo_source
            .as_deref()
            .is_some_and(|source| source != "none")
}

fn proxy_node_metadata_has_probe_observation(record: &ProxyNodeMetadataRecord) -> bool {
    record.probe_updated_at.is_some()
        || record.last_probe_ok.is_some()
        || record.last_latency_ms.is_some()
        || record.median_latency_ms.is_some()
        || !record.last_probe_samples.is_empty()
        || !record.recent_probe_samples.is_empty()
}

fn proxy_node_metadata_has_observation(record: &ProxyNodeMetadataRecord) -> bool {
    proxy_node_metadata_has_geo_observation(record)
        || proxy_node_metadata_has_probe_observation(record)
}

fn merge_backfilled_proxy_node_metadata(
    mut backfilled: ProxyNodeMetadataRecord,
    existing: Option<&ProxyNodeMetadataRecord>,
    recent_samples: Option<Vec<ProxyNodeProbeSampleRecord>>,
) -> ProxyNodeMetadataRecord {
    if let Some(existing) = existing {
        if should_preserve_existing_observation(
            proxy_node_metadata_has_geo_observation(existing),
            existing.geo_updated_at,
            proxy_node_metadata_has_geo_observation(&backfilled),
            backfilled.geo_updated_at,
        ) {
            backfilled.country_code = existing.country_code.clone();
            backfilled.country_name = existing.country_name.clone();
            backfilled.region_name = existing.region_name.clone();
            backfilled.city = existing.city.clone();
            backfilled.geo_source = existing.geo_source.clone();
            backfilled.geo_updated_at = existing.geo_updated_at;
        }
        if should_preserve_existing_observation(
            proxy_node_metadata_has_probe_observation(existing),
            existing.probe_updated_at,
            proxy_node_metadata_has_probe_observation(&backfilled),
            backfilled.probe_updated_at,
        ) {
            backfilled.probe_updated_at = existing.probe_updated_at;
            backfilled.last_probe_ok = existing.last_probe_ok;
            backfilled.last_latency_ms = existing.last_latency_ms;
            backfilled.median_latency_ms = existing.median_latency_ms;
            backfilled.last_probe_samples = existing.last_probe_samples.clone();
            backfilled.recent_probe_samples = existing.recent_probe_samples.clone();
        }
        backfilled.updated_at = backfilled.updated_at.max(existing.updated_at);
    }

    if let Some(recent_samples) = recent_samples {
        backfilled.recent_probe_samples = recent_samples;
    } else if backfilled.recent_probe_samples.is_empty()
        && !backfilled.last_probe_samples.is_empty()
    {
        backfilled.recent_probe_samples = legacy_probe_samples_as_recent(&backfilled);
    }

    backfilled
}

fn should_preserve_existing_observation(
    existing_has_observation: bool,
    existing_updated_at: Option<i64>,
    backfilled_has_observation: bool,
    backfilled_updated_at: Option<i64>,
) -> bool {
    if !existing_has_observation {
        return false;
    }
    if !backfilled_has_observation {
        return true;
    }
    match (existing_updated_at, backfilled_updated_at) {
        (Some(existing), Some(backfilled)) => existing > backfilled,
        (Some(_), None) => true,
        _ => false,
    }
}

fn default_system_settings() -> SystemSettings {
    SystemSettings {
        proxy_probe_interval_sec: DEFAULT_PROXY_PROBE_INTERVAL_SEC,
        updated_at: 0,
    }
}

fn normalize_country_codes(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let Some(item) = normalize_country_filter_token(Some(value.as_str())) else {
            continue;
        };
        if !seen.insert(item.clone()) {
            continue;
        }
        normalized.push(item);
    }
    normalized
}

fn normalize_city_values(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let item = value.trim().to_string();
        if item.is_empty() {
            continue;
        }
        let key = item.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        normalized.push(item);
    }
    normalized
}

fn normalize_city_filters(values: &[String]) -> HashSet<(Option<String>, String)> {
    let mut normalized = HashSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (country_code, city) = match trimmed.split_once("::") {
            Some((country, city)) => {
                let city = city.trim();
                if city.is_empty() {
                    continue;
                }
                (
                    normalize_city_country_token(Some(country.trim())),
                    city.to_ascii_lowercase(),
                )
            }
            None => (None, trimmed.to_ascii_lowercase()),
        };
        normalized.insert((country_code.filter(|code| !code.is_empty()), city));
    }
    normalized
}

fn normalize_ip_values(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let item = normalize_ip_text(value);
        if item.is_empty() || !seen.insert(item.clone()) {
            continue;
        }
        normalized.push(item);
    }
    normalized
}

fn build_open_selector_request(request: &OpenSessionRequest) -> BrokerResult<ExtractIpRequest> {
    let country_codes = normalize_country_codes(&request.country_codes);
    let cities = normalize_city_values(&request.cities);
    let specified_ips = normalize_ip_values(&request.specified_ips);
    let excluded_ips = normalize_ip_values(&request.excluded_ips);

    match request.selection_mode {
        SessionSelectionMode::Any => {
            if !country_codes.is_empty() || !cities.is_empty() || !specified_ips.is_empty() {
                return Err(BrokerError::InvalidRequest(
                    "selection_mode=any only accepts excluded_ips, sort_mode, and desired_port"
                        .to_string(),
                ));
            }
        }
        SessionSelectionMode::Geo => {
            if !specified_ips.is_empty() {
                return Err(BrokerError::InvalidRequest(
                    "selection_mode=geo does not accept specified_ips".to_string(),
                ));
            }
            if country_codes.is_empty() && cities.is_empty() {
                return Err(BrokerError::InvalidRequest(
                    "selection_mode=geo requires at least one country_codes or cities entry"
                        .to_string(),
                ));
            }
        }
        SessionSelectionMode::Ip => {
            if !country_codes.is_empty() || !cities.is_empty() {
                return Err(BrokerError::InvalidRequest(
                    "selection_mode=ip only accepts specified_ips and excluded_ips".to_string(),
                ));
            }
            if specified_ips.is_empty() {
                return Err(BrokerError::InvalidRequest(
                    "selection_mode=ip requires at least one specified_ips entry".to_string(),
                ));
            }
        }
    }

    let selector = ExtractIpRequest {
        country_codes,
        cities,
        specified_ips,
        blacklist_ips: excluded_ips,
        limit: None,
        sort_mode: request.sort_mode,
    };
    validate_conflict(&selector)?;
    Ok(selector)
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

fn search_session_options(
    ip_records: &[IpRecord],
    request: &SearchSessionOptionsRequest,
) -> BrokerResult<Vec<SessionOptionItem>> {
    let query = request
        .query
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let country_filters: HashSet<String> = normalize_country_codes(&request.country_codes)
        .into_iter()
        .collect();
    let city_filters = normalize_city_filters(&request.cities);
    let limit = request
        .limit
        .unwrap_or(DEFAULT_SESSION_OPTIONS_LIMIT)
        .min(100);

    let items = match request.kind {
        SessionOptionKind::Country => {
            let mut countries = HashMap::<String, SessionOptionItem>::new();
            for record in ip_records {
                let Some(country_code) = normalize_country_code(record.country_code.as_deref())
                else {
                    continue;
                };
                let country_name = record
                    .country_name
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let label = if country_name.is_empty() {
                    country_code.clone()
                } else {
                    format!("{country_name} ({country_code})")
                };
                let haystack = format!(
                    "{} {}",
                    country_code.to_ascii_lowercase(),
                    country_name.to_ascii_lowercase()
                );
                if !query.is_empty() && !haystack.contains(&query) {
                    continue;
                }
                countries
                    .entry(country_code.clone())
                    .or_insert(SessionOptionItem {
                        value: country_code.clone(),
                        label,
                        meta: (!country_name.is_empty()).then_some(country_name),
                    });
            }
            let mut items = countries.into_values().collect::<Vec<_>>();
            items.sort_by(|left, right| left.label.cmp(&right.label));
            items
        }
        SessionOptionKind::City => {
            let mut cities = HashMap::<String, SessionOptionItem>::new();
            for record in ip_records {
                let normalized_country_code =
                    normalize_country_code(record.country_code.as_deref());
                let country_filter_token =
                    normalize_country_filter_token(record.country_code.as_deref());
                let city_country_token =
                    normalize_city_country_token(record.country_code.as_deref());
                if !country_filters.is_empty() {
                    let Some(code) = country_filter_token.as_ref() else {
                        continue;
                    };
                    if !country_filters.contains(code) {
                        continue;
                    }
                }

                let Some(city) = record.city.as_ref() else {
                    continue;
                };
                let city_value = city.trim().to_string();
                if city_value.is_empty() {
                    continue;
                }
                let country_code = normalized_country_code
                    .clone()
                    .or_else(|| {
                        record
                            .country_code
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                let country_name = record.country_name.clone().unwrap_or_default();
                let meta = match (country_code.trim(), country_name.trim()) {
                    ("", "") => None,
                    ("", name) => Some(name.to_string()),
                    (code, "") => Some(code.to_string()),
                    (code, name) => Some(format!("{name} ({code})")),
                };
                let value = city_country_token.as_ref().map_or_else(
                    || city_value.clone(),
                    |country_code| format!("{country_code}::{city_value}"),
                );
                let key = value.to_ascii_lowercase();
                let haystack = format!(
                    "{} {} {}",
                    city_value.to_ascii_lowercase(),
                    city_country_token
                        .as_deref()
                        .unwrap_or(country_code.as_str())
                        .to_ascii_lowercase(),
                    country_name.to_ascii_lowercase()
                );
                if !query.is_empty() && !haystack.contains(&query) {
                    continue;
                }
                cities.entry(key).or_insert(SessionOptionItem {
                    value,
                    label: city_value,
                    meta,
                });
            }
            let mut items = cities.into_values().collect::<Vec<_>>();
            items.sort_by(|left, right| {
                left.label
                    .cmp(&right.label)
                    .then_with(|| left.value.cmp(&right.value))
            });
            items
        }
        SessionOptionKind::Ip => {
            let mut items = ip_records
                .iter()
                .filter(|record| {
                    let normalized_country_code =
                        normalize_country_code(record.country_code.as_deref());
                    let country_filter_token =
                        normalize_country_filter_token(record.country_code.as_deref());
                    let city_country_token =
                        normalize_city_country_token(record.country_code.as_deref());
                    if !country_filters.is_empty() {
                        let Some(code) = country_filter_token.as_ref() else {
                            return false;
                        };
                        if !country_filters.contains(code) {
                            return false;
                        }
                    }
                    if !city_filters.is_empty() {
                        let Some(city) = record.city.as_ref() else {
                            return false;
                        };
                        let city_name = city.trim().to_ascii_lowercase();
                        let matched = city_filters.iter().any(|(country_filter, city_filter)| {
                            city_name == *city_filter
                                && match country_filter {
                                    Some(code) => {
                                        city_country_token.as_deref() == Some(code.as_str())
                                    }
                                    None => true,
                                }
                        });
                        if !matched {
                            return false;
                        }
                    }
                    if query.is_empty() {
                        return true;
                    }
                    let haystack = format!(
                        "{} {} {} {}",
                        record.ip.to_ascii_lowercase(),
                        city_country_token
                            .as_deref()
                            .unwrap_or_else(|| {
                                normalized_country_code.as_deref().unwrap_or_default()
                            })
                            .to_ascii_lowercase(),
                        record
                            .country_name
                            .as_deref()
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                        record
                            .city
                            .as_deref()
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                    );
                    haystack.contains(&query)
                })
                .map(|record| SessionOptionItem {
                    value: record.ip.clone(),
                    label: record.ip.clone(),
                    meta: {
                        let geo = [
                            normalize_country_code(record.country_code.as_deref()),
                            record.city.clone(),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .collect::<Vec<_>>();
                        (!geo.is_empty()).then_some(geo.join(" / "))
                    },
                })
                .collect::<Vec<_>>();
            items.sort_by(|left, right| left.value.cmp(&right.value));
            items.dedup_by(|left, right| left.value == right.value);
            items
        }
    };

    Ok(items.into_iter().take(limit).collect())
}

fn format_proxy_import_source_label(record: &ProxyImportRecord) -> String {
    if let Some(name) = record
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return name.to_string();
    }
    let source_value = record.source_identity.source_value.trim();
    if !source_value.is_empty() {
        return source_value.to_string();
    }
    record.import_id.clone()
}

fn matches_session_node_query(item: &SessionNodeOptionItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    [
        Some(item.proxy_name.as_str()),
        item.import_name.as_deref(),
        item.source_label.as_deref(),
        item.primary_ip.as_deref(),
        item.country_code.as_deref(),
        item.country_name.as_deref(),
        item.region_name.as_deref(),
        item.city.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.to_ascii_lowercase().contains(query))
}

fn matches_session_ip_node_query(
    ip: &str,
    subscription_name: &str,
    item: &SessionIpNodeOptionNodeItem,
    query: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    [
        Some(ip),
        Some(subscription_name),
        Some(item.proxy_name.as_str()),
        item.import_name.as_deref(),
        item.source_label.as_deref(),
        item.country_code.as_deref(),
        item.country_name.as_deref(),
        item.region_name.as_deref(),
        item.city.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.to_ascii_lowercase().contains(query))
}

fn best_latency(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn compare_usage_desc(left: Option<i64>, right: Option<i64>) -> CmpOrdering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => CmpOrdering::Less,
        (None, Some(_)) => CmpOrdering::Greater,
        (None, None) => CmpOrdering::Equal,
    }
}

fn compare_session_node_options(
    left: &SessionNodeOptionItem,
    right: &SessionNodeOptionItem,
    sort_mode: SessionNodeSortMode,
) -> CmpOrdering {
    let ordering = match sort_mode {
        SessionNodeSortMode::SessionRecent => {
            compare_usage_desc(left.session_last_used_at, right.session_last_used_at)
        }
        SessionNodeSortMode::ProjectRecent => {
            compare_usage_desc(left.project_last_used_at, right.project_last_used_at)
        }
    };
    ordering
        .then_with(|| left.proxy_name.cmp(&right.proxy_name))
        .then_with(|| left.node_id.cmp(&right.node_id))
}

fn compare_session_ip_node_nodes(
    left: &SessionIpNodeOptionNodeItem,
    right: &SessionIpNodeOptionNodeItem,
) -> CmpOrdering {
    compare_candidate_probe(
        left.last_probe_ok,
        left.median_latency_ms,
        right.last_probe_ok,
        right.median_latency_ms,
    )
    .then_with(|| left.proxy_name.cmp(&right.proxy_name))
    .then_with(|| left.node_id.cmp(&right.node_id))
}

fn compare_session_ip_node_items(
    left: &SessionIpNodeOptionIpItem,
    right: &SessionIpNodeOptionIpItem,
) -> CmpOrdering {
    left.group_label
        .cmp(&right.group_label)
        .then_with(|| compare_usage_desc(left.last_used_at, right.last_used_at))
        .then_with(|| match (left.best_latency_ms, right.best_latency_ms) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => CmpOrdering::Less,
            (None, Some(_)) => CmpOrdering::Greater,
            (None, None) => CmpOrdering::Equal,
        })
        .then_with(|| left.ip.cmp(&right.ip))
}

fn compare_candidate_probe(
    left_ok: Option<bool>,
    left_latency: Option<u64>,
    right_ok: Option<bool>,
    right_latency: Option<u64>,
) -> CmpOrdering {
    match (left_ok, right_ok) {
        (Some(true), Some(false)) | (Some(true), None) | (None, Some(false)) => CmpOrdering::Less,
        (Some(false), Some(true)) | (None, Some(true)) | (Some(false), None) => {
            CmpOrdering::Greater
        }
        _ => match (left_latency, right_latency) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => CmpOrdering::Less,
            (None, Some(_)) => CmpOrdering::Greater,
            (None, None) => CmpOrdering::Equal,
        },
    }
}

fn choose_best_inventory_node_for_ip_excluding(
    selected_ip: &str,
    candidate_node_ids: &[String],
    excluded_node_ids: &HashSet<String>,
    nodes: &[ProxyInventoryRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
) -> BrokerResult<(ProxyInventoryRecord, Vec<String>)> {
    let available_candidate_node_ids = candidate_node_ids
        .iter()
        .filter(|node_id| !excluded_node_ids.contains(*node_id))
        .cloned()
        .collect::<Vec<_>>();
    if available_candidate_node_ids.is_empty() && !candidate_node_ids.is_empty() {
        return Err(BrokerError::NoHealthyProxyNodes);
    }
    choose_best_inventory_node_for_ip(
        selected_ip,
        &available_candidate_node_ids,
        nodes,
        metadata_by_pair,
        min_probe_updated_at,
    )
}

fn choose_best_inventory_node_for_ip(
    selected_ip: &str,
    candidate_node_ids: &[String],
    nodes: &[ProxyInventoryRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
) -> BrokerResult<(ProxyInventoryRecord, Vec<String>)> {
    let requested = normalized_candidate_node_ids("", candidate_node_ids);
    if requested.is_empty() {
        return Err(BrokerError::InvalidRequest(
            "candidate_node_ids must not be empty".to_string(),
        ));
    }
    for node_id in &requested {
        let valid = nodes.iter().any(|node| {
            node.node_id == *node_id && node.resolved_ips.iter().any(|ip| ip == selected_ip)
        });
        if !valid {
            return Err(BrokerError::InvalidRequest(
                "candidate_node_ids must belong to selected_ip and current project".to_string(),
            ));
        }
    }

    let allowed = requested
        .iter()
        .map(|item| item.as_str())
        .collect::<HashSet<_>>();
    let mut candidates = nodes
        .iter()
        .filter(|node| allowed.contains(node.node_id.as_str()))
        .filter(|node| node.resolved_ips.iter().any(|ip| ip == selected_ip))
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(BrokerError::InvalidRequest(
            "selected_ip must belong to at least one candidate node".to_string(),
        ));
    }
    if let Some(min_updated_at) = min_probe_updated_at {
        candidates.retain(|node| {
            metadata_by_pair
                .get(&(node.node_id.clone(), selected_ip.to_string()))
                .is_some_and(|metadata| {
                    proxy_node_metadata_is_fresh_healthy(metadata, min_updated_at)
                })
        });
        if candidates.is_empty() {
            return Err(BrokerError::NoHealthyProxyNodes);
        }
    }

    candidates.sort_by(|left, right| {
        let left_metadata = metadata_by_pair.get(&(left.node_id.clone(), selected_ip.to_string()));
        let right_metadata =
            metadata_by_pair.get(&(right.node_id.clone(), selected_ip.to_string()));
        let left_latency = min_probe_updated_at
            .and_then(|min_updated_at| {
                left_metadata
                    .and_then(|item| proxy_node_metadata_fresh_latency(item, min_updated_at))
            })
            .or_else(|| left_metadata.and_then(|item| item.median_latency_ms));
        let right_latency = min_probe_updated_at
            .and_then(|min_updated_at| {
                right_metadata
                    .and_then(|item| proxy_node_metadata_fresh_latency(item, min_updated_at))
            })
            .or_else(|| right_metadata.and_then(|item| item.median_latency_ms));
        compare_candidate_probe(
            left_metadata.and_then(|item| item.last_probe_ok),
            left_latency,
            right_metadata.and_then(|item| item.last_probe_ok),
            right_latency,
        )
        .then_with(|| left.proxy_name.cmp(&right.proxy_name))
        .then_with(|| left.node_id.cmp(&right.node_id))
    });

    let any_unknown = candidates.iter().any(|node| {
        metadata_by_pair
            .get(&(node.node_id.clone(), selected_ip.to_string()))
            .and_then(|item| item.last_probe_ok)
            .is_none()
    });
    let any_success = candidates.iter().any(|node| {
        metadata_by_pair
            .get(&(node.node_id.clone(), selected_ip.to_string()))
            .and_then(|item| item.last_probe_ok)
            == Some(true)
    });
    if !any_success && !any_unknown {
        return Err(BrokerError::InvalidRequest(
            "all candidate nodes for selected_ip are unavailable".to_string(),
        ));
    }

    candidates
        .into_iter()
        .next()
        .map(|node| (node, requested))
        .ok_or(BrokerError::ProxyInventoryNodeNotFound)
}

fn reselect_session_from_inventory(
    session: &SessionRecord,
    nodes: &[ProxyInventoryRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
) -> Option<SessionRecord> {
    let requested = normalized_candidate_node_ids(&session.node_id, &session.candidate_node_ids);
    let valid_candidate_node_ids = requested
        .into_iter()
        .filter(|node_id| {
            nodes.iter().any(|node| {
                node.node_id == *node_id
                    && node
                        .resolved_ips
                        .iter()
                        .any(|ip| ip == &session.selected_ip)
            })
        })
        .collect::<Vec<_>>();
    if valid_candidate_node_ids.is_empty() {
        return None;
    }

    let (node, candidate_node_ids) = choose_best_inventory_node_for_ip(
        &session.selected_ip,
        &valid_candidate_node_ids,
        nodes,
        metadata_by_pair,
        None,
    )
    .ok()?;

    let mut updated = session.clone();
    updated.node_id = node.node_id;
    updated.proxy_name = node.proxy_name;
    updated.candidate_node_ids = candidate_node_ids;
    Some(updated)
}

fn inventory_scope_matches_project(scope: &ProxyScope, project_id: &str) -> bool {
    matches!(
        scope,
        ProxyScope::Project {
            project_id: scope_project_id,
        } if scope_project_id == project_id
    )
}

fn compare_inventory_preference(
    project_id: &str,
    left: &ProxyInventoryRecord,
    right: &ProxyInventoryRecord,
) -> CmpOrdering {
    let left_direct = inventory_scope_matches_project(&left.allocation_scope, project_id);
    let right_direct = inventory_scope_matches_project(&right.allocation_scope, project_id);
    right_direct
        .cmp(&left_direct)
        .then_with(|| {
            let left_local_source = inventory_scope_matches_project(&left.source_scope, project_id);
            let right_local_source =
                inventory_scope_matches_project(&right.source_scope, project_id);
            right_local_source.cmp(&left_local_source)
        })
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| left.node_id.cmp(&right.node_id))
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

fn ip_in_scope(ip: &str, target_ips: Option<&HashSet<String>>) -> bool {
    target_ips
        .map(|target_ips| target_ips.contains(ip))
        .unwrap_or(true)
}

fn scoped_ip_records(
    ip_records: &[IpRecord],
    target_ips: Option<&HashSet<String>>,
) -> HashSet<String> {
    ip_records
        .iter()
        .filter(|record| ip_in_scope(&record.ip, target_ips))
        .map(|record| record.ip.clone())
        .collect()
}

fn filter_probe_records_to_ips(
    probe_records: &[ProbeRecord],
    target_ips: &HashSet<String>,
) -> Vec<ProbeRecord> {
    probe_records
        .iter()
        .filter(|record| target_ips.contains(&record.ip))
        .cloned()
        .collect()
}

fn scope_nodes_for_ips(
    nodes: &[ProxyNode],
    target_ips: Option<&HashSet<String>>,
) -> Vec<ProxyNode> {
    nodes
        .iter()
        .filter_map(|node| {
            let resolved_ips = node
                .resolved_ips
                .iter()
                .filter(|ip| ip_in_scope(ip, target_ips))
                .cloned()
                .collect::<Vec<_>>();
            if resolved_ips.is_empty() {
                None
            } else {
                let mut scoped = node.clone();
                scoped.resolved_ips = resolved_ips;
                Some(scoped)
            }
        })
        .collect()
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

fn preserve_or_advance_due_at(existing_due_at: Option<i64>, now: i64, interval_sec: u64) -> i64 {
    match existing_due_at {
        Some(due_at) if due_at > now => due_at,
        _ => now + interval_sec as i64,
    }
}

fn seed_due_at_if_missing(existing_due_at: Option<i64>, now: i64, interval_sec: u64) -> i64 {
    existing_due_at.unwrap_or(now + interval_sec as i64)
}

fn expand_incremental_task_scope(run: &mut TaskRunRecord, new_ips: &[String]) -> Option<usize> {
    if new_ips.is_empty() {
        return None;
    }

    match &mut run.scope {
        TaskRunScope::All => None,
        TaskRunScope::Ips { ips } => {
            let previous_len = ips.len();
            ips.extend(new_ips.iter().cloned());
            ips.sort();
            ips.dedup();
            (ips.len() != previous_len).then_some(ips.len())
        }
        TaskRunScope::Nodes { .. } => None,
    }
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

fn fresh_probe_summary(
    probes: &[ProbeRecord],
    min_updated_at: i64,
) -> HashMap<String, (bool, Option<u64>)> {
    let fresh = probes
        .iter()
        .filter(|probe| probe.updated_at >= min_updated_at)
        .cloned()
        .collect::<Vec<_>>();
    probe_summary(&fresh)
}

fn probe_record_is_fresh_healthy(probe: &ProbeRecord, min_updated_at: i64) -> bool {
    probe.ok && probe.updated_at >= min_updated_at
}

fn probe_node_ip_is_fresh_healthy(
    probes: &[ProbeRecord],
    proxy_name: &str,
    ip: &str,
    min_updated_at: i64,
) -> bool {
    probes.iter().any(|probe| {
        probe.proxy_name == proxy_name
            && probe.ip == ip
            && probe_record_is_fresh_healthy(probe, min_updated_at)
    })
}

fn probe_ip_is_fresh_healthy(probes: &[ProbeRecord], ip: &str, min_updated_at: i64) -> bool {
    probes
        .iter()
        .any(|probe| probe.ip == ip && probe_record_is_fresh_healthy(probe, min_updated_at))
}

fn proxy_node_metadata_is_fresh_healthy(
    metadata: &ProxyNodeMetadataRecord,
    min_updated_at: i64,
) -> bool {
    metadata.last_probe_ok == Some(true)
        && metadata
            .probe_updated_at
            .is_some_and(|updated_at| updated_at >= min_updated_at)
}

fn proxy_node_metadata_fresh_latency(
    metadata: &ProxyNodeMetadataRecord,
    min_updated_at: i64,
) -> Option<u64> {
    let mut latencies = metadata
        .recent_probe_samples
        .iter()
        .filter(|sample| sample.ok && sample.sampled_at >= min_updated_at)
        .filter_map(|sample| sample.latency_ms)
        .collect::<Vec<_>>();
    if !latencies.is_empty() {
        latencies.sort_unstable();
        return Some(latencies[latencies.len() / 2]);
    }
    metadata
        .probe_updated_at
        .is_some_and(|updated_at| updated_at >= min_updated_at)
        .then_some(metadata.last_latency_ms)
        .flatten()
}

fn fresh_healthy_ips_for_inventory_node_metadata(
    node: &ProxyInventoryRecord,
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_updated_at: i64,
) -> BrokerResult<Vec<String>> {
    if node.resolved_ips.is_empty() {
        return Err(BrokerError::SubscriptionInvalid);
    }
    let ips = node
        .resolved_ips
        .iter()
        .filter(|ip| {
            metadata_by_pair
                .get(&(node.node_id.clone(), (*ip).clone()))
                .is_some_and(|metadata| {
                    proxy_node_metadata_is_fresh_healthy(metadata, min_updated_at)
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    if ips.is_empty() {
        return Err(BrokerError::NoHealthyProxyNodes);
    }
    Ok(ips)
}

fn fresh_healthy_ips_for_inventory_node_health(
    node: &ProxyInventoryRecord,
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    probes: &[ProbeRecord],
    min_updated_at: i64,
) -> BrokerResult<Vec<String>> {
    if node.resolved_ips.is_empty() {
        return Err(BrokerError::SubscriptionInvalid);
    }
    let ips = node
        .resolved_ips
        .iter()
        .filter(|ip| {
            metadata_by_pair
                .get(&(node.node_id.clone(), (*ip).clone()))
                .is_some_and(|metadata| {
                    proxy_node_metadata_is_fresh_healthy(metadata, min_updated_at)
                })
                || probe_node_ip_is_fresh_healthy(probes, &node.proxy_name, ip, min_updated_at)
        })
        .cloned()
        .collect::<Vec<_>>();
    if ips.is_empty() {
        return Err(BrokerError::NoHealthyProxyNodes);
    }
    Ok(ips)
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
    let city_filters = normalize_city_filters(&request.cities);

    for record in ip_records {
        let record_ip_key = normalize_ip_text(&record.ip);
        let normalized_country_code = normalize_country_code(record.country_code.as_deref());
        let country_filter_token = normalize_country_filter_token(record.country_code.as_deref());
        let city_country_token = normalize_city_country_token(record.country_code.as_deref());

        if blacklist.contains(&record_ip_key) {
            continue;
        }

        let include = if !specified.is_empty() {
            specified.contains(&record_ip_key)
        } else {
            let country_pass = if country_set.is_empty() {
                true
            } else {
                country_filter_token
                    .as_ref()
                    .map(|code| country_set.contains(code))
                    .unwrap_or(false)
            };
            let city_pass = if city_filters.is_empty() {
                true
            } else {
                let Some(city) = record.city.as_ref() else {
                    continue;
                };
                let city_name = city.trim().to_ascii_lowercase();
                city_filters.iter().any(|(country_filter, city_filter)| {
                    city_name == *city_filter
                        && match country_filter {
                            Some(code) => city_country_token.as_deref() == Some(code.as_str()),
                            None => true,
                        }
                })
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
            country_code: normalized_country_code,
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

#[cfg(test)]
fn choose_ip_for_open(
    request: &OpenSessionRequest,
    ip_records: &[IpRecord],
    probes: &[ProbeRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
) -> BrokerResult<String> {
    candidate_ips_for_open(
        request,
        ip_records,
        probes,
        metadata_by_pair,
        min_probe_updated_at,
    )?
    .into_iter()
    .next()
    .ok_or(BrokerError::IpNotFound)
}

fn candidate_ips_for_open(
    request: &OpenSessionRequest,
    ip_records: &[IpRecord],
    probes: &[ProbeRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
) -> BrokerResult<Vec<String>> {
    let selector = build_open_selector_request(request)?;
    let mut items = filter_ip_records(ip_records.to_vec(), probes, &selector)?;
    if let Some(min_updated_at) = min_probe_updated_at {
        items.retain(|item| {
            probe_ip_is_fresh_healthy(probes, &item.ip, min_updated_at)
                || metadata_by_pair.iter().any(|((_, ip), metadata)| {
                    ip == &item.ip && proxy_node_metadata_is_fresh_healthy(metadata, min_updated_at)
                })
        });
        if items.is_empty() {
            return Err(BrokerError::NoHealthyProxyNodes);
        }
    }

    if matches!(request.selection_mode, SessionSelectionMode::Any) {
        let fresh_probe_index =
            min_probe_updated_at.map(|min_updated_at| fresh_probe_summary(probes, min_updated_at));
        // Preserve the legacy auto-pick quality bar for the unrestricted path:
        // healthy, low-latency candidates win before recency breaks ties.
        items.sort_by(|a, b| {
            let recency = match request.sort_mode {
                crate::models::SortMode::Mru => b.last_used_at.cmp(&a.last_used_at),
                crate::models::SortMode::Lru => a.last_used_at.cmp(&b.last_used_at),
            };
            let a_probe = fresh_probe_index
                .as_ref()
                .and_then(|index| index.get(&a.ip).copied())
                .unwrap_or((a.probe_ok, a.best_latency_ms));
            let b_probe = fresh_probe_index
                .as_ref()
                .and_then(|index| index.get(&b.ip).copied())
                .unwrap_or((b.probe_ok, b.best_latency_ms));
            b_probe
                .0
                .cmp(&a_probe.0)
                .then_with(|| a_probe.1.cmp(&b_probe.1))
                .then_with(|| recency)
                .then_with(|| a.ip.cmp(&b.ip))
        });
    }

    if items.is_empty() {
        return Err(BrokerError::IpNotFound);
    }
    Ok(items.into_iter().map(|item| item.ip).collect())
}

fn choose_node_for_ip(
    ip: &str,
    nodes: &[ProxyNode],
    probes: &[ProbeRecord],
    metadata_by_pair: &HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
    excluded_node_ids: &HashSet<String>,
) -> BrokerResult<ProxyNode> {
    let mut candidates: Vec<&ProxyNode> = nodes
        .iter()
        .filter(|node| !excluded_node_ids.contains(&runtime_node_id(node)))
        .filter(|node| node.resolved_ips.iter().any(|item| item == ip))
        .collect();
    if candidates.is_empty() {
        return Err(BrokerError::IpNotFound);
    }
    if let Some(min_updated_at) = min_probe_updated_at {
        candidates.retain(|node| {
            probe_node_ip_is_fresh_healthy(probes, &node.proxy_name, ip, min_updated_at)
                || metadata_by_pair
                    .get(&(runtime_node_id(node), ip.to_string()))
                    .is_some_and(|metadata| {
                        proxy_node_metadata_is_fresh_healthy(metadata, min_updated_at)
                    })
        });
        if candidates.is_empty() {
            return Err(BrokerError::NoHealthyProxyNodes);
        }
    }
    if candidates.len() == 1 {
        return Ok(candidates[0].clone());
    }

    let mut probe_by_proxy: HashMap<String, (bool, Option<u64>)> = HashMap::new();
    for probe in probes {
        if probe.ip != ip {
            continue;
        }
        if let Some(min_updated_at) = min_probe_updated_at
            && probe.updated_at < min_updated_at
        {
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
        .cloned()
        .ok_or(BrokerError::IpNotFound)
}

fn allocate_port(
    existing: &[SessionRecord],
    desired: Option<u16>,
    listen_ip: IpAddr,
    port_range: Option<(u16, u16)>,
) -> BrokerResult<u16> {
    let used: HashSet<u16> = existing.iter().map(|s| s.port).collect();
    if let Some(port) = desired {
        if port == 0 {
            return Err(BrokerError::InvalidPort);
        }
        if let Some(range) = port_range
            && !port_range_contains(range, port)
        {
            return Err(BrokerError::InvalidRequest(format!(
                "desired_port must fall within configured session port range {}-{}",
                range.0, range.1
            )));
        }
        if used.contains(&port) {
            return Err(BrokerError::PortInUse);
        }
        if std::net::TcpListener::bind((listen_ip, port)).is_err() {
            return Err(BrokerError::PortInUse);
        }
        return Ok(port);
    }

    if let Some((start, end)) = port_range {
        for port in start..=end {
            if used.contains(&port) {
                continue;
            }
            if let Ok(socket) = std::net::TcpListener::bind((listen_ip, port)) {
                drop(socket);
                return Ok(port);
            }
        }
        return Err(BrokerError::PortInUse);
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

fn port_range_contains(range: (u16, u16), port: u16) -> bool {
    range.0 <= port && port <= range.1
}

struct SessionCandidateContext<'a> {
    nodes: &'a [ProxyNode],
    probes: &'a [ProbeRecord],
    metadata_by_pair: &'a HashMap<(String, String), ProxyNodeMetadataRecord>,
    min_probe_updated_at: Option<i64>,
}

struct SessionPortConfig {
    listen_ip: IpAddr,
    port_range: Option<(u16, u16)>,
}

fn prepare_session(
    request: &OpenSessionRequest,
    ip_records: &[IpRecord],
    existing: &[SessionRecord],
    candidates: &SessionCandidateContext<'_>,
    ports: &SessionPortConfig,
    excluded_node_ids: &HashSet<String>,
) -> BrokerResult<SessionRecord> {
    let candidate_ips = candidate_ips_for_open(
        request,
        ip_records,
        candidates.probes,
        candidates.metadata_by_pair,
        candidates.min_probe_updated_at,
    )?;
    let mut last_selection_error = None;
    let (ip, node) = candidate_ips
        .into_iter()
        .find_map(|ip| {
            match choose_node_for_ip(
                &ip,
                candidates.nodes,
                candidates.probes,
                candidates.metadata_by_pair,
                candidates.min_probe_updated_at,
                excluded_node_ids,
            ) {
                Ok(node) => Some(Ok((ip, node))),
                Err(BrokerError::IpNotFound) => {
                    last_selection_error = Some(BrokerError::IpNotFound);
                    None
                }
                Err(BrokerError::NoHealthyProxyNodes) => {
                    last_selection_error = Some(BrokerError::NoHealthyProxyNodes);
                    None
                }
                Err(err) => Some(Err(err)),
            }
        })
        .unwrap_or_else(|| Err(last_selection_error.unwrap_or(BrokerError::IpNotFound)))?;
    let port = allocate_port(
        existing,
        request.desired_port,
        ports.listen_ip,
        ports.port_range,
    )?;
    let now = now_epoch_sec();

    let node_id = runtime_node_id(&node);
    Ok(SessionRecord {
        session_id: ids::random_session_id(),
        listen: ports.listen_ip.to_string(),
        port,
        selected_ip: ip,
        proxy_name: node.proxy_name.clone(),
        node_id: node_id.clone(),
        candidate_node_ids: vec![node_id],
        created_at: now,
    })
}

fn stage_batch_sessions(
    requests: &[OpenSessionRequest],
    candidate_ips: &[Vec<String>],
    candidate_indexes: &mut [usize],
    existing: &[SessionRecord],
    candidates: &SessionCandidateContext<'_>,
    ports: &SessionPortConfig,
) -> BrokerResult<Vec<SessionRecord>> {
    let mut staged = Vec::new();
    let excluded_node_ids = HashSet::new();
    for (request_index, request) in requests.iter().enumerate() {
        let mut all_sessions = existing.to_vec();
        all_sessions.extend(staged.clone());
        let request_candidate_ips = candidate_ips
            .get(request_index)
            .ok_or(BrokerError::NoHealthyProxyNodes)?;
        let mut selected = None;
        let mut last_selection_error = None;
        while let Some(ip) = request_candidate_ips
            .get(candidate_indexes[request_index])
            .cloned()
        {
            match choose_node_for_ip(
                &ip,
                candidates.nodes,
                candidates.probes,
                candidates.metadata_by_pair,
                candidates.min_probe_updated_at,
                &excluded_node_ids,
            ) {
                Ok(node) => {
                    selected = Some((ip, node));
                    break;
                }
                Err(BrokerError::IpNotFound) => {
                    last_selection_error = Some(BrokerError::IpNotFound);
                    candidate_indexes[request_index] += 1;
                }
                Err(BrokerError::NoHealthyProxyNodes) => {
                    last_selection_error = Some(BrokerError::NoHealthyProxyNodes);
                    candidate_indexes[request_index] += 1;
                }
                Err(err) => return Err(err),
            }
        }
        let (ip, node) = selected
            .ok_or_else(|| last_selection_error.unwrap_or(BrokerError::NoHealthyProxyNodes))?;
        let port = allocate_port(
            &all_sessions,
            request.desired_port,
            ports.listen_ip,
            ports.port_range,
        )?;
        let now = now_epoch_sec();
        let node_id = runtime_node_id(&node);
        let prepared = SessionRecord {
            session_id: ids::random_session_id(),
            listen: ports.listen_ip.to_string(),
            port,
            selected_ip: ip,
            proxy_name: node.proxy_name.clone(),
            node_id: node_id.clone(),
            candidate_node_ids: vec![node_id],
            created_at: now,
        };
        staged.push(prepared);
    }
    Ok(staged)
}

fn advance_candidate_indexes(indexes: &mut [usize], candidates: &[Vec<String>]) -> bool {
    for position in (0..indexes.len()).rev() {
        let len = candidates.get(position).map(Vec::len).unwrap_or_default();
        if indexes[position] + 1 < len {
            indexes[position] += 1;
            for item in indexes.iter_mut().skip(position + 1) {
                *item = 0;
            }
            return true;
        }
    }
    false
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

fn median_success_latency(samples: &[u64]) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut values = samples.to_vec();
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn runtime_node_id(node: &ProxyNode) -> String {
    node.node_id.clone().unwrap_or_else(|| {
        ids::stable_proxy_inventory_node_id_for_proxy(
            "legacy",
            &node.proxy_name,
            &node.proxy_type,
            &node.server,
            &node.raw_proxy,
        )
    })
}

fn runtime_node_keys(node: &ProxyNode) -> Vec<String> {
    let primary = runtime_node_id(node);
    if node.node_id.is_none() && node.proxy_name != primary {
        vec![primary, node.proxy_name.clone()]
    } else {
        vec![primary]
    }
}

fn valid_proxy_ip_pairs_for_node(node: &ProxyNode) -> Vec<(String, String)> {
    let keys = runtime_node_keys(node);
    node.resolved_ips
        .iter()
        .flat_map(|ip| keys.iter().cloned().map(move |key| (key, ip.clone())))
        .collect()
}

fn normalize_session_host(raw: Option<&str>) -> Option<String> {
    let candidate = raw?
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if candidate.starts_with('[') {
        let end = candidate.find(']')?;
        return Some(candidate[1..end].trim().to_string());
    }

    if let Some((host, port)) = candidate.rsplit_once(':')
        && !host.is_empty()
        && port.chars().all(|char| char.is_ascii_digit())
        && !host.contains(':')
    {
        return Some(host.trim().to_string());
    }

    Some(candidate.to_string())
}

fn is_wildcard_session_host(host: &str) -> bool {
    matches!(host.trim(), "0.0.0.0" | "::" | "[::]")
}

fn format_listen_endpoint(listen: &str, port: u16) -> String {
    listen
        .parse::<IpAddr>()
        .map(|ip| SocketAddr::new(ip, port).to_string())
        .unwrap_or_else(|_| format!("{listen}:{port}"))
}

fn session_runtime_key(session: &SessionRecord) -> &str {
    if session.node_id.trim().is_empty() {
        session.proxy_name.as_str()
    } else {
        session.node_id.as_str()
    }
}

fn normalized_candidate_node_ids(node_id: &str, candidate_node_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut values = candidate_node_ids
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter_map(|item| {
            let value = item.to_string();
            seen.insert(value.clone()).then_some(value)
        })
        .collect::<Vec<_>>();
    if values.is_empty() && !node_id.trim().is_empty() {
        values.push(node_id.to_string());
    }
    values
}

fn log_unrestored_sessions(
    project_id: &str,
    existing_sessions: &[SessionRecord],
    restorable_sessions: &[SessionRecord],
    message: &'static str,
) {
    if existing_sessions.is_empty() {
        return;
    }

    let restorable_ids = restorable_sessions
        .iter()
        .map(|session| session.session_id.as_str())
        .collect::<HashSet<_>>();
    for session in existing_sessions
        .iter()
        .filter(|session| !restorable_ids.contains(session.session_id.as_str()))
    {
        tracing::warn!(
            project_id,
            session_id = %session.session_id,
            node_id = %session.node_id,
            proxy_name = %session.proxy_name,
            selected_ip = %session.selected_ip,
            listen = %session.listen,
            port = session.port,
            "{message}"
        );
    }
}

fn sort_queued_runs_for_dispatch(runs: &mut [TaskRunRecord]) {
    runs.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| same_project_schedule_dispatch_order(left, right))
            .then_with(|| left.run_id.cmp(&right.run_id))
    });
}

fn same_project_schedule_dispatch_order(
    left: &TaskRunRecord,
    right: &TaskRunRecord,
) -> CmpOrdering {
    if left.project_id != right.project_id
        || left.created_at != right.created_at
        || left.trigger != TaskRunTrigger::Schedule
        || right.trigger != TaskRunTrigger::Schedule
    {
        return CmpOrdering::Equal;
    }

    scheduled_dispatch_rank(left.kind).cmp(&scheduled_dispatch_rank(right.kind))
}

fn scheduled_dispatch_rank(kind: TaskRunKind) -> u8 {
    match kind {
        TaskRunKind::SubscriptionSync => 0,
        TaskRunKind::MetadataRefreshIncremental => 1,
        TaskRunKind::MetadataRefreshFull => 2,
        TaskRunKind::ProxyMetadataRefresh => 3,
        TaskRunKind::ProxyLatencyProbe => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            ApiKeyRecord, IpRecord, LoadSubscriptionRequest, NodeUsageRecord, ProbeRecord,
            ProjectProxySettings, ProjectSyncConfig, ProxyCatalogQuery, ProxyImportKind,
            ProxyImportRecord, ProxyImportSourceIdentity, ProxyImportSyncConfig,
            ProxyInventoryRecord, ProxyNodeMetadataRecord, ProxyScope, ResolvedImportNameSource,
            SessionRecord, SortMode, SubscriptionSource, TaskListQuery, TaskRunEventRecord,
            TaskRunRecord,
        },
        runtime::MihomoRuntime,
        store::{BrokerStore, MemoryStore, SqliteStore},
        subscription::SUBSCRIPTION_FETCH_USER_AGENTS,
    };
    use anyhow::anyhow;
    use async_trait::async_trait;
    use axum::{
        Router,
        extract::{Path, State},
        http::{HeaderMap, HeaderValue, StatusCode},
        routing::get,
    };
    use std::collections::HashSet;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::net::TcpListener;
    use tokio::sync::Notify;

    #[derive(Default)]
    struct TestRuntime {
        fail_controller_meta: bool,
        fail_shutdown: bool,
        apply_calls: AtomicUsize,
        shutdown_calls: AtomicUsize,
        payloads: TokioMutex<Vec<String>>,
    }

    impl TestRuntime {
        fn with_failures(fail_controller_meta: bool, fail_shutdown: bool) -> Self {
            Self {
                fail_controller_meta,
                fail_shutdown,
                apply_calls: AtomicUsize::new(0),
                shutdown_calls: AtomicUsize::new(0),
                payloads: TokioMutex::new(Vec::new()),
            }
        }
    }

    struct ApplyFailOnCallRuntime {
        fail_on_call: usize,
        apply_calls: AtomicUsize,
        payloads: TokioMutex<Vec<String>>,
    }

    impl ApplyFailOnCallRuntime {
        fn new(fail_on_call: usize) -> Self {
            Self {
                fail_on_call,
                apply_calls: AtomicUsize::new(0),
                payloads: TokioMutex::new(Vec::new()),
            }
        }
    }

    struct ApplyFailThroughCallRuntime {
        fail_through_call: usize,
        apply_calls: AtomicUsize,
        payloads: TokioMutex<Vec<String>>,
    }

    impl ApplyFailThroughCallRuntime {
        fn new(fail_through_call: usize) -> Self {
            Self {
                fail_through_call,
                apply_calls: AtomicUsize::new(0),
                payloads: TokioMutex::new(Vec::new()),
            }
        }
    }

    #[derive(Default)]
    struct CoordinatedRuntime {
        first_apply_started: Notify,
        allow_first_apply: Notify,
        apply_calls: AtomicUsize,
        payloads: TokioMutex<Vec<String>>,
    }

    #[derive(Default)]
    struct ConcurrentProbeRuntime {
        apply_calls: AtomicUsize,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
    }

    #[derive(Default)]
    struct FailProjectSyncConfigStore {
        inner: MemoryStore,
    }

    #[async_trait]
    impl BrokerStore for FailProjectSyncConfigStore {
        async fn list_projects(&self) -> anyhow::Result<Vec<String>> {
            self.inner.list_projects().await
        }

        async fn create_project(&self, project_id: &str, created_at: i64) -> anyhow::Result<()> {
            self.inner.create_project(project_id, created_at).await
        }

        async fn replace_subscription(
            &self,
            project_id: &str,
            nodes: &[ProxyNode],
        ) -> anyhow::Result<()> {
            self.inner.replace_subscription(project_id, nodes).await
        }

        async fn apply_subscription_snapshot(
            &self,
            project_id: &str,
            nodes: &[ProxyNode],
            ip_records: &[IpRecord],
            probe_records: &[ProbeRecord],
        ) -> anyhow::Result<()> {
            self.inner
                .apply_subscription_snapshot(project_id, nodes, ip_records, probe_records)
                .await
        }

        async fn list_subscription(&self, project_id: &str) -> anyhow::Result<Vec<ProxyNode>> {
            self.inner.list_subscription(project_id).await
        }

        async fn list_proxy_inventory(&self) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
            self.inner.list_proxy_inventory().await
        }

        async fn replace_proxy_inventory_scope(
            &self,
            source_scope: &ProxyScope,
            nodes: &[ProxyInventoryRecord],
        ) -> anyhow::Result<()> {
            self.inner
                .replace_proxy_inventory_scope(source_scope, nodes)
                .await
        }

        async fn get_proxy_inventory_node(
            &self,
            node_id: &str,
        ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
            self.inner.get_proxy_inventory_node(node_id).await
        }

        async fn list_proxy_imports(&self) -> anyhow::Result<Vec<ProxyImportRecord>> {
            self.inner.list_proxy_imports().await
        }

        async fn get_proxy_import(
            &self,
            import_id: &str,
        ) -> anyhow::Result<Option<ProxyImportRecord>> {
            self.inner.get_proxy_import(import_id).await
        }

        async fn replace_proxy_inventory_import(
            &self,
            import_record: &ProxyImportRecord,
            nodes: &[ProxyInventoryRecord],
        ) -> anyhow::Result<()> {
            self.inner
                .replace_proxy_inventory_import(import_record, nodes)
                .await
        }

        async fn list_proxy_inventory_for_import(
            &self,
            import_id: &str,
        ) -> anyhow::Result<Vec<ProxyInventoryRecord>> {
            self.inner.list_proxy_inventory_for_import(import_id).await
        }

        async fn update_proxy_inventory_allocation(
            &self,
            node_id: &str,
            allocation_scope: &ProxyScope,
            updated_at: i64,
        ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
            self.inner
                .update_proxy_inventory_allocation(node_id, allocation_scope, updated_at)
                .await
        }

        async fn update_proxy_import_allocation(
            &self,
            import_id: &str,
            allocation_scope: &ProxyScope,
            updated_at: i64,
        ) -> anyhow::Result<Option<ProxyImportRecord>> {
            self.inner
                .update_proxy_import_allocation(import_id, allocation_scope, updated_at)
                .await
        }

        async fn delete_proxy_inventory_node(
            &self,
            node_id: &str,
        ) -> anyhow::Result<Option<ProxyInventoryRecord>> {
            self.inner.delete_proxy_inventory_node(node_id).await
        }

        async fn delete_proxy_import(
            &self,
            import_id: &str,
        ) -> anyhow::Result<Option<ProxyImportRecord>> {
            self.inner.delete_proxy_import(import_id).await
        }

        async fn replace_ip_records(
            &self,
            project_id: &str,
            records: &[IpRecord],
        ) -> anyhow::Result<()> {
            self.inner.replace_ip_records(project_id, records).await
        }

        async fn upsert_ip_records(
            &self,
            project_id: &str,
            records: &[IpRecord],
        ) -> anyhow::Result<()> {
            self.inner.upsert_ip_records(project_id, records).await
        }

        async fn list_ip_records(&self, project_id: &str) -> anyhow::Result<Vec<IpRecord>> {
            self.inner.list_ip_records(project_id).await
        }

        async fn replace_probe_records(
            &self,
            project_id: &str,
            records: &[ProbeRecord],
        ) -> anyhow::Result<()> {
            self.inner.replace_probe_records(project_id, records).await
        }

        async fn upsert_probe_records(
            &self,
            project_id: &str,
            records: &[ProbeRecord],
        ) -> anyhow::Result<()> {
            self.inner.upsert_probe_records(project_id, records).await
        }

        async fn list_probe_records(&self, project_id: &str) -> anyhow::Result<Vec<ProbeRecord>> {
            self.inner.list_probe_records(project_id).await
        }

        async fn upsert_proxy_node_metadata(
            &self,
            records: &[ProxyNodeMetadataRecord],
        ) -> anyhow::Result<()> {
            self.inner.upsert_proxy_node_metadata(records).await
        }

        async fn list_proxy_node_metadata(&self) -> anyhow::Result<Vec<ProxyNodeMetadataRecord>> {
            self.inner.list_proxy_node_metadata().await
        }

        async fn insert_proxy_node_probe_samples(
            &self,
            records: &[ProxyNodeProbeSampleRecord],
        ) -> anyhow::Result<()> {
            self.inner.insert_proxy_node_probe_samples(records).await
        }

        async fn list_recent_proxy_node_probe_samples(
            &self,
            limit_per_node_ip: usize,
        ) -> anyhow::Result<Vec<ProxyNodeProbeSampleRecord>> {
            self.inner
                .list_recent_proxy_node_probe_samples(limit_per_node_ip)
                .await
        }

        async fn get_system_settings(&self) -> anyhow::Result<Option<SystemSettings>> {
            self.inner.get_system_settings().await
        }

        async fn upsert_system_settings(&self, settings: &SystemSettings) -> anyhow::Result<()> {
            self.inner.upsert_system_settings(settings).await
        }

        async fn insert_session(
            &self,
            project_id: &str,
            session: &SessionRecord,
        ) -> anyhow::Result<()> {
            self.inner.insert_session(project_id, session).await
        }

        async fn insert_sessions(
            &self,
            project_id: &str,
            sessions: &[SessionRecord],
        ) -> anyhow::Result<()> {
            self.inner.insert_sessions(project_id, sessions).await
        }

        async fn insert_sessions_with_touch(
            &self,
            project_id: &str,
            sessions: &[SessionRecord],
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .insert_sessions_with_touch(project_id, sessions, last_used_at)
                .await
        }

        async fn delete_session(&self, project_id: &str, session_id: &str) -> anyhow::Result<()> {
            self.inner.delete_session(project_id, session_id).await
        }

        async fn list_sessions(&self, project_id: &str) -> anyhow::Result<Vec<SessionRecord>> {
            self.inner.list_sessions(project_id).await
        }

        async fn list_project_node_usages(
            &self,
            project_id: &str,
        ) -> anyhow::Result<Vec<NodeUsageRecord>> {
            self.inner.list_project_node_usages(project_id).await
        }

        async fn list_session_node_usages(
            &self,
            project_id: &str,
            session_id: &str,
        ) -> anyhow::Result<Vec<NodeUsageRecord>> {
            self.inner
                .list_session_node_usages(project_id, session_id)
                .await
        }

        async fn insert_api_key(&self, api_key: &ApiKeyRecord) -> anyhow::Result<()> {
            self.inner.insert_api_key(api_key).await
        }

        async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
            self.inner.get_api_key(key_id).await
        }

        async fn list_api_keys(&self, owner_subject: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
            self.inner.list_api_keys(owner_subject).await
        }

        async fn revoke_api_key(
            &self,
            owner_subject: &str,
            key_id: &str,
            revoked_at: i64,
        ) -> anyhow::Result<bool> {
            self.inner
                .revoke_api_key(owner_subject, key_id, revoked_at)
                .await
        }

        async fn touch_api_key_last_used(
            &self,
            key_id: &str,
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .touch_api_key_last_used(key_id, last_used_at)
                .await
        }

        async fn touch_ip_usage(
            &self,
            project_id: &str,
            ip: &str,
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .touch_ip_usage(project_id, ip, last_used_at)
                .await
        }

        async fn touch_ip_usages(
            &self,
            project_id: &str,
            ips: &[String],
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .touch_ip_usages(project_id, ips, last_used_at)
                .await
        }

        async fn upsert_project_sync_config(
            &self,
            _config: &ProjectSyncConfig,
        ) -> anyhow::Result<()> {
            Err(anyhow!("sync config unavailable"))
        }

        async fn upsert_proxy_import_sync_config(
            &self,
            _config: &ProxyImportSyncConfig,
        ) -> anyhow::Result<()> {
            Err(anyhow!("sync config unavailable"))
        }

        async fn get_project_sync_config(
            &self,
            project_id: &str,
        ) -> anyhow::Result<Option<ProjectSyncConfig>> {
            self.inner.get_project_sync_config(project_id).await
        }

        async fn get_proxy_import_sync_config(
            &self,
            import_id: &str,
        ) -> anyhow::Result<Option<ProxyImportSyncConfig>> {
            self.inner.get_proxy_import_sync_config(import_id).await
        }

        async fn list_project_sync_configs(&self) -> anyhow::Result<Vec<ProjectSyncConfig>> {
            self.inner.list_project_sync_configs().await
        }

        async fn list_proxy_import_sync_configs(
            &self,
        ) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
            self.inner.list_proxy_import_sync_configs().await
        }

        async fn list_proxy_import_sync_configs_for_project(
            &self,
            project_id: &str,
        ) -> anyhow::Result<Vec<ProxyImportSyncConfig>> {
            self.inner
                .list_proxy_import_sync_configs_for_project(project_id)
                .await
        }

        async fn delete_proxy_import_sync_config(&self, import_id: &str) -> anyhow::Result<()> {
            self.inner.delete_proxy_import_sync_config(import_id).await
        }

        async fn get_project_proxy_settings(
            &self,
            project_id: &str,
        ) -> anyhow::Result<Option<ProjectProxySettings>> {
            self.inner.get_project_proxy_settings(project_id).await
        }

        async fn upsert_project_proxy_settings(
            &self,
            settings: &ProjectProxySettings,
        ) -> anyhow::Result<()> {
            self.inner.upsert_project_proxy_settings(settings).await
        }

        async fn insert_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
            self.inner.insert_task_run(run).await
        }

        async fn update_task_run(&self, run: &TaskRunRecord) -> anyhow::Result<()> {
            self.inner.update_task_run(run).await
        }

        async fn get_task_run(&self, run_id: &str) -> anyhow::Result<Option<TaskRunRecord>> {
            self.inner.get_task_run(run_id).await
        }

        async fn list_task_runs(
            &self,
            query: &TaskListQuery,
        ) -> anyhow::Result<Vec<TaskRunRecord>> {
            self.inner.list_task_runs(query).await
        }

        async fn insert_task_run_event(&self, event: &TaskRunEventRecord) -> anyhow::Result<()> {
            self.inner.insert_task_run_event(event).await
        }

        async fn list_task_run_events(
            &self,
            run_id: &str,
        ) -> anyhow::Result<Vec<TaskRunEventRecord>> {
            self.inner.list_task_run_events(run_id).await
        }
    }

    #[async_trait]
    impl MihomoRuntime for TestRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            self.shutdown_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_shutdown {
                return Err(anyhow!("shutdown unavailable"));
            }
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            if self.fail_controller_meta {
                return Err(anyhow!("controller unavailable"));
            }
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, project_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(project_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _project_id: &str, _payload: &str) -> anyhow::Result<()> {
            self.apply_calls.fetch_add(1, Ordering::SeqCst);
            self.payloads.lock().await.push(_payload.to_string());
            if self.fail_controller_meta {
                return Err(anyhow!("apply unavailable"));
            }
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    #[async_trait]
    impl MihomoRuntime for ApplyFailOnCallRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, project_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(project_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _project_id: &str, payload: &str) -> anyhow::Result<()> {
            let call = self.apply_calls.fetch_add(1, Ordering::SeqCst) + 1;
            self.payloads.lock().await.push(payload.to_string());
            if call == self.fail_on_call {
                return Err(anyhow!("apply failed on call {call}"));
            }
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    #[async_trait]
    impl MihomoRuntime for ApplyFailThroughCallRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, project_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(project_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _project_id: &str, payload: &str) -> anyhow::Result<()> {
            let call = self.apply_calls.fetch_add(1, Ordering::SeqCst) + 1;
            self.payloads.lock().await.push(payload.to_string());
            if call <= self.fail_through_call {
                return Err(anyhow!("apply failed on call {call}"));
            }
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    #[async_trait]
    impl MihomoRuntime for CoordinatedRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, project_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(project_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _project_id: &str, payload: &str) -> anyhow::Result<()> {
            let call_index = self.apply_calls.fetch_add(1, Ordering::SeqCst);
            if call_index == 0 {
                self.first_apply_started.notify_waiters();
                self.allow_first_apply.notified().await;
            }
            self.payloads.lock().await.push(payload.to_string());
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    #[async_trait]
    impl MihomoRuntime for ConcurrentProbeRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, project_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(project_id).await?;
            Ok(addr)
        }

        async fn apply_config(&self, _project_id: &str, _payload: &str) -> anyhow::Result<()> {
            self.apply_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
            proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(in_flight, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            if proxy_name.contains("fail") {
                Ok(None)
            } else {
                Ok(Some(87))
            }
        }
    }

    async fn write_subscription_file(content: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "proxy-broker-subscription-{}.yaml",
            ids::random_temp_suffix()
        ));
        tokio::fs::write(&path, content)
            .await
            .expect("subscription file should be written");
        path.to_string_lossy().to_string()
    }

    fn temp_sqlite_store_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "proxy-broker-service-{}.db",
            ids::random_temp_suffix()
        ))
    }

    fn make_node(proxy_name: &str, ip: &str) -> ProxyNode {
        let raw_proxy = serde_json::json!({
            "name": proxy_name,
            "type": "socks5",
            "server": ip
        });
        ProxyNode {
            node_id: Some(ids::stable_proxy_inventory_node_id_for_proxy(
                "test-import",
                proxy_name,
                "socks5",
                ip,
                &raw_proxy,
            )),
            proxy_name: proxy_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy,
        }
    }

    #[derive(Clone)]
    struct TestSubscriptionServerState {
        payload: Arc<str>,
        status: StatusCode,
        accepted_user_agent: Option<Arc<str>>,
        response_headers: HeaderMap,
    }

    async fn test_subscription_handler(
        State(state): State<TestSubscriptionServerState>,
        headers: HeaderMap,
    ) -> (StatusCode, HeaderMap, String) {
        let user_agent = headers
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok());
        if let Some(accepted_user_agent) = state.accepted_user_agent.as_deref()
            && user_agent != Some(accepted_user_agent)
        {
            return (
                StatusCode::OK,
                HeaderMap::new(),
                "invalid-without-compat-ua".to_string(),
            );
        }
        (
            state.status,
            state.response_headers.clone(),
            state.payload.to_string(),
        )
    }

    async fn spawn_subscription_server(
        payload: &'static str,
        status: StatusCode,
        accepted_user_agent: Option<&'static str>,
        response_headers: Option<HeaderMap>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                payload: Arc::<str>::from(payload),
                status,
                accepted_user_agent: accepted_user_agent.map(Arc::<str>::from),
                response_headers: response_headers.unwrap_or_default(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should expose local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        (format!("http://{addr}/subscription"), handle)
    }

    async fn test_online_geo_handler(
        Path(_ip): Path<String>,
        State(payload): State<Arc<str>>,
    ) -> (StatusCode, String) {
        (StatusCode::OK, payload.to_string())
    }

    async fn spawn_online_geo_server(
        payload: &'static str,
    ) -> (String, String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/mmdb", get(|| async { StatusCode::NOT_FOUND }))
            .route("/{ip}", get(test_online_geo_handler))
            .with_state(Arc::<str>::from(payload));
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should expose local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        (
            format!("http://{addr}"),
            format!("http://{addr}/mmdb"),
            handle,
        )
    }

    fn make_session(
        session_id: &str,
        proxy_name: &str,
        ip: &str,
        created_at: i64,
    ) -> SessionRecord {
        let raw_proxy = serde_json::json!({
            "name": proxy_name,
            "type": "socks5",
            "server": ip
        });
        let node_id = ids::stable_proxy_inventory_node_id_for_proxy(
            "test-import",
            proxy_name,
            "socks5",
            ip,
            &raw_proxy,
        );
        SessionRecord {
            session_id: session_id.to_string(),
            listen: "127.0.0.1".to_string(),
            port: 18080,
            selected_ip: ip.to_string(),
            proxy_name: proxy_name.to_string(),
            node_id: node_id.clone(),
            candidate_node_ids: vec![node_id],
            created_at,
        }
    }

    fn make_inventory_record(node_id: &str, proxy_name: &str, ip: &str) -> ProxyInventoryRecord {
        ProxyInventoryRecord {
            import_id: "test-import".to_string(),
            node_id: node_id.to_string(),
            source_scope: ProxyScope::global(),
            allocation_scope: ProxyScope::global(),
            proxy_name: proxy_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy: serde_json::json!({
                "name": proxy_name,
                "type": "socks5",
                "server": ip,
                "port": 1080
            }),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn reselect_session_from_inventory_uses_surviving_candidate() {
        let ip = "203.0.113.10";
        let mut session = make_session("s-candidate", "old-node", ip, 1);
        session.node_id = "node-old".to_string();
        session.candidate_node_ids = vec!["node-old".to_string(), "node-next".to_string()];
        let nodes = vec![make_inventory_record("node-next", "next-node", ip)];
        let metadata = HashMap::new();

        let resolved = reselect_session_from_inventory(&session, &nodes, &metadata)
            .expect("surviving candidate should keep the session");

        assert_eq!(resolved.node_id, "node-next");
        assert_eq!(resolved.proxy_name, "next-node");
        assert_eq!(resolved.candidate_node_ids, vec!["node-next"]);
    }

    #[tokio::test]
    async fn load_subscription_preserves_unrestorable_sessions_without_runtime_apply() {
        let project_id = "p-load";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("old", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(project_id, &make_session("s1", "old", "1.1.1.1", 1))
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
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await;

        let _ = tokio::fs::remove_file(&source_path).await;

        assert!(
            result.is_ok(),
            "stale sessions should not block subscription load"
        );
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 1);
        let sessions = store
            .list_sessions(project_id)
            .await
            .expect("list sessions should succeed");
        assert_eq!(
            sessions.len(),
            1,
            "unrestorable session should remain persisted"
        );
        assert_eq!(sessions[0].session_id, "s1");
    }

    #[tokio::test]
    async fn load_subscription_keeps_other_project_sessions_in_shared_runtime() {
        let stale_project_id = "p-load-stale";
        let active_project_id = "p-load-active";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(stale_project_id, &[make_node("old", "1.1.1.1")])
            .await
            .expect("seed stale subscription should succeed");
        store
            .insert_session(
                stale_project_id,
                &make_session("stale-session", "old", "1.1.1.1", 1),
            )
            .await
            .expect("seed stale session should succeed");
        store
            .replace_subscription(active_project_id, &[make_node("active", "3.3.3.3")])
            .await
            .expect("seed active subscription should succeed");
        store
            .insert_session(
                active_project_id,
                &make_session("active-session", "active", "3.3.3.3", 2),
            )
            .await
            .expect("seed active session should succeed");

        let runtime = Arc::new(TestRuntime::default());
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

        service
            .load_subscription(
                stale_project_id,
                &SubscriptionSource::File(source_path.clone()),
            )
            .await
            .expect("subscription load should succeed");
        let _ = tokio::fs::remove_file(&source_path).await;

        let stale_sessions = store
            .list_sessions(stale_project_id)
            .await
            .expect("stale sessions should list");
        assert_eq!(stale_sessions.len(), 1);
        assert_eq!(stale_sessions[0].session_id, "stale-session");
        assert_eq!(
            runtime.shutdown_calls.load(Ordering::SeqCst),
            0,
            "refreshing one project must not shutdown shared runtime for other active sessions"
        );
        let payloads = runtime.payloads.lock().await.clone();
        let latest_payload = payloads
            .last()
            .expect("shared runtime should be reconfigured for active sessions");
        assert!(latest_payload.contains("broker-active-session"));
        assert!(!latest_payload.contains("broker-stale-session"));
    }

    #[tokio::test]
    async fn opening_session_filters_unrestorable_sessions_from_other_projects() {
        let stale_project_id = "p-open-stale";
        let active_project_id = "p-open-active";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(stale_project_id, 1)
            .await
            .expect("stale project should be created");
        store
            .create_project(active_project_id, 1)
            .await
            .expect("active project should be created");
        store
            .insert_session(
                stale_project_id,
                &make_session("stale-open-session", "old-node", "203.0.113.90", 1),
            )
            .await
            .expect("seed stale session should succeed");
        let active_node = make_inventory_record("node-active-open", "active", "3.3.3.3");
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: active_node.import_id.clone(),
                    name: Some("active-import".to_string()),
                    import_kind: ProxyImportKind::SingleNode,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&active_node.import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                std::slice::from_ref(&active_node),
            )
            .await
            .expect("seed active inventory should succeed");
        store
            .replace_probe_records(
                active_project_id,
                &[sample_probe(&active_node.proxy_name, "3.3.3.3")],
            )
            .await
            .expect("seed active probe should succeed");

        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        service
            .open_session_by_node(
                active_project_id,
                &OpenSessionByNodeRequest {
                    node_id: active_node.node_id.clone(),
                    desired_port: Some(18_081),
                },
                None,
            )
            .await
            .expect("active session should open");

        let payloads = runtime.payloads.lock().await.clone();
        let latest_payload = payloads
            .last()
            .expect("shared runtime should be configured for opened session");
        assert!(latest_payload.contains("broker-"));
        assert!(
            !latest_payload.contains("broker-stale-open-session"),
            "unrestorable retained sessions from other projects must not render into runtime"
        );
        let stale_sessions = store
            .list_sessions(stale_project_id)
            .await
            .expect("stale sessions should list");
        assert_eq!(stale_sessions.len(), 1);
    }

    #[tokio::test]
    async fn opening_session_filters_unrestorable_sessions_from_same_project() {
        let project_id = "p-open-same-project-stale";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        store
            .insert_session(
                project_id,
                &make_session("same-project-stale-session", "old-node", "203.0.113.91", 1),
            )
            .await
            .expect("seed stale session should succeed");
        let active_node = make_inventory_record("node-same-project-active", "active", "3.3.3.4");
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: active_node.import_id.clone(),
                    name: Some("active-import".to_string()),
                    import_kind: ProxyImportKind::SingleNode,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&active_node.import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                std::slice::from_ref(&active_node),
            )
            .await
            .expect("seed active inventory should succeed");
        store
            .replace_probe_records(
                project_id,
                &[sample_probe(&active_node.proxy_name, "3.3.3.4")],
            )
            .await
            .expect("seed active probe should succeed");

        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        service
            .open_session_by_node(
                project_id,
                &OpenSessionByNodeRequest {
                    node_id: active_node.node_id.clone(),
                    desired_port: Some(18_082),
                },
                None,
            )
            .await
            .expect("active session should open despite retained stale row");

        let payloads = runtime.payloads.lock().await.clone();
        let latest_payload = payloads
            .last()
            .expect("shared runtime should be configured for opened session");
        assert!(
            !latest_payload.contains("broker-same-project-stale-session"),
            "same-project stale retained sessions must not render into runtime"
        );
        let sessions = store
            .list_sessions(project_id)
            .await
            .expect("sessions should list");
        assert!(
            sessions
                .iter()
                .any(|session| session.session_id == "same-project-stale-session"),
            "stale session should remain persisted"
        );
    }

    #[tokio::test]
    async fn load_subscription_returns_success_with_warning_when_post_load_bookkeeping_fails() {
        let project_id = "p-load-bookkeeping-warning";
        let store = Arc::new(FailProjectSyncConfigStore::default());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;

        let response = service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription import should still succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 1);
        assert_eq!(response.distinct_ips, 1);
        assert!(
            response
                .warnings
                .iter()
                .any(|warning| warning.contains("automatic task bookkeeping failed"))
        );

        let nodes = store
            .list_subscription(project_id)
            .await
            .expect("subscription query should succeed");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].proxy_name, "new");

        let sync_config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed");
        assert!(sync_config.is_none());

        let task_runs = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert!(task_runs.is_empty());
    }

    #[tokio::test]
    async fn close_session_allows_last_session_cleanup_without_runtime() {
        let project_id = "p-close";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("old", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(project_id, &make_session("s1", "old", "1.1.1.1", 1))
            .await
            .expect("seed session should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        service
            .close_session(project_id, "s1")
            .await
            .expect("closing last session should still succeed");

        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 1);
        assert!(
            store
                .list_sessions(project_id)
                .await
                .expect("list sessions should succeed")
                .is_empty(),
            "last session should be removed from store"
        );
    }

    #[tokio::test]
    async fn load_subscription_rejects_when_no_resolved_ips() {
        let project_id = "p-no-ip";
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
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await;
        let _ = tokio::fs::remove_file(&source_path).await;

        assert!(matches!(result, Err(BrokerError::SubscriptionInvalid)));
    }

    #[tokio::test]
    async fn proxy_latency_probe_rejects_nodes_without_primary_ip() {
        let project_id = "alpha";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let import_id = "import-alpha".to_string();
        let node_id = ids::stable_proxy_inventory_node_id_for_proxy(
            &import_id,
            "alpha-unresolved",
            "socks5",
            "edge-a.example.com",
            &serde_json::json!({
                "name": "alpha-unresolved",
                "type": "socks5",
                "server": "edge-a.example.com"
            }),
        );
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("alpha-manual".to_string()),
                    import_kind: ProxyImportKind::SingleNode,
                    source_scope: ProxyScope::project(project_id),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::project(project_id),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.clone(),
                    node_id: node_id.clone(),
                    source_scope: ProxyScope::project(project_id),
                    allocation_scope: ProxyScope::project(project_id),
                    proxy_name: "alpha-unresolved".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "edge-a.example.com".to_string(),
                    resolved_ips: Vec::new(),
                    raw_proxy: serde_json::json!({
                        "name": "alpha-unresolved",
                        "type": "socks5",
                        "server": "edge-a.example.com"
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("inventory import should seed");

        let mut run = TaskRunRecord {
            run_id: "run-alpha-probe-no-ip".to_string(),
            project_id: project_id.to_string(),
            kind: TaskRunKind::ProxyLatencyProbe,
            trigger: TaskRunTrigger::Operator,
            status: TaskRunStatus::Queued,
            stage: TaskRunStage::Queued,
            progress_current: Some(0),
            progress_total: None,
            created_at: 1,
            started_at: None,
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
            scope: TaskRunScope::Nodes {
                node_ids: vec![node_id.clone()],
            },
        };

        service
            .execute_proxy_latency_probe_task(&mut run)
            .await
            .expect_err("probe task should reject unresolved nodes");
        assert!(
            matches!(
                service
                    .queue_proxy_latency_probe(&ProxyOperationRequest {
                        view: "project".to_string(),
                        project_id: Some(project_id.to_string()),
                        node_ids: vec![node_id],
                    })
                    .await,
                Err(BrokerError::SubscriptionInvalid)
            ),
            "queueing should also reject unresolved nodes"
        );
        assert!(
            store
                .list_proxy_node_metadata()
                .await
                .expect("metadata should list")
                .is_empty(),
            "failed probe should not persist empty-ip metadata"
        );
    }

    #[tokio::test]
    async fn proxy_latency_probe_runs_nodes_concurrently_and_persists_recent_samples() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(ConcurrentProbeRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions {
                probe_concurrency: 2,
                ..BrokerServiceOptions::default()
            },
        );
        let import_id = "imp-global-probe".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("global-probe".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-fast".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "fast-node".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "8.8.8.8".to_string(),
                        resolved_ips: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "fast-node",
                            "type": "socks5",
                            "server": "8.8.8.8",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-fail".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "fail-node".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "9.9.9.9".to_string(),
                        resolved_ips: vec!["9.9.9.9".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "fail-node",
                            "type": "socks5",
                            "server": "9.9.9.9",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-unresolved".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "unresolved-node".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "unresolved.example.test".to_string(),
                        resolved_ips: Vec::new(),
                        raw_proxy: serde_json::json!({
                            "name": "unresolved-node",
                            "type": "socks5",
                            "server": "unresolved.example.test",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                ],
            )
            .await
            .expect("inventory should seed");

        let mut run = TaskRunRecord {
            run_id: "run-global-probe".to_string(),
            project_id: GLOBAL_RUNTIME_PROJECT_ID.to_string(),
            kind: TaskRunKind::ProxyLatencyProbe,
            trigger: TaskRunTrigger::Operator,
            status: TaskRunStatus::Queued,
            stage: TaskRunStage::Queued,
            progress_current: Some(0),
            progress_total: None,
            created_at: 1,
            started_at: None,
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
            scope: TaskRunScope::All,
        };

        service
            .execute_proxy_latency_probe_task(&mut run)
            .await
            .expect("probe should complete");

        assert!(
            runtime.max_in_flight.load(Ordering::SeqCst) >= 2,
            "batch probing should overlap at least two nodes"
        );
        assert_eq!(run.progress_current, Some((PROXY_PROBE_ROUNDS * 3) as u64));
        let samples = store
            .list_recent_proxy_node_probe_samples(10)
            .await
            .expect("samples should list");
        assert_eq!(samples.len(), PROXY_PROBE_ROUNDS * 3);
        let metadata = store
            .list_proxy_node_metadata()
            .await
            .expect("metadata should list");
        let fast = metadata
            .iter()
            .find(|record| record.node_id == "node-fast")
            .expect("fast node metadata should exist");
        assert_eq!(fast.last_probe_ok, Some(true));
        assert_eq!(fast.last_latency_ms, Some(87));
        assert_eq!(fast.recent_probe_samples.len(), 5);
        let fast_second_ip = metadata
            .iter()
            .find(|record| record.node_id == "node-fast" && record.ip == "8.8.4.4")
            .expect("secondary IP metadata should exist");
        assert_eq!(fast_second_ip.last_probe_ok, Some(true));
        assert_eq!(fast_second_ip.recent_probe_samples.len(), 5);
        let second = metadata
            .iter()
            .find(|record| record.node_id == "node-fail")
            .expect("second node metadata should exist");
        assert_eq!(second.last_probe_ok, Some(true));
        assert_eq!(second.last_latency_ms, Some(87));
        assert_eq!(second.recent_probe_samples.len(), 5);
        assert!(
            metadata
                .iter()
                .all(|record| record.node_id != "node-unresolved"),
            "all-node probe should skip unresolved inventory records"
        );
    }

    #[tokio::test]
    async fn queue_proxy_latency_probe_ignores_nodes_already_queued_for_probe() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let import_id = "imp-probe-dedupe".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("probe-dedupe".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-probing".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "already-probing".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "203.0.113.10".to_string(),
                        resolved_ips: vec!["203.0.113.10".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "already-probing",
                            "type": "socks5",
                            "server": "203.0.113.10",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-ready".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "ready".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "203.0.113.11".to_string(),
                        resolved_ips: vec!["203.0.113.11".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "ready",
                            "type": "socks5",
                            "server": "203.0.113.11",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                ],
            )
            .await
            .expect("inventory should seed");

        let first = service
            .queue_proxy_latency_probe(&ProxyOperationRequest {
                view: "global".to_string(),
                project_id: None,
                node_ids: vec!["node-probing".to_string()],
            })
            .await
            .expect("first probe should queue");
        let second = service
            .queue_proxy_latency_probe(&ProxyOperationRequest {
                view: "global".to_string(),
                project_id: None,
                node_ids: vec!["node-probing".to_string(), "node-ready".to_string()],
            })
            .await
            .expect("second probe should queue only non-duplicate node");
        let skipped = service
            .queue_proxy_latency_probe(&ProxyOperationRequest {
                view: "global".to_string(),
                project_id: None,
                node_ids: vec!["node-probing".to_string()],
            })
            .await
            .expect("all-duplicate probe should return a skipped task");

        let first_run = store
            .get_task_run(&first.run_id)
            .await
            .expect("first run lookup should succeed")
            .expect("first run should exist");
        assert!(matches!(
            first_run.scope,
            TaskRunScope::Nodes { ref node_ids } if node_ids == &vec!["node-probing".to_string()]
        ));

        let second_run = store
            .get_task_run(&second.run_id)
            .await
            .expect("second run lookup should succeed")
            .expect("second run should exist");
        assert!(matches!(
            second_run.scope,
            TaskRunScope::Nodes { ref node_ids } if node_ids == &vec!["node-ready".to_string()]
        ));
        assert_eq!(second_run.status, TaskRunStatus::Queued);

        let skipped_run = store
            .get_task_run(&skipped.run_id)
            .await
            .expect("skipped run lookup should succeed")
            .expect("skipped run should exist");
        assert_eq!(skipped_run.status, TaskRunStatus::Skipped);
        assert!(matches!(
            skipped_run.scope,
            TaskRunScope::Nodes { ref node_ids } if node_ids == &vec!["node-probing".to_string()]
        ));
        assert_eq!(
            skipped_run.summary_json,
            Some(serde_json::json!({
                "reason": "all_nodes_already_probing",
                "requested_nodes": 1,
                "ignored_nodes": 1,
                "ignored_node_ids": ["node-probing"],
            }))
        );
    }

    #[tokio::test]
    async fn scheduled_proxy_latency_probe_all_scope_targets_subscription_imports_only() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(ConcurrentProbeRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let subscription_import_id = "imp-scheduled-subscription".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: subscription_import_id.clone(),
                    name: Some("scheduled-subscription".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&subscription_import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: subscription_import_id.clone(),
                    node_id: "node-scheduled-subscription".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "scheduled-subscription".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "8.8.8.8".to_string(),
                    resolved_ips: vec!["8.8.8.8".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "scheduled-subscription",
                        "type": "socks5",
                        "server": "8.8.8.8",
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("subscription inventory should seed");
        let manual_import_id = "imp-scheduled-manual".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: manual_import_id.clone(),
                    name: Some("scheduled-manual".to_string()),
                    import_kind: ProxyImportKind::SingleNode,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&manual_import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: manual_import_id.clone(),
                    node_id: "node-scheduled-manual".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "scheduled-manual".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "9.9.9.9".to_string(),
                    resolved_ips: vec!["9.9.9.9".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "scheduled-manual",
                        "type": "socks5",
                        "server": "9.9.9.9",
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("manual inventory should seed");

        let mut run = TaskRunRecord {
            run_id: "run-scheduled-global-probe".to_string(),
            project_id: GLOBAL_RUNTIME_PROJECT_ID.to_string(),
            kind: TaskRunKind::ProxyLatencyProbe,
            trigger: TaskRunTrigger::Schedule,
            status: TaskRunStatus::Queued,
            stage: TaskRunStage::Queued,
            progress_current: Some(0),
            progress_total: None,
            created_at: 1,
            started_at: None,
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
            scope: TaskRunScope::All,
        };

        service
            .execute_proxy_latency_probe_task(&mut run)
            .await
            .expect("scheduled probe should complete");

        assert_eq!(run.progress_current, Some(PROXY_PROBE_ROUNDS as u64));
        let metadata = store
            .list_proxy_node_metadata()
            .await
            .expect("metadata should list");
        assert!(
            metadata
                .iter()
                .any(|record| record.node_id == "node-scheduled-subscription")
        );
        assert!(
            metadata
                .iter()
                .all(|record| record.node_id != "node-scheduled-manual"),
            "scheduled all-node probe must not target single-node imports"
        );
    }

    #[tokio::test]
    async fn load_subscription_from_url_accepts_ua_gated_payload() {
        let project_id = "p-url-success";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let (url, server) = spawn_subscription_server(
            r#"
proxies:
  - name: url-node
    type: socks5
    server: 8.8.8.8
"#,
            StatusCode::OK,
            Some(SUBSCRIPTION_FETCH_USER_AGENTS[1]),
            None,
        )
        .await;

        let result = service
            .load_subscription(project_id, &SubscriptionSource::Url(url))
            .await;

        server.abort();

        let response = result.expect("service should load url subscription");
        assert_eq!(response.loaded_proxies, 1);
        assert_eq!(response.distinct_ips, 1);
        assert_eq!(response.warnings.len(), 1);
        assert!(response.warnings[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[1]));
    }

    #[tokio::test]
    async fn load_subscription_from_url_maps_invalid_payload_to_subscription_invalid() {
        let project_id = "p-url-invalid";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let (url, server) =
            spawn_subscription_server("still-not-a-subscription", StatusCode::OK, None, None).await;

        let result = service
            .load_subscription(project_id, &SubscriptionSource::Url(url))
            .await;

        server.abort();

        assert!(
            matches!(result, Err(BrokerError::SubscriptionInvalidDetail(message))
                if message.contains("default request project")
                    && message.contains("shape: bytes="))
        );
    }

    #[tokio::test]
    async fn load_subscription_from_url_maps_non_2xx_to_subscription_fetch_failed() {
        let project_id = "p-url-fetch";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let (url, server) =
            spawn_subscription_server("blocked", StatusCode::FORBIDDEN, None, None).await;

        let result = service
            .load_subscription(project_id, &SubscriptionSource::Url(url))
            .await;

        server.abort();

        assert!(
            matches!(result, Err(BrokerError::SubscriptionFetch(message)) if message.contains("returned non-2xx"))
        );
    }

    #[tokio::test]
    async fn load_subscription_request_persists_explicit_import_name() {
        let project_id = "p-named-import";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: named-node
    type: socks5
    server: 8.8.4.4
"#,
        )
        .await;

        service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: Some("ops-feed".to_string()),
                    source: Some(SubscriptionSource::File(source_path.clone())),
                    content: None,
                },
            )
            .await
            .expect("named import should succeed");
        let _ = tokio::fs::remove_file(&source_path).await;

        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].name.as_deref(), Some("ops-feed"));
    }

    #[tokio::test]
    async fn load_subscription_request_uses_parsed_title_and_metadata_when_name_is_blank() {
        let project_id = "p-title-derived";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let mut response_headers = HeaderMap::new();
        response_headers.insert("profile-title", HeaderValue::from_static("edge-feed"));
        response_headers.insert(
            "x-clash-meta-subscription-userinfo",
            HeaderValue::from_static("upload=10; download=20; total=100; expire=1710000000"),
        );
        let (url, server) = spawn_subscription_server(
            r#"
proxies:
  - name: title-node
    type: socks5
    server: 8.8.4.4
"#,
            StatusCode::OK,
            None,
            Some(response_headers),
        )
        .await;

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::Url(url.clone())),
                    content: None,
                },
            )
            .await
            .expect("derived title import should succeed");

        server.abort();

        assert_eq!(response.resolved_name.as_deref(), Some("edge-feed"));
        assert_eq!(
            response.resolved_name_source,
            Some(ResolvedImportNameSource::ParsedSource)
        );
        let metadata = response
            .subscription_metadata
            .clone()
            .expect("response metadata should exist");
        assert_eq!(metadata.source_title.as_deref(), Some("edge-feed"));
        assert_eq!(metadata.used_bytes, Some(30));
        assert_eq!(metadata.remaining_bytes, Some(70));
        assert_eq!(metadata.expire_at, Some(1_710_000_000));

        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].name.as_deref(), Some("edge-feed"));
        assert_eq!(
            imports.items[0]
                .subscription_metadata
                .as_ref()
                .and_then(|item| item.source_title.as_deref()),
            Some("edge-feed")
        );
        assert_eq!(
            imports.items[0]
                .subscription_metadata
                .as_ref()
                .and_then(|item| item.remaining_bytes),
            Some(70)
        );
    }

    #[tokio::test]
    async fn load_subscription_request_uses_file_name_without_persisting_source_title() {
        let project_id = "p-file-name-derived";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let source_path = std::env::temp_dir().join("proxy-broker-file-derived-name.yaml");
        tokio::fs::write(
            &source_path,
            r#"
proxies:
  - name: file-node
    type: socks5
    server: 8.8.4.4
"#,
        )
        .await
        .expect("subscription file should be written");

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::File(
                        source_path.to_string_lossy().to_string(),
                    )),
                    content: None,
                },
            )
            .await
            .expect("file source import should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(
            response.resolved_name.as_deref(),
            Some("proxy-broker-file-derived-name")
        );
        assert_eq!(
            response.resolved_name_source,
            Some(ResolvedImportNameSource::ParsedSource)
        );
        assert!(response.subscription_metadata.is_none());

        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(
            imports.items[0].name.as_deref(),
            Some("proxy-broker-file-derived-name")
        );
        assert!(imports.items[0].subscription_metadata.is_none());
    }

    #[tokio::test]
    async fn load_subscription_request_uses_url_host_when_headers_do_not_name_import() {
        let project_id = "p-url-host-derived";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let app = Router::new()
            .route("/api/v1/client/abcdef123", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                payload: Arc::<str>::from(
                    r#"
proxies:
  - name: host-node
    type: socks5
    server: 8.8.4.4
"#,
                ),
                status: StatusCode::OK,
                accepted_user_agent: None,
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should expose local addr");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let url = format!("http://{addr}/api/v1/client/abcdef123?token=secret");

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::Url(url)),
                    content: None,
                },
            )
            .await
            .expect("host-derived import should succeed");

        server.abort();

        assert_eq!(response.resolved_name.as_deref(), Some("127.0.0.1"));
        assert_eq!(
            response.resolved_name_source,
            Some(ResolvedImportNameSource::ParsedSource)
        );
        assert!(response.subscription_metadata.is_none());

        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].name.as_deref(), Some("127.0.0.1"));
        assert!(imports.items[0].subscription_metadata.is_none());
    }

    #[tokio::test]
    async fn load_subscription_request_keeps_existing_name_over_new_parsed_title() {
        let project_id = "p-existing-name";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let mut response_headers = HeaderMap::new();
        response_headers.insert("profile-title", HeaderValue::from_static("edge-feed"));
        let (url, server) = spawn_subscription_server(
            r#"
proxies:
  - name: title-node
    type: socks5
    server: 8.8.4.4
"#,
            StatusCode::OK,
            None,
            Some(response_headers),
        )
        .await;

        service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: Some("ops-feed".to_string()),
                    source: Some(SubscriptionSource::Url(url.clone())),
                    content: None,
                },
            )
            .await
            .expect("seed import should succeed");

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::Url(url)),
                    content: None,
                },
            )
            .await
            .expect("blank rename import should succeed");

        server.abort();

        assert_eq!(response.resolved_name.as_deref(), Some("ops-feed"));
        assert_eq!(
            response.resolved_name_source,
            Some(ResolvedImportNameSource::ExistingImport)
        );
        assert_eq!(
            response
                .subscription_metadata
                .as_ref()
                .and_then(|item| item.source_title.as_deref()),
            Some("edge-feed")
        );
    }

    #[tokio::test]
    async fn load_subscription_request_filters_information_nodes_from_source_imports() {
        let project_id = "p-filtered-source";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: 剩余流量 12GB
    type: socks5
    server: 1.1.1.1
  - name: live-node
    type: socks5
    server: 8.8.4.4
"#,
        )
        .await;

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::File(source_path.clone())),
                    content: None,
                },
            )
            .await
            .expect("source import should keep the usable node");
        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 1);
        assert!(
            response
                .warnings
                .iter()
                .any(|warning| warning.contains("filtered informational subscription entry"))
        );
        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].proxy_count, 1);
    }

    #[tokio::test]
    async fn load_subscription_request_filters_malformed_hysteria_nodes_from_source_imports() {
        let project_id = "p-filtered-malformed-source";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: bad-hy
    type: hysteria
    server: 1.1.1.1
    up: ""
    down: ""
  - name: live-node
    type: socks5
    server: 8.8.4.4
"#,
        )
        .await;

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: Some(SubscriptionSource::File(source_path.clone())),
                    content: None,
                },
            )
            .await
            .expect("source import should keep the usable node");
        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 1);
        assert!(
            response
                .warnings
                .iter()
                .any(|warning| warning.contains("filtered malformed proxy entry `bad-hy`"))
        );
        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].proxy_count, 1);
    }

    #[tokio::test]
    async fn manual_node_group_import_autogenerates_group_name() {
        let project_id = "p-manual-group";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: None,
                    content: Some(
                        r#"
proxies:
  - name: hk-entry
    type: socks5
    server: 1.1.1.1
    port: 1080
  - name: jp-entry
    type: socks5
    server: 8.8.8.8
    port: 1080
"#
                        .to_string(),
                    ),
                },
            )
            .await
            .expect("manual node group import should succeed");

        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].import_kind, ProxyImportKind::SingleNode);
        assert_eq!(imports.items[0].proxy_count, 2);
        assert_eq!(imports.items[0].name.as_deref(), Some("hk-entry +1"));
    }

    #[tokio::test]
    async fn manual_node_group_import_filters_malformed_hysteria_nodes_and_warns() {
        let project_id = "p-manual-malformed-group";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let response = service
            .load_subscription_request(
                project_id,
                &LoadSubscriptionRequest {
                    name: None,
                    source: None,
                    content: Some(
                        r#"
proxies:
  - name: bad-hy
    type: hysteria
    server: 1.1.1.1
    up: ""
    down: ""
  - name: jp-entry
    type: socks5
    server: 8.8.8.8
    port: 1080
"#
                        .to_string(),
                    ),
                },
            )
            .await
            .expect("manual import should keep the usable node");

        assert_eq!(response.loaded_proxies, 1);
        assert!(
            response
                .warnings
                .iter()
                .any(|warning| warning.contains("filtered malformed proxy entry `bad-hy`"))
        );
        let imports = service
            .list_proxy_imports(Some("project"), Some(project_id))
            .await
            .expect("proxy imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].proxy_count, 1);
        assert_eq!(imports.items[0].name.as_deref(), Some("jp-entry"));
    }

    #[tokio::test]
    async fn global_pool_rebuilds_projects_and_opt_out_removes_global_nodes() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project("default")
            .await
            .expect("default project should be created");
        service
            .create_project("edge-jp")
            .await
            .expect("edge project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: global-node
    type: socks5
    server: 7.7.7.7
"#,
        )
        .await;

        service
            .load_global_subscription(&SubscriptionSource::File(source_path.clone()))
            .await
            .expect("global import should succeed");
        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(
            store
                .list_subscription("default")
                .await
                .expect("default subscription should list")
                .len(),
            1
        );
        assert_eq!(
            store
                .list_subscription("edge-jp")
                .await
                .expect("edge subscription should list")
                .len(),
            1
        );

        service
            .update_project_proxy_settings("edge-jp", false)
            .await
            .expect("opt-out should succeed");

        assert_eq!(
            store
                .list_subscription("default")
                .await
                .expect("default subscription should still list")
                .len(),
            1
        );
        assert!(
            store
                .list_subscription("edge-jp")
                .await
                .expect("edge subscription should list")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn project_local_import_wins_over_same_named_global_proxy() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project("default")
            .await
            .expect("default project should be created");
        service
            .create_project("edge-jp")
            .await
            .expect("edge project should be created");

        let global_source_path = write_subscription_file(
            r#"
proxies:
  - name: shared-node
    type: socks5
    server: 7.7.7.7
"#,
        )
        .await;
        let local_source_path = write_subscription_file(
            r#"
proxies:
  - name: shared-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;

        service
            .load_global_subscription(&SubscriptionSource::File(global_source_path.clone()))
            .await
            .expect("global import should succeed");
        service
            .load_subscription(
                "edge-jp",
                &SubscriptionSource::File(local_source_path.clone()),
            )
            .await
            .expect("local import should succeed");
        let _ = tokio::fs::remove_file(&global_source_path).await;
        let _ = tokio::fs::remove_file(&local_source_path).await;

        let default_nodes = store
            .list_subscription("default")
            .await
            .expect("default subscription should list");
        let edge_nodes = store
            .list_subscription("edge-jp")
            .await
            .expect("edge subscription should list");

        assert_eq!(default_nodes[0].server, "7.7.7.7");
        assert_eq!(edge_nodes.len(), 1);
        assert_eq!(edge_nodes[0].server, "1.1.1.1");

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("edge-jp".to_string()),
            })
            .await
            .expect("project catalog should list");
        let shared_nodes = catalog
            .groups
            .into_iter()
            .flat_map(|group| group.nodes.into_iter())
            .filter(|node| node.proxy_name == "shared-node")
            .collect::<Vec<_>>();
        assert_eq!(shared_nodes.len(), 2);
        assert_ne!(shared_nodes[0].node_id, shared_nodes[1].node_id);
    }

    #[tokio::test]
    async fn proxy_catalog_backfills_node_metadata_from_legacy_project_records() {
        let project_id = "edge-jp";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: edge-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        let _ = tokio::fs::remove_file(&source_path).await;

        store
            .replace_ip_records(
                project_id,
                &[IpRecord {
                    ip: "1.1.1.1".to_string(),
                    country_code: Some("JP".to_string()),
                    country_name: Some("Japan".to_string()),
                    region_name: Some("Tokyo".to_string()),
                    city: Some("Shibuya".to_string()),
                    geo_source: Some("legacy".to_string()),
                    probe_updated_at: Some(11),
                    geo_updated_at: Some(10),
                    last_used_at: None,
                }],
            )
            .await
            .expect("legacy ip record should seed");
        store
            .replace_probe_records(
                project_id,
                &[
                    ProbeRecord {
                        proxy_name: "edge-node".to_string(),
                        ip: "1.1.1.1".to_string(),
                        target_url: "https://example-a.test".to_string(),
                        ok: true,
                        latency_ms: Some(91),
                        updated_at: 11,
                    },
                    ProbeRecord {
                        proxy_name: "edge-node".to_string(),
                        ip: "1.1.1.1".to_string(),
                        target_url: "https://example-b.test".to_string(),
                        ok: false,
                        latency_ms: None,
                        updated_at: 12,
                    },
                ],
            )
            .await
            .expect("legacy probe record should seed");

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some(project_id.to_string()),
            })
            .await
            .expect("project catalog should list");
        let metadata = &catalog.groups[0].nodes[0].ip_metadata[0];
        assert_eq!(metadata.country_code.as_deref(), Some("JP"));
        assert_eq!(metadata.city.as_deref(), Some("Shibuya"));
        assert_eq!(metadata.last_probe_ok, Some(false));
        assert_eq!(metadata.median_latency_ms, Some(91));
        assert_eq!(metadata.last_probe_samples, vec![Some(91), None]);
    }

    #[tokio::test]
    async fn proxy_catalog_sanitizes_invalid_country_codes_from_legacy_project_records() {
        let project_id = "edge-jp";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: edge-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        let _ = tokio::fs::remove_file(&source_path).await;

        store
            .replace_ip_records(
                project_id,
                &[IpRecord {
                    ip: "1.1.1.1".to_string(),
                    country_code: Some("global".to_string()),
                    country_name: Some("Japan".to_string()),
                    region_name: Some("Tokyo".to_string()),
                    city: Some("Shibuya".to_string()),
                    geo_source: Some("legacy".to_string()),
                    probe_updated_at: Some(11),
                    geo_updated_at: Some(10),
                    last_used_at: None,
                }],
            )
            .await
            .expect("legacy ip record should seed");

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some(project_id.to_string()),
            })
            .await
            .expect("project catalog should list");
        let metadata = &catalog.groups[0].nodes[0].ip_metadata[0];
        assert_eq!(metadata.country_code, None);
        assert_eq!(metadata.country_name.as_deref(), Some("Japan"));
        assert_eq!(metadata.city.as_deref(), Some("Shibuya"));
    }

    #[tokio::test]
    async fn session_ip_node_options_backfill_legacy_project_geo_and_probe_metadata() {
        let project_id = "edge-jp";
        let import_id = "imp-edge-jp".to_string();
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("edge".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.clone(),
                    node_id: "node-edge".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "edge-node".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "1.1.1.1".to_string(),
                    resolved_ips: vec!["1.1.1.1".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "edge-node",
                        "type": "socks5",
                        "server": "1.1.1.1",
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("inventory should seed");

        store
            .replace_ip_records(
                project_id,
                &[IpRecord {
                    ip: "1.1.1.1".to_string(),
                    country_code: Some("JP".to_string()),
                    country_name: Some("Japan".to_string()),
                    region_name: Some("Tokyo".to_string()),
                    city: Some("Shibuya".to_string()),
                    geo_source: Some("legacy".to_string()),
                    probe_updated_at: Some(12),
                    geo_updated_at: Some(10),
                    last_used_at: None,
                }],
            )
            .await
            .expect("legacy ip record should seed");
        store
            .replace_probe_records(
                project_id,
                &[
                    ProbeRecord {
                        proxy_name: "edge-node".to_string(),
                        ip: "1.1.1.1".to_string(),
                        target_url: "https://example-a.test".to_string(),
                        ok: true,
                        latency_ms: Some(91),
                        updated_at: 11,
                    },
                    ProbeRecord {
                        proxy_name: "edge-node".to_string(),
                        ip: "1.1.1.1".to_string(),
                        target_url: "https://example-b.test".to_string(),
                        ok: false,
                        latency_ms: None,
                        updated_at: 12,
                    },
                ],
            )
            .await
            .expect("legacy probe record should seed");
        store
            .upsert_proxy_node_metadata(&[ProxyNodeMetadataRecord {
                node_id: "node-edge".to_string(),
                ip: "1.1.1.1".to_string(),
                country_code: None,
                country_name: None,
                region_name: None,
                city: None,
                geo_source: None,
                probe_updated_at: Some(30),
                geo_updated_at: None,
                last_probe_ok: Some(true),
                last_latency_ms: Some(40),
                median_latency_ms: Some(40),
                last_probe_samples: vec![Some(40)],
                recent_probe_samples: Vec::new(),
                updated_at: 30,
            }])
            .await
            .expect("partial node metadata should seed");
        let legacy_metadata = service
            .load_legacy_project_metadata(&[project_id.to_string()])
            .await
            .expect("legacy metadata should load");
        let effective_records = service
            .compose_effective_proxy_inventory_records(project_id)
            .await
            .expect("effective records should load");
        let backfilled = service
            .backfill_proxy_node_metadata(
                &effective_records[0],
                "1.1.1.1",
                &[project_id.to_string()],
                &legacy_metadata,
            )
            .expect("legacy metadata should backfill");
        assert_eq!(backfilled.last_probe_ok, Some(false));

        let options = service
            .search_session_ip_node_options(
                project_id,
                &SearchSessionIpNodeOptionsRequest::default(),
            )
            .await
            .expect("ip node options should load");
        let item = options
            .groups
            .iter()
            .flat_map(|group| group.items.iter())
            .find(|item| item.ip == "1.1.1.1")
            .expect("ip option should exist");
        assert_eq!(item.country_code.as_deref(), Some("JP"));
        assert_eq!(item.city.as_deref(), Some("Shibuya"));
        assert_eq!(item.nodes[0].country_code.as_deref(), Some("JP"));
        assert_eq!(item.nodes[0].city.as_deref(), Some("Shibuya"));
        assert_eq!(item.best_latency_ms, Some(40));
        assert_eq!(item.nodes[0].last_probe_ok, Some(true));
        assert_eq!(item.nodes[0].median_latency_ms, Some(40));
        assert_eq!(item.nodes[0].recent_probe_samples.len(), 1);
        assert_eq!(item.nodes[0].recent_probe_samples[0].latency_ms, Some(40));
    }

    #[tokio::test]
    async fn session_node_options_prefer_primary_ip_legacy_metadata_over_other_ip_node_metadata() {
        let project_id = "edge-jp";
        let import_id = "imp-multi-ip".to_string();
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("multi-ip".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.clone(),
                    node_id: "node-multi".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "multi-node".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "1.1.1.1".to_string(),
                    resolved_ips: vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "multi-node",
                        "type": "socks5",
                        "server": "1.1.1.1",
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("inventory should seed");
        store
            .upsert_proxy_node_metadata(&[ProxyNodeMetadataRecord {
                node_id: "node-multi".to_string(),
                ip: "2.2.2.2".to_string(),
                country_code: Some("US".to_string()),
                country_name: Some("United States".to_string()),
                region_name: Some("California".to_string()),
                city: Some("Los Angeles".to_string()),
                geo_source: Some("node".to_string()),
                probe_updated_at: None,
                geo_updated_at: Some(1),
                last_probe_ok: None,
                last_latency_ms: None,
                median_latency_ms: None,
                last_probe_samples: Vec::new(),
                recent_probe_samples: Vec::new(),
                updated_at: 1,
            }])
            .await
            .expect("secondary node metadata should seed");
        store
            .replace_ip_records(
                project_id,
                &[IpRecord {
                    ip: "1.1.1.1".to_string(),
                    country_code: Some("JP".to_string()),
                    country_name: Some("Japan".to_string()),
                    region_name: Some("Tokyo".to_string()),
                    city: Some("Shibuya".to_string()),
                    geo_source: Some("legacy".to_string()),
                    probe_updated_at: None,
                    geo_updated_at: Some(2),
                    last_used_at: None,
                }],
            )
            .await
            .expect("primary legacy metadata should seed");
        store
            .insert_session(
                project_id,
                &SessionRecord {
                    session_id: "sess-primary".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: 10080,
                    selected_ip: "1.1.1.1".to_string(),
                    proxy_name: "multi-node".to_string(),
                    node_id: "node-multi".to_string(),
                    candidate_node_ids: vec!["node-multi".to_string()],
                    created_at: 3,
                },
            )
            .await
            .expect("session should seed");

        let options = service
            .search_session_node_options(
                project_id,
                "sess-primary",
                &SearchSessionNodeOptionsRequest::default(),
            )
            .await
            .expect("node options should load");
        let item = options
            .items
            .iter()
            .find(|item| item.node_id == "node-multi")
            .expect("node option should exist");
        assert_eq!(item.primary_ip.as_deref(), Some("1.1.1.1"));
        assert_eq!(item.country_code.as_deref(), Some("JP"));
        assert_eq!(item.city.as_deref(), Some("Shibuya"));
    }

    #[test]
    fn merge_backfilled_proxy_node_metadata_preserves_existing_geo_on_empty_legacy_geo() {
        let backfilled = ProxyNodeMetadataRecord {
            node_id: "node-edge".to_string(),
            ip: "1.1.1.1".to_string(),
            country_code: None,
            country_name: None,
            region_name: None,
            city: None,
            geo_source: Some("none".to_string()),
            probe_updated_at: None,
            geo_updated_at: Some(20),
            last_probe_ok: None,
            last_latency_ms: None,
            median_latency_ms: None,
            last_probe_samples: Vec::new(),
            recent_probe_samples: Vec::new(),
            updated_at: 20,
        };
        let existing = ProxyNodeMetadataRecord {
            node_id: "node-edge".to_string(),
            ip: "1.1.1.1".to_string(),
            country_code: Some("JP".to_string()),
            country_name: Some("Japan".to_string()),
            region_name: Some("Tokyo".to_string()),
            city: Some("Shibuya".to_string()),
            geo_source: Some("online".to_string()),
            probe_updated_at: None,
            geo_updated_at: Some(10),
            last_probe_ok: None,
            last_latency_ms: None,
            median_latency_ms: None,
            last_probe_samples: Vec::new(),
            recent_probe_samples: Vec::new(),
            updated_at: 10,
        };

        let merged = merge_backfilled_proxy_node_metadata(backfilled, Some(&existing), None);
        assert_eq!(merged.country_code.as_deref(), Some("JP"));
        assert_eq!(merged.country_name.as_deref(), Some("Japan"));
        assert_eq!(merged.region_name.as_deref(), Some("Tokyo"));
        assert_eq!(merged.city.as_deref(), Some("Shibuya"));
        assert_eq!(merged.geo_source.as_deref(), Some("online"));
        assert_eq!(merged.geo_updated_at, Some(10));
    }

    #[test]
    fn merge_backfilled_proxy_node_metadata_preserves_newer_existing_observations() {
        let backfilled = ProxyNodeMetadataRecord {
            node_id: "node-edge".to_string(),
            ip: "1.1.1.1".to_string(),
            country_code: Some("US".to_string()),
            country_name: Some("United States".to_string()),
            region_name: Some("California".to_string()),
            city: Some("Los Angeles".to_string()),
            geo_source: Some("legacy".to_string()),
            probe_updated_at: Some(20),
            geo_updated_at: Some(20),
            last_probe_ok: Some(true),
            last_latency_ms: Some(120),
            median_latency_ms: Some(120),
            last_probe_samples: vec![Some(120)],
            recent_probe_samples: Vec::new(),
            updated_at: 20,
        };
        let existing = ProxyNodeMetadataRecord {
            node_id: "node-edge".to_string(),
            ip: "1.1.1.1".to_string(),
            country_code: Some("JP".to_string()),
            country_name: Some("Japan".to_string()),
            region_name: Some("Tokyo".to_string()),
            city: Some("Shibuya".to_string()),
            geo_source: Some("online".to_string()),
            probe_updated_at: Some(30),
            geo_updated_at: Some(30),
            last_probe_ok: Some(true),
            last_latency_ms: Some(50),
            median_latency_ms: Some(50),
            last_probe_samples: vec![Some(50)],
            recent_probe_samples: Vec::new(),
            updated_at: 30,
        };

        let merged = merge_backfilled_proxy_node_metadata(backfilled, Some(&existing), None);
        assert_eq!(merged.country_code.as_deref(), Some("JP"));
        assert_eq!(merged.city.as_deref(), Some("Shibuya"));
        assert_eq!(merged.geo_source.as_deref(), Some("online"));
        assert_eq!(merged.geo_updated_at, Some(30));
        assert_eq!(merged.probe_updated_at, Some(30));
        assert_eq!(merged.last_latency_ms, Some(50));
        assert_eq!(merged.median_latency_ms, Some(50));
        assert_eq!(merged.last_probe_samples, vec![Some(50)]);
    }

    #[tokio::test]
    async fn refresh_persists_project_inventory_node_geo_metadata() {
        let (online_geo_base, mmdb_url, server) = spawn_online_geo_server(
            r#"{"success":true,"country_code":"JP","country":"Japan","region":"Tokyo","city":"Shibuya"}"#,
        )
        .await;
        let data_dir = std::env::temp_dir().join(format!(
            "proxy-broker-online-geo-test-{}",
            crate::ids::random_temp_suffix()
        ));
        tokio::fs::create_dir_all(&data_dir)
            .await
            .expect("temp data dir should be created");
        let project_id = "edge-jp";
        let options = BrokerServiceOptions {
            online_geo_base,
            mmdb_url,
            data_dir: data_dir.clone(),
            ..BrokerServiceOptions::default()
        };
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, options);
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: edge-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        let _ = tokio::fs::remove_file(&source_path).await;

        service
            .refresh(project_id, &RefreshRequest { force: true })
            .await
            .expect("refresh should succeed");

        server.abort();
        let _ = tokio::fs::remove_dir_all(&data_dir).await;

        let metadata = store
            .list_proxy_node_metadata()
            .await
            .expect("metadata should list");
        let record = metadata
            .iter()
            .find(|item| item.ip == "1.1.1.1")
            .expect("node metadata should be persisted");
        assert_eq!(record.country_code.as_deref(), Some("JP"));
        assert_eq!(record.country_name.as_deref(), Some("Japan"));
        assert_eq!(record.region_name.as_deref(), Some("Tokyo"));
        assert_eq!(record.city.as_deref(), Some("Shibuya"));
        assert_eq!(record.geo_source.as_deref(), Some("online"));
    }

    #[tokio::test]
    async fn refresh_geo_records_clears_malformed_online_country_code_only_responses() {
        let (online_geo_base, mmdb_url, server) =
            spawn_online_geo_server(r#"{"success":true,"country_code":"global"}"#).await;
        let data_dir = std::env::temp_dir().join(format!(
            "proxy-broker-online-geo-test-{}",
            crate::ids::random_temp_suffix()
        ));
        tokio::fs::create_dir_all(&data_dir)
            .await
            .expect("temp data dir should be created");
        let options = BrokerServiceOptions {
            online_geo_base,
            mmdb_url,
            data_dir: data_dir.clone(),
            ..BrokerServiceOptions::default()
        };

        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, options);
        let now = 1_713_309_999;
        let mut ip_records = vec![IpRecord {
            ip: "1.1.1.1".to_string(),
            country_code: Some("US".to_string()),
            country_name: Some("United States".to_string()),
            region_name: Some("California".to_string()),
            city: Some("San Jose".to_string()),
            geo_source: Some("legacy".to_string()),
            probe_updated_at: None,
            geo_updated_at: Some(now - 60),
            last_used_at: None,
        }];

        let changed = service
            .refresh_geo_records("default", true, now, &mut ip_records, None)
            .await
            .expect("refresh should succeed");

        server.abort();
        let _ = tokio::fs::remove_dir_all(&data_dir).await;

        assert_eq!(changed, 1);
        assert_eq!(ip_records[0].country_code, None);
        assert_eq!(ip_records[0].country_name, None);
        assert_eq!(ip_records[0].region_name, None);
        assert_eq!(ip_records[0].city, None);
        assert_eq!(ip_records[0].geo_source.as_deref(), Some("none"));
        assert_eq!(ip_records[0].geo_updated_at, Some(now));
    }

    #[tokio::test]
    async fn refresh_geo_records_clears_stale_country_code_when_online_country_code_is_malformed() {
        let (online_geo_base, mmdb_url, server) = spawn_online_geo_server(
            r#"{"success":true,"country_code":"global","country":"Japan","city":"Tokyo"}"#,
        )
        .await;
        let data_dir = std::env::temp_dir().join(format!(
            "proxy-broker-online-geo-test-{}",
            crate::ids::random_temp_suffix()
        ));
        tokio::fs::create_dir_all(&data_dir)
            .await
            .expect("temp data dir should be created");
        let options = BrokerServiceOptions {
            online_geo_base,
            mmdb_url,
            data_dir: data_dir.clone(),
            ..BrokerServiceOptions::default()
        };

        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, options);
        let now = 1_713_309_999;
        let mut ip_records = vec![IpRecord {
            ip: "1.1.1.1".to_string(),
            country_code: Some("US".to_string()),
            country_name: Some("United States".to_string()),
            region_name: Some("California".to_string()),
            city: Some("San Jose".to_string()),
            geo_source: Some("legacy".to_string()),
            probe_updated_at: None,
            geo_updated_at: Some(now - 60),
            last_used_at: None,
        }];

        let changed = service
            .refresh_geo_records("default", true, now, &mut ip_records, None)
            .await
            .expect("refresh should succeed");

        server.abort();
        let _ = tokio::fs::remove_dir_all(&data_dir).await;

        assert_eq!(changed, 1);
        assert_eq!(ip_records[0].country_code, None);
        assert_eq!(ip_records[0].country_name.as_deref(), Some("Japan"));
        assert_eq!(ip_records[0].city.as_deref(), Some("Tokyo"));
        assert_eq!(ip_records[0].geo_source.as_deref(), Some("online"));
        assert_eq!(ip_records[0].geo_updated_at, Some(now));
    }

    #[tokio::test]
    async fn load_subscription_accepts_duplicate_proxy_names_with_distinct_node_ids() {
        let project_id = "dup-names";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: dup
    type: socks5
    server: 1.1.1.1
  - name: dup
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;

        let response = service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("duplicate-name subscription should load");
        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 2);

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some(project_id.to_string()),
            })
            .await
            .expect("project catalog should list");
        let duplicate_nodes = catalog
            .groups
            .into_iter()
            .flat_map(|group| group.nodes.into_iter())
            .filter(|node| node.proxy_name == "dup")
            .collect::<Vec<_>>();
        assert_eq!(duplicate_nodes.len(), 2);
        assert_ne!(duplicate_nodes[0].node_id, duplicate_nodes[1].node_id);
    }

    #[tokio::test]
    async fn open_session_by_node_returns_full_listen_endpoint() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("edge-jp")
            .await
            .expect("edge project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: edge-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription("edge-jp", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("local import should succeed");
        service
            .refresh("edge-jp", &RefreshRequest { force: true })
            .await
            .expect("edge probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("edge-jp".to_string()),
            })
            .await
            .expect("project catalog should list");
        let node_id = catalog.groups[0].nodes[0].node_id.clone();

        let response = service
            .open_session_by_node(
                "edge-jp",
                &OpenSessionByNodeRequest {
                    node_id,
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("node-pinned open should succeed");

        assert!(response.port > 0);
        assert_eq!(response.listen, format!("127.0.0.1:{}", response.port));
        assert_eq!(response.bind_host, "127.0.0.1");
        assert_eq!(response.display_host, "127.0.0.1");
        assert_eq!(
            response.display_address,
            format!("127.0.0.1:{}", response.port)
        );
    }

    #[tokio::test]
    async fn update_session_node_keeps_listener_and_tracks_recent_usage() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: zzz-node
    type: socks5
    server: 1.1.1.1
  - name: aaa-node
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let opened = service
            .open_session(
                "browser",
                &OpenSessionRequest {
                    selection_mode: SessionSelectionMode::Ip,
                    specified_ips: vec!["1.1.1.1".to_string()],
                    desired_port: Some(10080),
                    ..OpenSessionRequest::default()
                },
                None,
            )
            .await
            .expect("session should open");

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("project catalog should list");
        let target_node = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .find(|node| node.proxy_name == "aaa-node")
            .expect("target node should exist");

        tokio::time::sleep(Duration::from_millis(1_100)).await;
        let before_switch = service
            .list_sessions("browser", None)
            .await
            .expect("sessions should list before switch");
        let original_created_at = before_switch.sessions[0].created_at;

        let updated = service
            .update_session_node(
                "browser",
                &opened.session_id,
                &UpdateSessionNodeRequest {
                    node_id: target_node.node_id.clone(),
                    selected_ip: None,
                    candidate_node_ids: Vec::new(),
                },
                None,
            )
            .await
            .expect("session node should switch");

        assert_eq!(updated.session_id, opened.session_id);
        assert_eq!(updated.listen, opened.listen);
        assert_eq!(updated.bind_host, opened.bind_host);
        assert_eq!(updated.display_host, opened.display_host);
        assert_eq!(updated.display_address, opened.display_address);
        assert_eq!(updated.port, opened.port);
        assert_eq!(updated.proxy_name, "aaa-node");
        assert_eq!(updated.selected_ip, "2.2.2.2");

        let sessions = service
            .list_sessions("browser", None)
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].session_id, opened.session_id);
        assert_eq!(sessions.sessions[0].created_at, original_created_at);
        assert_eq!(sessions.sessions[0].listen, "127.0.0.1:10080");
        assert_eq!(sessions.sessions[0].port, 10080);
        assert_eq!(sessions.sessions[0].node_id, target_node.node_id);

        let sorted = service
            .search_session_node_options(
                "browser",
                &opened.session_id,
                &SearchSessionNodeOptionsRequest {
                    sort_mode: SessionNodeSortMode::SessionRecent,
                    ..SearchSessionNodeOptionsRequest::default()
                },
            )
            .await
            .expect("node options should load");
        assert_eq!(sorted.items[0].proxy_name, "aaa-node");
        assert!(sorted.items[0].session_last_used_at.is_some());
        assert!(sorted.items[1].session_last_used_at.is_some());

        let filtered = service
            .search_session_node_options(
                "browser",
                &opened.session_id,
                &SearchSessionNodeOptionsRequest {
                    query: Some("2.2.2.2".to_string()),
                    sort_mode: SessionNodeSortMode::ProjectRecent,
                    ..SearchSessionNodeOptionsRequest::default()
                },
            )
            .await
            .expect("filtered node options should load");
        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.items[0].proxy_name, "aaa-node");
    }

    #[tokio::test]
    async fn search_session_node_options_returns_all_effective_nodes_without_limit() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");

        let proxies = (0..55)
            .map(|index| {
                format!("  - name: node-{index:02}\n    type: socks5\n    server: 10.0.0.{index}\n")
            })
            .collect::<Vec<_>>()
            .join("");
        let source_path = write_subscription_file(&format!("proxies:\n{proxies}")).await;
        service
            .load_subscription("browser", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("seed subscription should succeed");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("project catalog should list");
        let first_node_id = catalog.groups[0].nodes[0].node_id.clone();

        let opened = service
            .open_session_by_node(
                "browser",
                &OpenSessionByNodeRequest {
                    node_id: first_node_id,
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("session should open");

        let all_options = service
            .search_session_node_options(
                "browser",
                &opened.session_id,
                &SearchSessionNodeOptionsRequest {
                    sort_mode: SessionNodeSortMode::ProjectRecent,
                    limit: None,
                    ..SearchSessionNodeOptionsRequest::default()
                },
            )
            .await
            .expect("node options should load");
        assert_eq!(all_options.items.len(), 55);

        let limited_options = service
            .search_session_node_options(
                "browser",
                &opened.session_id,
                &SearchSessionNodeOptionsRequest {
                    sort_mode: SessionNodeSortMode::ProjectRecent,
                    limit: Some(50),
                    ..SearchSessionNodeOptionsRequest::default()
                },
            )
            .await
            .expect("limited node options should load");
        assert_eq!(limited_options.items.len(), 50);
    }

    #[tokio::test]
    async fn list_sessions_includes_selected_ip_geo_metadata() {
        let project_id = "geo-browser";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let node = make_node("jp-tokyo-entry", "203.0.113.10");
        let node_id = node
            .node_id
            .clone()
            .expect("test node should include a node id");
        store
            .replace_subscription(project_id, &[node])
            .await
            .expect("seed subscription should succeed");
        store
            .upsert_proxy_node_metadata(&[ProxyNodeMetadataRecord {
                node_id: node_id.clone(),
                ip: "203.0.113.10".to_string(),
                country_code: Some("JP".to_string()),
                country_name: Some("Japan".to_string()),
                region_name: Some("Tokyo".to_string()),
                city: Some("Chiyoda".to_string()),
                geo_source: Some("fixture".to_string()),
                probe_updated_at: Some(10),
                geo_updated_at: Some(11),
                last_probe_ok: Some(true),
                last_latency_ms: Some(88),
                median_latency_ms: Some(88),
                last_probe_samples: vec![Some(88)],
                recent_probe_samples: Vec::new(),
                updated_at: 12,
            }])
            .await
            .expect("metadata should be stored");
        store
            .insert_session(
                project_id,
                &make_session("s1", "jp-tokyo-entry", "203.0.113.10", 1),
            )
            .await
            .expect("seed session should succeed");

        let sessions = service
            .list_sessions(project_id, None)
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].node_id, node_id);
        assert_eq!(sessions.sessions[0].country_code.as_deref(), Some("JP"));
        assert_eq!(sessions.sessions[0].country_name.as_deref(), Some("Japan"));
        assert_eq!(sessions.sessions[0].region_name.as_deref(), Some("Tokyo"));
        assert_eq!(sessions.sessions[0].city.as_deref(), Some("Chiyoda"));
    }

    #[tokio::test]
    async fn list_sessions_uses_request_host_for_wildcard_display_address() {
        let project_id = "display-wildcard";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let mut session = make_session("s-display", "wild-node", "203.0.113.10", 1);
        session.listen = "0.0.0.0".to_string();
        session.port = 20002;
        store
            .replace_subscription(project_id, &[make_node("wild-node", "203.0.113.10")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(project_id, &session)
            .await
            .expect("seed session should succeed");

        let sessions = service
            .list_sessions(project_id, Some("panel.example.test"))
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions[0].listen, "0.0.0.0:20002");
        assert_eq!(sessions.sessions[0].bind_host, "0.0.0.0");
        assert_eq!(sessions.sessions[0].display_host, "panel.example.test");
        assert_eq!(
            sessions.sessions[0].display_address,
            "panel.example.test:20002"
        );
    }

    #[tokio::test]
    async fn search_session_ip_node_options_keeps_stable_groups_and_probe_history() {
        let project_id = "ip-node-options";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let first_import = ProxyImportRecord {
            import_id: "import-one".to_string(),
            name: Some("shared-name".to_string()),
            import_kind: ProxyImportKind::SingleNode,
            source_scope: ProxyScope::project(project_id),
            source_identity: ProxyImportSourceIdentity::manual("one"),
            allocation_scope: ProxyScope::project(project_id),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let second_import = ProxyImportRecord {
            import_id: "import-two".to_string(),
            name: Some("shared-name".to_string()),
            import_kind: ProxyImportKind::SingleNode,
            source_scope: ProxyScope::project(project_id),
            source_identity: ProxyImportSourceIdentity::manual("two"),
            allocation_scope: ProxyScope::project(project_id),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let first_node = ProxyInventoryRecord {
            import_id: first_import.import_id.clone(),
            node_id: "node-one".to_string(),
            source_scope: ProxyScope::project(project_id),
            allocation_scope: ProxyScope::project(project_id),
            proxy_name: "node-one".to_string(),
            proxy_type: "socks5".to_string(),
            server: "203.0.113.10".to_string(),
            resolved_ips: vec!["203.0.113.10".to_string()],
            raw_proxy: serde_json::json!({"name": "node-one", "type": "socks5", "server": "203.0.113.10"}),
            created_at: 1,
            updated_at: 1,
        };
        let second_node = ProxyInventoryRecord {
            import_id: second_import.import_id.clone(),
            node_id: "node-two".to_string(),
            source_scope: ProxyScope::project(project_id),
            allocation_scope: ProxyScope::project(project_id),
            proxy_name: "node-two".to_string(),
            proxy_type: "socks5".to_string(),
            server: "203.0.113.10".to_string(),
            resolved_ips: vec!["203.0.113.10".to_string()],
            raw_proxy: serde_json::json!({"name": "node-two", "type": "socks5", "server": "203.0.113.10"}),
            created_at: 1,
            updated_at: 1,
        };
        store
            .replace_proxy_inventory_import(&first_import, std::slice::from_ref(&first_node))
            .await
            .expect("first import should persist");
        store
            .replace_proxy_inventory_import(&second_import, std::slice::from_ref(&second_node))
            .await
            .expect("second import should persist");
        store
            .upsert_proxy_node_metadata(&[ProxyNodeMetadataRecord {
                node_id: first_node.node_id.clone(),
                ip: "203.0.113.10".to_string(),
                country_code: Some("JP".to_string()),
                country_name: Some("Japan".to_string()),
                region_name: Some("Tokyo".to_string()),
                city: Some("Chiyoda".to_string()),
                geo_source: Some("fixture".to_string()),
                probe_updated_at: Some(10),
                geo_updated_at: Some(10),
                last_probe_ok: Some(true),
                last_latency_ms: Some(88),
                median_latency_ms: Some(88),
                last_probe_samples: vec![Some(88)],
                recent_probe_samples: vec![ProxyNodeProbeSampleRecord {
                    node_id: first_node.node_id.clone(),
                    ip: "203.0.113.10".to_string(),
                    target_url: "https://www.gstatic.com/generate_204".to_string(),
                    ok: true,
                    latency_ms: Some(88),
                    sampled_at: 11,
                }],
                updated_at: 11,
            }])
            .await
            .expect("metadata should persist");

        let options = service
            .search_session_ip_node_options(
                project_id,
                &SearchSessionIpNodeOptionsRequest {
                    group_by: SessionIpNodeGroupBy::Subscription,
                    ..SearchSessionIpNodeOptionsRequest::default()
                },
            )
            .await
            .expect("options should load");

        assert_eq!(options.groups.len(), 2);
        assert_eq!(options.groups[0].label, "shared-name");
        assert_eq!(options.groups[1].label, "shared-name");
        assert_eq!(options.groups[0].items.len(), 1);
        assert_eq!(options.groups[1].items.len(), 1);
        let first_item = options
            .groups
            .iter()
            .flat_map(|group| group.items.iter())
            .find(|item| item.ip == "203.0.113.10")
            .expect("first IP item should exist");
        assert_eq!(first_item.nodes[0].recent_probe_samples.len(), 1);
        assert_eq!(
            first_item.nodes[0].recent_probe_samples[0].latency_ms,
            Some(88)
        );
    }

    #[tokio::test]
    async fn list_sessions_prefers_public_host_only_for_wildcard_binds() {
        let project_id = "display-explicit";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime,
            BrokerServiceOptions {
                session_public_host: Some("ops.example.test".to_string()),
                ..BrokerServiceOptions::default()
            },
        );
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let mut wildcard = make_session("s-wild", "wild-node", "203.0.113.10", 1);
        wildcard.listen = "0.0.0.0".to_string();
        wildcard.port = 20001;
        let mut explicit = make_session("s-explicit", "explicit-node", "203.0.113.20", 2);
        explicit.listen = "192.168.31.15".to_string();
        explicit.port = 20003;

        store
            .replace_subscription(
                project_id,
                &[
                    make_node("wild-node", "203.0.113.10"),
                    make_node("explicit-node", "203.0.113.20"),
                ],
            )
            .await
            .expect("seed subscription should succeed");
        store
            .insert_sessions(project_id, &[wildcard, explicit.clone()])
            .await
            .expect("seed sessions should succeed");

        let sessions = service
            .list_sessions(project_id, Some("console.example.test"))
            .await
            .expect("sessions should list");
        let wildcard_session = sessions
            .sessions
            .iter()
            .find(|item| item.session_id == "s-wild")
            .expect("wildcard session should exist");
        assert_eq!(wildcard_session.listen, "0.0.0.0:20001");
        assert_eq!(wildcard_session.display_host, "ops.example.test");
        assert_eq!(wildcard_session.display_address, "ops.example.test:20001");

        let explicit_session = sessions
            .sessions
            .iter()
            .find(|item| item.session_id == explicit.session_id)
            .expect("explicit session should exist");
        assert_eq!(explicit_session.listen, "192.168.31.15:20003");
        assert_eq!(explicit_session.bind_host, "192.168.31.15");
        assert_eq!(explicit_session.display_host, "192.168.31.15");
        assert_eq!(explicit_session.display_address, "192.168.31.15:20003");
    }

    #[tokio::test]
    async fn list_sessions_reuses_explicit_bind_host_without_public_override() {
        let project_id = "display-bind-host";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let mut session = make_session("s-bind", "bind-node", "203.0.113.30", 1);
        session.listen = "192.168.31.15".to_string();
        session.port = 20005;
        store
            .replace_subscription(project_id, &[make_node("bind-node", "203.0.113.30")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(project_id, &session)
            .await
            .expect("seed session should succeed");

        let sessions = service
            .list_sessions(project_id, Some("console.example.test"))
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions[0].listen, "192.168.31.15:20005");
        assert_eq!(sessions.sessions[0].display_host, "192.168.31.15");
        assert_eq!(sessions.sessions[0].display_address, "192.168.31.15:20005");
    }

    #[tokio::test]
    async fn update_session_node_rejects_node_outside_effective_project_pool() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");
        service
            .create_project("lab")
            .await
            .expect("lab project should be created");

        let browser_source = write_subscription_file(
            r#"
proxies:
  - name: browser-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(browser_source.clone()))
            .await
            .expect("browser subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("browser probes should refresh");
        let _ = tokio::fs::remove_file(&browser_source).await;

        let lab_source = write_subscription_file(
            r#"
proxies:
  - name: lab-node
    type: socks5
    server: 9.9.9.9
"#,
        )
        .await;
        service
            .load_subscription("lab", &SubscriptionSource::File(lab_source.clone()))
            .await
            .expect("lab subscription should load");
        let _ = tokio::fs::remove_file(&lab_source).await;

        let opened = service
            .open_session(
                "browser",
                &OpenSessionRequest {
                    selection_mode: SessionSelectionMode::Ip,
                    specified_ips: vec!["1.1.1.1".to_string()],
                    desired_port: Some(10080),
                    ..OpenSessionRequest::default()
                },
                None,
            )
            .await
            .expect("session should open");

        let lab_catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("lab".to_string()),
            })
            .await
            .expect("lab catalog should list");
        let foreign_node_id = lab_catalog.groups[0].nodes[0].node_id.clone();

        let err = service
            .update_session_node(
                "browser",
                &opened.session_id,
                &UpdateSessionNodeRequest {
                    node_id: foreign_node_id,
                    selected_ip: None,
                    candidate_node_ids: Vec::new(),
                },
                None,
            )
            .await
            .expect_err("foreign node should be rejected");
        assert!(matches!(err, BrokerError::ProxyInventoryNodeNotFound));
    }

    #[tokio::test]
    async fn update_session_node_compatibility_mode_persists_single_candidate() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: start-node
    type: socks5
    server: 1.1.1.1
  - name: next-node
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("project catalog should list");
        let start_node = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .find(|node| node.proxy_name == "start-node")
            .expect("start node should exist");
        let next_node = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .find(|node| node.proxy_name == "next-node")
            .expect("next node should exist");

        let opened = service
            .open_session_by_node(
                "browser",
                &OpenSessionByNodeRequest {
                    node_id: start_node.node_id.clone(),
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("session should open");

        let updated = service
            .update_session_node(
                "browser",
                &opened.session_id,
                &UpdateSessionNodeRequest {
                    node_id: next_node.node_id.clone(),
                    selected_ip: None,
                    candidate_node_ids: vec![
                        "foreign-node".to_string(),
                        start_node.node_id.clone(),
                    ],
                },
                None,
            )
            .await
            .expect("compatibility switch should ignore extra candidates");

        assert_eq!(updated.node_id, next_node.node_id);
        assert_eq!(updated.candidate_node_ids, vec![next_node.node_id.clone()]);
    }

    #[tokio::test]
    async fn update_session_node_rejects_candidate_node_ids_outside_effective_project_pool() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");
        service
            .create_project("lab")
            .await
            .expect("lab project should be created");

        let browser_source = write_subscription_file(
            r#"
proxies:
  - name: browser-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(browser_source.clone()))
            .await
            .expect("browser subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("browser probes should refresh");
        let _ = tokio::fs::remove_file(&browser_source).await;

        let lab_source = write_subscription_file(
            r#"
proxies:
  - name: lab-node
    type: socks5
    server: 9.9.9.9
"#,
        )
        .await;
        service
            .load_subscription("lab", &SubscriptionSource::File(lab_source.clone()))
            .await
            .expect("lab subscription should load");
        let _ = tokio::fs::remove_file(&lab_source).await;

        let browser_catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("browser catalog should list");
        let valid_node_id = browser_catalog.groups[0].nodes[0].node_id.clone();
        let lab_catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("lab".to_string()),
            })
            .await
            .expect("lab catalog should list");
        let foreign_node_id = lab_catalog.groups[0].nodes[0].node_id.clone();

        let opened = service
            .open_session_by_node(
                "browser",
                &OpenSessionByNodeRequest {
                    node_id: valid_node_id.clone(),
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("session should open");

        let err = service
            .update_session_node(
                "browser",
                &opened.session_id,
                &UpdateSessionNodeRequest {
                    node_id: String::new(),
                    selected_ip: Some("1.1.1.1".to_string()),
                    candidate_node_ids: vec![valid_node_id.clone(), foreign_node_id],
                },
                None,
            )
            .await
            .expect_err("foreign candidate should be rejected");
        assert!(matches!(err, BrokerError::InvalidRequest(_)));

        let sessions = service
            .list_sessions("browser", None)
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions[0].candidate_node_ids, vec![valid_node_id]);
    }

    #[tokio::test]
    async fn update_session_node_rolls_back_runtime_and_persistence_on_apply_failure() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(ApplyFailOnCallRuntime::new(3));
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: start-node
    type: socks5
    server: 1.1.1.1
  - name: next-node
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("project catalog should list");
        let start_node = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .find(|node| node.proxy_name == "start-node")
            .expect("start node should exist");
        let next_node = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .find(|node| node.proxy_name == "next-node")
            .expect("next node should exist");

        let opened = service
            .open_session_by_node(
                "browser",
                &OpenSessionByNodeRequest {
                    node_id: start_node.node_id.clone(),
                    desired_port: Some(10080),
                },
                None,
            )
            .await
            .expect("session should open");

        let err = service
            .update_session_node(
                "browser",
                &opened.session_id,
                &UpdateSessionNodeRequest {
                    node_id: next_node.node_id.clone(),
                    selected_ip: None,
                    candidate_node_ids: Vec::new(),
                },
                None,
            )
            .await
            .expect_err("switch should fail on runtime apply");
        assert!(matches!(
            err,
            BrokerError::Internal(_) | BrokerError::MihomoUnavailable(_)
        ));

        let sessions = service
            .list_sessions("browser", None)
            .await
            .expect("sessions should still list");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].node_id, start_node.node_id);
        assert_eq!(sessions.sessions[0].selected_ip, "1.1.1.1");

        let options = service
            .search_session_node_options(
                "browser",
                &opened.session_id,
                &SearchSessionNodeOptionsRequest {
                    sort_mode: SessionNodeSortMode::SessionRecent,
                    ..SearchSessionNodeOptionsRequest::default()
                },
            )
            .await
            .expect("node options should load");
        let next_item = options
            .items
            .iter()
            .find(|item| item.node_id == next_node.node_id)
            .expect("next node should still be present");
        assert!(next_item.session_last_used_at.is_none());
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn open_session_by_node_skips_malformed_global_inventory_in_shared_runtime_payload() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        service
            .create_project("browser")
            .await
            .expect("browser project should be created");

        let bad_import_id = "imp-bad-global".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: bad_import_id.clone(),
                    name: Some("bad-global".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&bad_import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[ProxyInventoryRecord {
                    import_id: bad_import_id.clone(),
                    node_id: "node-bad-hy".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "bad-hy".to_string(),
                    proxy_type: "hysteria".to_string(),
                    server: "5.5.5.5".to_string(),
                    resolved_ips: vec!["5.5.5.5".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "bad-hy",
                        "type": "hysteria",
                        "server": "5.5.5.5",
                        "up": "",
                        "down": "",
                    }),
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .await
            .expect("bad global inventory should seed");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: browser-node
    type: socks5
    server: 8.8.4.4
"#,
        )
        .await;
        service
            .load_subscription("browser", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("browser subscription should load");
        service
            .refresh("browser", &RefreshRequest { force: true })
            .await
            .expect("browser probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("browser".to_string()),
            })
            .await
            .expect("browser catalog should list");
        let catalog_nodes = catalog
            .groups
            .iter()
            .flat_map(|group| group.nodes.iter())
            .map(|node| node.proxy_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(catalog_nodes, vec!["browser-node"]);
        let node_id = catalog.groups[0].nodes[0].node_id.clone();

        service
            .open_session_by_node(
                "browser",
                &OpenSessionByNodeRequest {
                    node_id,
                    desired_port: Some(10080),
                },
                None,
            )
            .await
            .expect("node-pinned open should ignore malformed global inventory");

        let payloads = runtime.payloads.lock().await.clone();
        let final_payload = payloads
            .last()
            .expect("shared runtime should apply at least one payload");
        assert!(final_payload.contains("browser-node"));
        assert!(!final_payload.contains("bad-hy"));
        assert!(!final_payload.contains("up: ''"));
    }

    #[tokio::test]
    async fn inventory_views_hide_malformed_inventory_nodes() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());

        let import_id = "imp-global-filter".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("global-filter".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: 1,
                    updated_at: 1,
                },
                &[
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-good".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "good-node".to_string(),
                        proxy_type: "socks5".to_string(),
                        server: "8.8.8.8".to_string(),
                        resolved_ips: vec!["8.8.8.8".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "good-node",
                            "type": "socks5",
                            "server": "8.8.8.8",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                    ProxyInventoryRecord {
                        import_id: import_id.clone(),
                        node_id: "node-bad".to_string(),
                        source_scope: ProxyScope::global(),
                        allocation_scope: ProxyScope::global(),
                        proxy_name: "bad-node".to_string(),
                        proxy_type: "hysteria".to_string(),
                        server: "9.9.9.9".to_string(),
                        resolved_ips: vec!["9.9.9.9".to_string()],
                        raw_proxy: serde_json::json!({
                            "name": "bad-node",
                            "type": "hysteria",
                            "server": "9.9.9.9",
                            "up": "",
                            "down": "",
                        }),
                        created_at: 1,
                        updated_at: 1,
                    },
                ],
            )
            .await
            .expect("inventory should seed");

        let inventory = service
            .list_proxy_inventory(Some("all"), None)
            .await
            .expect("inventory should list");
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(inventory.items[0].proxy_name, "good-node");

        let imports = service
            .list_proxy_imports(Some("all"), None)
            .await
            .expect("imports should list");
        assert_eq!(imports.items.len(), 1);
        assert_eq!(imports.items[0].proxy_count, 1);
    }

    #[tokio::test]
    async fn concurrent_node_pinned_opens_keep_both_projects_in_shared_runtime_payload() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(CoordinatedRuntime::default());
        let service = Arc::new(BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        ));
        for project_id in ["alpha", "beta"] {
            service
                .create_project(project_id)
                .await
                .expect("project should be created");
            let source_path = write_subscription_file(&format!(
                r#"
proxies:
  - name: {project_id}-node
    type: socks5
    server: {}
"#,
                if project_id == "alpha" {
                    "1.1.1.1"
                } else {
                    "2.2.2.2"
                }
            ))
            .await;
            service
                .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
                .await
                .expect("subscription should load");
            store
                .replace_probe_records(
                    project_id,
                    &[sample_probe(
                        &format!("{project_id}-node"),
                        if project_id == "alpha" {
                            "1.1.1.1"
                        } else {
                            "2.2.2.2"
                        },
                    )],
                )
                .await
                .expect("probe records should seed");
            let _ = tokio::fs::remove_file(&source_path).await;
        }

        let alpha_node_id = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("alpha".to_string()),
            })
            .await
            .expect("alpha catalog should list")
            .groups[0]
            .nodes[0]
            .node_id
            .clone();
        let beta_node_id = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some("beta".to_string()),
            })
            .await
            .expect("beta catalog should list")
            .groups[0]
            .nodes[0]
            .node_id
            .clone();

        let alpha_service = Arc::clone(&service);
        let alpha_open = tokio::spawn(async move {
            alpha_service
                .open_session_by_node(
                    "alpha",
                    &OpenSessionByNodeRequest {
                        node_id: alpha_node_id,
                        desired_port: Some(10080),
                    },
                    None,
                )
                .await
        });

        runtime.first_apply_started.notified().await;

        let beta_service = Arc::clone(&service);
        let beta_open = tokio::spawn(async move {
            beta_service
                .open_session_by_node(
                    "beta",
                    &OpenSessionByNodeRequest {
                        node_id: beta_node_id,
                        desired_port: Some(10081),
                    },
                    None,
                )
                .await
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        runtime.allow_first_apply.notify_waiters();

        alpha_open
            .await
            .expect("alpha join should succeed")
            .expect("alpha open should succeed");
        beta_open
            .await
            .expect("beta join should succeed")
            .expect("beta open should succeed");

        let payloads = runtime.payloads.lock().await.clone();
        let final_payload = payloads
            .last()
            .expect("shared runtime should apply payloads");
        assert!(
            final_payload.contains("broker-"),
            "listener payload should be rendered"
        );
        assert!(final_payload.contains("port: 10080"));
        assert!(final_payload.contains("port: 10081"));
    }

    #[tokio::test]
    async fn reconcile_startup_preserves_node_pinned_sessions_created_from_inventory_nodes() {
        let project_id = "edge-jp";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: edge-node
    type: socks5
    server: 1.1.1.1
"#,
        )
        .await;
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        service
            .refresh(project_id, &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let catalog = service
            .list_proxy_catalog(&ProxyCatalogQuery {
                view: Some("project".to_string()),
                project_id: Some(project_id.to_string()),
            })
            .await
            .expect("project catalog should list");
        let node_id = catalog.groups[0].nodes[0].node_id.clone();

        let opened = service
            .open_session_by_node(
                project_id,
                &OpenSessionByNodeRequest {
                    node_id,
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("node-pinned session should open");

        let restarted = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        restarted
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should succeed");

        let sessions = restarted
            .list_sessions(project_id, None)
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].session_id, opened.session_id);
        assert_eq!(sessions.sessions[0].node_id, opened.node_id);
    }

    #[tokio::test]
    async fn reassign_delete_and_reimport_restore_inventory_consistently() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project("alpha")
            .await
            .expect("alpha project should be created");
        service
            .create_project("beta")
            .await
            .expect("beta project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: alpha-node
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;

        service
            .load_subscription("alpha", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("local import should succeed");

        let inventory = service
            .list_proxy_inventory(Some("all"), None)
            .await
            .expect("inventory should list");
        let node_id = inventory.items[0].node_id.clone();

        service
            .update_proxy_allocation(&node_id, &ProxyScope::global())
            .await
            .expect("reassignment should succeed");
        assert_eq!(
            store
                .list_subscription("beta")
                .await
                .expect("beta subscription should list")
                .len(),
            1
        );

        service
            .delete_proxy_inventory_node(&node_id)
            .await
            .expect("delete should succeed");
        assert!(
            store
                .list_subscription("alpha")
                .await
                .expect("alpha subscription should list")
                .is_empty()
        );
        assert!(
            store
                .list_subscription("beta")
                .await
                .expect("beta subscription should list")
                .is_empty()
        );

        service
            .load_subscription("alpha", &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("reimport should restore node");
        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(
            service
                .list_proxy_inventory(Some("all"), None)
                .await
                .expect("inventory should list")
                .items
                .len(),
            1
        );
        assert_eq!(
            store
                .list_subscription("alpha")
                .await
                .expect("alpha subscription should list")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn reconcile_startup_keeps_session_when_port_is_occupied() {
        let project_id = "p-reconcile";
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0))
            .expect("should reserve a local port for test");
        let occupied_port = occupied
            .local_addr()
            .expect("listener should expose local addr")
            .port();

        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(
                project_id,
                &SessionRecord {
                    session_id: "s1".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: occupied_port,
                    selected_ip: "1.1.1.1".to_string(),
                    proxy_name: "node-a".to_string(),
                    node_id: make_node("node-a", "1.1.1.1")
                        .node_id
                        .expect("test node should include a node id"),
                    candidate_node_ids: vec![
                        make_node("node-a", "1.1.1.1")
                            .node_id
                            .expect("test node should include a node id"),
                    ],
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
            .list_sessions(project_id)
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
    async fn reconcile_startup_preserves_legacy_sessions_when_subscription_rows_lack_node_ids() {
        let project_id = "p-legacy-node-alias";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let legacy_proxy_name = "legacy-node";
        let legacy_ip = "203.0.113.10";
        let raw_proxy = serde_json::json!({
            "name": legacy_proxy_name,
            "type": "socks5",
            "server": legacy_ip,
        });
        store
            .replace_subscription(
                project_id,
                &[ProxyNode {
                    node_id: None,
                    proxy_name: legacy_proxy_name.to_string(),
                    proxy_type: "socks5".to_string(),
                    server: legacy_ip.to_string(),
                    resolved_ips: vec![legacy_ip.to_string()],
                    raw_proxy,
                }],
            )
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(
                project_id,
                &SessionRecord {
                    session_id: "legacy-proxy-name".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: 10080,
                    selected_ip: legacy_ip.to_string(),
                    proxy_name: legacy_proxy_name.to_string(),
                    node_id: legacy_proxy_name.to_string(),
                    candidate_node_ids: vec![legacy_proxy_name.to_string()],
                    created_at: 1,
                },
            )
            .await
            .expect("proxy-name session should persist");
        store
            .insert_session(
                project_id,
                &SessionRecord {
                    session_id: "legacy-blank-node".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: 10081,
                    selected_ip: legacy_ip.to_string(),
                    proxy_name: legacy_proxy_name.to_string(),
                    node_id: String::new(),
                    candidate_node_ids: Vec::new(),
                    created_at: 2,
                },
            )
            .await
            .expect("blank-node session should persist");

        service
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should complete");

        let sessions = store
            .list_sessions(project_id)
            .await
            .expect("list sessions should succeed");
        assert_eq!(sessions.len(), 2, "legacy sessions should be preserved");
        let ids = sessions
            .iter()
            .map(|session| session.session_id.as_str())
            .collect::<HashSet<_>>();
        assert!(ids.contains("legacy-proxy-name"));
        assert!(ids.contains("legacy-blank-node"));
    }

    #[tokio::test]
    async fn reconcile_startup_keeps_persisted_sessions_when_proxy_validity_is_unknown() {
        let project_id = "p-unknown-validity";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        service
            .create_project(project_id)
            .await
            .expect("project should be created");
        store
            .insert_session(
                project_id,
                &make_session("s-keep", "orphaned-node", "203.0.113.90", 1),
            )
            .await
            .expect("seed session should succeed");

        service
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should complete");

        let sessions = store
            .list_sessions(project_id)
            .await
            .expect("list sessions should succeed");
        assert_eq!(sessions.len(), 1, "session should remain persisted");
        assert_eq!(sessions[0].session_id, "s-keep");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reconcile_startup_preserves_session_when_proxy_pair_is_not_in_current_pool() {
        let project_id = "p-pair-missing";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("current-node", "198.51.100.10")])
            .await
            .expect("seed subscription should succeed");
        store
            .insert_session(
                project_id,
                &make_session("s-keep-missing-pair", "old-node", "203.0.113.90", 1),
            )
            .await
            .expect("seed session should succeed");

        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );
        service
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should complete");

        let sessions = store
            .list_sessions(project_id)
            .await
            .expect("list sessions should succeed");
        assert_eq!(
            sessions.len(),
            1,
            "unrestorable session should remain persisted"
        );
        assert_eq!(sessions[0].session_id, "s-keep-missing-pair");
        assert_eq!(
            runtime.apply_calls.load(Ordering::SeqCst),
            0,
            "unrestorable session should not be applied to runtime"
        );
        assert_eq!(
            runtime.shutdown_calls.load(Ordering::SeqCst),
            1,
            "runtime should be stopped when no persisted session is restorable"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_sessions_survive_service_restart_and_startup_reconcile() {
        let project_id = "sqlite-restart";
        let path = temp_sqlite_store_path();
        let store = Arc::new(
            SqliteStore::open(&path)
                .await
                .expect("sqlite store should open"),
        );
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        service
            .create_project(project_id)
            .await
            .expect("project should be created");

        let source_path = write_subscription_file(
            r#"
proxies:
  - name: sqlite-node
    type: socks5
    server: 5.5.5.5
"#,
        )
        .await;
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("subscription should load");
        service
            .refresh(project_id, &RefreshRequest { force: true })
            .await
            .expect("subscription probes should refresh");
        let _ = tokio::fs::remove_file(&source_path).await;

        let opened = service
            .open_session(
                project_id,
                &OpenSessionRequest {
                    selection_mode: SessionSelectionMode::Ip,
                    specified_ips: vec!["5.5.5.5".to_string()],
                    desired_port: Some(10083),
                    ..OpenSessionRequest::default()
                },
                None,
            )
            .await
            .expect("session should open");

        drop(service);
        drop(store);

        let restarted_store = Arc::new(
            SqliteStore::open(&path)
                .await
                .expect("restarted sqlite store should open"),
        );
        let restarted = BrokerService::new(
            restarted_store.clone(),
            Arc::new(TestRuntime::default()),
            BrokerServiceOptions::default(),
        );
        restarted
            .reconcile_startup_sessions()
            .await
            .expect("startup reconcile should complete");

        let sessions = restarted
            .list_sessions(project_id, None)
            .await
            .expect("sessions should list");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].session_id, opened.session_id);
        assert_eq!(sessions.sessions[0].display_address, "127.0.0.1:10083");

        let _ = tokio::fs::remove_file(path).await;
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
            .open_batch("p-empty", &OpenBatchRequest { requests: vec![] }, None)
            .await
            .expect("empty batch should be a no-op");

        assert!(response.sessions.is_empty());
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.shutdown_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn open_batch_surfaces_invalid_request_errors() {
        let project_id = "p-batch-invalid";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(project_id, &[sample_probe("node-a", "1.1.1.1")])
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(TestRuntime::with_failures(true, true));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let err = service
            .open_batch(
                project_id,
                &OpenBatchRequest {
                    requests: vec![OpenSessionRequest {
                        desired_port: Some(0),
                        ..Default::default()
                    }],
                },
                None,
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
    async fn open_session_requires_fresh_healthy_probe_metadata() {
        let project_id = "p-open-health-gate";
        let store = Arc::new(MemoryStore::new());
        store
            .replace_subscription(project_id, &[make_node("node-a", "1.1.1.1")])
            .await
            .expect("seed subscription should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");

        let service = BrokerService::new(
            store,
            Arc::new(TestRuntime::default()),
            BrokerServiceOptions::default(),
        );

        let err = service
            .open_session(project_id, &OpenSessionRequest::default(), None)
            .await
            .expect_err("unprobed nodes should not open sessions");

        assert!(matches!(err, BrokerError::NoHealthyProxyNodes));
    }

    #[tokio::test]
    async fn open_session_retries_next_healthy_ip_after_runtime_apply_failure() {
        let project_id = "p-open-runtime-retry";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-runtime-retry".to_string(),
            name: Some("runtime-retry".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("runtime-retry"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let first_node = make_inventory_record("node-open-retry-a", "node-a", "1.1.1.1");
        let mut second_node = make_inventory_record("node-open-retry-b", "node-b", "2.2.2.2");
        second_node.import_id = import_record.import_id.clone();
        let mut first_node = first_node;
        first_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[first_node, second_node])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(
                project_id,
                &[sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                project_id,
                &[
                    sample_probe("node-a", "1.1.1.1"),
                    sample_probe("node-b", "2.2.2.2"),
                ],
            )
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_session(project_id, &OpenSessionRequest::default(), None)
            .await
            .expect("second healthy candidate should open after first apply failure");

        assert_eq!(opened.selected_ip, "2.2.2.2");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_session_retries_next_explicit_ip_after_runtime_apply_failure() {
        let project_id = "p-open-runtime-retry-explicit-ip";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-runtime-retry-explicit-ip".to_string(),
            name: Some("runtime-retry-explicit-ip".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("runtime-retry-explicit-ip"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-open-explicit-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-open-explicit-b", "node-b", "2.2.2.2");
        second_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[first_node, second_node])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(
                project_id,
                &[sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                project_id,
                &[
                    sample_probe("node-a", "1.1.1.1"),
                    sample_probe("node-b", "2.2.2.2"),
                ],
            )
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_session(
                project_id,
                &OpenSessionRequest {
                    selection_mode: SessionSelectionMode::Ip,
                    specified_ips: vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()],
                    desired_port: None,
                    ..OpenSessionRequest::default()
                },
                None,
            )
            .await
            .expect("second explicit healthy candidate should open after first apply failure");

        assert_eq!(opened.selected_ip, "2.2.2.2");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_session_retries_same_ip_sibling_node_after_runtime_apply_failure() {
        let project_id = "p-open-runtime-retry-sibling-node";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-runtime-retry-sibling-node".to_string(),
            name: Some("runtime-retry-sibling-node".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("runtime-retry-sibling-node"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-open-sibling-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-open-sibling-b", "node-b", "1.1.1.1");
        second_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[first_node, second_node])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                project_id,
                &[
                    sample_probe("node-a", "1.1.1.1"),
                    sample_probe("node-b", "1.1.1.1"),
                ],
            )
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_session(
                project_id,
                &OpenSessionRequest {
                    selection_mode: SessionSelectionMode::Ip,
                    specified_ips: vec!["1.1.1.1".to_string()],
                    desired_port: None,
                    ..OpenSessionRequest::default()
                },
                None,
            )
            .await
            .expect("same-ip sibling node should open after first node apply failure");

        assert_eq!(opened.selected_ip, "1.1.1.1");
        assert_eq!(opened.proxy_name, "node-b");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_session_accepts_fresh_proxy_node_metadata_without_legacy_probe_records() {
        let project_id = "p-open-metadata-only";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-metadata-only".to_string(),
            name: Some("metadata-only".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("metadata-only"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut node = make_inventory_record("node-open-metadata-only", "node-a", "1.1.1.1");
        node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[node.clone()])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");
        store
            .upsert_proxy_node_metadata(&[sample_proxy_node_metadata(&node.node_id, "1.1.1.1")])
            .await
            .expect("seed proxy node metadata should succeed");

        let service = BrokerService::new(
            store,
            Arc::new(TestRuntime::default()),
            BrokerServiceOptions::default(),
        );

        let opened = service
            .open_session(project_id, &OpenSessionRequest::default(), None)
            .await
            .expect("fresh node metadata should allow automatic open");

        assert_eq!(opened.selected_ip, "1.1.1.1");
        assert_eq!(opened.node_id, node.node_id);
    }

    #[tokio::test]
    async fn open_session_exhausts_all_healthy_ips_after_runtime_apply_failures() {
        let project_id = "p-open-runtime-retry-all";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-runtime-retry-all".to_string(),
            name: Some("runtime-retry-all".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("runtime-retry-all"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut nodes = vec![
            make_inventory_record("node-open-retry-all-a", "node-a", "1.1.1.1"),
            make_inventory_record("node-open-retry-all-b", "node-b", "2.2.2.2"),
            make_inventory_record("node-open-retry-all-c", "node-c", "3.3.3.3"),
            make_inventory_record("node-open-retry-all-d", "node-d", "4.4.4.4"),
        ];
        for node in &mut nodes {
            node.import_id = import_record.import_id.clone();
        }
        store
            .replace_proxy_inventory_import(&import_record, &nodes)
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(
                project_id,
                &[
                    sample_ip("1.1.1.1", None),
                    sample_ip("2.2.2.2", None),
                    sample_ip("3.3.3.3", None),
                    sample_ip("4.4.4.4", None),
                ],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                project_id,
                &[
                    sample_probe("node-a", "1.1.1.1"),
                    sample_probe("node-b", "2.2.2.2"),
                    sample_probe("node-c", "3.3.3.3"),
                    sample_probe("node-d", "4.4.4.4"),
                ],
            )
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailThroughCallRuntime::new(3));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_session(project_id, &OpenSessionRequest::default(), None)
            .await
            .expect("fourth healthy candidate should open after three apply failures");

        assert_eq!(opened.selected_ip, "4.4.4.4");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn open_session_by_node_uses_fresh_proxy_node_metadata_health() {
        let project_id = "p-node-open-metadata";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-node-open-metadata".to_string(),
            name: Some("node-open-metadata".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("node-open-metadata"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut node = make_inventory_record("node-open-metadata-a", "node-a", "1.1.1.1");
        node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[node.clone()])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");
        store
            .upsert_proxy_node_metadata(&[sample_proxy_node_metadata(&node.node_id, "1.1.1.1")])
            .await
            .expect("seed proxy node metadata should succeed");

        let service = BrokerService::new(
            store,
            Arc::new(TestRuntime::default()),
            BrokerServiceOptions::default(),
        );

        let opened = service
            .open_session_by_node(
                project_id,
                &OpenSessionByNodeRequest {
                    node_id: node.node_id.clone(),
                    desired_port: None,
                },
                None,
            )
            .await
            .expect("fresh proxy node metadata should allow node-pinned open");

        assert_eq!(opened.selected_ip, "1.1.1.1");
    }

    #[tokio::test]
    async fn open_batch_by_node_retries_next_healthy_ip_after_runtime_apply_failure() {
        let project_id = "p-batch-node-runtime-retry";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-batch-node-runtime-retry".to_string(),
            name: Some("batch-node-runtime-retry".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("batch-node-runtime-retry"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut node = make_inventory_record("node-batch-retry-a", "node-a", "1.1.1.1");
        node.import_id = import_record.import_id.clone();
        node.resolved_ips = vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()];
        store
            .replace_proxy_inventory_import(&import_record, &[node.clone()])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(
                project_id,
                &[sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .upsert_proxy_node_metadata(&[
                sample_proxy_node_metadata(&node.node_id, "1.1.1.1"),
                sample_proxy_node_metadata(&node.node_id, "2.2.2.2"),
            ])
            .await
            .expect("seed proxy node metadata should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_batch_by_node(
                project_id,
                &OpenBatchByNodeRequest {
                    node_ids: vec![node.node_id.clone()],
                    requests: Vec::new(),
                },
                None,
            )
            .await
            .expect("batch node open should retry the next healthy ip");

        assert_eq!(opened.sessions[0].selected_ip, "2.2.2.2");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_batch_retries_alternate_candidate_after_runtime_apply_failure() {
        let project_id = "p-batch-runtime-retry";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-batch-runtime-retry".to_string(),
            name: Some("batch-runtime-retry".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("batch-runtime-retry"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-batch-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-batch-b", "node-b", "2.2.2.2");
        second_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(
                &import_record,
                &[first_node.clone(), second_node.clone()],
            )
            .await
            .expect("seed inventory should succeed");
        store
            .replace_subscription(
                project_id,
                &[
                    ProxyNode {
                        node_id: Some(first_node.node_id.clone()),
                        proxy_name: first_node.proxy_name.clone(),
                        proxy_type: first_node.proxy_type.clone(),
                        server: first_node.server.clone(),
                        resolved_ips: first_node.resolved_ips.clone(),
                        raw_proxy: first_node.raw_proxy.clone(),
                    },
                    ProxyNode {
                        node_id: Some(second_node.node_id.clone()),
                        proxy_name: second_node.proxy_name.clone(),
                        proxy_type: second_node.proxy_type.clone(),
                        server: second_node.server.clone(),
                        resolved_ips: second_node.resolved_ips.clone(),
                        raw_proxy: second_node.raw_proxy.clone(),
                    },
                ],
            )
            .await
            .expect("seed subscription should succeed");
        store
            .replace_ip_records(
                project_id,
                &[sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                project_id,
                &[
                    sample_probe("node-a", "1.1.1.1"),
                    sample_probe("node-b", "2.2.2.2"),
                ],
            )
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_batch(
                project_id,
                &OpenBatchRequest {
                    requests: vec![OpenSessionRequest {
                        selection_mode: SessionSelectionMode::Ip,
                        specified_ips: vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()],
                        desired_port: None,
                        ..OpenSessionRequest::default()
                    }],
                },
                None,
            )
            .await
            .expect("batch open should retry an alternate healthy ip");

        assert_eq!(opened.sessions[0].selected_ip, "2.2.2.2");
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_batch_by_ip_retries_same_ip_sibling_node_after_runtime_apply_failure() {
        let project_id = "p-batch-ip-runtime-retry-sibling-node";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-batch-ip-runtime-retry-sibling-node".to_string(),
            name: Some("batch-ip-runtime-retry-sibling-node".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual(
                "batch-ip-runtime-retry-sibling-node",
            ),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-batch-ip-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-batch-ip-b", "node-b", "1.1.1.1");
        second_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(
                &import_record,
                &[first_node.clone(), second_node.clone()],
            )
            .await
            .expect("seed inventory should succeed");
        store
            .upsert_proxy_node_metadata(&[
                sample_proxy_node_metadata(&first_node.node_id, "1.1.1.1"),
                sample_proxy_node_metadata(&second_node.node_id, "1.1.1.1"),
            ])
            .await
            .expect("seed proxy node metadata should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_batch_by_ip(
                project_id,
                &OpenBatchByIpRequest {
                    requests: vec![OpenSessionByIpRequest {
                        selected_ip: "1.1.1.1".to_string(),
                        candidate_node_ids: vec![
                            first_node.node_id.clone(),
                            second_node.node_id.clone(),
                        ],
                        desired_port: None,
                    }],
                },
                None,
            )
            .await
            .expect("same-ip sibling candidate node should open after first node apply failure");

        assert_eq!(opened.sessions[0].selected_ip, "1.1.1.1");
        assert_eq!(opened.sessions[0].node_id, second_node.node_id);
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_batch_by_ip_keeps_single_candidate_entries_while_retrying_alternates() {
        let project_id = "p-batch-ip-runtime-retry-mixed-candidates";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-batch-ip-runtime-retry-mixed-candidates".to_string(),
            name: Some("batch-ip-runtime-retry-mixed-candidates".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual(
                "batch-ip-runtime-retry-mixed-candidates",
            ),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-batch-ip-mixed-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-batch-ip-mixed-b", "node-b", "1.1.1.1");
        second_node.import_id = import_record.import_id.clone();
        let mut stable_node = make_inventory_record("node-batch-ip-mixed-c", "node-c", "2.2.2.2");
        stable_node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(
                &import_record,
                &[first_node.clone(), second_node.clone(), stable_node.clone()],
            )
            .await
            .expect("seed inventory should succeed");
        store
            .upsert_proxy_node_metadata(&[
                sample_proxy_node_metadata(&first_node.node_id, "1.1.1.1"),
                sample_proxy_node_metadata(&second_node.node_id, "1.1.1.1"),
                sample_proxy_node_metadata(&stable_node.node_id, "2.2.2.2"),
            ])
            .await
            .expect("seed proxy node metadata should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_batch_by_ip(
                project_id,
                &OpenBatchByIpRequest {
                    requests: vec![
                        OpenSessionByIpRequest {
                            selected_ip: "1.1.1.1".to_string(),
                            candidate_node_ids: vec![
                                first_node.node_id.clone(),
                                second_node.node_id.clone(),
                            ],
                            desired_port: None,
                        },
                        OpenSessionByIpRequest {
                            selected_ip: "2.2.2.2".to_string(),
                            candidate_node_ids: vec![stable_node.node_id.clone()],
                            desired_port: None,
                        },
                    ],
                },
                None,
            )
            .await
            .expect("alternate first request should retry with unchanged single-candidate peer");

        assert_eq!(opened.sessions[0].node_id, second_node.node_id);
        assert_eq!(opened.sessions[1].node_id, stable_node.node_id);
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_batch_by_node_preserves_unaffected_node_candidate_when_retrying() {
        let project_id = "p-batch-node-combo-retry";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-batch-node-combo-retry".to_string(),
            name: Some("batch-node-combo-retry".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("batch-node-combo-retry"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut first_node = make_inventory_record("node-combo-a", "node-a", "1.1.1.1");
        first_node.import_id = import_record.import_id.clone();
        let mut second_node = make_inventory_record("node-combo-b", "node-b", "2.2.2.2");
        second_node.import_id = import_record.import_id.clone();
        second_node.resolved_ips = vec!["2.2.2.2".to_string(), "3.3.3.3".to_string()];
        store
            .replace_proxy_inventory_import(
                &import_record,
                &[first_node.clone(), second_node.clone()],
            )
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(
                project_id,
                &[
                    sample_ip("1.1.1.1", None),
                    sample_ip("2.2.2.2", None),
                    sample_ip("3.3.3.3", None),
                ],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .upsert_proxy_node_metadata(&[
                sample_proxy_node_metadata(&first_node.node_id, "1.1.1.1"),
                sample_proxy_node_metadata(&second_node.node_id, "2.2.2.2"),
                sample_proxy_node_metadata(&second_node.node_id, "3.3.3.3"),
            ])
            .await
            .expect("seed proxy node metadata should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(store, runtime.clone(), BrokerServiceOptions::default());

        let opened = service
            .open_batch_by_node(
                project_id,
                &OpenBatchByNodeRequest {
                    node_ids: vec![first_node.node_id.clone(), second_node.node_id.clone()],
                    requests: Vec::new(),
                },
                None,
            )
            .await
            .expect("batch node open should keep the first node candidate and advance the second");

        let selected = opened
            .sessions
            .iter()
            .map(|session| session.selected_ip.as_str())
            .collect::<Vec<_>>();
        assert_eq!(selected, vec!["1.1.1.1", "3.3.3.3"]);
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn open_session_preserves_runtime_apply_error_when_retries_exhaust_healthy_ips() {
        let project_id = "p-open-runtime-exhausted";
        let store = Arc::new(MemoryStore::new());
        store
            .create_project(project_id, 1)
            .await
            .expect("project should be created");
        let import_record = ProxyImportRecord {
            import_id: "imp-open-runtime-exhausted".to_string(),
            name: Some("runtime-exhausted".to_string()),
            import_kind: ProxyImportKind::Subscription,
            source_scope: ProxyScope::global(),
            source_identity: ProxyImportSourceIdentity::manual("runtime-exhausted"),
            allocation_scope: ProxyScope::global(),
            subscription_metadata: None,
            created_at: 1,
            updated_at: 1,
        };
        let mut node = make_inventory_record("node-open-exhausted-a", "node-a", "1.1.1.1");
        node.import_id = import_record.import_id.clone();
        store
            .replace_proxy_inventory_import(&import_record, &[node])
            .await
            .expect("seed inventory should succeed");
        store
            .replace_ip_records(project_id, &[sample_ip("1.1.1.1", None)])
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(project_id, &[sample_probe("node-a", "1.1.1.1")])
            .await
            .expect("seed probe records should succeed");

        let runtime = Arc::new(ApplyFailOnCallRuntime::new(1));
        let service = BrokerService::new(
            store.clone(),
            runtime.clone(),
            BrokerServiceOptions::default(),
        );

        let err = service
            .open_session(project_id, &OpenSessionRequest::default(), None)
            .await
            .expect_err("runtime apply failure should not be masked as no healthy nodes");

        assert!(
            matches!(err, BrokerError::ProxyRuntimeApplyFailed(_)),
            "unexpected error: {err:?}"
        );
        assert_eq!(runtime.apply_calls.load(Ordering::SeqCst), 1);
        assert!(
            store
                .list_sessions(project_id)
                .await
                .expect("sessions should list")
                .is_empty(),
            "failed runtime apply must not persist a session"
        );
    }

    #[tokio::test]
    async fn create_project_trims_and_lists_empty_project() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let created = service
            .create_project("  fresh-lab  ")
            .await
            .expect("create should succeed");
        assert_eq!(created.project_id, "fresh-lab");

        let projects = service
            .list_projects()
            .await
            .expect("list should succeed")
            .projects;
        assert_eq!(projects, vec!["fresh-lab"]);
    }

    #[tokio::test]
    async fn create_project_rejects_duplicates() {
        let store = Arc::new(MemoryStore::new());
        store
            .create_project("default", 1)
            .await
            .expect("seed create should succeed");
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let err = service
            .create_project("default")
            .await
            .expect_err("duplicate create should fail");
        assert!(matches!(err, BrokerError::ProjectExists));
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
            updated_at: now_epoch_sec(),
        }
    }

    fn sample_probe_with_latency(
        proxy_name: &str,
        ip: &str,
        latency_ms: u64,
        updated_at: i64,
    ) -> ProbeRecord {
        ProbeRecord {
            proxy_name: proxy_name.to_string(),
            ip: ip.to_string(),
            target_url: "https://www.gstatic.com/generate_204".to_string(),
            ok: true,
            latency_ms: Some(latency_ms),
            updated_at,
        }
    }

    fn sample_proxy_node_metadata(node_id: &str, ip: &str) -> ProxyNodeMetadataRecord {
        ProxyNodeMetadataRecord {
            node_id: node_id.to_string(),
            ip: ip.to_string(),
            country_code: None,
            country_name: None,
            region_name: None,
            city: None,
            geo_source: None,
            probe_updated_at: Some(now_epoch_sec()),
            geo_updated_at: None,
            last_probe_ok: Some(true),
            last_latency_ms: Some(100),
            median_latency_ms: Some(100),
            last_probe_samples: vec![Some(100)],
            recent_probe_samples: Vec::new(),
            updated_at: now_epoch_sec(),
        }
    }

    #[test]
    fn candidate_ips_for_open_ranks_by_fresh_probe_latency() {
        let now = now_epoch_sec();
        let min_updated_at = now - 30;
        let ips = vec![sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)];
        let probes = vec![
            sample_probe_with_latency("node-a", "1.1.1.1", 1, min_updated_at - 1),
            sample_probe_with_latency("node-a", "1.1.1.1", 200, now),
            sample_probe_with_latency("node-b", "2.2.2.2", 100, now),
        ];

        let ranked = candidate_ips_for_open(
            &OpenSessionRequest::default(),
            &ips,
            &probes,
            &HashMap::new(),
            Some(min_updated_at),
        )
        .expect("fresh healthy candidates should rank");

        assert_eq!(ranked, vec!["2.2.2.2", "1.1.1.1"]);
    }

    #[test]
    fn choose_node_for_ip_ranks_by_fresh_probe_latency() {
        let now = now_epoch_sec();
        let min_updated_at = now - 30;
        let ip = "1.1.1.1";
        let nodes = vec![sample_node("node-a", ip), sample_node("node-b", ip)];
        let probes = vec![
            sample_probe_with_latency("node-a", ip, 1, min_updated_at - 1),
            sample_probe_with_latency("node-a", ip, 200, now),
            sample_probe_with_latency("node-b", ip, 100, now),
        ];

        let selected = choose_node_for_ip(
            ip,
            &nodes,
            &probes,
            &HashMap::new(),
            Some(min_updated_at),
            &HashSet::new(),
        )
        .expect("fresh healthy node should rank");

        assert_eq!(selected.proxy_name, "node-b");
    }

    #[test]
    fn choose_best_inventory_node_for_ip_ranks_by_fresh_probe_latency() {
        let now = now_epoch_sec();
        let min_updated_at = now - 30;
        let ip = "1.1.1.1";
        let first = make_inventory_record("node-a", "node-a", ip);
        let second = make_inventory_record("node-b", "node-b", ip);
        let mut first_metadata = sample_proxy_node_metadata(&first.node_id, ip);
        first_metadata.median_latency_ms = Some(1);
        first_metadata.recent_probe_samples = vec![
            ProxyNodeProbeSampleRecord {
                node_id: first.node_id.clone(),
                ip: ip.to_string(),
                target_url: "https://www.gstatic.com/generate_204".to_string(),
                ok: true,
                latency_ms: Some(1),
                sampled_at: min_updated_at - 1,
            },
            ProxyNodeProbeSampleRecord {
                node_id: first.node_id.clone(),
                ip: ip.to_string(),
                target_url: "https://www.gstatic.com/generate_204".to_string(),
                ok: true,
                latency_ms: Some(200),
                sampled_at: now,
            },
        ];
        let mut second_metadata = sample_proxy_node_metadata(&second.node_id, ip);
        second_metadata.median_latency_ms = Some(150);
        second_metadata.recent_probe_samples = vec![ProxyNodeProbeSampleRecord {
            node_id: second.node_id.clone(),
            ip: ip.to_string(),
            target_url: "https://www.gstatic.com/generate_204".to_string(),
            ok: true,
            latency_ms: Some(100),
            sampled_at: now,
        }];
        let metadata = HashMap::from([
            ((first.node_id.clone(), ip.to_string()), first_metadata),
            ((second.node_id.clone(), ip.to_string()), second_metadata),
        ]);

        let (selected, _) = choose_best_inventory_node_for_ip(
            ip,
            &[first.node_id.clone(), second.node_id.clone()],
            &[first, second],
            &metadata,
            Some(min_updated_at),
        )
        .expect("fresh healthy inventory node should rank");

        assert_eq!(selected.node_id, "node-b");
    }

    fn sample_node(proxy_name: &str, ip: &str) -> ProxyNode {
        let raw_proxy = serde_json::json!({
            "name": proxy_name,
            "type": "socks5",
            "server": ip
        });
        ProxyNode {
            node_id: Some(ids::stable_proxy_inventory_node_id_for_proxy(
                "sample-import",
                proxy_name,
                "socks5",
                ip,
                &raw_proxy,
            )),
            proxy_name: proxy_name.to_string(),
            proxy_type: "socks5".to_string(),
            server: ip.to_string(),
            resolved_ips: vec![ip.to_string()],
            raw_proxy,
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
    fn batch_stage_failure_returns_underlying_error() {
        let requests = vec![OpenSessionRequest {
            selection_mode: SessionSelectionMode::Ip,
            specified_ips: vec!["9.9.9.9".to_string()],
            desired_port: None,
            ..Default::default()
        }];
        let nodes = vec![sample_node("proxy-a", "1.1.1.1")];
        let candidate_ips = vec![vec!["9.9.9.9".to_string()]];
        let mut candidate_indexes = vec![0usize];
        let metadata_by_pair = HashMap::new();
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &[],
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at: None,
        };
        let ports = SessionPortConfig {
            listen_ip: Ipv4Addr::LOCALHOST.into(),
            port_range: None,
        };
        let err = stage_batch_sessions(
            &requests,
            &candidate_ips,
            &mut candidate_indexes,
            &[],
            &candidates,
            &ports,
        )
        .expect_err("non-existent specified ip should fail");
        assert!(matches!(err, BrokerError::IpNotFound));
    }

    #[test]
    fn stage_batch_sessions_skips_ip_with_only_stale_node_probe_mapping() {
        let bad_ip = "1.1.1.1";
        let good_ip = "2.2.2.2";
        let requests = vec![OpenSessionRequest {
            selection_mode: SessionSelectionMode::Any,
            desired_port: None,
            ..Default::default()
        }];
        let candidate_ips = vec![vec![bad_ip.to_string(), good_ip.to_string()]];
        let mut candidate_indexes = vec![0usize];
        let nodes = vec![
            sample_node("current-bad-node", bad_ip),
            sample_node("healthy-node", good_ip),
        ];
        let min_updated_at = now_epoch_sec().saturating_sub(60);
        let probes = vec![
            sample_probe_with_latency("old-bad-node", bad_ip, 1, min_updated_at + 1),
            sample_probe_with_latency("healthy-node", good_ip, 100, min_updated_at + 1),
        ];
        let metadata_by_pair = HashMap::new();
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &probes,
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at: Some(min_updated_at),
        };
        let ports = SessionPortConfig {
            listen_ip: Ipv4Addr::LOCALHOST.into(),
            port_range: Some((20000, 20010)),
        };

        let sessions = stage_batch_sessions(
            &requests,
            &candidate_ips,
            &mut candidate_indexes,
            &[],
            &candidates,
            &ports,
        )
        .expect("healthy later batch candidate should be selected");

        assert_eq!(sessions[0].selected_ip, good_ip);
        assert_eq!(sessions[0].proxy_name, "healthy-node");
        assert_eq!(candidate_indexes[0], 1);
    }

    #[test]
    fn choose_ip_honors_sort_mode_for_any_selection() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Any,
            sort_mode: SortMode::Mru,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", Some(10)),
        ];
        let chosen = choose_ip_for_open(&request, &ips, &[], &HashMap::new(), None)
            .expect("should pick with mru");
        assert_eq!(chosen, "1.1.1.1");
    }

    #[test]
    fn choose_ip_for_any_selection_prefers_healthy_candidates() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Any,
            sort_mode: SortMode::Mru,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", Some(10)),
        ];
        let probes = vec![sample_probe("proxy-b", "2.2.2.2")];

        let chosen = choose_ip_for_open(&request, &ips, &probes, &HashMap::new(), None)
            .expect("healthy candidate should win");
        assert_eq!(chosen, "2.2.2.2");
    }

    #[test]
    fn choose_ip_for_any_selection_prefers_lower_latency_before_recency() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Any,
            sort_mode: SortMode::Mru,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![
            sample_ip("1.1.1.1", Some(100)),
            sample_ip("2.2.2.2", Some(10)),
        ];
        let probes = vec![
            ProbeRecord {
                latency_ms: Some(250),
                ..sample_probe("proxy-a", "1.1.1.1")
            },
            ProbeRecord {
                latency_ms: Some(40),
                ..sample_probe("proxy-b", "2.2.2.2")
            },
        ];

        let chosen = choose_ip_for_open(&request, &ips, &probes, &HashMap::new(), None)
            .expect("lower latency candidate should win before recency");
        assert_eq!(chosen, "2.2.2.2");
    }

    #[test]
    fn choose_ip_for_open_rejects_stale_or_failed_probe_candidates() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Ip,
            specified_ips: vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()],
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![sample_ip("1.1.1.1", None), sample_ip("2.2.2.2", None)];
        let min_updated_at = now_epoch_sec().saturating_sub(60);
        let probes = vec![
            ProbeRecord {
                updated_at: min_updated_at - 1,
                ..sample_probe("proxy-a", "1.1.1.1")
            },
            ProbeRecord {
                ok: false,
                latency_ms: None,
                ..sample_probe("proxy-b", "2.2.2.2")
            },
        ];

        let err = choose_ip_for_open(
            &request,
            &ips,
            &probes,
            &HashMap::new(),
            Some(min_updated_at),
        )
        .expect_err("stale and failed probes should be rejected");
        assert!(matches!(err, BrokerError::NoHealthyProxyNodes));
    }

    #[test]
    fn choose_node_for_ip_requires_fresh_healthy_probe_for_same_proxy_node() {
        let ip = "1.1.1.1";
        let nodes = vec![
            sample_node("stale-node", ip),
            sample_node("healthy-node", ip),
        ];
        let min_updated_at = now_epoch_sec().saturating_sub(60);
        let probes = vec![
            ProbeRecord {
                updated_at: min_updated_at - 1,
                ..sample_probe("stale-node", ip)
            },
            sample_probe("healthy-node", ip),
        ];

        let node = choose_node_for_ip(
            ip,
            &nodes,
            &probes,
            &HashMap::new(),
            Some(min_updated_at),
            &HashSet::new(),
        )
        .expect("fresh healthy node should be selected");
        assert_eq!(node.proxy_name, "healthy-node");
    }

    #[test]
    fn prepare_session_skips_ip_with_only_stale_node_probe_mapping() {
        let bad_ip = "1.1.1.1";
        let good_ip = "2.2.2.2";
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Any,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![sample_ip(bad_ip, None), sample_ip(good_ip, None)];
        let nodes = vec![
            sample_node("current-bad-node", bad_ip),
            sample_node("healthy-node", good_ip),
        ];
        let min_updated_at = now_epoch_sec().saturating_sub(60);
        let probes = vec![
            sample_probe_with_latency("old-bad-node", bad_ip, 1, min_updated_at + 1),
            sample_probe_with_latency("healthy-node", good_ip, 100, min_updated_at + 1),
        ];
        let metadata_by_pair = HashMap::new();
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &probes,
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at: Some(min_updated_at),
        };
        let ports = SessionPortConfig {
            listen_ip: "127.0.0.1".parse().expect("listen ip should parse"),
            port_range: Some((20000, 20010)),
        };

        let session = prepare_session(&request, &ips, &[], &candidates, &ports, &HashSet::new())
            .expect("healthy later candidate should be selected");

        assert_eq!(session.selected_ip, good_ip);
        assert_eq!(session.proxy_name, "healthy-node");
    }

    #[test]
    fn search_session_options_keeps_duplicate_city_names_across_countries() {
        let mut us_paris = sample_ip("1.1.1.1", None);
        us_paris.city = Some("Paris".to_string());
        us_paris.country_code = Some("US".to_string());
        us_paris.country_name = Some("United States".to_string());

        let mut fr_paris = sample_ip("2.2.2.2", None);
        fr_paris.city = Some("Paris".to_string());
        fr_paris.country_code = Some("FR".to_string());
        fr_paris.country_name = Some("France".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::City,
            query: Some("par".to_string()),
            country_codes: vec![],
            cities: vec![],
            limit: None,
        };

        let items = search_session_options(&[us_paris, fr_paris], &request)
            .expect("city options should be returned");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "Paris");
        assert_eq!(items[0].value, "FR::Paris");
        assert_eq!(items[0].meta.as_deref(), Some("France (FR)"));
        assert_eq!(items[1].value, "US::Paris");
        assert_eq!(items[1].meta.as_deref(), Some("United States (US)"));
    }

    #[test]
    fn search_session_options_ip_accepts_encoded_city_filters() {
        let mut fr_paris = sample_ip("1.1.1.1", None);
        fr_paris.city = Some("Paris".to_string());
        fr_paris.country_code = Some("FR".to_string());
        fr_paris.country_name = Some("France".to_string());

        let mut us_paris = sample_ip("2.2.2.2", None);
        us_paris.city = Some("Paris".to_string());
        us_paris.country_code = Some("US".to_string());
        us_paris.country_name = Some("United States".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::Ip,
            query: None,
            country_codes: vec![],
            cities: vec!["FR::Paris".to_string()],
            limit: None,
        };

        let items = search_session_options(&[fr_paris, us_paris], &request)
            .expect("ip options should respect encoded city filters");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "1.1.1.1");
    }

    #[test]
    fn search_session_options_drops_invalid_country_codes() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.city = Some("Tokyo".to_string());
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::Country,
            query: None,
            country_codes: vec![],
            cities: vec![],
            limit: None,
        };

        let items = search_session_options(&[invalid_country], &request)
            .expect("country options should be returned");
        assert!(items.is_empty());
    }

    #[test]
    fn filter_ip_records_sanitizes_invalid_country_codes() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());

        let items = filter_ip_records(vec![invalid_country], &[], &ExtractIpRequest::default())
            .expect("extract ip items should be returned");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].country_code, None);
        assert_eq!(items[0].country_name.as_deref(), Some("Japan"));
    }

    #[test]
    fn build_open_selector_request_preserves_invalid_country_filters() {
        let selector = build_open_selector_request(&OpenSessionRequest {
            selection_mode: SessionSelectionMode::Geo,
            country_codes: vec!["global".to_string()],
            ..Default::default()
        })
        .expect("legacy non-ISO filters should remain valid");

        assert_eq!(selector.country_codes, vec!["GLOBAL"]);
    }

    #[test]
    fn filter_ip_records_preserves_invalid_country_filters() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());
        let mut valid_country = sample_ip("2.2.2.2", None);
        valid_country.country_code = Some("FR".to_string());
        valid_country.country_name = Some("France".to_string());

        let items = filter_ip_records(
            vec![invalid_country, valid_country],
            &[],
            &ExtractIpRequest {
                country_codes: vec!["global".to_string()],
                ..Default::default()
            },
        )
        .expect("legacy non-ISO filters should still match");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].ip, "1.1.1.1");
        assert_eq!(items[0].country_code, None);
    }

    #[test]
    fn search_session_options_city_values_preserve_invalid_country_prefixes() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.city = Some("Tokyo".to_string());
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::City,
            query: None,
            country_codes: vec![],
            cities: vec![],
            limit: None,
        };

        let items = search_session_options(&[invalid_country], &request)
            .expect("city options should preserve opaque legacy tokens");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "GLOBAL::Tokyo");
        assert_eq!(items[0].label, "Tokyo");
    }

    #[test]
    fn search_session_options_city_filters_preserve_invalid_country_prefixes() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.city = Some("Paris".to_string());
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());
        let mut valid_country = sample_ip("2.2.2.2", None);
        valid_country.city = Some("Paris".to_string());
        valid_country.country_code = Some("FR".to_string());
        valid_country.country_name = Some("France".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::Ip,
            query: None,
            country_codes: vec![],
            cities: vec!["global::Paris".to_string()],
            limit: None,
        };

        let items = search_session_options(&[invalid_country, valid_country], &request)
            .expect("legacy malformed city filters should still match");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "1.1.1.1");
    }

    #[test]
    fn filter_ip_records_city_filters_preserve_invalid_country_prefixes() {
        let mut invalid_country = sample_ip("1.1.1.1", None);
        invalid_country.city = Some("Paris".to_string());
        invalid_country.country_code = Some("global".to_string());
        invalid_country.country_name = Some("Japan".to_string());
        let mut valid_country = sample_ip("2.2.2.2", None);
        valid_country.city = Some("Paris".to_string());
        valid_country.country_code = Some("FR".to_string());
        valid_country.country_name = Some("France".to_string());

        let items = filter_ip_records(
            vec![invalid_country, valid_country],
            &[],
            &ExtractIpRequest {
                cities: vec!["global::Paris".to_string()],
                ..Default::default()
            },
        )
        .expect("legacy malformed city filters should still match");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].ip, "1.1.1.1");
        assert_eq!(items[0].country_code, None);
    }

    #[test]
    fn search_session_options_keeps_maxmind_special_country_codes() {
        let mut anonymous_proxy = sample_ip("1.1.1.1", None);
        anonymous_proxy.country_code = Some("a1".to_string());
        anonymous_proxy.country_name = Some("Anonymous Proxy".to_string());

        let request = SearchSessionOptionsRequest {
            kind: SessionOptionKind::Country,
            query: None,
            country_codes: vec![],
            cities: vec![],
            limit: None,
        };

        let items = search_session_options(&[anonymous_proxy], &request)
            .expect("country options should load");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "A1");
        assert_eq!(items[0].label, "Anonymous Proxy (A1)");
    }

    #[test]
    fn filter_ip_records_keeps_maxmind_special_country_codes() {
        let mut anonymous_proxy = sample_ip("1.1.1.1", None);
        anonymous_proxy.country_code = Some("A1".to_string());
        anonymous_proxy.country_name = Some("Anonymous Proxy".to_string());

        let items = filter_ip_records(vec![anonymous_proxy], &[], &ExtractIpRequest::default())
            .expect("extract ip items should be returned");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].country_code.as_deref(), Some("A1"));
        assert_eq!(items[0].country_name.as_deref(), Some("Anonymous Proxy"));
    }

    #[test]
    fn resolve_online_geo_country_code_preserves_mmdb_code_on_malformed_online_value() {
        let country_code =
            resolve_online_geo_country_code(Some("JP".to_string()), true, Some("global"), true);

        assert_eq!(country_code.as_deref(), Some("JP"));
    }

    #[test]
    fn resolve_online_geo_country_code_clears_stale_code_without_lookup_source() {
        let country_code =
            resolve_online_geo_country_code(Some("US".to_string()), false, Some("global"), true);

        assert_eq!(country_code, None);
    }

    #[test]
    fn choose_ip_for_geo_selection_preserves_city_country_pairings() {
        let mut fr_paris = sample_ip("1.1.1.1", Some(100));
        fr_paris.city = Some("Paris".to_string());
        fr_paris.country_code = Some("FR".to_string());
        fr_paris.country_name = Some("France".to_string());

        let mut us_paris = sample_ip("2.2.2.2", Some(10));
        us_paris.city = Some("Paris".to_string());
        us_paris.country_code = Some("US".to_string());
        us_paris.country_name = Some("United States".to_string());

        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Geo,
            country_codes: vec!["FR".to_string(), "US".to_string()],
            cities: vec!["FR::Paris".to_string()],
            desired_port: None,
            ..Default::default()
        };

        let chosen =
            choose_ip_for_open(&request, &[fr_paris, us_paris], &[], &HashMap::new(), None)
                .expect("geo selection should preserve city-country pairings");
        assert_eq!(chosen, "1.1.1.1");
    }

    #[test]
    fn choose_ip_rejects_ip_mode_without_specified_ips() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Ip,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![sample_ip("1.1.1.1", Some(100))];
        let err = choose_ip_for_open(&request, &ips, &[], &HashMap::new(), None)
            .expect_err("ip mode without specified_ips should be rejected");
        assert!(matches!(err, BrokerError::InvalidRequest(_)));
    }

    #[test]
    fn desired_port_zero_is_invalid() {
        let err = allocate_port(&[], Some(0), Ipv4Addr::LOCALHOST.into(), None)
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

        let err = allocate_port(&[], Some(occupied_port), Ipv4Addr::UNSPECIFIED.into(), None)
            .expect_err("occupied wildcard port should be rejected");
        assert!(matches!(err, BrokerError::PortInUse));
    }

    #[test]
    fn allocate_port_auto_assignment_stays_within_configured_range() {
        let port = allocate_port(&[], None, Ipv4Addr::LOCALHOST.into(), Some((20000, 20002)))
            .expect("range-constrained auto assignment should succeed");
        assert!((20000..=20002).contains(&port));
    }

    #[test]
    fn allocate_port_rejects_desired_port_outside_configured_range() {
        let err = allocate_port(
            &[],
            Some(25000),
            Ipv4Addr::LOCALHOST.into(),
            Some((20000, 20999)),
        )
        .expect_err("out-of-range desired port should be rejected");
        assert!(matches!(err, BrokerError::InvalidRequest(_)));
    }

    #[test]
    fn prepare_session_uses_configured_listen_ip() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Ip,
            specified_ips: vec!["1.1.1.1".to_string()],
            desired_port: None,
            ..Default::default()
        };
        let nodes = vec![sample_node("proxy-a", "1.1.1.1")];
        let ips = vec![sample_ip("1.1.1.1", None)];
        let metadata_by_pair = HashMap::new();
        let candidates = SessionCandidateContext {
            nodes: &nodes,
            probes: &[],
            metadata_by_pair: &metadata_by_pair,
            min_probe_updated_at: None,
        };
        let ports = SessionPortConfig {
            listen_ip: Ipv4Addr::UNSPECIFIED.into(),
            port_range: None,
        };

        let session = prepare_session(&request, &ips, &[], &candidates, &ports, &HashSet::new())
            .expect("session should be prepared");

        assert_eq!(session.listen, "0.0.0.0");
    }

    #[tokio::test]
    async fn load_subscription_registers_sync_config_and_queues_post_load_task() {
        let project_id = "p-tasks";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;

        let response = service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 1);

        let config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should be persisted");
        assert!(matches!(config.source, SubscriptionSource::File(path) if path == source_path));
        assert!(config.enabled);

        let tasks = service
            .list_tasks(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert_eq!(tasks.runs.len(), 1);
        let run = &tasks.runs[0];
        assert_eq!(run.kind, TaskRunKind::MetadataRefreshIncremental);
        assert_eq!(run.trigger, TaskRunTrigger::PostLoad);
        assert_eq!(run.status, TaskRunStatus::Queued);

        let detail = service
            .get_task_run_detail(&run.run_id)
            .await
            .expect("task detail should succeed");
        assert_eq!(detail.events.len(), 1);
        assert_eq!(detail.events[0].stage, TaskRunStage::Queued);
    }

    #[tokio::test]
    async fn load_subscription_skips_post_load_task_when_full_refresh_is_pending() {
        let project_id = "p-tasks-with-full-refresh";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 6.6.6.6
"#,
        )
        .await;

        service
            .enqueue_task_run(
                project_id,
                TaskRunKind::MetadataRefreshFull,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("full refresh queue should succeed");

        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let tasks = service
            .list_tasks(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert_eq!(tasks.runs.len(), 1);
        assert_eq!(tasks.runs[0].kind, TaskRunKind::MetadataRefreshFull);
        assert_eq!(tasks.runs[0].trigger, TaskRunTrigger::Schedule);
    }

    #[tokio::test]
    async fn load_subscription_preserves_existing_auto_refresh_due_times() {
        let project_id = "p-tasks-preserve-due-at";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 7.7.7.7
"#,
        )
        .await;
        let source = SubscriptionSource::File(source_path.clone());
        let import_id = service.proxy_import_id(
            &ProxyScope::project(project_id),
            &service.proxy_import_source_identity(&source),
        );
        let now = now_epoch_sec();
        let expected_sync_due_at = now + 123;
        let expected_full_due_at = now + 456;

        store
            .upsert_project_sync_config(&ProjectSyncConfig {
                import_id: import_id.clone(),
                project_id: project_id.to_string(),
                source: source.clone(),
                enabled: true,
                sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                last_sync_due_at: Some(expected_sync_due_at),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(expected_full_due_at),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            })
            .await
            .expect("sync config seed should succeed");

        service
            .load_subscription(project_id, &source)
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_proxy_import_sync_config(&import_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert_eq!(config.last_sync_due_at, Some(expected_sync_due_at));
        assert_eq!(config.last_full_refresh_due_at, Some(expected_full_due_at));
    }

    #[tokio::test]
    async fn load_subscription_advances_overdue_sync_due_without_moving_full_refresh_due_at() {
        let project_id = "p-tasks-preserve-overdue-full-refresh-due-at";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: new
    type: socks5
    server: 7.7.7.7
"#,
        )
        .await;
        let source = SubscriptionSource::File(source_path.clone());
        let import_id = service.proxy_import_id(
            &ProxyScope::project(project_id),
            &service.proxy_import_source_identity(&source),
        );
        let now = now_epoch_sec();

        store
            .upsert_project_sync_config(&ProjectSyncConfig {
                import_id: import_id.clone(),
                project_id: project_id.to_string(),
                source: source.clone(),
                enabled: true,
                sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                last_sync_due_at: Some(now - 5),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(now - 10),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            })
            .await
            .expect("sync config seed should succeed");

        service
            .load_subscription(project_id, &source)
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_proxy_import_sync_config(&import_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert!(config.last_sync_due_at.expect("sync due at") > now);
        assert_eq!(config.last_full_refresh_due_at, Some(now - 10));
    }

    #[tokio::test]
    async fn load_subscription_creates_post_load_task_even_when_no_new_ips_arrive() {
        let project_id = "p-tasks-no-new-ips";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;

        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("first load should succeed");
        let mut existing_runs = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        let mut first_run = existing_runs.remove(0);
        first_run.status = TaskRunStatus::Succeeded;
        first_run.stage = TaskRunStage::Completed;
        first_run.finished_at = Some(now_epoch_sec());
        store
            .update_task_run(&first_run)
            .await
            .expect("task run update should succeed");
        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let tasks = service
            .list_tasks(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert_eq!(tasks.runs.len(), 2);
        assert!(tasks.runs.iter().all(|run| {
            run.kind == TaskRunKind::MetadataRefreshIncremental
                && run.trigger == TaskRunTrigger::PostLoad
        }));
    }

    #[tokio::test]
    async fn failed_full_refresh_advances_due_at_before_retry() {
        let project_id = "p-tasks-full-refresh-retry";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::with_failures(true, false));
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;
        let now = now_epoch_sec();

        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let mut config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should exist");
        config.last_full_refresh_due_at = Some(now - 1);
        store
            .upsert_project_sync_config(&config)
            .await
            .expect("sync config update should succeed");

        let mut run = service
            .enqueue_task_run(
                project_id,
                TaskRunKind::MetadataRefreshFull,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("full refresh queue should succeed");

        let err = service
            .execute_full_refresh_task(&mut run)
            .await
            .expect_err("full refresh should fail");
        assert!(matches!(err, BrokerError::MihomoUnavailable(_)));
        service
            .fail_task_run(&mut run, err)
            .await
            .expect("failure closeout should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert_ne!(config.last_full_refresh_due_at, Some(now - 1));
        assert!(
            config
                .last_full_refresh_due_at
                .expect("full refresh due at")
                >= now + DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC as i64
        );
    }

    #[tokio::test]
    async fn failed_subscription_sync_advances_due_at_before_retry() {
        let project_id = "p-tasks-sync-retry";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::with_failures(true, false));
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;
        let now = now_epoch_sec();

        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("initial load should succeed");
        tokio::fs::write(
            &source_path,
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
  - name: second
    type: socks5
    server: 5.5.5.5
"#,
        )
        .await
        .expect("subscription rewrite should succeed");

        let mut config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should exist");
        config.last_sync_due_at = Some(now - 1);
        store
            .upsert_project_sync_config(&config)
            .await
            .expect("sync config update should succeed");

        let mut run = service
            .enqueue_task_run(
                project_id,
                TaskRunKind::SubscriptionSync,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("subscription sync queue should succeed");

        let err = service
            .execute_subscription_sync_task(&mut run)
            .await
            .expect_err("subscription sync should fail");
        assert!(matches!(err, BrokerError::MihomoUnavailable(_)));
        service
            .fail_task_run(&mut run, err)
            .await
            .expect("failure closeout should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert_ne!(config.last_sync_due_at, Some(now - 1));
        assert!(
            config.last_sync_due_at.expect("sync due at")
                >= now + DEFAULT_AUTO_SYNC_EVERY_SEC as i64
        );
    }

    #[tokio::test]
    async fn failed_subscription_sync_event_includes_subscription_attempt_detail() {
        let project_id = "p-tasks-sync-invalid-detail";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;
        let now = now_epoch_sec();

        service
            .load_subscription(project_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("initial load should succeed");

        let config = store
            .get_project_sync_config(project_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should exist");
        let (url, server) =
            spawn_subscription_server("error code: 1102", StatusCode::OK, None, None).await;
        let mut due_config = config.clone();
        due_config.source = SubscriptionSource::Url(url);
        due_config.last_sync_due_at = Some(now - 1);
        store
            .upsert_project_sync_config(&due_config)
            .await
            .expect("sync config update should succeed");

        let mut run = service
            .enqueue_task_run(
                project_id,
                TaskRunKind::SubscriptionSync,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("subscription sync queue should succeed");

        let err = service
            .execute_subscription_sync_task(&mut run)
            .await
            .expect_err("subscription sync should fail on invalid source");
        assert!(
            matches!(&err, BrokerError::SubscriptionInvalidDetail(message)
                if message.contains(&config.import_id)
                    && message.contains("error code: 1102")
                    && message.contains("shape: bytes="))
        );
        service
            .fail_task_run(&mut run, err)
            .await
            .expect("failure closeout should succeed");

        server.abort();
        let _ = tokio::fs::remove_file(&source_path).await;

        let inventory = store
            .list_proxy_inventory_for_import(&config.import_id)
            .await
            .expect("inventory should list");
        assert_eq!(inventory.len(), 1, "failed sync should keep old inventory");
        assert_eq!(inventory[0].proxy_name, "first");

        let events = store
            .list_task_run_events(&run.run_id)
            .await
            .expect("events should list");
        let failed_event = events
            .iter()
            .rev()
            .find(|event| event.stage == TaskRunStage::Completed)
            .expect("failed completion event should exist");
        let payload = failed_event
            .payload_json
            .as_ref()
            .expect("failed event should include payload");
        assert_eq!(payload["error"]["code"], "subscription_invalid");
        let reason = payload["error"]["details"]["reason"]
            .as_str()
            .expect("subscription invalid reason should be recorded");
        assert!(reason.contains(&config.import_id));
        assert!(reason.contains("error code: 1102"));
        assert!(reason.contains("shape: bytes="));
    }

    #[tokio::test]
    async fn failed_subscription_sync_only_advances_attempted_imports() {
        let project_id = "p-tasks-sync-partial-failure";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let now = now_epoch_sec();

        store
            .create_project(project_id, now)
            .await
            .expect("project create should succeed");

        let first_source_path = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;
        let second_source_path = write_subscription_file(
            r#"
proxies:
  - name: second
    type: socks5
    server: 5.5.5.5
"#,
        )
        .await;
        let third_source_path = write_subscription_file(
            r#"
proxies:
  - name: third
    type: socks5
    server: 6.6.6.6
"#,
        )
        .await;

        let mut imports = vec![
            SubscriptionSource::File(first_source_path.clone()),
            SubscriptionSource::File(second_source_path.clone()),
            SubscriptionSource::File(third_source_path.clone()),
        ]
        .into_iter()
        .map(|source| {
            let import_id = service.proxy_import_id(
                &ProxyScope::project(project_id),
                &service.proxy_import_source_identity(&source),
            );
            (source, import_id)
        })
        .collect::<Vec<_>>();
        imports.sort_by(|left, right| left.1.cmp(&right.1));

        let failing_source_path = match &imports[1].0 {
            SubscriptionSource::File(path) => path.clone(),
            SubscriptionSource::Url(_) => unreachable!("test sources are file-backed"),
        };
        tokio::fs::remove_file(&failing_source_path)
            .await
            .expect("failing source should be removed");

        for (source, import_id) in &imports {
            store
                .upsert_project_sync_config(&ProjectSyncConfig {
                    import_id: import_id.clone(),
                    project_id: project_id.to_string(),
                    source: source.clone(),
                    enabled: true,
                    sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                    full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                    last_sync_due_at: Some(now - 1),
                    last_sync_started_at: None,
                    last_sync_finished_at: None,
                    last_full_refresh_due_at: Some(now + 3600),
                    last_full_refresh_started_at: None,
                    last_full_refresh_finished_at: None,
                    updated_at: now,
                })
                .await
                .expect("sync config seed should succeed");
        }

        let mut run = service
            .enqueue_task_run(
                project_id,
                TaskRunKind::SubscriptionSync,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("subscription sync queue should succeed");

        let err = service
            .execute_subscription_sync_task(&mut run)
            .await
            .expect_err("subscription sync should fail on the missing second source");
        service
            .fail_task_run(&mut run, err)
            .await
            .expect("failure closeout should succeed");

        let first_config = store
            .get_proxy_import_sync_config(&imports[0].1)
            .await
            .expect("first config query should succeed")
            .expect("first config should persist");
        let failed_config = store
            .get_proxy_import_sync_config(&imports[1].1)
            .await
            .expect("failed config query should succeed")
            .expect("failed config should persist");
        let untouched_config = store
            .get_proxy_import_sync_config(&imports[2].1)
            .await
            .expect("untouched config query should succeed")
            .expect("untouched config should persist");

        assert!(
            first_config.last_sync_due_at.expect("first due at")
                >= now + DEFAULT_AUTO_SYNC_EVERY_SEC as i64
        );
        assert!(
            failed_config.last_sync_due_at.expect("failed due at")
                >= now + DEFAULT_AUTO_SYNC_EVERY_SEC as i64
        );
        assert_eq!(untouched_config.last_sync_due_at, Some(now - 1));

        let _ = tokio::fs::remove_file(&first_source_path).await;
        let _ = tokio::fs::remove_file(&second_source_path).await;
        let _ = tokio::fs::remove_file(&third_source_path).await;
    }

    #[tokio::test]
    async fn load_subscription_coalesces_post_load_task_scope() {
        let project_id = "p-tasks-coalesce";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let first_source = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;
        let second_source = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 2.2.2.2
  - name: second
    type: socks5
    server: 3.3.3.3
"#,
        )
        .await;

        service
            .load_subscription(project_id, &SubscriptionSource::File(first_source.clone()))
            .await
            .expect("first load should succeed");
        service
            .load_subscription(project_id, &SubscriptionSource::File(second_source.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&first_source).await;
        let _ = tokio::fs::remove_file(&second_source).await;

        let tasks = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list query should succeed");
        assert_eq!(tasks.len(), 1);
        let run = tasks.first().expect("coalesced task should exist");
        match &run.scope {
            TaskRunScope::Ips { ips } => {
                let ips = ips.iter().cloned().collect::<HashSet<_>>();
                assert_eq!(ips.len(), 2);
                assert!(ips.contains("2.2.2.2"));
                assert!(ips.contains("3.3.3.3"));
            }
            TaskRunScope::All => panic!("post-load task should stay scoped to explicit IPs"),
            TaskRunScope::Nodes { .. } => panic!("post-load task should not use node scope"),
        }
    }

    #[tokio::test]
    async fn load_subscription_queues_follow_up_post_load_task_when_incremental_is_running() {
        let project_id = "p-tasks-follow-up-while-running";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let first_source = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 2.2.2.2
"#,
        )
        .await;
        let second_source = write_subscription_file(
            r#"
proxies:
  - name: first
    type: socks5
    server: 2.2.2.2
  - name: second
    type: socks5
    server: 3.3.3.3
"#,
        )
        .await;

        service
            .load_subscription(project_id, &SubscriptionSource::File(first_source.clone()))
            .await
            .expect("first load should succeed");

        let mut run = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list query should succeed")
            .into_iter()
            .next()
            .expect("incremental run should exist");
        run.status = TaskRunStatus::Running;
        run.stage = TaskRunStage::DiffingInventory;
        run.started_at = Some(now_epoch_sec());
        store
            .update_task_run(&run)
            .await
            .expect("task run update should succeed");

        service
            .load_subscription(project_id, &SubscriptionSource::File(second_source.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&first_source).await;
        let _ = tokio::fs::remove_file(&second_source).await;

        let tasks = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list query should succeed");
        assert_eq!(tasks.len(), 2);
        assert!(
            tasks
                .iter()
                .any(|task| task.status == TaskRunStatus::Running)
        );
        let queued_run = tasks
            .iter()
            .find(|task| {
                task.status == TaskRunStatus::Queued
                    && task.kind == TaskRunKind::MetadataRefreshIncremental
                    && task.trigger == TaskRunTrigger::PostLoad
            })
            .expect("follow-up queued incremental should exist");
        match &queued_run.scope {
            TaskRunScope::Ips { ips } => {
                let ips = ips.iter().cloned().collect::<HashSet<_>>();
                assert_eq!(ips.len(), 1);
                assert!(ips.contains("3.3.3.3"));
            }
            TaskRunScope::All => panic!("follow-up task should stay scoped to explicit IPs"),
            TaskRunScope::Nodes { .. } => panic!("follow-up task should not use node scope"),
        }
    }

    #[tokio::test]
    async fn enqueue_due_tasks_queues_sync_then_full_refresh_for_due_project() {
        let project_id = "p-schedule";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let now = now_epoch_sec();

        store
            .upsert_project_sync_config(&ProjectSyncConfig {
                import_id: format!("import::{project_id}"),
                project_id: project_id.to_string(),
                source: SubscriptionSource::Url("https://example.com/subscription".to_string()),
                enabled: true,
                sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                last_sync_due_at: Some(now - 1),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(now - 1),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            })
            .await
            .expect("config seed should succeed");

        service
            .enqueue_due_tasks()
            .await
            .expect("due tasks should enqueue");

        let tasks = service
            .list_tasks(&TaskListQuery {
                project_id: Some(project_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert_eq!(tasks.runs.len(), 2);
        let kinds = tasks
            .runs
            .iter()
            .map(|run| run.kind)
            .collect::<HashSet<_>>();
        assert!(kinds.contains(&TaskRunKind::SubscriptionSync));
        assert!(kinds.contains(&TaskRunKind::MetadataRefreshFull));
    }

    #[tokio::test]
    async fn enqueue_due_tasks_queues_global_proxy_probe_after_configured_interval() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let now = now_epoch_sec();
        service
            .update_system_settings(60)
            .await
            .expect("settings should update");
        let import_id = "imp-scheduled-probe".to_string();
        store
            .replace_proxy_inventory_import(
                &ProxyImportRecord {
                    import_id: import_id.clone(),
                    name: Some("scheduled-probe".to_string()),
                    import_kind: ProxyImportKind::Subscription,
                    source_scope: ProxyScope::global(),
                    source_identity: ProxyImportSourceIdentity::manual(&import_id),
                    allocation_scope: ProxyScope::global(),
                    subscription_metadata: None,
                    created_at: now,
                    updated_at: now,
                },
                &[ProxyInventoryRecord {
                    import_id: import_id.clone(),
                    node_id: "node-scheduled-probe".to_string(),
                    source_scope: ProxyScope::global(),
                    allocation_scope: ProxyScope::global(),
                    proxy_name: "scheduled-probe".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "8.8.4.4".to_string(),
                    resolved_ips: vec!["8.8.4.4".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "scheduled-probe",
                        "type": "socks5",
                        "server": "8.8.4.4",
                    }),
                    created_at: now,
                    updated_at: now,
                }],
            )
            .await
            .expect("inventory should seed");

        service
            .enqueue_due_tasks()
            .await
            .expect("due tasks should enqueue");
        service
            .enqueue_due_tasks()
            .await
            .expect("fresh scheduled probe should not duplicate");

        let tasks = store
            .list_task_runs(&TaskListQuery {
                project_id: Some(GLOBAL_RUNTIME_PROJECT_ID.to_string()),
                kind: Some(TaskRunKind::ProxyLatencyProbe),
                trigger: Some(TaskRunTrigger::Schedule),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert_eq!(tasks.len(), 1);
        assert!(matches!(tasks[0].scope, TaskRunScope::All));
    }

    #[tokio::test]
    async fn system_settings_default_and_validation_are_persisted() {
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());

        let defaults = service
            .get_system_settings()
            .await
            .expect("default settings should load");
        assert_eq!(
            defaults.proxy_probe_interval_sec,
            DEFAULT_PROXY_PROBE_INTERVAL_SEC
        );
        assert!(matches!(
            service.update_system_settings(59).await,
            Err(BrokerError::InvalidRequest(_))
        ));

        let updated = service
            .update_system_settings(900)
            .await
            .expect("settings should update");
        assert_eq!(updated.proxy_probe_interval_sec, 900);
        let loaded = service
            .get_system_settings()
            .await
            .expect("updated settings should load");
        assert_eq!(loaded.proxy_probe_interval_sec, 900);
    }

    #[test]
    fn dispatch_sort_keeps_due_sync_before_full_refresh_for_same_project() {
        let mut runs = vec![
            TaskRunRecord {
                run_id: "zzz".to_string(),
                project_id: "project-a".to_string(),
                kind: TaskRunKind::MetadataRefreshFull,
                trigger: TaskRunTrigger::Schedule,
                status: TaskRunStatus::Queued,
                stage: TaskRunStage::Queued,
                progress_current: Some(0),
                progress_total: None,
                created_at: 42,
                started_at: None,
                finished_at: None,
                summary_json: None,
                error_code: None,
                error_message: None,
                scope: TaskRunScope::All,
            },
            TaskRunRecord {
                run_id: "aaa".to_string(),
                project_id: "project-a".to_string(),
                kind: TaskRunKind::SubscriptionSync,
                trigger: TaskRunTrigger::Schedule,
                status: TaskRunStatus::Queued,
                stage: TaskRunStage::Queued,
                progress_current: Some(0),
                progress_total: None,
                created_at: 42,
                started_at: None,
                finished_at: None,
                summary_json: None,
                error_code: None,
                error_message: None,
                scope: TaskRunScope::All,
            },
        ];

        sort_queued_runs_for_dispatch(&mut runs);

        assert_eq!(runs[0].kind, TaskRunKind::SubscriptionSync);
        assert_eq!(runs[1].kind, TaskRunKind::MetadataRefreshFull);
    }

    #[tokio::test]
    async fn subscription_sync_defers_incremental_refresh_when_full_refresh_is_queued() {
        let project_id = "p-sync-deferred";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let source_path = write_subscription_file(
            r#"
proxies:
  - name: fresh
    type: socks5
    server: 4.4.4.4
"#,
        )
        .await;
        let now = now_epoch_sec();

        store
            .upsert_project_sync_config(&ProjectSyncConfig {
                import_id: format!("import::{project_id}"),
                project_id: project_id.to_string(),
                source: SubscriptionSource::File(source_path.clone()),
                enabled: true,
                sync_every_sec: DEFAULT_AUTO_SYNC_EVERY_SEC,
                full_refresh_every_sec: DEFAULT_AUTO_FULL_REFRESH_EVERY_SEC,
                last_sync_due_at: Some(now - 1),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(now - 1),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            })
            .await
            .expect("sync config seed should succeed");

        service
            .enqueue_task_run(
                project_id,
                TaskRunKind::MetadataRefreshFull,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("full refresh queue should succeed");
        let mut sync_run = service
            .enqueue_task_run(
                project_id,
                TaskRunKind::SubscriptionSync,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("sync queue should succeed");

        service
            .execute_subscription_sync_task(&mut sync_run)
            .await
            .expect("sync should defer inline refresh");

        let _ = tokio::fs::remove_file(&source_path).await;

        let detail = service
            .get_task_run_detail(&sync_run.run_id)
            .await
            .expect("task detail should succeed");
        assert_eq!(detail.run.status, TaskRunStatus::Succeeded);
        assert_eq!(
            detail.run.summary_json,
            Some(serde_json::json!({
                "loaded_proxies": 1,
                "distinct_ips": 1,
                "warnings": [],
                "new_ips": 1,
                "probed_ips": 0,
                "geo_updated": 0,
                "skipped_cached": 0,
                "deferred_to_full_refresh": true,
            }))
        );

        let probe_records = store
            .list_probe_records(project_id)
            .await
            .expect("probe record query should succeed");
        assert!(probe_records.is_empty());
    }
}
