---
id: REQ-035
title: 状态栏loop段收养回落
status: implemented
priority: must
trace: statusrender 单测收养回落一件（自有优先加零自有回落加空全量三断言）加 statusline 单测外会话任务翻转为显示改件；实弹 prs_c2coe 工位真 payload 双判据对照（旧判据活会话无行、新判据活会话渲染 30m 巡检行）
---

# REQ-035:状态栏loop段收养回落

## Scenario

用户报（2026-09-28）：prs_c2coe 操作台工位重启后状态栏 loop 行消失，而 durable 任务仍在发。取证：任务 83cb1fd8（*/30 巡检）`createdBySessionId` 指向当天 14:29 已终结的老会话，14:37 起新会话接管工位，任务 15:37 仍触发（lastFiredAt 新鲜、新会话 transcript 命中 37 次），Claude Code 不随收养改写 `createdBySessionId`；状态栏探针按等值过滤，新会话零命中，行隐藏 `[实证: prs_c2coe scheduled_tasks.json 加双会话 transcript 加 statusline --render 双 payload 对照]`。即 durable 任务项目作用域存活与等值归属判据的语义缺口，非数据丢失。

## Criteria

验收判据,可检验、可勾选:

- [x] 归属判据双层：`createdBySessionId` 等值优先；本会话零自有任务时收养回落取项目文件全量（PS1 与原生渲染器同判）
- [x] 自有任务在场时不混入他话任务（等值命中优先；同项目双会话各有自有任务时各显各的）
- [x] 全量也空才真零命中（无任务整行隐藏的零噪声判据不变）
- [x] PS1 侧 tasks 键缺位守卫（防 `@($null)` 计一假阳）
- [x] CLI 管理面不动：`hst loop list` 的 ours 标记与 goal 改写前置仍按等值判据（显示面与管理面判据分面，loopmgmt 模块头在册）
- [x] 测试：statusrender 收养回落新件加 statusline 外会话翻转改件；实弹本机真 payload 双判据对照

## 边界

- 同项目并行双会话、其中一方零自有任务时，该方也显示项目 loop（收养回落代价，接受：durable 任务本就项目作用域发进活会话，显示项目级任务不算误报）。
- goalmode 行（REQ-025）不在本 REQ 范围：goal 由会话内 /goal 设定，老会话终结后新会话未重设则如实无行（本次工位 goal 行消失即此形，重设即回）。
