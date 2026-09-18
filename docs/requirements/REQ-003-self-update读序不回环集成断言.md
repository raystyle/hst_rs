---
id: REQ-003
title: self-update读序不回环集成断言
status: implemented
priority: should
trace: tests/cli.rs self_update 两条假基址断言（守卫三态单测随 ADR-0008 退役，由 MirrorPlan::base 三态单测承接）
---

# REQ-003:self-update读序不回环集成断言

## Scenario

run 的读序分支收束（mirror-first 失败后 GitHub 再失败不回环、DefaultFallback 才触发回退）只有两轮真网 e2e 佐证，无断言级测试（codex D48 评审 F3）。

## Criteria

- [x] 假基址集成断言钉「不回环」性质：First 态假基址（127.0.0.1:9 连接拒收）加不存在的仓双失败，断言镜像腿只试一次（update.mirror=failed 恰 1 处）、无 update.fallback=mirror 标记、按源码安装提示收尾退出 0；GitHub 腿判据只看标记不看 detail（404 与 403 与断网同收一支，本机实跑恰逢 403 限流即验证）`[实证: 2026-09-16 tests/cli.rs self_update_mirror_first_does_not_loop_back 绿 0.5s]`
- [x] 三态读序分支覆盖：回退守卫抽命名纯函数 `github_fail_falls_back_to_mirror`（run 内调用零行为变化），单测钉三态（仅 DefaultFallback 真、First 与 Off 假）；Off 态实腿另有集成断言（全程无镜像标记）；First 态与 Off 态接线由假基址集成测试钉，DefaultFallback 实腿仍真网 e2e 佐证（假基址无法注入硬编码默认基址，安全注入需新增 env face 属过度设计，记档）`[实证: 2026-09-16 守卫单测加 Off 态集成断言绿]`
- [x] ADR-0008 追记（2026-09-18 缺省翻镜像优先）：「不回环」改由读序结构承载（镜像腿只首位试一次、GitHub 只整对回落一次、GitHub 再失败不回镜像），守卫两函数（`github_fail_falls_back_to_mirror` 加 `mirror_fallback_base`）退役，三态覆盖由 `MirrorPlan::base` 单测承接；本 REQ 假基址两条集成断言在新读序下复跑仍绿 `[实证: 2026-09-18 tests/cli.rs self_update 两条断言绿]`
