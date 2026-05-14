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
  - `api_key_project_scope?`
  - `project_id?` (compatibility field; only returned for single-project API keys)
- Error:
  - `authentication_required` (401)
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)

## GET /api/v1/projects

- Change: New
- Auth: admin human or development principal
- Success:
  - `projects[]`
  - Sorted by `project_id` ascending
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## POST /api/v1/projects

- Change: New
- Auth: admin human or development principal
- Body:
  - `project_id`: `string`
- Success:
  - `project_id`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `project_id` is empty after `trim`
  - `project_exists` (409) when the exact `project_id` already exists

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
  - `loaded_proxies`
  - `distinct_ips`
  - `resolved_name?`
  - `resolved_name_source?`: `explicit_input|existing_import|parsed_source|generated`
  - `subscription_metadata?`
    - `source_title?`
    - `upload_bytes?`
    - `download_bytes?`
    - `used_bytes?`
    - `total_bytes?`
    - `remaining_bytes?`
    - `expire_at?`
  - `warnings[]`
- Notes:
  - upserts one original import inside the global inventory scope using normalized `source.type + source.value` as the source identity
  - when `content` is used, creates one manual node-group import without auto-sync registration
  - only replaces nodes that belong to the same original import batch
  - other global imports remain untouched
  - source-based imports parse `profile-title`, `Content-Disposition filename/filename*`, URL/file fallback names, and `subscription-userinfo`
  - source-based imports conservatively filter informational pseudo-nodes (for example traffic/expire/notice style names) and expose those drops via `warnings[]`
  - rebuilds effective pools for every project with `use_global_proxies=true`
  - does not create or update project auto-sync schedules
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
  - `scope`: `all|global|project` (defaults to `all`)
  - `project_id`: required when `scope=project`
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
    - `effective_project_ids[]`
    - `subscription_metadata?`
      - `source_title?`
      - `upload_bytes?`
      - `download_bytes?`
      - `used_bytes?`
      - `total_bytes?`
      - `remaining_bytes?`
      - `expire_at?`
    - `created_at`
    - `updated_at`
- Notes:
  - this is the canonical admin list surface for `/proxies`
  - subscription imports are managed only at the import level, not per node
  - list primary labels should render `name` first and fall back to `import_id`
  - when `name` differs from `subscription_metadata.source_title`, clients may show the source title as secondary metadata
  - `import_id` is an opaque short string (`imp-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `scope` is invalid or `project_id` is missing for `scope=project`

## GET /api/v1/proxies

- Change: Compatibility
- Auth: admin human or development principal
- Query:
  - `scope`: `all|global|project` (defaults to `all`)
  - `project_id`: required when `scope=project`
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
    - `effective_project_ids[]`
- Notes:
  - kept for compatibility and internal detail views
  - no longer the primary admin allocation surface
  - `node_id` is an opaque short string (`node-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400) when `scope` is invalid or `project_id` is missing for `scope=project`

## PATCH /api/v1/proxy-imports/{import_id}/allocation

- Change: New
- Auth: admin human or development principal
- Body:
  - `allocation_scope`
    - `{ "type": "global" }`
    - `{ "type": "project", "project_id": "..." }`
- Success:
  - returns the updated import item with recomputed `effective_project_ids`
- Notes:
  - reallocates the whole original import batch
  - for `single_node` imports this is equivalent to reassigning that one node
  - only the affected projects are rebuilt
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `project_not_found` (404) when the target project does not exist
  - `proxy_inventory_node_not_found` (404)

## PATCH /api/v1/proxies/{node_id}/allocation

- Change: Compatibility
- Auth: admin human or development principal
- Body:
  - `allocation_scope`
    - `{ "type": "global" }`
    - `{ "type": "project", "project_id": "..." }`
- Success:
  - returns the updated inventory item with recomputed `effective_project_ids`
- Notes:
  - compatibility wrapper around import-level allocation
  - for nodes that belong to a subscription import, the whole original import batch is reallocated
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `project_not_found` (404) when the target project does not exist
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

