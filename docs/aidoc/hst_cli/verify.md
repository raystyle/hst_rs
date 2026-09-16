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
- hook：临时目录起无头会话，判据只押 SessionStart / UserPromptSubmit
  这类先于模型调用的事件——模型应答失败（无 token、网络错）不影响判
  定，state 文件落盘即 ok。D28 判据隔离：子进程带
  `HST_STATE_FILE` 指进临时目录（shim 与 hst hook 都认，env 经
  agent 进程继承给 hook 子进程）；万一某家不透传 env，回落扫用户级
  `~/.hst/state/` 里本轮窗口内新写的 `<agent>*.json`。注册走
  **真实用户级面**（四家同一形态，即产品面本身）：byte 备份五件配置、
  deploy、Drop 还原（kimi M058 实证全局触发面泛化到四家，2026-09-11）。

## Functions

- `any_fail` — 任一非跳过项失败即 true（进程退出 1 的判据）。
- `codex_builtin_statusline_ok` — 纯函数：config.toml 的 `[tui]` 段 `status_line`（单行或多行数组）含
- `headless_argv` — 各家的无头命令行（S033 取证）。codex 的 bypass 旗标必须：否则 hook
- `parse_state` — 纯函数：state 文件 JSON 的 state 字段 ∈ 四态且 event 非空才作数。
- `render` — kv marker 行渲染（风格对齐 statusline/install 等现有命令）。
- `run` — 验收主流程：逐家两层，skip（未装）不算失败。
- `statusline_marker_ok` — 纯函数：stdout 任一行含机读标记，两形兼容（D42 三行布局起 agent 态

## Types

- `AgentOutcome` — 单家验收结果（两层各一条；skip 时两层不跑）。
- `LayerVerdict` — 单层验收结论。

## Constants

- `DEFAULT_TIMEOUT_SECS` — 单家无头会话缺省最长秒数。
- `SUPPORTED` — verify 认识的四家（与 agents::SPECS 同集）。

