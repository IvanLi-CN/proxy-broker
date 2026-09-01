# Release Failure Notifications via Oidrune

## Context and Scope

This topic owns the repository-local notification wrapper for failed `Release`
workflow runs. The wrapper delegates delivery to
`IvanLi-CN/oidrune/.github/workflows/notify.yml` at the trusted immutable
revision `e48822f99c6402a753ed86557ea029754cbab20b`.

The scope is `.github/workflows/notify-release-failure.yml`, its workflow
contract test, and the CI entry that runs that test. Release target selection,
publication, artifacts, and the resolver's target-SHA lookup remain outside
this topic except for the metadata exposed in the notification summary.

## Requirements

### Trigger and Failure Semantics

- `REQ-NOTIFY-TRIGGER` The wrapper MUST retain a `workflow_run` trigger for
  `Release` completions on `main` and MUST notify only when the completed run's
  conclusion is `failure`.
- `REQ-NOTIFY-SMOKE` The wrapper MUST retain a manual `workflow_dispatch`
  path that sends a distinct smoke notification without changing release
  failure filtering.
- `REQ-NOTIFY-CONTEXT` The failure path MUST preserve the resolver's
  project-specific ref, actor, run-attempt, detail, and release-target SHA
  behavior.

### Oidrune Boundary

- `REQ-NOTIFY-PIN` Both caller paths MUST use the complete immutable Oidrune
  reference `IvanLi-CN/oidrune/.github/workflows/notify.yml@e48822f99c6402a753ed86557ea029754cbab20b`.
- `REQ-NOTIFY-OIDC` Each Oidrune caller MUST grant `id-token: write`; callers
  MUST omit `gateway_url` and `oidc_audience` so Oidrune's default gateway
  configuration remains authoritative.
- `REQ-NOTIFY-SECRETS` The wrapper MUST NOT forward the retired Telegram
  `SHOUTRRR_URL` secret or any other delivery secret.
- `REQ-NOTIFY-SUMMARY` Each caller MUST provide a complete summary containing
  the project name, status, target SHA, run URL, and a distinct failure or
  smoke title; failure summaries MUST also retain resolver details and smoke
  summaries MUST identify the manual smoke path.

## Verification

- `VER-NOTIFY-CONTRACT` covers: `REQ-NOTIFY-TRIGGER`, `REQ-NOTIFY-SMOKE`, `REQ-NOTIFY-CONTEXT`, `REQ-NOTIFY-PIN`, `REQ-NOTIFY-OIDC`, `REQ-NOTIFY-SECRETS`, and `REQ-NOTIFY-SUMMARY` through the static workflow contract test and CI registration.
- `VER-NOTIFY-SPEC` covers all `REQ-NOTIFY-*` requirements through this
  canonical specification and its companion implementation record.

## Related ADRs

- None
