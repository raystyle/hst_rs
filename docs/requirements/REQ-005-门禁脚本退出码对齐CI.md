---
id: REQ-005
title: 门禁脚本退出码对齐CI
status: rejected
priority: should
trace: 假违规文件三环境退出码实证（diary 2026-09-16）
---

# REQ-005:门禁脚本退出码对齐CI

## Scenario

mdcharlint 与 md-heading-scan 本地违规退出码为 0，命令链拦不住（GOAL 破折号两度靠 CI 兜底）。

## Criteria

- [x] 两脚本违规时退出码为 1：初版即如此，无需改 `[实证: 2026-09-16 git -S 证 return 1 逻辑自首提交（2e1ce7b 加 17639a7）至今未变]`
- [x] 本地链路复跑验证：假违规文件实测两链均拦停 `[实证: 2026-09-16 WSL bash 链 exit 1 未续行；Windows pwsh 7 回环 LASTEXITCODE 1 未续行]`；CI docs-gate 同命令同 shell 语义 `[推断: dev-release.yml docs-gate 步骤命令与本地逐字同串，本批未推未实跑（codex 评审 O4 改标）]`

## 裁定

rejected（2026-09-16）：三环境实证退出码均 1 且链路拦停，场景前提不成立，无需改码。09-15 diary 流程观察到的「退出码 0」与 M033 管道吞码同型（门禁命令接 `| tail` 后读到的是管道尾退出码），正解已录 M033（门禁裸跑加 `&&` 直连），不落脚本面 `[推断: 观察口径与 M033 同型，脚本本体无缺陷]`。
