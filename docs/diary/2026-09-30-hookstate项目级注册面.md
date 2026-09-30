# 2026-09-30：hookstate项目级注册面

> 用户报：prs_c2coe 工位状态栏没有显示项目级别的 hook。取证定因：hooked_aliases 只读用户级注册面，claude 的项目级 .claude/settings.json 零覆盖（REQ-039 补源）。

## 流水

1. **取证与修法（REQ-039）**：prs_c2coe 项目级挂三守卫（session-tool-guard 加 terminology-guard 加 knowledge-recall），行只显用户级三别名。修：hooked_aliases 增 payload project_dir 形参，claude 家并读项目级 `.claude/settings.json` 加 `settings.local.json`（stem 集合去重天然防重；未收录守卫 stem 按既有语义字典序殿后，不入别名表）。codex 加 kimi 加 grok 项目级不在源内（未实证候裁，边界在册）。
2. **测试与实弹**：既有件 hooked_aliases_list_known_foreign_and_fallback 扩项目级断言（settings 加 local 双文件并入加去重加殿后序）；实弹 debug 二进制喂 prs_c2coe 真 payload，行渲染 `herdr agent状态监控 | hst token护栏 | hst 会话状态同步 | knowledge-recall | session-tool-guard | terminology-guard` `[实证: 实弹渲染前后对照加单测]`。
