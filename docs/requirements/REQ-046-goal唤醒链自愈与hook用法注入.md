---
id: REQ-046
title: goal唤醒链自愈与hook用法注入
status: implemented
priority: must
trace: goal 单测十一件（子目录上溯加存量兼容加 parse 加判定矩阵加 wake 文本当量加假 herdr e2e 含 sweep 三态）加 statusrender 子目录显示扩断言加 cli 集成 goal heal 一件（dry-run 零副作用加 woke 加频控加 blocked 豁免加 show 留痕）；实弹真 herdr 三面（skip-working 三格豁免加 no-agent 孤儿报面加子目录 Stop 注入带用法）加 sweep 内嵌实跑零副作用
---

# REQ-046:goal唤醒链自愈与hook用法注入

## Scenario

pi-rs 工位（长跑解题会话）屏显 `Goal paused · the goal check timed out · send a message to continue` 后停摆：goal 在册未清而无人续驱动。用户令三段（2026-10-11 goal-01 单）加澄清令三条（设计约束级）：goal 常驻驱动唯一通道 = hst hook（Stop 腿），弃 CronCreate 通道；自愈须事件驱动实时挂在 hook 事件流上配合 herdr 可控自愈；可控 = 频控加豁免加可观测。

## 根因排查 现场实证

- **文案出处**：`Goal paused · the goal check timed out · send a message to continue` 是 Claude Code 原生 Goal Mode 的 transcript 系统事件（pi-rs 会话 2b1716ec 在档 22 处加 rate limited 变体 1 处），hst 侧 goalmode 探针词表早收录同族锚（statusrender 的 `Goal paused` system informational 锚）。触发条件 = 原生 goal check（API 支撑的达成判定）超时或限流后 Goal Mode 自暂停，属原生通道自停。
- **hst 腿执行链无超时环节**：shim 透传加二进制启动加单文件读写均为毫秒级，注册 timeout 10s 内无可超时步；实证该会话 goal 实档 `rounds: 0`（Stop 事件从未到达 goal 腿），非腿内超时。
- **根因 A（主因）**：pi-rs 会话 2026-10-03 启动（110MB transcript 在证），其 hook 快照早于 v2.10.0 的 Stop goal 注册（2026-10-11 本机 init）；同快照的 state 腿照常落盘（实档 pretooluse 新鲜）而 goal 腿缺席：运行中会话不热载 settings 变更，须重开会话。澄清令后存义：该工位存量 CronCreate 任务（*/47）已由总台清除，goal 驱动自此单轨走 hst hook 面。
- **根因 B（潜伏）**：hook payload 常只带 cwd，agent 多在项目子目录工作（pi-rs 实证 cwd = 项目/practice/eval），精确 slug 键不中致提醒静默漏发（即使快照新也会漏）。

## Criteria

验收判据,可检验、可勾选:

- [x] 根因 B 修：goal 解析改就近上溯（[`goal_file_for`]，深者胜），Stop 腿与状态栏 goal 段同判；CLI set/clear 管理面仍按给定根精确键
- [x] 澄清令 1（单轨零轮询）：自愈不引入 cron、落盘轮询、外部定时形态；事件驱动 sweep（[`heal_sweep`]）挂 Stop 事件流当场判定，盖章限频 `.heal-sweep`（5 分钟外闸）保零事件开销让位；`HST_GOAL_HEAL=off` 逃生闸
- [x] 澄清令 2（事件驱动实时加 herdr 可控自愈）：goal 腿先出本会话提醒 JSON 并 flush，后跑全量扫描；检测形态 = rounds 无增长加 Stop/idle 时限（静默窗 15 分钟），idle/blocked 态取 herdr 格状态实读非墙钟推断；唤醒经 `herdr agent prompt`（消费方只读 agent list 加单发 prompt，不改 herdr 仓），子进程有界（list 4s 加 prompt 8s，挂起即杀放弃本轮）
- [x] 澄清令 3（可控）:频控 = 每 goal 静默窗（15 分钟）加 sweep 盖章（5 分钟）；豁免 = agent 主动 clear 后无文件即无自愈、blocked 等人裁不唤、working 不扰；风暴闸 = 连续 3 次唤醒无 rounds 增长转人工报面；可观测 = `hst goal show` 出 last_heal_at 加 heal_streak 加 `.heal-log` 单行留痕（ts 加 slug 加 action 加 detail）
- [x] 手动面 `hst goal heal [--project] [--dry-run]`：操作者即时扫描口（非定时形态），dry-run 零副作用零落盘
- [x] 件 3（用法注入）：Stop 提醒与自愈唤醒文本均附 hst goal 用法一句（set/show/clear），agent 可自行改目标与达成后 clear 收束
- [x] 插队件（用户令 2026-10-11「也不要死循环 agent 自己控制这个 goal 停止」）：goal 驱动是 agent 的助手不是约束，agent 对 goal 完全自控（设/改/停）；提醒与唤醒文本的受阻分支补自主停机指引：受阻或判断目标不该继续（推不动、无意义或失效）时可自行 hst goal clear 停止并如实说明，自愈唤醒不得变绑架（频控豁免风暴闸之外，文本面再开停机出口）
- [x] 测试与实弹：单测加集成加假 herdr e2e 全绿；实弹真 herdr 三面（working 三格豁免、no-agent 孤儿、子目录 Stop 注入带用法）加 sweep 内嵌实跑零副作用

## 边界

- 自愈唤醒目标 = herdr 格位（pane id），按 goal 项目键上溯归属判；同项目多格取首个 idle/done 格唤醒，其余格不重复唤。
- 根因 A（快照陈旧）非代码可修：运行中会话不热载 hooks，须重开会话；降级运转下自愈链照常兜底（唤醒文本自带 goal 与用法，agent 可自行 clear 终止；rounds 恒零则风暴闸 3 次后转人工报面）。
- 扫描竞态（多 Stop 并发同扫）：盖章先写后扫，至多多扫一趟，per-goal 频控兜底。
- 本批发布面不动（装机位 2.10.0 无 goal-01 面），推远端加发版候总台转用户裁；pi-rs 工位为现成验证场（检测面已实弹核，唤醒面全链候装机端换装后自证或总台手动 heal）。

实现后回填 frontmatter 的 trace(测试路径或验收命令),状态改 implemented。
