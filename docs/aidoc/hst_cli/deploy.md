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

- `deploy_all` — # Errors
- `deploy_all_with` — # Errors
- `deploy_user_hooks` — # Errors
- `deploy_user_hooks_with` — # Errors
- `host_side` — 部署的host_side面（细则见 R002 与模块文档）。
- `retire_project_hooks_with` — # Errors

## Types

- `DeployReport` — 部署报告：写入、跳过、形态与警告清单。
- `OsSide` — Which OS consumes a codex registration field: `command` on Unix,

