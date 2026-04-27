# Rust API delta

- `MihomoRuntime` becomes a single shared runtime/controller facade
- `BrokerService` adds:
  - `list_proxy_catalog(view, profile_id)`
  - `start_proxy_metadata_refresh(view, profile_id, node_ids)`
  - `start_proxy_latency_probe(view, profile_id, node_ids)`
  - `open_session_by_node(profile_id, node_id, desired_port)`
  - `open_batch_by_node(profile_id, node_ids)`
  - `get_system_settings()`
  - `update_system_settings(proxy_probe_interval_sec)`
- `BrokerStore` adds:
  - `insert_proxy_node_probe_samples(records)`
  - `list_recent_proxy_node_probe_samples(limit_per_node_ip)`
  - `get_system_settings()`
  - `upsert_system_settings(settings)`
- `BrokerService::enqueue_due_tasks()` includes the global proxy latency probe scheduler. It reads `proxy_probe_interval_sec`, targets subscription imports only, skips when a global probe is pending/running, and uses checked interval arithmetic.
- `BrokerService::start_proxy_latency_probe()` filters queued/running duplicate node IDs before enqueueing; an all-duplicate request returns a terminal skipped run.