## POST /api/v1/proxy-imports/sync

- Change: New
- Auth: admin for global imports; project access for project-local imports
- Body:
  - `import_ids`: non-empty list of import ids
- Success:
  - `run_ids`: queued `subscription_sync` task run ids
- Notes:
  - accepts only source-backed subscription imports where `source_type` is `url` or `file`
  - groups requested imports by source scope before queueing work
  - project-local imports run in their owning project task domain and update existing import-level sync bookkeeping when present
  - global imports run in the global task domain and do not create automatic sync configuration
  - manual node-group imports are rejected because they have no upstream source to refresh
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
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

## GET /api/v1/projects/{project_id}/proxy-settings

- Change: New
- Auth: admin human or development principal
- Success:
  - `project_id`
  - `use_global_proxies`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `project_not_found` (404)

## PATCH /api/v1/projects/{project_id}/proxy-settings

- Change: New
- Auth: admin human or development principal
- Body:
  - `use_global_proxies`: `bool`
- Success:
  - `project_id`
  - `use_global_proxies`
- Notes:
  - toggling the flag rebuilds the effective pool immediately
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)
  - `invalid_request` (400)
  - `project_not_found` (404)

## POST /api/v1/projects/{project_id}/subscriptions/load

- Change: Updated
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Body:
  - `name?`: optional import display name
  - exactly one of:
    - `source.type`: `url|file`
    - `source.value`: `string`
    - `content`: raw Clash-compatible `proxies:` YAML or plain proxy list for one manual node group
- Notes:
  - upserts one original import inside the current project-local inventory scope and then rebuilds the effective pool for `{project_id}`
  - only replaces nodes that belong to the same original import batch
  - other project-local imports in the same project remain untouched
  - project-local imports register auto-sync state per `import_id`, so multiple subscriptions can coexist without overwriting each other
  - manual node-group loads do not register auto-sync config and always create a fresh original import batch
  - `source.type=url` is fetched server-side with a compatibility UA fallback
    set, currently trying `Clash.Meta/1.18.3`, `mihomo/1.18.3`, then
    `Clash Verge/1.7.7`
  - The request/response JSON contract does not change when the compatibility
    UA fallback is applied
- Success:
  - `loaded_proxies`
  - `distinct_ips`
  - `resolved_name?`
  - `resolved_name_source?`: `explicit_input|existing_import|parsed_source|generated`
  - `subscription_metadata?`
    - `source_title?`
    - `upload_bytes?`
    - `download_bytes?`
    - `used_bytes?`
    - `total_bytes?`
    - `remaining_bytes?`
    - `expire_at?`
  - `warnings[]`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `subscription_invalid` (400) when the upstream returns 2xx but the payload
    is not a supported Clash/Mihomo subscription
  - `subscription_fetch_failed` (502) when the upstream URL is unreachable or
    returns non-2xx

## POST /api/v1/projects/{project_id}/refresh

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Body (optional):
  - `force`: `bool`
- Success:
  - `probed_ips`, `geo_updated`, `skipped_cached`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/projects/{project_id}/ips/extract

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
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
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `ip_conflict_blacklist` (400)

## POST /api/v1/projects/{project_id}/ips/options/search

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
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
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed

