# 2026-10-13 statusbar v2 实现批 REQ-051

## 事

总台五笔回执核过(终稿四行结构加图标分隔律加 grok 邻格评审四点加 task 独立成行),--force 转实现批。lane/statusbar-v2,评审 w9:p3 hst-rev(kimi)。

四行结构落地:L1 身份与项目(agent 版本归一首位加 dir 加 git 加 clock)加 L2 运行时(model 加 context 三形加 duration 加 fleet)加 L3 hook(清单内部 `|` 细分,宽度截断)加 L4 task 独占(文本加 R 轮次加循环时间,goalmode 退化为前缀旗)。退化表 env COLUMNS 检测:40 档保底形(agent 加 dir 与 context 单格条)加 60 档(git 与 model 弃加 task 截 8 加 hook 留一)加 80 档(dir basename 加 fleet 弃加 hook 截三)加 100 档(clock 弃加 task 截 16)。kimi 恒单行专用道(五必留加 task 紧凑形 Rn·age 加 hook 首 id,分支直读 gitBranch 零子进程)。

fleet 快照道:FleetCounts 档形(state/fleet-snap.json)加 parse_agents_view(实装 v2.0.0 的 view 无 --json 面,TEXT 表 token 化:pane 形锚行加态词判定)加 fleet_refresh 原子落档(tmp 加 rename)加 ensure_fleet_refresh(60 秒节流戳 .fleet-snap-check,detach 独立进程组)加 sweeper tick 顺带加 HST_FLEET_REFRESH=off 逃生闸加 `hst task fleet-refresh` 子命令,hook 双腿挂载。渲染纯读零子进程。

## 证

- 单测 242(新增七件:agent 版本归一加 fleet 龄降级三态加 fmt_age 与 dir_basename 加 context 三形与条形格加 kimi 单行加 marker 直立形拒误报加 parse_agents_view 加 fleet_refresh 原子落档)集成 62(新增四件:fleet 段端到端加 fleet-refresh 子命令加 goalmode 前缀旗负例加 example 四行断言)全绿。
- 七门禁绿:fmt 加 clippy(新代码零告警)加 aidoc 重投影加 md 四门禁加 PEVO(装位在本仓 .tools 同源 project-evo,AGENTS 旧路径 /mnt/wsl 已失效待正)。
- 实弹:本机 17 格 agents view 经 hst task fleet-refresh 落快照(W2 I15,hosts local 12 加 lan-linux 1 加 pi-server 1 加 lan-win 3);120 列四行渲染 0.01s(<50ms 判据);39 与 40 与 80 与 99 列边界窗逐档验形;kimi 行实弹验形。
- verify marker 契约扩展:S025 机读标记从冒号两形扩直立形(agent 或 agent-版本 段界收),旧形兼容(装机位旧版渲染过渡期),路径尾段与同前缀名拒认;装机位活弹 verify_live 随新标记绿。

## 坑 记两笔

- rustfmt 双版互斥第二例:evo-lint 静检走 rustfmt edition 2024(大小写混排 use 组 ASCII 序),仓门禁 cargo fmt 走 edition 2021(大小写不敏感序),单组混排必有一版红,且 main 上存量(task.rs 的 serde_json::{json, Value})本就带此冲突 = 每次编辑该文件必弹。解法实证:拆行加中间注释断重排组(json 行与 Value as Json 行夹注释)双版并绿;statusrender 的 statusline use 组同修。存量的 style 级差异(match 尾 `})` 与 `});` 等)无法双版并绿,维持豁免在案(仓门禁 cargo fmt 为权威)。
- 性能实测命令漏 HST_ROOT:task set 误写真实 ~/.hst 在役档(rounds 归零)。当场依 REQ-051 mock 基准(text=eval 批 rounds=3)恢复并在 .task-audit 手记 restore 行。教训:凡跑二进制实测,env 前缀必须逐条目检,尤其 set 类写命令;判据类实测优先用只读面。

## 次

REQ-051 实现批提交推 lane/statusbar-v2 开评审闸门;遗留:goal alias 与 hook goal 摘除预定 2.13(REQ-050 追记在册);ark catalog 滚版管道;doctor detail 旧措辞随 2.13 清;PEVO check.py 装位路径待总台正。

## 装机位先行换防一笔

总台三令同根(实况栏仍显 bash 加 task 无循环时间):装机位 hst 2.12.0 旧默认序与旧模板,新渲染在未发布 lane 构建里。处置:release 构建原子换装(rename 盖 text file busy,直 cp 报错),备份 hst.bak-2.12.0 留根;实况即时四行形(bash 退,R5 · 20m 循环时间显,fleet 刷新)。版本面仍报 2.12.0(版本载体 Cargo.toml,2.13.0 发布批统一),正式装机换防随发布链覆盖。15 分钟检测默认确认:HEAL_IDLE_SECS = 15 * 60(task.rs:800,停摆静默窗),循环时间显示与检测同源读 last_stop_at 零新增检测环。

## 评审轮 hst-rev kimi 2026-10-13

r1 回执:F 一条 G 四条 CONFIRM 八项(F 与 G 详见 REQ-051 追记;回执落盘 target/target-review-statusbarv2/receipt-req051-r1.md)。F-1 = fleet 测试断言面押 ANSI 色码,NO_COLOR=1 环境必红(全仓首个押色码测试,本机 agent shell 即设 NO_COLOR):修为 ENV_LOCK 互斥加剥双开关加 HST_STATUSLINE_NO_ANSI=1 剥色反例锁,验收面 NO_COLOR=1 全测无管道直跑绿(免复审)。G 收养:G-1 example 注释收括加 G-2 goal-ascii 补 mode 占位加 G-3 直立形理论缝注记(模型 id 恰呈 claude-数字.数字 形的启发式缝,现行 id 多段形不触)加 G-4 CHANGELOG Unreleased 节里程碑行。顺手:总台两令 = fleet 段语义答疑(☰ W/N 在跑比总数)加 L4 图标后视觉空格(goal 模板 {icon} 与 {mode} 间加一空格,装机位热换即时生效)。

## 第六笔回执微调一笔

总台令「就显示有多少个 agent 在线可操作」:fleet 段收敛为在线计数 `☰ 18`(W/N 比与 !B 加 Dn 记号退役;快照计数面与龄降级保留,机读消费不受影响)。同批答疑:☰ 后数字 = agents view 在册格位总数(四主机合计,working 加 idle 全算在线可操作)。单测与集成断言随形,门禁全绿,装机位热换实况即显。

第七笔微调:fleet 图标换绵羊形 U+1F411(herd 牧群意象;herdr 本体无固定绵羊字形,官网与二进制核过纯文字隐喻)。图标可 [icons] fleet 覆写;Windows CP936 控制台面 emoji 有 S024 前科,lan-win 侧若见字面问号对可覆写回汉堡形或 nerd 字形。

第八笔微调:fleet 前置 L1 首位(总台令),L2 收敛三段;默认序加 example 加集成断言随动,退化表 id 弃位不变。

第九笔回执:W/N 形恢复且在跑数绿色异显(38;5;114,零在跑回落基色);评审 r1 F-1 教训随行 = 集成色码断言钉死双开关空串,环境 NO_COLOR 不渗子进程。
