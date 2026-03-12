# 数据库（DB）

## Schema (SQLite)

- `subscription_nodes`
  - `profile_id TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `proxy_type TEXT NOT NULL`
  - `server TEXT NOT NULL`
  - `resolved_ips_json TEXT NOT NULL`
  - `raw_proxy_json TEXT NOT NULL`
  - PK `(profile_id, proxy_name)`

- `ip_records`
  - `profile_id TEXT NOT NULL`
  - `ip TEXT NOT NULL`
  - `country_code TEXT`
  - `country_name TEXT`
  - `region_name TEXT`
  - `city TEXT`
  - `geo_source TEXT`
  - `probe_updated_at INTEGER`
  - `geo_updated_at INTEGER`
  - `last_used_at INTEGER`
  - PK `(profile_id, ip)`

- `probe_records`
  - `profile_id TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `ip TEXT NOT NULL`
  - `target_url TEXT NOT NULL`
  - `ok INTEGER NOT NULL`
  - `latency_ms INTEGER`
  - `updated_at INTEGER NOT NULL`
  - PK `(profile_id, proxy_name, ip, target_url)`

- `sessions`
  - `profile_id TEXT NOT NULL`
  - `session_id TEXT NOT NULL`
  - `listen TEXT NOT NULL`
  - `port INTEGER NOT NULL`
  - `selected_ip TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - PK `(profile_id, session_id)`

## Rollout

- SQLite `open()` 自动 `create_if_missing`。
- `probe_records` 支持从旧版主键 `(profile_id, ip, target_url)` 迁移到新版主键（新增 `proxy_name`）。
