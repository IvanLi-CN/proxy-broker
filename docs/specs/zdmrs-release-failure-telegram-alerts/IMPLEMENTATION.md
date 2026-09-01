# Implementation

## Coverage

- `.github/workflows/notify-release-failure.yml` keeps the existing
  `workflow_run` failure filter, release-context resolver, and manual smoke
  entrypoint.
- The two reusable-workflow calls now target Oidrune at
  `e48822f99c6402a753ed86557ea029754cbab20b`, pass `outcome` and a complete
  caller-generated `summary`, and grant `id-token: write`.
- `.github/scripts/test-notify-release-failure.py` checks the pinned reference,
  input boundary, permissions, summary fields, trigger filters, and removal of
  legacy secret forwarding. `.github/workflows/ci-pr.yml` runs it during script
  compilation and contract validation.

## Trusted Oidrune Revision

Live GitHub facts at implementation time confirm that:

- `IvanLi-CN/oidrune` is not archived.
- `main` resolves to `e48822f99c6402a753ed86557ea029754cbab20b`.
- That revision contains the same `.github/workflows/notify.yml` as `main` and
  is the target of the latest published `v0.1.14` release.
- The reusable workflow owns the default gateway and OIDC audience and requires
  `id-token: write` from its caller.

## Validation

- `python3 .github/scripts/test-notify-release-failure.py`
- `python3 -m py_compile .github/scripts/test-notify-release-failure.py`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/notify-release-failure.yml")'`
- `git diff --check`

Local validation does not dispatch the workflow and does not send a real
Telegram notification.
