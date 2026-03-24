# PR Label Release And Wildcard Listen（#r2m7k）

## 状态

- Status: 已完成
- Created: 2026-03-18
- Last: 2026-03-24

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
- Publish native GitHub Release tarballs for `linux/amd64`, `linux/arm64`,
  `darwin/amd64`, and `darwin/arm64`, plus a shared SHA256 manifest, while
  reusing the existing release object/tag on reruns or backfills.
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
- The GitHub Release also carries exactly four native binary tarballs and one
  aggregated `proxy-broker-<tag>-sha256.txt` asset, and rerunning the same tag
  replaces same-name hosted assets instead of failing with upload conflicts.
- Native tarball filenames keep the release tag form (`vX.Y.Z...`), while the
  embedded binary version stays aligned with the release effective version used
  by the container image (`X.Y.Z...` / `X.Y.Z-rc...`).
- Multi-platform release publishing runs `linux/amd64` and `linux/arm64`
  natively, verifies the merged manifest, and keeps `latest` reserved for the
  newest stable snapshot.
- A manual `workflow_dispatch(commit_sha)` backfill can attach missing native
  assets to an existing release without minting a new tag or release record,
  even when that commit's snapshot is already marked `released`, and that
  assets-only path must not republish container tags or implicitly publish
  other pending snapshots.
- The Rust service can bind session listeners to a configured IP, and the
  published container defaults to `0.0.0.0` for both HTTP and session listeners.
- Local and containerized validation demonstrate that wildcard binds do not
  regress core session orchestration behavior.

## Verification

- `cargo test --all-features`
- `cd web && bun run check`
- `cd web && bun run test`
- `cd web && bun run build`
- `APP_EFFECTIVE_VERSION=v0.0.0-ci .github/scripts/package_release_asset.sh --platform linux --arch amd64 --output-dir <tmp>`
- `bash .github/scripts/test-release-snapshot.sh`
- Shared testbox smoke: verify wildcard binds with both the native Linux
  release binary and a containerized runtime, load a sample subscription, open
  a session, and confirm the resulting listener binds on `0.0.0.0`.

## Outcome

- `proxy-broker` has a reproducible PR-label-driven release path.
- Mainline release decisions survive burst merges and reruns through immutable
  snapshots, current-first mainline target selection, and exact historical
  release backfills for `workflow_dispatch(commit_sha)` when the requested
  commit already carries the release tag, without implicitly publishing other
  queued snapshots.
- Stable releases continue to derive their next base version only from prior
  stable releases, so `channel:rc` snapshots cannot accidentally advance a
  later stable patch/minor/major release.
- GitHub Releases expose native Linux/macOS binaries and a checksum manifest,
  and historical releases can be backfilled by rerunning the existing release
  workflow against the original `main` commit SHA.
- Container releases publish an externally usable default bind strategy instead
  of a localhost-only deployment trap, with native dual-platform manifest
  publishing instead of a single serial emulated build.
- Local CLI workflows still retain explicit localhost binds when operators want
  private-only sessions.

## 变更记录（Change log）

- 2026-03-18: 初始规格，冻结 label-driven release、GHCR 发布和 wildcard bind 范围。
- 2026-03-21: 补充 GitHub Release 原生二进制资产、SHA256 清单与 `workflow_dispatch(commit_sha)` 回填契约。
- 2026-03-21: 明确原生资产文件名使用 release tag，但二进制内嵌版本必须与容器镜像保持一致。
- 2026-03-24: 后续规格 `#tqs62` 将主线自动发布语义收敛为 current-first。
- 2026-03-24: 后续规格 `#m8z4p` 进一步收敛为“默认 `GITHUB_TOKEN` + release anchor”发布路径，不再要求额外 publisher secrets。
