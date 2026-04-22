# 修复 malformed Hysteria 速率节点导致的 shared runtime 毒化（#7aqsu）

## 状态

- Status: 进行中（本地已验证）
- Created: 2026-04-23
- Last: 2026-04-23

## 背景 / 问题陈述

- 101 上 `proxy-broker` 当前服务健康，但创建节点定向会话会被共享 Mihomo runtime 的 `/configs` 阶段拒绝。
- 已确认根因是上游订阅返回了 `hysteria` 节点空 `up/down` 速率字段；当前导入链路会把该节点原样写入 `proxy_inventory_nodes` / `subscription_nodes`，shared runtime 渲染时又把脏节点直接带进 Mihomo payload。
- shared runtime 以“全量 inventory 节点 + 全量活动 session”建模，所以单个 profile 的坏节点会污染其他 profile 的会话创建与任务运行。

## 目标 / 非目标

### Goals

- 在订阅导入、手工节点组导入与后续 refresh 中统一拦截 malformed hysteria / hysteria2 速率节点，并按“整节点丢弃”处理。
- 在 shared runtime payload 组装阶段再加一道兜底，确保历史库存中的坏节点不会继续导致 `/configs` 400。
- 在 SQLite 打开阶段自愈已持久化的坏节点，清理 `subscription_nodes` 与 `proxy_inventory_nodes` 中命中的脏记录。
- 按快车道完成 PR、CI、release、101 更新与维护记录收尾。

### Non-goals

- 不做通用所有协议的全量 schema 校验器。
- 不改变现有 HTTP / TS contract 形状，也不重做 UI 展示。
- 不依赖上游先修复订阅后再恢复线上服务。

## 范围（Scope）

### In scope

- `src/subscription.rs`：source/manual 导入时识别并丢弃 malformed hysteria / hysteria2 节点，并把原因写入 `warnings`。
- `src/service.rs` / `src/config_render.rs` 相关 shared runtime 路径：过滤历史坏 inventory 节点，避免 payload 再次毒化 Mihomo。
- `src/store/sqlite.rs`：在 SQLite 打开/迁移后清理已持久化的坏 `subscription_nodes` / `proxy_inventory_nodes`。
- Rust 回归测试、spec 同步、PR / release / 101 验收。

### Out of scope

- 新增 UI 状态面板、批量数据库修复工具、或额外人工运维界面。
- 对 Hysteria 以外协议做无证据扩展的字段规范化。

## 需求（Requirements）

### MUST

- `hysteria` / `hysteria2` 节点若显式携带 `up` 或 `down` 且值为空、纯空白、或不含任何数字，则必须视为 malformed 并整节点丢弃。
- source/manual 导入命中该规则时，`LoadSubscriptionResponse.warnings` 必须包含可读原因，其余合法节点继续导入。
- shared runtime 构建 payload 时不得再把历史 malformed inventory 节点送进 Mihomo `/configs`。
- SQLite 打开新旧库后都必须自动清理已持久化的 malformed `subscription_nodes` / `proxy_inventory_nodes`。
- 快车道终点为 merge+cleanup，且 101 上 `browser` 节点定向会话恢复成功、相关日志恢复干净后才算完成。

### SHOULD

- runtime 兜底日志应带 `import_id`、`node_id` 与作用域信息，便于线上定位来源。
- 历史坏节点被清理后，不要求额外手工 SQL 批量修库即可恢复服务。

## 功能与行为规格（Functional/Behavior Spec）

### Core flows

- source-based 导入：订阅 payload 解析完成后，对每个候选节点执行协议感知校验；命中 malformed 规则的节点直接丢弃，并在 warnings 里记录节点名、协议和字段原因。
- manual content 导入：复用同一校验器；命中 malformed 规则的节点同样不得进入 inventory。
- profile/global inventory rebuild：即使历史库里还残留坏节点，shared runtime / effective profile 组装路径也只消费校验通过的节点。
- SQLite open：所有 schema/migration 完成后扫描 `subscription_nodes` 与 `proxy_inventory_nodes`，删除命中的坏记录，并保留 import 头记录与 sync 配置可读。

### Edge cases / errors

