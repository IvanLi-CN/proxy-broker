use crate::models::ProxyNode;

fn speed_field_names(proxy_type: &str) -> Option<&'static [&'static str]> {
    match proxy_type.trim().to_ascii_lowercase().as_str() {
        "hysteria" | "hysteria2" => Some(&["up", "down"]),
        _ => None,
    }
}

fn speed_value_is_valid(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Number(_) => true,
        serde_json::Value::String(value) => {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_digit())
        }
        _ => false,
    }
}

fn invalid_value_repr(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => format!("{value:?}"),
        _ => value.to_string(),
    }
}

pub(crate) fn malformed_proxy_reason(
    proxy_type: &str,
    raw_proxy: &serde_json::Value,
) -> Option<String> {
    let fields = speed_field_names(proxy_type)?;

    let mut invalid_fields = Vec::new();
    for field in fields {
        let Some(value) = raw_proxy.get(field) else {
            continue;
        };
        if !speed_value_is_valid(value) {
            invalid_fields.push(format!("`{field}`={}", invalid_value_repr(value)));
        }
    }

    if invalid_fields.is_empty() {
        None
    } else {
        Some(format!(
            "{} proxy has invalid rate field(s): {}",
            proxy_type.trim().to_ascii_lowercase(),
            invalid_fields.join(", ")
        ))
    }
}

pub(crate) fn malformed_proxy_warning(
    proxy_name: &str,
    proxy_type: &str,
    raw_proxy: &serde_json::Value,
) -> Option<String> {
    malformed_proxy_reason(proxy_type, raw_proxy)
        .map(|reason| format!("filtered malformed proxy entry `{proxy_name}`: {reason}"))
}

pub(crate) fn filter_malformed_proxy_nodes(
    nodes: Vec<ProxyNode>,
    warnings: &mut Vec<String>,
) -> Vec<ProxyNode> {
    nodes
        .into_iter()
        .filter(|node| {
            if let Some(warning) =
                malformed_proxy_warning(&node.proxy_name, &node.proxy_type, &node.raw_proxy)
            {
                warnings.push(warning);
                false
            } else {
                true
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{filter_malformed_proxy_nodes, malformed_proxy_reason};
    use crate::models::ProxyNode;

    fn sample_node(
        proxy_name: &str,
        proxy_type: &str,
        server: &str,
        raw_proxy: serde_json::Value,
    ) -> ProxyNode {
        ProxyNode {
            node_id: None,
            proxy_name: proxy_name.to_string(),
            proxy_type: proxy_type.to_string(),
            server: server.to_string(),
            resolved_ips: vec![server.to_string()],
            raw_proxy,
        }
    }

    #[test]
    fn malformed_proxy_reason_accepts_numeric_and_rate_strings() {
        assert!(
            malformed_proxy_reason(
                "hysteria",
                &serde_json::json!({
                    "name": "good-a",
                    "type": "hysteria",
                    "server": "1.1.1.1",
                    "up": "30 Mbps",
                    "down": 100
                }),
            )
            .is_none()
        );
    }

    #[test]
    fn malformed_proxy_reason_rejects_empty_or_non_numeric_speed_values() {
        let reason = malformed_proxy_reason(
            "hysteria2",
            &serde_json::json!({
                "name": "bad-a",
                "type": "hysteria2",
                "server": "1.1.1.1",
                "up": "",
                "down": "fast"
            }),
        )
        .expect("invalid hysteria rates should be flagged");

        assert!(reason.contains("`up`=\"\""));
        assert!(reason.contains("`down`=\"fast\""));
    }

    #[test]
    fn filter_malformed_proxy_nodes_keeps_only_valid_entries() {
        let mut warnings = Vec::new();
        let nodes = filter_malformed_proxy_nodes(
            vec![
                sample_node(
                    "good-node",
                    "socks5",
                    "1.1.1.1",
                    serde_json::json!({
                        "name": "good-node",
                        "type": "socks5",
                        "server": "1.1.1.1"
                    }),
                ),
                sample_node(
                    "bad-node",
                    "hysteria",
                    "2.2.2.2",
                    serde_json::json!({
                        "name": "bad-node",
                        "type": "hysteria",
                        "server": "2.2.2.2",
                        "up": "   ",
                        "down": "100 Mbps"
                    }),
                ),
            ],
            &mut warnings,
        );

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].proxy_name, "good-node");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("bad-node"));
    }
}
