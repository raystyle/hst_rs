---
id: REQ-016
title: claude注册迁移skills插件形
status: draft
priority: must
trace: 待实施后回填
---

# REQ-016：claude注册迁移skills插件形

## Scenario

REQ-015 裁定：claude 的 hst hook 注册从多写方共享的 `~/.claude/settings.json` 迁到自管的本机插件位 `~/.claude/skills/hst-hooks/`（REQ-015 探针实证可行），消 issue #31 的 clobber 类根因。不可逆部署形态选择，先立 ADR-0009。

## Criteria

- 立 ADR-0009（决策、过渡序、回退面：settings 注册与插件注册的先后摘放序、存量端迁移窗口）
- `hst hook init` 写 `~/.claude/skills/hst-hooks/.claude-plugin/plugin.json` 加 `hooks/hooks.json`（事件面与现行 settings 注册同构，命令仍指 `~/.hst/hooks` shim），并从 `~/.claude/settings.json` 摘 hst 自家条目（herdr 等他方条目不动）
- doctor 的 claude hooks.form 判据改插件面在位；状态栏哨兵探针 claude 臂改指插件 hooks.json（过渡期双探：settings 与插件任一在册即过，存量端摘净后单探插件）
- 状态键超窗静默探针（REQ-015 第 3 条裁定形）：state 非 idle 且状态键 mtime 超 1 小时时探注册，在册无感、缺位才升格 no-hook!
- 自愈腿烘绝对路径（部署时 `std::env::current_exe` 注入生成脚本）加 `~/.hst/statusline.toml` 自愈关闭开关（REQ-015 第 4 条裁定形）
- 四家 `hst agents verify` 集成断言随迁（claude 层改验插件面）
- 实弹验收：摘净 settings.json 全部 hst 条目后，插件钩子仍写状态；`claude plugin disable` 后状态栏如实施加压（退出开关面）

## trace

待实施后回填。
