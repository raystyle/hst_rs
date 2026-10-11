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
给 --project）。goal-01（2026-10-11）两修：其一子目录键漂——hook
payload 常只带 cwd 而 agent 多在项目子目录工作，精确 slug 键不中，
解析改就近上溯（goal_file_for）取最近在册 goal；其二自愈面
（run_heal，用户令「如果没用唤醒操作 herdr api 自愈唤醒一下」）
——goal 在册而停摆未被唤醒过静默窗时经 herdr agent prompt 唤醒，
blocked 等人裁与频控风暴闸豁免，提醒与唤醒文本均附 hst goal 用法
（用户令「让 agent 自行操作」）。

## Functions

- `clear_goal` — 清空本项目 goal（删文件；返回是否真删，无 goal 零改动合法）。
- `goal_dir` — goal 存储目录（`<hst 自管根>/state/goal/`；测试经 HST_ROOT 缝注入）。
- `goal_file_for` — 就近上溯解析（goal-01 根因 B 修）：hook payload 常只带 cwd 而 agent
- `goal_path` — 项目 goal 文件路径（`<dir>/<slug>.json`）。
- `heal_sweep` — hook 事件流挂载的自愈扫描入口（goal-01 澄清令：自愈须事件驱动实时，
- `heal_verdict` — 自愈判定（纯函数；检测形态 = goal-01 裁量口径：rounds 无增长加 Stop 后
- `is_stop_event` — 事件是否 Stop（事件参数形或 payload hook_event_name 形；heal_sweep 的
- `parse_agent_list` — `herdr agent list` stdout 解析（纯函数面，测试注入 JSON 串）。
- `read_record` — 读指定 goal 文件（缺失 = None；坏损 = Err 响亮）。
- `reminder_json` — Stop 提醒 JSON（Claude Code Stop 契约：`decision: block` 加 `reason`
- `run_heal` — 自愈扫描入口（`hst goal heal`）：扫在册 goal（`only` 给定则单项目），
- `set_goal` — 设置本项目 goal（覆盖旧 goal，rounds 归零重计）。
- `show_goal` — 查看本项目 goal（无 goal 返回 None，查看面不受阻）。
- `slug` — 项目键 slug（规则同 trace 与 projyolo marker：路径非字母数字一律 `-`）。
- `stop_hook` — Stop 腿入口（`hst hook goal`）：非 Stop 事件或本项目无 goal 返回
- `wake_text` — 自愈唤醒文本（herdr agent prompt 载体；附 hst goal 用法，用户令「让

## Types

- `GoalError` — goal 面错误（承接 REQ-021 类型契约先例）：`Display` 面向人，
- `GoalRecord` — goal 记录（自有存储单文件形，项目一键一文件）。
- `HealReport` — 单 goal 的自愈回执行（kv 面）。
- `HealVerdict` — 自愈判定两态族（纯函数面；输入 = goal 在册态加 herdr 格状态加当下时
- `PaneFace` — herdr 格状态面（`herdr agent list` 的 result.agents 投影，消费面只读

## Constants

- `HEAL_IDLE_SECS` — 自愈静默窗（goal-01 频控即此一闸）：goal 在役而 max(last_stop_at,
- `HEAL_STORM_CAP` — 风暴闸：连续无 rounds 增长的自愈唤醒次数上限（Stop 事件回场即归零），
- `SWEEP_MIN_GAP_SECS` — 扫描盖章限频（goal-01 澄清令「事件驱动、零轮询」面的频控外闸）：

