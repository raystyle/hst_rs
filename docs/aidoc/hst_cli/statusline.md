# hst-cli::statusline

`hst statusline`：四家状态栏写入面幂等合并与拆段拼装（S025/D18/D42 至 D51）。
agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
- claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
  读改写，保留 env/permissions 等，只覆盖 statusLine 键）
- codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
  `status_line` 为内置项 ID 数组（Codex 无外部命令面，S016）
REQ-038：pwsh 脚本载体完全淘汰（用户令 2026-09-29「powershell 完全淘汰
不用保留」，撤销 REQ-032 的弃用期保留一代）；渲染唯一载体 = 原生
`hst statusline --render`（ADR-0010），grok 的 thin `.cmd` 壳直调原生。
用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；hst 段 = 当前
agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025）。

## Functions

- `cleanup_legacy_script` — REQ-038：PS1 载体完全淘汰的退役清扫（幂等）。摘除弃用期保留的
- `default_icon` — 内嵌默认图标（码位与拆段前脚本逐字对齐；hst 机器人宽字形跟两空格，
- `default_template` — 内嵌默认模板（D18）。键 = 段 id；`context-ascii` 是 grok 的结构差异项
- `effective_orders_pub` — 段序生效值（D42 三行、D44 曾默认两行、REQ-024 起默认三行：第三行
- `merge_claude` — # Errors
- `merge_codex` — # Errors
- `merge_grok` — # Errors
- `merge_kimi` — # Errors
- `read_config` — # Errors
- `read_config_pub` — 原生渲染引擎（ADR-0010）复用口：等价 read_config。

## Types

- `StatuslineConfig` — `~/.hst/statusline.toml` 用户级定制（D18）。键级缺省回落内嵌默认：

## Constants

- `EXAMPLE_TOML` — `hst statusline --example` 打印的带注释全量示例（存到

