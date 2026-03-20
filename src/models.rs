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

#[derive(Debug, Clone, Default)]
pub struct ProfileSnapshot {
    pub nodes: Vec<ProxyNode>,
    pub ip_records: HashMap<String, IpRecord>,
    pub probe_records: Vec<ProbeRecord>,
    pub sessions: HashMap<String, SessionRecord>,
    pub api_keys: HashMap<String, ApiKeyRecord>,
}

pub fn now_epoch_sec() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}
