# 发布链路去饥饿化与专用 Publisher 身份修复（#tqs62）

## 状态

- Status: 已完成
- Created: 2026-03-24
- Last: 2026-03-24

## 背景 / 问题陈述

- 现有 `Release` workflow 会在主线触发时优先发布 first-parent 路径上最早的 `pending` snapshot，而不是刚合并的当前 commit。
- 当较早 backlog 因权限或环境问题持续失败时，新的 stable 版本会一直被阻塞，主线触发和实际发布目标不一致，维护者难以判断发布状态。
- 现有 tag / release / notes 写入依赖仓库默认 `GITHUB_TOKEN`，当目标提交修改过 `.github/workflows/**` 时，GitHub 会拒绝推送 release tag。

## 目标 / 非目标

### Goals

- 主线 `workflow_run` 触发固定发布当前 merge commit，不再被旧 backlog 饥饿阻塞。
- 手工 `workflow_dispatch(commit_sha)` 保持 exact target/backfill 能力，支持历史版本补发与 assets-only 路径。
- 发布链路改用专用 GitHub App publisher 处理 tag、GitHub Release、release snapshot notes 的写操作。
- 在 workflow 输出与日志中明确 `requested_sha`、`selected_target_sha`、`selected_release_tag`、backlog 摘要。

### Non-goals

- 不改 PR label 驱动的版本 bump 规则。
- 不引入新的外部队列或持久化系统，仍使用 git notes snapshot。
- 不新增自动部署或环境晋升逻辑。

## 范围（Scope）

### In scope

- 更新 `.github/workflows/release.yml` 的 target selection、publisher auth、summary 和 preflight。
- 更新 `.github/scripts/release_snapshot.py` 的 target selection 输出与 backlog 摘要。
- 更新 release 相关测试与项目文档，覆盖 current-first 与 manual backlog 语义。

### Out of scope

- 不修改 PR / main CI 的其他构建矩阵。
- 不调整 Docker/native asset 的产物形状。

## 需求（Requirements）

### MUST

- `workflow_run` 触发时，`selected_target_sha` 必须等于当前 `requested_sha`。
- 历史 backlog 不得自动阻塞当前主线版本发布。
- GitHub App publisher secrets 缺失时，workflow 必须在重型构建前显式失败。
- 历史版本的 manual backfill 仍可单独执行，且不自动清空 backlog。

### SHOULD

- 日志与 step summary 直接说明 requested/selected/backlog 关系。
- 保留现有 `next-pending` 诊断能力，便于维护者查看 backlog。

### COULD

- 在后续独立工作中补一条专门的 backlog inspect workflow 或运维脚本。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 主线 `CI Main` 成功后触发 `Release`：
  - workflow 固定选择当前 `workflow_run.head_sha` 作为发布目标。
  - 若该目标已有 release tag，则进入 assets-only 路径；否则按正常 stable/rc 规则发布该目标。
  - 若历史 backlog 仍存在，只作为 summary 可见信息保留，不自动 dispatch 下一个 pending target。
- 手工 `workflow_dispatch(commit_sha)`：
  - workflow 固定选择 `commit_sha` 指定的 snapshot。
  - 若目标已有 release tag，则只做 assets-only 回填。
  - 若目标尚未发布，则正常发布该目标；不会顺手补发别的 pending snapshot。

### Edge cases / errors

- 缺少 `RELEASE_PUBLISHER_APP_ID` 或 `RELEASE_PUBLISHER_APP_PRIVATE_KEY` 时，workflow 在 release-meta 阶段直接失败。
- GitHub App token 生成失败时，workflow 在 release-meta 阶段直接失败，不进入二进制和镜像矩阵。
- `release_enabled=false` 的 snapshot 继续走 no-op 语义，但 summary 仍输出 requested/selected/backlog 信息。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GitHub Release workflow outputs | internal | internal | Modify | None | release workflow | maintainers | 新增 requested/selected/backlog 输出 |
| Release publisher secrets | internal | internal | New | None | repo settings | release workflow | `RELEASE_PUBLISHER_APP_ID` / `RELEASE_PUBLISHER_APP_PRIVATE_KEY` |

### 契约文档（按 Kind 拆分）

None

## 验收标准（Acceptance Criteria）

- Given 主线存在更早的 `pending` snapshot，When 新的 merge commit 触发 `Release`，Then workflow 仍发布当前 merge commit，且 summary 明确展示 backlog 摘要。
- Given 发布目标修改过 `.github/workflows/**`，When `Release` 创建 tag，Then 不再因默认 `GITHUB_TOKEN` 缺少 `workflows` 权限而失败。
- Given 历史 commit 已存在 release tag，When 手工 `workflow_dispatch(commit_sha)`，Then workflow 只回填该目标的 release 资产，不自动补发其他 pending snapshot。
- Given publisher secrets 缺失，When `Release` 进入 release-meta，Then workflow 在重型构建前失败并输出明确缺口。

## 实现前置条件（Definition of Ready / Preconditions）

- 目标/非目标、范围与 current-first 语义已冻结
- 专用 publisher 身份采用 GitHub App，而不是 PAT
- manual backlog 仅手工 backfill 的产品语义已确定

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: 扩展 `.github/scripts/test-release-snapshot.sh`
- Integration tests: 解析并校验 `.github/workflows/release.yml`

### Quality checks

- `bash .github/scripts/test-release-snapshot.sh`
- `python3 -m py_compile .github/scripts/release_snapshot.py`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml")'`

## 文档更新（Docs to Update）

- `README.md`: 更新 GitHub Release 资产与 manual backfill 说明
- `docs/specs/README.md`: 新增本规格索引并更新日期
- `docs/specs/r2m7k-pr-label-release-and-wildcard-listen/SPEC.md`: 修正主线默认发布语义与权限契约

## 计划资产（Plan assets）

- Directory: `docs/specs/tqs62-release-current-first-publisher/assets/`
- In-plan references: `![...](./assets/<file>.png)`
- PR visual evidence source: maintain `## Visual Evidence (PR)` in this spec when PR screenshots are needed.

## Visual Evidence (PR)

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 新增 current-first publisher follow-up spec，并同步 release 文档口径
- [x] M2: 重构 release target selection、publisher preflight 与 summary 输出
- [x] M3: 切换 tag/release/notes 写操作到 GitHub App token，并移除自动清队列
- [x] M4: 更新 release snapshot 测试与相关校验，覆盖 current-first/manual backlog 语义

## 方案概述（Approach, high-level）

- 继续保留 git notes snapshot 作为 release source of truth，只调整发布目标选择逻辑与权限来源。
- 在 release-meta 阶段先生成 publisher token，提前暴露 secrets/installation 权限缺口。
- merge-manifest 阶段使用 publisher token checkout 与 GitHub API，确保 tag / release / notes 写入都不受 `GITHUB_TOKEN` 限制。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：仓库若未安装具备 `Contents: write`、`Workflows: write` 的 GitHub App，workflow 会开始 fast-fail。
- 需要决策的问题：None
- 假设（需主人确认）：GitHub App 将安装在当前仓库并提供当前 repo 级别 access。

## 变更记录（Change log）

- 2026-03-24: 新增 follow-up 规格，冻结 current-first、manual backlog 与 GitHub App publisher 语义。

## 参考（References）

- `docs/specs/r2m7k-pr-label-release-and-wildcard-listen/SPEC.md`
- `.github/workflows/release.yml`
- `.github/scripts/release_snapshot.py`
