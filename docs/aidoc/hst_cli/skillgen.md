# hst-cli::skillgen

`hst skill` 渲染器：从 clap 活命令树自适应生成 SKILL.md（D22/D49）。
hst 自适应 SKILL 渲染（D22）：从 clap 活命令树生成 SKILL.md（Agent
Skills 标准形态：frontmatter name 加 description 含何时用）。新命令/
新旗标自动出现在生成物里，不需要手维护命令表；`hst skill` 打印、
`hst skill --write` 落用户级 `~/.claude/skills/hst/`（D49 起技能名翻
hst；旧牌 ohmyagents 不留兼容窗，第 2 轮用户裁定直接删除加幂等清扫）。
项目级 init 生成物（deploy.rs COMMAND_MAP 加 marker 覆写语义）是另一
层，不共用渲染器（lib 拿不到 bin 的 Cli 树，双层记档于 R002）。

## Functions

- `render_skill` — 渲染完整 SKILL.md（canonical，name = hst）。

## Constants

- `SKILL_NAME` — 技能生成的D49面（细则见 R002 与模块文档）。

