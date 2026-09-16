# hst-cli::statusline

`hst statusline`：四家状态栏写入面幂等合并与拆段拼装（S025/D18/D42 至 D51）。
agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
- claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
  读改写，保留 env/permissions 等，只覆盖 statusLine 键）
- codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
  `status_line` 为内置项 ID 数组（Codex 无外部命令面，S016）
状态栏脚本本体（pwsh）随 hst 释放到 `~/.hst/statusline/`。
用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；hst 段 = 当前
agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025），另探测
agent 宿主 shell（macOS 走 ps 兜底）。

## Functions

- `custom_active` — 自备脚本当前是否在场（kv 面 statusline.custom 用）。
- `deploy_custom_script` — # Errors
- `deploy_script` — # Errors
- `merge_claude` — # Errors
- `merge_codex` — # Errors
- `merge_grok` — # Errors
- `merge_kimi` — # Errors
- `pwsh_on_path` — pwsh is the statusline runtime on every platform. Advisory only: the
- `read_config` — # Errors
- `restore_builtin_script` — # Errors

## Types

- `StatuslineConfig` — `~/.hst/statusline.toml` 用户级定制（D18）。键级缺省回落内嵌默认：

## Constants

- `EXAMPLE_TOML` — `hst statusline --example` 打印的带注释全量示例（存到

