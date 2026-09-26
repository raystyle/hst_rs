---
id: ADR-0009
title: 受限编排面herdr即时生效通道
status: accepted
date: 2026-09-26
deciders:
  - raystyle
supersedes: []
superseded_by: None
tags: ['定位', 'herdr', 'loop']
---

# ADR-0009:受限编排面herdr即时生效通道

## Context

ADR-0001（D15，2026-09-08）裁定 hst 去编排，spawn/send/status/serve/mcp 与 rmux 后端整体移除。2026-09-26 实证（diary 流水 2、3、6）：hst 写盘的 loop/goal 对在跑会话不生效（调度器真相在内存注册表，盘上文件是重启装载镜像），仅会话启动装载生效；舰队需要在跑会话即时生效。herdr 本地 NDJSON RPC（Unix socket 与 Windows 命名管道）的 `agent.prompt` 带 `wait` 单请求可派指令到在跑 agent 走原生 CronCreate 内存通道，即时生效。用户裁定（2026-09-26）局部解除 D15 的 send 面禁令。

## Decision

hst 内嵌 herdr NDJSON RPC 最小客户端，仅服务 loop/goal 即时生效派发：`ping` 探活与 `agent.prompt`（带 wait）两方法，`hst loop set 与 loop del 加 goal set 与 goal clear --via-herdr <target>` 四入口（当前会话面维持既有盘面命令不动）；一连接一请求短连接，不做事件流长连接、不恢复 spawn/status/serve/mcp。传输零新依赖：std Unix socket、Windows 管道按文件开（`\\.\pipe\` 前缀映射，对齐 herdr 二进制）、serde_json 组帧、sha256 请求 id（tokio/thiserror/uuid/dirs 不进 Cargo.toml）。

## Consequences

ADR-0001 的 D15 在本通道范围内局部解除（token 注入禁令 D20 不动：本通道不注入任何 token，本地 socket 权限即边界；工件口令情报 HERDR_SOCKET_PASSWORD 经评审四路核实非 herdr 机制，F2 回填）。herdr 缺席或目标不可达时 `--via-herdr` 报错退出，不静默回落写盘（即时通道与重启装载通道语义不混，hint 与文档双通道口径并存）。节奏契约单源仍在 hst（every_to_cron/at_to_cron 翻译后随指令派发），目标 agent 只执行不翻译。目标边界：时间面四入口仅 Claude Code 格（原生 Cron 家族）；codex 无对等 /loop（其 /goal 是 Goal Mode 条件驱动续跑，与本通道的 durable cron 语义不同物），错发由目标自报无工具。
