---
id: REQ-034
title: actl适配反馈件三件
status: implemented
priority: must
trace: main.rs init --yes 去 requires compact_pct 加非 compact 范围确认打点加 llms 适配面一行；doctor.rs OURS_SHIM_STEMS 壳名族判据（token 腿落 shim 不再误判 absolute）；cli 集成 init_compact_pct_preview_apply_off_and_doctor_check 加 llms_manual_covers_command_tree_and_flags 断言加 doctor 单测 codex_side_form_classifies_shim_bare_dead 扩两断言
---

# REQ-034:actl适配反馈件三件

## Scenario

ai-cloud 框架把 `actl hst` 做成透传适配面（写级动词过 actl 写闸、`--json` 出 TOON 信封、非 compact 面 `--yes` 由 actl 消费剥离），hst 独立直用不变。适配过程挂出三件反馈，总台派单要求 hst 侧直接落地（用户令「周知各工位直接做」）。

## Criteria

验收判据,可检验、可勾选:

- [x] 件一（反馈件三）`init --yes` 全局面接受：去 clap 的 `requires = compact_pct`，非 compact 范围把 `--yes` 当确认收下（全量部署语义与外层幂等不变，打点 `init.confirm=yes`），compact 面预览与落盘行为不变。此件完成后总台可撤 actl 侧 `toolOwnsYesIf` 特判。
- [x] 件二（反馈件二）doctor `hooks.form` 误报收编：REQ-028 加 token 腿后判据未同步，`hst-token` 壳被归「带路径分隔符的 ours 绝对路径」落 `absolute`，codex 侧取最差形态遂常驻 warn。改按 ours 壳名族（hst-state 加 hst-token 加退役 hst-pentest 加 oma-state）判 shim 与 shim-dead，json 与 codex 两侧共用判据。
- [x] 件三 `--llms` 补一行适配说明：独立直用完全不变；经 actl 调用时写级动词过其写闸（预览缺省、加 `--yes` 执行），`--json` 出 TOON 信封，非 compact 面 `--yes` 由 actl 消费剥离。
- [x] 验收：`hst init --yes` 无 compact 退 0 且出 `init.scope=full`；`hst init --compact-pct 70` 预览零写加 `--yes` 落盘回读自证不变；`hst doctor` codex `hooks.form` 出 ok（detail `command(unix)=shim`）；`hst --llms` 含适配面行；全测绿（258 单元加 49 集成）。
