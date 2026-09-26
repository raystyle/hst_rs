---
id: REQ-028
title: hook命令解耦与穿透命令族
status: implemented
priority: must
trace: hook.rs 拆腿（run_state 加 run_token，run 保双职责弃用期）加 main.rs 双子命令加 shim 三载体拆分（STATE 剥 guard 腿、TOKEN 新三件）加 deploy 四家双挂（merge 多 handler 形、codex 腿分流）加别名三拆条加 shim/deploy/statusrender/hook 测试面改加；全测 253 加 49 绿；实弹三查（注册面双挂、shim 八件、状态栏三别名）加 e2e 三态（key 阻断 exit 2、干净放行、旧二进制 fail-open）
---

# REQ-028:hook命令解耦与穿透命令族

## Scenario

用户令（2026-09-27）：「hst-state 不应该双职责 拆分成2个脚本和hst hook payload穿透命令 以后还要扩展很多hook命令」。现状 hst-state shim 双职责（四态写盘加 PreToolUse/UserPromptSubmit 透传 `hst hook status` 跑 secretguard），注册面单 stem 承载两功能，状态栏别名只能以值内嵌分隔模拟拆分（REQ-026 五令终形）；后续还有更多 hook 功能待挂（扩展面）。前置令（同日早）：命令参数与状态栏别名对齐（`hst state` 与 `hst token` 命名风格对齐 herdr `herdr state`），三 hook 对三别名。

## Criteria

验收判据,可检验、可勾选:

- [x] 命令族解耦：`hst hook state`（payload 穿透，四态写盘，D28 读序含会话键与 SessionEnd GC，不扫密钥）与 `hst hook token`（payload 穿透，secretguard 扫描，block 级 stderr 加 exit 2、内部故障 exit 0 fail-open，不写盘）；`hst hook status` 保留一代弃用期（双职责原样，clap 注释标弃用）
- [x] 双脚本拆分：shim 面 hst-state（剥 guard 透传腿，纯四态写盘）加新 hst-token 三载体（sh/cmd/ps1：读 payload 透传 `hst hook token --agent`，M060a 白名单透传形：仅 hst 自判 block 透传 2 并回放原因，其余非零 fail-open 放行）
- [x] 部署重接线：claude/grok/kimi 状态事件仍挂 hst-state shim、PreToolUse 与 UserPromptSubmit 增挂 hst-token shim（Windows grok 经 hst-token-grok.cmd 单路径包装，评审 F1，M048 同款，shim 共八件）；codex 双侧字段同构挂 shim 形（state 维持 hst-state 载体、token 挂 hst-token 载体，`hst hook state/token --agent codex` 裸形仅作 schema 必填兜底；trust hash 按最终 hooks.json 重种）；ours 判定与清扫面认 hst-token 族
- [x] 状态栏别名拆条：三 stem 三别名（herdr-agent-state 映射 herdr agent状态监控、hst-state 与 hst hook state 映射 hst 会话状态同步、hst-token 与 hst hook token 映射 hst token护栏），分隔符 ` | ` 终形不变，值内嵌分隔拆除
- [x] 扩展面预留：新 hook 命令循 `hst hook <name>` 穿透契约（stdin payload 进、exit 码语义出）加别名表加条即入列；HOOK_ALIASES 收录指引注释同步
- [x] doctor 兼容：hook 注册形态检测认新命令形（hst-token shim 与裸形）；REQ-014 哨兵标记面不破（state shim 名不变）
- [x] 测试：state 与 token 命令行为件（写盘、block exit 2、fail-open）、部署注册形断言（四家双挂）、shim 内容形（state 无 guard 腿、token 白名单腿）、别名三拆条、ours 判定新形
- [x] 实弹：本工位重部署后注册面三 hook 并存、状态栏第5行三别名对三 hook、密钥拦截 e2e 仍阻断
- [x] 评审吸收（F1 加 G1 至 G4）：F1 Windows grok token 腿补 hst-token-grok.cmd 单路径包装（多 token 形 grok 不可 spawn 且 fail-open 会吞成静默放行）；G1 组形口径在册（claude 系同组双挂、codex 两组，语义等价全量 matcher，trust 键带组下标重种无害）；G2 迁移把 ours 组重建到组序末尾（外来 matcher 不受影响）；G3 shim 件数口径七改八同步；G4 本行与 diary 回填
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
