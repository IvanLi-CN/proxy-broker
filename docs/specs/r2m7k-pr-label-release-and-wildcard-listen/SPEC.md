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
  labels (`type:*` + `channel:*`), with immutable release snapshots frozen on
  `main`.
- Add a configurable session listener bind IP so operators can choose
  `127.0.0.1` for local runs or `0.0.0.0` for wildcard deployments.
- Ensure container builds can inject the effective release version into the Rust
  binary metadata and OCI image metadata.
- Keep multi-platform image publishing on native `linux/amd64` and
  `linux/arm64` runners, then merge the digests into a final manifest list.

## Non-Goals

- No automatic server-side deployment after release.
- No new auth layer or reverse-proxy policy for public exposure.
- No UI redesign beyond release/deployment documentation updates.

## Acceptance Criteria

- PRs targeting `main` can be labeled with exactly one `type:*` and exactly one
  `channel:*`, and invalid label combinations fail a dedicated label gate.
- The repository runs distinct PR CI and mainline CI workflows, and release
  only starts after `CI Main` succeeds for a merged commit on `main`.
- `CI Main` freezes one immutable release snapshot per merged `main` commit, and
  release/backfill runs only consume those stored decisions.
- Release publishing creates an idempotent Git tag, a GitHub Release, and GHCR
  image tags that follow the PR label policy, reusing an existing tag when the
  same commit is retried.
- Multi-platform release publishing runs `linux/amd64` and `linux/arm64`
  natively, verifies the merged manifest, and keeps `latest` reserved for the
  newest stable snapshot.
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
- `bash .github/scripts/test-release-snapshot.sh`
- Shared testbox smoke: verify wildcard binds with both the native Linux
  release binary and a containerized runtime, load a sample subscription, open
  a session, and confirm the resulting listener binds on `0.0.0.0`.

## Outcome

- `proxy-broker` has a reproducible PR-label-driven release path.
- Mainline release decisions survive burst merges and reruns through immutable
  snapshots plus oldest-pending backfill selection.
- Stable releases continue to derive their next base version only from prior
  stable releases, so `channel:rc` snapshots cannot accidentally advance a
  later stable patch/minor/major release.
- Container releases publish an externally usable default bind strategy instead
  of a localhost-only deployment trap, with native dual-platform manifest
  publishing instead of a single serial emulated build.
- Local CLI workflows still retain explicit localhost binds when operators want
  private-only sessions.
