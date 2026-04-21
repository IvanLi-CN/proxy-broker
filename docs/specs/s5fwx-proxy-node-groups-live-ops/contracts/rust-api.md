# Rust API delta

- `MihomoRuntime` becomes a single shared runtime/controller facade
- `BrokerService` adds:
  - `list_proxy_catalog(view, profile_id)`
  - `start_proxy_metadata_refresh(view, profile_id, node_ids)`
  - `start_proxy_latency_probe(view, profile_id, node_ids)`
  - `open_session_by_node(profile_id, node_id, desired_port)`
  - `open_batch_by_node(profile_id, node_ids)`
