---
id: REQ-045
title: goal自治与loop面退役
status: implemented
priority: must
trace: goal 单测五件（slug 同源加回环重置加 Stop 腿三态加根链加坏损响亮）加 doctor 单测三件（注册加缺失加坏损）加 statusrender 单测一件（自有源渲染六断言）加 cli 集成三件（回环加 Stop 腿 e2e 加退役退非零）加 shim 十二件改件；实弹本机 hst init 部署 Stop 双挂与 hst goal/hook goal 回环
---

# REQ-045:goal自治与loop面退役

## Scenario

用户令三段（2026-10-11 深夜合成单）：「hst 不再关联 goal 和 loop」「hst 自己用 hook 实现 goal」「自己能管理、自己能控制状态栏显示 goal」。hst 现形态（REQ-019）是 Claude Code 原生 CronCreate 出现前的自建替代：goal 是 scheduled_tasks.json 任务行的 prompt 字段，loop 是该文件的 set/list/del 管理壳，状态栏 loop/goal 段同源只读。现 Claude Code 原生 CronCreate/ScheduleWakeup 已完备且常驻会话实时生效 [实证: 2026-09-26 阴性实证被 2026-10-11 用户确认推翻，常驻会话原生通道实时生效]，hst 退役 loop 面（原生域），goal 搬家为 hst 自治功能。

## Criteria

验收判据,可检验、可勾选:

- [x] goal 数据面自治：自有存储 `~/.hst/state/goal/<slug>.json`（slug 规则同 trace 与 projyolo：路径非字母数字一律 `-`；项目级一键一文件），与 `.claude/scheduled_tasks.json` 一切读写解除（src 全仓无该文件触点）
- [x] goal 命令组语义承接：`hst goal set/show/clear`（--project 缺省当前目录；set 覆盖旧 goal 且 rounds 重计；show 无 goal 给 goal.present=false；clear 删文件、无 goal 零改动退 0；空文本 `goal error=empty_goal` 短码承接 REQ-021 先例）
- [x] loop 命令组退役撤净（REQ-031 pentest 退役同 precedent）：`hst loop` 不在命令树，unrecognized subcommand 非零退；迁移面（Claude Code 原生 CronCreate/ScheduleWakeup 与 /loop）记 changelog 与 REQ-019 演进节
- [x] Stop 事件接线：claude 用户级 Stop 事件注册双挂（state 腿加新 goal 腿），goal 腿单对 hst-goal shim（sh 加 ps1 加 cmd 三载体，claude 专属面；codex/kimi/grok 不挂）
- [x] Stop 触发形三合一：rounds 计数落盘（每轮 Stop 递增，防环放行轮也计）加自然停提醒注入（payload 无 stop_hook_active 时 stdout 出 `{"decision":"block","reason":"…"}` 续跑一轮 [实证: code.claude.com/docs/en/hooks 2026-10-11 查：stop_hook_active 在场须 exit 0 防死环；block 加 reason 续跑]）加状态栏数据刷新（下帧渲染即见新轮次）
- [x] 状态栏 goal 段同源切换：seg_goal 改读自有存储（项目根解析序承接旧 loop 探针链：workspace.project_dir 回落 current_dir 回落 dir），四家同源显示（不再锚 claude session），默认第四行 `DEFAULT_SEGMENTS4 = ["goal"]`，loop 段与 loop-ascii 模板图标出仓（用户 toml 显式残留 loop 段 id 响亮报错，与未知段 typo 同判）
- [x] doctor 检测项：check=goal 两面（claude Stop 事件 hst-goal 注册在场，缺席 warn 提示 rerun hst init；自有存储可读，坏损 warn；本项目 goal 现状为对账面不告警），REQ-020 的 scheduled_tasks 读检出仓
- [x] herdr 即时通道随 loop/goal arm 出仓（REQ-023、ADR-0009 追记）：herdrrpc 模块与 herdr_arm 删（全仓零调用面）
- [x] 测试与实弹：goal 单测五件加 doctor 三件加 statusrender 一件加 cli 三件加 shim 十二件改件；实弹本机 init 部署回环

## 边界

- 提醒注入仅 claude（decision block 契约是 Claude Code 专属面）；goal 设置与状态栏显示四家同源不受限。
- 自然停每轮恰一次提醒（stop_hook_active 防环），goal 在役期每个用户轮次至多多一轮续跑，恒无死环；goal 达成由 agent 侧自判清空（reason 文本内置 hst goal clear 指引）。
- 项目键子目录边界（同 projyolo marker 先例）：CLI 子目录调用与状态栏 payload 项目根不同 slug 键不中，须在项目根跑或显式给 --project。
- rounds 计数是 Stop 事件计数非提醒次数（防环放行轮也计）；显示面 `{count}` 占位符可自配。
- 存储坏损读侧容忍（状态栏隐行）加 doctor warn 双面；CLI show/clear 响亮报错不覆写。
- goalmode 段（REQ-025/037/040，原生 /goal transcript 探针）不在本 REQ 范围，保持不动（原生 goal 态的只读可见面，与 hst 自治 goal 并存）。

实现后回填 frontmatter 的 trace(测试路径或验收命令),状态改 implemented。
