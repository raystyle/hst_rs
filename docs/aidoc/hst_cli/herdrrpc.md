# hst-cli::herdrrpc

herdr 本地 NDJSON RPC 最小客户端（ADR-0009、REQ-023）：ping 探活与
agent.prompt 派发两方法，一连接一请求短连接，传输零新依赖。
herdr 本地 NDJSON RPC 最小客户端（ADR-0009、REQ-023）：仅 `ping` 探
活与 `agent.prompt`（带 wait）派发两方法，一连接一请求短连接，不做
事件流长连接。传输零新依赖：Unix 走 std UnixStream，Windows 命名管
道按文件开（`\\.\pipe\` 前缀映射，对齐 herdr 二进制 strings 实证）；
serde_json 组帧；sha256 请求 id（tokio/thiserror/uuid/dirs 不进
Cargo.toml）。socket 路径解析序：`HERDR_SOCKET_PATH` 到
`HERDR_SESSION`（`~/.config/herdr/sessions/<名>/herdr.sock`）到缺省
`~/.config/herdr/herdr.sock`。本通道不注入任何 token：本地 socket
权限（srw-------）即边界（评审 F2 回填；用户工件情报
HERDR_SOCKET_PASSWORD 经 schema 加文档加二进制加全站检索四路核实
非 herdr 机制）。

## Functions

- `agent_prompt` — 派发即回执：`agent.prompt` 带 wait（until 三态含 blocked，blocked 单
- `call` — 发一次 NDJSON RPC（一连接一请求）：写请求行，读一行响应，id 配对
- `ping` — 探活：`ping` 一发（返回 pong result 原样）。
- `socket_path` — socket 路径解析：`HERDR_SOCKET_PATH` 到 `HERDR_SESSION`

## Types

- `HerdrError` — herdr RPC 面错误（REQ-023）：`Display` 面向人，`code` 给 agent 短码

