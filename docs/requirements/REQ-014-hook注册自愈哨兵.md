---
id: REQ-014
title: hook注册自愈哨兵
status: implemented
priority: must
trace: statusline.rs 单测四件加本机断注册实弹自愈闭环两轮
---

# REQ-014：hook注册自愈哨兵

## Scenario

issue #31 实况：`~/.claude/settings.json` 被外部重写（疑 herdr 集成安装器非合并写或他端会话 stale-snapshot 写），hst 用户级 hook 注册整段丢失，状态写端静默消失、状态栏 agent 段全仓 `unknown`；事发时唯一哨兵是手动 `hst doctor` 的 hooks.form warn。状态栏脚本本身每次渲染都在跑（读端常驻），是天然的自愈触发面。

## Criteria

- [x] 探针：状态栏 hst 段在 state=unknown 时按 agent 定位注册配置文件（claude=`.claude/settings.json`、codex=`.codex/hooks.json`、grok=`.grok/hooks/ohmyagents-state.json`、kimi=`.kimi-code/config.toml`），查 `hst-state` 干 stem 标记（覆盖 Unix `.sh` 与 Windows `.ps1`/`.cmd` 注册形态，评审 F2；`Select-String -LiteralPath`）；非四家在册 agent 名不探（维持 unknown）
- [x] 升格告警：注册面缺位时 hst 段升格 `no-hook!`（红色 203，与 blocked 同级可见性）
- [x] 自愈：缺位且节流窗（每 agent 1 小时，stamp=`~/.hst/state/.hookcheck-<agent>`）到期才 best-effort 调 `hst hook init`（幂等重注册，仅 hook 面保语义不保字节：claude settings.json 会 pretty 重排、codex config.toml 注释会被删、未在位的 grok/kimi 配置会被创建，评审 G3）；hst 不在 PATH 或调用失败静默吞，不阻塞渲染（rc 恒 0）
- [x] 节流读侧走 `[IO.File]` 直取 mtime（dotfile 在 Unix pwsh 带 Hidden 属性，`Get-Item` 不带 `-Force` 取不到，读侧失败 fail-closed 不自愈，评审 F1）；stamp 首创建独占落痕（并发窗格双跑有界且 heal 幂等，败者仍会 heal，记档接受，评审 G2/G6）
- 已知边界一：探针是「任一 hst 注册标记在册即过」的启发式（stem 形，注释里提及 hst-state 同样判在册，评审 G9），部分丢失（如仅 PostToolUse 被摘、SessionStart 条目幸存）不触发自愈；实测事故形态是整段抹除，哨兵覆盖该形态
- 已知边界二（评审 G1）：会话键在册时的部分丢失盲区，读侧无 TTL 停在最后一态，既不红也不自愈（SessionEnd 随注册失效不清键）；缓解（状态键超窗转探针或 stale 提示）归 REQ-015 一并评估
- 已知边界三（评审 G4）：自愈腿依赖 PATH 上的 hst，GUI/IDE 精简 PATH 时永红不自愈；有意只留状态栏不留 hook 的用户无退出开关，烘绝对路径或落痕开关归 REQ-015 评估
- 根因侧（注册形态迁移防覆盖）立 REQ-015 另裁

## trace

statusline.rs 单测四件（sentinel_selfheal_markers_present_in_script、sentinel_probe_covers_four_agents、sentinel_flags_missing_registration_and_stamps_throttle、sentinel_throttle_holds_across_renders 节流回归锁；heal 腿经 stub PATH 隔离真部署，CI 可移植）；本机实弹闭环两轮（2026-09-19）：一轮摘净注册渲染出红字 `claude-2.1.270:no-hook!` 同次渲染自愈（标记 0 处回 8 处）二轮回 `unknown`；二轮评审收口后 PATH 无 hst 时节流窗内 stamp mtime 分毫不动（F1 回归实证）、清 stamp 后真 PATH 一渲染即愈回 8 处。
