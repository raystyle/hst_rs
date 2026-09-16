# hst-cli::trace

trace 六视图：联邦读四家原生会话库归一检索（P0013/P0014，D19）。
hst trace：查询时联邦的四家会话日志检索（P0013，S019）。
直接读各家原生会话库并归一化——零采集设施、可回溯 hst 部署前的历史。
写库即归一化原则（S018 坑 3）：文件路径一律正斜杠；项目比较统一「正斜杠 + 小写」。

## Functions

- `apply_filter` — apply_filter：trace 六视图联邦检索的公开入口（行为细则与 marker 见 R002）。
- `apply_filter_counted` — 过滤加窗口计数（D26）：返回（窗口内事件，过滤后总数）。事件流为时间
- `claude_events` — claude transcript：父链近似为行序——tool_use 前最近的 assistant text 是操作意图、
- `claude_project_slug` — `~/.claude/projects/<slug>/`：slug 规则是路径串里非字母数字一律换 `-`
- `claude_sessions_in` — claude_sessions_in：trace 六视图联邦检索的公开入口（行为细则与 marker 见 R002）。
- `codex_events` — codex rollout：编辑主源是 `event_msg/item_completed` 的 `FileChange` item（绝对路径 +
- `codex_sessions_under` — codex rollout：首行 session_meta.payload.cwd 决定项目归属。
- `file_matches` — 文件过滤：glob 有则用 glob 匹配，解析失败退回子串（S018：regex 非法退字面子串的同款姿态）。
- `grok_events` — grok 双源分发（S020）：updates.jsonl 是权威日志，chat_history.jsonl 是派生缓存
- `grok_sessions_in` — grok：`~/.grok/sessions/<百分号编码的项目路径>/<会话uuid>/`。
- `group_blocks` — 按首次出现顺序聚合事件为块（一个 operation_id 一块）。
- `kimi_events` — kimi wire.jsonl（协议 1.5，S019 源码核实）：`turn.prompt` 且 `origin.kind=="user"` 是
- `kimi_sessions_in` — kimi：`~/.kimi-code/session_index.jsonl` 行 {sessionId, sessionDir, workDir}；
- `list_sessions` — 四家环境入口：列指定项目的全部会话。
- `ms_to_iso` — ms_to_iso：trace 六视图联邦检索的公开入口（行为细则与 marker 见 R002）。
- `parse_apply_patch` — 解析 apply_patch 补丁头的 Add/Update/Delete File 行。
- `search_matches` — 检索：query 按正则匹配 patch、file、双意图四域；非法正则退回字面子串。
- `timeline` — 四家环境入口：项目内全部编辑事件，按统一 epoch ms 排序（无时间的排最后）。
- `ts_to_ms` — "2026-08-31T09:24:32.851Z" 形（毫秒可选）→ epoch ms；纯数字串原样解析。

## Types

- `EditKind` — EditKind：trace 六视图联邦检索的取值集。
- `TraceBlock` — 意图操作块：同一 operation_id（一次工具调用，可能多文件）的事件聚合。
- `TraceEvent` — 归一化事件：四家 loader 的公共产出。operation_id = session_id:call_id（S018 核心设计）。
- `TraceFilter` — TraceFilter：trace 六视图联邦检索的数据面。
- `TraceSession` — TraceSession：trace 六视图联邦检索的数据面。

## Constants

- `DEFAULT_LIMIT` — 分页 clamp（S018 参数形状）。
- `MAX_LIMIT` — MAX_LIMIT：trace 六视图联邦检索常量。

