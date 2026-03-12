use std::{collections::HashSet, net::IpAddr, sync::Arc};

use anyhow::{Context, anyhow};
use base64::Engine;
use serde_yaml::Value;
use thiserror::Error;

use crate::{constants::DEFAULT_DNS_CONCURRENCY, models::ProxyNode};

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

pub async fn load_from_source(
    client: &reqwest::Client,
    source: &crate::models::SubscriptionSource,
) -> Result<(Vec<ProxyNode>, Vec<String>), SubscriptionLoadError> {
    let raw = match source {
        crate::models::SubscriptionSource::Url(url) => client
            .get(url)
            .send()
            .await
            .map_err(|err| {
                SubscriptionLoadError::SourceRead(format!(
                    "failed to fetch subscription url `{url}`: {err}"
                ))
            })?
            .error_for_status()
            .map_err(|err| {
                SubscriptionLoadError::SourceRead(format!(
                    "subscription url `{url}` returned non-2xx: {err}"
                ))
            })?
            .text()
            .await
            .map_err(|err| {
                SubscriptionLoadError::SourceRead(format!(
                    "failed to read subscription response body: {err}"
                ))
            })?,
        crate::models::SubscriptionSource::File(path) => {
            tokio::fs::read_to_string(path).await.map_err(|err| {
                SubscriptionLoadError::InvalidPayload(format!(
                    "failed to read subscription file `{path}`: {err}"
                ))
            })?
        }
    };

    let proxies = match extract_proxies_from_yaml(&raw) {
        Ok(p) => p,
        Err(yaml_err) => {
            let decoded = decode_base64_yaml(&raw).map_err(|base64_err| {
                SubscriptionLoadError::InvalidPayload(format!(
                    "yaml parse failed: {yaml_err}; base64 fallback failed: {base64_err}"
                ))
            })?;
            extract_proxies_from_yaml(&decoded)
                .map_err(|err| SubscriptionLoadError::InvalidPayload(err.to_string()))?
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
    let mut warnings = Vec::new();
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
    use super::{SubscriptionLoadError, load_from_source};
    use crate::models::SubscriptionSource;

    #[tokio::test]
    async fn missing_file_is_reported_as_invalid_payload() {
        let client = reqwest::Client::new();
        let source = SubscriptionSource::File("/tmp/proxy-broker-missing-file.yaml".to_string());
        let err = load_from_source(&client, &source)
            .await
            .expect_err("missing file should fail");
        assert!(matches!(err, SubscriptionLoadError::InvalidPayload(_)));
    }
}
