use anyhow::Context;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{ProxyNode, SessionRecord};

pub(crate) fn dedicated_ip_proxy_name(proxy_name: &str, ip: &str) -> String {
    let key = format!("{proxy_name}|{ip}");
    let digest = Uuid::new_v5(&Uuid::NAMESPACE_URL, key.as_bytes())
        .simple()
        .to_string();
    format!("broker-ip-{digest}")
}

pub fn render_payload(
    controller_addr: &str,
    secret: Option<&str>,
    nodes: &[ProxyNode],
    sessions: &[SessionRecord],
) -> anyhow::Result<String> {
    let node_by_name: HashMap<&str, &ProxyNode> = nodes
        .iter()
        .map(|node| (node.proxy_name.as_str(), node))
        .collect();

    let mut proxies = nodes
        .iter()
        .map(|n| n.raw_proxy.clone())
        .collect::<Vec<_>>();
    for node in nodes {
        for ip in &node.resolved_ips {
            let mut dedicated = node.raw_proxy.clone();
            dedicated["name"] =
                serde_json::Value::String(dedicated_ip_proxy_name(&node.proxy_name, ip));
            dedicated["server"] = serde_json::Value::String(ip.clone());
            proxies.push(dedicated);
        }
    }

    let listeners = sessions
        .iter()
        .map(|session| {
            let session_proxy_name = if node_by_name.contains_key(session.proxy_name.as_str()) {
                dedicated_ip_proxy_name(&session.proxy_name, &session.selected_ip)
            } else {
                session.proxy_name.clone()
            };

            serde_json::json!({
                "name": format!("broker-{}", session.session_id),
                "type": "mixed",
                "listen": session.listen,
                "port": session.port,
                "proxy": session_proxy_name,
            })
        })
        .collect::<Vec<_>>();

    let mut root = serde_json::json!({
        "mode": "rule",
        "log-level": "warning",
        "allow-lan": false,
        "external-controller": controller_addr,
        "proxies": proxies,
        "listeners": listeners,
        "rules": ["MATCH,DIRECT"],
    });

    if let Some(secret) = secret {
        root["secret"] = serde_json::Value::String(secret.to_string());
    }

    serde_yaml::to_string(&root).context("failed to serialize mihomo payload")
}
