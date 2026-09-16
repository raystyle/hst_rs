---
id: REQ-004
title: grok运行时退单排pwsh闸门断言
status: implemented
priority: should
trace: src/statusline.rs default_layout_renders_two_lines_and_explicit_third_row_opts_in（pwsh 闸门行为测试）
---

# REQ-004:grok运行时退单排pwsh闸门断言

## Scenario

kimi/grok 运行时退单排的 pwsh 闸门断言只剩 grok 未钉（codex D40 评审 G2；D42 已顺带钉 kimi）。

## Criteria

- [x] grok 缺 pwsh 退单排断言落地：与 D42 kimi 钉同载体同测试（default_layout 行为测试的 pwsh 闸门内），三条判据一次过绿 `[实证: 2026-09-16 同配置并一行（1 行）加 agent 态与 token 绝对值可见（grok-2.1.270:unknown 与 195k/977k）加 ASCII 面全行无 Nerd PUA 字形（M046 运行时钉）]`
