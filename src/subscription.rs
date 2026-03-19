use std::{collections::HashSet, net::IpAddr, sync::Arc};

use anyhow::{Context, anyhow};
use base64::Engine;
use reqwest::header::USER_AGENT;
use serde_yaml::Value;
use thiserror::Error;

use crate::{constants::DEFAULT_DNS_CONCURRENCY, models::ProxyNode};

pub const SUBSCRIPTION_FETCH_USER_AGENTS: &[&str] =
    &["Clash.Meta/1.18.3", "mihomo/1.18.3", "Clash Verge/1.7.7"];

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

async fn fetch_url_source(
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<Value>, Vec<String>), SubscriptionLoadError> {
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
        let raw = match request.send().await {
            Ok(response) => match response.error_for_status() {
                Ok(response) => response.text().await.map_err(|err| {
                    SubscriptionLoadError::SourceRead(format!(
                        "failed to read subscription response body with {}: {}",
                        attempt_label, err
                    ))
                }),
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

        let raw = match raw {
            Ok(raw) => {
                received_success_body = true;
                raw
            }
            Err(SubscriptionLoadError::SourceRead(message)) => {
                fetch_errors.push(message);
                continue;
            }
            Err(err) => return Err(err),
        };

        match parse_subscription_payload(&raw) {
            Ok(proxies) if payload_has_usable_proxy_entries(&proxies) => {
                let mut warnings = Vec::new();
                if index > 0 {
                    warnings.push(format!(
                        "subscription payload required fallback {}",
                        attempt_label
                    ));
                }
                return Ok((proxies, warnings));
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

pub async fn load_from_source(
    client: &reqwest::Client,
    source: &crate::models::SubscriptionSource,
) -> Result<(Vec<ProxyNode>, Vec<String>), SubscriptionLoadError> {
    let (proxies, mut warnings) = match source {
        crate::models::SubscriptionSource::Url(url) => fetch_url_source(client, url).await?,
        crate::models::SubscriptionSource::File(path) => {
            let raw = tokio::fs::read_to_string(path).await.map_err(|err| {
                SubscriptionLoadError::InvalidPayload(format!(
                    "failed to read subscription file `{path}`: {err}"
                ))
            })?;
            (parse_subscription_payload(&raw)?, Vec::new())
        }
    };

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
    Ok((nodes, warnings))
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
    use super::{SUBSCRIPTION_FETCH_USER_AGENTS, SubscriptionLoadError, load_from_source};
    use crate::models::SubscriptionSource;
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, StatusCode},
        routing::get,
    };
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[derive(Clone)]
    struct TestSubscriptionServerState {
        accepted_user_agent: Option<Arc<str>>,
        success_payload: Arc<str>,
        fallback_status: Option<StatusCode>,
    }

    async fn test_subscription_handler(
        State(state): State<TestSubscriptionServerState>,
        headers: HeaderMap,
    ) -> (StatusCode, String) {
        let user_agent = headers
            .get(reqwest::header::USER_AGENT)
            .and_then(|value| value.to_str().ok());
        if user_agent == state.accepted_user_agent.as_deref() {
            (StatusCode::OK, state.success_payload.to_string())
        } else if user_agent.is_some() {
            (
                state.fallback_status.unwrap_or(StatusCode::OK),
                "not-a-clash-subscription".to_string(),
            )
        } else {
            (StatusCode::OK, "not-a-clash-subscription".to_string())
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

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].proxy_name, "ua-ok");
        assert_eq!(result.1.len(), 1);
        assert!(result.1[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[1]));
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

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].proxy_name, "stub-recovered");
        assert_eq!(result.1.len(), 1);
        assert!(result.1[0].contains(SUBSCRIPTION_FETCH_USER_AGENTS[0]));
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

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].proxy_name, "default-ok");
        assert!(result.1.is_empty());
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
}
