use anyhow::Context;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};

use crate::{
    ids,
    models::{ProxyNode, SessionRecord},
};

pub(crate) fn dedicated_ip_proxy_name(proxy_name: &str, ip: &str) -> String {
    ids::stable_dedicated_ip_proxy_name(proxy_name, ip)
}

pub fn render_payload(
    controller_addr: &str,
    secret: Option<&str>,
    nodes: &[ProxyNode],
    sessions: &[SessionRecord],
) -> anyhow::Result<String> {
    let allow_lan = sessions.iter().any(session_exposes_lan);
    let node_by_id: HashMap<&str, &ProxyNode> = nodes
        .iter()
        .filter_map(|node| node.node_id.as_deref().map(|node_id| (node_id, node)))
        .collect();
    let mut proxy_name_counts = HashMap::<&str, usize>::new();
    for node in nodes {
        *proxy_name_counts
            .entry(node.proxy_name.as_str())
            .or_insert(0) += 1;
    }
    let unique_node_by_proxy_name: HashMap<&str, &ProxyNode> = nodes
        .iter()
        .filter(|node| proxy_name_counts.get(node.proxy_name.as_str()) == Some(&1))
        .map(|node| (node.proxy_name.as_str(), node))
        .collect();
    let legacy_alias_names: HashSet<&str> = sessions
        .iter()
        .filter(|session| unique_node_by_proxy_name.contains_key(session.proxy_name.as_str()))
        .map(|session| session.proxy_name.as_str())
        .collect();

    let mut proxies = Vec::new();
    for node in nodes {
        let runtime_name = runtime_proxy_name(node);
        let mut raw = node.raw_proxy.clone();
        raw["name"] = serde_json::Value::String(runtime_name.clone());
        proxies.push(raw);

        if runtime_name != node.proxy_name && legacy_alias_names.contains(node.proxy_name.as_str())
        {
            let mut raw = node.raw_proxy.clone();
            raw["name"] = serde_json::Value::String(node.proxy_name.clone());
            proxies.push(raw);
        }

        for ip in &node.resolved_ips {
            let mut dedicated = node.raw_proxy.clone();
            dedicated["name"] =
                serde_json::Value::String(dedicated_ip_proxy_name(&runtime_name, ip));
            dedicated["server"] = serde_json::Value::String(ip.clone());
            proxies.push(dedicated);
        }
    }

    let listeners = sessions
        .iter()
        .map(|session| {
            let session_proxy_name =
                session_proxy_name(session, &node_by_id, &unique_node_by_proxy_name);

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
        "allow-lan": allow_lan,
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

fn runtime_proxy_name(node: &ProxyNode) -> String {
    node.node_id
        .clone()
        .unwrap_or_else(|| node.proxy_name.clone())
}

fn session_proxy_name(
    session: &SessionRecord,
    node_by_id: &HashMap<&str, &ProxyNode>,
    unique_node_by_proxy_name: &HashMap<&str, &ProxyNode>,
) -> String {
    if node_by_id.contains_key(session.node_id.as_str()) {
        return dedicated_ip_proxy_name(&session.node_id, &session.selected_ip);
    }

    if let Some(node) = unique_node_by_proxy_name.get(session.proxy_name.as_str()) {
        if node
            .resolved_ips
            .iter()
            .any(|ip| ip == &session.selected_ip)
        {
            return dedicated_ip_proxy_name(&runtime_proxy_name(node), &session.selected_ip);
        }
        return session.proxy_name.clone();
    }

    if !session.node_id.trim().is_empty() {
        session.node_id.clone()
    } else {
        session.proxy_name.clone()
    }
}

fn session_exposes_lan(session: &SessionRecord) -> bool {
    session
        .listen
        .parse::<IpAddr>()
        .map(|ip| !ip.is_loopback())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProxyNode;

    fn sample_node() -> ProxyNode {
        ProxyNode {
            node_id: Some("node-a".to_string()),
            proxy_name: "proxy-a".to_string(),
            proxy_type: "socks5".to_string(),
            server: "example.test".to_string(),
            resolved_ips: vec!["1.1.1.1".to_string()],
            raw_proxy: serde_json::json!({
                "name": "proxy-a",
                "type": "socks5",
                "server": "example.test",
                "port": 1080
            }),
        }
    }

    fn sample_session(listen: &str) -> SessionRecord {
        SessionRecord {
            session_id: "sess-A7c2Kp9LmQ4RsT1v".to_string(),
            listen: listen.to_string(),
            port: 20000,
            selected_ip: "1.1.1.1".to_string(),
            proxy_name: "proxy-a".to_string(),
            node_id: "node-a".to_string(),
            candidate_node_ids: vec!["node-a".to_string()],
            created_at: 0,
        }
    }

    #[test]
    fn render_payload_keeps_allow_lan_disabled_for_loopback_listeners() {
        let payload = render_payload(
            "127.0.0.1:9090",
            None,
            &[sample_node()],
            &[sample_session("127.0.0.1")],
        )
        .expect("payload should render");
        assert!(payload.contains("allow-lan: false"));
    }

    #[test]
    fn render_payload_falls_back_to_proxy_name_for_sessions_without_node_id() {
        let payload = render_payload(
            "127.0.0.1:9090",
            None,
            &[sample_node()],
            &[SessionRecord {
                node_id: String::new(),
                ..sample_session("127.0.0.1")
            }],
        )
        .expect("payload should render");
        assert!(payload.contains("name: proxy-a"));
        assert!(payload.contains(&format!(
            "proxy: {}",
            dedicated_ip_proxy_name("node-a", "1.1.1.1")
        )));
    }

    #[test]
    fn render_payload_falls_back_to_legacy_proxy_alias_for_missing_node_id_mapping() {
        let payload = render_payload(
            "127.0.0.1:9090",
            None,
            &[sample_node()],
            &[SessionRecord {
                node_id: "node-missing".to_string(),
                ..sample_session("127.0.0.1")
            }],
        )
        .expect("payload should render");
        assert!(payload.contains("name: proxy-a"));
        assert!(payload.contains(&format!(
            "proxy: {}",
            dedicated_ip_proxy_name("node-a", "1.1.1.1")
        )));
    }

    #[test]
    fn render_payload_uses_legacy_proxy_alias_when_selected_ip_is_no_longer_present() {
        let payload = render_payload(
            "127.0.0.1:9090",
            None,
            &[sample_node()],
            &[SessionRecord {
                node_id: String::new(),
                selected_ip: "9.9.9.9".to_string(),
                ..sample_session("127.0.0.1")
            }],
        )
        .expect("payload should render");
        assert!(payload.contains("name: proxy-a"));
        assert!(payload.contains("proxy: proxy-a"));
    }

    #[test]
    fn render_payload_enables_allow_lan_for_wildcard_listeners() {
        let payload = render_payload(
            "127.0.0.1:9090",
            None,
            &[sample_node()],
            &[sample_session("0.0.0.0")],
        )
        .expect("payload should render");
        assert!(payload.contains("allow-lan: true"));
    }
}
