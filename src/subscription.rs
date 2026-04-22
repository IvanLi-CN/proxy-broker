use std::{collections::HashSet, net::IpAddr, path::Path, sync::Arc};

use anyhow::{Context, anyhow};
use base64::Engine;
use reqwest::header::{CONTENT_DISPOSITION, HeaderMap, USER_AGENT};
use serde_yaml::Value;
use thiserror::Error;

use crate::{
    constants::DEFAULT_DNS_CONCURRENCY,
    models::{ProxyNode, SubscriptionMetadata},
};

pub const SUBSCRIPTION_FETCH_USER_AGENTS: &[&str] =
    &["Clash.Meta/1.18.3", "mihomo/1.18.3", "Clash Verge/1.7.7"];
const INFO_PROXY_KEYWORDS_EN: &[&str] = &["traffic", "expire", "expired", "notice"];
const INFO_PROXY_KEYWORDS_EN_WEAK: &[&str] = &["subscription", "official", "support"];
const INFO_PROXY_KEYWORDS_ZH: &[&str] = &["流量", "剩余", "过期", "到期", "公告", "说明"];
const INFO_PROXY_KEYWORDS_ZH_WEAK: &[&str] = &["官网", "订阅", "客服"];

#[derive(Debug, Clone, Default)]
pub struct LoadedSubscription {
    pub nodes: Vec<ProxyNode>,
    pub warnings: Vec<String>,
    pub metadata: Option<SubscriptionMetadata>,
    pub parsed_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ParsedResponseMetadata {
    metadata: Option<SubscriptionMetadata>,
    parsed_name: Option<String>,
    warnings: Vec<String>,
    _profile_update_interval_sec: Option<u64>,
}

fn decode_base64_yaml(input: &str) -> anyhow::Result<String> {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&compact)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&compact))
        .context("base64 decode failed")?;
    String::from_utf8(bytes).context("base64 payload is not utf-8")
}

fn extract_proxies_from_yaml(content: &str) -> anyhow::Result<Vec<Value>> {
    let root: Value = serde_yaml::from_str(content).context("yaml parse failed")?;

    if let Some(proxies) = root.get("proxies").and_then(Value::as_sequence) {
        return Ok(proxies.to_vec());
    }

    if let Some(seq) = root.as_sequence() {
        return Ok(seq.to_vec());
    }

    Err(anyhow!("subscription yaml does not contain `proxies`"))
}

#[derive(Debug, Error)]
pub enum SubscriptionLoadError {
    #[error("subscription source read failed: {0}")]
    SourceRead(String),
    #[error("subscription payload invalid: {0}")]
    InvalidPayload(String),
}

fn decode_base64_text(input: &str) -> anyhow::Result<String> {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&compact)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&compact))
        .context("base64 decode failed")?;
    String::from_utf8(bytes).context("base64 payload is not utf-8")
}

fn to_json_value(value: &Value) -> anyhow::Result<serde_json::Value> {
    let s = serde_yaml::to_string(value).context("failed to serialize yaml node")?;
    let json = serde_yaml::from_str::<serde_json::Value>(&s)
        .context("failed to convert yaml node to json")?;
    Ok(json)
}

fn extract_proxy_fields(proxy: &serde_json::Value) -> anyhow::Result<(String, String, String)> {
    let name = proxy
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("proxy missing `name`"))?
        .to_string();
    let proxy_type = proxy
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("proxy missing `type`"))?
        .to_string();
    let server = proxy
        .get("server")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("proxy missing `server`"))?
        .to_string();
    Ok((name, proxy_type, server))
}

async fn resolve_server_ips(server: &str) -> anyhow::Result<Vec<String>> {
    if let Ok(ip) = server.parse::<IpAddr>() {
        return Ok(vec![ip.to_string()]);
    }

    let resolved = tokio::net::lookup_host((server, 443)).await;
    match resolved {
        Ok(iter) => {
            let mut uniq = HashSet::new();
            for addr in iter {
                uniq.insert(addr.ip().to_string());
            }
            let mut ips: Vec<String> = uniq.into_iter().collect();
            ips.sort();
            Ok(ips)
        }
        Err(err) => Err(anyhow!("dns lookup failed for {server}: {err}")),
    }
}

