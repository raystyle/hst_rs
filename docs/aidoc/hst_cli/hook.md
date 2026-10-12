# hst-cli::hook

`hst hook`：事件到四态映射、用户级 session 分键 state 落盘与密钥拦截分流（D28/S030）。

## Functions

- `event_label` — 事件标签公共解析（REQ-047 goal 快道共用）：事件参数形直 normalize,
- `map_event` — Map a hook event (already normalized) to a four-state label.
- `read_stdin_json` — 读 stdin 全量为 JSON（空串与坏形返回 None；REQ-045 起共用给 goal 腿
- `run_token` — token 腿入口（`hst hook token`）：只做 secretguard 密钥扫描（block 级
- `state_for_payload` — Claude Notification is mixed (tips vs permission). Only permission-shaped

