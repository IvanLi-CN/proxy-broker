# Rust API

## Public Traits

- `BrokerStore`
  - `list_projects()`
  - `create_project(project_id, created_at)`
  - `replace_subscription(project_id, nodes)`
  - `apply_subscription_snapshot(project_id, nodes, ip_records, probe_records)`
  - `list_subscription(project_id)`
  - `list_proxy_inventory()`
  - `replace_proxy_inventory_scope(source_scope, nodes)`
  - `list_proxy_imports()`
  - `get_proxy_import(import_id)`
  - `replace_proxy_inventory_import(import_record, nodes)`
  - `get_proxy_inventory_node(node_id)`
  - `list_proxy_inventory_for_import(import_id)`
  - `update_proxy_inventory_allocation(node_id, allocation_scope, updated_at)`
  - `update_proxy_import_allocation(import_id, allocation_scope, updated_at)`
  - `delete_proxy_inventory_node(node_id)`
  - `delete_proxy_import(import_id)`
  - `replace_ip_records(project_id, records)`
  - `upsert_ip_records(project_id, records)`
  - `list_ip_records(project_id)`
  - `replace_probe_records(project_id, records)`
  - `upsert_probe_records(project_id, records)`
  - `list_probe_records(project_id)`
  - `insert_session(project_id, session)`
  - `insert_sessions(project_id, sessions)`
  - `insert_sessions_with_touch(project_id, sessions, last_used_at)`
  - `delete_session(project_id, session_id)`
  - `list_sessions(project_id)`
  - `touch_ip_usage(project_id, ip, last_used_at)`
  - `touch_ip_usages(project_id, ips, last_used_at)`
  - `upsert_proxy_import_sync_config(config)`
  - `get_proxy_import_sync_config(import_id)`
  - `list_proxy_import_sync_configs()`
  - `list_proxy_import_sync_configs_for_project(project_id)`
  - `delete_proxy_import_sync_config(import_id)`
  - legacy compatibility wrappers:
    - `upsert_project_sync_config(config)`
    - `get_project_sync_config(project_id)`
    - `list_project_sync_configs()`

- `MihomoRuntime`
  - `ensure_started(project_id)`
  - `shutdown_project(project_id)`
  - `controller_meta(project_id) -> (controller_addr, secret)`
  - `controller_addr(project_id)`
  - `apply_config(project_id, payload_yaml)`
  - `measure_proxy_delay(project_id, proxy_name, url, timeout_ms)`

## Service Facade

- `BrokerService`
  - `reconcile_startup_sessions()`
  - `list_projects()`
  - `create_project(project_id)`
  - `load_subscription(project_id, source)`
  - `load_subscription_request(project_id, request)`
  - `refresh(project_id, request)`
  - `extract_ips(project_id, request)`
  - `open_session(project_id, request)`
  - `open_batch(project_id, request)`
  - `list_sessions(project_id)`
  - `close_session(project_id, session_id)`

- `BrokerService::refresh(project_id, request)` updates legacy project IP/probe records and backfills effective inventory `(node_id, ip)` metadata so node catalog and session candidate queries keep geo/probe summaries after upgrades.
- Session candidate queries use node-level metadata first and fall back to legacy project `ip_records` / `probe_records` when node-level metadata is absent.


- `BrokerService`
  - `load_global_subscription(source)`
  - `load_global_subscription_request(request)`
  - `list_proxy_imports(scope, project_id)`
  - `list_proxy_inventory(scope, project_id)`
  - `sync_proxy_imports(import_ids)`
  - `update_proxy_import_allocation(import_id, allocation_scope)`
  - `refresh_proxy_import(import_id)`
  - `update_proxy_allocation(node_id, allocation_scope)`
  - `delete_proxy_import(import_id)`
  - `delete_proxy_inventory_node(node_id)`
  - `get_project_proxy_settings(project_id)`
  - `update_project_proxy_settings(project_id, use_global_proxies)`
  - `load_subscription(project_id, source)` now upserts one project-local original import and rebuilds the effective pool instead of treating the upstream result as the final pool directly
  - `load_global_subscription(source)` now upserts one global original import instead of replacing the entire global scope
  - `refresh_proxy_import(import_id)` reloads an existing subscription import from its persisted source identity without requiring a sync config; project-local imports keep existing sync config enabled state, intervals, and due times registered

## Key data contracts

- `ProxyImportRecord`
  - internal persisted import batch metadata
  - fields: `import_id`, `name`, `import_kind`, `source_scope`, `source_identity`, `allocation_scope`, `subscription_metadata?`, `created_at`, `updated_at`
  - `subscription_metadata` is the import-level snapshot for source-derived title / quota / expiry metadata

- `ProxyImportItem`
  - public API projection for `/api/v1/proxy-imports`
  - fields: `import_id`, `name`, `import_kind`, `source_scope`, `source_identity`, `allocation_scope`, `proxy_count`, `distinct_ip_count`, `effective_project_ids`, `subscription_metadata?`, `created_at`, `updated_at`

- `ProxyInventoryRecord` / `ProxyInventoryItem`
  - retain node-level detail for compatibility
  - now include `import_id` so nodes can be traced back to the original import batch

- `ProxyImportSyncConfig`
  - import-level auto-sync state for project-local subscription imports
  - keyed by `import_id`

- `SyncProxyImportsRequest` / `SyncProxyImportsResponse`
  - request carries `import_ids[]`
  - response returns queued `subscription_sync` task `run_ids[]`
  - manual sync accepts only source-backed subscription imports

- `LoadSubscriptionRequest`
  - fields: `name?`, `source?`, `content?`
  - exactly one of `source` or `content` must be present
  - `content` carries one manual node-group import payload and skips auto-sync registration

- `LoadSubscriptionResponse`
  - fields: `loaded_proxies`, `distinct_ips`, `resolved_name?`, `resolved_name_source?`, `subscription_metadata?`, `warnings[]`
  - `resolved_name_source` follows the fixed precedence:
    - `explicit_input`
    - `existing_import`
    - `parsed_source`
    - `generated`
  - source-based imports parse `profile-title`, `Content-Disposition filename/filename*`, URL/file fallback names, and `subscription-userinfo`
  - source-based imports also filter informational pseudo-node names before inventory persistence and report those drops through `warnings[]`