fn parse_subscription_payload(raw: &str) -> Result<Vec<Value>, SubscriptionLoadError> {
    match extract_proxies_from_yaml(raw) {
        Ok(proxies) => Ok(proxies),
        Err(yaml_err) => {
            let decoded = decode_base64_yaml(raw).map_err(|base64_err| {
                SubscriptionLoadError::InvalidPayload(format!(
                    "yaml parse failed: {yaml_err}; base64 fallback failed: {base64_err}"
                ))
            })?;
            extract_proxies_from_yaml(&decoded)
                .map_err(|err| SubscriptionLoadError::InvalidPayload(err.to_string()))
        }
    }
}

fn payload_has_usable_proxy_entries(proxies: &[Value]) -> bool {
    proxies.iter().any(|proxy| {
        to_json_value(proxy)
            .and_then(|json| extract_proxy_fields(&json))
            .is_ok()
    })
}

fn find_header_value(
    headers: &HeaderMap,
    header_name: &str,
    allow_meta_prefix: bool,
) -> Option<String> {
    let exact = headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if exact.is_some() {
        return exact;
    }
    if !allow_meta_prefix {
        return None;
    }

    headers.iter().find_map(|(name, value)| {
        let name = name.as_str().to_ascii_lowercase();
        if !name.starts_with("x-") || !name.ends_with(&format!("meta-{header_name}")) {
            return None;
        }
        value
            .to_str()
            .ok()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn percent_decode_lossy(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok()?;
                let byte = u8::from_str_radix(hex, 16).ok()?;
                decoded.push(byte);
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn clean_source_title(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn decode_profile_title(value: &str) -> anyhow::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let decoded = if let Some(encoded) = trimmed.strip_prefix("base64:") {
        decode_base64_text(encoded)?
    } else {
        trimmed.to_string()
    };
    Ok(clean_source_title(&decoded))
}

fn parse_rfc5987_filename(value: &str) -> Option<String> {
    let raw = value.trim().trim_matches('"');
    let mut parts = raw.splitn(3, '\'');
    let _charset = parts.next()?;
    let _language = parts.next()?;
    let encoded = parts.next()?;
    percent_decode_lossy(encoded).and_then(|decoded| clean_source_title(&decoded))
}

fn parse_content_disposition_filename(value: &str) -> Option<String> {
    let mut filename = None;
    let mut filename_star = None;
    for segment in value.split(';').skip(1) {
        let (key, raw_value) = match segment.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim().to_ascii_lowercase();
        let raw_value = raw_value.trim();
        match key.as_str() {
            "filename*" => {
                filename_star = parse_rfc5987_filename(raw_value);
            }
            "filename" => {
                filename = clean_source_title(raw_value);
            }
            _ => {}
        }
    }
    filename_star.or(filename)
}

fn content_disposition_mentions_filename(value: &str) -> bool {
    value.split(';').skip(1).any(|segment| {
        segment
            .split_once('=')
            .map(|(key, _)| {
                matches!(
                    key.trim().to_ascii_lowercase().as_str(),
                    "filename" | "filename*"
                )
            })
            .unwrap_or(false)
    })
}

fn parse_subscription_userinfo(value: &str) -> (Option<SubscriptionMetadata>, Option<String>) {
    let mut upload_bytes = None;
    let mut download_bytes = None;
    let mut total_bytes = None;
    let mut expire_at = None;
    let mut saw_known_field = false;
    let mut saw_invalid_known_field = false;

    for part in value.split([';', ',']) {
        let (key, raw_value) = match part.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim().to_ascii_lowercase();
        let raw_value = raw_value.trim();
        match key.as_str() {
            "upload" => {
                saw_known_field = true;
                match raw_value.parse::<u64>() {
                    Ok(value) => upload_bytes = Some(value),
                    Err(_) => saw_invalid_known_field = true,
                }
            }
            "download" => {
                saw_known_field = true;
                match raw_value.parse::<u64>() {
                    Ok(value) => download_bytes = Some(value),
                    Err(_) => saw_invalid_known_field = true,
                }
            }
            "total" => {
                saw_known_field = true;
                match raw_value.parse::<u64>() {
                    Ok(value) => total_bytes = Some(value),
                    Err(_) => saw_invalid_known_field = true,
                }
            }
            "expire" => {
                saw_known_field = true;
                match raw_value.parse::<i64>() {
                    Ok(value) => expire_at = Some(value),
                    Err(_) => saw_invalid_known_field = true,
                }
            }
            _ => {}
        }
    }

    let used_bytes = if upload_bytes.is_some() || download_bytes.is_some() {
        Some(
            upload_bytes
                .unwrap_or(0)
                .saturating_add(download_bytes.unwrap_or(0)),
        )
    } else {
        None
    };
    let remaining_bytes = total_bytes.map(|total| total.saturating_sub(used_bytes.unwrap_or(0)));

    let metadata = SubscriptionMetadata {
        source_title: None,
        upload_bytes,
        download_bytes,
        used_bytes,
        total_bytes,
        remaining_bytes,
        expire_at,
    }
    .normalized();

    let warning = if !saw_known_field || saw_invalid_known_field {
        Some("ignored invalid `subscription-userinfo` header".to_string())
    } else {
        None
    };

    (metadata, warning)
}

fn parse_profile_update_interval(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn derive_title_from_file_path(path: &str) -> Option<String> {
    let file_name = Path::new(path)
        .file_name()?
        .to_string_lossy()
        .trim()
        .to_string();
    let stem = Path::new(&file_name)
        .file_stem()
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty());
    stem.or_else(|| clean_source_title(&file_name))
}

fn derive_title_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let path_title = parsed
        .path_segments()
        .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
        .and_then(percent_decode_lossy)
        .and_then(|value| {
            Path::new(&value)
                .file_stem()
                .map(|stem| stem.to_string_lossy().trim().to_string())
                .filter(|stem| !stem.is_empty())
                .or_else(|| clean_source_title(&value))
        });
    path_title.or_else(|| clean_source_title(parsed.host_str().unwrap_or_default()))
}

fn is_information_proxy_name(name: &str) -> bool {
    let lowered = name.trim().to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }

    if INFO_PROXY_KEYWORDS_EN
        .iter()
        .any(|keyword| lowered.contains(keyword))
        || INFO_PROXY_KEYWORDS_ZH
            .iter()
            .any(|keyword| name.contains(keyword))
    {
        return true;
    }

    let english_tokens = lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let english_weak_hits = INFO_PROXY_KEYWORDS_EN_WEAK
        .iter()
        .filter(|keyword| english_tokens.iter().any(|token| token == *keyword))
        .count();
    if english_weak_hits >= 2 {
        return true;
    }

    INFO_PROXY_KEYWORDS_ZH_WEAK
        .iter()
        .filter(|keyword| name.contains(**keyword))
        .count()
        >= 2
}

fn filter_information_proxies(proxies: Vec<Value>, warnings: &mut Vec<String>) -> Vec<Value> {
    proxies
        .into_iter()
        .filter(|proxy| {
            let name = proxy
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            match name {
                Some(name) if is_information_proxy_name(name) => {
                    warnings.push(format!(
                        "filtered informational subscription entry `{name}`"
                    ));
                    false
                }
                _ => true,
            }
        })
        .collect()
}

fn parse_response_metadata(
    headers: &HeaderMap,
    fallback_title: Option<String>,
) -> ParsedResponseMetadata {
    let mut warnings = Vec::new();
    let mut metadata = SubscriptionMetadata::default();
    if let Some(value) = find_header_value(headers, "subscription-userinfo", true) {
        let (parsed_metadata, warning) = parse_subscription_userinfo(&value);
        if let Some(parsed_metadata) = parsed_metadata {
            metadata = parsed_metadata;
        }
        if let Some(warning) = warning {
            warnings.push(warning);
        }
    }

    let mut source_title = None;
    if let Some(raw_title) = find_header_value(headers, "profile-title", false) {
        match decode_profile_title(&raw_title) {
            Ok(title) => source_title = title,
            Err(err) => {
                warnings.push(format!("ignored invalid `profile-title` header: {err}"));
            }
        }
    }

    if source_title.is_none()
        && let Some(content_disposition) = headers
            .get(CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
    {
        match parse_content_disposition_filename(content_disposition) {
            Some(title) => source_title = Some(title),
            None => {
                if content_disposition_mentions_filename(content_disposition) {
                    warnings.push("ignored invalid `Content-Disposition` filename".to_string());
                }
            }
        }
    }

    let parsed_name = source_title.clone().or(fallback_title);
    metadata.source_title = source_title;

    ParsedResponseMetadata {
        metadata: metadata.normalized(),
        parsed_name,
        warnings,
        _profile_update_interval_sec: find_header_value(headers, "profile-update-interval", false)
            .and_then(|value| parse_profile_update_interval(&value)),
    }
}

async fn fetch_url_source(
    client: &reqwest::Client,
    url: &str,
) -> Result<
    (
        Vec<Value>,
        Vec<String>,
        Option<SubscriptionMetadata>,
        Option<String>,
    ),
    SubscriptionLoadError,
> {
    let mut fetch_errors = Vec::new();
    let mut parse_errors = Vec::new();
    let mut received_success_body = false;

    let attempts: Vec<(Option<&str>, String)> =
        std::iter::once((None, "default request profile".to_string()))
            .chain(
                SUBSCRIPTION_FETCH_USER_AGENTS
                    .iter()
                    .copied()
                    .map(|user_agent| (Some(user_agent), format!("User-Agent `{}`", user_agent))),
            )
            .collect();

    for (index, (user_agent, attempt_label)) in attempts.iter().enumerate() {
        let request = match user_agent {
            Some(user_agent) => client.get(url).header(USER_AGENT, *user_agent),
            None => client.get(url),
        };
        let response = match request.send().await {
            Ok(response) => match response.error_for_status() {
                Ok(response) => Ok(response),
                Err(err) => Err(SubscriptionLoadError::SourceRead(format!(
                    "subscription url `{url}` returned non-2xx with {}: {}",
                    attempt_label, err
                ))),
            },
            Err(err) => Err(SubscriptionLoadError::SourceRead(format!(
                "failed to fetch subscription url `{url}` with {}: {}",
                attempt_label, err
            ))),
        };

        let response = match response {
            Ok(response) => {
                received_success_body = true;
                response
            }
            Err(SubscriptionLoadError::SourceRead(message)) => {
                fetch_errors.push(message);
                continue;
            }
            Err(err) => return Err(err),
        };
        let headers = response.headers().clone();
        let raw = response.text().await.map_err(|err| {
            SubscriptionLoadError::SourceRead(format!(
                "failed to read subscription response body with {}: {}",
                attempt_label, err
            ))
        })?;

        match parse_subscription_payload(&raw) {
            Ok(proxies) if payload_has_usable_proxy_entries(&proxies) => {
                let mut warnings = Vec::new();
                let filtered_proxies = filter_information_proxies(proxies, &mut warnings);
                if !payload_has_usable_proxy_entries(&filtered_proxies) {
                    parse_errors.push(format!(
                        "{}: payload only contained informational entries after filtering",
                        attempt_label
                    ));
                    continue;
                }
                if index > 0 {
                    warnings.push(format!(
                        "subscription payload required fallback {}",
                        attempt_label
                    ));
                }
                let mut metadata = parse_response_metadata(&headers, derive_title_from_url(url));
                warnings.append(&mut metadata.warnings);
                return Ok((
                    filtered_proxies,
                    warnings,
                    metadata.metadata,
                    metadata.parsed_name,
                ));
            }
            Ok(_) => {
                parse_errors.push(format!(
                    "{}: payload parsed but did not contain any usable proxy entries",
                    attempt_label
                ));
            }
            Err(SubscriptionLoadError::InvalidPayload(message)) => {
                parse_errors.push(format!("{}: {}", attempt_label, message));
            }
            Err(err) => return Err(err),
        }
    }

    if received_success_body {
        return Err(SubscriptionLoadError::InvalidPayload(format!(
            "subscription payload was not parseable with any compatibility user agent: {}",
            parse_errors.join(" | ")
        )));
    }

    Err(SubscriptionLoadError::SourceRead(format!(
        "failed to fetch subscription url `{url}` with all compatibility attempts: {}",
        fetch_errors.join(" | ")
    )))
}

async fn load_from_proxies(
    proxies: Vec<Value>,
    mut warnings: Vec<String>,
) -> Result<LoadedSubscription, SubscriptionLoadError> {
    let sem = ArcSemaphore::new(DEFAULT_DNS_CONCURRENCY);
    let mut tasks = Vec::new();
    for yaml_proxy in proxies {
        let permit = sem.acquire().await;
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let json_proxy = to_json_value(&yaml_proxy)?;
            let (name, proxy_type, server) = extract_proxy_fields(&json_proxy)?;
            let mut warnings = Vec::new();
            let resolved_ips = match resolve_server_ips(&server).await {
                Ok(ips) => ips,
                Err(err) => {
                    warnings.push(format!(
                        "dns lookup failed for proxy `{}` server `{}`: {}",
                        name, server, err
                    ));
                    Vec::new()
                }
            };
            Ok::<(ProxyNode, Vec<String>), anyhow::Error>((
                ProxyNode {
                    node_id: None,
                    proxy_name: name,
                    proxy_type,
                    server,
                    resolved_ips,
                    raw_proxy: json_proxy,
                },
                warnings,
            ))
        }));
    }

    let mut nodes = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok((node, node_warnings))) => {
                nodes.push(node);
                warnings.extend(node_warnings);
            }
            Ok(Err(err)) => warnings.push(err.to_string()),
            Err(err) => warnings.push(format!("task join error: {err}")),
        }
    }

    nodes.sort_by(|a, b| a.proxy_name.cmp(&b.proxy_name));
    Ok(LoadedSubscription {
        nodes,
        warnings,
        metadata: None,
        parsed_name: None,
    })
}

