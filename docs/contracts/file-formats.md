# File formats

## Runtime payload to mihomo `/configs`

- Apply API: `PUT /configs?force=true`
- Payload contains full YAML string serialized from broker state.
- Must include:
  - `listeners[]`: each session -> one `mixed` listener
  - `proxies[]`: all parsed proxies + 每个 `(proxy_name, ip)` 的固定 IP 代理副本
  - minimal routing defaults to keep config valid (`mode`, `rules`, `dns`, `log-level`)

## Listener shape

- `name`: `broker-<session_id>`
- `type`: `mixed`
- `listen`: configured session listener bind IP (`127.0.0.1` by default, `0.0.0.0` for wildcard deployments)
- `port`: allocated port
- `proxy`: `broker-ip-<hash>`（由 `proxy_name + selected_ip` 派生）
