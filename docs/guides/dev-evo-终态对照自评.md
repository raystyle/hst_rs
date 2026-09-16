# dev-evo 终态对照自评

> 2026-09-16 对 ProjectEvo wH 终态讨论（第六十一批）的三件回执存档：17 篇逐面自评、agent CLI 面对照、退役三篇引用排查（全仓零引用，实证见回执）。

## 十七篇逐面自评

| 参考 | 判定 | 依据 |
| --- | --- | --- |
| base-init | 已落 | REQ-001 存量迁移（五节合同、ADR/REQ、地图承接 INDEX） |
| base-agents-contract | 已落 | AGENTS 五节 44 行；旧四段留档 guides |
| base-adr | 已落 | ADR-0001 至 0004（supersede 链含 P0004 指针） |
| base-req | 已落 | REQ-001 至 007（implemented 带 trace） |
| base-projection | 已落 | aidoc 23 artifact 强制（ADR-0006 口径）加无自有 API 面范式引用加 regenerate-and-diff 测试钉（init_rerun_is_byte_idempotent） |
| flow-testing | 已落 | 185 单测加 28 集成；R004 分层与断言纪律在册 |
| tool-cli-agents | 部分 | 见下节 agent CLI 面对照（差距三项全裁定不适用或已有承载） |
| tool-rust | 已落 | missing_docs deny 加 aidoc --check --strict 入 Commands 加契约注释格式纪律 337 处 |
| tool-typescript | 不适用 | Rust 仓 |
| tool-python | 不适用 | Rust 仓 |
| flow-archive | 已落 | diary 一天一篇加 research SNNN 索引（35 件登记）加沉淀升 ADR 链 |
| base-writing-standards | 已落 | G001/G005 与六态同源（零翻译成本）；四件套加 check.py 门禁 |
| flow-release | 已落 | v1.1.5 与 v1.2.0 两封版实跑（CI 六岗、digest 三方对账、镜像复验、舰队收敛） |
| exp-pitfalls | 已落 | mistakes M 体系在册（M064 长行锚聚合）；同型二犯升格链在 AGENTS Must |
| env-platform | 已落 | 三平台 CI 矩阵加 WSL/Windows 双侧开发加 cross-test.sh 实弹矩阵（REQ-007） |
| exp-sedimentation | 已落 | AGENTS Must 沉淀条款（旧 G004 对应）；proven 择要升 ADR 四件实证 |
| tool-project | 已落 | .tools 归档带 README 清单（cross-test.sh 随批登记） |

## agent CLI 面对照（tool-cli-agents 第十一节）

| 面 | 判定 | 依据 |
| --- | --- | --- |
| 发现通道 skills 自生成 | 已落 | `hst skill` 从 clap 活命令树派生 SKILL.md（D22，同源派生思想） |
| 发现通道 mcp add | 不适用 | D15 编排面移除（ADR-0001）；hst 是部署配置工具非数据 CLI |
| 发现通道 --llms 清单 | 已承载 | aidoc llms.txt 入口索引（23 artifact 之一，ADR-0006 强制面） |
| 输出信封 {ok,data,meta} 同构 | 已落 | `--format kv/json/jsonl` 加 stderr 单行 JSON 错误（R011；第十一节原文引 hst 为跨栈同构样板） |
| CTA | 已落 | doctor warn 行自带处置提示是产品惯例（hooks.form、yolo.parse 等） |
| token 计量与分页 | 不适用（裁定） | 输出面是配置诊断 marker 行无大输出截断场景；trace 六视图自有 --limit/--offset 分页已落 |
| 输出过滤 --filter-output | 不适用（裁定） | kv marker 行单键取值；trace 自有 --agent/--file/--glob 过滤已落 |
| agent 探测输出策略 | 部分（裁现有形态足够） | kv 与 json 双态已分叉 human 与 agent 面；TTY 探测分叉无实弹需求暂不立项 |

## 退役三篇引用排查

env-environment 与 flow-events 与 tool-selection 三篇全仓零引用（rg 排除 diary 加 proven 加 research 加 aidoc 历史档后空），无需清理。
