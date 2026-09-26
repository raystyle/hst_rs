---
id: ADR-0010
title: 状态栏渲染原生化退役pwsh脚本
status: accepted
date: 2026-09-26
deciders:
  - raystyle
supersedes: []
superseded_by: None
tags: ['状态栏', '架构', '性能']
---

# ADR-0010:状态栏渲染原生化退役pwsh脚本

## Context

状态栏渲染载体自 D18 起是生成的 pwsh 脚本（`~/.hst/statusline/hst-statusline.ps1`，约千行 PS1 引擎：shell 探测、git、版本探测缓存、context/token 格式化、loop/goal/goalmode 探针）。用户令（2026-09-26）：全面回归，pwsh 脚本改成 hst 二进制自己实现，hook 执行指向 hst 命令进行输入输出。动因：pwsh 冷启动约 300ms 加 goalmode 探针（105MB transcript）合计约 0.95s/次渲染；PS1 与 Rust 双语言维护（版本探测、loop 探针在两侧重复实现）；goalmode 倒序分块扫描在 PS 实现复杂且踩大小写同变量等坑。

## Decision

状态栏渲染收进 hst 二进制：新子命令 `hst statusline render <agent>`（stdin 喂 agent JSON、stdout 出状态行，契约同现 pwsh 脚本）；claude/kimi/grok 的 statusLine 配置指向 hst 二进制路径；codex 维持内置 ID 面（S016：无外部命令面，不受影响）。探针单源化：版本探测复用 agents.rs、loop/goal 探针复用 loopmgmt.rs、goalmode 倒序分块扫描移植为 Rust（memmap 倒扫）。pwsh 脚本进入弃用期（部署面保留一代供回退，默认不再指向）。hook 面本就指向 hst 命令（shim 形），不在本 ADR 范围。

## Consequences

渲染延迟从约 0.95s 降到毫秒级（无 pwsh 冷启动）；双语言维护面收敛为 Rust 单源；PS1 特有坑族（大小写同变量、转义层）根除。代价：千行渲染引擎移植与回归（五端实弹矩阵必须全绿）；grok ASCII 面与模板定制语义（statusline.toml）需在新实现里保持兼容；弃用期内两载体并存一轮。
