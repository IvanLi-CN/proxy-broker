# HTTP API

## POST /api/v1/profiles/{profile_id}/subscriptions/load

- Change: New
- Auth: none (localhost default)
- Body:
  - `source.type`: `url|file`
  - `source.value`: `string`
- Success:
  - `loaded_proxies`, `distinct_ips`, `warnings`
- Error:
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/profiles/{profile_id}/refresh

- Change: New
- Auth: none
- Body (optional):
  - `force`: `bool`
- Success:
  - `probed_ips`, `geo_updated`, `skipped_cached`
- Error:
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/profiles/{profile_id}/ips/extract

- Change: New
- Body:
  - `country_codes`: `string[]`
  - `cities`: `string[]`
  - `specified_ips`: `string[]`
  - `blacklist_ips`: `string[]`
  - `limit`: `u32`
  - `sort_mode`: `mru|lru`
- Success:
  - `items[]` with ip, geo, probe, last_used_at
- Error:
  - `invalid_request` (400) when JSON body is malformed
  - `ip_conflict_blacklist` (400)

## POST /api/v1/profiles/{profile_id}/sessions/open

- Change: New
- Body:
  - `specified_ip`: `string?`
  - `selector`: same shape as extract request
  - `desired_port`: `u16?`
- Success:
  - `session_id`, `listen`, `port`, `selected_ip`, `proxy_name`
- Error:
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/profiles/{profile_id}/sessions/open-batch

- Change: New
- Body:
  - `requests[]`: `OpenSessionRequest`
- Success:
  - `sessions[]` (empty `requests` returns `sessions=[]` as no-op)
- Error:
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `ip_not_found` (404)
  - `ip_conflict_blacklist` (400)
  - `batch_open_failed` (409), strict rollback for runtime/persist stage failures

## GET /api/v1/profiles/{profile_id}/sessions

- Change: New
- Success:
  - `sessions[]`

## DELETE /api/v1/profiles/{profile_id}/sessions/{session_id}

- Change: New
- Success: 204

## GET /healthz

- Change: New
- Success:
  - `status=ok`
