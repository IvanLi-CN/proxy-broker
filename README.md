# proxy-broker

`proxy-broker` is a standalone Rust service for:

- loading mihomo subscriptions from URL or file sources,
- building an IP-oriented pool with probe and geo metadata,
- opening and closing local proxy listener sessions by selected IP,
- serving an embedded Bun-built operator web console from the same binary,
- enforcing human admin access with Forward Auth identity headers and profile-scoped API keys.

The service binds `127.0.0.1` by default, session listeners also default to
`127.0.0.1`, and it exposes REST endpoints under `/api/v1`, a health probe at
`/healthz`, and the SPA shell at `/`.

Authentication defaults to `enforce`, which means the UI and protected APIs
expect either:

- a forwarded human identity that resolves to an admin user or admin group, or
- a valid profile-scoped machine API key for profile business endpoints.

## Start the service

```bash
cargo run -- \
  --store sqlite \
  --sqlite-path .proxy-broker/state.sqlite \
  --listen 127.0.0.1:8080 \
  --session-listen-ip 127.0.0.1 \
  --auth-mode development
```

For local UI work, `--auth-mode development` is the simplest option because it
injects a fixed local admin principal. If you omit it, the default `enforce`
mode requires forwarded identity headers for human access.

For container or remote-host deployments, publish both the HTTP service and
session listeners on wildcard binds:

```bash
docker run --rm -p 8080:8080 ghcr.io/ivanli-cn/proxy-broker:latest
```

The published container image defaults to:

- `PROXY_BROKER_LISTEN_ADDR=0.0.0.0:8080`
- `PROXY_BROKER_SESSION_LISTEN_IP=0.0.0.0`
- `PROXY_BROKER_AUTH_MODE=enforce`

The binary now accepts both CLI flags and `PROXY_BROKER_*` environment
variables for runtime configuration. For containers, prefer environment
variables over `command:` argument lists.

## Authentication

Human callers are identified from configurable Forward Auth response headers:

- `PROXY_BROKER_AUTH_SUBJECT_HEADERS`
- `PROXY_BROKER_AUTH_EMAIL_HEADERS`
- `PROXY_BROKER_AUTH_GROUPS_HEADERS`
- `PROXY_BROKER_AUTH_TRUSTED_PROXIES`

Forwarded human identity headers are ignored unless the TCP peer IP matches
`PROXY_BROKER_AUTH_TRUSTED_PROXIES`. The default trusts loopback only, so
container or reverse-proxy deployments must set this explicitly for the proxy
hop that injects the headers.

Admin access is decided inside the application:

- `PROXY_BROKER_AUTH_ADMIN_USERS`
- `PROXY_BROKER_AUTH_ADMIN_GROUPS`

Development mode uses:

- `PROXY_BROKER_AUTH_DEV_USER`
- `PROXY_BROKER_AUTH_DEV_EMAIL`
- `PROXY_BROKER_AUTH_DEV_GROUPS`

Machine callers use profile-scoped API keys issued through:

- `POST /api/v1/profiles/{profile_id}/api-keys`

Then call profile business endpoints with either:

- `Authorization: Bearer pbk_<key_id>_<random>`
- `X-API-Key: pbk_<key_id>_<random>`

See [docs/deployment.md](docs/deployment.md) for the full route matrix, local
development mode, and a Traefik Forward Auth example.

## Forward Auth Smoke Stack

The repository includes a reusable Docker Compose stack for:

- Traefik as the TLS edge and Forward Auth middleware host,
- Authelia as the human identity provider,
- `proxy-broker` as the protected upstream.

Files and scripts:

- `deploy/forward-auth/compose.yaml`
- `deploy/forward-auth/compose.build.yaml`
- `deploy/forward-auth/authelia/users_database.yml`
- `scripts/forward-auth/render-stack.sh`
- `scripts/forward-auth/run-stack-smoke.sh`
- `scripts/forward-auth/run-shared-testbox.sh`

Shared-testbox validation:

```bash
./scripts/forward-auth/run-shared-testbox.sh
```

