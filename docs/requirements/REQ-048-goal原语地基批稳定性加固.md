---
id: REQ-048
title: goal原语地基批稳定性加固
status: implemented
priority: must
trace: goal 单测新增十件(goal_files_filter_and_stale_tmp_swept 加 heal_and_goal_audit_logs_rotate_at_cap 加 list_goals_reports_rows_with_repair_notes 加 race_guard_full_record_cas_same_second_set_detected 加 write_verify_bak_promote_and_drop_repair_paths 加 stop_hook_and_fastpath_repair_corrupt_store 加 concurrent_writers_lww_never_corrupt_never_spurious_error 加 wake_throttle_durable_before_prompt 加 yolo_heal_cooldown_durable_before_sequence 加 sweeper_heavy_cap_one_action_per_pass)加改件两件(corrupt_store_is_loud_not_silent 随读面自愈契约改形加 run_heal_end_to_end_with_fake_herdr 随 run_heal 签名改形;blocked_fastpath_alarms_and_debounces 未改,rev-codex F-3 勘误)加集成新增四件(goal_show_json_envelope_and_filter_output 加 goal_list_kv_json_jsonl_and_repair_rows 加 goal_crash_injection_kill9_store_stays_consistent 加 goal_concurrent_process_sets_both_succeed_lww);单测 226 升 236 集成 53 升 57 绿;rev-codex 三轮评审放行前收口(首修批 8b9d857 加二修批 bc25337,F 全清 G 采纳五裁定三,终轮 F-6 文档同步)
---

# REQ-048:goal原语地基批稳定性加固

## Scenario

舰队 goal 原语已定 = 全端 hst goal 互视互改(查看所有 agent 的 goal 加彼此可改),全员自适应升级协调等场景全骑其上,用户令必须非常稳定(goalhard-01,2026-10-12)。五件:并发原子性、heal 风暴闸复核、坏档自愈、机读面、三族回归。

## 设计

### 写原语四硬化 write_record

- 唯一 tmp 名(pid 加 getrandom token):本地与远端(ssh/mesh exec 落本机)并发写者互不吞 tmp,旧固定名形下一家 rename 吞他家的 tmp 致另一家假错,实证见并发锤测试。
- 写后验(真源写后验非空教训):rename 后回读解析加 text 非空,不通过响亮报错;主位缺失 = 并发 clear 已收,按 LWW 合法态放行。
- unix fsync(tmp 文件加 rename 后父目录,尽力而为;Windows 无对应原语跳过)。
- `.bak` 上次好档维护:主档可解析才刷(坏档不覆好 bak),clear 保留 bak(误清恢复源)。

### 并发语义

- 整档最后写胜(LWW):rename 原子性保证文件层语义,set/clear 并发收敛到单写者结果,测试断言终态 ∈ 写者值集合加读者永不见坏档。
- 增量路(Stop rounds/heal streak/blocked streak 读改写)写前整档 CAS 乐观守卫(write_record_guarded):任一字段变(含同秒 set 只换文本,set_at 秒粒度分不出的盲区)即弃写让位,增量不吞管理写;clear 后不复活。
- 读侧竞态收编:read_record 的 is_file 判真与读之间被并发 clear 删走按缺失收(非坏档)。

### 审计道

`.goal-audit`(ts 加 slug 加 action 加 detail):set/clear/repair 管理动作留痕,detail 带 host 加 user 加 pid 归属(远端落本机时进程身份即调用方代理身份,跨机聚合按各机审计自证)。heal 域动作仍在 `.heal-log` 不重记。两道共用的追加带上限轮转(`AUDIT_LOG_CAP_BYTES` 1MB,keep-one 滚 `.1`),单 write_all 含换行(unix O_APPEND 单写原子,多笔写会在行中拼进他行,实证)。

### 坏档自愈读面修复

- 坏档不再卡死:读面(show 加 stop_hook 加 heal 扫描加 blocked 快道加 list)遇 Corrupt 即隔离留证(`<slug>.json.corrupt-<ts>-<token>`)加 bak 有好档晋升回主档(写后验同形)或判缺省,动作落审计道。
- doctor 体检面保持只读纯读(坏档响亮报不修复),show_goal 契约不变。
- 崩溃残件唯一 tmp 超 `TMP_STALE_SECS`(1h)在扫描趟顺手清。

### heal 风暴闸耐久序复核修得

