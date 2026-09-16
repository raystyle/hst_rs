---
id: REQ-007
title: 全平台SSH实测矩阵
status: implemented
priority: should
trace: .tools/cross-test.sh
---

# REQ-007:全平台SSH实测矩阵

## Scenario

WSL 内可 ssh lan-mac（Darwin arm64）与 lan-ubuntu（Linux x86_64），Windows 宿主机通道待 sshd 开启；三平台实弹测试不再依赖 ohmycloud 舰队转述。

## Criteria

- [x] SSH 连通性探测（lan-mac 与 lan-ubuntu mesh 实证 OK；lan-win 走 127.0.0.1 回环实证 OK，第六十三批连接姿势口径：WSL 到宿主恒走回环不走 mesh IP）
- [x] .tools/cross-test.sh：本地构建目标平台产物、scp 到各端、跑 --version 与 doctor 冒烟、汇总矩阵
- [x] 实测一轮四端绿（linux 本机加 ubuntu 加 mac 加 win 回环，win 端 powershell 冒烟 --version 与 doctor）
- [x] AGENTS Commands 与 .tools/README 登记
- [x] 第五端 lan-linux mesh 纳入矩阵（用户令五端验收 2026-09-16；ssh lan-linux 实证 Linux x86_64 BatchMode 免密，复用 linux 产物；AGENTS 与 .tools/README 同步五端口径）`[实证: 2026-09-16 ssh -o BatchMode=yes lan-linux uname 通过]`
