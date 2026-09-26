# Requirements 索引

> 需求登记:新需求先立 REQ 再实现,实现后回填 trace(测试或验收命令)。新建拷 0000-template.md,编号接当前最大号。状态 draft 到 implemented 到 rejected。历史 D 编号需求清单已随 ADR-0006 清退出仓（git 历史可考）。

| id | 状态 | 优先级 | 标题 | trace |
|---|---|---|---|---|
| REQ-001 | implemented | must | 文档体系迁移dev-evo | check.py 全项 |
| REQ-002 | implemented | should | 资产名与关键词取参纯函数重构 | asset_name 加 asset_keywords 加 pick_asset_for 全平台表断言 |
| REQ-003 | implemented | should | self-update读序不回环集成断言 | self_update 假基址断言加 MirrorPlan::base 三态单测（守卫随 ADR-0008 退役） |
| REQ-004 | implemented | should | grok运行时退单排pwsh闸门断言 | default_layout pwsh 闸门行为测试 |
| REQ-005 | rejected | should | 门禁脚本退出码对齐CI | 三环境退出码实证（场景不复现，M033 同型） |
| REQ-006 | implemented | must | aidoc投影强制重构 | aidoc --check --strict |
| REQ-007 | implemented | should | 全平台SSH实测矩阵 | .tools/cross-test.sh |
| REQ-008 | draft | should | 三扫描器缺省根统一仓根 | |
| REQ-009 | implemented | must | 项目级yolo干扰一键清除 | clear_project_yolo_interference 单测加 cli 集成 init --clear-project-yolo |
| REQ-010 | implemented | must | 统一issue入口集成 | hst issue new/list/show 实弹加 #52 同型修加 #53 keyset 翻页（原 issues.ohmygh.com 面已切 ledger，见 REQ-010 附注与 REQ-018） |
| REQ-011 | draft | should | 输出协议可选件评估 | 待回填 |
| REQ-012 | draft | should | 库面残破描述清算 | 待回填 |
| REQ-013 | implemented | must | 自更新家族统一标准对齐 | update.rs 单测九件（八新加一存量改）加 cli 集成 update_version_output_parses 加实弹缺省镜像腿与降级收束 |
| REQ-014 | implemented | must | hook注册自愈哨兵 | statusline.rs 单测四件加本机断注册实弹自愈闭环两轮 |
| REQ-015 | implemented | should | hook注册形态迁移评估 | 探针插件无头双钩实发加 evo-adr 活例加不入册三证 |
| REQ-016 | draft | must | claude注册迁移skills插件形 | 待实施后回填 |
| REQ-017 | implemented | should | 项目级yolo干扰检测告警 | yolo.rs 检测单测加 cli 集成加 statusline 单测两件加实弹 |
| REQ-018 | implemented | must | ledger账本集成 | ledger.rs 单测六件加 cli 集成三件加实弹三验 |
| REQ-019 | implemented | must | 状态栏loop与goal段加管理命令面 | loopmgmt 单测五件加 statusline 单测四件加门控扩加 cli 集成三件加实弹（loop 与 goal 回环加部署脚本直跑） |
| REQ-020 | implemented | should | doctor增loop与goal健康检查 | loopmgmt 单测三件加 doctor 单测三态三件 |
| REQ-021 | implemented | should | loopmgmt类型契约化试点 | loopmgmt 单测十一件（含评审回填五面）加 cli 集成五断言面（双缺加双给加 bad_every 加 corrupt 加 no_session 短码） |
| REQ-022 | implemented | should | 类型契约化推广裁定与compact收尾 | compact 单测三件加推广面实测盘点 |
| REQ-023 | implemented | must | herdr通道loop即时生效面 | herdrrpc 加 arm 单测加假服务器集成加真 socket ping 实弹（ADR-0009） |
| REQ-024 | implemented | must | 状态栏第三行loop与goal专属行 | statusline 单测四件改加加实弹三行渲染（goal 截断放宽 60） |
| REQ-025 | implemented | must | 状态栏第四行goalmode专属行 | goalmode 四态单测加实弹（105MB 真机 transcript 倒序分块扫描） |
| REQ-026 | implemented | must | 状态栏渲染原生化与hookstate专属行 | statusrender 单测三件加 statusline plumbing 加实弹（第5行渲染、0.217s 对 0.95s、BYTE-IDENTICAL diff 空；ADR-0010） |
