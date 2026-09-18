# Requirements 索引

> 需求登记:新需求先立 REQ 再实现,实现后回填 trace(测试或验收命令)。新建拷 0000-template.md,编号接当前最大号。状态 draft 到 implemented 到 rejected。历史 D 编号需求清单已随 ADR-0006 清退出仓（git 历史可考）。

| id | 状态 | 优先级 | 标题 | trace |
|---|---|---|---|---|
| REQ-001 | implemented | must | 文档体系迁移dev-evo | check.py 全项 |
| REQ-002 | implemented | should | 资产名与关键词取参纯函数重构 | asset_name 加 asset_keywords 加 pick_asset_for 全平台表断言 |
| REQ-003 | implemented | should | self-update读序不回环集成断言 | self_update 假基址断言加守卫三态单测 |
| REQ-004 | implemented | should | grok运行时退单排pwsh闸门断言 | default_layout pwsh 闸门行为测试 |
| REQ-005 | rejected | should | 门禁脚本退出码对齐CI | 三环境退出码实证（场景不复现，M033 同型） |
| REQ-006 | implemented | must | aidoc投影强制重构 | aidoc --check --strict |
| REQ-007 | implemented | should | 全平台SSH实测矩阵 | .tools/cross-test.sh |
| REQ-008 | draft | should | 三扫描器缺省根统一仓根 | |
| REQ-009 | implemented | must | 项目级yolo干扰一键清除 | clear_project_yolo_interference 单测加 cli 集成 init --clear-project-yolo |
| REQ-010 | implemented | must | 统一issue入口集成 | hst issue new/list/show 实弹（issues.ohmygh.com，契约 = 总台 REQ-057） |
| REQ-011 | draft | should | 输出协议可选件评估 | 待回填 |
| REQ-012 | draft | should | 库面残破描述清算 | 待回填 |
| REQ-013 | implemented | must | 自更新家族统一标准对齐 | update.rs 单测六件加 cli 集成 update_version_output_parses 加实弹缺省镜像腿 |
