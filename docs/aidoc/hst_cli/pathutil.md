# hst-cli::pathutil

的路径工具面（细则见 R002 与模块文档）。

## Functions

- `abs_display` — Absolute path without Windows `\\?\` prefix. Agent trust stores use this shape.
- `data_dir` — `<parent>/.hst`。旧名（`.oma` 加 `.ohmyagents`）仅旧在、新不在时**同卷
- `find_on_path` — Resolve an executable name through PATH (`which` handles PATHEXT on
- `forward_slash` — 正斜杠形（跨侧键匹配用）。
- `keys_match` — 跨侧配置键匹配（原生与正斜杠双形、Windows 大小写不敏感）。
- `native_slash` — 平台原生斜杠形。
- `norm_key` — Windows 配置键归一：反斜杠加小写（跨侧键匹配）。
- `project_dir` — 路径的项目侧数据根面（细则见 R002 与模块文档）。
- `same_location` — D52 铁证修复共享件：两路径是否同一位置（canonicalize 双侧归一符号
- `user_home` — # Errors

## Constants

- `DIR` — 数据目录名（D29，2026-09-12 用户裁更名 HST）：`.hst`。
- `LEGACY_DIR` — 2026-08-31 至 D14 的更旧名。仅旧在、新不在时改名迁过去。
- `LEGACY_OMA_DIR` — D14 至 D29 的旧名（oma 纪元）。仅旧在、新不在时改名迁过去。