This script syncs the repo to `codex-testbox`, renders the stack, starts the
compose project with the LXC-safe caps override, runs the smoke checks, and
cleans up by default. It uses the build override by default so the current
workspace changes are validated.

Published-image example:

```bash
./scripts/forward-auth/render-stack.sh
docker compose \
  --env-file deploy/forward-auth/generated/stack.env \
  -f deploy/forward-auth/compose.yaml \
  up -d
```

The helper renders a per-run private subnet and a fixed Traefik IP inside that
subnet, then wires `PROXY_BROKER_AUTH_TRUSTED_PROXIES=<traefik-ip>/32` into the
compose env file. That keeps the trust boundary explicit without risking subnet
collisions on a shared Docker host.

## Web console

Build the frontend before release builds:

```bash
cd web
bun install
bun run build
cd ..
```

Then open the local operator UI from the Rust server:

```bash
open http://127.0.0.1:8080
```

The embedded UI is admin-only. In practice that means:

- local development: run with `--auth-mode development`
- deployed environment: place the service behind Forward Auth and forward the
  identity headers

For frontend-only development, run the Vite app on `127.0.0.1:38181` and proxy
API calls back to the local Rust service:

```bash
cargo run -- \
  --store sqlite \
  --sqlite-path .proxy-broker/state.sqlite \
  --listen 127.0.0.1:8080 \
  --session-listen-ip 127.0.0.1

cd web
bun run dev
```

Storybook runs on `127.0.0.1:38182`:

```bash
cd web
bun run storybook
```

## Health check

```bash
curl http://127.0.0.1:8080/healthz
```

## Current identity

```bash
curl http://127.0.0.1:8080/api/v1/auth/me \
  -H "X-Forwarded-User: admin@example.com" \
  -H "X-Forwarded-Email: admin@example.com" \
  -H "X-Forwarded-Groups: proxy-broker-admins"
```

## REST API

Base path: `http://127.0.0.1:8080/api/v1/profiles/{profile_id}`

### Load subscription

- `POST /subscriptions/load`
- Request body:

```json
{
  "source": {
    "type": "url",
    "value": "https://example.com/subscription.yaml"
  }
}
```

### Refresh probe and geo metadata

- `POST /refresh`
- Optional request body:

```json
{
  "force": true
}
```

### Extract IPs

- `POST /ips/extract`
- Request body:

```json
{
  "country_codes": ["US", "JP"],
  "cities": ["Tokyo"],
  "specified_ips": ["1.2.3.4"],
  "blacklist_ips": ["5.6.7.8"],
  "limit": 20,
  "sort_mode": "lru"
}
```

### Open one session

- `POST /sessions/open`
- Request body:

```json
{
  "selector": {
    "country_codes": ["JP"],
    "limit": 1,
    "sort_mode": "lru"
  },
  "desired_port": 10080
}
```

The response `listen` field reflects the configured session listener bind IP.

### Open sessions in batch

- `POST /sessions/open-batch`

### List sessions

- `GET /sessions`

### Close session

- `DELETE /sessions/{session_id}`

### Issue a profile API key

- `POST /api/v1/profiles/{profile_id}/api-keys`
- Request body:

```json
{
  "name": "deploy-bot"
}
```

### List profile API keys

- `GET /api/v1/profiles/{profile_id}/api-keys`

### Revoke a profile API key

- `DELETE /api/v1/profiles/{profile_id}/api-keys/{key_id}`

### Machine caller example

```bash
curl -X POST http://127.0.0.1:8080/api/v1/profiles/default/refresh \
  -H "Authorization: Bearer pbk_<key_id>_<random>" \
  -H "Content-Type: application/json" \
  -d '{"force":true}'
```

## Validation

```bash
cd web
bun install --frozen-lockfile
bun run check
bun run test
bun run typecheck
bun run build
bun run verify:stories
bun run build-storybook
bun run test-storybook
bun run build
bun run test:e2e
cd ..
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Contracts

- Library API: `docs/contracts/rust-api.md`
- HTTP API: `docs/contracts/http-apis.md`
- SQLite schema: `docs/contracts/db.md`
- Mihomo config payload: `docs/contracts/file-formats.md`
- Web UI spec: `docs/specs/web-admin-ui.md`
