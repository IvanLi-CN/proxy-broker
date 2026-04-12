# 全局代理池、Profile 分配与 Proxies 工作区

## Goal

把代理节点管理从各个 profile 的分散入口收拢到统一的管理员工作区，支持：

- 导入全局代理池。
- 导入当前 profile 的本地代理池。
- 查看全局与所有 profile 的代理节点库存。
- 调整节点当前分配到 `global` 或某个 `profile`。
- 删除当前库存里的导入节点。
- 为每个 profile 提供二态开关：`使用全局代理 / 不使用全局代理`。

## Scope

- 新增管理员工作区 `/proxies`，并在 AppShell 导航中暴露 `Proxies`。
- 后端新增 inventory layer：导入节点同时记录 `source_scope` 与 `allocation_scope`。
- 现有 profile 业务接口继续按 profile 工作，但有效池改为：
  - 本地节点
  - 加上全局节点（仅当 `use_global_proxies=true`）
- 在这些事件后重建受影响 profile 的有效快照：
  - 导入全局源
  - 导入 profile 本地源
  - 修改 allocation
  - 删除 inventory 节点
  - 切换 `use_global_proxies`
- existing / new profiles 默认 `use_global_proxies=true`。

## Non-Goals

- 不新增独立 `project` 实体。
- 不支持手动单节点新增/编辑。
- 不支持 profile 级细粒度挑选全局节点。
- 不引入新的全局自动同步任务或 task center 扩展。

## Data Model

### Proxy inventory node

每个导入节点都持有：

- `node_id`
- `proxy_name`
- `proxy_type`
- `server`
- `resolved_ips[]`
- `raw_proxy`
- `source_scope`
- `allocation_scope`
- `created_at`
- `updated_at`

### Profile proxy settings

- `profile_id`
- `use_global_proxies`

## Effective Pool Rules

- `allocation_scope=global` 的节点会进入所有 `use_global_proxies=true` 的 profile。
- `allocation_scope=profile:<id>` 的节点只进入对应 profile。
- 同名代理在同一个 profile 的 effective pool 里只保留一个赢家，优先级为：
  1. 直接分配给当前 profile 的节点优先于 global。
  2. 同级时，来源就是当前 profile 的节点优先。
  3. 再按 `updated_at` 新的优先。
  4. 最后按 `node_id` 稳定收敛。
- 删除或改分配只影响当前 inventory 快照；后续重新导入以源数据为准，上游仍存在的节点会恢复。

## API Contract

### 新增管理员接口

- `POST /api/v1/proxies/global/subscriptions/load`
  - 导入全局源并重建所有已启用 global 的 profile 有效池。
- `GET /api/v1/proxies?scope=all|global|profile&profile_id=...`
  - 返回 inventory 列表：
    - `node_id`
    - `proxy_name`
    - `proxy_type`
    - `server`
    - `resolved_ips`
    - `source_scope`
    - `allocation_scope`
    - `effective_profile_ids`
- `PATCH /api/v1/proxies/{node_id}/allocation`
  - 把节点当前分配到 `global` 或某个 `profile`。
- `DELETE /api/v1/proxies/{node_id}`
  - 删除当前 inventory 快照中的导入节点。
- `GET /api/v1/profiles/{profile_id}/proxy-settings`
- `PATCH /api/v1/profiles/{profile_id}/proxy-settings`
  - 仅暴露 `use_global_proxies`。

### 保留接口的语义变更

- `POST /api/v1/profiles/{profile_id}/subscriptions/load`
  - 语义更新为：导入当前 profile 的本地节点并重建该 profile 的 effective pool。

## UI Contract

### Proxies workspace

页面拆成两个明确的工作区页签：

1. `全局工作区`
   - 全局导入卡片。
   - 全局 inventory table：展示所有导入节点、来源作用域、当前分配作用域、生效 profile，并提供改分配与删除动作。
2. `当前配置工作区`
   - 当前 profile 本地导入卡片。
   - 当前 profile 的 `use_global_proxies` 开关。

全局配置不得嵌在当前 profile 语义的内容区里；即使当前 profile selector 仍存在于 AppShell，全局池也必须通过单独的工作区入口访问。

### Overview workspace

- 移除代理导入主入口。
- 保留 health、refresh、access control 等非 inventory 运营面。

## Acceptance Criteria

- 当某 profile 同时拥有本地节点和全局节点，且 `use_global_proxies=true` 时，`/ips`、`/sessions`、`/refresh`、`suggested-port` 都基于联合池工作。
- 当某 profile 关闭 `use_global_proxies` 时，其 effective pool 立即只保留本地节点，依赖纯全局节点的 session 会按现有 reconcile 规则被清退。
- 管理员导入 global source 后，所有默认启用 global 的 existing / new profiles 都能立即看到这些节点，无需逐 profile 再细配。
- 节点从 `global -> profile`、`profile -> global` 或 `profileA -> profileB` 改分配时，仅受影响的 profiles 会重建快照。
- 删除导入节点后会立刻从当前 inventory 消失；后续若相同 source 重新导入且上游仍包含它，则节点会恢复。
- 非管理员或 API key 访问 `Proxies` 页面与新增管理员接口时，必须被拒绝，不返回 profile-scoped 降级结果。

## Verification

- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run check`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run typecheck`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run test`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run verify:stories`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run build`
- `cd /Users/ivan/.codex/worktrees/12bb/proxy-broker/web && bun run test:e2e`

## Outcome

- 新的管理员工作区 `Proxies` 已接管全局导入、本地导入、profile 级 global 开关、以及节点 inventory 分配/删除操作。
- profile 有效池现在由 inventory layer 统一组合，不再直接把订阅源导入结果视为 profile 最终池。
- SQLite 与 memory store 都已持久化 inventory 节点与 `profile_proxy_settings`。
- 现有业务接口保持 profile 视角不变，但快照重建逻辑已改为读取 effective pool。

## Visual Evidence

- `source_type=storybook_canvas`
- `target_program=mock-only`
- `capture_scope=element`
- `sensitive_exclusion=N/A`
- `submission_gate=pending-owner-approval`
- `story_id_or_title=Pages/ProxiesPage/ZhCN`
- `state=全局工作区（zh-CN）`
- `evidence_note=展示全局池拥有独立工作区入口，并在该工作区中承载全局导入与跨 profile inventory 分配。`

![Proxies workspace global tab](./assets/proxies-workspace-global-tab-zh-cn.png)

- `source_type=storybook_canvas`
- `target_program=mock-only`
- `capture_scope=element`
- `sensitive_exclusion=N/A`
- `submission_gate=pending-owner-approval`
- `story_id_or_title=Pages/ProxiesPage/ZhCN`
- `state=当前配置工作区（zh-CN）`
- `evidence_note=展示当前 profile 的本地导入和 use_global_proxies 策略被单独收拢到当前配置工作区，不再混入全局池配置。`

![Proxies workspace profile tab](./assets/proxies-workspace-profile-tab-zh-cn.png)

- `source_type=storybook_canvas`
- `target_program=mock-only`
- `capture_scope=element`
- `sensitive_exclusion=N/A`
- `submission_gate=pending-owner-approval`
- `story_id_or_title=Pages/ProxiesPage/AccessDenied`
- `state=admin-only access gate`
- `evidence_note=展示非管理员访问 Proxies 工作区时的拒绝态，证明新入口不会退化成 profile-scoped 降级结果。`

![Proxies workspace access denied](./assets/proxies-workspace-access-denied.png)
