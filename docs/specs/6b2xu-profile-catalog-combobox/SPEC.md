# Profile Catalog And Combobox（#6b2xu）

## 状态

- Status: 已完成
- Created: 2026-03-19
- Last: 2026-03-19

## 背景 / 问题陈述

- 当前侧栏的 `Profile ID` 只是自由文本输入，缺少已存在 profile 的可发现性，也缺少明确的新建动作。
- 现有后端只有按 `profile_id` 作用域工作的业务 API，没有面向 profile catalog 的 HTTP contract。
- SQLite 的 `list_profiles()` 仅从业务数据表做 `UNION`，无法持久化“刚创建、但还没有订阅/会话数据”的空 profile。

## 目标 / 非目标

### Goals

- 提供可下拉、可搜索、可显式新增的 profile selector。
- 为空 profile 引入真实可持久化的 profile catalog。
- 保持现有 load / refresh / extract / sessions 工作流按 `profile_id` 隔离。

### Non-goals

- 不做 profile rename、delete、批量管理或 MRU 排序。
- 不改动现有业务 API 的语义与 payload 结构。

## 范围（Scope）

### In scope

- 新增 `GET /api/v1/profiles` 与 `POST /api/v1/profiles`。
- 新增 profile catalog 存储与空 profile 持久化。
- 将侧栏 `Profile ID` 改为 anchored combobox，支持搜索与显式创建。
- 更新相关 stories、单测、路由测试与 e2e smoke。

### Out of scope

- 新增 profile 权限、标签、描述等元数据。
- 改变现有页面对 `profileId` 的 query key 设计。

## 需求（Requirements）

### MUST

- 已存在 profile 可以在下拉中搜索并选择。
- 不存在的 profile 可以通过明确的 create action 创建。
- 新创建但尚无业务数据的 profile 刷新后仍可列出。
- 创建入口只做 `trim + non-empty` 校验；精确重名返回冲突。

### SHOULD

- 当前已选但暂未出现在后端列表里的 `profileId` 仍应保留在候选中，避免上下文丢失。
- 组件支持键盘导航、回车选择、回车创建与清晰的 loading/empty/error 状态。

### COULD

- 创建成功后给出轻量 toast 反馈。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- 打开侧栏 selector 时，显示当前 active profile、搜索输入框与 profile 列表。
- 输入关键字后，列表实时过滤；匹配项为空时，显示空状态和 `Create "<query>"` 操作。
- 选择已有 profile 时，立即切换 active profile，关闭下拉，并让后续路由请求使用新的 `profileId`。
- 创建新 profile 成功后，立即切换到该 profile、刷新候选列表，并保持当前路由上下文。

### Edge cases / errors

- 输入仅空白字符时，不允许创建。
- 创建已存在的 profile 时，前端展示可恢复错误，并重新拉取列表与现有值对齐。
- profiles 列表请求失败时，不阻断当前已选 profile 的使用；selector 展示失败提示并允许重试。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Profile catalog HTTP API | HTTP | external | New | ./contracts/http-apis.md | proxy-broker | web admin UI | 列表与创建 |
| Profile catalog persistence | DB | internal | New | ./contracts/db.md | proxy-broker | Rust store/service | 支持空 profile 持久化 |

### 契约文档（按 Kind 拆分）

- [contracts/README.md](./contracts/README.md)
- [contracts/http-apis.md](./contracts/http-apis.md)
- [contracts/db.md](./contracts/db.md)

## 验收标准（Acceptance Criteria）

- Given 已有 `default` 与 `edge-jp` 两个 profile
  When 操作员打开 selector 并输入 `jp`
  Then 下拉只显示 `edge-jp` 且可直接选中。
- Given `fresh-lab` 尚不存在
  When 操作员在 selector 中执行 create
  Then 后端返回 201、active profile 切换到 `fresh-lab`，刷新页面后它仍在列表中。
- Given `POST /api/v1/profiles` 收到空白 `profile_id`
  When 请求到达后端
  Then 返回 `400 invalid_request`。
- Given `POST /api/v1/profiles` 收到已存在的 `profile_id`
  When 请求到达后端
  Then 返回 `409 profile_exists`，前端保留可恢复状态。

## 实现前置条件（Definition of Ready / Preconditions）

- 目标、范围与非目标已冻结。
- 空 profile 需要真实持久化而不是前端假列表，这一点已确认。
- 新增 HTTP contract 与 SQLite schema 变更已接受。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: store/service 校验、前端 selector 行为。
- Integration tests: Axum router 的 profiles list/create 路由。
- E2E tests (if applicable): smoke 覆盖选择已有 profile 与创建新 profile。

### UI / Storybook (if applicable)

- Stories to add/update: `ProfileSwitcher` 的 default、populated、search-no-match、creating。
- `play` / interaction coverage to add/update: selector 打开、过滤、创建。

### Quality checks

- `cargo test`
- `bun run check`
- `bun run typecheck`
- `bun run test`
- `bun run verify:stories`
- `bun run build`
- `bun run test:e2e`

## 文档更新（Docs to Update）

- `docs/contracts/http-apis.md`: 记录新的 profiles list/create endpoint。
- `docs/contracts/rust-api.md`: 补充 store/service contract。
- `docs/contracts/db.md`: 记录新的 `profiles` 表。
- `docs/specs/README.md`: 新增并更新规格台账。

## 计划资产（Plan assets）

- Directory: `docs/specs/6b2xu-profile-catalog-combobox/assets/`
- In-plan references: `![...](./assets/<file>.png)`
- PR visual evidence source: maintain `## Visual Evidence (PR)` in this spec when PR screenshots are needed.

## Visual Evidence (PR)

本次如需 PR 证据图，统一放在 `./assets/` 下。

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 新 spec、contracts 与全局文档索引完成更新。
- [x] M2: Rust store / service / HTTP 完成 profile catalog 支持。
- [x] M3: Web `ProfileSwitcher` 完成 searchable + creatable combobox 改造。
- [x] M4: Stories、单测、e2e 与验证脚本全部通过。

## 方案概述（Approach, high-level）

- 为 SQLite 增加独立 `profiles` 表，并让 `list_profiles()` 同时兼容历史业务表中的 profile。
- 在 Rust API 中引入显式 list/create contract，让 Web UI 不再依赖隐式输入新值。
- 前端 selector 采用 `Popover + Command` 组合，保持 props-driven 与 Storybook 友好。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：旧数据仓库可能没有 `profiles` 表，迁移必须保持向后兼容。
- 需要决策的问题：None。
- 假设（需主人确认）：profile ID 继续保持宽松语义，不新增字符集限制。

## 变更记录（Change log）

- 2026-03-19: 初始规格，冻结 profile catalog 与 combobox 方案。
- 2026-03-19: 实现完成，profiles catalog、combobox 与验证闭环全部落地。

## 参考（References）

- `docs/specs/web-admin-ui.md`
- `docs/specs/s3zu5-admin-ui-refresh/SPEC.md`
