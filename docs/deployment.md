# Deployment

## Auth Model

`proxy-broker` treats Forward Auth as an identity source, not as the final authorization layer.

- Human users are identified from forwarded headers.
- The service decides whether that human is an admin by checking `PROXY_BROKER_AUTH_ADMIN_USERS` and `PROXY_BROKER_AUTH_ADMIN_GROUPS`.
- Machine callers authenticate with profile-scoped API keys issued by the service itself.
- `development` mode bypasses forwarded headers and injects a fixed local admin principal.

The default mode is `enforce`.

## Auth Configuration

- `PROXY_BROKER_LISTEN_ADDR`
  - default: `127.0.0.1:8080`
- `PROXY_BROKER_SESSION_LISTEN_IP`
  - default: `127.0.0.1`
- `PROXY_BROKER_STORE`
  - default: `sqlite`
- `PROXY_BROKER_SQLITE_PATH`
  - default: `.proxy-broker/state.sqlite`
- `PROXY_BROKER_RUNTIME_DIR`
  - default: `.proxy-broker/runtime`
- `PROXY_BROKER_DATA_DIR`
  - default: `.proxy-broker/data`
- `PROXY_BROKER_MIHOMO_AUTO_DOWNLOAD`
  - default: `true`
- `PROXY_BROKER_AUTH_MODE=enforce|development`
- `PROXY_BROKER_AUTH_SUBJECT_HEADERS`
  - default: `X-Forwarded-User,X-Auth-Request-User,Remote-User`
- `PROXY_BROKER_AUTH_EMAIL_HEADERS`
  - default: `X-Forwarded-Email,X-Auth-Request-Email`
- `PROXY_BROKER_AUTH_GROUPS_HEADERS`
  - default: `X-Forwarded-Groups,X-Auth-Request-Groups`
- `PROXY_BROKER_AUTH_TRUSTED_PROXIES`
  - default: `127.0.0.1/32,::1/128`
  - forwarded human identity headers are only accepted from these proxy peer IPs
- `PROXY_BROKER_AUTH_ADMIN_USERS`
  - comma-separated exact subject matches
- `PROXY_BROKER_AUTH_ADMIN_GROUPS`
  - comma-separated exact group matches
- `PROXY_BROKER_AUTH_DEV_USER`
  - default: `dev@local`
- `PROXY_BROKER_AUTH_DEV_EMAIL`
  - default: `dev@local`
- `PROXY_BROKER_AUTH_DEV_GROUPS`
  - default: `proxy-broker-dev-admin`

## Local Development

Use explicit development mode if you want to open the embedded UI without running a real Forward Auth chain:

```bash
cargo run -- \
  --store sqlite \
  --sqlite-path .proxy-broker/state.sqlite \
  --listen 127.0.0.1:8080 \
  --session-listen-ip 127.0.0.1 \
  --auth-mode development
```

In this mode:

- the service always resolves the caller as the configured development user,
- that user is always treated as an admin,
- `/`, static assets, and admin APIs work locally without forwarded headers.

## Traefik Forward Auth

When you deploy behind Traefik, configure Forward Auth to populate the identity headers that `proxy-broker` expects. The intent is identity enrichment only: Traefik should forward any headers the auth service can derive, while `proxy-broker` remains the component that returns the final `401` or `403`.

Example dynamic configuration:

```yaml
http:
  middlewares:
    proxy-broker-forward-auth:
      forwardAuth:
        address: http://forward-auth:4181/auth
        trustForwardHeader: true
        authResponseHeaders:
          - X-Forwarded-User
          - X-Forwarded-Email
          - X-Forwarded-Groups
```

Example container environment:

```bash
PROXY_BROKER_AUTH_MODE=enforce
PROXY_BROKER_AUTH_TRUSTED_PROXIES=10.0.0.5/32
PROXY_BROKER_AUTH_ADMIN_USERS=admin@example.com
PROXY_BROKER_AUTH_ADMIN_GROUPS=proxy-broker-admins
```

In this setup:

- Forward Auth identifies the human user.
- `proxy-broker` only trusts those forwarded identity headers when the request
  arrives from a peer IP inside `PROXY_BROKER_AUTH_TRUSTED_PROXIES`.
- `proxy-broker` decides whether that user is an admin.
- Non-admin humans can call `/api/v1/auth/me`, but cannot access the UI or admin APIs.
- If the caller has no forwarded identity, `proxy-broker` returns `401 authentication_required`.

## Reference Compose Stack

The repository ships a reusable reference stack under `deploy/forward-auth/`:

- `compose.yaml`
  - production-style example that pulls `ghcr.io/ivanli-cn/proxy-broker:latest`
  - Traefik terminates TLS and applies Forward Auth as an identity-header enricher.
  - `proxy-broker` is configured through `PROXY_BROKER_*` environment variables rather than a compose `command:` list.
  - consumes `FORWARD_AUTH_SUBNET` and `FORWARD_AUTH_TRAEFIK_IP` from the rendered env file so `proxy-broker` can trust only the Traefik hop without hard-coding a shared subnet.