- Wake 路:节流先落盘(last_heal_at 加 streak)后发 prompt。旧形 prompt 先发写后落,写盘持续失败时节流永不持久,每趟 sweep 重唤即 prompt 风暴,风暴闸(只计成功写)拦不住。
- Blocked/yolo 治路:冷却先落盘后跑治序列。旧形治后落,写失败时冷却不持久,每 5 分钟重启格一次。
- sweeper 单趟治动作上帽一个(heavy_cap=Some(1),手动面不帽):单趟 heavy 与 goal 数无关地有界,多 goal 同 blocked 不叠破锁陈旧阈(240s)被接管致双 sweeper。
- 互改放大推演:每 goal 唤醒率 ≤ 1/15min 加连续 3 次无效转人工;yolo 治 ≤ 1/6h;报警防抖 1/15min;每机每驱动扫描 ≤ 1/5min(事件与 sweeper 各表)。互改 set 重置全部计数(新 goal 新周期),不放大唤醒面。
- 单档写盘失败折行(skip-write-fail)不弃整趟;并发 set 撞增量即 skip-raced 让位;回执行留痕判据单源(report_logworthy)。

### 机读面

- `hst goal show --json`:fmtio 信封(data = present/project/slug/file/goal 全字段/yolo_heal_armed/修复注记),kv 面补 goal.repair 行;jsonl 裸实体(无信封层,家族先例 issue show 与 artifact attest 同形;rev-codex G-4)。
- `hst goal list`(新面):全机在册清单(kv 逐档行加 json 信封 count/goals 加 jsonl 逐档对象),字典序稳定,坏档修复注记入行,goal 插件 list --all 逐机消费。

## Criteria

- [x] 并发写:双进程/四线程锤同键零假错、零坏档、终态 LWW、审计对账(单测 concurrent_writers 加集成 goal_concurrent_process_sets)
- [x] 原子写形:唯一 tmp 加写后验加 fsync 加 bak(单测 write_verify_bak_promote_and_drop_repair_paths)
- [x] 坏档自愈:隔离留证加 bak 晋升加判缺省加审计(单测三件加集成 repair 行)
- [x] 风暴闸:节流先落盘加冷却先落盘加上帽,写故障不引重试风暴(单测 wake_throttle 加 yolo_cooldown 加 heavy_cap)
- [x] 机读面:show --json 信封加 filter-output 加 list 三态(集成两件)
- [x] 崩溃注入:kill -9 实弹 40 发 store 恒一致(集成 goal_crash_injection)
- [x] 门禁:fmt 加 clippy 加 aidoc 重投影加 md 四门禁绿

## 追记

- 判断项 J1(新增 goal list 子命令):任务字面只要 show --json,但「查看所有 agent 的 goal」需要机器级清单面,否则插件 list --all 须自带项目发现;rev-codex 评审轮 CONFIRM。
- rev-codex 首轮回执(2026-10-12,A 至 G 主项全 CONFIRM):F-1 三件新单测补 cfg(unix) 门(Windows 宿主编译红,CI 不红低危)、F-2 AGENTS.md check.py 装位第二真相同步、F-3 trace 改件数勘误(blocked_fastpath 测试未改,实改件两件);G 级采纳:G-1 模块档措辞改整档快照守卫、G-2 heavy-deferred 让位补 best-effort 守卫写(dry 零副作用保持)、G-3 clear 同删 .bak(旧 bak 复活已清目标是真风险,误清恢复由审计道 clear 行 text 快照承载)、G-4 机读面四处形对齐(show jsonl 裸实体随家族先例、repaired 恒在键对象形、list meta.project 传 goal 存储目录)、G-5 崩溃件补文件级断言(零隔离件加主档裸字节可解析)。
- rev-codex 裁定保持项:G-6 手动 heal 不加 --force(治序列是重锤,人工强推可等窗或手改档,后续需求单另立);G-7 wake-fail 吃 heal_streak 预算(驱动链坏死即转人工,恢复面 = rounds 增长,语义入 run_heal 契约注);clear 并发收走按已清收(两机同清不假错);修复面 stop_hook/fastpath 读路上的窄窗风险(隔离 rename 搬走他方刚写好的新档)判可接受(自愈优先于窄窗停滞,隔离档留证可查)。
- 测试面三坑实录:std::env 全局可变面在并行测试串扰(fake herdr 路径烤进脚本免 env 依赖;PATH 存取用原值快照,replace 回写会清空 PATH 致下一测试的 cat 找不到);writeln! 两笔写在 O_APPEND 下行交错(单 write_all);panic 持锁毒化连带他测(PoisonError 连锁)。
