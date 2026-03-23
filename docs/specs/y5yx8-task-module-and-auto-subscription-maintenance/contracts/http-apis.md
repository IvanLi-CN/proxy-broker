# HTTP Contracts

## GET /api/v1/tasks

- Auth: admin human or development principal
- Query:
  - `profile_id?`: `string|all`
  - `kind?`: `subscription_sync|metadata_refresh_incremental|metadata_refresh_full`
  - `status?`: `queued|running|succeeded|failed|skipped`
  - `trigger?`: `schedule|post_load`
  - `running_only?`: `bool`
  - `since?`: `unix timestamp seconds`
  - `limit?`: `u32`
  - `cursor?`: `string`
- Success: `200`
- Response:
  - `summary`
  - `runs[]`
  - `next_cursor?`

## GET /api/v1/tasks/{run_id}

- Auth: admin human or development principal
- Success: `200`
- Response:
  - `run`
  - `events[]`

## GET /api/v1/tasks/events

- Auth: admin human or development principal
- Query: 与 `GET /api/v1/tasks` 对齐
- Success: `200`
- Content-Type: `text/event-stream`
- Event types:
  - `snapshot`
  - `run-upsert`
  - `run-event`
  - `summary`
  - `heartbeat`
