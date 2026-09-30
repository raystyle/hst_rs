---
id: REQ-039
title: 状态栏hookstate项目级注册面
status: implemented
priority: must
trace: statusrender 单测 hooked_aliases_list_known_foreign_and_fallback 扩件（项目级 settings 加 settings.local 并入加 stem 去重加未收录守卫字典序殿后）；实弹 prs_c2coe 真 payload 渲染三守卫入列
---

# REQ-039:状态栏hookstate项目级注册面

## Scenario

用户报（2026-09-30）：prs_c2coe 工位状态栏 hookstate 行只显用户级三别名（herdr agent状态监控加 hst token护栏加 hst 会话状态同步），项目级 hook 不显示。取证：该项目 `.claude/settings.json` 挂三守卫（PreToolUse Bash 的 session-tool-guard、PreToolUse Edit 加 Write 的 terminology-guard、UserPromptSubmit 的 knowledge-recall），hooked_aliases 只读用户级注册面（`~/.claude/settings.json` 族），项目级零覆盖 `[实证: 项目 settings.json 对读加实弹渲染前后对照]`。

## Criteria

验收判据,可检验、可勾选:

- [x] claude 家并读项目级注册面：payload `workspace.project_dir` 下 `.claude/settings.json` 加 `.claude/settings.local.json`，命令并入收集
- [x] stem 去重天然防用户级与项目级同 stem 重复
- [x] 未收录 stem（项目守卫族）按既有语义字典序殿后回落本名，不新增别名表项（项目守卫非舰队公共面）；跨文件同 stem 去重（评审 G3 夹具锁）
- [x] codex 加 kimi 加 grok 项目级不在源内（项目级注册面未实证，候裁）
- [x] 测试：既有件扩项目级断言；实弹真 payload 三守卫入列

## 边界

- project_dir 缺失或空（payload 无 workspace.project_dir）时只读用户级，行为与旧版一致。
- 项目守卫显示为脚本干 stem 本名（session-tool-guard 族），不入 HOOK_ALIASES 别名表。
