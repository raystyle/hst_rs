---
id: REQ-010
title: 统一issue入口集成
status: implemented
priority: must
trace: cargo test --locked（issue.rs 单测三件加 cli 集成）加实弹 issue #6（issues.ohmygh.com/i/6，list 可见）
date: 2026-09-17
owner: raystyle
tags: ['issue', 'REQ-057', 'v2.2.0']
---

# REQ-010：统一 issue 入口集成

## Scenario

总台统一 issue 入口 issues.ohmygh.com 上线（Worker 加 D1 真源，每 IP 10 条每时），hst 集成 `issue` 子命令：agent 使用过程中遇缺陷一键反馈，命令自动带上下文（tool=hst 加版本加平台加主机名）。契约真源 = ohmycloud REQ-057。

## Criteria

- `hst issue new "<标题>" [--body <正文>]`：本地校验（title trim 后 1 至 200）先行；POST /api/issues 体 {tool:"hst", title, body<=20000, version<=40, platform<=64, host<=64}（客户端先截断）；201 回 {ok,id,url} 出 marker 行；429 限速与 400 校验文案透传。
- `hst issue list [--tool hst] [--status] [--limit 1-100]`：GET 列面新到旧；`hst issue show <id>`：详情含正文。
- env `HST_ISSUES_API` 覆盖基址（测与灰度）。
- 输出走 fmtio 三态（kv marker 行加 json 信封加 jsonl 行）。
- agent 纪律入 AGENTS（遇缺陷即 `hst issue new` 一键反馈）。

## trace

issue.rs 单测三件（本地校验先行、平台与主机截断形、数字 id 本地拒）加 cli 集成（无效标题本地拒）加实弹：issue #6 提交且 list 可见、show 全文 `[实证: 2026-09-17 issues.ohmygh.com/i/6]`。

- [x] list 契约扩面（#52 同型修）：默认 limit 100（服务端上限钳制单源）、count 语义（返回条数非在册总数）入 help、返回条数不少于钳制后 limit 时 stderr 截断提示 `[实证: 2026-09-19 --limit 1 实弹提示行 rc=0]`
- [x] list keyset 翻页（#53，参照 browse 0167e8c）：--before <id> 游标透传（取该 id 之前更旧一页；带 before 响应含 has_more 权威翻页信号且饱和提示以它判定，非法值服务端 400 透传归因；不带 before 旧形回执不变）；饱和提示补 --before 翻页出口 `[实证: 2026-09-19 --limit 3 --before 51 取 tool=hst 名下 id<51 降序页加 has_more=true 加提示行 rc=0]`
