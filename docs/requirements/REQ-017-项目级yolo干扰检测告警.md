---
id: REQ-017
title: 项目级yolo干扰检测告警
status: implemented
priority: should
trace: yolo.rs 检测单测加 cli 集成 yolo_check_reports_hits_and_writes_marker 加 statusline 单测两件加实弹
---

# REQ-017：项目级yolo干扰检测告警

## Scenario

用户裁层一（2026-09-20）：hook 期检测非 hst 的项目级设置对用户级 yolo 的干扰，命中时状态栏升格可见告警，CTA 指向既有 `hst init --clear-project-yolo`；侵入度零改动（不修改任何配置文件）。裁层时排除静默自动清除（意图不可证明：项目级可能是用户或团队故意选择；写战风险：hst 会变成 issue #31 里的覆盖方；REQ-009 当年裁手动是有意的）。

## Criteria

- [x] 检测集与 `clear_project_yolo_interference` 同源同键（claude 项目两层 permissions.defaultMode 任意值加 ask 任意形态加 blockReadsOutsideWorkingDirectories 仅 true；codex 项目 sandbox_mode 加 approval_policy；kimi 项目 default_permission_mode），只读零改动，家目录守卫（root 为用户家目录返回空，判据与清除侧同源走 same_location 加失败语义同判走错误传播，评审 G4）
- [x] 探针落位裁定：不入 hook 工具门路径（shim 是零 hst 依赖自包含写端 D28，hook 门延迟敏感），落在状态栏哨兵面（REQ-014 先例），节流窗 10 分钟到期同步跑 `hst yolo check --project <dir>`（渲染阻塞有界）
- [x] `hst yolo check` 命令：kv 行（hit 加 count 加 key 明细加 hint 指向清除命令；只出 kv marker 行不走 fmtio 三态，评审 G3 记档）加写 marker `~/.hst/state/projyolo/<slug>.json`（slug 同 trace 规则，project 原样透传；real 字段 = canonicalize 形且双 slug 各落一份同容，评审 G1/G6：符号链接漂移与手动 CTA 的物理路径拼写都覆盖）
- [x] 状态栏 hst 段：读 marker（廉价每渲染），命中（project 与 real 任一等值，评审 G1）且新鲜（2 倍窗 1200 秒内）升格红字：实时态有价值时复合并显（`working/proj-yolo!` 等，评审 G2 裁）、unknown 等低价值态直接替换；与 no-hook! 的序（写端健康优先，no-hook! 在位不叠加）；CTA 在命令 help 与 kv hint 行双面
- [x] 已知边界：用户清除干扰键后到下一节流窗（至多 10 分钟）内告警可能滞留（marker 新鲜窗 2 倍）；slug 碰撞（路径非字母数字归一为 -）理论上可致异项目误报；real 加双 slug 已覆盖在位路径的拼写差（符号链接与物理路径），残余 = 两拼写的路径都不存在时的任意差，接受记档；探针腿 PATH 依赖同 REQ-014 边界三（评审 G6 指注）

## trace

yolo.rs 单测 project_yolo_interferences_detects_without_touching（检测集加只读面加干净零命中）；cli 集成 yolo_check_reports_hits_and_writes_marker（kv 行加 marker 落盘加字节原样，HST_ROOT 隔离）；statusline.rs 单测两件（形锁加 marker 驱动升格与项目不符不升格）；本机实弹（2026-09-20）：种干扰项目跑 check 出 hit=true 加键行加 hint、marker 落盘、部署后状态栏升格 proj-yolo!。
