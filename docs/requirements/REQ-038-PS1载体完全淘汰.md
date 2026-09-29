---
id: REQ-038
title: PS1载体完全淘汰
status: implemented
priority: must
trace: statusline 纯函数面保留件（配置解析加四家 merge 加 grok 壳）加 deploy 单测清扫断言改件加 cli 集成三断言改件；实弹本机 init 清扫加原生渲染冒烟加五端滚动（v2.9.13 轮）
---

# REQ-038:PS1载体完全淘汰

## Scenario

用户令（2026-09-29 晚，REQ-037 实施中）：「powershell 完全淘汰 不用保留」，撤销 REQ-032 的「pwsh 脚本弃用期保留一代」约定。四家状态栏配置自 ADR-0010 起全部指向原生渲染（`hst statusline --render`，sh 加 cmd thin 壳透传），PS1 脚本生成面与部署面是纯残余，维护成本（每次探针改动双侧同判）照付。

## Criteria

验收判据,可检验、可勾选:

- [x] statusline.rs 的 PS1 全生成面出仓：常量族（HEAD 加 COMMON 加 CTXPROBE 加 PROBE 加 LOOPPROBE 加 GOALPROBE 加 SEG 族加 TAIL）、拼装（assemble 加 cfg 烘焙块）、deploy_script 加 deploy_custom_script 加 restore_builtin_script 加 marker/custom 面、pwsh_on_path
- [x] `--script` 加 `--builtin` 旗标摘除（clap 面加集成测试同步；`--render` 与 `--example` 互斥保留）
- [x] init 退役清扫：幂等摘除 `~/.hst/statusline/hst-statusline.ps1` 与 `.custom` 标记；grok thin `.cmd` 壳改由 merge_grok 落位（内容判等幂等）
- [x] verify 直跑原生渲染（部署方二进制加 mock stdin，机读标记判据不变）；doctor 判据去脚本在位与 pwsh 缺失面
- [x] 纯函数面保留：statusline.toml 解析加 effective_orders 加四家 merge 幂等测试全绿
- [x] 实弹：本机 init 清扫旧件加渲染冒烟；五端随 v2.9.13 滚动

## 边界

- doctor 对存量 pwsh 形配置的判废夹具保留（更陈旧的错误配置仍要报）
- Windows grok 仍走 thin `.cmd` 壳（M048 单路径约束，壳内直调原生渲染，不含 PowerShell 面）
