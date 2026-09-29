---
id: REQ-037
title: 状态栏goalmode设标形
status: implemented
priority: must
trace: statusrender 单测 scan_markers_goal_set_forms 四断言一件加 goalmode_probe_reads_goal_set_form e2e 一件；实弹 prs_c2coe 真会话 payload 渲染 goalmode 行（设标文本截 60）
---

# REQ-037:状态栏goalmode设标形

## Scenario

用户报（2026-09-29 晚）：prs_c2coe 工位 goal 明明 active（TUI 指示在），状态栏 goalmode 行不显示。取证：2.1.270 的 /goal 设标以排队件落盘：主链 `queue-operation` 的 `"content":"Goal set: <文本>"` 加 `queued_command` 附件的 `"prompt":"Goal set: <文本>"`（同事件多副本同文本幂等）；check-in 标记只在轮提前结束时落，常稳运转的 goal 从不产出。REQ-025 探针词表只认 check-in 加 paused 加 clear 三形，缺设标形则新设 goal 行恒隐（昨日 f3ad229d 同款，TUI active 而 transcript 零 check-in 的谜底）`[实证: 93b81bbc transcript 三载体对读加实弹渲染前后对照]`。

## Criteria

验收判据,可检验、可勾选:

- [x] goalmode 探针词表补设标双锚：`"content":"Goal set:` 与 `"prompt":"Goal set:`，取 active 态加解转义文本（倒扫时序下 paused 后到自然覆盖，clear 判清态不变）
- [x] 排队 clear 形补锚（`"prompt":"/goal` 加 clear/off/stop 尾）
- [x] 解转义单源化：json_capture 原语（串内续解，含 \u 代理对拼合），json_string_at 改薄壳
- [x] 仅原生面：PS1 载体随 REQ-038 本版完全淘汰，词表差异不复存在
- [x] 测试：scan_markers 四断言（主链形加附件形加引文假阳拒中加排队 clear）加 probe 级 e2e（仅设标形出行）；实弹真会话 payload

## 边界

- 锚是载体无关的：嵌套对象载体（如 CronCreate input 的 prompt 字段）与普通用户消息 content 文本恰以 `Goal set:` 起头均出 active（评审 G1 合验证过两形；全量语料 15 处真设标全在 queued_command 加 queue-operation 载体，理论面；收紧形候裁）
- 序列化空白：紧凑与冒号后带空格两形同认（评审 G2，020a4f1b 实证带空格形并存；其它空白形不再扩）
- rendered 回声副本（「The user sent a new message …: Goal set: …」）content 不以 Goal set 起头，锚不中，不重复计；深层引文（字符串值内嵌）经再编码不中（单测锁）
- goal_status 附件（`{"type":"goal_status","met":true,…}` 权威态载体）与 local-command-stdout 形未入词表：met 后仍显示 active 属既有缺口（评审 G8 实弹三例在证，候裁后续 REQ）
- goalmode 加会话 loop 探针的 4MB 分块加 512B 重叠反扫在原生面无多块夹具（PS1 时代跨块夹具随载体退役；扫描器经真 transcript 实弹验证，夹具候裁补）
