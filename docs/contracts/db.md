# 数据库（DB）

## Schema (SQLite)

- `projects`
  - `project_id TEXT PRIMARY KEY`
  - `created_at INTEGER NOT NULL`

- `subscription_nodes`
  - `project_id TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `proxy_type TEXT NOT NULL`
  - `server TEXT NOT NULL`
  - `resolved_ips_json TEXT NOT NULL`
  - `raw_proxy_json TEXT NOT NULL`
  - PK `(project_id, proxy_name)`

- `ip_records`
  - `project_id TEXT NOT NULL`
  - `ip TEXT NOT NULL`
  - `country_code TEXT`
  - `country_name TEXT`
  - `region_name TEXT`
  - `city TEXT`
  - `geo_source TEXT`
  - `probe_updated_at INTEGER`
  - `geo_updated_at INTEGER`
  - `last_used_at INTEGER`
  - PK `(project_id, ip)`

- `probe_records`
  - `project_id TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `ip TEXT NOT NULL`
  - `target_url TEXT NOT NULL`
  - `ok INTEGER NOT NULL`
  - `latency_ms INTEGER`
  - `updated_at INTEGER NOT NULL`
  - PK `(project_id, proxy_name, ip, target_url)`

- `sessions`
  - `project_id TEXT NOT NULL`
  - `session_id TEXT NOT NULL`
  - `listen TEXT NOT NULL`
  - `port INTEGER NOT NULL`
  - `selected_ip TEXT NOT NULL`
  - `proxy_name TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - PK `(project_id, session_id)`
  - generated `session_id` values are opaque short strings: `sess-<16 alnum chars>`

- `proxy_inventory_nodes`
  - `node_id TEXT PRIMARY KEY`
  - `import_id TEXT NOT NULL`
  - `source_scope_kind TEXT NOT NULL` (`global|project`)
  - `source_scope_project_id TEXT`
  - `source_type TEXT NOT NULL`
  - `source_value TEXT NOT NULL`
  - `allocation_scope_kind TEXT NOT NULL` (`global|project`)
  - `allocation_scope_project_id TEXT`
  - `proxy_name TEXT NOT NULL`
  - `proxy_type TEXT NOT NULL`
  - `server TEXT NOT NULL`
  - `resolved_ips_json TEXT NOT NULL`
  - `raw_proxy_json TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - indexes on `(import_id)`, `(source_scope_kind, source_scope_project_id)` and `(allocation_scope_kind, allocation_scope_project_id)`
  - unique index on `(import_id, proxy_name)`
  - generated `node_id` values are deterministic short strings: `node-<16 alnum chars>`

- `proxy_imports`
  - `import_id TEXT PRIMARY KEY`
  - `name TEXT`
  - `import_kind TEXT NOT NULL` (`subscription|single_node`)
  - `source_scope_type TEXT NOT NULL` (`global|project`)
  - `source_scope_project_id TEXT`
  - `source_type TEXT NOT NULL`
  - `source_value TEXT NOT NULL`
  - `allocation_scope_type TEXT NOT NULL` (`global|project`)
  - `allocation_scope_project_id TEXT`
  - `source_title TEXT`
  - `upload_bytes INTEGER`
  - `download_bytes INTEGER`
  - `used_bytes INTEGER`
  - `total_bytes INTEGER`
  - `remaining_bytes INTEGER`
  - `expire_at INTEGER`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - indexes on `(source_scope_type, source_scope_project_id)` and `(allocation_scope_type, allocation_scope_project_id)`
  - generated `import_id` values are opaque short strings: stable sources use deterministic `imp-<16 alnum chars>`, manual imports use a fresh random `imp-<16 alnum chars>`

- `proxy_import_sync_configs`
  - `import_id TEXT PRIMARY KEY`
  - `project_id TEXT NOT NULL`
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
  - index on `(project_id)`
  - `import_id` follows the same short-ID contract as `proxy_imports.import_id`

- `project_proxy_settings`
  - `project_id TEXT PRIMARY KEY`
  - `use_global_proxies INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`

- `api_keys`
  - `key_id TEXT PRIMARY KEY`
  - `name TEXT NOT NULL`
  - `secret_prefix TEXT NOT NULL`
  - `secret_salt TEXT NOT NULL`
  - `secret_hash TEXT NOT NULL`
  - `created_by_subject TEXT NOT NULL`
  - `scope_kind TEXT NOT NULL`
  - `created_at INTEGER NOT NULL`
  - `last_used_at INTEGER`
  - generated `key_id` values are opaque short strings: `key-<16 alnum chars>`
  - `revoked_at INTEGER`
  - unique index on `secret_hash`

- `api_key_projects`
  - `key_id TEXT NOT NULL`
  - `project_id TEXT NOT NULL`
  - PK `(key_id, project_id)`
  - index on `project_id`

## Rollout

- SQLite `open()` 自动 `create_if_missing`。
- `projects` 表用于持久化空 project，使其在尚无业务数据时仍能被重新列出。
- `probe_records` 支持从旧版主键 `(project_id, ip, target_url)` 迁移到新版主键（新增 `proxy_name`）。
- `api_keys` 只保存 `secret_prefix`、`secret_salt` 与 `secret_hash`，不保存明文 secret。
- `api_keys.created_by_subject` 是 API Key 的 canonical owner；`scope_kind + api_key_projects` 共同表示授权范围。
- 历史单 project API Key 会在迁移时自动回填为 `scope_kind=selected_projects`，并在 `api_key_projects` 中补一条 legacy `project_id` 关联。
- 旧 UUID 形状的 `sessions` / `task_runs` / `task_run_events` / `proxy_imports` / `proxy_inventory_nodes` / `proxy_import_sync_configs` 会在升级时一次性改写到新短 ID；历史 API keys 会在升级时被清空并要求重发。
- `last_used_at` 在成功完成 API Key 认证后更新，`revoked_at` 用于软撤销。

- `proxy_imports` 是原始导入批次的真相源；`proxy_inventory_nodes` 是导入批次下的节点明细层，并通过 `import_id` 关联。
- `proxy_imports.name` 保存导入显示名称；允许为空，前端或服务端可在创建时生成名称，列表显示时再回退到 `import_id`。
- source-based imports additionally persist one subscription metadata snapshot in `proxy_imports` (`source_title`, upload/download/used/total/remaining bytes, `expire_at`); historical imports may keep all of these columns `NULL`.
- 订阅内节点唯一性从 `scope + proxy_name` 改成 `(import_id, proxy_name)`，允许同一 scope 下多个订阅并存且包含同名节点。
- `single_node` kind 现在表示手动节点组导入：一次提交的一个或多个节点作为同一个原始导入批次保存。
- `proxy_import_sync_configs` 以 `import_id` 为键保存 project-local 订阅的自动同步状态，取代“每个 project 只有一个 source”的旧模型；旧数据会在迁移时回填成 import 级记录。
- `proxy_inventory_nodes` 保留 `source_type/source_value` 兼容列，用于从历史库存回填 import 记录与跨版本迁移。
- `subscription_nodes` / `ip_records` / `probe_records` / `sessions` 继续作为按 project 物化后的 effective snapshot。
- `project_proxy_settings` 让 existing / new projects 都能持久化 `use_global_proxies`，默认值为启用。
