---
id: REQ-020
title: doctor增loop与goal健康检查
status: implemented
priority: should
trace: loopmgmt 单测 health 两件加 g8 一件加 doctor 单测三态三件（零任务 ok 加坏损 warn 加死属主 warn）
---

# REQ-020:doctor增loop与goal健康检查

## Scenario

舰队用 durable loop 常驻跑循环，但 doctor 对 scheduled_tasks.json 零覆盖（2026-09-26 实查：doctor 77 行五族检查无一项碰 loop 与 goal，diagnose 为网关活性面正交）；文件坏损、死属主孤儿任务、外会话任务堆积时诊断盲区，用户只能翻文件人工对账。

## Criteria

验收判据,可检验、可勾选:

- [x] doctor 的 claude 族新增 check=loop 行：读项目根 `.claude/scheduled_tasks.json`，缺文件或空 tasks 判 ok（零任务合法态，同 yolo 未配置口径）
- [x] 文件在场但不可解析判 warn（同 yolo.parse 形，detail 点名路径与原因）
- [x] Linux 属主活性子面：createdByPid 非零且 `/proc/<pid>` 目录不在、或 createdByProcStart 落盘非空且与 stat 第 22 字段不匹配的任务计 dead_owner（目录在而 stat 不可读如 hidepid 属未知不判死；procStart 落盘为空只查进程在否），大于零判 warn；非 Linux 平台该子面降 ok 加无判据注记（Status 模型无 info 档，同 compact 信息型先例）
- [x] detail 行含任务计数加 dead_owner 计数；会话可解析时附本会话 ours 计数与外会话计数（堆积对账面，info 性质不告警；非 Linux 的 ok 分支以无判据注记替代 dead_owner 计数）
- [x] doctor 单测三态三件（diagnose 直调）：正常（ok）、坏损（warn）、死属主（warn）
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
