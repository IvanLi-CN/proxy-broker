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
| r2m7k | PR Label 发版与通配监听 | 已完成 | `r2m7k-pr-label-release-and-wildcard-listen/SPEC.md` | 2026-03-18 | 补齐 label-driven release、GHCR 发布与 `0.0.0.0` 监听链路 |
| s3zu5 | 管理台 UI 控制室重构 | 已完成 | `s3zu5-admin-ui-refresh/SPEC.md` | 2026-03-19 | 侧栏头部紧凑化与 Storybook 视觉证据已补齐，保持 API 合同不变 |
| kgm4c | 安装 UI UX Pro Max Codex 项目技能 | 部分完成（2/3） | `kgm4c-install-ui-ux-pro-max-codex-skill/SPEC.md` | 2026-03-13 | 项目内 Codex skill 已安装，待 PR 收敛 |
