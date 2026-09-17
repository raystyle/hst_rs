# docs 地图

> 文档即代码（ADR-0006/D56）：契约在代码（clap help 加 `///` 加 tests），投影双面（`hst --llms` CLI 面加 `docs/aidoc/` 库面，CI 漂移门禁），why 进 ADR，需求进 REQ，过程进 diary，证据进 research。老文档层（references 的 R 系、guide 的 G 系、mistakes 的 M 系、proven 的 P 系、根目录 INDEX/PRD/GOAL/TODO/PLAN）已删除出仓，git 历史即档案。

## 目录

| 目录 | 讲什么 | 何时看 |
| --- | --- | --- |
| `adr/` | 架构决策记录（Context/Decision/Consequences 加索引表） | 改对应决策时 |
| `requirements/` | REQ 需求登记（draft 到 implemented，trace 回填） | 立需求与查验收时 |
| `guides/` | 任务导向指南（环境事实等；叙述性可手写，不作契约权威） | 做事查步骤时 |
| `diary/` | 项目日记，一天一篇（裁定、踩坑、门禁实录） | 查过程与教训时 |
| `research/` | S 编号研究档案（六态标注的证据件） | 查动机与实证时 |
| `aidoc/` | Rust API 投影（生成物勿手改；llms.txt 入口加 hst_cli 分模块加 api JSON） | 查公开项契约时 |

## 写作规范（一句话版）

- 六态标记：`[实证: 依据]`（已验证）、`[推断]`（逻辑推出）、`[经验]`（历史惯例）、`[记忆]`（建议复核）、`[假设]`（待验证）、`[直觉]`（无据倾向）；关键结论必标，禁止把没验证写成已验证。
- 四类禁字：emoji、破折号、Unicode 箭头、非法全角；真门禁是 `.tools/mdcharlint.py` 与 `rumdl check`，豁免区外零容忍。

## 骨架合规自检

```bash
PEVO_CHECK_ALLOW='^docs/aidoc/;^docs/diary/2026-08-31-;^docs/diary/2026-09-01-;^docs/diary/2026-09-02-;^docs/diary/2026-09-03-' uv run /mnt/wsl/repos/project-evo/plugins/evo-adr/skills/code-kit/scripts/check.py .
```

豁免语义：aidoc 是生成物渲染格式；diary 四篇为存量禁字债（过程档不改写，PE-11 历史豁免通道）。
