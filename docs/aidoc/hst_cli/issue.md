# hst-cli::issue

统一 issue 入口客户端面（issues.ohmygh.com，REQ-057 对齐）。
hst issue 面（REQ-057 对齐，总台统一入口 issues.ohmygh.com）：agent
使用过程中遇缺陷一键反馈，自动带上下文（tool=hst 加版本加平台加主机名）。
契约真源 = ohmycloud docs/requirements/REQ-057（POST /api/issues 体与
校验、GET 列表与详情、每 IP 10 条每时限速）；本模块是客户端投影，
env `HST_ISSUES_API` 覆盖基址（测与灰度）。

## Functions

- `base_url` — API 基址（env `HST_ISSUES_API` 覆盖，缺省总台统一入口）。
- `clamp_issue_limit` — issue list 的 limit 钳制（1 至 100，服务端上限；#52 同型修）：钳制
- `file_issue` — 提交面（new）：title trim 后 1 至 200、body 至多 20000、version 至多 40
- `issue_list_truncation_hint` — issue list 饱和提示行（#52 同型修）：返回条数打满钳制后 limit 时出
- `list_issues` — # Errors
- `render_show_kv` — 详情面 kv 行渲染（codex 三面批二轮 F1 G3：抽纯函数加单测根治「无测试
- `show_issue` — # Errors

## Types

- `Filed` — 统一 issue 入口的提交回执面（细则见模块文档）。

