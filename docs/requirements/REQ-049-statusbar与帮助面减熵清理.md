---
id: REQ-049
title: statusbar与帮助面减熵清理
status: implemented
priority: must
trace: deploy 单测改件六件(注册矩阵随动)加 statusrender 单测改件两件(段名纯英文化)加 shim 单测改件一件(八件壳)加 verify 单测改件一件(grok trust 族随 state 判据退役)加集成改件五件(注册断言矩阵加 token 面直跑加验证判据)加集成新增一件(help_faces_carry_no_archaeology_markers 帮助面零考古门);单测 236 降 229(state 写盘族退役)集成 57 绿;活弹 agents verify claude 实跑 hook=ok rounds=2(goal 腿端到端新判据)
---

# REQ-049:statusbar与帮助面减熵清理

## Scenario

用户令(2026-10-12 追单,goalhard-01 收口后接续):herdr 的 herdr-agent-state hook 已覆盖会话状态同步,hst 的 state 腿多余须整体剔除;statusbar 段名纯英文(中文显示名全撤);命令帮助文本去历史考古痕迹面向用户重写(hook 子命令面最乱优先)。

## 设计

### hst-state 腿整体剔除

- 注册矩阵改形(deploy.rs):claude = token 腿挂 PreToolUse 与 UserPromptSubmit 加 goal 腿挂 Stop 与 SessionEnd 与 Notification 与 PermissionRequest(均单挂,SessionStart 与 PostToolUse 退出注册面);codex/grok/kimi = 只剩 token 腿双事件。存量 state 注册经 sweep_unmanaged_ours 族与 merge 陈旧替换清出(承 pentest 退役先例);CodexLeg 枚举与 codex_side_command 等四件 state 生成器死码清零。
- shim 面(shim.rs):腿表去 state,退役清扫面加 hst-state 四件(带生成标记才删,承 oma-state 与 pentest 先例)。
- CLI 面(main.rs):`hst hook status` 与 `hst hook state` 子命令退役;`hst hook token` 与 `hst hook goal` 存续。
- 写盘面(hook.rs):state 四态写盘族(run 加 run_state 加 with_payload 系加 session_of 加 sweep)退役;事件解析公共件(map_event 加 event_label 加 state_for_payload 加 read_stdin_json)与 token 腿(guard 面)存续。
- 判断项 J1(评审重点):verify 判据重定:claude 改押 goal rounds(临时 task 在册,无头会话 Stop 后 rounds 递增即 ok,判据档文本明示勿清;实证首版判据档被无头会话依提醒自清,goal-record-vanished 假阴);codex/grok/kimi hook 层 skip 带注(hst 运行时可观测腿仅 claude task 面,注册面形态由 doctor hooks.form 验)。
- doctor 面:hooks.form 分类判据从 hst-state shim 扩为现行腿(hst-token 与 hst-goal)加存量名(hst-state 与 oma-state 兼认)。

### 段名纯英文

HOOK_ALIASES 表(statusrender.rs):herdr-agent-state 与 hst-goal 直用英文 id(表项即恒等映射);hst-token 显示名 hst-token-guard;hst-state 条目随腿退役移除;未收录 stem 回落本名(天然英文)。

### 帮助面减熵

clap about 全清(main.rs 十八处重写):D/S/P/REQ/M 编号、ome 与 ohmycloud 旧字样、用户令日期引文全撤;语义保留(旗标范围语义、退出码、判据口径)。task 正名先行(用户令 2026-10-13「不叫 goal 了就是 task 任务」):goal 命令组 about 以 task 称谓,现役命令名 `hst goal` 待 task-migrate-01 批更名。回归门:集成测试 help_faces_carry_no_archaeology_markers 遍 22 个帮助面断言零考古标记(~/.oma 旧根迁移为活功能不作痕)。

## Criteria

- [x] hst-state 腿整体剔除:注册面与 shim 面清零,CLI 双子命令退役,写盘族死码清零
- [x] 段名纯英文:表内中文显示名全撤,hst-token-guard 与英文 id 直用
- [x] 帮助减熵:22 帮助面零考古标记(集成门锁),hook 面重写,task 正名先行
- [x] verify 判据重定并活弹验证(claude hook=ok rounds=2 端到端)
- [x] 改面随动测试全绿,四门禁加 aidoc 加 PEVO 绿(见 trace)

## 追记

- 实证一笔:无头会话会依 Stop 提醒里的自清授权把验收判据档 clear 掉(提醒文本「停机权在你」对无头会话同样生效),判据档文本须明示勿清;此坑对 task-migrate-01 批的判据面同样适用。
- 本机实况一笔:本机用户级注册存量为旧形(Stop 仅挂 hst-state,goal 腿未注册),本批 sweep 机制在下次 `hst init` 时自动清;活弹 verify 经 UserHooksGuard 的 byte 备份加 deploy 加还原走现行面,不受存量影响。
