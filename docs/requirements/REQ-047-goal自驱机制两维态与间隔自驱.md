---
id: REQ-047
title: goal自驱机制两维态与间隔自驱
status: implemented
priority: must
trace: goal 单测新增五件(bump_blocked_streak 加 wake v2 当量加 arm 开关加锁三态加快道防抖)加 yolo 单测一件(四家读侧)加集成三件(blocked 分型报警加快道 stdout 零输出加 yolo 治假 herdr 全链);活弹双证(blocked 信任屏报警留痕 + idle sweeper 唤醒注入到 agent 自行 clear 闭环,实录在 diary 2026-10-12)
---

# REQ-047:goal自驱机制两维态与间隔自驱

## Scenario

用户架构批委托(2026-10-12,五件规格):goal 自驱机制从「Stop 提醒加事件驱动自愈」升为完整自驱闭环。背景落差:总台 cron 巡检依赖总台活会话,hst 原生道才自洽;现状 goal 档 rounds/heal_streak 恒 0,heal 从未真火 [实证: 2026-10-12 侦察,`~/.hst/state/goal/` 在册档 heal 三字段全零]。根因有二:其一 sweep 只挂在 Stop 事件流上,死会话(快照陈旧或卡死)零事件即零扫描;其二 blocked 态一律豁免 skip,工位卡审批屏与闲置同为 goal 死因却不分型处置。

## 设计

五件规格对应五节。组件总览:检测源(三路)加判定核(两维态矩阵,纯函数)加处置族(wake/alarm/yolo-heal/report 四型)加驱动面(事件 sweep 加间隔 sweeper 加手动 heal 三驱)加预授权面(yolo-heal arm 开关)。

### 组件

- 检测源三路:Stop 事件流(rounds 计数,会话活性信号,既有);herdr agent list(格位 idle/blocked/working 实态,blocked 主源);Notification 与 PermissionRequest 事件(带内 blocked 快道,payload 权限形即报,零 herdr 依赖)。
- 判定核:`heal_verdict` 扩两维态矩阵(纯函数,输入 goal 档加格态加当下时刻加授权态)。
- 处置族四型:wake(idle 维,herdr agent prompt 注入防死提示词 v2);alarm(blocked 维,报警留痕 .heal-log 加 show 面);yolo-heal(预授权下的 yolo 治,落键加重启格加续 goal);report(no-agent、storm、corrupt,只报不动手)。
- 驱动面三驱:事件 sweep(Stop 与 SessionEnd,盖章限频 5 分钟,既有面扩事件);间隔 sweeper(hook 起有界子进程,单实例锁);手动 `hst goal heal`(操作者即时口,同一引擎)。
- 预授权面:`hst goal yolo-heal on|off` 落 `~/.hst/state/goal/.yolo-heal-armed` 标记(缺省 off,显式预授权才开 yolo 治)。

### hook 形

goal 腿(`hst hook goal`,单对 hst-goal shim)注册面从 Stop 单挂扩为四事件挂(均与 state 腿双挂):Stop(既有)、SessionEnd、Notification、PermissionRequest。分事件行为:

- Stop:提醒注入(既有 additionalContext 形)+ sweep + ensure sweeper。
- SessionEnd:stop_hook 对非 Stop 恒静默,sweep + ensure sweeper(会话收束即扫描时机)。
- Notification 与 PermissionRequest:带内 blocked 快道(payload 权限形,复用 hook 面 state_for_payload 判 blocked)对该项目 goal 报警留痕加 streak 递增;stdout 零输出纪律(不扰权限决策面,exit 0 恒定)+ ensure sweeper。
- 幂等:Stop 提醒 stop_hook_active 防环(既有);快道报警 blocked_alarm_at 防抖。

### 提示词模板

wake v2 自包含五件(单行契约):

1. goal 文本(既有)。
2. 上轮收束态:第 N 轮、上次 Stop 距今约 M 分钟、期间自愈唤醒 S 次未转化。
3. 即刻推进指令:立即选定下一个最小可行动子项并开始执行,不等不空谈;达成即 hst goal clear 收束简报;受阻即说明受阻点。
4. 防卡死防空转边界:同一失败路径不得重复超一次;禁止只汇报不行动;连续两轮无实质推进即判空转,须 hst goal clear 停机如实说明;停机权在 agent。
5. hst goal 用法(set/show/clear,既有)。

blocked 报警不注入提示词(格位卡在选择屏,prompt 会被 herdr 拒 [实证: herdr agent prompt --help 明示 blocked 态提交即拒 agent_blocked]),报警走 .heal-log 留痕加 show 面加 sweeper report。

### 幂等防抖

