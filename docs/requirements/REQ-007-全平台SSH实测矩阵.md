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

- [x] SSH 连通性探测（lan-mac 与 lan-ubuntu BatchMode 实证 OK；lan-win 22 拒绝，通道待宿主开 OpenSSH Server 后启用）
- [x] .tools/cross-test.sh：本地构建目标平台产物、scp 到各端、跑 --version 与 doctor 冒烟、汇总矩阵
- [x] 实测一轮三端绿（linux 本机加 mac 加 ubuntu）
- [x] AGENTS Commands 与 .tools/README 登记
