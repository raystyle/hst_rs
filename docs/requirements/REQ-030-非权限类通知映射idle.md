---
id: REQ-030
title: 非权限类通知映射idle
status: implemented
priority: should
trace: hook.rs map_event 加 state_for_payload 加 shim 四载体（sh 加 cmd 两形加 ps1）同步映射；单测两件期望改（permission 类仍 blocked、idle_prompt 类改 idle）；实弹 Notification payload 经 hst hook state 写盘 idle 绿；全测 256 加 49 绿
---

# REQ-030:非权限类通知映射idle

## Scenario

用户实报（2026-09-27「为什么是灰色？」）：ai_ccoe 工位处理完周知后收到任务完成类 Notification 事件，S025 既有语义把非权限类通知映射 unknown，状态栏灰数小时（面板闲置停在那一帧）。「agent 干完活等用户看」语义上更接近 idle（绿），unknown 灰有误导性。

## Criteria

验收判据,可检验、可勾选:

- [x] hook.rs：state_for_payload 的 notification 分支非权限类回落 idle（permission 与 elicitation 类仍 blocked）；map_event 的 notification 同判 idle（无 payload 细分形）
- [x] shim 四载体同步：STATE_SH（notification case 基态 idle）、STATE_CMD 与 STATE_CMD_JQ（Notification 基态 idle，permission 键值判仍 blocked）、STATE_PS1（Notification 分支 idle）
- [x] 测试：permission_prompt 仍 blocked、idle_prompt 改 idle、map_event 期望更新
- [x] 评审吸收（G1 加 G2）：G1 sh 与 ps1 载体的 permission 判定从整包子串收窄到 notification_type 键值段（对齐 cmd/Rust；反例 task_complete 加 message 含 permission 一词原误染红）；G2 备录无 type 的 Notification 一律 idle，上游新增「需人工介入」类通知须同步扩 blocked 词表
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
