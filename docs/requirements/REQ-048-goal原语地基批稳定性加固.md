---
id: REQ-048
title: goal原语地基批稳定性加固
status: implemented
priority: must
trace: goal 单测新增十件(goal_files_filter_and_stale_tmp_swept 加 heal_and_goal_audit_logs_rotate_at_cap 加 list_goals_reports_rows_with_repair_notes 加 race_guard_full_record_cas_same_second_set_detected 加 write_verify_bak_promote_and_drop_repair_paths 加 stop_hook_and_fastpath_repair_corrupt_store 加 concurrent_writers_lww_never_corrupt_never_spurious_error 加 wake_throttle_durable_before_prompt 加 yolo_heal_cooldown_durable_before_sequence 加 sweeper_heavy_cap_one_action_per_pass)加改件三件(corrupt_store_is_loud_not_silent 随读面自愈契约改形加 blocked_fastpath_alarms_and_debounces 随耐久序改形加 run_heal_end_to_end 随签名改形)加集成新增四件(goal_show_json_envelope_and_filter_output 加 goal_list_kv_json_jsonl_and_repair_rows 加 goal_crash_injection_kill9_store_stays_consistent 加 goal_concurrent_process_sets_both_succeed_lww);单测 226 升 236 集成 53 升 57 绿
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

- `hst goal show --json`:fmtio 信封(data = present/project/slug/file/goal 全字段/yolo_heal_armed/修复注记),kv 面补 goal.repair 行;jsonl 单实体视同信封。
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

- 判断项 J1(新增 goal list 子命令):任务字面只要 show --json,但「查看所有 agent 的 goal」需要机器级清单面,否则插件 list --all 须自带项目发现;评审轮核。
- 测试面三坑实录:std::env 全局可变面在并行测试串扰(fake herdr 路径烤进脚本免 env 依赖;PATH 存取用原值快照,replace 回写会清空 PATH 致下一测试的 cat 找不到);writeln! 两笔写在 O_APPEND 下行交错(单 write_all);panic 持锁毒化连带他测(PoisonError 连锁)。