- `compose.build.yaml`
  - workspace validation override that builds the current checkout instead of pulling GHCR
- `authelia/users_database.yml`
  - static smoke-only users for admin and non-admin coverage.
- `generated/`
  - rendered at runtime by the helper scripts and ignored from git.

The rendered Traefik topology exposes four hosts on a shared test domain:

- `auth.<domain>`
  - Authelia portal
- `broker.<domain>`
  - human-facing route where Traefik forwards any session-derived identity headers
- `broker-basic.<domain>`
  - smoke-only route where Traefik forwards any HTTP Basic-derived identity
    headers through the same Authelia `forward-auth` endpoint
- `machine-broker.<domain>`
  - machine-facing route with no proxy-side human auth so profile API keys can
    reach `proxy-broker` directly

Session listeners are separate raw TCP entrypoints. They do not ride through the
HTTPS web domain and should be reached through the broker host/IP that exposes
the listener port range.

The reusable scripts are:

- `scripts/forward-auth/render-stack.sh`
  - generates TLS material plus Authelia and Traefik config
- `scripts/forward-auth/run-stack-smoke.sh`
  - starts either the `build` or `ghcr` compose variant and runs the smoke test
- `scripts/forward-auth/run-shared-testbox.sh`
  - syncs the repository to `codex-testbox`, applies the LXC-safe compose caps
    override, runs the selected variant, and cleans up by default

Shared-testbox example:

```bash
./scripts/forward-auth/run-shared-testbox.sh
```

Published-image example:

```bash
./scripts/forward-auth/render-stack.sh
docker compose \
  --env-file deploy/forward-auth/generated/stack.env \
  -f deploy/forward-auth/compose.yaml \
  up -d
```

Keep the environment running for inspection:

```bash
./scripts/forward-auth/run-shared-testbox.sh --keep-run
```

Validate the published GHCR image instead of the current workspace build:

```bash
./scripts/forward-auth/run-shared-testbox.sh --variant ghcr
```

The Authelia policy for `broker.<domain>` and `broker-basic.<domain>` is rendered in `scripts/forward-auth/render-stack.sh` under `access_control.rules`, and both hosts are set to `bypass`. This is deliberate: Traefik is not allowed to make the access-control decision for `proxy-broker`.
The same render step also allocates a free private `/24` subnet when you do not provide one explicitly, then picks a fixed Traefik IP inside that subnet and passes it through `FORWARD_AUTH_TRAEFIK_IP` and `PROXY_BROKER_AUTH_TRUSTED_PROXIES`.

## Route Matrix

- `/healthz`
  - public
- `/` and embedded static assets
  - admin human or development principal only
- `/api/v1/auth/me`
  - any authenticated principal
- `/api/v1/profiles`
  - admin human or development principal only
- `/api/v1/profiles/{profile_id}/subscriptions/load`
- `/api/v1/profiles/{profile_id}/refresh`
- `/api/v1/profiles/{profile_id}/nodes/query`
- `/api/v1/profiles/{profile_id}/nodes/export`
- `/api/v1/profiles/{profile_id}/nodes/open-sessions`
- `/api/v1/profiles/{profile_id}/ips/extract`
- `/api/v1/profiles/{profile_id}/ips/options/search`
- `/api/v1/profiles/{profile_id}/sessions`
- `/api/v1/profiles/{profile_id}/sessions/open`
- `/api/v1/profiles/{profile_id}/sessions/open-batch`
- `/api/v1/profiles/{profile_id}/sessions/suggested-port`
- `/api/v1/profiles/{profile_id}/sessions/{session_id}`
  - admin human, development principal, or API key bound to that `profile_id`
- `/api/v1/profiles/{profile_id}/api-keys`
- `/api/v1/profiles/{profile_id}/api-keys/{key_id}`
  - admin human or development principal only

## Machine Access

Profile API keys can be sent either as a bearer token or through `X-API-Key`.

Bearer example:

```bash
curl -X POST http://127.0.0.1:8080/api/v1/profiles/default/refresh \
  -H "Authorization: Bearer pbk_<key_id>_<secret>" \
  -H "Content-Type: application/json" \
  -d '{"force":true}'
```

Header example:

```bash
curl http://127.0.0.1:8080/api/v1/profiles/default/sessions \
  -H "X-API-Key: pbk_<key_id>_<secret>"
```

## Session Port Pool

- `PROXY_BROKER_SESSION_PORT_RANGE`
  - optional inclusive `start-end` range that constrains both suggested ports
    and automatic session listener allocation
  - use this when the deployment only exposes a fixed host port pool such as
    `20000-20999`

Profile API keys are limited to their bound `profile_id`. Using a valid key against another profile returns `403 profile_access_denied`.
