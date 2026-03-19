# 数据库（DB）

## Schema (SQLite)

- `profiles`
  - `profile_id TEXT PRIMARY KEY`
  - `created_at INTEGER NOT NULL`

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

- `api_keys`
  - `key_id TEXT PRIMARY KEY`
  - `profile_id TEXT NOT NULL`
  - `name TEXT NOT NULL`
  - `secret_prefix TEXT NOT NULL`
  - `secret_salt TEXT NOT NULL`
  - `secret_hash TEXT NOT NULL`
  - `created_by_subject TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `last_used_at INTEGER`
  - `revoked_at INTEGER`
  - unique index on `secret_hash`

## Rollout

- SQLite `open()` 自动 `create_if_missing`。
- `profiles` 表用于持久化空 profile，使其在尚无业务数据时仍能被重新列出。
- `probe_records` 支持从旧版主键 `(profile_id, ip, target_url)` 迁移到新版主键（新增 `proxy_name`）。
- `api_keys` 只保存 `secret_prefix`、`secret_salt` 与 `secret_hash`，不保存明文 secret。
- `last_used_at` 在成功完成 API Key 认证后更新，`revoked_at` 用于软撤销。
