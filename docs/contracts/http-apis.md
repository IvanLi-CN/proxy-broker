# HTTP API

## GET /

- Change: New
- Auth: none (localhost default)
- Success:
  - Returns the embedded SPA shell (`index.html`)
- Notes:
  - Unknown non-API frontend `GET` routes also fall back to this shell
  - `/api/v1/*` and `/healthz` keep higher priority than the SPA fallback

## GET /assets/*

- Change: New
- Auth: none (localhost default)
- Success:
  - Returns embedded frontend static assets from the Bun/Vite build output

## GET /api/v1/profiles

- Change: New
- Auth: none
- Success:
  - `profiles[]`
  - Sorted by `profile_id` ascending

## POST /api/v1/profiles

- Change: New
- Auth: none
- Body:
  - `profile_id`: `string`
- Success:
  - `profile_id`
- Error:
  - `invalid_request` (400) when `profile_id` is empty after `trim`
  - `profile_exists` (409) when the exact `profile_id` already exists

## POST /api/v1/profiles/{profile_id}/subscriptions/load

- Change: New
- Auth: none (localhost default)
- Body:
  - `source.type`: `url|file`
  - `source.value`: `string`
- Notes:
  - `source.type=url` is fetched server-side with `User-Agent: mihomo/1.18.3`
  - The request/response JSON contract does not change when the compatibility
    UA is applied
- Success:
  - `loaded_proxies`, `distinct_ips`, `warnings`
- Error:
  - `invalid_request` (400) when JSON body is malformed
  - `subscription_invalid` (400) when the upstream returns 2xx but the payload
    is not a supported Clash/Mihomo subscription
  - `subscription_fetch_failed` (502) when the upstream URL is unreachable or
    returns non-2xx

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
  - `listen` echoes the configured session listener bind IP (`127.0.0.1` for
    local runs, `0.0.0.0` for wildcard deployments)
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