pub async fn load_from_source(
    client: &reqwest::Client,
    source: &crate::models::SubscriptionSource,
) -> Result<LoadedSubscription, SubscriptionLoadError> {
    let (mut proxies, mut warnings, metadata, parsed_name) = match source {
        crate::models::SubscriptionSource::Url(url) => fetch_url_source(client, url).await?,
        crate::models::SubscriptionSource::File(path) => {
            let raw = tokio::fs::read_to_string(path).await.map_err(|err| {
                SubscriptionLoadError::InvalidPayload(format!(
                    "failed to read subscription file `{path}`: {err}"
                ))
            })?;
            (
                parse_subscription_payload(&raw)?,
                Vec::new(),
                None,
                derive_title_from_file_path(path),
            )
        }
    };
    proxies = filter_information_proxies(proxies, &mut warnings);
    let mut loaded = load_from_proxies(proxies, warnings).await?;
    loaded.metadata = metadata;
    loaded.parsed_name = parsed_name;
    Ok(loaded)
}

pub async fn load_from_content(raw: &str) -> Result<LoadedSubscription, SubscriptionLoadError> {
    load_from_proxies(parse_subscription_payload(raw)?, Vec::new()).await
}

#[derive(Clone)]
struct ArcSemaphore(Arc<tokio::sync::Semaphore>);

