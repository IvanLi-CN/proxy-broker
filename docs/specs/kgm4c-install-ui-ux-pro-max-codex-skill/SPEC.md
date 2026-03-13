# 安装 UI UX Pro Max Codex 项目技能（#kgm4c）

## 状态

- Status: 部分完成（2/3）
- Created: 2026-03-13
- Last: 2026-03-13

## 背景 / 问题陈述

- 当前仓库没有项目内 Codex UI/UX 设计 skill，相关 UI/UX 工作只能依赖外部上下文或全局能力。
- 主人要求把 `nextlevelbuilder/ui-ux-pro-max-skill` 直接安装到当前仓库，确保该能力随仓库一起版本化与评审。
- 若不在仓库内安装，后续协作者无法从仓库直接获得同样的 skill 入口，也无法在 PR 中审阅安装内容。

## 目标 / 非目标

### Goals

- 在仓库内生成 `.codex/skills/ui-ux-pro-max/`，让 Codex 可按项目级 skill 使用 UI UX Pro Max。
- 记录安装来源、版本策略、验证方式与交付边界，确保安装过程可追踪。
- 通过本地验证、提交、推送、PR 与 review-loop 收敛完成快车道交付。

### Non-goals

- 不安装到全局 `~/.codex/skills`。
- 不修改 Rust 运行时代码、HTTP/DB/file format 合同或现有业务逻辑。
- 不顺手做 UI 改造、设计重构或额外脚手架调整。

## 范围（Scope）

### In scope

- 引入 `.codex/skills/ui-ux-pro-max/` 下的项目级技能文件、脚本与数据资产。
- 建立并维护 `docs/specs/README.md` 与本 spec，记录安装与收敛情况。
- 运行最小 smoke test，确认 skill 搜索脚本可执行。
- 完成本地提交、远端分支、PR、CI 与 review-loop 收敛。

### Out of scope

- 全局 skill 安装。
- 任何 `.codex/**`、`docs/specs/**` 之外的源码或合同变更。
- 与本次安装无关的 CI、构建流程或仓库结构优化。

## 需求（Requirements）

### MUST

- 使用项目官方 Codex 安装路径，在仓库根目录执行 `uipro init --ai codex` 等效流程。
- 生成的文件必须位于 `.codex/**`，并可由 Git 跟踪。
- 变更完成后必须能通过至少一次基于 `search.py` 的 smoke test。
- 整个 PR diff 仅允许包含 `.codex/**` 与 `docs/specs/**`。

### SHOULD

- 记录 npm CLI 版本与上游 release 版本，便于后续复现。
- 让 spec 明确写出 CI 与 review-loop 的收敛结果。

### COULD

- 在 spec 中记录生成目录结构摘要，帮助后续维护者理解安装结果。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 从 `main` 当前提交切出 `th/install-ui-ux-pro-max-skill` 分支。
- 使用 `npx uipro-cli@2.2.3 init --ai codex` 在仓库内生成 Codex skill。
- 校验 `.codex/skills/ui-ux-pro-max/SKILL.md` 与 `scripts/search.py` 存在，且脚本可执行最小查询。
- 将变更提交并通过 GitHub MCP 创建 PR，等待 CI 与 review-loop 收敛。

### Edge cases / errors

- 若安装流程尝试覆盖已有 `.codex/skills/ui-ux-pro-max/` 内容，则中止并人工确认，不做静默覆盖。
- 若安装结果触及 `.codex/**` 之外的项目文件，则视为越界并中止快车道。
- 若 smoke test 失败，则必须先定位并修复安装问题，再进入 PR 阶段。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| None | None | internal | None | None | N/A | Codex 项目技能加载 | 本计划不修改公开接口 |

### 契约文档（按 Kind 拆分）

None

## 验收标准（Acceptance Criteria）

- Given 仓库根目录不存在项目级 UI UX Pro Max Codex skill，When 执行安装流程，Then `.codex/skills/ui-ux-pro-max/SKILL.md` 与相关脚本/数据资产被创建并纳入 Git 变更。
- Given 安装已完成，When 运行 `python3 .codex/skills/ui-ux-pro-max/scripts/search.py "proxy broker dashboard" --domain style`，Then 命令成功返回并输出可解析的结果。
- Given 本次交付进入 PR 阶段，When 检查变更文件列表，Then 仅包含 `.codex/**` 与 `docs/specs/**`。
- Given PR 已创建，When CI 与 review-loop 收敛结束，Then PR 不再存在待处理的 checks 或 review 阻塞项。

