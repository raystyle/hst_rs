# hst-cli::deploy

`hst init` 部署层：hook 注册四家用户级、shim 落位、SKILL 命令图与状态栏面（D53）。
`hst init` hook/skill deployment（D28：hook 注册与 shim 常驻用户级）。
用户级注册面（用户裁 2026-09-11「hook 应用户全局」，对齐 codex 用户层）：
claude `~/.claude/settings.json`（settings 家族用户层生效，S015）、codex
`~/.codex/hooks.json` 加 `~/.codex/config.toml` features 与 trusted_hash
预种、grok `~/.grok/hooks/ohmyagents-state.json`（global 层）、kimi
`~/.kimi-code/config.toml [[hooks]]`（kimi 仅用户级，S015）。shim 常驻
`~/.hst/hooks/`，状态按 session 分键写 `~/.hst/state/`（D28）。项目级
旧注册与 `.oma/hooks/` 由 init 迁移退役（未 init 项目零数据根因消除）。
skills 与 AGENTS/CLAUDE 说明仍是项目级（项目内语义）。Schemas are
first-hand verified in S015 (official docs + openai/codex, xai-org/grok-build,
MoonshotAI/kimi-code sources).

## Functions

- `deploy_all` — Deploy the full init surface (D28)：用户级 hook 注册加 shim（真实家目录
- `deploy_all_with` — Test seam：user_home 与 oma 根注入（不碰真实家目录），side 注入双测。
- `deploy_user_hooks` — 生产入口：真实家目录 + hst 自管根。
- `deploy_user_hooks_with` — 用户级部署总入口（可注入：测试传临时 user_home 与 oma 根；生产传真实
- `host_side` — host_side：init 部署层的公开入口（行为细则与 marker 见 R002）。
- `retire_project_hooks_with` — 项目面退役（D28）：摘除项目级 ours hook 注册（claude/codex/grok）、

## Types

- `DeployReport` — DeployReport：init 部署层的数据面。
- `OsSide` — Which OS consumes a codex registration field: `command` on Unix,

