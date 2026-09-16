# hst-cli::trace

trace 六视图：联邦读四家原生会话库归一检索（P0013/P0014，D19）。
hst trace：查询时联邦的四家会话日志检索（P0013，S019）。
直接读各家原生会话库并归一化——零采集设施、可回溯 hst 部署前的历史。
写库即归一化原则（S018 坑 3）：文件路径一律正斜杠；项目比较统一「正斜杠 + 小写」。

## Functions

- `apply_filter` — trace 六视图的apply_filter面（细则见 R002 与模块文档）。
- `apply_filter_counted` — 过滤加窗口计数（D26）：返回（窗口内事件，过滤后总数）。事件流为时间
- `claude_events` — claude transcript：父链近似为行序——tool_use 前最近的 assistant text 是操作意图、
- `claude_project_slug` — `~/.claude/projects/<slug>/`：slug 规则是路径串里非字母数字一律换 `-`
- `claude_sessions_in` — trace 六视图的claude_sessions_in面（细则见 R002 与模块文档）。
- `codex_events` — codex rollout：编辑主源是 `event_msg/item_completed` 的 `FileChange` item（绝对路径 +
- `codex_sessions_under` — codex rollout：首行 session_meta.payload.cwd 决定项目归属。
- `file_matches` — trace 六视图的文件过滤面（细则见 R002 与模块文档）。
- `grok_events` — grok 双源分发（S020）：updates.jsonl 是权威日志，chat_history.jsonl 是派生缓存
- `grok_sessions_in` — trace 六视图的grok面（细则见 R002 与模块文档）。
- `group_blocks` — # Panics
- `kimi_events` — kimi wire.jsonl（协议 1.5，S019 源码核实）：`turn.prompt` 且 `origin.kind=="user"` 是
- `kimi_sessions_in` — trace 六视图的kimi面（细则见 R002 与模块文档）。
- `list_sessions` — trace 六视图的四家环境入口面（细则见 R002 与模块文档）。
- `ms_to_iso` — trace 六视图的ms_to_iso面（细则见 R002 与模块文档）。
- `parse_apply_patch` — 解析 apply_patch 补丁头的 Add/Update/Delete File 行。
- `search_matches` — trace 六视图的检索面（细则见 R002 与模块文档）。
- `timeline` — trace 六视图的四家环境入口面（细则见 R002 与模块文档）。
- `ts_to_ms` — "2026-08-31T09:24:32.851Z" 形（毫秒可选）→ epoch ms；纯数字串原样解析。

## Types

- `EditKind` — trace 六视图的EditKind面（细则见 R002 与模块文档）。
- `TraceBlock` — trace 六视图的意图操作块面（细则见 R002 与模块文档）。
- `TraceEvent` — trace 六视图的归一化事件面（细则见 R002 与模块文档）。
- `TraceFilter` — trace 六视图的TraceFilter面（细则见 R002 与模块文档）。
- `TraceSession` — trace 六视图的TraceSession面（细则见 R002 与模块文档）。

## Constants

- `DEFAULT_LIMIT` — 分页 clamp（S018 参数形状）。
- `MAX_LIMIT` — trace 六视图的MAX_LIMIT面（细则见 R002 与模块文档）。

