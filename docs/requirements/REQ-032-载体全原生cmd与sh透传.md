---
id: REQ-032
title: 载体全原生cmd与sh透传
status: implemented
priority: must
trace: shim.rs 重写为 thin 生成函数（sh 按 host_shell 兼容 zsh 加 bash 加 sh、cmd 单路径、grok 包装三件）加 ps1 三件退役清扫加 deploy 四家 Windows 注册改单路径 .cmd（M048 形）加 grok 状态栏壳 thin 化加 is_ours 兼容；单测 thin 形加九件幂等加 ps1 清扫加行为透传（stdin 与 exit 2 直通）；全测 266 加 49 绿；实弹重部署九件落、注册面零 powershell、三腿 e2e（state 写盘加 token 阻断 2 加 pentest 短路）
---

# REQ-032:载体全原生cmd与sh透传

## Scenario

用户令（2026-09-27）：「脱离pwsh 全部原生hst通过cmd sh 透传hst状态栏和hook」。现状 hook 载体是自包含 bash/PS1 shim（D28 零 hst 依赖设计，Windows 注册经 powershell.exe -File ps1 形），状态栏 grok Windows 面仍走 .cmd 壳调 pwsh 脚本（ADR-0010 已知边界）。裁定：全部收进 hst 二进制本体，载体退化为 thin 透传壳（Unix sh 加 Windows cmd），部署时烘焙部署方二进制绝对路径。

## Criteria

验收判据,可检验、可勾选:

- [x] thin 壳三件（state/token/pentest）：sh 形 `exec "<烘焙 exe>" hook <腿> --agent "$1"`（stdin 经 exec 透传）；cmd 形 `@echo off` 加 `"<烘焙 exe>" hook <腿> --agent %1`；M060a 语义由 hst 本体内化（token/pentest 腿 exit 硬约束 0 与 2，hst 缺位或故障 shell 退出非 2 即 fail-open）
- [x] pwsh 退役：STATE_PS1 加 TOKEN_PS1 加 PENTEST_PS1 生成面删除；状态栏 grok Windows 包装 hst-statusline-grok.cmd 改 thin cmd 透传 `statusline --render grok`（stdin 继承）
- [x] 部署重接线：四家 Windows 注册从 powershell.exe -File 形改单路径 .cmd 形（M048 已证 claude 系可 spawn）；Unix 维持 quoted .sh 形；烘焙 exe = 部署方 current_exe 绝对路径（self update 原位替换路径不漂）
- [x] D28 语义翻转记档：state 写盘不再零 hst 依赖（hst 缺位时 hook 全链 fail-open，状态栏回落 unknown）；is_ours 族与清扫面认新形（.cmd 单路径形与 .sh 形不变）
- [x] 测试：thin 壳内容锁、部署注册形断言（Unix 与 Windows 双侧注入）、exit 语义经 hst 本体（token 阻断 2 加 state 静默 0）、ps1 不再生成
- [x] 实弹：本机重部署后注册面 thin 形、hook 三腿 e2e 绿、状态栏渲染不变、无任何 powershell 串入注册面与壳体
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
