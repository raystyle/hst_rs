---
id: REQ-032
title: 载体全原生cmd与sh透传
status: implemented
priority: must
trace: shim.rs 重写为 thin 生成函数（sh 按 host_shell 兼容 zsh 加 bash 加 sh、ps1 一行壳、cmd、grok 包装四载体十二件）加 deploy 四家 Windows 注册改 thin ps1 D39 通吃形（powershell.exe -File 正斜杠）加 grok 状态栏壳 thin 化加 is_ours 兼容加 clap 用法错映射 1（F1）；单测 thin 形加十二件幂等加行为透传（stdin 与 exit 2 直通）；全测 266 加 49 绿；实弹重部署十二件落、三腿 e2e（state 写盘加 token 阻断 2 加 pentest 短路）
---

# REQ-032:载体全原生cmd与sh透传

## Scenario

用户令（2026-09-27）：「脱离pwsh 全部原生hst通过cmd sh 透传hst状态栏和hook」。现状 hook 载体是自包含 bash/PS1 shim（D28 零 hst 依赖设计，Windows 注册经 powershell.exe -File ps1 形），状态栏 grok Windows 面仍走 .cmd 壳调 pwsh 脚本（ADR-0010 已知边界）。裁定：全部收进 hst 二进制本体，载体退化为 thin 透传壳（Unix sh 加 Windows 走 D39 三壳通吃形），部署时烘焙部署方二进制绝对路径。

## Criteria

验收判据,可检验、可勾选:

- [x] thin 壳四载体（state/token/pentest 三腿）：sh 形 `exec "<烘焙 exe>" hook <腿> --agent "$1"`（shebang 按宿主 shell 兼容 zsh 加 bash 加 sh，stdin 经 exec 透传）；ps1 一行壳 `& "<烘焙 exe>" hook <腿> --agent $args[0]`（逻辑全在 hst 本体，BOM 落盘，exit $LASTEXITCODE 直通）；cmd 形 `"<烘焙 exe>" hook <腿> --agent %1`（grok M048 包装与手工面）；M060a 语义由 hst 本体内化（token/pentest 腿 exit 硬约束 0 与 2，hst 缺位或故障 shell 退出非 2 即 fail-open）
- [x] 自包含 pwsh 脚本退役：STATE_PS1 加 TOKEN_PS1 加 PENTEST_PS1 生成面删除（ps1 载体回归但只是一行透传壳，无任何业务逻辑）；状态栏 grok Windows 包装 hst-statusline-grok.cmd 改 thin cmd 透传 `statusline --render grok`（stdin 继承）
- [x] 部署重接线：四家 Windows 注册 = thin ps1 走 D39 三壳通吃形 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <正斜杠 ps1> <agent>`（评审 F2 方案③终裁；解释器按名解析恒在、无前导斜杠参数不触发 MSYS 转换、sh 系经 powershell 兼通）；Unix 维持 quoted .sh 形；烘焙 exe = 部署方 current_exe 绝对路径（self update 原位替换路径不漂）；grok 维持 M048 单路径包装（.cmd 只留此面）
- [x] D28 语义翻转记档：state 写盘不再零 hst 依赖（hst 缺位时 hook 全链 fail-open，状态栏回落 unknown）；is_ours 族与清扫面认新旧两形（powershell -File 与 cmd.exe /c 包裹形都作 ours 认领，防存量残留误判外来）
- [x] 测试：thin 壳内容锁（含 ps1 一行壳形）、部署注册形断言（Unix 与 Windows 双侧注入，Windows 期望 powershell -File ps1 形）、exit 语义经 hst 本体（token 阻断 2 加 state 静默 0）、十二件幂等
- [x] 实弹：本机重部署后注册面 thin 形、hook 三腿 e2e 绿、状态栏渲染不变、自包含 pwsh 脚本不再生成
- [x] 评审吸收两轮（F1 加 F2 必修、G1 至 G4）：F1 clap 用法错缺省退 2 污染薄壳「自判 block 出 2」硬约束：main() 改 Cli::try_parse() 把用法错映射 1（help 加 version 的 exit_code 0 原样），集成测试五处期望 2 改 1；F2 两轮收敛终裁方案③：一轮裁定弃 pwsh 后走 `cmd.exe /c "<.cmd>"` 救济形，二轮实证该形正是 D39 当场打回形（Git Bash 里 MSYS 参数转换吃 `/c`），终裁保留一行 thin ps1（body 仅 `& "<exe>" hook <腿> --agent $args[0]`，逻辑仍在 hst 本体，不违背「逻辑原生」）走 D39 通吃形 powershell.exe -File 正斜杠路径，.cmd 只留 grok M048 包装与手工面；G1 采纳（ps1 走按名解释器加参数位正斜杠路径）；kimi 三命令 agent 参一轮迁移误植 codex 已修；G4 回退路径记档（自包含 pwsh 载体已退役，回退需重装旧版二进制或 revert；thin ps1 是新式一行壳非回退）；G2 加 G3 备录（%1 无引号低风险、跨机搬 HOME shebang 边角）
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
