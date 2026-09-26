---
id: REQ-026
title: 状态栏渲染原生化与hookstate专属行
status: implemented
priority: must
trace: statusrender 单测三件（cron 节拍形、标记扫描两形加引文不触发、fmt 形）加 statusline plumbing（segments5 键解析加 effective_orders 五元组加 example 锁）加实弹（本会话第5行 hook working 渲染、release 0.217s 对 pwsh 0.95s、真 payload 双引擎 BYTE-IDENTICAL diff 空）
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
- [x] 测试：statusrender 单测三件（cron_to_cadence 六形、scan_markers 全链加短形加深层再编码引文不触发、fmt_tok 加 fmt_dur）；statusline 集成 example 锁含 segments5；实弹本会话第5行渲染
- [x] 已知边界：tools 段（显式选用面）首版渲染空（transcript 尾 500 行计数未移植）；hst 段 REQ-014 no-hook! 哨兵与 REQ-017 proj-yolo! 哨兵未移植（unknown 直显，哨兵面后续批回填）；now_hm 经 date 子进程（毫秒级可接受）；五端实弹矩阵待 lan-ubuntu 在线后补三查
- [x] 实现后回填 frontmatter 的 trace，状态改 implemented
