# hst-cli::hook

`hst hook`：事件到四态映射、用户级 session 分键 state 落盘与密钥拦截分流（D28/S030）。

## Functions

- `map_event` — Map a hook event (already normalized) to a four-state label.
- `run` — Hook entry: always exit-path friendly. `HST_STATE_FILE` 覆盖互斥
- `state_for_payload` — Claude Notification is mixed (tips vs permission). Only permission-shaped

## Types

- `HookOutcome` — hook 出口：状态通道 + 密钥 guard（S030 第二职责）。

