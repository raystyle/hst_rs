---
id: REQ-026
title: 状态栏渲染原生化与hookstate专属行
status: implemented
priority: must
trace: statusrender 单测十件（cron 节拍、标记扫描两形加严格锚拒中、fmt、六工具链版本提取、context 双口径、duration 阈值加 model 回落、会话闸全候选、mcp 双回落、图标覆盖）加 statusline plumbing（segments5 键解析加 effective_orders 五元组加 example 锁）加实弹（本会话第5行 hook working 渲染、release 0.19 至 0.22s 对 pwsh 1s 档、真 payload 双引擎对版逐字相同；口径 = pwsh 强制 Ansi 形，管道缺省形态旧载体本就无色）加评审一轮 F1 至 F10 吸收
---

# REQ-026:状态栏渲染原生化与hookstate专属行

## Scenario

用户令两条（2026-09-26）：「全面回归 pwsh脚本改成hst二进制自己实现 hook执行指向 hst 命令 进行输入输出」（git 历史先例：hook 面曾指向本项目自生二进制的裸命令形）；「第5行显示hook状态」。动因：pwsh 冷启动约 300ms 加 goalmode 探针（105MB transcript）合计约 0.95s/次渲染；PS1 与 Rust 双语言维护（版本探测、loop 探针两侧重复）；goalmode 倒序分块扫描在 PowerShell 踩大小写同变量等坑族。决策记录 ADR-0010。

## Criteria

验收判据,可检验、可勾选:

- [x] 新子命令 `hst statusline --render <agent>`：stdin 喂 agent JSON、stdout 出状态行，契约与 pwsh 脚本逐字对齐（clap 注释加 conflicts_with_all 守卫）
- [x] 原生渲染引擎 src/statusrender.rs：全段实现（shell 祖先链、dir、git porcelain、clock、package 加项目判型、hst、model、context、duration、loop、goal、goalmode 倒序分块扫描、mcp、tokens、六工具链段）；单源复用 StatuslineConfig 加 effective_orders 加 default_template 加 default_icon 加 loopmgmt 探针语义；模板定制与 grok ASCII 面兼容
- [x] hook 状态读序单源化 hook_state()（D28 全序）：HST_STATE_FILE 覆盖加用户级会话键加用户级 agent 键（会话闸）加项目级（会话闸），hst 段与 hookstate 段共享；态到色映射 state_color 同 pwsh 表
- [x] 部署接线：claude（settings.json statusLine）加 kimi（tui.toml）加 grok（config.toml，Unix 面）指部署方 hst 绝对路径加 `statusline --render <agent>`（PATH 上旧版无该面故用绝对路径；stable 全体带面后可回 hook 先例裸 PATH 形）；codex 维持内置 ID 面；pwsh 脚本弃用期保留一代供回退
- [x] 第5行 hookstate 专属行：statusline.toml 新键 `segments5`、`DEFAULT_SEGMENTS5 = ["hookstate"]`、模板 `{icon}hook {state}` 加 ascii 变体、图标 U+F0F1、状态色同 hst 段四色；状态文件全缺整行隐藏（零噪声）；pwsh 侧该段空块（弃用期不出，行隐）
- [x] 老配置升级语义不破：未写 segments5 默认补 hookstate；写过任一后续行键的配置补默认并去重
- [x] 性能：release 渲染 0.217s 对 pwsh 0.95s（本机同 payload 实测）；无 pwsh 冷启动
- [x] 测试：statusrender 单测十件（cron_to_cadence 六形、scan_markers 全链加短形加严格锚拒中四形、fmt_tok 加 fmt_dur、六工具链版本提取、context 双口径、duration 阈值加 model 回落、会话闸全候选、mcp 双回落、图标覆盖）；statusline 集成 example 锁含 segments5；实弹本会话第5行渲染
- [x] 已知边界：tools 段（显式选用面）首版渲染空（transcript 尾 500 行计数未移植）；hst 段 REQ-014 no-hook! 哨兵与 REQ-017 proj-yolo! 哨兵未移植（unknown 直显，回填排 REQ-027 draft）；版本本地探测缓存面（D46）未移植（payload version 字段归一承载，评审 F7 措辞已对齐）；mcp 对象形 mcp_servers 原生按真实键数、pwsh PSCustomObject 管道坑计 1（原生形更正确，口径差记档）；goal 文本截断原生按字符数保代理对完整、pwsh 按 UTF-16 单元可裁星面字符（原生形更正确，记档）；now_hm 经 date 子进程（Unix 面毫秒级可接受；Windows 无 date 命令 clock 段空，后续批补时区面）；Windows grok 仍走 .cmd 壳调 pwsh 脚本（弃用期删 pwsh 前须迁移该壳，评审 G4）；五端实弹矩阵待 lan-ubuntu 在线后补三查；claude 与 kimi 两端 ANSI 显色实弹证据候补（评审 F1 验收项，grok S025 已实证）
- [x] 评审一轮（hst-codex-review，F1 至 F10 全修加 G2/G5/G6/G7 吸收）：F1 ANSI 面（对版口径 = pwsh 强制 Ansi 形；原生加 NO_COLOR 与 HST_STATUSLINE_NO_ANSI 剥色开关，ADR 追记）；F2 会话闸改全候选闸（序号免闸在会话键缺位时泄漏跨会话态，本机复现实锤）；F3 icon_of 补 [icons] 键级覆盖；F4 duration 阈值门（不低于 1000ms 才出段）；F5 context 向下取整加 remaining 回退（token 由取整后百分比反推）；F6 model.id 回落；F7 版本探测措辞对齐（ADR 改文）；F8 工具链版本提取重写（撤 regex_lite 字面前缀法，go 逐出现位扫描、zig 串首锚、cpp 含点串，三型恒不出缺陷根除）；F9 mcp 双回落（用户级加项目级合计，项目级无条件叠加同 pwsh）；F10 goalmode 严格锚（全链形闭合链加尾缀、短形 is still active 尾缀，头对链不全与链外引文不中）；G2 macOS ps -ax 单次建链；G5 root 探测按消费者门控；G6 哨兵面排 REQ-027；G7 单测补八件（版本提取、严格锚拒中、context 双口径、duration 阈值、model 回落、会话闸全候选、mcp 回落、图标覆盖）；G8 边界并入上条。全测 250 加 49 绿
- [x] 评审二轮（hst-codex-review 复核：旧 F1 至 F10 全 CONFIRM，双引擎对版 harness 26 例两形态 19 例全同、余 7 例差异全落已记档口径与原生独占面）：F-new1 node 段补 ts 子段（就近四层 node_modules/typescript/package.json 读版本，两 part 各自包 ANSI 拼回逐字等形）；F-new2 cpp 色码对齐 38;5;110；F-new3 tokens 色码对齐 38;5;117；F-new4 包版本探针补 pyproject version 行与 build.zig.zon 的 .version 两源（build.zig 在场即判 zig，zon 缺位不跳判型）；G1 计数措辞改齐；G2 NO_COLOR 非空判定拒改（no-color.org 标准原文「present and not an empty string」，非空判定正是标准形）；G3 未知段 id 渲染期响亮报错（对齐 pwsh 部署期硬错，配置 typo 不静默丢段）。全测 250 加 49 绿
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
