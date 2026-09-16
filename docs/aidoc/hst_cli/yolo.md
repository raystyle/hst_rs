# hst-cli::yolo

`hst init --yolo`：四家分级落盘、ours 退役与 pretrust（D33/D52）。

## Functions

- `apply_pretrust` — Trust stores in the user home. Not hook registration.
- `apply_project_yolo` — 项目级 yolo 与非阻塞键（D28 第 3 轮裁定 2026-09-11：yolo 命令分两级，
- `apply_project_yolo_level` — 项目级分级落键（full / partial；off 调 retire_project_yolo）。
- `apply_user_yolo` — 生产入口：真实家目录。
- `apply_user_yolo_level` — 生产入口：真实家目录按级落键（full / partial）。
- `apply_user_yolo_level_with` — 用户级分级落键（full / partial；off 调 retire_user_yolo_with）。
- `apply_user_yolo_with` — 用户级 yolo 与非阻塞键（D28 第 2 轮裁定 2026-09-11：yolo 模式与非阻塞
- `kimi_workspace_key` — kimi_workspace_key：yolo 键落盘的公开入口（行为细则与 marker 见 R002）。
- `retire_project_yolo` — 项目级旧 yolo 键退役（D28 第 2 轮）：oma 写过的项目面键摘除（值等于
- `retire_user_yolo` — 生产入口：真实家目录用户级退役（off）。
- `retire_user_yolo_with` — 用户级 yolo 键退役（D33 off）：与 retire_project_yolo 同款 ours 等值

## Types

- `ApplyReport` — ApplyReport：yolo 键落盘的数据面。
- `YoloLevel` — yolo 分级（D33，2026-09-13 ohmycloud 协调批）：full = 现行全 bypass；

