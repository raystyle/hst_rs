---
id: REQ-054
title: goal别名摘除与Stop收程修批
status: implemented
priority: must
trace: 实现批随 2.15.0。goal 别名族退役测试改形(task_command_family_and_goal_alias_retired:主族在加 goal 命令族与 hook goal failure 形加 HST_GOAL_* 旧闸名随 task 正名);Stop 收程修 = cmd_hook_task 决策文档 flush 后 exit(0)(rounds 先落盘语义不变,尾程扫描骑他事件与 sweeper);条形五块断言锁(bar_cells 42/0/100 三形);单测 244 集成 62 绿门禁全绿
---

# REQ-054:goal别名摘除与Stop收程修批

## Scenario

总台令(2026-10-13 连令):①「去掉」= goal 别名族摘除(REQ-050 一版兼容期满,原定 2.13 两顺延至本版);② Stop hook 报错修(Claude Code 报「hook output parsed as a document that grants…stdio went quiet before end-of-stream…refusing to honor the suspect capture」);③ 状态栏 context 条形改五块(「▰▰▰▱▱ 五块」)。定版 2.15.0。

## 设计

- **goal 别名族退役**:`hst goal` 命令组 clap alias 与 `hst hook goal` 静默别名与 `HST_GOAL_HEAL`/`HST_GOAL_SWEEPER` 双名闸全摘(`HST_TASK_*` 唯一);不属别名族不动 = 状态栏段 id goal 映射(用户 toml 面)与 state/goal 存量迁移道与 REQ-053 legacy 无尾键兼容读。
- **Stop 收程修**:根因 = 决策文档(Stop 提醒 JSON)打印 flush 后,cmd_hook_task 尾程同步跑 heal_sweep(herdr 唤醒走网络,秒到数十秒),撞 hook 注册 10s 超时被杀,stdout 无 EOF,Claude Code 判疑似截获拒收文档报错;修法 = rounds 落盘在前(既有序),文档 println 加 flush 后即刻 std::process::exit(0),扫描与 fleet 刷新骑后续事件与 sweeper 自驱(ensure_sweeper/ensure_fleet_refresh 均独立进程组不连坐)。
- **条形五块**:bar_cells 渲染位 4 升 5(42% 出 ▰▰▰▱▱,ceil(pct*5/100))。

## Criteria

- [x] goal 命令族与 hook goal 与 HST_GOAL_* failure 形(退役测试改形)
- [x] Stop 决策文档即出即退,rounds 先落盘语义不变
- [x] 条形五块三形断言锁
- [x] 2.15.0 定名,门禁全绿
