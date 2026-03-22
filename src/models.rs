use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SubscriptionSource {
    Url(String),
    File(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSubscriptionRequest {
    pub source: SubscriptionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSubscriptionResponse {
    pub loaded_proxies: usize,
    pub distinct_ips: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileResponse {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub probed_ips: usize,
    pub geo_updated: usize,
    pub skipped_cached: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortMode {
    Mru,
    #[default]
    Lru,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractIpRequest {
    #[serde(default)]
    pub country_codes: Vec<String>,
    #[serde(default)]
    pub cities: Vec<String>,
    #[serde(default)]
    pub specified_ips: Vec<String>,
    #[serde(default)]
    pub blacklist_ips: Vec<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub sort_mode: SortMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub specified_ip: Option<String>,
    pub selector: Option<ExtractIpRequest>,
    pub desired_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchRequest {
    pub requests: Vec<OpenSessionRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub session_id: String,
    pub listen: String,
    pub port: u16,
    pub selected_ip: String,
    pub proxy_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractIpItem {
    pub ip: String,
    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub region_name: Option<String>,
    pub city: Option<String>,
    pub probe_ok: bool,
    pub best_latency_ms: Option<u64>,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractIpResponse {
    pub items: Vec<ExtractIpItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProfilesResponse {
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchResponse {
    pub sessions: Vec<OpenSessionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthPrincipalType {
    Human,
    ApiKey,
    Development,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMeResponse {
    pub authenticated: bool,
    pub principal_type: AuthPrincipalType,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    pub is_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub key_id: String,
    pub profile_id: String,
    pub name: String,
    pub prefix: String,
    pub created_by: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListApiKeysResponse {
    pub api_keys: Vec<ApiKeySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKeySummary,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunKind {
    SubscriptionSync,
    MetadataRefreshIncremental,
    MetadataRefreshFull,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunTrigger {
    Schedule,
    PostLoad,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStage {
    Queued,
    LoadingSubscription,
    DiffingInventory,
    Probing,
    GeoEnrichment,
    Persisting,
    Completed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSyncConfig {
    pub profile_id: String,
    pub source: SubscriptionSource,
    pub enabled: bool,
    pub sync_every_sec: u64,
    pub full_refresh_every_sec: u64,
    pub last_sync_due_at: Option<i64>,
    pub last_sync_started_at: Option<i64>,
    pub last_sync_finished_at: Option<i64>,
    pub last_full_refresh_due_at: Option<i64>,
    pub last_full_refresh_started_at: Option<i64>,
    pub last_full_refresh_finished_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskRunScope {
    #[default]
    All,
    Ips {
        ips: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunRecord {
    pub run_id: String,
    pub profile_id: String,
    pub kind: TaskRunKind,
    pub trigger: TaskRunTrigger,
    pub status: TaskRunStatus,
    pub stage: TaskRunStage,
    pub progress_current: Option<u64>,
    pub progress_total: Option<u64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub summary_json: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub scope: TaskRunScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunEventRecord {
    pub event_id: String,
    pub run_id: String,
    pub profile_id: String,
    pub at: i64,
    pub level: TaskEventLevel,
    pub stage: TaskRunStage,
    pub message: String,
    pub payload_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunSummary {
    pub run_id: String,
    pub profile_id: String,
    pub kind: TaskRunKind,
    pub trigger: TaskRunTrigger,
    pub status: TaskRunStatus,
    pub stage: TaskRunStage,
    pub progress_current: Option<u64>,
    pub progress_total: Option<u64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub summary_json: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl TaskRunRecord {
    pub fn as_summary(&self) -> TaskRunSummary {
        TaskRunSummary {
            run_id: self.run_id.clone(),
            profile_id: self.profile_id.clone(),
            kind: self.kind,
            trigger: self.trigger,
            status: self.status,
            stage: self.stage,
            progress_current: self.progress_current,
            progress_total: self.progress_total,
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            summary_json: self.summary_json.clone(),
            error_code: self.error_code.clone(),
            error_message: self.error_message.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunEvent {
    pub event_id: String,
    pub run_id: String,
    pub at: i64,
    pub level: TaskEventLevel,
    pub stage: TaskRunStage,
    pub message: String,
    pub payload_json: Option<serde_json::Value>,
}

impl TaskRunEventRecord {
    pub fn as_public(&self) -> TaskRunEvent {
        TaskRunEvent {
            event_id: self.event_id.clone(),
            run_id: self.run_id.clone(),
            at: self.at,
            level: self.level,
            stage: self.stage,
            message: self.message.clone(),
            payload_json: self.payload_json.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub total_runs: usize,
    pub queued_runs: usize,
    pub running_runs: usize,
    pub failed_runs: usize,
    pub succeeded_runs: usize,
    pub skipped_runs: usize,
    pub last_run_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskListQuery {
    pub profile_id: Option<String>,
    pub kind: Option<TaskRunKind>,
    pub status: Option<TaskRunStatus>,
    pub trigger: Option<TaskRunTrigger>,
    #[serde(default)]
    pub running_only: bool,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResponse {
    pub summary: TaskSummary,
    pub runs: Vec<TaskRunSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunDetail {
    pub run: TaskRunSummary,
    pub events: Vec<TaskRunEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStreamEnvelope {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub proxy_name: String,
    pub proxy_type: String,
    pub server: String,
    pub resolved_ips: Vec<String>,
    pub raw_proxy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRecord {
    pub ip: String,
    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub region_name: Option<String>,
    pub city: Option<String>,
    pub geo_source: Option<String>,
    pub probe_updated_at: Option<i64>,
    pub geo_updated_at: Option<i64>,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeRecord {
    pub proxy_name: String,
    pub ip: String,
    pub target_url: String,
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub listen: String,
    pub port: u16,
    pub selected_ip: String,
    pub proxy_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub key_id: String,
    pub profile_id: String,
    pub name: String,
    pub secret_prefix: String,
    pub secret_salt: String,
    pub secret_hash: String,
    pub created_by_subject: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

impl ApiKeyRecord {
    pub fn as_summary(&self) -> ApiKeySummary {
        ApiKeySummary {
            key_id: self.key_id.clone(),
            profile_id: self.profile_id.clone(),
            name: self.name.clone(),
            prefix: self.secret_prefix.clone(),
            created_by: self.created_by_subject.clone(),
            created_at: self.created_at,
            last_used_at: self.last_used_at,
            revoked_at: self.revoked_at,
        }
    }
}

impl SubscriptionSource {
    pub fn parts(&self) -> (&'static str, &str) {
        match self {
            Self::Url(value) => ("url", value.as_str()),
            Self::File(value) => ("file", value.as_str()),
        }
    }

    pub fn from_parts(source_type: &str, source_value: String) -> Option<Self> {
        match source_type {
            "url" => Some(Self::Url(source_value)),
            "file" => Some(Self::File(source_value)),
            _ => None,
        }
    }
}

macro_rules! impl_task_enum_codec {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }

            pub fn parse(raw: &str) -> Option<Self> {
                match raw {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

impl_task_enum_codec!(TaskRunKind {
    SubscriptionSync => "subscription_sync",
    MetadataRefreshIncremental => "metadata_refresh_incremental",
    MetadataRefreshFull => "metadata_refresh_full",
});

impl_task_enum_codec!(TaskRunTrigger {
    Schedule => "schedule",
    PostLoad => "post_load",
});

impl_task_enum_codec!(TaskRunStatus {
    Queued => "queued",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
    Skipped => "skipped",
});

impl_task_enum_codec!(TaskRunStage {
    Queued => "queued",
    LoadingSubscription => "loading_subscription",
    DiffingInventory => "diffing_inventory",
    Probing => "probing",
    GeoEnrichment => "geo_enrichment",
    Persisting => "persisting",
    Completed => "completed",
});

impl_task_enum_codec!(TaskEventLevel {
    Info => "info",
    Warning => "warning",
    Error => "error",
});

#[derive(Debug, Clone, Default)]
pub struct ProfileSnapshot {
    pub nodes: Vec<ProxyNode>,
    pub ip_records: HashMap<String, IpRecord>,
    pub probe_records: Vec<ProbeRecord>,
    pub sessions: HashMap<String, SessionRecord>,
    pub api_keys: HashMap<String, ApiKeyRecord>,
    pub sync_config: Option<ProfileSyncConfig>,
    pub task_runs: HashMap<String, TaskRunRecord>,
    pub task_run_events: HashMap<String, Vec<TaskRunEventRecord>>,
}

pub fn now_epoch_sec() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}
