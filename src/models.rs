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
pub struct ListProfilesResponse {
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummaryResponse {
    pub profile_id: String,
    pub initialized: bool,
    pub proxy_count: usize,
    pub distinct_ip_count: usize,
    pub session_count: usize,
    pub probe_ip_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBatchResponse {
    pub sessions: Vec<OpenSessionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
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

#[derive(Debug, Clone, Default)]
pub struct ProfileSnapshot {
    pub nodes: Vec<ProxyNode>,
    pub ip_records: HashMap<String, IpRecord>,
    pub probe_records: Vec<ProbeRecord>,
    pub sessions: HashMap<String, SessionRecord>,
}

pub fn now_epoch_sec() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}
