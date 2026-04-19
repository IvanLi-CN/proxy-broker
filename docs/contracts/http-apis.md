# HTTP API

## Authentication model

- Human identity comes from configured Forward Auth response headers.
- Machine identity comes from owner-scoped API keys sent as:
  - `Authorization: Bearer pbk_<key_id>_<random>`
  - `X-API-Key: pbk_<key_id>_<random>`
- Generated IDs are opaque short strings:
  - `key_id`: `key-<16 alnum chars>`
  - API key secret random fragment: `<24 alnum chars>` (underscore-safe)
  - `session_id` / `run_id` / `event_id` / `import_id` / `node_id` use fixed prefixes plus a 16-character alnum body
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
  - `api_key_id?`
  - `api_key_owner_subject?`
  - `api_key_profile_scope?`
  - `profile_id?` (compatibility field; only returned for single-profile API keys)
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

## POST /api/v1/proxies/global/subscriptions/load

- Change: Updated
- Auth: admin human or development principal
- Body:
  - `name?`: optional import display name
  - exactly one of:
    - `source.type`: `url|file`
    - `source.value`: `string`
    - `content`: raw Clash-compatible `proxies:` YAML or plain proxy list for one manual node group
- Success:
  - `loaded_proxies`, `distinct_ips`, `warnings`
- Notes:
  - upserts one original import inside the global inventory scope using normalized `source.type + source.value` as the source identity
  - when `content` is used, creates one manual node-group import without auto-sync registration
  - only replaces nodes that belong to the same original import batch
  - other global imports remain untouched
  - rebuilds effective pools for every profile with `use_global_proxies=true`
  - does not create or update profile auto-sync schedules
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `subscription_invalid` (400)
  - `subscription_fetch_failed` (502)

## GET /api/v1/proxy-imports

- Change: New
- Auth: admin human or development principal
- Query:
  - `scope`: `all|global|profile` (defaults to `all`)
  - `profile_id`: required when `scope=profile`
- Success:
  - `items[]`
  - each item contains:
    - `import_id`
    - `name?`
    - `import_kind`: `subscription|single_node`
    - `source_scope`
    - `source_identity`
      - `source_type`
      - `source_value`
    - `allocation_scope`
    - `proxy_count`
    - `distinct_ip_count`
    - `effective_profile_ids[]`
    - `created_at`
    - `updated_at`
