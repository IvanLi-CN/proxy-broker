# Rust API

## Public Traits

- `BrokerStore`
  - `list_profiles()`
  - `create_profile(profile_id, created_at)`
  - `replace_subscription(profile_id, nodes)`
  - `apply_subscription_snapshot(profile_id, nodes, ip_records, probe_records, removed_session_ids)`
  - `list_subscription(profile_id)`
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
  - `replace_ip_records(profile_id, records)`
  - `upsert_ip_records(profile_id, records)`
  - `list_ip_records(profile_id)`
  - `replace_probe_records(profile_id, records)`
  - `upsert_probe_records(profile_id, records)`
  - `list_probe_records(profile_id)`
  - `insert_session(profile_id, session)`
  - `insert_sessions(profile_id, sessions)`
  - `insert_sessions_with_touch(profile_id, sessions, last_used_at)`
  - `delete_session(profile_id, session_id)`
  - `list_sessions(profile_id)`
  - `touch_ip_usage(profile_id, ip, last_used_at)`
  - `touch_ip_usages(profile_id, ips, last_used_at)`
  - `upsert_proxy_import_sync_config(config)`
  - `get_proxy_import_sync_config(import_id)`
  - `list_proxy_import_sync_configs()`
  - `list_proxy_import_sync_configs_for_profile(profile_id)`
  - `delete_proxy_import_sync_config(import_id)`
  - legacy compatibility wrappers:
    - `upsert_profile_sync_config(config)`
    - `get_profile_sync_config(profile_id)`
    - `list_profile_sync_configs()`

- `MihomoRuntime`
  - `ensure_started(profile_id)`
  - `shutdown_profile(profile_id)`
  - `controller_meta(profile_id) -> (controller_addr, secret)`
  - `controller_addr(profile_id)`
  - `apply_config(profile_id, payload_yaml)`
  - `measure_proxy_delay(profile_id, proxy_name, url, timeout_ms)`

## Service Facade

- `BrokerService`
  - `reconcile_startup_sessions()`
  - `list_profiles()`
  - `create_profile(profile_id)`
  - `load_subscription(profile_id, source)`
  - `load_subscription_request(profile_id, request)`
  - `refresh(profile_id, request)`
  - `extract_ips(profile_id, request)`
  - `open_session(profile_id, request)`
  - `open_batch(profile_id, request)`
  - `list_sessions(profile_id)`
  - `close_session(profile_id, session_id)`


- `BrokerService`
  - `load_global_subscription(source)`
  - `load_global_subscription_request(request)`
  - `list_proxy_imports(scope, profile_id)`
  - `list_proxy_inventory(scope, profile_id)`
  - `update_proxy_import_allocation(import_id, allocation_scope)`
  - `update_proxy_allocation(node_id, allocation_scope)`
  - `delete_proxy_import(import_id)`
  - `delete_proxy_inventory_node(node_id)`
  - `get_profile_proxy_settings(profile_id)`
  - `update_profile_proxy_settings(profile_id, use_global_proxies)`
  - `load_subscription(profile_id, source)` now upserts one profile-local original import and rebuilds the effective pool instead of treating the upstream result as the final pool directly
  - `load_global_subscription(source)` now upserts one global original import instead of replacing the entire global scope

## Key data contracts

- `ProxyImportRecord`
  - internal persisted import batch metadata
  - fields: `import_id`, `name`, `import_kind`, `source_scope`, `source_identity`, `allocation_scope`, `created_at`, `updated_at`

- `ProxyImportItem`
  - public API projection for `/api/v1/proxy-imports`
  - fields: `import_id`, `name`, `import_kind`, `source_scope`, `source_identity`, `allocation_scope`, `proxy_count`, `distinct_ip_count`, `effective_profile_ids`, `created_at`, `updated_at`

- `ProxyInventoryRecord` / `ProxyInventoryItem`
  - retain node-level detail for compatibility
  - now include `import_id` so nodes can be traced back to the original import batch

- `ProxyImportSyncConfig`
  - import-level auto-sync state for profile-local subscription imports
  - keyed by `import_id`

- `LoadSubscriptionRequest`
  - fields: `name?`, `source?`, `content?`
  - exactly one of `source` or `content` must be present
  - `content` carries one manual node-group import payload and skips auto-sync registration
