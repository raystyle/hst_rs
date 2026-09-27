# hst-cli::shim

shim 三形态自包含状态写入器：cmd/ps1/sh 加 grok 包装（D27/D28/D39）。
用户级 hook shim（D27 自包含 + D28 用户级常驻 + REQ-028 双脚本拆分）：
hook 与 hst 二进制解耦，注册与 shim 常驻 `~/.hst/hooks/`（用户裁
2026-09-11「hook 应用户全局」，对齐 codex 用户层形态），各家 hook 注册
指向 shim——state 通道零 hst 依赖，hst 可任意时刻无痛升级轮换，未
init 的项目也有状态数据。
REQ-028（用户令 2026-09-27「hst-state 不应该双职责 拆分成2个脚本和
hst hook payload穿透命令 以后还要扩展很多hook命令」）：一命令一脚本
（`hst hook state` 单对 hst-state、`hst hook token` 单对 hst-token，
未来 `hst hook <x>` 循此式）。state 载体纯四态写盘；token 载体纯密钥
拦截（fail-open 委托加 M060a 白名单：PreToolUse / UserPromptSubmit 时
hst 在位则转发 payload，仅「exit 2 且 stderr 带 `hst secretguard:` 前
缀」判定为自判 block 透传 2 并回放原因；hst 故障的其余非零（升级期
坏二进制、CLI 契约漂移含 clap 用法错的 exit 2）一律 fail-open 放行，
不在位同放行——护无痛轮换）。`hst hook status` 双职责入口弃用期一代
（老注册兼容），`hst hook` 族保留为手动入口与委托目标。
状态落 `~/.hst/state/` 按 session 分键（D28，防 herdr 多会话互踩）：
默认双写 `<agent>.json`（agent 最新，供无 session 标识的消费面）加
`<agent>-<session>.json`（session 键，状态栏按当前会话直读）；
SessionEnd 删本 session 键文件（GC，崩溃残留由 hst hook 侧陈旧清扫）。
`HST_STATE_FILE` 覆盖优先且互斥（单文件语义，verify 与测试用）。
cmd 形态两级（用户裁 2026-09-10：jq 归 ome 部署，shim 部署前探 PATH）：
jq 在位用 jq 解析（转义免疫、ts 取 jq now），缺位回落 findstr 硬解析并
warn 指向 `ark install jq`（ome 更名 Ark 随批）。sh 侧 sed 是 POSIX 基线不引依赖。
session 标识三源：payload `session_id`（claude/codex）、`sessionId`
（kimi）、grok runner 注入的 `GROK_SESSION_ID` env（payload 无该字段）。

## Functions

- `deploy_shims` — # Errors
- `deploy_shims_with` — # Errors
- `host_shell` — 宿主 shell 选择（M060b 抽出成映射）：macOS 落 zsh shebang（缺
- `pentest_sh_for` — PENTEST_SH 的 shebang 互换（同 state_sh_for）。
- `state_sh_for` — 部署时替换 shebang 行得到 zsh 变体（语义同 bash 形；zsh 无 bashisms 可用
- `token_sh_for` — TOKEN_SH 的 shebang 互换（同 state_sh_for）。

## Constants

- `PENTEST_CMD` — REQ-031 pentest 腿 shim 的 cmd 载体（授权获取命令短路；白名单前缀
- `PENTEST_GROK_CMD` — REQ-031 pentest 腿的 grok 单路径包装（M048 同款，与 state/token 包装
- `PENTEST_PS1` — REQ-031 pentest 腿 shim 的 ps1 载体（同 token 形；BOM 落盘语义同 D39）。
- `PENTEST_SH` — REQ-031 pentest 腿 shim（授权获取命令短路服务，白名单透传形同 token
- `STATE_CMD` — Windows cmd shim，findstr 回落形态（jq 缺位时部署，warn 指向 ark install
- `STATE_CMD_JQ` — Windows cmd shim，jq 形态（首选）。约定：`%1` = agent 名；stdin = hook
- `STATE_GROK_CMD` — Windows grok 包装（M048 形态：grok 的 command 必须是可整串 spawn 的单
- `STATE_PS1` — Windows PowerShell shim（D39，2026-09-13 宿主实弹）：claude 在 Windows
- `STATE_SH` — POSIX sh shim（Linux = bash、mac = zsh 同一语义；shebang 由部署侧按宿主
- `TOKEN_CMD` — REQ-028 token 腿 shim 的 cmd 载体（密钥拦截单职责；透传白名单同 sh 形）。
- `TOKEN_GROK_CMD` — REQ-028 token 腿的 grok 单路径包装（评审 F1，M048 同款：Windows grok
- `TOKEN_PS1` — REQ-028 token 腿 shim 的 ps1 载体（密钥拦截单职责；M060a 白名单透传
- `TOKEN_SH` — REQ-028 token 腿 shim（密钥拦截单职责，与 hst-state 拆分）：读 payload

