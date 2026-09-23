# hst-cli::loopmgmt

`hst loop`：当前会话 durable 定时任务与 goal 管理面（REQ-019）。
loop 与 goal 管理面（REQ-019）。唯一真相 = 项目根
`.claude/scheduled_tasks.json`（Claude Code durable CronCreate 的同源
落盘形，实证 2026-09-23 探针：tasks[] 含 id / cron / prompt /
recurring / createdAt / createdBySessionId / createdByPid /
createdByProcStart）；hst 是该文件的读写管理面（set / list / del），
状态栏 loop 与 goal 段同源只读消费，不自建第二状态文件。会话归属
判据 = `createdBySessionId` 与当前会话 id 等值。goal 语义 = 任务
`prompt` 文本。并发面与 Claude Code 同款读改写（无锁，后写覆盖先写）；
同一会话内混用 hst 与 agent 原生 cron 工具会互相覆盖盘上变更，管理
纪律是单向（要么全经 hst，要么全经会话内工具）。

## Functions

- `at_to_cron` — 一次性时刻转 cron：`HH:MM`（24 小时制）得 `M H * * *` 加 recurring
- `del_loops` — 删除任务：target 为任务 id（精确匹配任意任务，不限会话）、`latest`
- `every_to_cron` — 周期间隔转 cron：`Nm`（1 至 59）得 `*/N * * * *`；`Nh`（1 至 23）得
- `list_tasks` — 列出项目定时任务（全量，不按会话过滤；`ours` 标当前会话归属，
- `resolve_session` — 解析「当前会话」：显式 `--session` 优先，其次 agent 会话内环境变量
- `scheduled_tasks_path` — 定时任务文件路径（项目根下 `.claude/scheduled_tasks.json`）。
- `set_loop` — 设置当前会话 loop：`--every` 与 `--at` 二选一（双缺或双给报错），

## Types

- `SetReport` — set 落盘报告：新任务的 id、cron、recurring、归属会话与文件路径。
- `TaskRow` — 任务行（list / del 判据用）：文件内每条任务的展开视图。

