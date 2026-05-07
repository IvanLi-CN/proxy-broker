# DB delta

- `sessions` adds `node_id`
- `sessions` adds `candidate_node_ids` JSON text; legacy rows default to `[node_id]` when a node is known and to `[]` only when the row cannot be backfilled yet
- new node/IP metadata persistence keyed by `(node_id, ip)` for geo + probe summaries
- `proxy_node_probe_samples` stores individual probe samples:
  - `node_id`
  - `ip`
  - `target_url`
  - `ok`
  - `latency_ms`
  - `sampled_at`
- `proxy_node_probe_samples` retains the newest 10 rows per `(node_id, ip)` and orders history by `sampled_at DESC` plus insertion order.
- `proxy_node_metadata` keeps legacy summary fields and exposes `recent_probe_samples` from the sample table, falling back to legacy `last_probe_samples` when no sample rows exist.
- `system_settings` persists the singleton system configuration payload, currently `proxy_probe_interval_sec` with `updated_at`.
- existing project `ip_records` / `probe_records` remain for legacy compatibility and best-effort backfill
