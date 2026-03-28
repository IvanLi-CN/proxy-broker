# HTTP API

## Authentication model

- Human identity comes from configured Forward Auth response headers.
- Machine identity comes from profile-scoped API keys sent as:
  - `Authorization: Bearer pbk_<key_id>_<random>`
  - `X-API-Key: pbk_<key_id>_<random>`
- `development` mode ignores incoming identity headers and forces the configured development principal.
- When a human identity and an API key are both present on the same request, the service rejects the request with `authentication_required` (401).

## GET /

- Change: New
- Auth: admin human or development principal
- Success:
  - Returns the embedded SPA shell (`index.html`)
- Notes:
  - Unknown non-API frontend `GET` routes also fall back to this shell
  - `/api/v1/*` and `/healthz` keep higher priority than the SPA fallback
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## GET /assets/*

- Change: New
- Auth: admin human or development principal
- Success:
  - Returns embedded frontend static assets from the Bun/Vite build output
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## GET /api/v1/auth/me

- Change: New
- Auth:
  - any authenticated human, development principal, or valid API key
- Success:
  - `authenticated`
  - `principal_type`: `human|api_key|development`
  - `subject`
  - `email?`
  - `groups[]`
  - `is_admin`
  - `profile_id?`
  - `api_key_id?`
- Error:
  - `authentication_required` (401)
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)

## GET /api/v1/profiles

- Change: New
- Auth: admin human or development principal
- Success:
  - `profiles[]`
  - Sorted by `profile_id` ascending
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## POST /api/v1/profiles

- Change: New
- Auth: admin human or development principal
- Body:
  - `profile_id`: `string`
- Success:
  - `profile_id`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `profile_id` is empty after `trim`
  - `profile_exists` (409) when the exact `profile_id` already exists

## POST /api/v1/profiles/{profile_id}/subscriptions/load

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Body:
  - `source.type`: `url|file`
  - `source.value`: `string`
- Notes:
  - `source.type=url` is fetched server-side with a compatibility UA fallback
    set, currently trying `Clash.Meta/1.18.3`, `mihomo/1.18.3`, then
    `Clash Verge/1.7.7`
  - The request/response JSON contract does not change when the compatibility
    UA fallback is applied
- Success:
  - `loaded_proxies`, `distinct_ips`, `warnings`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `subscription_invalid` (400) when the upstream returns 2xx but the payload
    is not a supported Clash/Mihomo subscription
  - `subscription_fetch_failed` (502) when the upstream URL is unreachable or
    returns non-2xx

## POST /api/v1/profiles/{profile_id}/refresh

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Body (optional):
  - `force`: `bool`
- Success:
  - `probed_ips`, `geo_updated`, `skipped_cached`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/profiles/{profile_id}/ips/extract

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
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
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `ip_conflict_blacklist` (400)

## POST /api/v1/profiles/{profile_id}/ips/options/search

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Body:
  - `kind`: `country|city|ip`
  - `query`: `string?`
  - `country_codes`: `string[]`
  - `cities`: `string[]`
  - `limit`: `u32?` (defaults to `25`, capped at `100`)
- Success:
  - `items[]`
  - each item contains `value`, `label`, `meta?`
  - `city` item `value`s are opaque selection tokens so duplicate city names can
    stay disambiguated by country
  - `city` results can be filtered by `country_codes`
  - `ip` results can be filtered by `country_codes` and `cities`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/profiles/{profile_id}/sessions/open

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Body:
  - `selection_mode`: `any|geo|ip` (defaults to `any`)
  - `country_codes`: `string[]`
  - `cities`: `string[]`
  - `specified_ips`: `string[]`
  - `excluded_ips`: `string[]`
  - `sort_mode`: `mru|lru` (defaults to `lru`)
  - `desired_port`: `u16?`
- Constraints:
  - `selection_mode=any` only accepts `excluded_ips`, `sort_mode`, and `desired_port`
  - `selection_mode=geo` requires at least one `country_codes` or `cities` entry
  - `selection_mode=ip` requires at least one `specified_ips` entry and rejects geo fields
  - `specified_ips` and `excluded_ips` must not intersect
  - omitting `desired_port` lets the backend auto-allocate a free listener port
  - when `PROXY_BROKER_SESSION_PORT_RANGE` is configured, both auto-allocation
    and explicit `desired_port` must stay inside that inclusive range
- Success:
  - `session_id`, `listen`, `port`, `selected_ip`, `proxy_name`
  - `listen` echoes the configured session listener bind IP (`127.0.0.1` for
    local runs, `0.0.0.0` for wildcard deployments)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `ip_not_found` (404)
  - `ip_conflict_blacklist` (400)

## POST /api/v1/profiles/{profile_id}/sessions/open-batch

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Body:
  - `requests[]`: same shape and constraints as `POST /sessions/open`
- Success:
  - `sessions[]` (empty `requests` returns `sessions=[]` as no-op)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `ip_not_found` (404)
  - `ip_conflict_blacklist` (400)
  - `batch_open_failed` (409), strict rollback for runtime/persist stage failures

## GET /api/v1/profiles/{profile_id}/sessions/suggested-port

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Success:
  - `port`
- Notes:
  - returns the next available listener port suggestion for the profile
  - the port is not reserved; callers must still omit `desired_port` or submit
    a real value when opening the session
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)

## GET /api/v1/profiles/{profile_id}/sessions

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Success:
  - `sessions[]`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)

## DELETE /api/v1/profiles/{profile_id}/sessions/{session_id}

- Change: New
- Auth:
  - admin human or development principal
  - API key bound to `{profile_id}`
- Success: 204
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)

## GET /api/v1/profiles/{profile_id}/api-keys

- Change: New
- Auth: admin human or development principal
- Success:
  - `api_keys[]`
  - each item contains `key_id`, `profile_id`, `name`, `prefix`, `created_by`, `created_at`, `last_used_at?`, `revoked_at?`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `profile_not_found` (404)

## POST /api/v1/profiles/{profile_id}/api-keys

- Change: New
- Auth: admin human or development principal
- Body:
  - `name`: `string`
- Success:
  - `api_key`
  - `secret`
- Notes:
  - `secret` is only returned on create
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `profile_not_found` (404)
  - `invalid_request` (400) when `name` is empty after `trim`

## DELETE /api/v1/profiles/{profile_id}/api-keys/{key_id}

- Change: New
- Auth: admin human or development principal
- Success: 204
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `profile_not_found` (404)
  - `api_key_not_found` (404)

## GET /healthz

- Change: New
- Success:
  - `status=ok`
