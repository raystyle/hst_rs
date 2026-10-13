---
id: REQ-051
title: statusbar三行终稿与舰队指标面
status: draft
priority: must
trace: null
---

# REQ-051:statusbar三行终稿与舰队指标面

## Scenario

用户令(2026-10-13 状态栏令升级,授权 hst 全面调整规划):终稿三条并入(首行首位 agent 类型加版本、hook 零中文、task 循环时间)加新增全局 agents 指标入栏(舰队紧凑计数)。本单为规划面:REQ 加三行线框 mock 先交总台快核,核过再实现。

## 设计 提案稿

### 三行结构定稿 现行五段行收敛为三行

现行五段行(项目行、agent 态行、hookstate 行、task 行、goalmode 行)收敛为四行(L1 身份与项目、L2 运行时、L3 hook、L4 task;总台令第五笔 task 独立成行);字段序与行归属:

- **L1 身份与项目行**:`agent 类型加版本`(首位,用户令)接 dir 加 git 加 clock。dir 与 git 保留现显示形(总台令第二笔:bash 与运行时环境版本段默认退役;段 id 保留可显式写回 toml 自选)。agent 版本取 payload version 归一(现行 seg_hst 的版本归一逻辑随迁)。
- **L2 运行时行**:model 加 context 加 duration 加 **fleet 段**(舰队紧凑计数,新)。agent 态显示(agent:state 文本与态色)整体退役(总台令「不需要显示 claude:working」;承 REQ-049 F-3 裁定态源已枯,会话态归 herdr 域):seg_hst 拆解 = 版本归一逻辑迁 L1 身份段,态文本与 state_color 通道退场,行色恒中性。
- **L3 hook 行**:hookstate(英文 id 清单,零中文,清单内部用 `|`)。
- **L4 task 行**(总台令第五笔「task应该单独一行」):`◉` 前缀接文本加轮次加循环时间,独占一行;goalmode 退化为 L4 前缀旗(active 加 paused),在役才显;无 task 整行隐藏(现行为保持)。

**分隔与图标律(总台令第四笔「没有图标和分隔符?」:保留)**:段间 ` | ` 分隔(现行风格沿用),各段配 nerd 图标(路径、分支、时钟、模型、时长、fleet、hook 清单、task 各一);hook 清单内部与 task 正文前仍用 `|` 细分。

**fleet 计数形(grok 评审计数语义采纳)**:`☰ 1/12`(在跑比总数);blocked 非零加 `!1`;done 非零加 `D1`(不折入 idle,数字与字形一致);三数并排形 `☰ 12·W1·I10`。

**宽度与窄窗退化**(右起逐段弃,保底恒显 L1 身份段 agent 类型加版本与 dir):

| 窗宽 | 退化动作 |
| --- | --- |
| 小于 100 列 | clock 弃;task 文本截 16 字省略 |
| 小于 80 列 | dir 取 basename;fleet 弃;hook 清单截首三个 |
| 小于 60 列 | git 弃;model 弃;task 文本截 8 字;hook 清单只留首一个 |
| 小于 40 列 | 只保 L1(agent 加版本加 dir basename)与 L2 的 context |

### 全局指标快照道 渲染零网络零子进程小于50ms

- **快照档**:`~/.hst/state/fleet-snap.json`,形 `{ts, total, working, idle, blocked, done, hosts:[{name,panes}]}`;原子写同 task 档先例。
- **生产者**:`hst task fleet-refresh`(内部面)跑 `agents view --json`(五原语 presence 面 = primlib 聚合同源),解析计数落快照;primlib 不在场或报错 = 快照缺,段隐藏零噪声。
- **刷新骑现有循环**(零新 daemon 零 cron):其一 hook 腿任一事件 detach 刷新,60 秒节流(复用 throttle_due 戳机制,戳文件 `.fleet-snap-check`);其二 task sweeper 存活时每 tick 顺带刷新。
- **渲染**:seg_fleet 纯读快照档整形,零网络零子进程;平时 `☰ W/N`(在跑比总数),blocked 非零加 `!B`,done 非零加 `Dn`(不折入 idle);三数并排形 `12·W1·I10`。

### 四家承接 邻格 grok 评审采纳

