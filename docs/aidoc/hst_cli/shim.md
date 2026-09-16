# hst-cli::shim

shim 三形态自包含状态写入器：cmd/ps1/sh 加 grok 包装（D27/D28/D39）。
用户级状态写入 shim（D27 自包含 + D28 用户级常驻）：hook 与 hst 二进制
解耦，注册与 shim 常驻 `~/.hst/hooks/`（用户裁 2026-09-11「hook 应用户
全局」，对齐 codex 用户层形态），各家 hook 注册指向 shim——state 通道
零 hst 依赖，hst 可任意时刻无痛升级轮换，未 init 的项目也有状态数据。
状态落 `~/.hst/state/` 按 session 分键（D28，防 herdr 多会话互踩）：
默认双写 `<agent>.json`（agent 最新，供无 session 标识的消费面）加
`<agent>-<session>.json`（session 键，状态栏按当前会话直读）；
SessionEnd 删本 session 键文件（GC，崩溃残留由 hst hook 侧陈旧清扫）。
`HST_STATE_FILE` 覆盖优先且互斥（单文件语义，verify 与测试用）。
secretguard 由 shim fail-open 委托加 M060a 白名单：PreToolUse /
UserPromptSubmit 时 hst 在位则转发 payload，仅「exit 2 且 stderr 带
`hst secretguard:` 前缀」判定为自判 block 透传 2 并回放原因；hst 故障的其余非零（升级期坏二进制、CLI 契约漂移含 clap 用法错的 exit 2）一律
fail-open 放行，不在位同放行（state 已写）——护无痛轮换。
`hst hook` 保留为手动入口与委托目标（含完整 notification 形状解析）。
cmd 形态两级（用户裁 2026-09-10：jq 归 ome 部署，shim 部署前探 PATH）：
jq 在位用 jq 解析（转义免疫、ts 取 jq now），缺位回落 findstr 硬解析并
warn 指向 `ark install jq`（ome 更名 Ark 随批）。sh 侧 sed 是 POSIX 基线不引依赖。
session 标识三源：payload `session_id`（claude/codex）、`sessionId`
（kimi）、grok runner 注入的 `GROK_SESSION_ID` env（payload 无该字段）。

## Functions

- `deploy_shims` — # Errors
- `deploy_shims_with` — # Errors
- `host_shell` — 宿主 shell 选择（M060b 抽出成映射）：macOS 落 zsh shebang（缺
- `state_sh_for` — 部署时替换 shebang 行得到 zsh 变体（语义同 bash 形；zsh 无 bashisms 可用

## Constants

- `STATE_CMD` — Windows cmd shim，findstr 回落形态（jq 缺位时部署，warn 指向 ark install
- `STATE_CMD_JQ` — Windows cmd shim，jq 形态（首选）。约定：`%1` = agent 名；stdin = hook
- `STATE_GROK_CMD` — Windows grok 包装（M048 形态：grok 的 command 必须是可整串 spawn 的单
- `STATE_PS1` — Windows PowerShell shim（D39，2026-09-13 宿主实弹）：claude 在 Windows
- `STATE_SH` — POSIX sh shim（Linux = bash、mac = zsh 同一语义；shebang 由部署侧按宿主