- Notes:
  - this is the canonical admin list surface for `/proxies`
  - subscription imports are managed only at the import level, not per node
  - list primary labels should render `name` first and fall back to `import_id`
  - `import_id` is an opaque short string (`imp-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `scope` is invalid or `profile_id` is missing for `scope=profile`

## GET /api/v1/proxies

- Change: Compatibility
- Auth: admin human or development principal
- Query:
  - `scope`: `all|global|profile` (defaults to `all`)
  - `profile_id`: required when `scope=profile`
- Success:
  - `items[]`
  - each item contains:
    - `node_id`
    - `proxy_name`
    - `proxy_type`
    - `server`
    - `resolved_ips[]`
    - `import_id`
    - `source_scope`
    - `allocation_scope`
    - `effective_profile_ids[]`
- Notes:
  - kept for compatibility and internal detail views
  - no longer the primary admin allocation surface
  - `node_id` is an opaque short string (`node-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `scope` is invalid or `profile_id` is missing for `scope=profile`

## PATCH /api/v1/proxy-imports/{import_id}/allocation

- Change: New
- Auth: admin human or development principal
- Body:
  - `allocation_scope`
    - `{ "type": "global" }`
    - `{ "type": "profile", "profile_id": "..." }`
- Success:
  - returns the updated import item with recomputed `effective_profile_ids`
- Notes:
  - reallocates the whole original import batch
  - for `single_node` imports this is equivalent to reassigning that one node
  - only the affected profiles are rebuilt
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `profile_not_found` (404) when the target profile does not exist
  - `proxy_inventory_node_not_found` (404)

## PATCH /api/v1/proxies/{node_id}/allocation

- Change: Compatibility
- Auth: admin human or development principal
- Body:
  - `allocation_scope`
    - `{ "type": "global" }`
    - `{ "type": "profile", "profile_id": "..." }`
- Success:
  - returns the updated inventory item with recomputed `effective_profile_ids`
- Notes:
  - compatibility wrapper around import-level allocation
  - for nodes that belong to a subscription import, the whole original import batch is reallocated
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `profile_not_found` (404) when the target profile does not exist
  - `proxy_inventory_node_not_found` (404)

## DELETE /api/v1/proxy-imports/{import_id}

- Change: New
- Auth: admin human or development principal
- Success:
  - `204 No Content`
- Notes:
  - deletes the whole original import batch and its import-level sync config, if any
  - a later re-import from the same source restores the batch if the upstream still contains it
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `proxy_inventory_node_not_found` (404)

## DELETE /api/v1/proxies/{node_id}

- Change: Compatibility
- Auth: admin human or development principal
- Success:
  - `204 No Content`
- Notes:
  - compatibility wrapper around import-level deletion
  - for nodes that belong to a subscription import, deleting the node deletes the whole original import batch
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `proxy_inventory_node_not_found` (404)

## GET /api/v1/profiles/{profile_id}/proxy-settings

- Change: New
- Auth: admin human or development principal
- Success:
  - `profile_id`
  - `use_global_proxies`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `profile_not_found` (404)

## PATCH /api/v1/profiles/{profile_id}/proxy-settings

- Change: New
- Auth: admin human or development principal
- Body:
  - `use_global_proxies`: `bool`
- Success:
  - `profile_id`
  - `use_global_proxies`
- Notes:
  - toggling the flag rebuilds the effective pool immediately
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `profile_not_found` (404)

## POST /api/v1/profiles/{profile_id}/subscriptions/load

- Change: Updated
- Auth:
  - admin human or development principal
  - API key whose scope allows `{profile_id}`
- Body:
  - `name?`: optional import display name
  - exactly one of:
    - `source.type`: `url|file`
    - `source.value`: `string`
    - `content`: raw Clash-compatible `proxies:` YAML or plain proxy list for one manual node group
- Notes:
  - upserts one original import inside the current profile-local inventory scope and then rebuilds the effective pool for `{profile_id}`
  - only replaces nodes that belong to the same original import batch
  - other profile-local imports in the same profile remain untouched
  - profile-local imports register auto-sync state per `import_id`, so multiple subscriptions can coexist without overwriting each other
  - manual node-group loads do not register auto-sync config and always create a fresh original import batch
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
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
  - `session_id` is an opaque short string (`sess-<16 alnum chars>`)
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
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
  - API key whose scope allows `{profile_id}`
- Success: 204
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `profile_access_denied` (403)

## GET /api/v1/api-keys

- Change: New
- Auth: admin human or development principal
- Success:
  - `api_keys[]`
  - each item contains `key_id`, `name`, `prefix`, `created_by`, `owner_subject`, `profile_scope`, `created_at`, `last_used_at?`, `revoked_at?`
  - `profile_id?` is a compatibility field and only appears when `profile_scope.kind=selected_profiles` with exactly one selected profile
  - `key_id` is an opaque short string (`key-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## POST /api/v1/api-keys

- Change: New
- Auth: admin human or development principal
- Body:
  - `name`: `string`
  - `profile_scope.kind`: `selected_profiles|all_profiles`
  - `profile_scope.profile_ids[]`: required for `selected_profiles`, forbidden for `all_profiles`
- Success:
  - `api_key`
  - `secret`
- Notes:
  - the key owner is always the current `principal.subject`
  - `secret` is only returned on create
  - returned secrets use `pbk_<key_id>_<random>` where `<random>` is a 24-character underscore-safe alnum fragment
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `name` is empty after `trim`
  - `invalid_request` (400) when `profile_scope` is malformed, `selected_profiles` is empty, `all_profiles` includes `profile_ids`, or any referenced profile does not exist

## DELETE /api/v1/api-keys/{key_id}

- Change: New
- Auth: admin human or development principal
- Success: 204
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `api_key_not_found` (404) when the key does not belong to the current owner or does not exist

## GET /healthz

- Change: New
- Success:
  - `status=ok`
