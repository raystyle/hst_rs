# hst-cli::goal

`hst goal` 自治面（REQ-045，REQ-019 演进）：自有存储、Stop 事件提醒
腿与状态栏自源显示（用户令 2026-10-11「hst 不再关联 goal 和 loop」）。
hst goal 自治面（REQ-045，REQ-019 演进；用户令 2026-10-11「hst 不再
关联 goal 和 loop」「hst 自己用 hook 实现 goal」）：goal 文本由 hst
自有存储承载（`~/.hst/state/goal/<slug>.json`，slug 规则同 trace 与
projyolo：路径非字母数字一律 `-`），与项目 `.claude/scheduled_tasks.json`
的一切读写解除（该文件回归 Claude Code 原生 CronCreate 域，loop 面
v2.10.0 退役）。Stop 事件腿（`hst hook goal`，注册面 Claude Stop 事件
与 state 腿双挂，仅 claude）：每轮 agent 停止 rounds 计数落盘（状态栏
goal 段的事件驱动数据源），自然停（payload 无 stop_hook_active）出
`decision: block` 加 reason 提醒注入一轮、续跑引发的再停放行（Claude
Code Stop 契约，[实证: code.claude.com/docs/en/hooks 2026-10-11 查：
stop_hook_active 在场须 exit 0 防死环；block 加 reason 续跑]），
恒无死环。CLI 面 set/show/clear 语义承接 REQ-019 管理命令面（项目级，
--project 缺省当前目录；项目键 = CLI 侧项目根与 payload 侧
workspace.project_dir 同一 slug，子目录调用键不中，须在项目根跑或显式
给 --project，边界同 projyolo marker 先例）。

## Functions

- `clear_goal` — 清空本项目 goal（删文件；返回是否真删，无 goal 零改动合法）。
- `goal_dir` — goal 存储目录（`<hst 自管根>/state/goal/`；测试经 HST_ROOT 缝注入）。
- `goal_path` — 项目 goal 文件路径（`<dir>/<slug>.json`）。
- `reminder_json` — Stop 提醒 JSON（Claude Code Stop 契约：`decision: block` 加 `reason`
- `set_goal` — 设置本项目 goal（覆盖旧 goal，rounds 归零重计）。
- `show_goal` — 查看本项目 goal（无 goal 返回 None，查看面不受阻）。
- `slug` — 项目键 slug（规则同 trace 与 projyolo marker：路径非字母数字一律 `-`）。
- `stop_hook` — Stop 腿入口（`hst hook goal`）：非 Stop 事件或本项目无 goal 返回

## Types

- `GoalError` — goal 面错误（承接 REQ-021 类型契约先例）：`Display` 面向人，
- `GoalRecord` — goal 记录（自有存储单文件形，项目一键一文件）。

