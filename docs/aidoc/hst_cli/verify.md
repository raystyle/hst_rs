# hst-cli::verify

`hst agents verify`：四家无头验收两层判据（D17/S033）。
`hst agents verify`（D17）：四家 agent 的 hook 与状态栏全平台无头验收。
两层判据（S033 源码实证底座；D28 起注册面全量用户级）：
- 状态栏：**验收已部署的面**（D37，2026-09-13 wsl 总台验收适配）：脚本
  本体直跑（mock 空 JSON 喂 stdin，断言 stdout 任一行含 `agent:state`
  机读标记（D42 三行布局起 agent 态在第二行；D46 起可带连字符版本形
  `agent-<version>:state`），S025；脚本由 verify 按
  需释放）；codex 无外部命令面（M045），断
  `~/.codex/config.toml` 的 `[tui] status_line` 含内置项 ID。面未部署
  （无 `[tui] status_line`）或可选运行时缺位（pwsh 不在 PATH）= skip
  不计败、带 CTA（状态栏是可选面，doctor 另有 warn）；已部署但断链
  （marker 缺、内置项缺 run-state 锚、脚本非零退出）仍 fail。
- hook：临时目录起无头会话。entropy-01（2026-10-12 用户令）：hst-state
  腿退役（herdr 的 herdr-agent-state hook 已覆盖会话状态同步），运行时
  可观测腿只剩 claude 的 goal 面——claude 判据改押 task rounds（Stop
  事件端到端：临时 task 在册，无头会话收束后 rounds 递增即 ok，判据只
  看计数不看应答内容）；codex/grok/kimi 无运行时可观测腿，skip 带注
  （注册面形态由 hst doctor 的 hooks.form 验）。注册走**真实用户级
  面**：byte 备份五件配置、deploy、Drop 还原。

## Functions

- `any_fail` — 任一非跳过项失败即 true（进程退出 1 的判据）。
- `codex_builtin_statusline_ok` — 无头验收的纯函数面（细则见模块文档与集成测试）。
- `headless_argv` — 各家的无头命令行（S033 取证）。codex 的 bypass 旗标必须：否则 hook
- `render` — kv marker 行渲染（风格对齐 statusline/install 等现有命令）。
- `run` — # Errors
- `statusline_marker_ok` — 无头验收的纯函数面（细则见模块文档与集成测试）。

## Types

- `AgentOutcome` — 单家验收结果（两层各一条；skip 时两层不跑）。
- `LayerVerdict` — 单层验收结论。

## Constants

- `DEFAULT_TIMEOUT_SECS` — 单家无头会话缺省最长秒数。
- `SUPPORTED` — verify 认识的四家（与 agents::SPECS 同集）。

