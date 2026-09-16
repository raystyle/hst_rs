# 2026-09-16：skill面退役与llms手册面

> 用户裁定 D54：有 llms 投影即不需要 skill 面。本篇记裁定、执行与验证。

## 流水

1. **裁定与立档**：用户令「有 --llms 就不需要 skill 参数命令」「不再有 hst SKILL 安装」。事实边界先核清：aidoc llms.txt 是库面投影只在本仓、skill 是仓外装机运行时发现面，两层不同构；用户知悉后裁定全删。ADR-0005 立档（c4cba3f）：删 `hst skill` 子命令与 init 项目级 fan-out、不再有任何技能安装面、agent 紧凑说明书以 `hst --llms` 出口（活命令树自适应渲染直打 stdout，不落盘）。
2. **代码面**：skillgen.rs 整删；main.rs 删 Skill 变体与 cmd_skill、retire_user_skill 移入 deploy；deploy.rs 删 COMMAND_MAP 加 skill_md 加 write_skill 加 deploy_kimi_project，deploy_skills 改造为 retire_skills（四目录乘 hst 与 ohmyagents 两名，marker 家族判 ours）加 retire_user_skills（用户级 `~/.claude/skills/` 两名，生成签名「活命令树自适应生成」判 ours），外科式伴生资源保留；`--llms` 旗标加 render_llms/walk/synopsis（复用旧 skillgen 遍历器，输出 llms 风格功能面加命令表加输出契约）。契约破裂升 2.0.0。
3. **文档同步**：R002 skill 行换 `hst --llms` 行、init 行与 yolo 行摘 skill 部署措辞、同步链缩两处（R002 行加 docs README；无生成物需重生）；AGENTS Must 两句改（四处同步缩两处、手改生成物清单换状态栏脚本加 aidoc 面）；dev-evo 对照表发现通道三行改口径（skills 自生成已退役、--llms 双面承载）；ADR-0005 并入 --llms 决策。
4. **验证**：183 单测加 31 集成绿（含新增 llms_flag_prints_compact_manual 与 init 清扫断言翻转）`[实证: 2026-09-16 cargo test --locked]`；aidoc 重生成 22 artifact（skillgen 模块消失）strict 退出码 0 `[实证: 2026-09-16 cargo aidoc --check --strict]`；实弹 `hst --llms` 出手册、`hst skill` 报 unrecognized subcommand、dogfood init 清扫本仓 `.agents/skills/hst` 与用户级 `~/.claude/skills/hst` 双 `(retired)` `[实证: 2026-09-16 ./target/debug/hst init]`；doctor blocked=true 系既有环境项（codex 加 kimi 加 grok 项目信任未种、kimi 登录墓碑、grok token 过期），无本次改动引入的新发现 `[实证: 2026-09-16 ./target/debug/hst doctor]`；md 四门禁加 PEVO 合规（PE-11 活跃面无禁字）。
5. **顺手清债**：当日早前 diary 三处破折号禁字（item 18/19/20）改冒号，PE-11 复绿；独立小笔提交。
6. **封版评审闸门（herdr 右侧 codex，快核转全量）**：回执 F 四加 G 五。F1 用户级清扫漏三家（`~/.agents`、`~/.grok`、`~/.kimi-code` 的 skills 还挂 home-root init 落的 marker 件，且判据分叉：签名判据收不了 marker 件），修法：`skill_md_is_ours` 统一判据（marker 家族或旧静态版或生成签名），用户级清扫面扩为四家 skills 根乘两名，测试补 marker 加 `.agents` 用例，实弹 `hst init` 清三残件 `[实证: 2026-09-16 ./target/debug/hst init 三行 (retired) 后四根全空]`；F2 aidoc 孤儿 `hst_cli/skillgen.md` tracked 残件 git rm（`--check` 只比内容不报孤儿，CI 抓不到的盲区）；F3 根 README 三处 skill 现役措辞；F4 R001 两处加 R002 D29 注记补退役括注。G1 自适应口径精确化（命令表自适应，功能面五条与输出契约是手写概览段）；G2 synopsis 裸旗标取值语义渲染（`[--yolo[=full|partial|off]]`，有枚举值列值集，与 R002 对齐）；G3 INDEX 三行刷新（迁移期留档但代码表自称有效）；G4 legacy 四段指南头部加归档注记；G5 跨仓路径债换新位（`/mnt/wsl/repos/project-evo/plugins/evo-adr/skills/code-kit/scripts/check.py`，旧 D 盘位已裁撤）。门禁复跑：183 加 31 绿、aidoc 22 artifact strict 0、md 四门禁加 PEVO 合规、rumdl MD056 一处（R002 表行裸管道转义）。存量非本批：clippy 四条 dead-code 加 deploy 两条 redundant closure 在基线 d933f85 已在，未扩面。

## 自省

- 「有 X 就不需要 Y」的替代式裁定，先核 X 与 Y 是否同层再动手：本轮 llms.txt 与 skill 受众、分发、版本锚三层都不同，靠先摆事实边界让裁定落在知情的面上，ADR 里把放弃的仓外发现面写成 Consequences 而不是埋掉。
- 退役不是删代码完：在位装机件要有清扫路径（ours 判据加外科式），否则留僵尸生成物；本轮复用 D49 的 marker 家族与生成签名判据，零新判据面。
- 评审闸门真抓到东西：F1（清扫面漏三家加判据分叉）与 F2（孤儿 tracked 生成物，门禁盲区）都是我自验全绿后仍漏的面；「已验面不转嫁复验」写清边界反而让评审方专攻增量，一轮回执的命中率比自查高。
