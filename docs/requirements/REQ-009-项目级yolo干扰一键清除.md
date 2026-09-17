---
id: REQ-009
title: 项目级yolo干扰一键清除
status: implemented
priority: must
trace: cargo test --locked（clear_project_yolo_interference 单测加 cli 集成 init --clear-project-yolo）；五端实况核 2026-09-17 在册
date: 2026-09-17
owner: raystyle
tags: ['yolo', 'D55', 'REQ-009']
---

# REQ-009：项目级 yolo 干扰一键清除

## 背景与动机

用户裁定（D55，2026-09-17）：五端四机缺省全部走用户级 full yolo（现况已核实：五端四家用户级键齐、无项目级干扰在位）。项目级键会遮蔽或干扰用户级 yolo（claude 项目层 defaultMode 非 bypass 即静默降级、ask 规则 bypass 下照弹、blockReads=true 读沙箱照问；codex 与 kimi 项目 config 覆盖用户层），典型复发源是 Claude Code 界面自写 acceptEdits 进项目 settings.local.json。现有 `--project-yolo=off` 只摘 ours 落值（等值摘除），收不了外来干扰键。

## 需求

一条命令清除项目级对用户级 yolo 的干扰键（ours 与外来都摘）：

- `hst init --clear-project-yolo [--project PATH]`，与 `--yolo`/`--project-yolo` 互斥，keys-only 不部署 hook。
- claude：`.claude/settings.json` 加 `.claude/settings.local.json` 摘 `permissions.defaultMode`（任意值）、`permissions.ask`、`permissions.blockReadsOutsideWorkingDirectories=true`；`permissions` 空则摘键，整文件空对象则删文件。
- codex：项目 `.codex/config.toml` 摘顶层 `sandbox_mode` 加 `approval_policy`（任意值）；项目信任键不动。
- kimi：项目 `.kimi-code/config.toml` 摘 `default_permission_mode`（任意值）。
- grok：无项目级面，n/a。
- 边界：干扰面 = D50 三键（defaultMode 加 ask 加 blockReads=true）加 codex/kimi 两键，ours 自落的项目级 `skipDangerousModePermissionPrompt` 与 `enableAllProjectMcpServers` 残余归 `--project-yolo=off`（codex 评审 O-1 口径）；`permissions.allow`/`deny` 与其余非干扰键不动（deny 是安全 carve-out，D50 分类学未列干扰）；家目录守卫同 `--project-yolo`（root 是家目录整支跳过打点）；输出 `init.retired=<路径> (cleared-yolo)` 行族。
- doctor 三处冲突 CTA 扩 `or hst init --clear-project-yolo`。

## 验收

- [x] 单元：外来 acceptEdits 加 ask 加 blockReads=true 被摘且 allow/deny 保留；codex/kimi 外来值被摘；空文件删、无干扰键零写入；家目录守卫跳过。
- [x] 集成：临时项目跑 `hst init --clear-project-yolo` 断言输出行与文件面。
- [x] 五端四机实况核（2026-09-17 扫描在案：无在位干扰，功能为防复发面）。
