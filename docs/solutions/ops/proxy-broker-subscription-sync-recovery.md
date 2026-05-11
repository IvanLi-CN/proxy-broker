# Proxy Broker Subscription Sync Recovery

## Context

Proxy Broker automatic subscription sync can fail even when the service is healthy if an upstream subscription source returns UA-sensitive responses such as `error code: 1102`, a marketing/error page, or a YAML payload without usable `proxies:` entries.

## Response Pattern

- Treat `proxy-broker` health and subscription health separately. A healthy container can still have a failing `subscription_sync` task.
- Mitigate runtime/store drift by closing stale sessions through the Broker close-session API. Do not delete rows directly from SQLite.
- Sample task events, sync config, and proxy inventory before changing code so the failing import id and retained inventory state are known.
- Keep the last good `proxy_inventory_nodes` snapshot on sync failure; only replace inventory after a subscription source has parsed, filtered, and produced usable resolved nodes.

## Implementation Guardrails

- URL subscription fetch should keep trying compatibility user agents after default request failures, non-2xx responses, or non-proxy payloads.
- Invalid payload errors should preserve attempt labels and a bounded response shape summary. Avoid logging full subscription URLs or large/raw token-like payloads.
- Task failure closeout should write structured error details into `task_run_events.payload_json`, not only `task_runs.error_message`.
- Regression tests should cover fallback success after `error code: 1102`, all-attempt failure detail, and failed sync preserving existing inventory.
