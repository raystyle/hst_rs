---
id: REQ-022
title: 类型契约化推广裁定与compact收尾
status: implemented
priority: should
trace: compact 单测三件（threshold 断言六条锁新类型，5 改加 1 None 原样，含 by_pct 字面双形锁）加推广面实测盘点（String 错误计数加 bool 元组全仓扫描加 Json 载体审查，2026-09-26 diary 流水 12）
---

# REQ-022:类型契约化推广裁定与compact收尾

## Scenario

REQ-021 试点后用户令推广（2026-09-26「都做完」）。推广不铺满：循参考共识「newtype 加 parse 性价比最高，不铺到每个 CRUD」逐面裁收益，改有净收益处，记裁定防后续盲推。

## Criteria

验收判据,可检验、可勾选:

- [x] 推广面实测盘点：全仓 String 错误计数（deploy 19 加 yolo 14 加 statusline 13 加 update 11 等）、bool 元组返回扫描（唯一 (值, 旗标) 冗余形残点 = compact threshold_approx；另 doctor::codex_hooks_sides 返 (bool, bool) 属两独立事实非同类）、Json 载体审查（外部属主长 schema 面名单）
- [x] compact 收尾：threshold_approx 撤 `(u64, bool)` 元组改 `Threshold { ByPct(u64), Default(u64) }` 枚举（bool 与 effective_pct().is_some() 冗余消除；输出串逐字保形不回退）
- [x] 裁定记录（外部属主面）：settings.json 加 claude.json 等外部属主长 schema 文件保持 raw Json 读改写（部分键编辑纪律，flatten typed 承载不适配部分编辑且会重排未知键），此为设计非弱契约
- [x] 裁定记录（错误面）：存量成熟模块不整批 enum 化（错误直达 error_exit 人面，agent 判别走既有 `error=<code>` 前缀契约与结构化 message 面）；新模块与被触碰模块循 loopmgmt 先例顺手补型，非永久禁令
- [x] 结构化短码字段独立项仍归 REQ-011（在册 draft 评估）不抢跑
