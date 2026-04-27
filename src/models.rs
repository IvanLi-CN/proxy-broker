use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SubscriptionSource {
    Url(String),
    File(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSubscriptionRequest {
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub name: Option<String>,
    #[serde(default)]
    pub source: Option<SubscriptionSource>,
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSubscriptionResponse {
    pub loaded_proxies: usize,
    pub distinct_ips: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_name_source: Option<ResolvedImportNameSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_metadata: Option<SubscriptionMetadata>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionSelectionMode {
    #[default]
    Any,
    Geo,
    Ip,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionOptionKind {
    Country,
    City,
    Ip,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenSessionRequest {
    #[serde(default)]
    pub selection_mode: SessionSelectionMode,
    #[serde(default)]
    pub country_codes: Vec<String>,
    #[serde(default)]
    pub cities: Vec<String>,
    #[serde(default)]
    pub specified_ips: Vec<String>,
    #[serde(default)]
    pub excluded_ips: Vec<String>,
    #[serde(default)]
    pub sort_mode: SortMode,
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
    pub bind_host: String,
    pub display_host: String,
    pub display_address: String,
    pub port: u16,
    pub selected_ip: String,
    pub proxy_name: String,
    pub node_id: String,
    pub candidate_node_ids: Vec<String>,
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
    pub sessions: Vec<SessionListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProfilesResponse {
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProxyScope {
    Global,
    Profile { profile_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProxyInventoryResponse {
    pub items: Vec<ProxyInventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProxyImportResponse {
    pub items: Vec<ProxyImportItem>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProxyImportKind {
    Subscription,
    SingleNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyImportSourceIdentity {
    pub source_type: String,
    pub source_value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedImportNameSource {
    ExplicitInput,
    ExistingImport,
    ParsedSource,
    Generated,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyImportItem {
    pub import_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub import_kind: ProxyImportKind,
    pub source_scope: ProxyScope,
    pub source_identity: ProxyImportSourceIdentity,
    pub allocation_scope: ProxyScope,
    pub proxy_count: usize,
    pub distinct_ip_count: usize,
    pub effective_profile_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_metadata: Option<SubscriptionMetadata>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInventoryItem {
    pub import_id: String,
    pub node_id: String,
    pub proxy_name: String,
    pub proxy_type: String,
    pub server: String,
    pub resolved_ips: Vec<String>,
    pub source_scope: ProxyScope,
    pub allocation_scope: ProxyScope,
    pub effective_profile_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCatalogNodeItem {
    pub import_id: String,
    pub node_id: String,
    pub proxy_name: String,
    pub proxy_type: String,
    pub server: String,
    pub resolved_ips: Vec<String>,
    pub source_scope: ProxyScope,
    pub allocation_scope: ProxyScope,
    pub effective_profile_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_ip: Option<String>,
    pub ip_metadata: Vec<ProxyNodeMetadataRecord>,
    pub can_open_session: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCatalogGroupItem {
    pub import: ProxyImportItem,
    pub nodes: Vec<ProxyCatalogNodeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCatalogResponse {
    pub view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub groups: Vec<ProxyCatalogGroupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProxyAllocationRequest {
    pub allocation_scope: ProxyScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProxyImportAllocationRequest {
    pub allocation_scope: ProxyScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyOperationRequest {
    pub view: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyOperationAcceptedResponse {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProxySettings {
    pub profile_id: String,
    pub use_global_proxies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileProxySettingsRequest {
    pub use_global_proxies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchResponse {
    pub sessions: Vec<OpenSessionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionByNodeRequest {
    pub node_id: String,
    pub desired_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSessionNodeRequest {
    #[serde(default)]
    pub node_id: String,
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub selected_ip: Option<String>,
    #[serde(default)]
    pub candidate_node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionByIpRequest {
    pub selected_ip: String,
    #[serde(default)]
    pub candidate_node_ids: Vec<String>,
    pub desired_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchByIpRequest {
    pub requests: Vec<OpenSessionByIpRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchByNodeRequest {
    #[serde(default)]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub requests: Vec<OpenSessionByNodeRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedPortResponse {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSessionOptionsRequest {
    pub kind: SessionOptionKind,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub country_codes: Vec<String>,
    #[serde(default)]
    pub cities: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOptionItem {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSessionOptionsResponse {
    pub items: Vec<SessionOptionItem>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionNodeSortMode {
    #[default]
    SessionRecent,
    ProfileRecent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchSessionNodeOptionsRequest {
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub query: Option<String>,
    #[serde(default)]
    pub sort_mode: SessionNodeSortMode,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNodeOptionItem {
    pub node_id: String,
    pub proxy_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_last_used_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSessionNodeOptionsResponse {
    pub items: Vec<SessionNodeOptionItem>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionIpNodeGroupBy {
    #[default]
    Subscription,
    City,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchSessionIpNodeOptionsRequest {
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub query: Option<String>,
    #[serde(default)]
    pub group_by: SessionIpNodeGroupBy,
    #[serde(default, deserialize_with = "deserialize_optional_trimmed_string")]
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIpNodeOptionNodeItem {
    pub node_id: String,
    pub proxy_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_last_used_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIpNodeOptionIpItem {
    pub ip: String,
    pub group_key: String,
    pub group_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_latency_ms: Option<u64>,
    pub nodes: Vec<SessionIpNodeOptionNodeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIpNodeOptionGroupItem {
    pub key: String,
    pub label: String,
    pub items: Vec<SessionIpNodeOptionIpItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSessionIpNodeOptionsResponse {
    pub groups: Vec<SessionIpNodeOptionGroupItem>,
}

#[cfg(test)]
mod tests {
    use super::OpenSessionRequest;

    #[test]
    fn open_session_request_rejects_legacy_single_payload_fields() {
        let payload = serde_json::json!({
            "specified_ip": "203.0.113.10",
            "selector": {
                "country_codes": ["JP"],
                "limit": 1,
                "sort_mode": "lru"
            }
        });

        let err = serde_json::from_value::<OpenSessionRequest>(payload)
            .expect_err("legacy single-open payload should be rejected");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn open_session_request_rejects_legacy_batch_row_fields() {
        let payload = serde_json::json!({
            "specified_ip": "203.0.113.10",
            "selector": {
                "specified_ips": ["203.0.113.10"],
                "limit": 1
            },
            "desired_port": 10080
        });

        let err = serde_json::from_value::<OpenSessionRequest>(payload)
            .expect_err("legacy batch row payload should be rejected");
        assert!(err.to_string().contains("unknown field"));
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyProfileScopeKind {
    SelectedProfiles,
    AllProfiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyProfileScope {
    pub kind: ApiKeyProfileScopeKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile_ids: Vec<String>,
}

impl ApiKeyProfileScope {
    pub fn selected<I>(profile_ids: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        Self {
            kind: ApiKeyProfileScopeKind::SelectedProfiles,
            profile_ids: profile_ids.into_iter().collect(),
        }
    }

    pub fn all_profiles() -> Self {
        Self {
            kind: ApiKeyProfileScopeKind::AllProfiles,
            profile_ids: Vec::new(),
        }
    }

    pub fn single_profile_id(&self) -> Option<String> {
        if self.kind == ApiKeyProfileScopeKind::SelectedProfiles && self.profile_ids.len() == 1 {
            return self.profile_ids.first().cloned();
        }
        None
    }

    pub fn allows_profile(&self, profile_id: &str) -> bool {
        match self.kind {
            ApiKeyProfileScopeKind::AllProfiles => true,
            ApiKeyProfileScopeKind::SelectedProfiles => {
                self.profile_ids.iter().any(|item| item == profile_id)
            }
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_owner_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_profile_scope: Option<ApiKeyProfileScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub profile_scope: ApiKeyProfileScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub key_id: String,
    pub name: String,
    pub prefix: String,
    pub created_by: String,
    pub owner_subject: String,
    pub profile_scope: ApiKeyProfileScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
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
    ProxyMetadataRefresh,
    ProxyLatencyProbe,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunTrigger {
    Schedule,
    PostLoad,
    Operator,
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
pub struct ProxyImportSyncConfig {
    pub import_id: String,
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

pub type ProfileSyncConfig = ProxyImportSyncConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskRunScope {
    #[default]
    All,
    Ips {
        ips: Vec<String>,
    },
    Nodes {
        node_ids: Vec<String>,
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
    pub since: Option<i64>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyInventoryListQuery {
    pub scope: Option<String>,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyImportListQuery {
    pub scope: Option<String>,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyCatalogQuery {
    pub view: Option<String>,
    pub profile_id: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub proxy_name: String,
    pub proxy_type: String,
    pub server: String,
    pub resolved_ips: Vec<String>,
    pub raw_proxy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInventoryRecord {
    pub import_id: String,
    pub node_id: String,
    pub source_scope: ProxyScope,
    pub allocation_scope: ProxyScope,
    pub proxy_name: String,
    pub proxy_type: String,
    pub server: String,
    pub resolved_ips: Vec<String>,
    pub raw_proxy: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
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
    pub node_id: String,
    #[serde(default)]
    pub candidate_node_ids: Vec<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListItem {
    pub session_id: String,
    pub listen: String,
    pub bind_host: String,
    pub display_host: String,
    pub display_address: String,
    pub port: u16,
    pub selected_ip: String,
    pub proxy_name: String,
    pub node_id: String,
    pub candidate_node_ids: Vec<String>,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUsageRecord {
    pub node_id: String,
    pub last_used_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNodeMetadataRecord {
    pub node_id: String,
    pub ip: String,
    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub region_name: Option<String>,
    pub city: Option<String>,
    pub geo_source: Option<String>,
    pub probe_updated_at: Option<i64>,
    pub geo_updated_at: Option<i64>,
    pub last_probe_ok: Option<bool>,
    pub last_latency_ms: Option<u64>,
    pub median_latency_ms: Option<u64>,
    #[serde(default)]
    pub last_probe_samples: Vec<Option<u64>>,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ProxyImportRecord {
    pub import_id: String,
    pub name: Option<String>,
    pub import_kind: ProxyImportKind,
    pub source_scope: ProxyScope,
    pub source_identity: ProxyImportSourceIdentity,
    pub allocation_scope: ProxyScope,
    pub subscription_metadata: Option<SubscriptionMetadata>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub key_id: String,
    pub name: String,
    pub secret_prefix: String,
    pub secret_salt: String,
    pub secret_hash: String,
    pub created_by_subject: String,
    pub profile_scope: ApiKeyProfileScope,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

impl ApiKeyRecord {
    pub fn as_summary(&self) -> ApiKeySummary {
        ApiKeySummary {
            key_id: self.key_id.clone(),
            name: self.name.clone(),
            prefix: self.secret_prefix.clone(),
            created_by: self.created_by_subject.clone(),
            owner_subject: self.created_by_subject.clone(),
            profile_scope: self.profile_scope.clone(),
            profile_id: self.profile_scope.single_profile_id(),
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

impl ProxyImportSourceIdentity {
    pub fn from_source(source: &SubscriptionSource) -> Self {
        let (source_type, source_value) = source.parts();
        Self {
            source_type: source_type.to_string(),
            source_value: source_value.trim().to_string(),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.source_type, self.source_value)
    }

    pub fn manual(import_id: impl Into<String>) -> Self {
        let import_id = import_id.into();
        Self {
            source_type: "manual".to_string(),
            source_value: import_id,
        }
    }
}

impl SubscriptionMetadata {
    pub fn is_empty(&self) -> bool {
        self.source_title.is_none()
            && self.upload_bytes.is_none()
            && self.download_bytes.is_none()
            && self.used_bytes.is_none()
            && self.total_bytes.is_none()
            && self.remaining_bytes.is_none()
            && self.expire_at.is_none()
    }

    pub fn normalized(mut self) -> Option<Self> {
        self.source_title = self
            .source_title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        if self.is_empty() { None } else { Some(self) }
    }
}

fn deserialize_optional_trimmed_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

impl ProxyScope {
    pub fn global() -> Self {
        Self::Global
    }

    pub fn profile(profile_id: impl Into<String>) -> Self {
        Self::Profile {
            profile_id: profile_id.into(),
        }
    }

    pub fn profile_id(&self) -> Option<&str> {
        match self {
            Self::Global => None,
            Self::Profile { profile_id } => Some(profile_id.as_str()),
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Global => "global".to_string(),
            Self::Profile { profile_id } => format!("profile:{profile_id}"),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Profile { .. } => "profile",
        }
    }

    pub fn from_parts(scope_type: &str, profile_id: Option<String>) -> Option<Self> {
        match scope_type {
            "global" => Some(Self::Global),
            "profile" => Some(Self::profile(profile_id?)),
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
    ProxyMetadataRefresh => "proxy_metadata_refresh",
    ProxyLatencyProbe => "proxy_latency_probe",
});

impl_task_enum_codec!(TaskRunTrigger {
    Schedule => "schedule",
    PostLoad => "post_load",
    Operator => "operator",
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

impl_task_enum_codec!(ApiKeyProfileScopeKind {
    SelectedProfiles => "selected_profiles",
    AllProfiles => "all_profiles",
});

#[derive(Debug, Clone, Default)]
pub struct ProfileSnapshot {
    pub nodes: Vec<ProxyNode>,
    pub ip_records: HashMap<String, IpRecord>,
    pub probe_records: Vec<ProbeRecord>,
    pub sessions: HashMap<String, SessionRecord>,
    pub profile_node_usages: HashMap<String, i64>,
    pub session_node_usages: HashMap<String, HashMap<String, i64>>,
    pub api_keys: HashMap<String, ApiKeyRecord>,
    pub proxy_imports: HashMap<String, ProxyImportRecord>,
    pub sync_configs: HashMap<String, ProxyImportSyncConfig>,
    pub task_runs: HashMap<String, TaskRunRecord>,
    pub task_run_events: HashMap<String, Vec<TaskRunEventRecord>>,
}

pub fn now_epoch_sec() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}
