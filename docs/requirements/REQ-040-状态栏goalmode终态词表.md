---
id: REQ-040
title: 状态栏goalmode终态词表
status: implemented
priority: must
trace: statusrender 单测 scan_markers_req040_terminal_forms 新件（lcs 设标剥尾加 cleared 判终加 met:true 判终加 met:false 零标记加时序）；实弹 prs_c2coe 真会话 payload（3e21eee2，末态 cleared）行由冻显旧文转隐藏
---

# REQ-040:状态栏goalmode终态词表

## Scenario

用户报（2026-10-01）：prs_c2coe 工位 goal 状态不刷新：goal 已清，状态栏 goalmode 行仍显示旧设标文本。取证：该会话 transcript 末态为 `Goal cleared`（TUI 斜杠路径回执，`type:system` 的 `<local-command-stdout>` 载体）×2 与 `goal_status` 附件 `met:true`（权威态载体），两形均不在 REQ-025 加 REQ-037 词表内，倒扫最后命中停留在早前 `Goal set:`，行冻在旧文本 `[实证: 3e21eee2 transcript 标记序对读加实弹渲染前后对照]`。REQ-037 评审 G8 曾记 goal_status 形候裁，本 REQ 收口（并补 lcs 载体形）。

## Criteria

验收判据,可检验、可勾选:

- [x] 词表补三形：`"content":"<local-command-stdout>Goal set: `（取文本剥 `</local-command-stdout>` 尾）、`"content":"<local-command-stdout>Goal cleared: `（判终）、`"type":"goal_status","met":true`（判终达成）；各带冒号后空格双形（G2 纪律）
- [x] `met:false` 是活态确认，不产标记（不干扰回溯的设标文本）
- [x] 判终语义 = Clear（行隐藏），与既有 clear 加 paused 时序回溯兼容（set 后 cleared 取新者）
- [x] 测试：新件五断言加时序；实弹真会话 payload 行由冻显转隐藏

## 边界

- goal_status 的 condition 字段不作文本源（设标与 check-in 形已载文本；condition 与 prompt 同文本时冗余，不同步时以显式设标为准）。
- met:true 后若再设新 goal，set 标记更晚自然翻回 active（时序回溯既有语义）。
- lcs 载体的 system 型行可能被会话引文再编码（字符串值内嵌形不中，既有防线）。
