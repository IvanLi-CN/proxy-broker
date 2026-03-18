# PR Label Release And Wildcard Listen

## Goal

Make `proxy-broker` shippable as a real deployable service by wiring a
label-driven GitHub release flow and by allowing both the HTTP server and
session listeners to run on wildcard binds for container and remote-host
deployments.

## Scope

- Add a checked-in GitHub workflow topology for PR label validation, PR CI,
  mainline CI, and post-merge release publishing.
- Publish versioned GitHub Releases plus GHCR container images from merged PR
  labels (`type:*` + `channel:*`).
- Add a configurable session listener bind IP so operators can choose
  `127.0.0.1` for local runs or `0.0.0.0` for wildcard deployments.
- Ensure container builds can inject the effective release version into the Rust
  binary metadata and OCI image metadata.

## Non-Goals

- No automatic server-side deployment after release.
- No new auth layer or reverse-proxy policy for public exposure.
- No UI redesign beyond release/deployment documentation updates.

## Acceptance Criteria

- PRs targeting `main` can be labeled with exactly one `type:*` and exactly one
  `channel:*`, and invalid label combinations fail a dedicated label gate.
- The repository runs distinct PR CI and mainline CI workflows, and release
  only starts after `CI Main` succeeds for a merged commit on `main`.
- Release publishing creates an idempotent Git tag, a GitHub Release, and GHCR
  image tags that follow the PR label policy.
- The Rust service can bind session listeners to a configured IP, and the
  published container defaults to `0.0.0.0` for both HTTP and session listeners.
- Local and containerized validation demonstrate that wildcard binds do not
  regress core session orchestration behavior.

## Verification

- `cargo test --all-features`
- `cargo build --release`
- `cd web && bun run check`
- `cd web && bun run test`
- `cd web && bun run build`
- Shared testbox smoke: build the container, run it with wildcard binds, load a
  sample subscription, open a session, and verify the resulting listener binds
  on `0.0.0.0`.

## Outcome

- `proxy-broker` has a reproducible PR-label-driven release path.
- Container releases publish an externally usable default bind strategy instead
  of a localhost-only deployment trap.
- Local CLI workflows still retain explicit localhost binds when operators want
  private-only sessions.
