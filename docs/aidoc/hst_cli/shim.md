# hst-cli::shim

shim 三形态自包含状态写入器：cmd/ps1/sh 加 grok 包装（D27/D28/D39）。
用户级 hook 壳（REQ-032 载体全原生）：thin 透传，cmd 与 sh 两载体，
全部逻辑收进 hst 二进制本体（`hst hook state/token/pentest`）。部署时
烘焙部署方二进制绝对路径（self update 原位替换，路径不漂）。D28 自包
含设计退役记档：hst 缺位或故障时 shell 退出码非 2，agent 侧按非阻塞
错误处理 = fail-open（状态通道回落 unknown）；M060a 白名单语义由 hst
本体内化（token/pentest 腿 exit 硬约束 0 与 2，仅自判 block 出 2）。
grok 的 Windows 单路径包装（M048）保留：包装烘焙 agent 参转调 thin
cmd。Windows 注册走 thin ps1 一行壳（REQ-032 评审 F2 方案③，D39 三壳
通吃形 powershell.exe -File <正斜杠>）；.cmd 只留 grok 包装与手工面。

## Functions

- `deploy_shims` — # Errors
- `deploy_shims_with` — # Errors
- `grok_wrapper` — grok 单路径包装生成（M048：Windows grok 只认可整串 spawn 的单路径；
- `host_shell` — 宿主 shell 选择（M060b 抽出成映射）：macOS 落 zsh shebang（缺
- `thin_cmd` — thin cmd 壳生成（REQ-032）：`"<exe>" hook <腿> --agent %1`，stdin 由
- `thin_ps1` — thin ps1 壳生成（REQ-032 评审 F2 方案③：一行壳 `& "<exe>" hook <腿>
- `thin_sh` — thin sh 壳生成（REQ-032，用户令「sh要兼容zsh和bash」）：shebang 按宿主

