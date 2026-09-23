# hst-cli::compact

claude 压缩触发（auto-compact）配置读写面（总台功能单 2026-09-23，ledger n6）。
claude 压缩触发（auto-compact）配置面（总台功能单 2026-09-23，ledger
issue n6）。配置面实证（本机 claude 2.1.270 二进制反混淆，2026-09-23）：
百分比阈值唯一面 = 环境变量 `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`
（parseFloat，0 < 值 <= 100，百分数刻度，"70" 即 70%；二进制内名
testPctOverride，官方文档无此键，未文档面，升级可能变）；窗口面 =
环境变量 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`（裸 token 数，界
[100000, 1000000]，优先于 settings 顶层键 `autoCompactWindow`、
/autocompact 命令与 --autocompact 旗标）；`autoCompactEnabled` 顶层
键缺省 true。阈值算式：有效窗口 = 窗口 - min(模型输出上限, 20000)；
阈值 = min(floor(有效窗口 × pct / 100), 有效窗口 - 13000)；无 pct 时
阈值 = 有效窗口 - 13000（缺省）。`CLAUDE_CODE_AUTO_COMPACT_THRESHOLD`
不存在；`autoCompactThreshold` 是计算态遥测字段非配置键。hst 写面 =
`hst init --compact-pct <1-100|off> [--yes]`（预览不加 --yes），
读面 = `hst doctor` 的 claude compact 检查项。

## Functions

- `apply_pct` — 写 pct（1 至 100）或摘键（None）到用户级 settings env 块，读改写保留
- `read_state` — 读用户级 `~/.claude/settings.json` 压缩触发快照（BOM 容忍，文件

## Types

- `CompactState` — 压缩触发配置快照（原文保留，解析宽容：坏值不报错按缺省口径报告）。

## Constants

- `DEFAULT_RESERVE` — 缺省阈值预留（无 pct 时阈值 = 有效窗口 - 此值）。
- `ENV_PCT` — env 键：百分比阈值覆盖（未文档面，0 < 值 <= 100 百分数刻度）。
- `ENV_WINDOW` — env 键：窗口覆盖（裸 token 数，优先于 settings 键与命令面）。
- `OUTPUT_RESERVE_CAP` — 输出预留上限（阈值算式常量，随二进制实证）。

