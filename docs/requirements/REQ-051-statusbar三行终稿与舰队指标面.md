---
id: REQ-051
title: statusbar三行终稿与舰队指标面
status: draft
priority: must
trace: null
---

# REQ-051:statusbar三行终稿与舰队指标面

## Scenario

用户令(2026-10-13 状态栏令升级,授权 hst 全面调整规划):终稿三条并入(首行首位 agent 类型加版本、hook 零中文、task 循环时间)加新增全局 agents 指标入栏(舰队 N 格/W 跑/I 闲紧凑计数)。本单为规划面:REQ 加三行线框 mock 先交总台快核,核过再实现。

## 设计 提案稿

### 三行结构定稿 现行五段行收敛为三行

现行五段行(项目行、agent 态行、hookstate 行、task 行、goalmode 行)收敛为三行;字段序与行归属:

- **L1 身份与项目行**:`agent 类型加版本`(首位,用户令)接 shell 加 dir 加 git 加 package 尾巴加 clock。agent 版本取 payload version 归一(现行 seg_hst 的版本归一逻辑随迁),态色不动(仍按 state 上色整行)。
- **L2 运行时行**:agent 态(agent:state)加 model 加 context 加 duration 加 **fleet 段**(舰队紧凑计数,新)。
- **L3 hst 域行**:hookstate(英文 id 清单,零中文)加 task(文本加轮次加**循环时间**)。goalmode 退化为 L3 前缀旗(active 加 paused),在役才显,无 goalmode 零占位。

**宽度与窄窗退化**(右起逐段弃,保底恒显 agent 加版本与 agent 态):

| 窗宽 | 退化动作 |
| --- | --- |
| 小于 100 列 | clock 弃;package 尾巴弃(留主 package);task 文本截 16 字省略 |
| 小于 80 列 | dir 取 basename;fleet 弃;hook 清单截首三个 |
| 小于 60 列 | git 弃;model 弃;task 文本截 8 字;hook 清单只留首一个 |
| 小于 40 列 | 只保 L1(agent 加版本加 dir basename)与 L2(agent 态) |

### 全局指标快照道 渲染零网络零子进程小于50ms

- **快照档**:`~/.hst/state/fleet-snap.json`,形 `{ts, total, working, idle, blocked, done, hosts:[{name,panes}]}`;原子写同 task 档先例。
- **生产者**:`hst task fleet-refresh`(内部面,不入帮助首屏)跑 `agents view --json`(五原语 presence 面 = **primlib 聚合同源**,契约真源 ai-ccoe plugins/CONTRACT.md),解析计数落快照;primlib 不在场或报错 = 快照缺,段隐藏零噪声。
- **刷新骑现有循环**(零新 daemon 零 cron):其一 hook 腿任一事件(Stop/SessionEnd/Notification/PreToolUse)detach 刷新,60 秒节流(复用 throttle_due 戳机制,戳文件 `.fleet-snap-check`);其二 task sweeper 存活时每 tick 顺带刷新。无 task 无事件 = 快照老化,渲染按龄降级(5 分钟内常显,超 5 分钟灰显带龄,超 30 分钟段隐藏)。
- **渲染**:seg_fleet 纯读快照档整形,零网络零子进程;紧凑形 `☰12 ⚡1 ⏻10`(nerd)或 `flt 12 W1 I10`(ascii);blocked 非零加 `!B1`;done 折入 idle 计数后的 D 注(`⏻10` 表 idle 加 done 合计,明细在 hst doctor)。

### hook 零中文

HOOK_ALIASES 已纯英文(REQ-049);残余中文源 = 插件面 statusMessage 标签(如 `evo-lint 后缀静检`)。修:plugin_display 对含 CJK 的标签回落纯插件名(不双拼),英文标签保持双拼。

### task 循环时间

task 段(L3)增循环时间:`R<rounds> <age>`(age = 距上轮活动 last_stop_at 的 d/h/m 形;无 Stop 时距 set_at);与既有 rounds 同源读档,零新 IO。

## 线框 mock 总台快核件

120 列满窗(本机实数据:12 格 W1 I10 D1,task 在册 R3 距上轮 18m):

```text
L1  ✳ claude-2.1.163   󰉋 hst_rs   main   󰏗 v2.12.0   󰥔 07:51
L2  ◑ claude:working   ✦ glm-4.6   ▰▰▰▱ 42% (84k/200k)   ⏱ 12m   ☰12 ⚡1 ⏻10
L3  ⚉ hst-task|herdr-agent-state|evo-lint   ◉ eval 批 R3 · 18m
```

80 列窄窗(退化后):

```text
L1  ✳ claude-2.1.163   󰉋 hst_rs   main   󰏗 v2.12.0
L2  ◑ claude:working   ▰▰▰▱ 42%   ⏱ 12m
L3  ⚉ hst-task|herdr-agent-state|evo-lint   ◉ eval 批 R3…
```

40 列极窄:

```text
L1  ✳ claude-2.1.163  hst_rs
L2  ◑ claude:working
```

无 task 会话:L3 整行隐藏(现行为保持);无快照:fleet 段隐藏;快照 6 分钟: `☰12 ⚡1 ⏻10·6m`(灰);快照 31 分钟:fleet 段隐藏。

## Criteria 实现批验收 快核后回填

- [ ] 三行结构落地(五段行收敛,字段序与退化表如稿)
- [ ] fleet 快照道(生产者加节流刷新加龄降级),渲染零网络零子进程实测小于 50ms
- [ ] hook 零中文(插件 CJK 标签回落)
- [ ] task 循环时间(R 加 age)
- [ ] 测试与门禁全绿,评审过闸

## 追记

待总台快核:三行字段序与行归属、退化阈、fleet 计数形(图标/ascii 双形)、done 折入 idle 的口径、快照老化阈(5 分钟灰显 30 分钟隐藏)。核过转 implemented 批。
