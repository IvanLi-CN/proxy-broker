# DB delta

- `sessions` adds `node_id`
- `sessions` adds `candidate_node_ids` JSON text; legacy rows default to `[node_id]` when a node is known and to `[]` only when the row cannot be backfilled yet
- new node/IP metadata persistence keyed by `(node_id, ip)` for geo + probe summaries
- existing profile `ip_records` / `probe_records` remain for legacy compatibility and best-effort backfill
