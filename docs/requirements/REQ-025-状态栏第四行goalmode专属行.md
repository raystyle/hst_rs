---
id: REQ-025
title: 状态栏第四行goalmode专属行
status: implemented
priority: must
trace: statusline 单测 goalmode 四态一件（active 加 paused 回溯加 clear 覆盖加无标记隐藏）加全测 238 加 49 绿加实弹（真 pentest 105MB transcript 第四行渲染 goal:paused 继续，直到所有vulhub漏洞回归，渲染 0.95s）
---

# REQ-025:状态栏第四行goalmode专属行

## Scenario

用户令（2026-09-26）：第三行列 loop 设置的提示词与状态（现状已满足），第四行列 goal 设置的提示词与状态。fleet 实况：pentest 工位在役 `/goal` 目标 «继续，直到所有vulhub漏洞回归»（transcript 有 active 加 paused 态迁移），现有状态栏无此面。goal 语义对照：本行是 Claude Code Goal Mode（条件驱动续跑），与 hst 的 goal 段（REQ-019 任务 prompt）不同物。

## Criteria

验收判据,可检验、可勾选:

- [x] 行架构扩到四行：statusline.toml 新键 `segments4`、`DEFAULT_SEGMENTS4 = ["goalmode"]`、拼装与渲染四行泛化（空行剔除复用）
- [x] 新段 `goalmode`：goal mode 条件文本加在役态（active/paused），模板 `{icon}goal:{state} {text}` 加 grok `-ascii` 变体加图标键
- [x] 探针 PS1_GOALPROBE：由 stdin session_id 加项目根定位会话 transcript（`~/.claude/projects/<slug>/<会话id>.jsonl`，slug 非字母数字换 `-`），倒序分块流读独立回溯最后态标记与最近捕获（`Goal check-in: «文本» is still active` 取 active 与文本；`Goal paused` 取 paused，文本回溯最近 «»）；无标记或文件缺失整段隐藏；文本截 60 同 goal 段口径
- [x] 无 goal mode 会话第四行整行隐藏回三行（零噪声不变）；老配置升级语义不破（未写 segments4 默认补 goalmode）
- [x] 测试：transcript 夹具三态（active、paused、无标记）渲染断言、四行布局判据、example 锁同步（D51）、门控断言
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
