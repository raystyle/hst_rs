---
id: REQ-052
title: tokenguard全面退钩批
status: implemented
priority: must
trace: 实现批 lane/token-unhook。单测随动 = 主矩阵件与 shim 件与 codex 三件随退钩形重写加开关全链件 token_guard_switch_off_to_on_to_off_transitions 加 kimi 清扫件重写加 sentinel 他家不哨断言;集成新增 init_token_guard_switch_roundtrip_and_runtime_hatch(缺省退钩加开位回归加关位清出加运行时逃生闸)加 init_full 与幂等件随形;单测 242 集成 63 绿,门禁全绿含 NO_COLOR=1 直跑
---

# REQ-052:tokenguard全面退钩批

## Scenario

总台令(2026-10-13):token-guard 全面退钩(「默认关掉 hst-token-guard」「不 hook hst-token-guard」:默认不注册加存量清出);随即收窄令「只摘除 hst-token-guard 相关」钉死范围 = 本单只做 token 腿退钩,goal alias 与 hook goal 摘除(原同步单顺延件)移出本单另批承载,插件 hook 纯名(237ad50)已在 main 随版携带。发布序照 v2.13.0 前例;滚版矩阵归总台,各端滚后 hst init 重部署清存量,总台全端 doctor 复验 hook 面干净。CLI hst hook token 保留为手动面(退钩不灭功能)。

## 设计

- **token-guard 全面退钩**(承 entropy-01 hst-state 腿退役先例,全腿退役):
  - 注册矩阵:四家 token 腿全摘(claude PreToolUse 与 UserPromptSubmit 双事件退场,余 task 四事件独挂;codex 与 grok 与 kimi 零 hst 钩,deploy 面转纯清扫道)。
  - 存量清出:既有 hst-token 注册经新 init 一趟清出(sweep 与 merge 陈旧替换道);shim 腿表去 token 加退役清扫删 hst-token 四载体。
  - CLI 与运行面:`hst hook token` 子命令保留为手动面(退钩不灭功能),shim hst-token 生成面退役(腿表出列加退役清扫)。
  - 判据随动:doctor hooks form 判据与 check=token 面、statusrender sentinel no-hook 判据(改押 hst-task,仅 claude 有腿;codex 与 grok 与 kimi 零钩为常态不哨)、HOOK_ALIASES 过渡期保留 hst-token-guard 条目(装机位 init 前存量仍显)。
- **goal 族不动**:收窄令明确只摘 token-guard 相关;goal alias 与 hook goal 与 HST_GOAL_* 双名全保(移另批)。
- **插件 hook 纯名**:237ad50 在案(statusMessage 不上栏,plugin_display 退役),随版携带非本单工面。

## Criteria

- [x] 四家 token 腿注册面退场,存量 init 一趟清出(开关全链件 codex 分侧外科件 kimi 清扫件)
- [x] shim hst-token 腿表出列加退役清扫删除(仅 OFF 位),CLI hook token 保留手动面
- [x] doctor 与 sentinel 判据随动,codex 与 grok 与 kimi 零钩不哨(doctor 认开关标记档不误报 ON 机)
- [x] goal 族零触碰(收窄令验面;goal alias 与 hook goal 与 HST_GOAL_* 全保)
- [x] 开关面:`hst init --token-guard/--no-token-guard` 加标记档加 HST_TOKEN_GUARD=off 运行时逃生闸(总台追加令)
- [x] 2.14.0 定名(Cargo 加 CHANGELOG),发布链照 v2.13.0 前例
- [ ] 全测门禁绿,评审过闸,完批回执总台(版本号加资产在册)

## 追记

实现追记(2026-10-13):总台追加令「不能有个开关直接关掉这个 hook」落开关面三件 = CLI 双旗标(--token-guard 开加 --no-token-guard 关,缺省不动按现态)加标记档(~/.hst/token-guard.on,doctor 认档不误报 ON 机)加运行时逃生闸(HST_TOKEN_GUARD=off 直通);开关 ON 道注册机件自 HEAD 原样恢复收在开关后(deploy 四家条件分流加 shim 腿表与退役清扫分流)。收窄令「只摘除 hst-token-guard 相关」钉死范围:goal 族零触碰(goal alias 与 hook goal 摘除移后续批,HST_GOAL_* 双名全保)。
