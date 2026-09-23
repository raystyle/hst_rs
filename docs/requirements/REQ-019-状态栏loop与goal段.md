---
id: REQ-019
title: 状态栏loop与goal段
status: implemented
priority: must
trace: loopmgmt 单测五件加 statusline 单测四件加门控扩加 cli 集成三件加实弹（hst loop set/list/del 与 hst goal set/show/clear 回环加部署脚本直跑）
---

# REQ-019:状态栏loop与goal段

## Scenario

舰队用 `/loop` durable 定时任务跑常驻循环,任务起后状态栏不可见,只能翻 `.claude/scheduled_tasks.json` 人工对账;状态栏应显示本会话 loop 计数加节拍,以及最新任务的 goal 文本。扩面(用户令 2026-09-23):hst 要能管理当前会话的 loop 与 goal,包括设置与删除,不自建第二状态文件。

## Criteria

### 状态栏显示面

- [x] 新增 `loop` 与 `goal` 两个独立段 id,可在 `segments2` 配置里分别增删换位(用户裁定 2026-09-23:两段分立)
- [x] 两段进默认第二行(DEFAULT_SEGMENTS2 尾部追加;无任务时整段隐藏零噪声)
- [x] 只统计 `createdBySessionId` 等于当前会话的任务(用户裁定:仅本会话口径,不看项目全部)
- [x] 数据源为项目根 `.claude/scheduled_tasks.json`;项目根解析序:`workspace.project_dir` 回落 `current_dir` 回落 `cwd` 回落 `Get-Location`
- [x] loop 段占位符 `{icon}{count}{cadence}`,cadence 为最新任务 cron 的简单形人性化(`*/N` 分 `×Nm`、小时步进 `×Nh`、每小时 `×1h`、一次性 `@HH:mm`),解析不出留空不留残迹
- [x] goal 段占位符 `{icon}{goal}`,goal 取最新任务(createdAt 最大)的 prompt,截断 16 字符加省略号
- [x] grok ASCII 路径两段无图标字形(`-ascii` 模板变体)
- [x] session-only(非 durable)循环只存在内存,盘上无文件,状态栏不可见(已知边界,写明不隐藏)
- [x] `hst statusline --example` 示例文档与集成测试同步锁(D51)覆盖新段

### 管理命令面

- [x] `hst loop set <goal> --every Nm|Nh` 与 `--at HH:MM` 二选一,读改写保留既有任务(含外会话与 agent 原生落盘项)
- [x] 会话解析序:显式 `--session` 加 `CLAUDE_CODE_SESSION_ID` 环境变量加 `~/.claude.json` 项目表 lastSessionId(键 = 项目根正斜杠形)
- [x] 属主字段同源:会话内跑时 `createdByPid` 取 `CLAUDE_PID`,`createdByProcStart` 读 Linux `/proc` 第 22 字段(liveness 同源形)
- [x] `hst loop list` 全量列出并标 `ours` 归属;`hst loop del` 三目标:id 精确(任意)、latest 本会话最新、all 本会话全部,外会话任务不误删
- [x] 零改动删除合法回执(退 0);双缺或双给节奏旗标报错
- [x] `hst goal set/show/clear`(用户令「hst goal 呢」扩面):改写、回读、清空当前会话最新 loop 的 goal 文本,节奏与属主不动;无本会话任务 set 报错、clear 零改动退 0、show 会话不可解析不受阻

实现后回填 frontmatter 的 trace(测试路径或验收命令),状态改 implemented。