- 若导入 payload 过滤 malformed 节点后已无任何可用节点或无任何可用 IP，则继续返回现有 `subscription_invalid`。
- 若某个 import 清理后节点数变为 `0`，保留 `proxy_imports` 行，等待后续 refresh 或重新导入恢复。
- 历史 session 若引用被清理的坏节点，由现有 startup reconcile / rebuild 规则收敛，不新增专用 session 迁移。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LoadSubscriptionResponse.warnings | HTTP | external | Modify | None | proxy-broker | web load cards / callers | 仅新增 malformed 节点 warning 文案，不增删字段 |
| Shared runtime node sanitation | Rust API | internal | Modify | None | proxy-broker | service/runtime | 过滤历史 inventory 坏节点 |
| SQLite malformed-node self-heal | DB | internal | Modify | None | proxy-broker | store/service | 打开数据库时删除命中的坏节点行 |

### 契约文档（按 Kind 拆分）

- None

## 验收标准（Acceptance Criteria）

- Given 订阅 payload 包含 `hysteria` 节点 `up: ""` / `down: ""` 与其他合法节点
  When 导入或 refresh 成功
  Then 坏节点不会写入 inventory，response warnings 能看到丢弃原因，其余合法节点继续可用。

- Given 历史 SQLite 中保留了 malformed hysteria inventory 节点
  When 新版本启动并进入 shared runtime / startup reconcile
  Then Mihomo payload 不再因该节点 `/configs 400`，无关 profile 的会话创建恢复成功。

- Given 101 上当前坏节点来自 `Tavily / 江江公益` 订阅
  When 合并、release 并在 101 更新到修复版
  Then `browser` 节点定向会话成功创建，`Tavily` 不再出现 `invalid upload speed` 同类日志，容器保持 `Up (healthy)`。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Rust unit/integration tests：订阅导入过滤、shared runtime 兜底、SQLite 打开自愈。
- Fast-flow PR gates：review proof + CI PR + release workflow + 101 production smoke。

### Quality checks

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- `bun run check`
- `bun run test`
- `bun run verify:stories`
- `bun run build`

## 文档更新（Docs to Update）

- `docs/specs/README.md`
- 本规格 `docs/specs/7aqsu-malformed-hysteria-rate-guard/SPEC.md`
- `/home/ivan/srv/maintenance/*.md`（记录 101 部署与验收）

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 引入 malformed hysteria 速率节点校验器，并在 source/manual 导入路径整节点丢弃 + warnings 落地
- [x] M2: 为 shared runtime / effective profile 组装路径补运行时 guard，并补齐 SQLite 打开自愈
- [ ] M3: 完成回归测试、PR/CI/release/101 更新与维护记录收尾

## 方案概述（Approach, high-level）

- 用一个内部复用的协议感知校验器统一识别 malformed hysteria / hysteria2 速率节点；导入链路与 store/service/runtime 兜底共用同一套判定口径。
- 导入阶段负责阻止新增脏节点入库；runtime 阶段负责保护共享 Mihomo payload；SQLite open 阶段负责清掉历史脏节点。
- 发布后优先依赖新版本自愈逻辑恢复，不把精确 SQL 手修当主路径。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：上游未来可能出现其他协议或其他字段形态的 malformed 节点；本规格仅覆盖当前已证实会炸 Mihomo 的 hysteria / hysteria2 速率字段。
- 风险：若历史 session 恰好引用被清掉的坏节点，恢复期会依赖现有 reconcile 逻辑清退旧 session。
- 假设：Mihomo 官方 Hysteria / Hysteria2 节点配置都以 `up` / `down` 表示速率字段，非空且含数字的标量值应继续视为合法。

## 变更记录（Change log）

- 2026-04-23: 新建规格，冻结“整节点丢弃 malformed hysteria 速率节点 + shared runtime 兜底 + SQLite 自愈 + 101 闭环修复”的范围。

## 参考（References）

- `docs/specs/9hv34-subscription-metadata-import/SPEC.md`
- `docs/specs/s5fwx-proxy-node-groups-live-ops/SPEC.md`
- `docs/specs/98slt-fix-legacy-inventory-sync-migration-crash/SPEC.md`
- `/home/ivan/srv/proxy-broker/proxy-broker.md`
