---
id: ADR-0011
title: loop与goal面退役及goal自治
status: accepted
date: 2026-10-11
deciders:
  - raystyle
supersedes: ['ADR-0009（herdr 即时生效通道）']
superseded_by: None
tags: ['定位', 'goal', 'hook', '退役']
---

# ADR-0011:loop与goal面退役及goal自治

## Context

REQ-019 起的 loop/goal 面是 Claude Code 原生 CronCreate 出现前的自建替代：hst 读写项目 `.claude/scheduled_tasks.json`（goal 即任务行 prompt 字段），状态栏同源只读，herdr 即时通道（ADR-0009）补在跑会话生效缺口。2026-10-11 用户确认原生 CronCreate 在常驻会话下实时生效（推翻 2026-09-26 的重启装载阴性实证口径），并令三段：「hst 不再关联 goal 和 loop」「hst 自己用 hook 实现 goal」「自己能管理、自己能控制状态栏显示 goal」。

## Decision

hst 与 `.claude/scheduled_tasks.json` 的一切读写关联解除（属主回归 Claude Code 原生 CronCreate/ScheduleWakeup 域）；loop 管理命令组与状态栏 loop 段撤净（REQ-031 pentest 退役同 precedent，不留迁移桩）。goal 搬家为 hst 自治功能：自有存储 `~/.hst/state/goal/<slug>.json`（项目键 slug 同 trace 与 projyolo），CLI `hst goal set/show/clear` 语义承接；呈现经 Claude Code Stop 事件 hook 腿（claude Stop 与 state 腿双挂，单对 hst-goal shim）：rounds 计数落盘加自然停一轮 `decision: block` 提醒注入（stop_hook_active 防环放行）；状态栏 goal 段同源切自有存储，四家同源显示。herdr 即时通道随 arm 入口出仓，ADR-0001 D15 的局部解除回归全禁（send 面不再有豁免口）。契约细则与 trace 见 REQ-045。

## Consequences

loop/goal 的跨进程耦合面清零：hst 不再写 agent 属主文件（原读改写覆盖风险面消失），goal 数据主权归 hst（schema 自定，原子写）；goal 提醒的时效从 durable cron 周期改为每自然停一轮，防环由 Claude Code stop_hook_active 契约保证（恒无死环，goal 在役期每用户轮次至多多一轮续跑）。代价：用户 toml 显式残留 loop 段 id 会响亮报错（须手改 statusline.toml，changelog 记迁移）；goal 提醒仅 claude 格（decision block 契约专属面）；提醒注入是行为面变化（agent 停止被 goal 在役态续跑一轮），达成与否由 agent 侧自判清空。REQ-020/021/023/024/035/036/041/042 记退役留痕，ADR-0009 转 superseded（本篇互指）。

## 追记 2026-10-11 goal-01 herdr 消费面窄开口

用户澄清令（2026-10-11）：goal 常驻驱动唯一通道 = hst hook（Stop 腿），自愈须事件驱动实时挂在 hook 事件流上，「配合 herdr 可控自愈」。据此本 ADR 的「D15 send 面回归全禁」在单点窄面再开口：hst 作为消费方经 herdr CLI 子进程做两件事（agent list 只读格状态加 agent prompt 单发 goal 唤醒），挂在 Stop 事件流当场判定（盖章限频，零 cron 零落盘轮询零外部定时），子进程有界（list 4s 加 prompt 8s）且不修 herdr 仓、不做事件流长连接、不恢复 spawn/status/serve/mcp（D15 其余禁令不动）。可控三件（频控加豁免加可观测留痕）与裁决形见 REQ-046；pi-rs 停摆现场与根因（会话 hook 快照陈旧加子目录键漂）同在册 REQ-046 根因排查节。
