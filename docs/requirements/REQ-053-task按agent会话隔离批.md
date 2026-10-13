---
id: REQ-053
title: task按agent会话隔离批
status: implemented
priority: must
trace: 实现批 lane/token-unhook 随 2.14。单测新增 task_agent_isolation_and_legacy_migration(隔离加互清不串加 legacy 兼容读加首写迁移加键名拆解);随动 = 键路径期望与 heal 扫描 slug 剥尾与 list 行 agent 列与 kv/json 面;verify 判据档落 legacy 键位(装位 shim 恒烘焙装位 exe,旧版 hook 无 agent 维不识新键,legacy 两态都认)装机落差窗活弹 rounds=2 绿;单测 243 集成 63 绿门禁全绿
---

# REQ-053:task按agent会话隔离批

## Scenario

总台令(2026-10-13,bug 报):同一工作台目录设 task 会串台影响不同 agent(D:\pve-harness 上 claude code 设了 task,grok 的状态栏也显示受影响);追加令「task应该区分不同agent的会话」。REQ-045 期设计为项目级四家同源显示,本单改为按 agent 隔离(同项目各 agent 各自独立 task 键)。

## 设计

- **键形**:存储档 `<项目 slug>--<agent>.json`(task_dir 下);legacy 无后缀 `<slug>.json` 为 claude 兼容位(Stop 腿仅 claude,历史 rounds 皆 claude 产),claude 首写趟 rename 迁移到 claude 键,他家只认自家键。
- **就近上溯**:task_file_for 带 agent 维走原上溯逻辑(子目录 cwd 容形保持);claude 查找兼容 legacy 位。
- **面随动**:CLI task set/show/clear 增 `--agent`(缺省 claude);Stop 腿与 blocked 快道按注册 agent 键写(hook task 腿仅 claude 注册);heal 唤醒只投 rec.agent 对应格位;状态栏 task 段按渲染 agent 取键(claude 的 task 不再上 grok 栏);kimi 紧凑形同理。
- **机读面**:TaskRecord 增 agent 字段(serde default claude,legacy 档读侧归 claude);list 行 slug 键含 agent 后缀。

## Criteria

- [x] 键隔离:同项目 claude 与 grok 各自 task 互不可见(隔离回归件)
- [x] legacy 迁移:无后缀档 claude 首写 rename 到 claude 键,审计记 migrate 行
- [x] CLI --agent 三态与 Stop 腿 agent 键写(腿恒 claude 注册)
- [x] 状态栏按渲染 agent 取键(heal 格位匹配按剥尾项目 slug,pane 匹配面不变)
- [x] 全测门禁绿,随 2.14.0 发布

实现追记(2026-10-13):装机落差窗实证一笔,装位 shim 恒烘焙装位 exe(REQ-043 律),2.13 及此前装位 hook 无 agent 维不识新键,verify 判据档若落新键则活弹 rounds=0;修为落 legacy 键位两态都认,活弹复绿 rounds=2。滚版后装位 2.14 全链新键自洽。
