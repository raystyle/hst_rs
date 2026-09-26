---
id: REQ-024
title: 状态栏第三行loop与goal专属行
status: implemented
priority: must
trace: statusline 单测新增专属行判据一件（三行布局加节拍与 goal 只落三行加一二行干净）加截断测试改 60 加默认段序断言翻转加 cli example 锁同步；实弹部署脚本直跑三行渲染（1×15m 加全文 goal）后任务即清
---

# REQ-024:状态栏第三行loop与goal专属行

## Scenario

用户令（2026-09-26）：loop 与 goal 设置的提示词要在状态栏第三行单独显示（`/loop` `/goal` 专属行）。REQ-019 原把两段放第二行尾（goal 截 16 字符），混行且截短；REQ-024 翻转 D44 第三行默认空裁定为专属行默认位。

## Criteria

验收判据,可检验、可勾选:

- [x] `DEFAULT_SEGMENTS3 = ["loop", "goal"]`（翻转 D44 默认空），`DEFAULT_SEGMENTS2` 撤 loop 与 goal 尾段（`hst/model/context/duration` 四段）
- [x] 无任务时两段皆隐、整行剔除回两行布局（D44 空行剔除机制复用，零噪声不变）
- [x] goal 文本截断 16 放宽 60（专属行独占宽度；UTF-16 代理对防劈保留）
- [x] tools / mcp / tokens 三段仍可显式写 `segments3` 与专属段同线换位或替换（配置语义不破，显式覆盖默认）
- [x] 老配置升级语义不变（effective_orders 判据不动；未写 segments3 的老双排配置默认新增专属行，无任务时整行隐藏）
- [x] 测试：专属行三布局判据新件、截断 60 改件、默认段序断言翻转、example 锁同步（D51）；实弹部署脚本直跑三行渲染
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
