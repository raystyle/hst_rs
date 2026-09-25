---
id: REQ-023
title: herdr通道loop即时生效面
status: implemented
priority: must
trace: herdrrpc 单测一件（路径解析三序）加 loopmgmt 单测 arm 一件（四形指令自含加换行压平）加 cli 假服务器集成一件（agent.prompt schema 加 wait 含 blocked 加 idle 成功加 api 错透传加 blocked 分流加 no_socket 不写盘）加实弹（真 socket ping 通 2026-09-26）
---

# REQ-023:herdr通道loop即时生效面

## Scenario

hst 写盘的 loop/goal 对在跑会话不生效（2026-09-26 实证，重启装载才生效）；舰队需要给在跑 agent 即时布 loop。herdr 本地 NDJSON RPC 的 agent.prompt 可派 CronCreate 指令到目标 agent 原生内存通道即时生效（ADR-0009 局部解除 D15）。

## Criteria

验收判据,可检验、可勾选:

- [x] 四入口 `--via-herdr <target>`：loop set（CronCreate 指令）、loop del（id/latest/all 三形 CronDelete 指令）、goal set（own-latest 删旧建新保节奏指令）、goal clear（own-latest 整任务退役指令；原生无空 prompt 重建形，语义差异明示）；wait until 含 blocked，timeout 默认 120s
- [x] 指令文本单源：cron 与 recurring 由 every_to_cron/at_to_cron 翻译（hst 仍是节奏契约单源），四形指令均明示只用原生 Cron 工具勿手改盘上文件；goal 与文本内换行压空格防半截提交
- [x] 传输零新依赖：路径解析 HERDR_SOCKET_PATH 到 HERDR_SESSION（`~/.config/herdr/sessions/<名>/herdr.sock`）到缺省 `~/.config/herdr/herdr.sock`；Windows `\\.\pipe\` 前缀映射；HERDR_SOCKET_PASSWORD 进 params.password；一连接一请求短连接
- [x] 错误面 HerdrError 枚举带 code()（no_socket/connect/io/protocol/api 透传 agent_blocked 与 agent_not_found 等），CLI 错误输出带 `herdr error=<code>`
- [x] herdr 缺席或目标不可达报错退出，不静默回落写盘
- [x] 假服务器协议测试（unix cfg 门控）：请求 schema（method/target/text/wait.until）断言、成功响应解析、api 错误分流；socket 路径解析与指令组合单测全平台
- [x] 实弹：真 socket ping 探活加向本会话格派发指令回执闭环
- [x] loop.hint 与模块文档补双通道口径（写盘重启装载加 herdr 即时）
- [x] 目标边界：时间面四入口仅 Claude Code 格（原生 Cron 家族）；codex 无对等 /loop（Goal Mode 条件驱动属另一机制），错发由目标自报无工具
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
