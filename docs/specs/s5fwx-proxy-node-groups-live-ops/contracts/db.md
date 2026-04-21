# DB delta

- `sessions` adds `node_id`
- new node/IP metadata persistence keyed by `(node_id, ip)` for geo + probe summaries
- existing profile `ip_records` / `probe_records` remain for legacy compatibility and best-effort backfill
