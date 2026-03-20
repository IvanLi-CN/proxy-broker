# Forward Auth Stack

This stack is a reusable deployment and smoke-test harness for:

- Traefik as the edge proxy,
- Authelia as the Forward Auth identity provider,
- `proxy-broker` as the protected upstream service.

It is designed for local or shared-testbox validation and intentionally includes a second protected host that sends HTTP Basic credentials through the same Authelia `forward-auth` endpoint so the smoke script can deterministically validate identity forwarding without driving a browser login form.

## Hosts

- `auth.<domain>`
  - Authelia portal
- `broker.<domain>`
  - human-facing `proxy-broker` route protected by the normal Authelia redirect flow
- `broker-basic.<domain>`
  - smoke-only helper route protected by Authelia Forward Auth with HTTP Basic credentials
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
  - validates redirect flow, admin gating, non-admin denial, and machine API-key access
- `scripts/forward-auth/run-stack-smoke.sh`
  - builds the stack, runs smoke tests, and optionally keeps the stack running
- `scripts/forward-auth/run-shared-testbox.sh`
  - syncs the repo to `codex-testbox`, runs the stack there, and cleans up safely by default
