---
id: REQ-016
title: claude注册迁移skills插件形
status: draft
priority: must
trace: 待实施后回填
---

# REQ-016：claude注册迁移skills插件形

## Scenario

REQ-015 裁定：claude 的 hst hook 注册从多写方共享的 `~/.claude/settings.json` 迁到自管的本机插件位 `~/.claude/skills/hst-hooks/`（REQ-015 探针实证可行），消 issue #31 的 hook 注册面 clobber 根因。注意收窄：statusLine 装载键仍在 settings.json（本次事故幸存但无保证），同抹时哨兵自身消失、最坏零信号，该面是否纳入本迁或另立待裁。不可逆部署形态选择，先立 ADR-0009。

## Criteria

- 立 ADR-0009（决策、回退面、存量端迁移窗口，加两项：摘放序与过渡双发语义（插件先落 settings 后摘之间同事件双 hook 双发，shim 状态写幂等但 secretguard 双跑，明写接受或排序规避）；版本门槛（skills-dir 自动加载实证于 claude 2.1.270，旧端未验，记最低版本或部署前探测，旧端 hook 静默失效由哨兵红字兜底）
- 交互式首载信任屏实弹补验（REQ-015 探针的零信任屏是 headless 结论）
- `hst hook init` 写 `~/.claude/skills/hst-hooks/.claude-plugin/plugin.json` 加 `hooks/hooks.json`（事件面与现行 settings 注册同构，命令仍指 `~/.hst/hooks` shim），并从 `~/.claude/settings.json` 摘 hst 自家条目（herdr 等他方条目不动）
- doctor 的 claude hooks.form 判据改插件面在位；状态栏哨兵探针 claude 臂改指插件 hooks.json（过渡期双探：settings 与插件任一在册即过，存量端摘净后单探插件）
- 状态键超窗静默探针（REQ-015 第 3 条裁定形）：state 非 idle 且状态键 mtime 超 1 小时时探注册，在册无感、缺位才升格 no-hook!
- 自愈腿烘绝对路径（部署时 `std::env::current_exe` 注入生成脚本）加 `~/.hst/statusline.toml` 自愈关闭开关（REQ-015 第 4 条裁定形）
- 四家 `hst agents verify` 集成断言随迁（claude 层改验插件面）
- 启停态落点考据入 ADR-0009：enabledPlugins 持久化在 ~/.claude/settings.json 共享面（本机四家 evo 插件在册 `[实证]`）；officecli 不在册仍活载即缺省启用 `[实证]`，被抹时插件回缺省启用（对 hst 是 fail-safe 向）；disable 退出开关的持久性与 settings.json 存活相关
- 实弹验收：摘净 settings.json 全部 hst 条目后，插件钩子仍写状态；`claude plugin disable` 后状态键停止更新且哨兵转 no-hook!，`claude plugin enable` 后恢复（退出开关面）

## trace

待实施后回填。
