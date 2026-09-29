---
id: REQ-036
title: 状态栏会话级loop段
status: implemented
priority: must
trace: statusrender 单测 session_loop_probe_verdicts_four_forms（末条胜加 stop 判终加滞隐加 durable 优先集成四断言）加 statusline 单测 session_loop_row_falls_back_when_no_durable（PS1 面活态出行加 stop 行隐）；实弹 prs_c2coe 真会话 payload 渲染 `25m / 确认解题后沉淀了browse skill和解题步骤`
---

# REQ-036:状态栏会话级loop段

## Scenario

用户报（2026-09-29）：prs_c2coe 工位跑会话级 `/loop` 动态自调度（ScheduleWakeup，非 durable cron），状态栏 loop 行不显示。取证：durable 任务已按令删除（`scheduled_tasks.json` tasks 空），会话 loop 状态只活在本会话调度器内存与 transcript（`"name":"ScheduleWakeup","input":{"delaySeconds":1500,"prompt":"/loop …","reason":"…"}`，条目带 timestamp），hst 既有探针无此源 `[实证: prs_c2coe transcript 4 条工具调用对读加 scheduled_tasks.json 空态]`。

## Criteria

验收判据,可检验、可勾选:

- [x] loop 行归属三层：durable 等值优先、durable 收养回落（REQ-035）、零 durable 时会话级 ScheduleWakeup 回落（补齐「本会话实际在跑的 loop」完整语义）
- [x] 源 = 会话 transcript 倒序分块反扫（4MB 块加 512B 跨界重叠，与 goalmode 探针同技术），末条工具调用先中即锁；取 delaySeconds 与 prompt（剥 `/loop` 前缀截 60，代理对防劈）
- [x] 判终两形：`stop:true` 显式终；就近回取条目 timestamp，now 超 ts 加两倍心跳未续期视为弃约滞隐（timestamp 取不到不判龄，乐观显）
- [x] 节拍人性化秒分级（`Xs` 加 `Xm` 加 `XhYm` 加 `XdYh`），复用 loop 段模板图标，count 记 1；零新段 id 零配置迁移
- [x] PS1 与原生渲染器同判（转义解序 PS1 侧常见形，原生侧全形含 `\u` 代理对）
- [x] durable 在场时会话不混入（等值与收养层优先）
- [x] 测试：原生四形单件加 PS1 面两态件；实弹真会话 payload

## 边界

- 会话级（非 durable）CronCreate 任务不在源内（transcript 形未实证，候裁）；本 REQ 只盖 ScheduleWakeup 面。
- 自治 loop 的哨兵 prompt（`<<autonomous-loop-dynamic>>`）显示为原文本（goal 段空串语义同 durable 空合法态）。
- timestamp 回取限距（Rust 就近无界至块首、PS1 LastIndexOf 限窗）：条目 timestamp 距标记超窗或跨界劈开时不判龄乐观显；Rust 侧 ts 回取不限行界，标记行跨界劈开时会取到上一条目 ts（判龄偏老的理论假阴，活态不误隐：上一条 ts 加两心跳约等于本条 ts 加一心跳恒大于 now；评审 G3）。滞隐判据依赖模型按心跳续期的纪律，静默弃约最长滞显两倍心跳时长。
- 标记假阴残余（评审 G2）：更新的未再编码嵌套对象引文排在真行之后且无 delaySeconds 字段时，末中即锁会把更老的真行掩掉；转义引文形实测不命中，概率极低，收紧面（tool_use 前锚校验或假阴续扫）候裁。
- goal 段（可选段）仍只取 durable 源，会话级 goal 文本只在 loop 行显示。