- **claude**:三行全量落位(基准面)。
- **codex**:保持内置 `[tui] status_line`(无外部命令面),默认项换序对齐 L1 至 L2 语义:`codex-version, current-dir, git-branch, model-with-reasoning, context-used, context-window-size, used-tokens`;`run-state` 出默认集;fleet 加 hook 加 task 不上 codex(无对应内置项,不硬造);文案为 codex 固定句不可改。
- **grok**:三行落位(1.0.50 起 stdout 至多 5 行,短窗自底丢行 = hook 与 task 放 L3 正确);摘除旧约束(单行折叠与 nerd 图标剥离,系早期版本事实,现与 1.0.50 不符;图标与分隔符随第四笔总台令全量恢复);版本取顶层 `version` 键,归一成 `数字.数字` 才显示 `grok-1.0.50`,缺键只显 `grok` 不造形不起本地探测;context 直读 `context_window.used_percentage`(条与百分比)加 `context_tokens`(分子)加 `context_window_size`(分母),禁百分比反推,三键缺任一整段隐藏不画 0%;`session_usage` 族是会话累计禁用于画条;时长段注意 `cost.total_duration_ms` 是进程挂会话后毫秒(resume 重计),非全程会话时长;分支取 `workspace.branch`(detached 缺键段隐藏);时钟不自跳,要跳须 `refresh_interval`。
- **kimi**:恒单行(2.1.1 只取 stdout 首行,300ms 超时杀,非 0 退整行回落内置 footer,1 秒节流);payload 键族 = model(字符串)加 cwd 加 gitBranch(可 null)加 contextUsage(比值,0.42 表 42%)加 contextTokens 加 maxContextTokens 加 version(CLI 版本,故 kimi 能显 `kimi-2.1.1`);一行从左到右 `kimi-2.1.1 | hst_rs | lane/statusbar-v2 | <model> | 42% 84k/200k | R3·18m`,必留 = 版本加目录 basename 加分支(有才留)加模型加 context;task 在役只留 `R3·18m`,hook 至多留首 id,fleet 与 clock 不上;不起 git 不起 shell(300ms 被杀即见内置栏,分支直读 payload)。

### hook 零中文

HOOK_ALIASES 已纯英文(REQ-049);残余中文源 = 插件面 statusMessage 标签(如 `evo-lint 后缀静检`)。修:plugin_display 对含 CJK 的标签回落纯插件名(不双拼),英文标签保持双拼。

### task 循环时间

task 段(L3)增循环时间:`R<rounds> <age>`(age = 距上轮活动 last_stop_at 的 d/h/m 形;无 Stop 时距 set_at);与既有 rounds 同源读档,零新 IO。

## 线框 mock 总台快核件

120 列满窗(本机实数据:12 格 W1 I10 D1,task 在册 R3 距上轮 18m):

```text
L1  ✳ claude-2.1.163 | 󰉋 /home/ray/repos/hst_rs | 󰂠 lane/statusbar-v2 | 󰥔 07:51
L2  ✦ glm-4.6 | ▰▰▰▱ 42% (84k/200k) | ⏱ 12m | ☰ 1/12
L3  ⚉ hst-task|herdr-agent-state|evo-lint
L4  ◉ eval 批 R3 · 18m
```

80 列窄窗(退化后,与退化表同规则):

```text
L1  ✳ claude-2.1.163 | 󰉋 hst_rs | 󰂠 lane/statusbar-v2
L2  ✦ glm-4.6 | ▰▰▰▱ 42% | ⏱ 12m
L3  ⚉ hst-task|herdr-agent-state|evo-lint
L4  ◉ eval 批 R3…
```

40 列极窄:

```text
L1  ✳ claude-2.1.163 | 󰉋 hst_rs
L2  ▰ 42%
```

无 task 会话:L3 整行隐藏(现行为保持);无快照:fleet 段隐藏;快照 6 分钟:`☰ 1/12 ·6m`(灰);快照 31 分钟:fleet 段隐藏。

## Criteria 实现批验收 快核后回填

- [ ] 三行结构落地(五段行收敛,字段序与退化表如稿)
- [ ] fleet 快照道(生产者加节流刷新加龄降级),渲染零网络零子进程实测小于 50ms
- [ ] 四家承接落地(codex 内置换序,grok 摘旧约束加 context 三键直读,kimi 单行五必留禁子进程)
- [ ] hook 零中文(插件 CJK 标签回落)
- [ ] task 循环时间(R 加 age)
- [ ] 测试与门禁全绿,评审过闸

## 追记

总台快核回执四笔(2026-10-13):其一「不需要显示 claude:working」,agent 态段(agent:state 文本与态色)整体退役并入本稿,L2 起手改 model;其二「dir 加 git 保留现显示形,bash 与运行时环境版本段(python/rust/node/zig/go/cpp 加 package)默认退役」;其四「没有图标和分隔符?」= 保留现行 ` | ` 段分隔与各段 nerd 图标,fleet 形 `☰ 1/12` 带图标。

其三 = 邻格 grok 四点评审采纳(带实证;其美观律的极简向被第四笔部分推翻,计数语义保留):codex 内置换序不上三行;grok 三行落位摘旧折叠约束,context 直读 context_window 三键禁反推,三键缺一整段隐藏,时长注意 cost.total_duration_ms 语义,分支 workspace.branch,时钟 refresh_interval;kimi 恒单行五必留(kimi 有 CLI version 可显 kimi-2.1.1),禁起子进程分支直读 payload;done 不折 idle 独立 Dn;80 列 mock 与退化表不一致已修(验收以表为准)。

过程一笔:fb91060 提交信息声称 grok 修订全落,实盘仅 mock 一行改动(脚本中途断言失败未写盘,提交信息超前于实跑,同门禁假绿同族坑);本笔整文件重写落齐,以本文件为准。

其五「task应该单独一行」= task 出 L3 独占 L4(结构改四行),goalmode 前缀旗随迁 L4。待核余项:退化阈、快照老化阈(5 分钟灰显 30 分钟隐藏)。核过转 implemented 批。
