# Forward Auth Stack

This stack is a reusable deployment and smoke-test harness for:

- Traefik as the edge proxy,
- Authelia as the Forward Auth identity enricher,
- `proxy-broker` as the protected upstream service.

It is designed for local or shared-testbox validation and intentionally includes a second protected host that sends HTTP Basic credentials through the same Authelia `forward-auth` endpoint so the smoke script can deterministically validate identity forwarding without driving a browser login form.
The Authelia `access_control` policy for both human-facing broker hosts is `bypass`, so Traefik does not enforce access control for `proxy-broker`; it only forwards any identity headers that Authelia can derive from a session cookie or HTTP Basic credentials.

## Compose Variants

- `compose.yaml`
  - production-style example that uses `ghcr.io/ivanli-cn/proxy-broker:latest`
- `compose.build.yaml`
  - local/shared-testbox override that builds the current workspace instead of
    pulling the published image

## Hosts

- `auth.<domain>`
  - Authelia portal
- `broker.<domain>`
  - human-facing `proxy-broker` route with optional session-derived identity headers
- `broker-basic.<domain>`
  - smoke-only helper route with optional HTTP Basic-derived identity headers
- `machine-broker.<domain>`
  - machine-facing route with no proxy-side auth so `proxy-broker` profile API keys can authenticate directly

## Test Users

- Admin
  - username: `admin`
  - password: `ProxyBrokerAdmin123!`
  - groups: `proxy-broker-admins`
- Viewer
  - username: `viewer`
  - password: `ProxyBrokerViewer123!`
  - groups: `proxy-broker-viewers`

These credentials exist only for the reusable smoke stack and are not suitable for production.

## Scripts

- `scripts/forward-auth/render-stack.sh`
  - renders TLS material and generated Authelia/Traefik config into `deploy/forward-auth/generated/`
- `scripts/forward-auth/smoke.sh`
  - validates app-owned `401/403` behavior, admin gating, non-admin denial, and machine API-key access
- `scripts/forward-auth/run-stack-smoke.sh`
  - runs smoke tests against either the `build` or `ghcr` compose variant
- `scripts/forward-auth/run-shared-testbox.sh`
  - syncs the repo to `codex-testbox`, runs the selected variant there, and cleans up safely by default

## proxy-broker Configuration

Both compose variants configure `proxy-broker` entirely through
`PROXY_BROKER_*` environment variables. They intentionally do not use a compose
`command:` list for application settings.

The stack passes `PROXY_BROKER_AUTH_TRUSTED_PROXIES=<traefik-ip>/32` to
`proxy-broker`. `render-stack.sh` allocates a free private `/24` subnet and a
fixed Traefik IP inside that subnet unless you override
`FORWARD_AUTH_SUBNET` or `FORWARD_AUTH_TRAEFIK_IP` yourself. That keeps the
trust boundary explicit without colliding with other Docker networks on a
shared host.
