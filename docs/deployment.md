# Deployment

## Auth Model

`proxy-broker` treats Forward Auth as an identity source, not as the final authorization layer.

- Human users are identified from forwarded headers.
- The service decides whether that human is an admin by checking `PROXY_BROKER_AUTH_ADMIN_USERS` and `PROXY_BROKER_AUTH_ADMIN_GROUPS`.
- Machine callers authenticate with profile-scoped API keys issued by the service itself.
- `development` mode bypasses forwarded headers and injects a fixed local admin principal.

The default mode is `enforce`.

## Auth Configuration

- `PROXY_BROKER_AUTH_MODE=enforce|development`
- `PROXY_BROKER_AUTH_SUBJECT_HEADERS`
  - default: `X-Forwarded-User,X-Auth-Request-User,Remote-User`
- `PROXY_BROKER_AUTH_EMAIL_HEADERS`
  - default: `X-Forwarded-Email,X-Auth-Request-Email`
- `PROXY_BROKER_AUTH_GROUPS_HEADERS`
  - default: `X-Forwarded-Groups,X-Auth-Request-Groups`
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

When you deploy behind Traefik, configure Forward Auth to populate the identity headers that `proxy-broker` expects. The service only needs the response headers from the auth service to reach the upstream request.

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
PROXY_BROKER_AUTH_ADMIN_USERS=admin@example.com
PROXY_BROKER_AUTH_ADMIN_GROUPS=proxy-broker-admins
```

In this setup:

- Forward Auth identifies the human user.
- `proxy-broker` decides whether that user is an admin.
- Non-admin humans can call `/api/v1/auth/me`, but cannot access the UI or admin APIs.

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
- `/api/v1/profiles/{profile_id}/ips/extract`
- `/api/v1/profiles/{profile_id}/sessions`
- `/api/v1/profiles/{profile_id}/sessions/open`
- `/api/v1/profiles/{profile_id}/sessions/open-batch`
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

Profile API keys are limited to their bound `profile_id`. Using a valid key against another profile returns `403 profile_access_denied`.