| 面 | 闸 |
|---|---|
| Stop 提醒 | stop_hook_active 防环加平台 8 连续续跑帽(既有) |
| 事件 sweep | 5 分钟盖章 .heal-sweep(既有) |
| sweeper | 锁文件 .sweeper-lock(create_new 原子建,pid 加 ts,每 tick 心跳重写,90 秒陈旧接管;有界 2 小时生命周期,goal 目录空提前退) |
| wake | 每 goal 15 分钟静默窗加风暴闸 3 次(既有) |
| alarm | blocked_alarm_at 15 分钟防抖窗 |
| yolo-heal | arm 预授权加 blocked_streak 至少 2 加工位非 yolo(读侧判)加 6 小时冷却;非 yolo 前置即天然幂等(治后工位已 yolo,不再触发) |
| streak | blocked_streak 连续目击计数,rounds 增长即归零(blocked_rounds 快照判) |

sweeper 逃生闸:`HST_GOAL_SWEEPER=off` 只关间隔子进程(区别于 HST_GOAL_HEAL=off 关整个 heal 面);测试与诊断面用。

### 两维态矩阵

判定优先序(per goal,纯函数):

| 序 | 判定 | 条件 | 处置 |
|---|---|---|---|
| 1 | no-agent | 无归属格 | report(孤儿,人裁清档) |
| 2 | skip-working | 任人格 working | 不扰 |
| 3 | blocked 维 | 任人格 blocked | alarm(报警留痕);armed 且 streak 至少 2 且工位非 yolo 且过冷却即 yolo-heal |
| 4 | idle 维 | 静默窗过且 rounds 无增长且格 idle/done | wake(风暴闸内) |
| 5 | skip-throttled | 静默窗内 | 频控跳过 |
| 6 | skip-storm-capped | heal_streak 到顶 | 转人工 report |

blocked 先于频控与风暴闸(急症优先,自有 alarm 防抖);快道报警只报不治,yolo 治只出自引擎(须 herdr 格态实证 blocked,防瞬态权限弹窗误治)。

### yolo 治序列

armed 时触发,参考 actl 面 herdr yolo 原语:

1. 落键:`yolo::apply_user_yolo_level_with(Full)`,即 `hst init --yolo full` 的落键半体(四家用户级键,不做全量 redeploy)。
2. 重启格:blocked 格 prompt 被拒,走原始键序 `herdr agent send-keys <pane> esc` 加 `/exit` 加 `enter`,有界轮询 `herdr agent get` 至格回 shell(至多 15 秒)。
3. fresh 起:`herdr agent start <name> --kind <agent> --pane <pane>`(name 缺省按 slug 造 goal-heal 前缀名)。
4. 续 goal:fresh 格 prompt 注入 wake v2 文本。
5. 留痕:.heal-log action=yolo-heal(失败 yolo-heal-fail,同记冷却防锤)。

治面四家 kind(claude/codex/kimi/grok,落键面本就四家齐写);herdr 消费面从「list 加单发 prompt」扩为「list 加 prompt 加 send-keys 加 get 加 start」,仍不改 herdr 仓(REQ-046 边界顺势放宽此四读一写,委托令明示 fresh 起原语)。

## Criteria

验收判据,可检验、可勾选:

- [x] 件 1 结束事件 hook:goal 腿注册面四事件(Stop/SessionEnd/Notification/PermissionRequest),Stop 提醒语义不变,SessionEnd 触发 sweep
- [x] 件 2 间隔循环:hook 起 sweeper 有界子进程(60 秒 tick,2 小时寿命,锁单实例,心跳加陈旧接管),事件旱季(全格 blocked 或全死)自驱不断
- [x] 件 3 两维态:blocked 维分型处置(alarm 留痕加 show 面;不再静默豁免),判定矩阵六序如设计节
- [x] 件 4 防死注入:wake v2 五件自包含模板;blocked 报警留痕(.heal-log 加 show);预授权缺省 off
- [x] 件 5 配合 yolo:yolo-heal arm 开关加治序列五步加留痕;非 yolo 读侧判四家
- [x] `hst goal yolo-heal on|off` CLI 加 show 面新字段(blocked_streak 加 yolo_heal_at 加 armed)
- [x] 单测加集成全绿;`cargo aidoc` 重投影;四 md 门禁绿
- [x] 活弹双证:测试 goal 人为闲置(唤醒注入全链)与人为 blocked(报警留痕全链)各一,herdr 实格实录

## 边界

- sweeper 有界非 daemon:单次寿命 2 小时,续命须新 hook 事件(设计取舍:零常驻进程,REQ-046 零轮询约束在本批由委托令升格为有界间隔,非无限轮询)。
- 快道报警不治:治动作只出自引擎 sweep(herdr 格态实证),瞬态权限弹窗不触发 yolo 治。
- yolo 治不可逆面:落键用户级四家,off 可经 `hst init --yolo off` 退役;治动作全程 .heal-log 留痕。
- GoalRecord 增四字段(blocked_streak 加 blocked_rounds 加 blocked_alarm_at 加 yolo_heal_at),serde default 存量兼容。
- 本批发布面不动(版本定名批另走),装机端换装后自证。

实现后回填 frontmatter 的 trace,状态改 implemented。
