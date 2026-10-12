# hst-cli::yolo

`hst init --yolo`：四家分级落盘、ours 退役与 pretrust（D33/D52）。

## Functions

- `apply_pretrust` — # Panics
- `apply_project_yolo` — # Errors
- `apply_project_yolo_level` — # Panics
- `apply_user_yolo` — # Errors
- `apply_user_yolo_level` — # Errors
- `apply_user_yolo_level_with` — 用户级分级落键（full / partial；off 调 retire_user_yolo_with）。
- `apply_user_yolo_with` — # Errors
- `clear_project_yolo_interference` — # Panics
- `kimi_workspace_key` — yolo 键的kimi_workspace_key面（细则见模块文档与集成测试）。
- `project_yolo_interferences` — 项目级 yolo 干扰只读检测（REQ-017，检测集与
- `retire_project_yolo` — # Panics
- `retire_user_yolo` — # Errors
- `retire_user_yolo_with` — # Panics
- `user_yolo_active_with` — 用户级 yolo 活性读侧判（REQ-047 goal 治面前置；rev-codex F-1 契约

## Types

- `ApplyReport` — 键落盘报告：写入路径清单。
- `YoloLevel` — yolo 分级（D33，2026-09-13 ohmycloud 协调批）：full = 现行全 bypass；