## 实现前置条件（Definition of Ready / Preconditions）

- 目标与范围已冻结为“项目内 Codex skill 安装”
- 安装命令、验证方式、提交与 PR 收口方式已明确
- 本计划不涉及新增或修改任何公开接口契约

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: N/A
- Integration tests: N/A
- E2E tests (if applicable): N/A

### UI / Storybook (if applicable)

- Stories to add/update: None
- Visual regression baseline changes (if any): None

### Quality checks

- Smoke test: `python3 .codex/skills/ui-ux-pro-max/scripts/search.py "proxy broker dashboard" --domain style`
- CI: 仓库现有 GitHub Actions `Rust checks`

## 文档更新（Docs to Update）

- `docs/specs/README.md`: 增加本规格索引并同步状态/备注
- `docs/specs/kgm4c-install-ui-ux-pro-max-codex-skill/SPEC.md`: 记录安装、验证、PR 与 review-loop 收敛结果

## 安装记录（Implementation Notes）

- 安装命令：`npx uipro-cli@2.2.3 init --ai codex`
- 安装结果：CLI 输出 `Generated from templates!`，在仓库内创建 `.codex/skills/ui-ux-pro-max/`。
- 关键产物：`SKILL.md`、`scripts/{search.py,core.py,design_system.py}`、`data/*.csv`。
- 体量摘要：共 28 个文件，目录大小约 `576K`。
- 版本记录：npm CLI `uipro-cli@2.2.3`；调研时上游 GitHub 最新 release 为 `v2.5.0`（2026-03-10）。
- review-loop 修复：补齐项目内命令路径、项目级持久化路径说明、持久化 slug 安全校验，以及 dashboard page override 与 master 风格一致性。
- review-loop 二轮修复：让 Unicode 项目名可作为 slug 保存、让默认持久化提示与真实目录一致、在写盘前完成 page 参数校验，并保留 `AI/UI/UX/3D` 等短关键词的搜索能力。
- review-loop 三轮修复：避免 `--page login` 被 dashboard 项目上下文误判为 dashboard 页面，并为 `--persist` 引入显式 `--force` 覆盖门禁，防止静默覆盖人工维护内容。

## 计划资产（Plan assets）

- Directory: `docs/specs/kgm4c-install-ui-ux-pro-max-codex-skill/assets/`
- In-plan references: `![...](./assets/<file>.png)`
- PR visual evidence source: maintain `## Visual Evidence (PR)` in this spec when PR screenshots are needed.
- If an asset must be used in impl (runtime/test/official docs), list it in `资产晋升（Asset promotion）` and promote it to a stable project path during implementation.

## Visual Evidence (PR)

本次计划默认不需要截图；若后续 PR 需要展示安装结果，再补充真实证据图。

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 建立 `docs/specs/` 索引与本安装规格
- [x] M2: 在项目内安装 UI UX Pro Max Codex skill 并完成 smoke test
- [ ] M3: 完成本地提交、PR、CI 与 review-loop 收敛

## 方案概述（Approach, high-level）

- 采用官方 CLI 的 Codex 平台模板生成项目内技能，避免手工拷贝造成结构漂移。
- 将规格与安装结果一起版本化，确保后续 review 能直接看到安装内容与验证依据。
- 仅在 review/checks 暴露明确问题时做最小修复，避免把此 PR 扩大为泛化清理。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：上游 CLI 拉取最新 release 时可能与 npm 包内置模板存在版本差异。
- 风险：生成文件量较大，需特别核对没有越出 `.codex/**`。
- 需要决策的问题：None
- 假设（需主人确认）：当前仓库允许提交项目内 `.codex/**` 目录。

## 变更记录（Change log）

- 2026-03-13: 创建规格，冻结为项目内安装 UI UX Pro Max Codex skill。
- 2026-03-13: 完成项目内安装与 smoke test，等待提交、PR 与 review-loop 收敛。
- 2026-03-13: 根据 PR review-loop 修复持久化路径逃逸与文档/override 不一致问题。
- 2026-03-13: 根据二轮 review-loop 修复 Unicode slug、短关键词搜索与持久化提示/原子性问题。
- 2026-03-13: 根据三轮 review-loop 修复 page type 误判与 persist 静默覆盖问题。

## 参考（References）

- https://github.com/nextlevelbuilder/ui-ux-pro-max-skill
- https://www.npmjs.com/package/uipro-cli
