---
id: REQ-021
title: loopmgmt类型契约化试点
status: implemented
priority: should
trace: loopmgmt 单测七件（回环保留外会话、latest 判据、Cadence 校验加错误短码、goal 回环、错型响亮、任务级未知键幸存、会话解析三级）加 cli 集成（双缺报错加 clap conflicts_with 双给加 bad_every 短码）
---

# REQ-021:loopmgmt类型契约化试点

## Scenario

社区共识（用户令 2026-09-26 参考 Grok 对齐）：契约要做成机器可检查物，类型先红实现后绿，非法状态不可表示。本仓 loopmgmt 现状三处弱契约：`set_loop` 以 `Option<&str>` 加 `Option<&str>` 双旗标承载互斥语义（双缺/双给靠运行时检查）；任务行以裸 `serde_json::Value` 承载（字段错型静默吞默认、schema 漂移不响）；错误全 `String`（调用方与 agent 只能字符串前缀分流）。以 loopmgmt 为试点补型，验证后候裁推广。

## Criteria

验收判据,可检验、可勾选:

- [x] 节拍枚举 `Cadence { Every(&str), At(&str) }` 替换双 `Option` 旗标，互斥在类型层不可表示；双缺/双给检查上移 CLI 解析层，报错文案不回退
- [x] `every_to_cron` 与 `at_to_cron` 撤掉 `(String, bool)` 元组返回（recurring 恒由变体决定：Every 恒 true、At 恒 false），只返 cron 串
- [x] 任务行 `Task` 结构体 typed 承载：核心八字段加 `#[serde(default)]` 容忍缺键、`#[serde(flatten)]` 保未知键读改写幸存；字段错型（如 createdAt 为字符串）由静默默认改为响亮报错
- [x] 顶层 tasks 外兄弟键保真不回退（评审 G1 纪律维持）
- [x] 错误枚举 `LoopError`（Display 面向人、`code()` 恒稳定短码面向 agent），CLI 错误输出带 `error=<code>` 机器可判别面
- [x] 不新增第三方依赖（Display 手写，thiserror 不进 Cargo.toml）
- [x] 单测：既有五件全绿加新增三件（错型响亮、任务级未知键幸存、Cadence 校验），cli 集成绿
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
