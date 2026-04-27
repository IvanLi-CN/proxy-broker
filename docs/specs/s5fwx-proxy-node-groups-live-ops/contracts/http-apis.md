# HTTP APIs

## GET /api/v1/proxy-catalog

- Query:
  - `view=global|profile`
  - `profile_id` required when `view=profile`
- Success:
  - grouped imports with node rows, node metadata summary, and capability flags
  - node metadata includes legacy `last_probe_samples` and `recent_probe_samples[]`
- `recent_probe_samples[]` item shape:
  - `node_id`
  - `ip`
  - `target_url`
  - `ok`
  - `latency_ms?`
  - `sampled_at`

## POST /api/v1/proxy-ops/refresh

- Body:
  - `view=global|profile`
  - `profile_id?`
  - `node_ids[]`
- Success:
  - `202 Accepted`
  - `run_id`

## POST /api/v1/proxy-ops/probe

- Body:
  - `view=global|profile`
  - `profile_id?`
  - `node_ids[]`
- Success:
  - `202 Accepted`
  - `run_id`
- Notes:
  - per-node breadth-first five rounds; final median latency comes from successful samples only
  - queued/running nodes already covered by another probe are ignored
  - if every requested node is ignored, the accepted `run_id` points to a terminal `skipped` run
  - each completed sample is persisted immediately and becomes visible through `recent_probe_samples`

## POST /api/v1/profiles/{profile_id}/sessions/open-by-node

- Body:
  - `node_id`
  - `desired_port?`
- Success:
  - `session_id`
  - `listen`
  - `port`
  - `selected_ip`
  - `proxy_name`
  - `node_id`

## POST /api/v1/profiles/{profile_id}/sessions/open-batch-by-node

- Body:
  - `node_ids[]`
- Success:
  - `sessions[]` with the same shape as single-open-by-node

## GET /api/v1/system-settings

- Auth:
  - admin only
- Success:
  - `proxy_probe_interval_sec`
  - `updated_at`
- Notes:
  - default response uses `proxy_probe_interval_sec=3600` before a persisted setting exists

## PATCH /api/v1/system-settings

- Auth:
  - admin only
- Body:
  - `proxy_probe_interval_sec`
- Validation:
  - minimum `60`
- Success:
  - persisted settings with `proxy_probe_interval_sec`
  - `updated_at`
