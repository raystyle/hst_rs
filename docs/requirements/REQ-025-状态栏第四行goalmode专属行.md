---
id: REQ-025
title: 状态栏第四行goalmode专属行
status: implemented
priority: must
trace: statusline 单测 goalmode 四态一件加多块跨块回溯与块界重叠一件（评审 F1/F2 复现形）加门控与 segments4 语义一件加 example 锁；全测 240 加 49 绿；实弹真 pentest 105MB transcript 双态渲染（评审复现配方跨块与边界形双绿）
---

# REQ-025:状态栏第四行goalmode专属行

## Scenario

用户令（2026-09-26）：第三行列 loop 设置的提示词与状态（现状已满足），第四行列 goal 设置的提示词与状态。fleet 实况：pentest 工位在役 `/goal` 目标 «继续，直到所有vulhub漏洞回归»（transcript 有 active 加 paused 态迁移），现有状态栏无此面。goal 语义对照：本行是 Claude Code Goal Mode（条件驱动续跑），与 hst 的 goal 段（REQ-019 任务 prompt）不同物。

## Criteria

验收判据,可检验、可勾选:

- [x] 行架构扩到四行：statusline.toml 新键 `segments4`、`DEFAULT_SEGMENTS4 = ["goalmode"]`、拼装与渲染四行泛化（空行剔除复用）
- [x] 新段 `goalmode`：goal mode 原始参数文本（风格统一令 2026-09-26：态不入显示，{state} 占位符保留），模板 `{icon}goal {text}`（去冒号形，用户令 2026-09-26） 加 grok `-ascii` 变体加图标键
- [x] 探针 PS1_GOALPROBE：由 stdin session_id 加项目根定位会话 transcript（`~/.claude/projects/<slug>/<会话id>.jsonl`，slug 非字母数字换 `-`），倒序分块流读独立回溯最后态标记与最近捕获（`Goal check-in: «文本» is still active` 取 active 与文本；`Goal paused` 取 paused，文本回溯最近 «»）；无标记或文件缺失整段隐藏；文本截 60 同 goal 段口径
- [x] 无 goal mode 会话第四行整行隐藏回三行（零噪声不变）；老配置升级语义不破（未写 segments4 默认补 goalmode）
- [x] 测试：transcript 夹具四态（active、paused 文本回溯、clear 覆盖、无标记）渲染断言、多块夹具跨块回溯与块界重叠两形（评审 F1/F2 复现形）、门控断言（仅 goalmode 注 GOALPROBE、无 goalmode 不注、跨行去重单次注入）、segments4 空数组抑制、example 锁同步（D51 含 segments4 与 goalmode 模板）
- [x] 已知边界：状态词表是闭集（check-in/paused 加 clear/off/stop 三形），上游新态词（完成/过期类）会陈旧恒显，出现时扩词表；clear 正则无角色锚，助手引用该串亦清态（显示面容错）；无 goal 大 transcript 全扫约 1s/次重绘（106MB 实测），头寸留 (path,size,mtime) 缓存或限次回看；GOALPROBE 隐式依赖 LOOPPROBE 前置变量（契约注释在册，评审 G2）
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
