---
id: REQ-044
title: 状态栏hookstate插件面与宽感知折行
status: implemented
priority: must
trace: statusrender 新件二（hooked_aliases_plugin_face_enabled_gated_and_display 加 wrap_sep_lines_cjk_aware_greedy）加既有件改形四断言；实弹真 payload 渲染（COLUMNS=96 折两行加无 COLUMNS 单行全列加逐行 SGR）
---

# REQ-044:状态栏hookstate插件面与宽感知折行

## Scenario

用户报（2026-10-03）：prs_c2coe 工位 hookstate 行 `session-tool-guard` 被终端截断成 `session-tool-g…`（96 列终端），且全局插件挂的 hook（evo-adr 的 md 禁字门禁）整条不显示。取证：hooked_aliases 只读 settings 注册面（用户级加项目级，REQ-039），`~/.claude/plugins/` 插件面零覆盖；行宽无感知，Claude Code 渲染层按终端列宽硬截 `[实证: 用户屏照加本机 COLUMNS=96 实弹 env 抓帧]`。同日用户令别名改连字符形（herdr-agent状态监控加 hst-token护栏加 hst-会话状态同步）。

## Criteria

验收判据,可检验、可勾选:

- [x] claude 家并入插件注册面：`~/.claude/plugins/installed_plugins.json`（v2 形 plugins 名到条目数组），enabled 门控读 enabledPlugins（项目 local 大于项目大于用户级首见即用，缺省未启用），user 作用域全收、project 作用域仅 projectPath 命中当前根者，逐条读 installPath 下 hooks/hooks.json 收集命令
- [x] 插件 hook 显示形 = 插件名连字符加 statusMessage（hooks.json 自带态文案），statusMessage 缺席回落干 stem；与 settings 面同 stem 去重；未收录 stem 既有字典序殿后语义不变
- [x] COLUMNS 宽感知折行：Claude Code 跑状态栏脚本前设 COLUMNS（官方文档钦定宽源，本机实弹 96 验证），hookstate 行按显示宽（CJK 双格）贪心折行、续行两空格缩进、逐行着色；COLUMNS 缺席不折（现状不变）
- [x] 三别名改连字符形：herdr-agent状态监控加 hst-token护栏加 hst-会话状态同步（用户令 2026-10-03）
- [x] codex 加 kimi 加 grok 不入插件源（无插件体系）；installed_plugins.json 缺席加坏损零命中不炸
- [x] 测试：插件面三态（enabled 加 disabled 加 projectPath 不命中）加折行件加既有别名件改形；实弹真 payload 渲染全 hook 入列不截断

## 边界

- 折行只在 ` | ` 分隔符处断；单条超宽不硬拆（该条仍受终端截断，与现状同）。
- nerd PUA 图标按 2 格保守计宽；宽表是状态栏启发式非全量 East Asian Width 实现。
- 企业 managed settings 的强制 enabledPlugins 面不入源（读用户加项目两级）。
- 已知表 stem（舰队公共别名）不被插件覆写形覆盖，表形优先。
- 生效面：部署位 hst 发布新版后状态栏实装（渲染走 ~/.local/bin/hst，仓内 debug 实弹已验）。
