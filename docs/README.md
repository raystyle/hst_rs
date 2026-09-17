# docs 地图

> 全仓文档地图（dev-evo 文档即代码体系；ADR-0006 起旧文档层已删除出仓，git 历史即档案）。
>
> 红线（dev-evo 用户裁定 2026-09-16）：`diary/` 与 `research/` 是保留核心结构，不可裁撤。
> 双目录并存裁定：`guide/`（G 编号元规范，被 AGENTS 与 R002 等全仓引用的活档体系）与 `guides/`（dev-evo 任务导向指南与留档）语义不同故并存，不合并。

## 活跃体系

> dev-evo 文档即代码体系。

| 位置 | 讲什么 | 何时看 |
|---|---|---|
| `../AGENTS.md` | 五节合同（Commands/Must/Must not/Read first/环境） | 每轮开工前 |
| `adr/README.md` | 架构决策索引（ADR-NNNN，仍约束现状的决策择要） | 立不可逆选择前 |
| `requirements/README.md` | 需求登记索引（REQ-NNN，draft 到 implemented 带 trace） | 立需求或查验收时 |
| `guides/` | 任务指南（getting-started、旧四段协作规则留档、dev-evo 终态对照自评） | 做事前查方法 |
| `diary/` | YYYY-MM-DD 一天一篇过程与自省 | 查当天做了什么 |
| `research/` + README | SNNN 研究档案（六态标注） | 找为什么时 |
| `aidoc/` | Rust API 投影（生成物勿手改；llms.txt 入口加 hst_cli 分模块 md 加 api JSON） | 查公开项契约时 |
| `../CHANGELOG.md` 加 `../ROADMAP.md` | 版本成果与阶段 | 查历史与进度 |

## 历史体系

> 迁移留档，指针有效。

| 位置 | 讲什么 | 迁移去向 |
|---|---|---|
| `proven/` | P 编号方案归档（已完成 plan 全文） | 择要升 ADR（四件已转），全文留档 |
| `references/` | R 编号开发测试参考（R002 命令唯一权威仍活） | 活档保留 |
| `guide/` | G 编号元规范（G001/G002/G005 写作与六态仍活） | 活档保留 |
| `mistakes/` | M 编号错误档案（同根因聚合） | 新坑改走 ADR 或 exp 链，存量留档 |
| `web/` | 前端资源包输入区（D15 退役） | 历史资产 |
| 根 `PRD.md` | D 编号需求清单（53 行历史） | 新需求走 REQ；历史留档 |
| 根 `GOAL/PLAN/TODO` | 旧四原语（队列已转 REQ-002 至 005） | 历史留档 |

## 迁移映射

> dev-evo base-init 存量迁移口径。

PRD 条目对应 REQ；PLAN/TODO 对应 REQ 的 Criteria 与 trace；GOAL 定位句并入 AGENTS 头部；INDEX 职责由本地图加 AGENTS Read first 承接；proven 语义由 implemented REQ 加关联 ADR 承接；mistakes 并入 ADR 或 exp 沉淀链。迁移不是搬运是重审：历史不回填、活档不搬家、断链必回归。

### 存量禁字债口径

PE-11 历史档案豁免走 `PEVO_CHECK_ALLOW` 机制（分号分隔正则，匹配 `docs/` 下 `相对路径:行`，命中报 SKIP 带处数；根三件 AGENTS/README/CHANGELOG 永不受益）。本仓标准命令：

```bash
PEVO_CHECK_ALLOW='^docs/aidoc/;^docs/diary/2026-08-31-;^docs/diary/2026-09-01-;^docs/diary/2026-09-02-;^docs/diary/2026-09-03-;^docs/proven/'   uv run /mnt/wsl/repos/project-evo/plugins/evo-adr/skills/code-kit/scripts/check.py .
```

覆盖历史档案面（diary 四篇加 proven 存量整目录；存量清偿后逐步收缩正则）加 aidoc 投影渲染格式（条目分隔符 em dash 是 cargo-aidoc 输出格式无开关，漂移真门禁是 cargo aidoc --check --strict，沿 tool-rust 豁免实务）；活跃面（AGENTS、adr、requirements、guides、research、根 README 与地图）零容忍，新文件违规即修。