## POST /api/v1/projects/{project_id}/sessions/open

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
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
  - `session_id`, `listen`, `bind_host`, `display_host`, `display_address`,
    `port`, `selected_ip`, `proxy_name`
  - `listen` is kept for backward compatibility and still returns the bound
    listener endpoint (`<bind_host>:<port>`)
  - `bind_host` is the runtime listener bind host used by mihomo
  - `display_host` / `display_address` are owner-facing values for UI and copy
    actions; wildcard binds (`0.0.0.0` / `::`) resolve through
    `PROXY_BROKER_SESSION_PUBLIC_HOST` when configured, otherwise through the
    current operator-plane hostname
  - `session_id` is an opaque short string (`sess-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `ip_not_found` (404)
  - `ip_conflict_blacklist` (400)

## POST /api/v1/projects/{project_id}/sessions/open-batch

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Body:
  - `requests[]`: same shape and constraints as `POST /sessions/open`
- Success:
  - `sessions[]` (empty `requests` returns `sessions=[]` as no-op)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `ip_not_found` (404)
  - `ip_conflict_blacklist` (400)
  - `batch_open_failed` (409), strict rollback for runtime/persist stage failures

## POST /api/v1/projects/{project_id}/sessions/open-by-node

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Body:
  - `node_id`: opaque node id from the project catalog
  - `desired_port`: `u16?`
- Constraints:
  - `node_id` must belong to the effective proxy inventory for `{project_id}`
  - omitting `desired_port` lets the backend auto-allocate a free listener port
  - when `PROXY_BROKER_SESSION_PORT_RANGE` is configured, both auto-allocation
    and explicit `desired_port` must stay inside that inclusive range
- Success:
  - `session_id`, `listen`, `bind_host`, `display_host`, `display_address`,
    `port`, `selected_ip`, `proxy_name`, `node_id`, `candidate_node_ids`
  - `candidate_node_ids` contains the requested `node_id`
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)
  - `invalid_request` (400) when JSON body is malformed
  - `invalid_port` (400)
  - `port_in_use` (409)
  - `proxy_inventory_node_not_found` (404)
  - `no_healthy_proxy_nodes` (503) when the node has no fresh healthy IP

## POST /api/v1/projects/{project_id}/sessions/open-batch-by-node

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Body:
  - either `node_ids[]`: node ids from the project catalog, opened with auto-assigned ports
  - or `requests[]`: objects with the same shape as `POST /sessions/open-by-node`
  - when `requests[]` is non-empty it takes precedence over `node_ids[]`
- Success:
  - `sessions[]` with the same shape as single open-by-node
- Error:
  - same auth, validation, inventory, port, and health errors as single open-by-node
  - `batch_open_failed` (409), strict rollback for runtime/persist stage failures

## GET /api/v1/projects/{project_id}/sessions/suggested-port

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Success:
  - `port`
- Notes:
  - returns the next available listener port suggestion for the project
  - the port is not reserved; callers must still omit `desired_port` or submit
    a real value when opening the session
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)

## GET /api/v1/projects/{project_id}/sessions

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Success:
  - `sessions[]`
  - each item carries `listen`, `bind_host`, `display_host`, `display_address`,
    `port`, `selected_ip`, `proxy_name`, `node_id`, `created_at`
  - owner-facing UI must use `display_address`; `listen` remains the raw
    backward-compatible bind surface
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)

## DELETE /api/v1/projects/{project_id}/sessions/{session_id}

- Change: New
- Auth:
  - admin human or development principal
  - API key whose scope allows `{project_id}`
- Success: 204
- Error:
  - `authentication_required` (401)
  - `admin_required` (403) for non-admin human callers
  - `api_key_invalid` (401)
  - `api_key_revoked` (401)
  - `project_access_denied` (403)

## GET /api/v1/api-keys

- Change: New
- Auth: admin human or development principal
- Success:
  - `api_keys[]`
  - each item contains `key_id`, `name`, `prefix`, `created_by`, `owner_subject`, `project_scope`, `created_at`, `last_used_at?`, `revoked_at?`
  - `project_id?` is a compatibility field and only appears when `project_scope.kind=selected_projects` with exactly one selected project
  - `key_id` is an opaque short string (`key-<16 alnum chars>`)
- Error:
  - `authentication_required` (401)
  - `admin_required` (403)

## POST /api/v1/api-keys

- Change: New
- Auth: admin human or development principal
- Body:
  - `name`: `string`
  - `project_scope.kind`: `selected_projects|all_projects`
  - `project_scope.project_ids[]`: required for `selected_projects`, forbidden for `all_projects`
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
  - `invalid_request` (400) when `project_scope` is malformed, `selected_projects` is empty, `all_projects` includes `project_ids`, or any referenced project does not exist

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
