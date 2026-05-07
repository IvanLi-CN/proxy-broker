# DB Contracts

## projects

- `project_id TEXT PRIMARY KEY`
- `created_at INTEGER NOT NULL`

## Compatibility

- `list_projects()` 必须同时覆盖：
  - `projects`
  - `subscription_nodes`
  - `ip_records`
  - `probe_records`
  - `sessions`
- 旧仓库即使没有 `projects` 历史数据，也必须在迁移后保持可读。