impl ArcSemaphore {
    fn new(limit: usize) -> Self {
        Self(Arc::new(tokio::sync::Semaphore::new(limit.max(1))))
    }

    async fn acquire(&self) -> tokio::sync::OwnedSemaphorePermit {
        self.0
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore should not close")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SUBSCRIPTION_FETCH_USER_AGENTS, SubscriptionLoadError, load_from_source,
        parse_content_disposition_filename, parse_response_metadata, percent_decode_lossy,
    };
    use crate::models::SubscriptionSource;
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, HeaderValue, StatusCode},
        routing::get,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::Duration;

    #[derive(Clone)]
    struct TestSubscriptionServerState {
        accepted_user_agent: Option<Arc<str>>,
        success_payload: Arc<str>,
        fallback_status: Option<StatusCode>,
        response_headers: HeaderMap,
    }

    async fn test_subscription_handler(
        State(state): State<TestSubscriptionServerState>,
        headers: HeaderMap,
    ) -> (StatusCode, HeaderMap, String) {
        let user_agent = headers
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok());
        if user_agent == state.accepted_user_agent.as_deref() {
            (
                StatusCode::OK,
                state.response_headers.clone(),
                state.success_payload.to_string(),
            )
        } else if user_agent.is_some() {
            (
                state.fallback_status.unwrap_or(StatusCode::OK),
                HeaderMap::new(),
                "not-a-clash-subscription".to_string(),
            )
        } else {
            (
                StatusCode::OK,
                HeaderMap::new(),
                "not-a-clash-subscription".to_string(),
            )
        }
    }

    async fn test_forbidden_handler() -> (StatusCode, &'static str) {
        (StatusCode::FORBIDDEN, "blocked")
    }

    async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .route("/forbidden", get(test_forbidden_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: Some(Arc::<str>::from(SUBSCRIPTION_FETCH_USER_AGENTS[1])),
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: ua-ok
    type: socks5
    server: 1.1.1.1
"#,
                ),
                fallback_status: None,
                response_headers: HeaderMap::new(),
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
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn missing_file_is_reported_as_invalid_payload() {
        let client = reqwest::Client::new();
        let source = SubscriptionSource::File("/tmp/proxy-broker-missing-file.yaml".to_string());
        let err = load_from_source(&client, &source)
            .await
            .expect_err("missing file should fail");
        assert!(matches!(err, SubscriptionLoadError::InvalidPayload(_)));
    }

    #[tokio::test]
    async fn url_source_uses_mihomo_user_agent_and_loads_yaml_payload() {
        let client = reqwest::Client::new();
        let (base_url, server) = spawn_test_server().await;
        let source = SubscriptionSource::Url(format!("{base_url}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("url source should load when a compatibility ua succeeds");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "ua-ok");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[1]));
    }

    #[tokio::test]
    async fn url_source_retries_when_default_payload_is_yaml_stub() {
        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: Some(Arc::<str>::from(SUBSCRIPTION_FETCH_USER_AGENTS[0])),
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: stub-recovered
    type: socks5
    server: 4.4.4.4
"#,
                ),
                fallback_status: None,
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });

        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));
        let result = load_from_source(&client, &source)
            .await
            .expect("yaml stub should trigger ua fallback");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "stub-recovered");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[0]));
    }

    #[tokio::test]
    async fn url_source_keeps_default_request_profile_before_fallbacks() {
        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: None,
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: default-ok
    type: socks5
    server: 2.2.2.2
"#,
                ),
                fallback_status: None,
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("default request profile should still work");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "default-ok");
        assert!(result.warnings.is_empty());
    }

    #[tokio::test]
    async fn url_source_retries_when_default_payload_only_contains_information_nodes() {
        #[derive(Clone)]
        struct InfoFallbackState {
            accepted_user_agent: Arc<str>,
        }

        async fn info_fallback_handler(
            State(state): State<InfoFallbackState>,
            headers: HeaderMap,
        ) -> (StatusCode, HeaderMap, String) {
            let user_agent = headers
                .get(reqwest::header::USER_AGENT)
                .and_then(|value| value.to_str().ok());
            if user_agent == Some(state.accepted_user_agent.as_ref()) {
                return (
                    StatusCode::OK,
                    HeaderMap::new(),
                    r#"
proxies:
  - name: real-node
    type: socks5
    server: 6.6.6.6
"#
                    .to_string(),
                );
            }

            (
                StatusCode::OK,
                HeaderMap::new(),
                r#"
proxies:
  - name: 剩余流量 12GB
    type: socks5
    server: 1.1.1.1
"#
                .to_string(),
            )
        }

        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(info_fallback_handler))
            .with_state(InfoFallbackState {
                accepted_user_agent: Arc::<str>::from(SUBSCRIPTION_FETCH_USER_AGENTS[0]),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("info-only default payload should still allow compatibility fallback");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "real-node");
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains(SUBSCRIPTION_FETCH_USER_AGENTS[0]))
        );
    }

    #[tokio::test]
    async fn url_source_reports_fetch_error_on_non_2xx_response() {
        let client = reqwest::Client::new();
        let (base_url, server) = spawn_test_server().await;
        let source = SubscriptionSource::Url(format!("{base_url}/forbidden"));

        let err = load_from_source(&client, &source)
            .await
            .expect_err("non-2xx source should fail");

        server.abort();

        assert!(
            matches!(err, SubscriptionLoadError::SourceRead(message) if message.contains("returned non-2xx"))
        );
    }

    #[tokio::test]
    async fn url_source_retries_after_transport_failure_until_a_compatibility_ua_succeeds() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("test client should build");
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let accepts = Arc::new(AtomicUsize::new(0));
        let accepts_task = accepts.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut stream, _peer)) = listener.accept().await {
                let attempt = accepts_task.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    drop(stream);
                    continue;
                }

                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let read = stream
                        .read(&mut buffer)
                        .await
                        .expect("request should be readable");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }

                let request = String::from_utf8(request).expect("request should be utf-8");
                assert!(
                    request.contains(&format!(
                        "user-agent: {}\r\n",
                        SUBSCRIPTION_FETCH_USER_AGENTS[0]
                    )) || request.contains(&format!(
                        "User-Agent: {}\r\n",
                        SUBSCRIPTION_FETCH_USER_AGENTS[0]
                    )),
                    "second request should use the first compatibility user agent"
                );

                let body = r#"
