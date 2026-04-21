# HTTP APIs

## GET /api/v1/proxy-catalog

- Query:
  - `view=global|profile`
  - `profile_id` required when `view=profile`
- Success:
  - grouped imports with node rows, node metadata summary, and capability flags

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
