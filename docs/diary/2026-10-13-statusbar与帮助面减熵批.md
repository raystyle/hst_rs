# 2026-10-13 statusbar 与帮助面减熵批 REQ-049

## 事

用户令(2026-10-12 追单排 goalhard-01 后,2026-10-13 执行):三件 = hst-state 腿整体剔除(herdr 的 herdr-agent-state hook 已覆盖会话状态同步)加段名纯英文加帮助面去考古。lane/entropy-reduce 批,评审走 w9:p3 hst-rev(kimi)。

state 腿四去:注册矩阵改形(claude 六事件 token×2 加 goal×4 单挂;codex/grok/kimi 只剩 token×2;SessionStart 与 PostToolUse 退出注册面,存量 state 注册经 sweep 与 merge 陈旧替换清出);shim 腿表去 state 加退役清扫四件;CLI 双子命令(hook status 与 hook state)退役;hook.rs state 写盘族死码清零(事件解析公共件与 token 腿存续)。段名表纯英文化(herdr-agent-state 与 hst-goal 直用 id,hst-token 显示 hst-token-guard,hst-state 条目移除)。帮助面 clap about 十八处重写零考古,task 正名先行(用户令 2026-10-13「不叫 goal 了就是 task 任务」:goal 命令组 about 以 task 称谓,命令名更名归 task-migrate-01 批)。

## 证

- 判据重定活弹:verify 的 claude 面 new 判据 = 临时 task 在册,无头会话 Stop 后 rounds 递增。首版实跑红:无头会话依 Stop 提醒里的自清授权把判据档自己 clear(goal-record-vanished);判据档文本明示勿清后实跑绿:`verify.claude=ok 加 verify.claude.statusline=ok 加 verify.claude.hook=ok 加 verify.claude.hook.state=rounds=2`。codex/grok/kimi hook 层 skip 带注(hst 运行时可观测腿仅 claude task 面)。
- 全测:单测 229(hook state 写盘族三件与 grok trust 族随判据退役净减)加集成 57(新增帮助面零考古门遍 22 面断言 REQ-/D 系/P 系/M 系/S 系/ohmycloud/日期零命中)全绿;fmt 加 clippy 加 aidoc 重投影加 md 四门禁加 PEVO 绿(见提交)。
- 实操一笔:本机用户级注册存量为旧形(Stop 仅挂 hst-state),本批 sweep 会在下次 hst init 自动清;活弹 verify 走 UserHooksGuard 备份加 deploy 加还原,不受存量影响。

## 坑 记两笔

- 无头会话对 Stop 提醒的自清授权是真执行不挑场合:验收判据档被会话自清导致判据假阴(goal-record-vanished)。凡以 task 档为判据的无头面,档文本须明示勿清。此坑直接适用于 task-migrate-01 批。
- 大文件块替换三教训承 goalhard-01 而再验:python 批量替换必须逐条 assert(本批一处格式差异被 assert 拦下未酿静默失败);行号定位先读后删;测试种子随接口改形时,期望串与种子要同批改(段名表测试中文期望与 hst-state 种子分两批改导致两轮回红)。

## 评审轮(hst-rev kimi,2026-10-13)

首轮回执:F 三条 G 七条(A 升级清出不闭环,B/C/D/E/F 主项全 CONFIRM)。F-1 statusline no-hook 哨兵判据滞旧(押 hst-state 标记,新形注册下健康装机永久误报 no-hook! 加每小时白跑自愈,实证 scratch 复现;测试种子旧形假绿)→ 判据改押现行腿,旧形注册不认即自愈升级向量。F-2 codex 管理事件旧形 state 条目清出破洞(ours_this_leg 只认 token 加 sweep 跳管理事件,双挂中的 state 条目两路不管而 shim 已删,每次 PreToolUse 调死脚本,实证 scratch 旧形种子复现;claude 与 kimi 面实证净)→ merge 单趟重建外科摘键,回归件在测。F-3 状态栏态源枯竭(评审格明示须总台加用户裁定)→ 呈三选一,用户裁定「态显示随腿退役」:读序保留加 48h 新鲜度闸,陈旧档按缺位收,AGENTS.md 状态写句同步。G 收养五条(G-1 hint 单臂化,G-2 模块注释考古清,G-3 批次代号出 about 加门禁标记扩,G-4 goal 组 about task 化,G-5 verify 措辞收敛),G-6 记档归 task-migrate-01,G-7 缝保留注记。修后 230 加 57 全绿。

评审实操:K3 逐窗权限代批(Ask When Needed 模式,逐命令弹窗, approve-for-session 不粘)与上下文策略窗代答各一笔;评审格自带 todo 清单推进(事实核/diff/残留搜/门禁 worktree 复证/机读面实证),全流程约 50 分钟。

## 次

REQ-049 登记与索引行;评审轮次与回执见追记;task-migrate-01(goal 改名 task 迁移批,用户令五件)接续开工。
