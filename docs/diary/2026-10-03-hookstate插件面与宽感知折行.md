# 2026-10-03：hookstate插件面与宽感知折行

> 用户报：prs_c2coe 工位 hookstate 行 `session-tool-guard` 被截成 `session-tool-g…`（96 列终端），全局插件 hook（evo-adr 的 md 禁字门禁）整条不显示；同日用户令三别名改连字符形。

## 流水

1. **取证三件（REQ-044）**：一、hooked_aliases 只读 settings 注册面（用户级加项目级 REQ-039），`~/.claude/plugins/` 零覆盖，evo-adr 的 hooks/hooks.json（PostToolUse md-guard.py 带 statusMessage「md 禁字门禁」）整面漏列；二、payload 无宽度字段（官方 schema 对读），Claude Code 跑状态栏脚本前设 `COLUMNS` 是钦定宽源，本机包壳抓 env 实锤 `COLUMNS=96`（临时包 statusLine 命令抓帧后即还原）；三、截断 `…` 是 Claude Code 渲染层行为，hst 侧只能自行折行规避 `[实证: env 抓帧加屏照]`。
2. **插件面并入**：installed_plugins.json（v2 形）列 name@marketplace 到条目数组；enabled 门控读 enabledPlugins（项目 local 大于项目大于用户级首见即用，缺省关，企业 managed 面不入源）；user 作用域全收，project 作用域仅 projectPath 命中当前根者；显示形 = 插件名连字符加 statusMessage（缺席回落干 stem），记 stem 到显示覆写表，已知表 stem 不覆写；collect_json_commands 抽 walk_json_hooks 双出（command 加 statusMessage）共用。过程坑一笔：单测夹具 format! 直插含引号命令产坏 JSON（serde 解析静默零命中，实弹探针 heredoc 手转义形反而通），夹具改 json! 构；plug-d 项目作用域漏项目级 enabled 显式项首跑红，补项目档入链即绿 `[实证: 单测三态件加实弹探针对照]`。
3. **宽感知折行**：COLUMNS 在场时 hookstate 行按显示宽（CJK 双格启发式，nerd PUA 保守 2 格）在 ` | ` 处贪心折行，续行两空格缩进，逐行各自带 SGR（跨行续色不赌渲染器）；单条超宽不硬拆；COLUMNS 缺席维持单行现状形。实弹真 payload（复刻 prs_c2coe 根）：96 列折两行（首行 herdr-agent状态监控 至 knowledge-recall 77 格，续行 evo-adr-md 禁字门禁 至 session-tool-guard 74 格），无 COLUMNS 单行八条全列 `[实证: 实弹渲染前后对照加 cat -v 逐行 SGR]`。
4. **别名改形（用户令）**：herdr agent状态监控 改 herdr-agent状态监控、hst token护栏 改 hst-token护栏、hst 会话状态同步 改 hst-会话状态同步；HOOK_ALIASES 表加四断言加 EXAMPLE_TOML 注释同步。
5. **门禁**：全测 229 加 50 绿（statusrender 26 件含新二）；fmt 加 clippy 零新增（既有 27 条）；aidoc --check --strict 27 生成面无漂移；md 四门禁绿；PEVO PE-08 登记 REQ-044 入索引，PE-11 与基线逐字同（老日记加生成面既有态，非本批引入）。
