---
id: REQ-042
title: 状态栏loop全源支持
status: implemented
priority: must
trace: statusrender 单测 session_cron_probe_reads_croncreate_family 新件（会话级 CronCreate 出行加 CronDelete 新于创建判取消加 durable:true 归文件层）加 loop_probe_adoption 扩全局收养断言；实弹 prs_c2coe 真会话 payload（fc60bb08，文件恒空）渲染 30m 行
---

# REQ-042:状态栏loop全源支持

## Scenario

用户报（2026-10-02）：工位跑 `/loop 30m 继续 按进度完成goal`，状态栏 loop 行不显示。取证：该形走 CronCreate（`{"cron":"*/30 * * * *","prompt":"…","recurring":true}`，无 durable 旗标），durable 缺省 false 属会话内存态，`scheduled_tasks.json` 恒空（文件恒空实证）；REQ-036 只盖 ScheduleWakeup 源，边界节在册的「会话级 CronCreate 不在源内（候裁）」正是此形，本 REQ 收口 `[实证: transcript CronCreate 输入对读加文件空态加实弹渲染前后对照]`。

## Criteria

验收判据,可检验、可勾选:

- [x] 会话级 loop 探针源族三标记取新者：ScheduleWakeup 加 CronCreate 加 CronDelete（取消判终，新于创建即隐）
- [x] durable 层两文件并源（用户令「用户级 全局 和 会话内存态都要支持」）：项目级 `<root>/.claude/scheduled_tasks.json` 加用户级全局 `~/.claude/scheduled_tasks.json`（家根会话的 durable 落点，全局 loop 处处可见），等值与收养回落跨两文件取新者
- [x] CronCreate 判定：`durable:true` 属文件层归 None（本探针只在文件层零命中时被咨询）；会话级取 cron 节拍（cron_to_cadence 剥 × 对齐 SW 形）加 prompt（剥 /loop 前缀截 60）；next 回落节拍（会话 cron 无单点下次时刻）
- [x] 测试：新件三断言；实弹真会话 payload 渲染 30m 行

## 边界

- 全局源语义（评审 F1 裁）：`~/.claude/scheduled_tasks.json` 非独立用户级命名空间，是家根会话的 durable 落点（2.1.270 的 CronCreate durable 只有项目文件档）；「他话全局任务在任何项目处处可见」是用户令「全局都要支持」的裁定形非默认语义；家根会话两路径同文件去重（评审 F1 修复），全局 home 解析失败只跳过全局源不炸项目层。
- goalmode 词面（评审 G 完备扫收口）：retry 形 system informational（`Goal still active · the goal check could not complete · retrying`）产 Active 态；`/goal clear|off|stop` 用户指令载体真语料零命中（真实清态全走 lcs `Goal cleared:`），两形已在词表仅未以真样本锁形，备查。
- cron_to_cadence 补 offset-step 分钟形（`3-59/23` 按步进取节拍，真语料在证）；绝对日期形（`12 7 30 9 *`）无周期义回落空，候裁。
- CronDelete 未按 jobId 配对（transcript 的 job id 在 tool_result 载体，配对复杂度高）；任何新于最后一次 CronCreate 的 CronDelete 判取消，误伤面 = 同会话先建会话 cron 再删旧 durable 件的序（文件层与会的删除交错，概率低，记档）。
- 一次性会话 cron（recurring:false）同显（触发即自删的短窗如实显示）。
- durable:true 且文件被外部删除的窗口期：文件层零命中时会话层也归 None，行隐（durable 层语义优先，接受）。