proxies:
  - name: recovered-after-transport-failure
    type: socks5
    server: 5.5.5.5
"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("response should be writable");
                break;
            }
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("compatibility ua should recover after transport failure");
        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(
            result.nodes[0].proxy_name,
            "recovered-after-transport-failure"
        );
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[0]));
        assert_eq!(accepts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn url_source_keeps_invalid_payload_when_attempts_mix_fetch_and_parse_failures() {
        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: Some(Arc::<str>::from("unmatched-user-agent")),
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: unreachable
    type: socks5
    server: 3.3.3.3
"#,
                ),
                fallback_status: Some(StatusCode::FORBIDDEN),
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let err = load_from_source(&client, &source)
            .await
            .expect_err("mixed fetch and parse failures should stay invalid payload");

        server.abort();

        assert!(
            matches!(err, SubscriptionLoadError::InvalidPayload(message) if message.contains("default request profile"))
        );
    }

    #[tokio::test]
    async fn url_source_parses_profile_title_and_subscription_userinfo_metadata() {
        let client = reqwest::Client::new();
        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            "profile-title",
            HeaderValue::from_static("base64:ZWRnZS1mZWVk"),
        );
        response_headers.insert(
            "x-clash-meta-subscription-userinfo",
            HeaderValue::from_static("upload=10; download=20; total=100; expire=1710000000"),
        );
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: None,
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: jp-main
    type: socks5
    server: 1.1.1.1
