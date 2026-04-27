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
  - `candidate_node_ids`

## POST /api/v1/profiles/{profile_id}/sessions/open-batch-by-node

- Body:
  - `node_ids[]`
- Success:
  - `sessions[]` with the same shape as single-open-by-node

## POST /api/v1/profiles/{profile_id}/sessions/ip-node-options/search

- Body:
  - `query?`
  - `group_by=subscription|city`
  - `session_id?`
  - `limit?`
- Success:
  - grouped IP rows with `ip`, grouping labels, usage/latency summaries, and candidate node rows
- Notes:
  - used by `/sessions` create and switch dialogs; rows are limited to the profile effective proxy pool

## POST /api/v1/profiles/{profile_id}/sessions/open-by-ip

- Body:
  - `selected_ip`
  - `candidate_node_ids[]`
  - `desired_port?`
- Success:
  - session response with `selected_ip`, active `node_id`, and persisted `candidate_node_ids`
- Notes:
  - active `node_id` is selected from the candidate set by availability and lowest known median latency

## POST /api/v1/profiles/{profile_id}/sessions/open-batch-by-ip

- Body:
  - `requests[]` where each row matches `open-by-ip`
- Success:
  - `sessions[]` with one session per requested IP

## PATCH /api/v1/profiles/{profile_id}/sessions/{session_id}/node

- Body:
  - preferred: `selected_ip` + `candidate_node_ids[]`
  - compatibility: `node_id`
- Success:
  - session response preserving `session_id`, listener, port, and `created_at`
- Notes:
  - compatibility `node_id` requests are treated as a single-candidate switch
