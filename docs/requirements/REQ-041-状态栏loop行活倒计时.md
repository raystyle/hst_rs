---
id: REQ-041
title: 状态栏loop行活倒计时
status: implemented
priority: must
trace: statusrender 单测 session_loop_probe_verdicts_four_forms 扩件（空输入 SW 取消判终加 stop 终态优先加三元组免墙钟断言）；实弹夹具会话双时点渲染倒计时跳变（24m 到 23m）
---

# REQ-041:状态栏loop行活倒计时

## Scenario

用户令（2026-10-01，REQ-040 同日续）：「loop 的状态栏也要刷新」「主要是 loop 和 goal 刷新实时状态，不要删除取消后还在状态栏挂起」。取证：活会话 3e21eee2 末条 ScheduleWakeup 为空输入退化调用（无 delaySeconds 无 stop，2026-10-01T08:16Z 实证）；且行内 `{every}` 是静态节拍，不随时钟走 `[实证: transcript 空条目对读加夹具双时点渲染]`。

## Criteria

验收判据,可检验、可勾选:

- [x] 判定改两态：Active 加 Terminal；空输入退化调用按用户裁定 = 取消（布了空定时即不再排程，循环即止行立隐，即使更早布防仍在滞隐窗内，防取消后挂起）；stop 显式终与滞隐同判终
- [x] `{next}` 新占位符 = 活倒计时（ts 加 delay 减 now，负值钳 0，timestamp 取不到回落节拍）；durable 面缺省值 = 节拍（无单点下次时刻），显示与旧版一致
- [x] 缺省 loop 模板 `{every}` 换 `{next}`：durable 行显示不变，会话行每帧走到点随渲染刷新
- [x] 测试：既有件扩（空输入取消判终加 stop 终态优先加三元组免墙钟断言）；实弹夹具双时点渲染倒计时跳变

## 边界

- 倒计时粒度随节拍人性化档（秒分级），渲染频率随状态栏渲染事件（不独立走秒）。
- 空输入 SW 语义按用户裁定 = 取消（Claude Code 工具面全参可选；上游若明文赋予其它语义需回填）。
- 滞隐窗（两心跳）内已过期的会话 loop 显示 0s（到期未续的真话形）。
