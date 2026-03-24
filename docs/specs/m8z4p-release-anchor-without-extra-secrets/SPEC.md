# 发布锚点与无额外 Secret 的主线发版修复（#m8z4p）

## 状态

- Status: 已完成
- Created: 2026-03-24
- Last: 2026-03-24

## 背景 / 问题陈述

- `#tqs62` 虽然解决了“主线 current-first 发版”的饥饿问题，但把 tag / release / notes 写入改成了 GitHub App publisher。
- 当前仓库并没有配置额外的 publisher secret，导致 `Release` workflow 在 release-meta 阶段直接失败，最新版本依旧无法发布。
- 问题的本质不是必须引入新的认证主体，而是发布目标一旦直接落在修改过 `.github/workflows/**` 的 merge commit 上，默认 `GITHUB_TOKEN` 无法直接为该 commit 创建 release tag。

## 目标 / 非目标

### Goals

- 保持主线 `workflow_run` 的 current-first 语义，不再回退到更早 backlog。
- 不要求仓库新增 `RELEASE_PUBLISHER_*` secrets。
- 对 workflow 文件树落后于当前 `main` 的发布目标，自动改用一个 bot 生成的“锚点提交”承接 tag / GitHub Release，而不是阻塞发版。
- 手工 `workflow_dispatch(commit_sha)` 的 backfill / assets-only 语义继续成立。

### Non-goals

- 不改 PR label 驱动的版本 bump 规则。
- 不引入新的外部发布服务。
- 不让 release job 为了发版再回写 `main`。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 主线 `CI Main` 成功后触发 `Release`：
  - 固定选择当前 `workflow_run.head_sha` 对应的 immutable snapshot。
  - 若目标 commit 未修改 `.github/workflows/**`，继续直接对该 commit 发布。
  - 若目标 commit 的 `.github/workflows/**` 文件树与当前 `origin/main` 不同，workflow 会创建或复用 `release-anchors/<release_tag>` 分支上的合成锚点提交：主体文件树来自目标 commit，但 `.github/workflows/**` 必须回退为最新 `origin/main`。
  - 若上述“主体文件树 + 当前 workflow 树”合成后没有产生任何内容差异，workflow 必须直接发布原始目标 commit，而不是为了走锚点路径强行制造 no-op commit。
  - 锚点提交不回写 `main`，因此不会干扰后续主线合并与 CI。
- 手工 `workflow_dispatch(commit_sha)`：
  - 仍只处理指定 commit。
  - 若该 snapshot 已发布，即使 release tag 最终指向锚点提交，也必须继续走 assets-only 回填。
  - 旧版本 backfill 必须复用 snapshot 已解析出的 publication tags；只有当前最新 stable 版本可以持有 `latest`，历史补发不得抢占 latest release。

### Observability

- release summary 明确输出：
  - `requested_sha`
  - `selected_target_sha`
  - `selected_release_tag`
  - `target_touches_workflows`
  - `requires_release_anchor`
  - backlog 摘要

## 验收标准（Acceptance Criteria）

- Given 主线存在 backlog，When 新 merge commit 触发 `Release`，Then 仍发布当前 merge commit 对应版本，而不是自动补发旧 backlog。
- Given 发布目标的 `.github/workflows/**` 文件树与当前 `main` 存在差异，When `Release` 发布该版本，Then workflow 不要求新增 publisher secret，且会改用 `release-anchors/<tag>` 上的合成锚点提交承接 tag / GitHub Release。
- Given 发布目标修改了 `.github/workflows/**` 但 workflow 树已经等于当前 `main`，When `Release` 发布该版本，Then workflow 直接发布原始目标 commit，而不是在 `git commit` 处因为 no-op 失败。
- Given 已发布 snapshot 的 release tag 指向锚点提交而不是原始 main commit，When 手工 `workflow_dispatch(commit_sha)` 重新补资产，Then workflow 仍识别为 assets-only，不会误触发全量重发。
- Given 历史 stable 版本通过 `workflow_dispatch(commit_sha)` 补发，When workflow 创建或复用 GitHub Release，Then 该 release 不会被标记为 latest，且最新 stable 版本继续保留 `latest`。

## 非功能性验收 / 质量门槛（Quality Gates）

- `bash .github/scripts/test-release-snapshot.sh`
- `python3 -m py_compile .github/scripts/release_snapshot.py`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml")'`
- `git diff --check`

## 文档更新（Docs to Update）

- `README.md`
- `docs/specs/README.md`
- `docs/specs/r2m7k-pr-label-release-and-wildcard-listen/SPEC.md`
- `docs/specs/tqs62-release-current-first-publisher/SPEC.md`

## 方案概述（Approach, high-level）

- 保留 immutable snapshot + current-first 选择逻辑。
- 移除 GitHub App publisher preflight 与 secrets 依赖，恢复使用默认 `GITHUB_TOKEN` 完成 tag / release / notes 写入。
- 当目标 commit 触碰 workflow 文件且 workflow 树确实需要被“替换为当前 main”时，在专用 `release-anchors/<tag>` 分支上生成一个合成锚点提交；该提交保留目标内容，但 workflow 文件树始终来自当前 `main`，从而可以由默认 token 正常承接 tag 创建。
- `mark-released` 仍标记原始 snapshot target，让发布事实继续绑定到主线 merge commit，而不是锚点分支。
- GitHub Release 的 `make_latest` 必须由 snapshot 导出的 publication tags 决定，而不是由手工 backfill 的创建时间决定。

## 风险 / 假设

- 风险：仓库会保留 `release-anchors/*` 分支作为发布锚点痕迹，需要维护者接受这类分支存在。
- 假设：默认 `GITHUB_TOKEN` 具备 `contents: write`，且允许创建非受保护的 `release-anchors/*` 分支。

## 变更记录（Change log）

- 2026-03-24: 新增 follow-up 规格，收敛为“无额外 secrets + 必要时自动锚点提交”的主线发版语义。
- 2026-03-24: 收敛锚点细节为“目标内容树 + 当前 workflow 树”的合成提交，并在无差异时直接回退到原始目标 commit。
- 2026-03-24: 补充历史 backfill 不得抢占 `latest` 的发布约束，GitHub Release latest 状态改由 snapshot publication tags 决定。
- 2026-03-24: 扩大 anchor 判定条件为“目标 workflow 树与当前 main 不同”而不仅是“目标 commit 自身修改了 workflow 文件”。

## 参考（References）

- `docs/specs/tqs62-release-current-first-publisher/SPEC.md`
- `.github/workflows/release.yml`
- `.github/scripts/release_snapshot.py`