"#,
                ),
                fallback_status: None,
                response_headers,
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("metadata response should load");

        server.abort();

        let metadata = result.metadata.expect("subscription metadata should exist");
        assert_eq!(metadata.source_title.as_deref(), Some("edge-feed"));
        assert_eq!(metadata.upload_bytes, Some(10));
        assert_eq!(metadata.download_bytes, Some(20));
        assert_eq!(metadata.used_bytes, Some(30));
        assert_eq!(metadata.total_bytes, Some(100));
        assert_eq!(metadata.remaining_bytes, Some(70));
        assert_eq!(metadata.expire_at, Some(1_710_000_000));
    }

    #[test]
    fn response_metadata_falls_back_to_content_disposition_when_profile_title_is_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "profile-title",
            HeaderValue::from_static("base64:not-valid-base64"),
        );
        headers.insert(
            reqwest::header::CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment; filename*=UTF-8''fallback-name.yaml"),
        );

        let parsed = parse_response_metadata(&headers, Some("url-fallback".to_string()));

        let metadata = parsed
            .metadata
            .expect("content disposition title should survive");
        assert_eq!(metadata.source_title.as_deref(), Some("fallback-name.yaml"));
        assert_eq!(parsed.parsed_name.as_deref(), Some("fallback-name.yaml"));
        assert!(
            parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("ignored invalid `profile-title` header"))
        );
    }

    #[test]
    fn response_metadata_falls_back_to_url_title_when_higher_priority_headers_are_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "profile-title",
            HeaderValue::from_static("base64:not-valid-base64"),
        );
        headers.insert(
            reqwest::header::CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment; filename*=UTF-8''%ZZ"),
        );

        let parsed = parse_response_metadata(&headers, Some("url-fallback".to_string()));

        assert!(parsed.metadata.is_none());
        assert_eq!(parsed.parsed_name.as_deref(), Some("url-fallback"));
        assert!(
            parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("ignored invalid `profile-title` header"))
        );
        assert!(
            parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("ignored invalid `Content-Disposition` filename"))
        );
    }

    #[test]
    fn percent_decode_keeps_literal_plus_characters() {
        assert_eq!(
            percent_decode_lossy("A+B.yaml").as_deref(),
            Some("A+B.yaml")
        );
        assert_eq!(
            percent_decode_lossy("A%20B.yaml").as_deref(),
            Some("A B.yaml")
        );
    }

    #[test]
    fn content_disposition_filename_star_preserves_plus_characters() {
        assert_eq!(
            parse_content_disposition_filename("attachment; filename*=UTF-8''A+B.yaml").as_deref(),
            Some("A+B.yaml")
        );
    }

    #[test]
    fn content_disposition_filename_star_accepts_language_tag() {
        assert_eq!(
            parse_content_disposition_filename("attachment; filename*=UTF-8'en'A+B.yaml")
                .as_deref(),
            Some("A+B.yaml")
        );
    }

    #[test]
    fn response_metadata_ignores_bare_content_disposition_without_warning() {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment"),
        );

        let parsed = parse_response_metadata(&headers, Some("url-fallback".to_string()));

        assert!(parsed.metadata.is_none());
        assert_eq!(parsed.parsed_name.as_deref(), Some("url-fallback"));
        assert!(
            parsed
                .warnings
                .iter()
                .all(|warning| !warning.contains("Content-Disposition"))
        );
    }

    #[test]
    fn response_metadata_warns_on_invalid_subscription_userinfo() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "subscription-userinfo",
            HeaderValue::from_static("upload=nope; total=bad"),
        );

        let parsed = parse_response_metadata(&headers, Some("url-fallback".to_string()));

        assert!(parsed.metadata.is_none());
        assert_eq!(parsed.parsed_name.as_deref(), Some("url-fallback"));
        assert!(
            parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("ignored invalid `subscription-userinfo` header"))
        );
    }

    #[tokio::test]
    async fn url_source_filters_information_nodes_and_warns() {
        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: None,
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: 剩余流量 12GB
    type: socks5
    server: 1.1.1.1
  - name: jp-01
    type: socks5
    server: 8.8.8.8
"#,
                ),
                fallback_status: None,
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("filtering should keep usable nodes");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "jp-01");
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("filtered informational subscription entry"))
        );
    }

    #[tokio::test]
    async fn url_source_keeps_legitimate_nodes_with_single_weak_branding_keyword() {
        let client = reqwest::Client::new();
        let app = Router::new()
            .route("/subscription", get(test_subscription_handler))
            .with_state(TestSubscriptionServerState {
                accepted_user_agent: None,
                success_payload: Arc::<str>::from(
                    r#"
proxies:
  - name: Official-US-01
    type: socks5
    server: 8.8.4.4
"#,
                ),
                fallback_status: None,
                response_headers: HeaderMap::new(),
            });
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test listener should bind");
        let addr = listener.local_addr().expect("listener addr should exist");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should serve requests");
        });
        let source = SubscriptionSource::Url(format!("http://{addr}/subscription"));

        let result = load_from_source(&client, &source)
            .await
            .expect("single weak keyword should not make the node informational");

        server.abort();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].proxy_name, "Official-US-01");
        assert!(
            result
                .warnings
                .iter()
                .all(|warning| !warning.contains("filtered informational subscription entry"))
        );
    }

    #[tokio::test]
    async fn file_source_uses_file_stem_as_name_fallback() {
        let client = reqwest::Client::new();
        let path = std::env::temp_dir().join("proxy-broker-source-title.yaml");
        tokio::fs::write(
            &path,
            r#"
proxies:
  - name: file-node
    type: socks5
    server: 9.9.9.9
"#,
        )
        .await
        .expect("subscription file should be written");

        let result = load_from_source(
            &client,
            &SubscriptionSource::File(path.to_string_lossy().to_string()),
        )
        .await
        .expect("file source should load");

        let _ = tokio::fs::remove_file(&path).await;

        assert!(result.metadata.is_none());
        assert_eq!(
            result.parsed_name.as_deref(),
            Some("proxy-broker-source-title")
        );
    }
}
