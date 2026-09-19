---
id: REQ-014
title: hook注册自愈哨兵
status: implemented
priority: must
trace: statusline.rs 单测三件加本机断注册实弹自愈闭环
---

# REQ-014：hook注册自愈哨兵

## Scenario

issue #31 实况：`~/.claude/settings.json` 被外部重写（疑 herdr 集成安装器非合并写或他端会话 stale-snapshot 写），hst 用户级 hook 注册整段丢失，状态写端静默消失、状态栏 agent 段全仓 `unknown`；事发时唯一哨兵是手动 `hst doctor` 的 hooks.form warn。状态栏脚本本身每次渲染都在跑（读端常驻），是天然的自愈触发面。

## Criteria

- [x] 探针：状态栏 hst 段在 state=unknown 时按 agent 定位注册配置文件（claude=`.claude/settings.json`、codex=`.codex/hooks.json`、grok=`.grok/hooks/ohmyagents-state.json`、kimi=`.kimi-code/config.toml`），查 `hst-state.sh` 标记串；非四家在册 agent 名不探（维持 unknown）
- [x] 升格告警：注册面缺位时 hst 段升格 `no-hook!`（红色 203，与 blocked 同级可见性）
- [x] 自愈：缺位且节流窗（每 agent 1 小时，stamp=`~/.hst/state/.hookcheck-<agent>`）到期才 best-effort 调 `hst hook init`（幂等重注册，仅 hook 面不动 yolo 与状态栏配置）；hst 不在 PATH 或调用失败静默吞，不阻塞渲染（rc 恒 0）
- [x] 独立性：哨兵逻辑自含于生成的状态栏脚本，零常驻进程零新依赖；stamp 先于自愈落盘（失败不自旋）
- 已知边界：探针是「任一 hst 注册标记在册即过」的启发式，部分丢失（如仅 PostToolUse 被摘、SessionStart 条目幸存）不触发自愈；实测事故形态是整段抹除，哨兵覆盖该形态。根因侧（注册形态迁移防覆盖）立 REQ-015 另裁

## trace

statusline.rs 单测（sentinel_selfheal_markers_present_in_script、sentinel_probe_covers_four_agents、sentinel_flags_missing_registration_and_stamps_throttle，CI 可移植不依赖 hst 在 PATH）；本机实弹闭环（2026-09-19）：摘净 settings.json 全部 hst 条目后渲染出红字 `claude-2.1.270:no-hook!`，同次渲染自动调 `hst hook init` 愈合（标记 0 处回 8 处），stamp 落盘，二轮渲染回 `unknown` 常态，rc 恒 0。
