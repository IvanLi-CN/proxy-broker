# Rust API

## Public Traits

- `BrokerStore`
  - `list_profiles()`
  - `create_profile(profile_id, created_at)`
  - `replace_subscription(profile_id, nodes)`
  - `apply_subscription_snapshot(profile_id, nodes, ip_records, probe_records, removed_session_ids)`
  - `list_subscription(profile_id)`
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
  - `refresh(profile_id, request)`
  - `extract_ips(profile_id, request)`
  - `open_session(profile_id, request)`
  - `open_batch(profile_id, request)`
  - `list_sessions(profile_id)`
  - `close_session(profile_id, session_id)`
