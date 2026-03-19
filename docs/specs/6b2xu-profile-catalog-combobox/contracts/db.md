# DB Contracts

## profiles

- `profile_id TEXT PRIMARY KEY`
- `created_at INTEGER NOT NULL`

## Compatibility

- `list_profiles()` 必须同时覆盖：
  - `profiles`
  - `subscription_nodes`
  - `ip_records`
  - `probe_records`
  - `sessions`
- 旧仓库即使没有 `profiles` 历史数据，也必须在迁移后保持可读。
