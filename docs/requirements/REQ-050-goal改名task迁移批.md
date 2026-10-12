---
id: REQ-050
title: goal改名task迁移批
status: implemented
priority: must
trace: task 单测新增两件(legacy_goal_dir_migrates_to_task_dir_on_access 迁移回放加 legacy_goal_dir_merge_when_task_dir_exists 双在场合并)加集成新增两件(task_command_family_and_goal_alias_compat 别名兼容加 task_store_migrates_from_legacy_goal_dir 进程级迁移回放)加全量随动改件(lib 与集成 goal 面 130 余处随正名);单测 232 集成 59 绿;装机落差门一件(verify_live 在装机位 hst 不支持 task 腿时 skip 带注,装机升级自活)
---

# REQ-050:goal改名task迁移批

## Scenario

用户令(2026-10-12 追单五件,2026-10-13「不叫 goal 了就是 task 任务」正名,--force 即刻开工):hst goal 改名 task,状态档与钩子腿随迁,2.12.0 一版带完。

## 设计

- 命令族:`hst task`(set/show/list/clear/heal/yolo-heal)正名主面;`hst goal` 兼容别名一版(clap subcommand alias,同档同语义);`hst hook task` 腿正名(`hook goal` 静默别名,stdout 零污染);kv 标记行 task.*(json 信封 data 键 task)。
- 状态档:`state/task` 新位;`task_dir()` 访问时 `state/goal` 整目录 rename 平移(读旧写新),双在场逐档合并(新位不动,旧位非冲突搬走,冲突留旧);治理件随迁;`.task-audit` 新写(`.goal-audit` 随迁留档);审计落 migrate 行。
- 钩子腿:shim 腿表 task(hst-task 四载体),退役清扫加 hst-goal 四件(带生成标记删);注册矩阵 handler 换 hst-task 指向,存量 hst-goal 注册经 merge 陈旧替换清出(is_ours 兼认 = 清出面);doctor check=task(判据 hst-task,存量 hst-goal 仍认);statusline 段 id 加 task(goal 兼容映射同一渲染),HOOK_ALIASES hst-task。
- 逃生闸:HST_TASK_HEAL/HST_TASK_SWEEPER 正名,旧 HST_GOAL_* 双名兼容一版。
- 模块随正名:src/goal.rs 改 src/task.rs,crate::goal 改 crate::task,主 API task 化(set_task/TaskRecord/task_dir/task_path/list_tasks 等);.heal-log 名不动(heal 域);提醒与 wake 文案用法段 hst task 化。
- s5 流:tasks/shared 承接(公告笔落),goals/shared 旧档只读留档。
- 版本:2.12.0(minor:命令族与档位与腿名 = 行为与能力变化),CHANGELOG 里程碑记;发布链归总台。

## Criteria

- [x] hst task 主命令族 + goal 一版兼容别名(集成件走通)
- [x] 状态档平移迁移(单测回放 + 集成进程级回放,读旧写新)
- [x] hst-task 腿更名 + init 重部署清出存量(退役清扫 + merge 替换,deploy/shim 测试随动)
- [x] tasks/shared 承接公告,goals/shared 只读留档
- [x] 2.12.0 定名(Cargo + CHANGELOG)
- [x] 全测四门禁 aidoc PEVO 绿

## 追记

- 装机落差门:verify 的活弹判据经 shim 烘焙回落装机位 hst,装机位版本不支持 hook task(2.11.0 前)时端到端必红,属环境事实;verify_live 集成件加装机位探针,旧版即 skip 带注,装机升级 2.12.0 后自活。
- 全局替换教训一笔:src 面 hst-goal 批量替换误伤退役清扫表(旧 hst-goal 四件被换成 hst-task = 部署自删新腿,测试当场捉出);替换后必须 grep 语义冲突位(同词异义:现行腿名 vs 退役腿名)。
