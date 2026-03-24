# 规格（Spec）总览

本目录用于管理工作项的**规格与追踪**：记录范围、验收标准、任务清单与状态，作为交付依据；实现与验证应以对应 `SPEC.md` 为准。

> Legacy compatibility: historical repos may still contain `docs/plan/**/PLAN.md`. New entries must be created under `docs/specs/**/SPEC.md`.

## 快速新增一个规格

1. 生成一个新的规格 `ID`（推荐 5 个字符的 nanoId 风格，降低并行建规格时的冲突概率）。
2. 新建目录：`docs/specs/<id>-<title>/`（`<title>` 用简短 slug，建议 kebab-case）。
3. 在该目录下创建 `SPEC.md`。
4. 在下方 Index 表新增一行，并把 `Status` 设为 `待设计` 或 `待实现`，并填入 `Last`。

## Index（固定表格）

| ID   | Title | Status | Spec | Last | Notes |
|-----:|-------|--------|------|------|-------|
| h2w7p | Forward Auth 身份识别、管理员授权与 Profile API Key | 已完成 | `h2w7p-forward-auth-admin-and-profile-keys/SPEC.md` | 2026-03-20 | 新增 Forward Auth 身份解析、管理员白名单、开发模式与 Profile 级 API Key 管理 |
| r2m7k | PR Label 发版与通配监听 | 已完成 | `r2m7k-pr-label-release-and-wildcard-listen/SPEC.md` | 2026-03-24 | 补齐 GitHub Release 原生二进制资产、GHCR 发布与 `workflow_dispatch` backfill 路径，并由 `#tqs62` 收敛为 current-first 主线发布 |
| s3zu5 | 管理台 UI 控制室重构 | 已完成 | `s3zu5-admin-ui-refresh/SPEC.md` | 2026-03-19 | 侧栏头部紧凑化与 Storybook 视觉证据已补齐，保持 API 合同不变 |
| kgm4c | 安装 UI UX Pro Max Codex 项目技能 | 部分完成（2/3） | `kgm4c-install-ui-ux-pro-max-codex-skill/SPEC.md` | 2026-03-13 | 项目内 Codex skill 已安装，待 PR 收敛 |
| 6b2xu | Profile catalog 与可新建选择器 | 已完成 | `6b2xu-profile-catalog-combobox/SPEC.md` | 2026-03-19 | 为空 profile 引入持久化 catalog，并把侧栏输入框升级为 searchable combobox |
| y5yx8 | 任务模块与自动订阅维护 | 已完成 | `y5yx8-task-module-and-auto-subscription-maintenance/SPEC.md` | 2026-03-22 | 新增自动订阅调度、任务监控中心与 SSE 实时推送，并补齐 Storybook 视觉证据 |
| tqs62 | 发布链路去饥饿化与专用 Publisher 身份修复 | 已完成 | `tqs62-release-current-first-publisher/SPEC.md` | 2026-03-24 | 主线改为 current-first 发版，GitHub App publisher 接管 tag/release/notes 写入 |
