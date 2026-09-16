# hst-cli::pathutil

路径工具：家目录与 hst 根解析、跨侧键匹配（D28/D29）。

## Functions

- `abs_display` — Absolute path without Windows `\\?\` prefix. Agent trust stores use this shape.
- `data_dir` — `<parent>/.hst`。旧名（`.oma` 加 `.ohmyagents`）仅旧在、新不在时**同卷
- `find_on_path` — Resolve an executable name through PATH (`which` handles PATHEXT on
- `forward_slash` — forward_slash：路径工具的公开入口（行为细则与 marker 见 R002）。
- `keys_match` — keys_match：路径工具的公开入口（行为细则与 marker 见 R002）。
- `native_slash` — native_slash：路径工具的公开入口（行为细则与 marker 见 R002）。
- `norm_key` — norm_key：路径工具的公开入口（行为细则与 marker 见 R002）。
- `project_dir` — 项目侧数据根：`<project>/.oma`（session / state / tasks）。
- `same_location` — D52 铁证修复共享件：两路径是否同一位置（canonicalize 双侧归一符号
- `user_home` — 用户家目录解析（D28）：`HST_USER_HOME` 覆盖优先（集成测试与 verify 的

## Constants

- `DIR` — 数据目录名（D29，2026-09-12 用户裁更名 HST）：`.hst`。
- `LEGACY_DIR` — 2026-08-31 至 D14 的更旧名。仅旧在、新不在时改名迁过去。
- `LEGACY_OMA_DIR` — D14 至 D29 的旧名（oma 纪元）。仅旧在、新不在时改名迁过去。

