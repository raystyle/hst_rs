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
- [x] 评审吸收（F1 加 F2 必修、G1 至 G4）：F1 clap 用法错缺省退 2 污染薄壳「自判 block 出 2」硬约束：main() 改 Cli::try_parse() 把用法错映射 1（help 加 version 的 exit_code 0 原样），集成测试五处期望 2 改 1；F2 Windows 直路径 .cmd 与 D39 宿主实弹相反（claude hook 执行 shell 是 POSIX sh 不认盘符路径），弃 pwsh 后的承载形 = D39 自己的救济形 `cmd.exe /c "<.cmd>" <agent>`（宿主实弹 rc=0），grok 维持 M048 单路径包装，stale cfg!(windows) 断言同步；G4 回退路径记档（pwsh 载体已退役，回退需重装旧版二进制或 revert，.ps1 不再生成）；G1 加 G2 加 G3 备录（%1 无引号低风险、跨机搬 HOME shebang 边角、stamp 内容 ts 与 pwsh mtime 不同源单载体后无影响）
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
