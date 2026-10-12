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
additionalContext 提醒注入一轮（goal-02 干净显示形，续跑同 decision
block 语义）、续跑引发的再停放行（Claude
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

REQ-047（2026-10-12 架构批）：自驱机制升两维态。检测三路（Stop 事件
流加 herdr 格态加 Notification/PermissionRequest 带内快道）；判定矩阵
六序（no-agent、working 不扰、blocked 分型、idle 唤醒、频控、风暴）；
blocked 不再静默豁免，alarm 报警留痕（`.heal-log` 加 show 面），预授权
（`hst goal yolo-heal on`）下反复 blocked 触发 yolo 治（落键加重启格加
续 goal）；间隔自驱 = hook 起 sweeper 有界子进程（60 秒 tick、2 小时
寿命、`.sweeper-lock` 单实例锁 pid 加 token 身份验主、tick 加最坏
heavy 两倍陈旧接管），`HST_GOAL_SWEEPER=off`
逃生闸。wake 文本 v2 自包含五件（goal 文本加上轮收束态加即刻推进加
防卡死防空转边界加用法）。

goalhard-01（2026-10-12 goal 原语地基批）：舰队互视互改地基加固。
写原语四硬化——唯一 tmp 名（并发写者不互吞，LWW 语义下 set/clear
永不假错）、写后回读验非空（真源写后验教训）、unix fsync（断电窗
耐久）、`.bak` 上次好档维护；审计道 `.goal-audit`（set/clear/repair
管理动作留痕，host 加 user 加 pid 归属）；坏档自愈 = 隔离留证加
bak 晋升或判缺省（doctor 体检面保持只读纯读）；风暴耐久序 = 唤醒
节流先落盘后发 prompt、yolo 治冷却先落盘后跑治序列（写盘故障不再
引发每趟重试风暴）、sweeper 每趟至多一个治动作；增量路写前整档
快照乐观守卫（并发 set 不被增量旧档吞，同秒只换文本亦判变）；机读面
`goal show` json 信封加 `goal list` 全机清单（插件 list --all 聚合消费）。

## Functions

- `blocked_fastpath` — blocked 带内快道（REQ-047 两维态的实时腿）：goal 腿挂上 Notification 与
- `clear_goal` — 清空本项目 goal（删文件；返回是否真删，无 goal 零改动合法）。`.bak`
- `ensure_sweeper` — hook 起 sweeper（REQ-047 间隔自驱的起动面）：goal 在册且无鲜活锁即
- `goal_dir` — goal 存储目录（`<hst 自管根>/state/goal/`；测试经 HST_ROOT 缝注入）。
- `goal_file_for` — 就近上溯解析（goal-01 根因 B 修）：hook payload 常只带 cwd 而 agent
- `goal_path` — 项目 goal 文件路径（`<dir>/<slug>.json`）。
- `heal_sweep` — hook 事件流挂载的自愈扫描入口（goal-01 澄清令：自愈须事件驱动实时，
- `heal_verdict` — 自愈判定（纯函数；检测形态 = goal-01 裁量口径：rounds 无增长加 Stop 后
- `is_stop_event` — 事件是否 Stop（事件参数形或 payload hook_event_name 形；heal_sweep 的
- `list_goals` — 列本机全部在册 goal（goalhard-01）：按统一收集器 goal_files 字典
- `parse_agent_list` — `herdr agent list` stdout 解析（纯函数面，测试注入 JSON 串）。
- `read_record` — 读指定 goal 文件（缺失 = None；坏损 = Err 响亮）。并发 clear 竞态
- `read_record_repairing` — 读面自愈（goalhard-01 坏档重建路径）：主档坏损时不再卡死——隔离坏档
- `reminder_json` — Stop 提醒 JSON（goal-02 输出面调优）：`hookSpecificOutput.additionalContext`
- `run_heal` — 自愈扫描入口（`hst goal heal`）：扫在册 goal（`only` 给定则单项目），
- `set_goal` — 设置本项目 goal（覆盖旧 goal，rounds 归零重计；整档最后写胜语义，
- `set_yolo_heal_armed` — yolo 治预授权写侧（`hst goal yolo-heal on|off`）。返回是否真改动
- `show_goal` — 查看本项目 goal（无 goal 返回 None，查看面不受阻）。
- `slug` — 项目键 slug（规则同 trace 与 projyolo marker：路径非字母数字一律 `-`）。
- `stop_hook` — Stop 腿入口（`hst hook goal`）：非 Stop 事件或本项目无 goal 返回
- `sweeper_loop` — sweeper 有界主循环（`hst goal heal --interval` 子进程体）：抢锁让位即
- `wake_text` — 自愈唤醒文本 v2（REQ-047 自包含五件：goal 文本加上轮收束态加即刻推进
- `yolo_heal_armed` — yolo 治预授权读侧（REQ-047）：标记文件在场即 armed。

## Types

- `GoalError` — goal 面错误（承接 REQ-021 类型契约先例）：`Display` 面向人，
- `GoalListRow` — 全机在册 goal 清单行（`hst goal list` 的数据面，goalhard-01 机读
- `GoalRecord` — goal 记录（自有存储单文件形，项目一键一文件）。
- `HealReport` — 单 goal 的自愈回执行（kv 面）。
- `HealVerdict` — 自愈判定两态族（纯函数面；输入 = goal 在册态加 herdr 格状态加当下时
- `PaneFace` — herdr 格状态面（`herdr agent list` 的 result.agents 投影，消费面只读
- `RepairNote` — 坏档修复注记（读面自愈的回执：隔离留证 + 晋升或判缺省）。

## Constants

- `AUDIT_LOG_CAP_BYTES` — 留痕日志上限（goalhard-01）：`.heal-log` 与 `.goal-audit` 追加前查
- `BLOCKED_ALARM_SECS` — blocked 报警防抖窗（REQ-047）：同 goal 两次报警的最小间隔（streak 仍
- `BLOCKED_STORM_THRESHOLD` — yolo 治阈值（REQ-047）：blocked_streak 到此值即「反复 blocked」，预授权
- `HEAL_IDLE_SECS` — 自愈静默窗（goal-01 频控即此一闸）：goal 在役而 max(last_stop_at,
- `HEAL_STORM_CAP` — 风暴闸：连续无 rounds 增长的自愈唤醒次数上限（Stop 事件回场即归零），
- `SWEEPER_LIFE_SECS` — sweeper 单次寿命（REQ-047 有界性）：到点即退，续命须新 hook 事件
- `SWEEPER_STALE_SECS` — sweeper 锁陈旧阈（REQ-047 单实例）：心跳重写超此时长即判死锁接管。
- `SWEEPER_TICK_SECS` — sweeper tick 间隔（REQ-047 间隔自驱）：有界子进程的检查节拍。
- `SWEEP_MIN_GAP_SECS` — 扫描盖章限频（goal-01 澄清令「事件驱动、零轮询」面的频控外闸）：
- `TMP_STALE_SECS` — 崩溃残件 tmp 的清扫阈（goalhard-01）：唯一 tmp 名后写者不再互踩，
- `YOLO_HEAL_COOLDOWN_SECS` — yolo 治冷却窗（REQ-047）：两次治动作的最小间隔（成败同计，防锤）。

