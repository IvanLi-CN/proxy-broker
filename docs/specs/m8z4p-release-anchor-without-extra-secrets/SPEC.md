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
- 对修改过 `.github/workflows/**` 的发布目标，自动改用一个 bot 生成的“锚点提交”承接 tag / GitHub Release，而不是阻塞发版。
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
  - 若目标 commit 修改了 `.github/workflows/**`，workflow 会创建或复用 `release-anchors/<release_tag>` 分支上的空锚点提交，并对该锚点提交创建 git tag / GitHub Release。
  - 锚点提交不回写 `main`，因此不会干扰后续主线合并与 CI。
- 手工 `workflow_dispatch(commit_sha)`：
  - 仍只处理指定 commit。
  - 若该 snapshot 已发布，即使 release tag 最终指向锚点提交，也必须继续走 assets-only 回填。

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
- Given 发布目标修改了 `.github/workflows/**`，When `Release` 发布该版本，Then workflow 不要求新增 publisher secret，且会改用 `release-anchors/<tag>` 上的锚点提交承接 tag / GitHub Release。
- Given 已发布 snapshot 的 release tag 指向锚点提交而不是原始 main commit，When 手工 `workflow_dispatch(commit_sha)` 重新补资产，Then workflow 仍识别为 assets-only，不会误触发全量重发。

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
- 当目标 commit 触碰 workflow 文件时，在专用 `release-anchors/<tag>` 分支上生成一个空锚点提交；该提交不修改 workflow 文件，因此可以由默认 token 正常承接 tag 创建。
- `mark-released` 仍标记原始 snapshot target，让发布事实继续绑定到主线 merge commit，而不是锚点分支。

## 风险 / 假设

- 风险：仓库会保留 `release-anchors/*` 分支作为发布锚点痕迹，需要维护者接受这类分支存在。
- 假设：默认 `GITHUB_TOKEN` 具备 `contents: write`，且允许创建非受保护的 `release-anchors/*` 分支。

## 变更记录（Change log）

- 2026-03-24: 新增 follow-up 规格，收敛为“无额外 secrets + 必要时自动锚点提交”的主线发版语义。

## 参考（References）

- `docs/specs/tqs62-release-current-first-publisher/SPEC.md`
- `.github/workflows/release.yml`
- `.github/scripts/release_snapshot.py`
