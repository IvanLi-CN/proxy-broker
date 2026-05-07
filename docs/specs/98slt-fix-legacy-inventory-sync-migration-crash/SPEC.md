# 修复 legacy inventory import 触发的 SQLite 启动迁移崩溃（#98slt）

## 状态

- Status: 已实现（shared testbox 已验证）
- Created: 2026-04-20
- Last: 2026-04-20

## 背景 / 问题陈述

- 101 上 `proxy-broker` 在升级到短 ID 版本后进入 crash-loop，Traefik 对 `proxy-broker.ivanli.cc` 回落到默认 `404`。
- 线上容器日志持续报错：`failed to initialize sqlite ... unsupported project sync source type: inventory`。
- 现场 SQLite 数据显示：`proxy_imports` 中仍保留 legacy `source_type=inventory` 的 import 记录，而 `proxy_import_sync_configs` 里的同步来源仍是合法的 `url`；当前短 ID 迁移会把 import 的 `source_type/source_value` 覆盖写回 sync config，导致启动时再次解析 sync source 失败。

## 目标 / 非目标

### Goals

- 修复 short-id 启动迁移，避免把 legacy import 的 `inventory/legacy_scope/manual` source 误写进 `proxy_import_sync_configs`。
- 补充 SQLite 回归测试，覆盖“legacy inventory import + url sync config + stable short-id row 并存”的真实崩溃路径。
- 把 101 的真实 SQLite 资产复制到共享测试机复现，并验证修复后服务可以成功启动、数据库可完成迁移。
- 按快车道完成 PR、CI、合并、发布与 101 线上修复收尾。

### Non-goals

- 不改动代理分配模型或 import/node 业务语义。
- 不对 Web UI 做视觉改动。
- 不在生产上做无补丁前提的手工数据热修，除非补丁验证失败且主人另行授权。

## 范围（Scope）

### In scope

- `src/store/sqlite.rs` 中 short-id / sync-config 迁移逻辑。
- Rust 回归测试、共享测试机复现脚本与最小运维记录。
- PR、CI、release workflow 与 101 上的 compose 更新验证。

### Out of scope

- 额外 schema 设计或新的迁移批次。
- 与本次 crash 无关的 UUID/short-id 后续重构。

## 需求（Requirements）

### MUST

- `proxy_import_sync_configs` 的 `source_type/source_value` 必须继续只承载合法的订阅源（当前为 `url/file`），不得被 legacy inventory import source 覆盖。
- 对已存在 stable short-id sync config 的旧 UUID / legacy import 记录，迁移必须可幂等执行并能安全合并到正确的 stable import。
- 必须用 101 的真实 SQLite 资产在共享测试机验证“修复前可复现、修复后可启动”。
- 快车道终点为 `merge+cleanup`，且在 101 上完成更新和健康验证后才算收工。

### SHOULD

- 保持修改最小、可回滚，并为真实资产验证保留清晰命令记录。
- 把新增测试收敛到 store/sqlite 迁移回归，不引入无关耦合。

## 验收标准（Acceptance Criteria）

- Given legacy SQLite 中存在 `proxy_imports.source_type=inventory` 且 `proxy_import_sync_configs.source_type=url`
  When 新版本打开数据库执行迁移
  Then 服务不会再因 `unsupported project sync source type: inventory` 崩溃，sync config 仍保持可解析订阅源。

- Given 101 上的真实 `state.sqlite` 资产被复制到共享测试机
  When 在共享测试机运行修复后的构建/容器启动
  Then 服务能成功通过健康检查并完成 SQLite 初始化。

- Given 修复分支进入快车道
  When 完成 review proof、CI、合并、release 与 101 更新
  Then `proxy-broker.ivanli.cc` 恢复到应用鉴权响应，容器保持稳定 `Up (healthy)`。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Rust targeted tests：SQLite migration regression for legacy inventory imports / sync configs。
- Shared testbox validation：使用 101 真实 `state.sqlite` 资产验证修复前后行为差异。
- Fast-flow PR gates：review proof + CI + release workflow + 101 production smoke。

### Quality checks

- `cargo fmt --all`
- `cargo test`
- 共享测试机上的真实资产启动验证

## 文档更新（Docs to Update）

- `docs/specs/README.md`
- 本规格 `docs/specs/98slt-fix-legacy-inventory-sync-migration-crash/SPEC.md`
- 若线上 steady state 或修复流程有新增事实，则补一条 `/home/ivan/srv/maintenance/*.md`

## 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 修复 SQLite 迁移逻辑并补本地回归测试
- [x] M2: 共享测试机用 101 真实数据库资产复现并验证通过
- [ ] M3: 快车道 PR/CI/release/101 更新全部完成

## 方案概述（Approach, high-level）

- 让 `migrate_short_ids()` 在改写 `proxy_import_sync_configs` 时只迁移 `import_id`，并把 sync source 的规范化继续交给 sync-config 自己的 source 解析 / merge 路径处理。
- 用最小化的 SQLite fixture 覆盖“legacy inventory import + stable sync config 并存”场景，再用 101 真实数据库资产在共享测试机验证。
- 合并后等待 release workflow 发布新的 GHCR `latest`，再按 101 部署卡执行 `docker compose pull && up -d` 和健康检查。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：101 真实数据库里可能还带有本地测试未覆盖的历史 shape；因此必须做共享测试机真实资产验证。
- 风险：release workflow 可能排队或失败，快车道需要持续盯到真正产出可部署镜像。
- 假设：当前生产恢复仍可通过发布新镜像并执行常规 compose 更新完成，无需额外数据修复。

## 变更记录（Change log）

- 2026-04-20: 新建规格，锁定“修复 legacy inventory import 导致的 short-id 启动迁移 crash，并完成真实资产验证后上线”的范围。

## 参考（References）

- `docs/specs/2e86e-short-id-without-uuid/SPEC.md`
- `docs/specs/qvbmc-proxy-import-allocation/SPEC.md`
- `/home/ivan/srv/proxy-broker/proxy-broker.md`
- `/home/ivan/srv/maintenance/2026-04-19-ops-proxy-broker-v0.10.1-legacy-sqlite-migration-recovery.md`
