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

- `proxy_inventory_nodes`
  - `node_id TEXT PRIMARY KEY`
  - `import_id TEXT NOT NULL`
  - `source_scope_kind TEXT NOT NULL` (`global|profile`)
  - `source_scope_profile_id TEXT`
  - `source_type TEXT NOT NULL`
  - `source_value TEXT NOT NULL`
  - `allocation_scope_kind TEXT NOT NULL` (`global|profile`)
  - `allocation_scope_profile_id TEXT`
  - `proxy_name TEXT NOT NULL`
  - `proxy_type TEXT NOT NULL`
  - `server TEXT NOT NULL`
  - `resolved_ips_json TEXT NOT NULL`
  - `raw_proxy_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - indexes on `(import_id)`, `(source_scope_kind, source_scope_profile_id)` and `(allocation_scope_kind, allocation_scope_profile_id)`
  - unique index on `(import_id, proxy_name)`

- `proxy_imports`
  - `import_id TEXT PRIMARY KEY`
  - `name TEXT`
  - `import_kind TEXT NOT NULL` (`subscription|single_node`)
  - `source_scope_type TEXT NOT NULL` (`global|profile`)
  - `source_scope_profile_id TEXT`
  - `source_type TEXT NOT NULL`
  - `source_value TEXT NOT NULL`
  - `allocation_scope_type TEXT NOT NULL` (`global|profile`)
  - `allocation_scope_profile_id TEXT`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - indexes on `(source_scope_type, source_scope_profile_id)` and `(allocation_scope_type, allocation_scope_profile_id)`

- `proxy_import_sync_configs`
  - `import_id TEXT PRIMARY KEY`
  - `profile_id TEXT NOT NULL`
  - `source_type TEXT NOT NULL`
  - `source_value TEXT NOT NULL`
  - `enabled INTEGER NOT NULL`
  - `sync_every_sec INTEGER NOT NULL`
  - `full_refresh_every_sec INTEGER NOT NULL`
  - `last_sync_due_at INTEGER`
  - `last_sync_started_at INTEGER`
  - `last_sync_finished_at INTEGER`
  - `last_full_refresh_due_at INTEGER`
  - `last_full_refresh_started_at INTEGER`
  - `last_full_refresh_finished_at INTEGER`
  - `updated_at INTEGER NOT NULL`
  - index on `(profile_id)`

- `profile_proxy_settings`
  - `profile_id TEXT PRIMARY KEY`
  - `use_global_proxies INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`

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

- `proxy_imports` 是原始导入批次的真相源；`proxy_inventory_nodes` 是导入批次下的节点明细层，并通过 `import_id` 关联。
- `proxy_imports.name` 保存导入显示名称；允许为空，前端或服务端可在创建时生成名称，列表显示时再回退到 `import_id`。
- 订阅内节点唯一性从 `scope + proxy_name` 改成 `(import_id, proxy_name)`，允许同一 scope 下多个订阅并存且包含同名节点。
- `single_node` kind 现在表示手动节点组导入：一次提交的一个或多个节点作为同一个原始导入批次保存。
- `proxy_import_sync_configs` 以 `import_id` 为键保存 profile-local 订阅的自动同步状态，取代“每个 profile 只有一个 source”的旧模型；旧数据会在迁移时回填成 import 级记录。
- `proxy_inventory_nodes` 保留 `source_type/source_value` 兼容列，用于从历史库存回填 import 记录与跨版本迁移。
- `subscription_nodes` / `ip_records` / `probe_records` / `sessions` 继续作为按 profile 物化后的 effective snapshot。
- `profile_proxy_settings` 让 existing / new profiles 都能持久化 `use_global_proxies`，默认值为启用。
