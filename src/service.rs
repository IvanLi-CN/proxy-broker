use std::{
    cmp::Ordering as CmpOrdering,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr},
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
        DEFAULT_PROBE_TIMEOUT_MS, DEFAULT_PROBE_TTL_SEC, DEFAULT_SESSION_LISTEN_IP,
    },
    error::{BrokerError, BrokerResult},
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProfileResponse, ExtractIpItem,
        ExtractIpRequest, ExtractIpResponse, IpRecord, ListApiKeysResponse, ListProfilesResponse,
        ListSessionsResponse, LoadSubscriptionResponse, OpenBatchRequest, OpenBatchResponse,
        OpenSessionRequest, OpenSessionResponse, ProbeRecord, ProfileSyncConfig, ProxyNode,
        RefreshRequest, RefreshResponse, SearchSessionOptionsRequest,
        SearchSessionOptionsResponse, SessionOptionItem, SessionOptionKind, SessionRecord,
        SessionSelectionMode, SubscriptionSource, SuggestedPortResponse, TaskEventLevel,
        TaskListQuery, TaskListResponse, TaskRunDetail, TaskRunEventRecord, TaskRunKind,
        TaskRunRecord, TaskRunScope, TaskRunStage, TaskRunStatus, TaskRunSummary,
        TaskRunTrigger, now_epoch_sec,
    },
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
    task_events: broadcast::Sender<TaskBusEvent>,
    task_active_profiles: Arc<TokioMutex<HashSet<String>>>,
    task_supervisor_started: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct LoadSubscriptionOutcome {
    response: LoadSubscriptionResponse,
    new_ips: Vec<String>,
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
            profile_locks: (0..64).map(|_| Arc::new(TokioMutex::new(()))).collect(),
            task_events,
            task_active_profiles: Arc::new(TokioMutex::new(HashSet::new())),
            task_supervisor_started: Arc::new(AtomicBool::new(false)),
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
            .list_profile_sync_configs()
            .await
            .map_err(BrokerError::from)?;
        let now = now_epoch_sec();

        for config in configs {
            if !config.enabled {
                continue;
            }
            if self
                .has_pending_or_running_tasks(&config.profile_id)
                .await?
            {
                continue;
            }

            let sync_due = config.last_sync_due_at.map(|ts| ts <= now).unwrap_or(false);
            let full_due = config
                .last_full_refresh_due_at
                .map(|ts| ts <= now)
                .unwrap_or(false);

            if !sync_due && !full_due {
                continue;
            }

            if sync_due {
                self.enqueue_task_run(
                    &config.profile_id,
                    TaskRunKind::SubscriptionSync,
                    TaskRunTrigger::Schedule,
                    TaskRunScope::All,
                )
                .await?;
            }

            if full_due {
                self.enqueue_task_run(
                    &config.profile_id,
                    TaskRunKind::MetadataRefreshFull,
                    TaskRunTrigger::Schedule,
                    TaskRunScope::All,
                )
                .await?;
            }
        }

        Ok(())
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
            if !self.claim_task_profile(&run.profile_id).await {
                continue;
            }

            let service = Arc::clone(self);
            tokio::spawn(async move {
                service.run_task(run).await;
            });
        }

        Ok(())
    }

    async fn claim_task_profile(&self, profile_id: &str) -> bool {
        let mut active = self.task_active_profiles.lock().await;
        active.insert(profile_id.to_string())
    }

    async fn release_task_profile(&self, profile_id: &str) {
        let mut active = self.task_active_profiles.lock().await;
        active.remove(profile_id);
    }

    async fn has_pending_or_running_tasks(&self, profile_id: &str) -> BrokerResult<bool> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .map_err(BrokerError::from)?;
        Ok(runs
            .into_iter()
            .any(|run| matches!(run.status, TaskRunStatus::Queued | TaskRunStatus::Running)))
    }

    async fn queued_or_running_task_runs(
        &self,
        profile_id: &str,
    ) -> BrokerResult<Vec<TaskRunRecord>> {
        let runs = self
            .store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        };

        if let Err(err) = result {
            tracing::warn!(
                run_id = %run.run_id,
                profile_id = %run.profile_id,
                error = %err,
                "task run failed"
            );
            let _ = self.fail_task_run(&mut run, err).await;
        }

        self.release_task_profile(&run.profile_id).await;
    }

    async fn execute_subscription_sync_task(&self, run: &mut TaskRunRecord) -> BrokerResult<()> {
        self.mark_task_running(run, TaskRunStage::LoadingSubscription, None, None)
            .await?;
        self.append_task_event(
            run,
            TaskEventLevel::Info,
            TaskRunStage::LoadingSubscription,
            "Refreshing subscription feed for profile.",
            None,
        )
        .await?;

        self.mark_sync_started(&run.profile_id).await?;
        let config = self
            .store
            .get_profile_sync_config(&run.profile_id)
            .await
            .map_err(BrokerError::from)?
            .ok_or_else(|| {
                BrokerError::InvalidRequest(format!(
                    "profile `{}` has no persisted subscription source",
                    run.profile_id
                ))
            })?;
        let outcome = self
            .load_subscription_internal(&run.profile_id, &config.source)
            .await?;
        let targeted_ips = outcome.new_ips.len() as u64;
        run.progress_total = Some(targeted_ips);
        self.update_task_run_and_emit(run).await?;
        self.append_task_event(
            run,
            TaskEventLevel::Info,
            TaskRunStage::DiffingInventory,
            format!(
                "Subscription sync finished with {} new IP(s).",
                outcome.new_ips.len()
            ),
            Some(serde_json::json!({
                "loaded_proxies": outcome.response.loaded_proxies,
                "distinct_ips": outcome.response.distinct_ips,
                "warnings": outcome.response.warnings,
                "new_ips": outcome.new_ips,
            })),
        )
        .await?;

        if outcome.new_ips.is_empty() {
            self.mark_sync_finished(&run.profile_id, now_epoch_sec())
                .await?;
            self.complete_task_run(
                run,
                TaskRunStatus::Succeeded,
                Some(serde_json::json!({
                    "loaded_proxies": outcome.response.loaded_proxies,
                    "distinct_ips": outcome.response.distinct_ips,
                    "warnings": outcome.response.warnings,
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
            .queued_or_running_task_runs(&run.profile_id)
            .await?
            .into_iter()
            .any(|queued_run| {
                queued_run.run_id != run.run_id
                    && queued_run.kind == TaskRunKind::MetadataRefreshFull
            })
        {
            self.mark_sync_finished(&run.profile_id, now_epoch_sec())
                .await?;
            self.complete_task_run(
                run,
                TaskRunStatus::Succeeded,
                Some(serde_json::json!({
                    "loaded_proxies": outcome.response.loaded_proxies,
                    "distinct_ips": outcome.response.distinct_ips,
                    "warnings": outcome.response.warnings,
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

        let target_ip_set = outcome.new_ips.iter().cloned().collect::<HashSet<_>>();
        let refresh = self
            .refresh_metadata_internal(
                &run.profile_id,
                false,
                Some(&target_ip_set),
                Some(&run.run_id),
            )
            .await?;

        self.mark_sync_finished(&run.profile_id, now_epoch_sec())
            .await?;
        self.complete_task_run(
            run,
            TaskRunStatus::Succeeded,
            Some(serde_json::json!({
                "loaded_proxies": outcome.response.loaded_proxies,
                "distinct_ips": outcome.response.distinct_ips,
                "warnings": outcome.response.warnings,
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
            TaskRunScope::All => self
                .store
                .list_ip_records(&run.profile_id)
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
                &run.profile_id,
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
        self.mark_full_refresh_started(&run.profile_id).await?;
        let refresh = self
            .refresh_metadata_internal(&run.profile_id, true, None, Some(&run.run_id))
            .await;
        let refresh = refresh?;
        self.mark_full_refresh_finished(&run.profile_id, now_epoch_sec())
            .await?;

        let targeted_ips = self
            .store
            .list_ip_records(&run.profile_id)
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
        profile_id: &str,
        kind: TaskRunKind,
        trigger: TaskRunTrigger,
        scope: TaskRunScope,
    ) -> BrokerResult<TaskRunRecord> {
        let run = TaskRunRecord {
            run_id: uuid::Uuid::new_v4().to_string(),
            profile_id: profile_id.to_string(),
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
            event_id: uuid::Uuid::new_v4().to_string(),
            run_id: run.run_id.clone(),
            profile_id: run.profile_id.clone(),
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
                TaskRunKind::SubscriptionSync => {
                    self.mark_sync_failed(&run.profile_id, failed_at).await?;
                }
                TaskRunKind::MetadataRefreshFull => {
                    self.mark_full_refresh_failed(&run.profile_id, failed_at)
                        .await?;
                }
                TaskRunKind::MetadataRefreshIncremental => {}
            }
        }
        self.complete_task_run(
            run,
            TaskRunStatus::Failed,
            None,
            Some(error.code().to_string()),
            Some(error.to_string()),
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

    async fn register_profile_sync_source(
        &self,
        profile_id: &str,
        source: &SubscriptionSource,
    ) -> BrokerResult<()> {
        let now = now_epoch_sec();
        let mut config = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
            .unwrap_or(ProfileSyncConfig {
                profile_id: profile_id.to_string(),
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
            .upsert_profile_sync_config(&config)
            .await
            .map_err(BrokerError::from)
    }

    async fn mark_sync_started(&self, profile_id: &str) -> BrokerResult<()> {
        let now = now_epoch_sec();
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_sync_started_at = Some(now);
            config.updated_at = now;
            self.store
                .upsert_profile_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_sync_finished(&self, profile_id: &str, finished_at: i64) -> BrokerResult<()> {
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_sync_finished_at = Some(finished_at);
            config.last_sync_due_at = Some(finished_at + config.sync_every_sec as i64);
            config.updated_at = finished_at;
            self.store
                .upsert_profile_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_sync_failed(&self, profile_id: &str, failed_at: i64) -> BrokerResult<()> {
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_sync_due_at = Some(preserve_or_advance_due_at(
                config.last_sync_due_at,
                failed_at,
                config.sync_every_sec,
            ));
            config.updated_at = failed_at;
            self.store
                .upsert_profile_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_started(&self, profile_id: &str) -> BrokerResult<()> {
        let now = now_epoch_sec();
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_full_refresh_started_at = Some(now);
            config.updated_at = now;
            self.store
                .upsert_profile_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_failed(&self, profile_id: &str, failed_at: i64) -> BrokerResult<()> {
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_full_refresh_due_at = Some(preserve_or_advance_due_at(
                config.last_full_refresh_due_at,
                failed_at,
                config.full_refresh_every_sec,
            ));
            config.updated_at = failed_at;
            self.store
                .upsert_profile_sync_config(&config)
                .await
                .map_err(BrokerError::from)?;
        }
        Ok(())
    }

    async fn mark_full_refresh_finished(
        &self,
        profile_id: &str,
        finished_at: i64,
    ) -> BrokerResult<()> {
        if let Some(mut config) = self
            .store
            .get_profile_sync_config(profile_id)
            .await
            .map_err(BrokerError::from)?
        {
            config.last_full_refresh_finished_at = Some(finished_at);
            config.last_full_refresh_due_at =
                Some(finished_at + config.full_refresh_every_sec as i64);
            config.updated_at = finished_at;
            self.store
                .upsert_profile_sync_config(&config)
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
        let outcome = self.load_subscription_internal(profile_id, source).await?;
        let mut response = outcome.response;
        if let Err(err) = self
            .register_post_load_bookkeeping(profile_id, source, &outcome.new_ips)
            .await
        {
            tracing::warn!(
                profile_id,
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
        profile_id: &str,
        request: &RefreshRequest,
    ) -> BrokerResult<RefreshResponse> {
        self.refresh_metadata_internal(profile_id, request.force, None, None)
            .await
    }

    async fn load_subscription_internal(
        &self,
        profile_id: &str,
        source: &SubscriptionSource,
    ) -> BrokerResult<LoadSubscriptionOutcome> {
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
        let existing_ip_keys: HashSet<String> = existing_ip_map.keys().cloned().collect();

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

        let mut new_ips = valid_ips
            .difference(&existing_ip_keys)
            .cloned()
            .collect::<Vec<_>>();
        new_ips.sort();

        Ok(LoadSubscriptionOutcome {
            response: LoadSubscriptionResponse {
                loaded_proxies: nodes.len(),
                distinct_ips: valid_ips.len(),
                warnings,
            },
            new_ips,
        })
    }

    async fn register_post_load_bookkeeping(
        &self,
        profile_id: &str,
        source: &SubscriptionSource,
        new_ips: &[String],
    ) -> BrokerResult<()> {
        self.register_profile_sync_source(profile_id, source)
            .await?;

        let queued_or_running = self.queued_or_running_task_runs(profile_id).await?;
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
                profile_id,
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
        profile_id: &str,
        force: bool,
        target_ips: Option<&HashSet<String>>,
        run_id: Option<&str>,
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
            .list_probe_records(profile_id)
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
            self.refresh_probe_records(profile_id, now, &nodes, Some(&scoped_ip_set))
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
                .upsert_probe_records(profile_id, &probe_records)
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
                profile_id,
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
            .upsert_ip_records(profile_id, &ip_records)
            .await
            .map_err(BrokerError::from)?;

        if !should_probe {
            probe_records = filter_probe_records_to_ips(
                &self
                    .store
                    .list_probe_records(profile_id)
                    .await
                    .map_err(BrokerError::from)?,
                &scoped_ip_set,
            );
        }

        let sessions = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        self.cleanup_profile_runtime_if_idle(profile_id, &sessions)
            .await;

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
        profile_id: &str,
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

    pub async fn suggested_port(&self, profile_id: &str) -> BrokerResult<SuggestedPortResponse> {
        if !self.profile_exists(profile_id).await? {
            return Err(BrokerError::ProfileNotFound);
        }

        let _profile_guard = self.lock_profile(profile_id).await;
        let existing = self
            .store
            .list_sessions(profile_id)
            .await
            .map_err(BrokerError::from)?;
        let port = allocate_port(&existing, None, self.options.session_listen_ip)?;
        Ok(SuggestedPortResponse { port })
    }

    pub async fn search_session_options(
        &self,
        profile_id: &str,
        request: &SearchSessionOptionsRequest,
    ) -> BrokerResult<SearchSessionOptionsResponse> {
        if !self.profile_exists(profile_id).await? {
            return Err(BrokerError::ProfileNotFound);
        }

        let ip_records = self
            .store
            .list_ip_records(profile_id)
            .await
            .map_err(BrokerError::from)?;

        let items = search_session_options(&ip_records, request)?;
        Ok(SearchSessionOptionsResponse { items })
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

fn normalize_country_codes(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let item = value.trim().to_ascii_uppercase();
        if item.is_empty() || !seen.insert(item.clone()) {
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
        let trimmed = value.trim();
        let item = trimmed
            .split_once("::")
            .map(|(_, city)| city.trim())
            .unwrap_or(trimmed)
            .to_string();
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
                (Some(country.trim().to_ascii_uppercase()), city.to_ascii_lowercase())
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
    let limit = request.limit.unwrap_or(DEFAULT_SESSION_OPTIONS_LIMIT).min(100);

    let items = match request.kind {
        SessionOptionKind::Country => {
            let mut countries = HashMap::<String, SessionOptionItem>::new();
            for record in ip_records {
                let Some(code) = record.country_code.as_ref() else {
                    continue;
                };
                let country_code = code.trim().to_ascii_uppercase();
                if country_code.is_empty() {
                    continue;
                }
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
                countries.entry(country_code.clone()).or_insert(SessionOptionItem {
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
                if !country_filters.is_empty() {
                    let Some(code) = record.country_code.as_ref() else {
                        continue;
                    };
                    if !country_filters.contains(&code.to_ascii_uppercase()) {
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
                let country_code = record.country_code.clone().unwrap_or_default();
                let country_name = record.country_name.clone().unwrap_or_default();
                let meta = match (country_code.trim(), country_name.trim()) {
                    ("", "") => None,
                    ("", name) => Some(name.to_string()),
                    (code, "") => Some(code.to_string()),
                    (code, name) => Some(format!("{name} ({code})")),
                };
                let value = if country_code.trim().is_empty() {
                    city_value.clone()
                } else {
                    format!("{}::{}", country_code.trim().to_ascii_uppercase(), city_value)
                };
                let key = value.to_ascii_lowercase();
                let haystack = format!(
                    "{} {} {}",
                    city_value.to_ascii_lowercase(),
                    country_code.to_ascii_lowercase(),
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
                    if !country_filters.is_empty() {
                        let Some(code) = record.country_code.as_ref() else {
                            return false;
                        };
                        if !country_filters.contains(&code.to_ascii_uppercase()) {
                            return false;
                        }
                    }
                    if !city_filters.is_empty() {
                        let Some(city) = record.city.as_ref() else {
                            return false;
                        };
                        let city_name = city.trim().to_ascii_lowercase();
                        let country_code = record
                            .country_code
                            .as_ref()
                            .map(|code| code.trim().to_ascii_uppercase());
                        let matched = city_filters.iter().any(|(country_filter, city_filter)| {
                            city_name == *city_filter
                                && match country_filter {
                                    Some(code) => country_code.as_deref() == Some(code.as_str()),
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
                        record.country_code.as_deref().unwrap_or_default().to_ascii_lowercase(),
                        record.country_name.as_deref().unwrap_or_default().to_ascii_lowercase(),
                        record.city.as_deref().unwrap_or_default().to_ascii_lowercase()
                    );
                    haystack.contains(&query)
                })
                .map(|record| SessionOptionItem {
                    value: record.ip.clone(),
                    label: record.ip.clone(),
                    meta: {
                        let geo = [record.country_code.clone(), record.city.clone()]
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
    let selector = build_open_selector_request(request)?;
    let mut items = filter_ip_records(ip_records.to_vec(), probes, &selector)?;

    if matches!(request.selection_mode, SessionSelectionMode::Any) {
        // Preserve the legacy auto-pick quality bar for the unrestricted path:
        // healthy, low-latency candidates win before recency breaks ties.
        items.sort_by(|a, b| {
            let recency = match request.sort_mode {
                crate::models::SortMode::Mru => b.last_used_at.cmp(&a.last_used_at),
                crate::models::SortMode::Lru => a.last_used_at.cmp(&b.last_used_at),
            };
            b.probe_ok
                .cmp(&a.probe_ok)
                .then_with(|| a.best_latency_ms.cmp(&b.best_latency_ms))
                .then_with(|| recency)
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

fn sort_queued_runs_for_dispatch(runs: &mut [TaskRunRecord]) {
    runs.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| same_profile_schedule_dispatch_order(left, right))
            .then_with(|| left.run_id.cmp(&right.run_id))
    });
}

fn same_profile_schedule_dispatch_order(
    left: &TaskRunRecord,
    right: &TaskRunRecord,
) -> CmpOrdering {
    if left.profile_id != right.profile_id
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            ApiKeyRecord, IpRecord, ProbeRecord, ProfileSyncConfig, SessionRecord, SortMode,
            SubscriptionSource, TaskListQuery, TaskRunEventRecord, TaskRunRecord,
        },
        runtime::MihomoRuntime,
        store::{BrokerStore, MemoryStore},
        subscription::SUBSCRIPTION_FETCH_USER_AGENTS,
    };
    use anyhow::anyhow;
    use async_trait::async_trait;
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, StatusCode},
        routing::get,
    };
    use std::collections::HashSet;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::net::TcpListener;

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

    #[derive(Default)]
    struct FailProfileSyncConfigStore {
        inner: MemoryStore,
    }

    #[async_trait]
    impl BrokerStore for FailProfileSyncConfigStore {
        async fn list_profiles(&self) -> anyhow::Result<Vec<String>> {
            self.inner.list_profiles().await
        }

        async fn create_profile(&self, profile_id: &str, created_at: i64) -> anyhow::Result<()> {
            self.inner.create_profile(profile_id, created_at).await
        }

        async fn replace_subscription(
            &self,
            profile_id: &str,
            nodes: &[ProxyNode],
        ) -> anyhow::Result<()> {
            self.inner.replace_subscription(profile_id, nodes).await
        }

        async fn apply_subscription_snapshot(
            &self,
            profile_id: &str,
            nodes: &[ProxyNode],
            ip_records: &[IpRecord],
            probe_records: &[ProbeRecord],
            removed_session_ids: &[String],
        ) -> anyhow::Result<()> {
            self.inner
                .apply_subscription_snapshot(
                    profile_id,
                    nodes,
                    ip_records,
                    probe_records,
                    removed_session_ids,
                )
                .await
        }

        async fn list_subscription(&self, profile_id: &str) -> anyhow::Result<Vec<ProxyNode>> {
            self.inner.list_subscription(profile_id).await
        }

        async fn replace_ip_records(
            &self,
            profile_id: &str,
            records: &[IpRecord],
        ) -> anyhow::Result<()> {
            self.inner.replace_ip_records(profile_id, records).await
        }

        async fn upsert_ip_records(
            &self,
            profile_id: &str,
            records: &[IpRecord],
        ) -> anyhow::Result<()> {
            self.inner.upsert_ip_records(profile_id, records).await
        }

        async fn list_ip_records(&self, profile_id: &str) -> anyhow::Result<Vec<IpRecord>> {
            self.inner.list_ip_records(profile_id).await
        }

        async fn replace_probe_records(
            &self,
            profile_id: &str,
            records: &[ProbeRecord],
        ) -> anyhow::Result<()> {
            self.inner.replace_probe_records(profile_id, records).await
        }

        async fn upsert_probe_records(
            &self,
            profile_id: &str,
            records: &[ProbeRecord],
        ) -> anyhow::Result<()> {
            self.inner.upsert_probe_records(profile_id, records).await
        }

        async fn list_probe_records(&self, profile_id: &str) -> anyhow::Result<Vec<ProbeRecord>> {
            self.inner.list_probe_records(profile_id).await
        }

        async fn insert_session(
            &self,
            profile_id: &str,
            session: &SessionRecord,
        ) -> anyhow::Result<()> {
            self.inner.insert_session(profile_id, session).await
        }

        async fn insert_sessions(
            &self,
            profile_id: &str,
            sessions: &[SessionRecord],
        ) -> anyhow::Result<()> {
            self.inner.insert_sessions(profile_id, sessions).await
        }

        async fn insert_sessions_with_touch(
            &self,
            profile_id: &str,
            sessions: &[SessionRecord],
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .insert_sessions_with_touch(profile_id, sessions, last_used_at)
                .await
        }

        async fn delete_session(&self, profile_id: &str, session_id: &str) -> anyhow::Result<()> {
            self.inner.delete_session(profile_id, session_id).await
        }

        async fn list_sessions(&self, profile_id: &str) -> anyhow::Result<Vec<SessionRecord>> {
            self.inner.list_sessions(profile_id).await
        }

        async fn insert_api_key(&self, api_key: &ApiKeyRecord) -> anyhow::Result<()> {
            self.inner.insert_api_key(api_key).await
        }

        async fn get_api_key(&self, key_id: &str) -> anyhow::Result<Option<ApiKeyRecord>> {
            self.inner.get_api_key(key_id).await
        }

        async fn list_api_keys(&self, profile_id: &str) -> anyhow::Result<Vec<ApiKeyRecord>> {
            self.inner.list_api_keys(profile_id).await
        }

        async fn revoke_api_key(
            &self,
            profile_id: &str,
            key_id: &str,
            revoked_at: i64,
        ) -> anyhow::Result<bool> {
            self.inner
                .revoke_api_key(profile_id, key_id, revoked_at)
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
            profile_id: &str,
            ip: &str,
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .touch_ip_usage(profile_id, ip, last_used_at)
                .await
        }

        async fn touch_ip_usages(
            &self,
            profile_id: &str,
            ips: &[String],
            last_used_at: i64,
        ) -> anyhow::Result<()> {
            self.inner
                .touch_ip_usages(profile_id, ips, last_used_at)
                .await
        }

        async fn upsert_profile_sync_config(
            &self,
            _config: &ProfileSyncConfig,
        ) -> anyhow::Result<()> {
            Err(anyhow!("sync config unavailable"))
        }

        async fn get_profile_sync_config(
            &self,
            profile_id: &str,
        ) -> anyhow::Result<Option<ProfileSyncConfig>> {
            self.inner.get_profile_sync_config(profile_id).await
        }

        async fn list_profile_sync_configs(&self) -> anyhow::Result<Vec<ProfileSyncConfig>> {
            self.inner.list_profile_sync_configs().await
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

    #[derive(Clone)]
    struct TestSubscriptionServerState {
        payload: Arc<str>,
        status: StatusCode,
        accepted_user_agent: Option<Arc<str>>,
    }

    async fn test_subscription_handler(
        State(state): State<TestSubscriptionServerState>,
        headers: HeaderMap,
    ) -> (StatusCode, String) {
        let user_agent = headers
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok());
        if let Some(accepted_user_agent) = state.accepted_user_agent.as_deref()
            && user_agent != Some(accepted_user_agent)
        {
            return (StatusCode::OK, "invalid-without-compat-ua".to_string());
        }
        (state.status, state.payload.to_string())
    }

    async fn spawn_subscription_server(
        payload: &'static str,
        status: StatusCode,
        accepted_user_agent: Option<&'static str>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                payload: Arc::<str>::from(payload),
                status,
                accepted_user_agent: accepted_user_agent.map(Arc::<str>::from),
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
    async fn load_subscription_returns_success_with_warning_when_post_load_bookkeeping_fails() {
        let profile_id = "p-load-bookkeeping-warning";
        let store = Arc::new(FailProfileSyncConfigStore::default());
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
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
            .list_subscription(profile_id)
            .await
            .expect("subscription query should succeed");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].proxy_name, "new");

        let sync_config = store
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed");
        assert!(sync_config.is_none());

        let task_runs = store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
                ..TaskListQuery::default()
            })
            .await
            .expect("task list should succeed");
        assert!(task_runs.is_empty());
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
    async fn load_subscription_from_url_accepts_ua_gated_payload() {
        let profile_id = "p-url-success";
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
        )
        .await;

        let result = service
            .load_subscription(profile_id, &SubscriptionSource::Url(url))
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
        let profile_id = "p-url-invalid";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let (url, server) =
            spawn_subscription_server("still-not-a-subscription", StatusCode::OK, None).await;

        let result = service
            .load_subscription(profile_id, &SubscriptionSource::Url(url))
            .await;

        server.abort();

        assert!(matches!(result, Err(BrokerError::SubscriptionInvalid)));
    }

    #[tokio::test]
    async fn load_subscription_from_url_maps_non_2xx_to_subscription_fetch_failed() {
        let profile_id = "p-url-fetch";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store, runtime, BrokerServiceOptions::default());
        let (url, server) = spawn_subscription_server("blocked", StatusCode::FORBIDDEN, None).await;

        let result = service
            .load_subscription(profile_id, &SubscriptionSource::Url(url))
            .await;

        server.abort();

        assert!(
            matches!(result, Err(BrokerError::SubscriptionFetch(message)) if message.contains("returned non-2xx"))
        );
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
                        desired_port: Some(0),
                        ..Default::default()
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
            selection_mode: SessionSelectionMode::Ip,
            specified_ips: vec!["9.9.9.9".to_string()],
            desired_port: None,
            ..Default::default()
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
        let chosen = choose_ip_for_open(&request, &ips, &[]).expect("should pick with mru");
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

        let chosen =
            choose_ip_for_open(&request, &ips, &probes).expect("healthy candidate should win");
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

        let chosen = choose_ip_for_open(&request, &ips, &probes)
            .expect("lower latency candidate should win before recency");
        assert_eq!(chosen, "2.2.2.2");
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
    fn choose_ip_rejects_ip_mode_without_specified_ips() {
        let request = OpenSessionRequest {
            selection_mode: SessionSelectionMode::Ip,
            desired_port: None,
            ..Default::default()
        };
        let ips = vec![sample_ip("1.1.1.1", Some(100))];
        let err = choose_ip_for_open(&request, &ips, &[])
            .expect_err("ip mode without specified_ips should be rejected");
        assert!(matches!(err, BrokerError::InvalidRequest(_)));
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
            selection_mode: SessionSelectionMode::Ip,
            specified_ips: vec!["1.1.1.1".to_string()],
            desired_port: None,
            ..Default::default()
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

    #[tokio::test]
    async fn load_subscription_registers_sync_config_and_queues_post_load_task() {
        let profile_id = "p-tasks";
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        assert_eq!(response.loaded_proxies, 1);

        let config = store
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should be persisted");
        assert!(matches!(config.source, SubscriptionSource::File(path) if path == source_path));
        assert!(config.enabled);

        let tasks = service
            .list_tasks(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        let profile_id = "p-tasks-with-full-refresh";
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
                profile_id,
                TaskRunKind::MetadataRefreshFull,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("full refresh queue should succeed");

        service
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let tasks = service
            .list_tasks(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        let profile_id = "p-tasks-preserve-due-at";
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
        let now = now_epoch_sec();
        let expected_sync_due_at = now + 123;
        let expected_full_due_at = now + 456;

        store
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: profile_id.to_string(),
                source: SubscriptionSource::Url("https://example.com/subscription".to_string()),
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert_eq!(config.last_sync_due_at, Some(expected_sync_due_at));
        assert_eq!(config.last_full_refresh_due_at, Some(expected_full_due_at));
    }

    #[tokio::test]
    async fn load_subscription_advances_overdue_sync_due_without_moving_full_refresh_due_at() {
        let profile_id = "p-tasks-preserve-overdue-full-refresh-due-at";
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
        let now = now_epoch_sec();

        store
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: profile_id.to_string(),
                source: SubscriptionSource::Url("https://example.com/subscription".to_string()),
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let config = store
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should persist");
        assert!(config.last_sync_due_at.expect("sync due at") > now);
        assert_eq!(config.last_full_refresh_due_at, Some(now - 10));
    }

    #[tokio::test]
    async fn load_subscription_creates_post_load_task_even_when_no_new_ips_arrive() {
        let profile_id = "p-tasks-no-new-ips";
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("first load should succeed");
        let mut existing_runs = store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&source_path).await;

        let tasks = service
            .list_tasks(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        let profile_id = "p-tasks-full-refresh-retry";
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
            .await
            .expect("load should succeed");

        let mut config = store
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should exist");
        config.last_full_refresh_due_at = Some(now - 1);
        store
            .upsert_profile_sync_config(&config)
            .await
            .expect("sync config update should succeed");

        let mut run = service
            .enqueue_task_run(
                profile_id,
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
            .get_profile_sync_config(profile_id)
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
        let profile_id = "p-tasks-sync-retry";
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
            .load_subscription(profile_id, &SubscriptionSource::File(source_path.clone()))
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
            .get_profile_sync_config(profile_id)
            .await
            .expect("sync config query should succeed")
            .expect("sync config should exist");
        config.last_sync_due_at = Some(now - 1);
        store
            .upsert_profile_sync_config(&config)
            .await
            .expect("sync config update should succeed");

        let mut run = service
            .enqueue_task_run(
                profile_id,
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
            .get_profile_sync_config(profile_id)
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
    async fn load_subscription_coalesces_post_load_task_scope() {
        let profile_id = "p-tasks-coalesce";
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
            .load_subscription(profile_id, &SubscriptionSource::File(first_source.clone()))
            .await
            .expect("first load should succeed");
        service
            .load_subscription(profile_id, &SubscriptionSource::File(second_source.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&first_source).await;
        let _ = tokio::fs::remove_file(&second_source).await;

        let tasks = store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        }
    }

    #[tokio::test]
    async fn load_subscription_queues_follow_up_post_load_task_when_incremental_is_running() {
        let profile_id = "p-tasks-follow-up-while-running";
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
            .load_subscription(profile_id, &SubscriptionSource::File(first_source.clone()))
            .await
            .expect("first load should succeed");

        let mut run = store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
            .load_subscription(profile_id, &SubscriptionSource::File(second_source.clone()))
            .await
            .expect("second load should succeed");

        let _ = tokio::fs::remove_file(&first_source).await;
        let _ = tokio::fs::remove_file(&second_source).await;

        let tasks = store
            .list_task_runs(&TaskListQuery {
                profile_id: Some(profile_id.to_string()),
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
        }
    }

    #[tokio::test]
    async fn enqueue_due_tasks_queues_sync_then_full_refresh_for_due_profile() {
        let profile_id = "p-schedule";
        let store = Arc::new(MemoryStore::new());
        let runtime = Arc::new(TestRuntime::default());
        let service = BrokerService::new(store.clone(), runtime, BrokerServiceOptions::default());
        let now = now_epoch_sec();

        store
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: profile_id.to_string(),
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
                profile_id: Some(profile_id.to_string()),
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

    #[test]
    fn dispatch_sort_keeps_due_sync_before_full_refresh_for_same_profile() {
        let mut runs = vec![
            TaskRunRecord {
                run_id: "zzz".to_string(),
                profile_id: "profile-a".to_string(),
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
                profile_id: "profile-a".to_string(),
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
        let profile_id = "p-sync-deferred";
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
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: profile_id.to_string(),
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
                profile_id,
                TaskRunKind::MetadataRefreshFull,
                TaskRunTrigger::Schedule,
                TaskRunScope::All,
            )
            .await
            .expect("full refresh queue should succeed");
        let mut sync_run = service
            .enqueue_task_run(
                profile_id,
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
            .list_probe_records(profile_id)
            .await
            .expect("probe record query should succeed");
        assert!(probe_records.is_empty());
    }
}
