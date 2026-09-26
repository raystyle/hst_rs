# hst-cli::hook

`hst hook`：事件到四态映射、用户级 session 分键 state 落盘与密钥拦截分流（D28/S030）。

## Functions

- `map_event` — Map a hook event (already normalized) to a four-state label.
- `run` — # Errors
- `run_state` — REQ-028 state 腿（payload 穿透命令族）：只做四态写盘（D28 读序含会话
- `run_token` — REQ-028 token 腿（payload 穿透命令族）：只做 secretguard 密钥扫描
- `state_for_payload` — Claude Notification is mixed (tips vs permission). Only permission-shaped

## Types

- `HookOutcome` — hook 出口：状态通道 + 密钥 guard（S030 第二职责）。

