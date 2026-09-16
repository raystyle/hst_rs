# Changelog

本文件只记录**大版本里程碑**：定位变更、发布、阶段完成、核心能力整体落地。细碎条目由 `docs\diary\YYYY-MM-DD-*.md` 与 git 历史承载。

## [Unreleased]

### 里程碑

> 2026-08-29 至 2026-08-31：从空仓库到 Windows 全量可用。方案 P0001 至 P0019，过程与经验在各 `docs\proven\` 文档。

- **项目定位**（P0004）：Oh My Agents，通用智能体多路复用任务编排器。首期切面见 P0001。
- **部件 POC 全绿**（P0005）：Windows 范围十四件 example（端点、会话、布局、驱动、对话框、粘贴、定位、流、状态、部署、负例、yolo 诊断、label 桥、备屏）。
- **产品命令闭环**（P0006 至 P0010）：spawn/status/send/cleanup 全链路、send 多行三段式粘贴（中文验收）、oma run 状态门分派、真四路拉通（claude 路全通）、settle 自愈信任（codex hash 复现与白名单点框互兜）。
- **自适应本机安装部署**（P0012）：oma 接管 rmux 与四家 agent 的安装：catalog 两层 pin（出厂锚 + `~/.ohmyagents` 用户本地层写回）、渠道序 github 主 CDN 兜底、sha256 信任锚、装后探针；`oma agents install/update`。Windows 四家装机全绿。
- **联邦轨迹检索**（P0013/P0014）：`oma trace` 六视图查询时直读四家原生会话库；双意图、operation_id 归组、epoch ms 归一；grok 主源升级 updates.jsonl 权威日志（逐事件真实时间，S020）。
- **三传输编排面**（P0011/P0016）：api 传输无关层一份核心三消费：`oma serve`（六操作 RESTish、JSON 信封、可视化网页、SSE 渲染行画面、trace 端点）、`oma mcp`（stdio 九 tools）、REPL（裸 `oma`，编排面内嵌）；三通道共测全绿。
- **输出与易用**（P0015）：六会话命令 `--json` 信封、`oma status` TTY 表格、`oma completions`、R002 输出规范节。
- **Windows 全量收口**（P0017）：send 回显间隔产品化（S005 铁律）、SKILL.md 命令图生成（S016 末件）、grok 无头实跑（S007 回填）、`oma mcp --print-config`。
- **指令集检测**（P0018）：`oma doctor` CPU 能力段（avx/avx2/avx512f）与探针异常退出分类（illegal-instruction 带缓解 hint），S021 问题类的 Windows 落地。
- **文档地基**：AGENTS 四段、三原语、P/S/R/G/M 编号体系、六态标记、rumdl 加两件自研扫描进门禁、`.tools` 脚本归档。

### 里程碑 2026-09-01

- **web 镜像与看板**（P0021 至 P0023）：`oma web` 三面接管 rmux web-share（operator、PIN、TTL）；前端源码本地构建托管、serve 主页即 web-mirror-server；看板资源包化（build.rs 打 tar.gz 嵌二进制、首启释放 oma 数据根）。
- **和解式编排**（P0024）：spawn 三态（会话不在新开、在则活路附加、死路重开）；`oma respawn` 单路强制重开；精确集合与布局自适应。
- **serve 守护化**（P0025）：`serve start` 即调即退（CREATE_NO_WINDOW）；`serve stop` 协议化停机（DELETE /shutdown 优雅排空）。
- **code review 修复**（P0026）：并发安全与健壮性三切片（看板默认只读加 Host 校验、cleanup 僵局解除、task id 原子占位等高 5 中 7 全修）。
- **任务与委派**：`oma task` 带产物等待（任务目录协议 prompt.md/output.md/DONE）；send/run/task 任务开始确认与阻塞告警；`oma key` 单键守卫。
- **流程件**：G004 经验沉淀细则；README 三段重写；oma 编排的 agent 轮换接力 review 工作流（`.tools/review-round.py`）。

### 里程碑 2026-09-02

- **四环境部署自适应**（P0027）：PATH 探针 bare 形态与 codex 字段所有权（Windows 与 WSL 双侧并存不互踢）；状态栏重铸（starship 风格、`agent:state` 机读标记、`oma status` 扫屏交叉核对）。
- **agent doctor 部署诊断与登录引导**（P0028）：doctor warn 层四类部署检查（登录态、hook 形态、状态栏、会话健康）；`oma agents login` 跨机设备码引导。
- **密钥与权限面**：spawn claude 路固定 `--dangerously-skip-permissions`（S029）；`oma hook` 密钥拦截闸（S030，八层防误报）；`oma agents secrets` 一钥两密文与四 shell 懒注入（S031）；`oma agents providers` 别名注入（S027）。
- **生态**：仓库更名 ohmyagents-rs；四仓生态定调（ohmyenv-rs / ohmyagents-rs / ohmypwsh / ohmycloud）。
- **三平台验收**：Windows、macOS、WSL Linux 四家 agent 安装与真身四路全链绿（P0012 收口）。
- **流程件（2026-09-03）**：参考 reader_rs 文档体系重构：PRD 四原语引入、AGENTS 工作规则重组加文档对齐义务表、R002 升命令面唯一权威、INDEX 收敛九节修复登记缺陷、TODO 残表清退、根级五文件禁字合规退出豁免清单。

### 里程碑 2026-09-07

- **迁册批**（P0029 / D07）：`oma agents install` / `update` deprecated 指向 `ome install`；`catalog\agents.toml` 冻结历史锚；doctor 四类检查归 agents 域。
- **doctor 检查面**（P0030 / D08）：Grok 状态栏 command 三态（Windows 只认 `.cmd` 单路径）；JSON hook args 数组 warn。热修 M044 至 M048。
- **边界**（D09）：oma 不管种子，只管诊断、配置、hook、状态栏和编排。
- **G005 存量字符**（P0031 / D10）：SKIP_DIRS 外四类禁字清零；封闭清单删除；mdcharlint 零容忍。
- **状态栏工具链**（P0032 / D11）：projKind 扩展 zig / go / cpp（build.zig / go.mod / CMakeLists 或 meson）；图标 cmap 实证 seti-zig E6A9、seti-go E627、seti-cpp E646。
- **任务目录孤儿**（P0033 / D12）：删除误落 `.ohmyagents/t006/`；协议尾注路径显式 `tasks/`。
- **数据目录**（P0034 / D14）：项目与家目录 `.ohmyagents` 改名为 `.oma`；旧目录仅旧在则改名迁过去。不是 `.omc`（ohmycloud）。

### 里程碑 2026-09-08

- **重新定位**（P0035 / D15）：oma 去编排，退化为 Agent 全平台 token、hook 与状态栏部署配置工具。编排命令（check / spawn / respawn / status / send / key / run / task / settle / cleanup / REPL / web / serve / mcp / trace）与 rmux 后端整体移除；保留 init / doctor / agents（检测 / login / providers / statusline / secrets 加 deprecated install / update）/ hook / self update / completions。
- **self update 镜像通道**（P0036 / D16）：`OMA_MIRROR=<基址>` 只覆盖 dev 通道；边车 sha256 判新免 manifest、下载强制校验（不符报错不回落）、网络失败回落 GitHub；尾巴修缓存击穿（边车 `?t=` 时间戳、资产 `?v=` 边车锚，每滚天然新键）。e2e 一次性副本实证三分支全绿。
- **首个正式版 v0.1.0**：整理清理后封版，v* tag 触发正式 release（三平台 zip / tar.gz 加 .sha256 边车，同 dev 形态）；`oma self update --stable` 走 releases/latest 吃到。镜像分发与四端测试验收归 ohmycloud。
- **trace 六视图全量恢复**（P0037 / D19）：D15 连坐删除的对话历史检索以只读检索面回归（与 rmux 零耦合）；代码自 git 历史原样带回，定位文补「对话历史检索」域。
- **无头验收**（P0038 / D17）：`oma agents verify` 四家 agent 无头验收（状态栏 mock 直跑加 hook 无头落盘两层判据，S033 取证底座）；本机四家全绿。
- **v0.2.0 封版**：trace 恢复后第二正式版（v* tag 触发，三平台资产加边车同形态）；`oma self update --stable` 吃 releases/latest 即到此版。

### 里程碑 2026-09-09

- **状态栏用户级定制**（D18）：脚本拆段拼装（HEAD 加 COMMON 加 PROBE 加 14 段块加 TAIL，行为等价迁移）；`~/.oma/statusline.toml` 三层定制（`segments` 段落显隐与序、`[template]` 段内模板加 `[icons]` 图标映射、`[codex] items` 内置项子集），键级缺省回落内嵌默认、坏文件硬错；`oma agents statusline --script <路径>` 整脚本替换（marker 跳过内嵌覆盖）加 `--builtin` 还原；`--example` 模板打印。README 精练为人类面向（三平台预编译安装加使用示例）。
- **去 token 注入收窄**（D20）：oma 收敛为五功能（可用性诊断、hook、状态栏、trace、yolo）；删 `oma agents secrets`（一钥两密文与四 shell 懒注入）、`providers` 别名簿、`login` 设备码引导、`install` / `update` 兼容层与 catalog 安装机器（净删约三千行）；secretguard 实值比对只看环境变量；密钥安全归 ohmypwsh、agent 二进制归 ome。本地部署位与 shell profile 懒注入块不动（只改程序代码）。
- **活性诊断族**（D21，ohmyagents-rs#8 / ohmycloud D45 配套）：`oma diagnose cache`（网关别名双连缓存命中矩阵，ds 线自动前缀不可见特判，三连取优抗网关异区）与 `oma diagnose agents`（配置指向、别名在册核对、key 活性、thinking 上限对照）；与 doctor 的零网络体检契约分家；真网关实收 codex 线 hit、ds 线 auto-prefix。
- **自适应技能生成**（D22）：`oma skill [--write]` 从 clap 活命令树渲染 oma 自身 SKILL.md（Agent Skills 标准形态，新命令自动出现），`--write` 落用户级 `~/.claude/skills/ohmyagents/`。
- **secretguard 完整 token 匹配**（D23，#9）：实值比对由裸子串改边界匹配（杀模型别名连字符超集误报；真实密钥完整值仍 block），告警 masked 带 `名[头4字符…长度]` 脱敏前缀。
- **codex hooks schema 修复**（M055）：hooks.json 的 `command` 字段为 codex 必填，Windows 侧新部署补 bare oma 兜底（修复 0.149.1 起 commandWindows 单独存在导致整份 hooks 解析失败被弃用、hook 不触发）。
- **trace 面五项修复**（D26，browser-harness-ts 实测反馈）：六视图全量吃 `--format` 三态（json 信封加 jsonl 逐行，此前恒 kv）；列表视图加 `--offset` 翻页加截断显式标记（has_more 加 total 加 offset_next，不再静默 clamp）；sessions 补 `--limit`；claude 会话 started 解析（跳过首行元数据取首个 timestamp 行）；「数据只回溯某日」证伪为截断症状（全量在库）。
- **codex hooks cmd 形态根修**（M056，herdr 实测报修）：commandWindows 由 PowerShell 调用语法 `& "exe"` 改 cmd 形态直接引号路径（codex 在 Windows 用 cmd 执行该字段，`&` 前缀必炸 code 1）；顺带单一来源卫生：init 清 config.toml `[hooks]` 非 state 定义键（消双 representation 警告）。
- **跨仓共识与优先级落档**（D24 / D25）：诊断分工（oma 单机运行时治理 / omc 舰队面）、集成优先级序（诊断检测首要、恢复治愈次之、安装部署配置最后，ROADMAP 阶段 8）、同文件协调三点裁（状态栏段 omc 永不碰、yolo 托管端 omc 为准、pretrust 归 oma）；herdr 周知口径与三活仓路径登记。
- **v0.5.0 封版**：三仓里程碑对齐版（omc 0.3.0 / ome 0.2.0 / oma v0.5.0 同日，用户裁提前至今日）；M056 加共识落档为主增量。
- **v0.4.0 封版**：D22 加 D23 加 M055 三件合并（v* tag 触发，三平台资产加边车同形态）；`oma self update --stable` 吃 releases/latest 即到此版。
- **D41 关账**：ohmycloud 双段验收全绿（dev 段与 stable 段三方对账 sha 逐字一致，回执 issuecomment-5598606699 / 5599216966），「我滚你播」接力退役。
- **v0.3.0 封版**：D18 后第三正式版（v* tag 触发，三平台资产加边车同形态）；mirror job 随 tag 首次填充 oma/stable 段；`oma self update --stable` 吃 releases/latest 即到此版。

### 里程碑 2026-09-10

- **hook 与 oma 二进制解耦**（D27 / P0044，用户裁全平台 shim）：`oma init` 先落自包含状态写入脚本到 `.oma/hooks/`（Windows cmd 加 Linux bash 加 mac zsh 三份全侧落齐，跨 OS 共享项目并存），各家 hook 注册指向 shim，state 通道零 oma 依赖（oma 任意时刻可无痛轮换升级）；secretguard 由 shim fail-open 委托（oma 在位转发 payload 透传 exit 2，不在位放行）。cmd shim 两级形态：部署前探 PATH 上 jq（jq 归 ome 部署），在位落 jq 解析版（提取与生成全走 jq --arg，ts 取 now 加 floor），缺位落 findstr 回落版加 warn 指向 `ome install jq`；sh 侧 sed 基线。陈旧收敛：老 bare 与旧 exe 形态重部署一律收敛 shim 单条；doctor `hooks.form` 加 shim 读法。
- **codex hooks PS 调用操作符根修**（M057，订正 M056）：codex 的 hook 经会话环境 shell 执行（Windows 缺省 PowerShell，源码 session/mod.rs 实证），commandWindows 正确形态是 `& "路径" codex`；M056 的「cmd 直引号形态」系验证通道错误（只经 cmd /c 直测未跑真 codex），实测直引号三事件全 Failed、调用操作符全 Completed 且 state 落盘。存量用户级 `~/.codex/hooks.json` 里 M056 期条目需手改（oma 不动家目录）。
- **verify kimi 层改临时全局注册**（M058，S033 勘误）：kimi print 模式只触发全局 `~/.kimi-code/config.toml` 的 `[[hooks]]`（项目级不触发；Windows 侧历次绿是被用户全局条目掩蔽），且 hook 命令必须不带引号（引号形态静默不执行）；verify 改 KimiGlobalHooksGuard 字节备份加临时注册加 Drop 还原。
- **trace 金档化与三平台测试矩阵**（用户裁）：`OMA_TRACE_HOME` 覆盖会话库根（联调与测试通道，Windows 家目录解析走 SHGetKnownFolderPath 环境重定向无效）；trace 集成测试自种金档（CI 零数据可跑，修 D26 测试 CI 三平台红）；三平台同步测试（Windows 本机、WSL 共仓 Linux 运行时、mac clone 镜像单测集成验收三层）落地 R004 第 6 条，与 ome 仓讨论定标。
- **v0.5.1 封版**：D27 加 M057 加 M058（bug 清零后同发，用户裁）；三平台资产加边车同形态；三平台测试矩阵全绿（Windows 140+22、WSL 139+22、lan-mac 139+22）。
- **v0.5.3 验收加固**（codex 独立 review 三轮对齐收口）：merge 同形重复去重（claude/grok 面与 codex 面，A,B,A 集合语义）；doctor hooks.form 加 shim-dead 死链读法与 codex 分侧辨形（宿主侧判据，非宿主 bare 兜底不误报）；三个 md 扫描加 rumdl 进 CI docs-gate job；TODO/PLAN 对账补 D27 段；M060 挂账落档（guard 透传白名单化、mac shebang 可断言化）。三平台矩阵 Windows 143+22、WSL 142+22、lan-mac 142+22。
- **v0.5.2 快修**（M059）：注册形态统一无引号正斜杠绝对路径（`D:/路径/.oma/hooks/oma-state.cmd 名`）。v0.5.1 的 `&` 调用操作符形态在 claude 本体（Windows 装 Git Bash 时 hook 经 /usr/bin/bash -c 执行）是语法错误且退出码 2 等同阻断（dogfood 本仓实证：会话全工具被拦）；新形态 bash / PowerShell / cmd 三 shell 实测全过，真 codex 加真 claude 会话双活体验证，路径含空格部署侧 warn。三平台矩阵复跑全绿。

### 里程碑 2026-09-12

- **v0.6.1 更名收尾批**（D30 / P0047，净会话核验 D29 残面后清完即发）：四 sweep 连坐回归修：data_dir 被 bin 先建根挡迁移（README 装法先落 `~/.hst/bin` 则 rename 永不触发、老根 `~/.oma` 搁浅；新根仅含 `bin/` 或空视为未初始化，旧根子项逐个迁入、撞名保新侧）；项目 shim 退役环 v0.6.0 误换单查新名成死码（环补旧名 `oma-state.*`，标记双收 oma 与 hst 两种）；SKILL marker 整串匹配断旧项目再生（v0.5.x 旧尾巴 marker 不被认作生成物永不覆写；改 hst marker 加历史前缀双收）；技能身份统一回 ohmyagents 旧牌（D14 裁定延续：`hst skill --write` 落 `~/.claude/skills/ohmyagents/`，frontmatter name 与目录一致，v0.6.0 曾拧成 name: hst）。串面残留清扫：README Windows 安装块断链级路径错（`.oma\bin\oma-x86_64` 加四处旧仓 URL）、doctor/verify/deploy/main/statusline 用户可见串旧口径（hooks.form 路径、oma init CTA、隐藏别名 help、verbose 标记）、UA 两处随 hst、源注释现状描述批。156 单测加 24 集成全绿；本机老用户硬路径迁移实证（oma self update 拿 stub、装 hst、rename 迁移加 heal、doctor 绿、stub 转调）随批落 diary。
- **oma 更名 HST / hst / v0.6.0**（D29 / P0046，用户定夺破坏性一次做完）：产品 HST（Hooks, Statusline, Trace）、crate hst-cli（bin 加 lib `hst`）、数据根 `~/.hst`（`HST_ROOT` 覆盖；旧 `~/.oma` 首启同卷 rename 自动迁移，跨根并存用新不动旧）、win bin 目录 `.hst\bin`、env `HST_MIRROR/HST_GATEWAY_URL/HST_GATEWAY_KEY`（旧 `OMA_*` 一个版本内读并 stderr 提示）、资产 `hst-<target>`（release 兼容期同挂 oma stub 资产：警告后转调 hst）、镜像 `hst/` 主段 sync 加 `oma/` 兼容段推 stub（对齐 ark 策略）；命令面 `hst hook init|status|verify`、`hst statusline` 一级化（`agents statusline` 隐藏别名一个小版本）、`hst agents [list|verify]`；`hst init` 并入 heal（四家托管注册 `~/.oma` 改 `~/.hst`、oma 改 hst，is_ours 签名只动自家；doctor 加 `hooks.migrate` warn 探 shim 实体在位性）；guard stderr 前缀与白名单锚随迁 `hst secretguard:`；fmtio kv 前缀 `hst:`；skill/AGENTS/CLAUDE 模板与 README 全换（含 Hipo/hst PATH 共存注）；迁移三用例（旧无新 rename / 两者都有用新 / 只新根）钉 pathutil。职责不变：hook 落盘、statusline、只读 trace、doctor、yolo；不编排、不装二进制（归 ark）。

### 里程碑 2026-09-11

- **hook 用户级常驻与 session 分键状态**（D28 / P0045，用户裁「hook 应用户全局」，状态栏跨项目失效根修）：hook 注册与 shim 常驻用户级（四家统一：claude `~/.claude/settings.json`、codex `~/.codex/hooks.json` 加 config 信任预种、grok `~/.grok/hooks/` global 层、kimi `~/.kimi-code/config.toml` `[[hooks]]`；未 init 项目也有状态数据，消除 codex 用户级而 claude 项目级的不一致）；状态迁 `~/.oma/state/` 按 session 分键双写（`<agent>.json` agent 最新加 `<agent>-<session>.json`，session 三源 payload `session_id` / `sessionId` / `GROK_SESSION_ID`；SessionEnd GC 加 7 天陈旧清扫，防 herdr 多会话互踩）；状态栏 oma 段读序重排（session 键直读，会话闸不符续找不再 unknown 断头）；`oma init` 迁移退役项目级 ours 注册与 `.oma/hooks/` shim（外来保留，用户手治同型注册自动收敛）；doctor 检查面迁用户级（项目残留 `hooks.retired` warn、用户级状态不 block 不误归因）；verify 判据 env 隔离加用户级注册 byte 备份 Drop 还原；`OMA_USER_HOME` / `OMA_HOME` 隔离缝全链（测试不碰真实家目录）。
- **M060 挂账两笔清讫**（v0.5.3 codex 验收 review 提出，2026-09-11）：guard 透传白名单化（shim 三形态仅「exit 2 且 stderr 带 `oma secretguard:` 前缀」透传 block 并回放原因，oma 故障非零一律 fail-open 护无痛轮换；fake oma 三态行为测试 Windows 加 Unix 双钉）；mac shebang 注入缝（`deploy_shims_with(root, shell)` 加 `host_shell()`，zsh/bash 落盘变体任何宿主可单测，消除编译期 cfg 分支无测试点）。
- **v0.5.4 封版**：D28 根修加三轮裁定加 codex review 九条修复加 R2 六条残留清零。第 2 轮（用户同日两令）：yolo 与非阻塞键全量用户级（四家用户配置、项目旧键 init 等值退役、doctor 判据随迁，覆盖 D25 项目级口径；git 历史佐证用户级诉求自 POC 期逐面翻正）；状态栏中文目录乱码根修（stdin 字节级 UTF-8 解码，防 CP936 误解码「缁跨洘」形）。第 3 轮：yolo 命令两级显式（`--yolo` 用户级加 `--project-yolo` 项目级互斥，doctor 双级接受）。codex review：F1 codex 信任键源根修（键源 = hooks.json 路径，双版本无 bypass 活体落盘实证，P0010 旧账闭案记 M061）、F2 doctor 逐键对账、F3 家目录短路守卫、F4 外来定义保留、F5-F9 锁与辨形与回落形态加固。herdr codex review 三轮对齐后发（终局签收可发版）；三平台资产加边车同形态；三平台测试矩阵全绿（Windows 151+24、WSL 150+24、lan-mac 150+24）；ohmycloud 对端验收回执全绿（三资产镜像段锚对齐：win c7af19e1、linux 7f9b227c、mac 8a56ce5b，oma/stable 锚账闭合）。

### 里程碑 2026-09-13

- **状态栏 agent 版本段 v1.1.4**（D46，用户裁两轮）：二行 hst 段渲染 `agent-<version>:state` 形（连字符拼接，第 2 轮用户裁，如 `claude-2.1.268:working`，探不到版本回落旧形 `claude:working`）；版本并入 `{agent}` 值、模板与占位符零变化（代价 = 无法单独摆放或屏蔽版本段，记档 R002）。数据源两级：payload `version` 字段优先（claude/kimi/grok 三家 stdin 契约都带，S034 D46 追记二进制实证），取值与 probe 输出同走一条归一化（`数字.数字` 起头 token，非数字如 nightly 不显示；显示面与 verify 判据同源，S025 规范语法 `<agent-id>[-<version>]:<state>`）；payload 缺 version 时本地探（回退路径每帧定位零 spawn）：定位序 `HST_<AGENT>_BIN` > `HST_AGENT_PATH` 目录 > PATH（与 agents 探测多数对齐）、四家白名单放行探针；`<bin> --version` 只在缓存 miss 时跑（两步取值 stdout 优先未中再取 stderr，与 read_version 同口径），缓存 `~/.hst/cache/agent-version-<agent>.json` 键 (终目标路径, mtime, size) 三元组（ticks 整数记法：pwsh `ConvertFrom-Json` 会把 ISO 日期串自动转 `[DateTime]` 致串比较永不相等，本机实弹踩坑；软链解析一层按链接所在目录拼；7 天兜底重探覆盖壳 shim 与保留 mtime 的升级）、临时件加 [IO.File]::Move 原子落盘、无缓存可写不探；三态机（键变重探 / 键同有版本直用满 7 天强探 / 键同空值 5 分钟静默窗）入 R002；verify 与测试 `HST_VER_CACHE_DIR` 隔离（系统临时目录用完即删；冷缓存四家约 8s 量级口径同档）。codex 侧版本走内置项 `codex-version`（源码实证 `StatusLineItem::CodexVersion` + 本机 codex v0.154.0 二进制复核），用户裁去 `context-remaining`（left 百分比与 `Context N% used` 重复占宽）缺省集回十二项，**四家全覆盖是两套机制**（pwsh 面并入 agent 名、codex 面自家内置项）；herdr codex 设计轮 F1 至 F8 加 diff 轮 F1 至 F6 加 G1 全收口（归一化同源、定位序对齐、三元组加兜底、白名单加原子写、verify 隔离、状态机与机制分叉入档、探针两步取值）。
- **状态栏两行终态与 oma 遗产清扫 v1.1.3**（D44/D45，用户六令连发）：**D44 布局终态** = 默认两行：一行**项目状态**（shell / cwd / git / 包版本与工具链尾巴，环境与项目同线）、二行**agent 状态**（agent 态 / 模型 / context 百分比加 token 绝对值 / 耗时）；第三行去掉（`segments3` 默认空，tools / mcp 计数与 token 用量三段全退默认位、显式选用才出现，`{mix}` 仍显式占位）；拼装器空行剔除（干净两行脚本，无残余收线）、CTXPROBE 默认不拼（无消费者不解析 transcript）。**D45 oma 遗产清扫**（D29 定的兼容窗已过）：状态栏段 id `oma` 更名 `hst`（老配置报错带改名 CTA）；`OHMYAGENTS_STATE_FILE` / `OHMYAGENTS_AGENT` / `OHMYAGENTS_PROJECT` env 更名 `HST_STATE_FILE` / `HST_AGENT` / `HST_PROJECT`（shim 三形态、hook、verify、状态栏脚本全链同批）；状态读序去 `.ohmyagents` 项目回落；`OMA_MIRROR` / `OMA_GATEWAY_*` 兼容读删除；agent 探测 env 更名 `HST_AGENT_PATH` / `HST_<AGENT>_BIN`；`agents statusline` 隐藏别名与 `--pretrust` 旧拼写删除；**oma stub 二进制与 oma-\* release 资产退役**（镜像 oma/ 兼容段停推，ohmycloud 侧收段）；init 幂等清扫用户根旧名 shim（oma-state.\* 四件，P0047 观察面二收口）。**根修**：`hst self update` 自 D29 起一直选 oma-\* stub 资产装到自己头上（镜像边车 URL 也指向不存在的 hst/oma-\* 路径恒回落），本批资产名钉 hst-\* 后自更新通道才真正可用（此前各版升级实走镜像直装）。170 单测加 28 集成全绿。
- **状态栏三行精修 v1.1.2**（D43，用户五点裁定）：三组默认段序重排为终态分组：一行**项目状态** = cwd / git 分支（去 shell）、二行**agent 状态** = agent 态 / 模型 / context `46% [449k/977k]`（百分比加 token 绝对值，构成 mix 退位）/ 耗时、三行**运行时状态** = shell / tools / mcp 加包与工具链尾巴（token 用量整段退出默认行，token 绝对值并入 context 括号）；context 缺省模板改 `{icon}{pct}% [{used}/{window}]`（`{mix}` 占位符与 `tokens` 段 id 保留可显式选用）；CTXPROBE 门控随批收紧（codex F1：无 tools 段且生效模板不含 `{mix}` 时不解析 transcript，省每 tick 尾段解析）；`--example` 与老配置升级语义随批（显式行原样、未写行补默认去重）。169 单测加 28 集成全绿；本机 dogfood 三行新形实弹（`46% [439k/954k]` 加 `3d10h` 加 shell 行）。
- **状态栏三行重分组 v1.1.1**（D42，用户裁定双排版嫌丑重设计）：三行定名**项目状态**（shell / cwd / git 分支）/ **agent 状态**（agent 态 / 模型 / context 构成 mix 与百分比）/ **运行时状态**（tools 计数 / mcp 计数 / token 用量 / 耗时 / 包与工具链尾巴）；`segments3` 第三行配置键（缺省回落新分组）；机读标记 `agent:state` 从「首行含」放宽为「任一行含」（agent 态迁二行，verify 与 grep 消费面同步）；拼装器泛化为任意行数（`ps1_rowsplit(n)` / `ps1_tail_multi`），kimi / grok 运行时并单行退化与空行守卫沿用；行为测试改三行断言（agent 态在二行、运行时要素在三行、kimi 并一行同钉，codex D40 评审 G2 顺带收口）。
- **状态栏 D40 批 v1.1.0**（wsl 总台实弹三令 / S034 研究落地）：**双排布局**（`segments` / `segments2` 两清单加 `single_line` 退单排逃生门，用户裁定分组一排 shell/cwd/agent 态/模型/context 构成、二排 tools/MCP/token 用量/耗时）加**三要素**（tools 段 = transcript `tool_use` 计数；mcp 段 = stdin `mcp_servers` > `~/.claude.json` + `.mcp.json` 键数；context 段 `{mix}` = transcript 尾段行字符量三分估算构成占比）；CTXPROBE 共享探针按消费方门控（kimi 300ms 预算不伤）；codex 缺省内置项升十二项（token 细分 used/total-input/total-output/window 四项承载「token 用量」），源码取证约 30 项内置、无外部命令面（openai/codex#17827/#20244），tools/MCP/构成三要素 codex 不可达（差距说明入 S034/R002）；跨平台同一份 pwsh 脚本（宿主与 WSL 同构）。**旧配置外观变化与回退**：只写 `segments` 的老配置自动多出默认第二排；context 模板的 used/window 移入 tokens 段、新增 `{mix}` 占位；回退法 = `segments2 = []` 或 `single_line = true`；kimi 只取首行（S025 实证）加 grok 多行未实证，**运行时自动退单排**（两排并一行，包版本与工具链段对 kimi 保持可见）。168 单测加 28 集成全绿；本机 dogfood 双排实弹渲染。
- **v1.0.1 适配批**（D37 至 D39 / P0048 当日补丁节，宿主终验驱动）：verify 状态栏层改「验收已部署的面」（`[tui] status_line` 未部署或 pwsh 缺 PATH = skip 带 CTA 不计败，断链仍 fail，D37）；仓测活体验收环境容错（grok / kimi 跑一次不押断言、hook 层 ok 才计入，wsl 总台三轮到点，D38）；**Windows hook 注册 sh 兼容形态 ps1 桥**（D39 四轮：`powershell.exe -NoProfile -ExecutionPolicy Bypass -File <hst-state.ps1> <agent>` 四态通吃加新增 ps1 shim；is_ours 按 stem 认解释器头（含全路径头）加包裹分支收窄；非管理事件清扫与 codex 面外科摘键（异侧活注册逐字节保留）；doctor 判形经 program_token 防误报；ps1 本体带 UTF-8 BOM、state 输出无 BOM、`$input` 惰性枚举加读前 InputEncoding（三层根因链 M062/M063）；verify 判据补 event 非空加直管道往返集成钉）。herdr codex 三轮评审（F1 至 F6、G1 至 G4、H1 至 H2）零异议收口；166 单测加 28 集成与四文档门禁全绿。
- **hst 版本线重开 v1.0.0**（D34，用户裁 2026-09-13）：hst 版本号重新算起，不接 oma 老项目版本线；本版起 HST 线从 1.0.0 起算（与 ark 1.0.0 同语义开山版），本文件此前 oma 0.x 与 v0.6.x 各节一律读作**更名过渡期记录**；镜像 hst/stable 滚动照旧。
- **ohmycloud 协调批**（D31 至 D33 加 D35 / P0048）：私有网关域名全仓清扫（diagnose 注释加 README / R002 / P0041 脱敏，表述统一「api 缓存回归测试端点（配置注入）」；端点本就 env 加 agent 配置注入无硬编码，逻辑不动）；init 旗标连字符化 `--pre-trust`（旧 `--pretrust` 隐藏别名兼容到 1.1.0 兼容窗与 OMA_* env 同批清，kv 标记 `init.pretrust.*` 冻结不动，全仓 md 28 处随函更正含历史档案）；yolo 分级关闭 `--yolo[=full|partial|off]` 与 `--project-yolo[=级别]`（裸旗标兼容、级别缺省 full；partial 危险操作仍确认：claude `acceptEdits`、codex `workspace-write`/`on-request`、kimi `auto`、grok `auto`（grok-build `permissions.rs` canonical 值集实证，S007 缺口收口）；partial 摘 ours 落的 enableAll（同批或重跑 `--pre-trust` 会再开；名单混 agent 原生用户审批无法归因一律保留）；off 按 ours 等值摘除（新用户级 retire 面，值集含 full 与 partial 两代，用户自设值保留空文件删除）；doctor 判据分级接受不再误报、冲突 CTA 带 `=<level>`；marker 只增 `init.yolo.level` 与 `init.retired`）；README 精简重写专注安装部署使用加 GitHub 仓库描述一句话（D35，canonical 仓名 `hst_rs`）。herdr codex 三轮独立评审达成一致后封版；160 单测加 27 集成与四门禁全绿；本仓 dogfood init 四处 SKILL 再生与用户级技能刷新。

### 里程碑 2026-09-14

- **Windows 构建切 gnu 交叉编译 v1.1.5**（D47，用户裁摆脱 VC，stable 切 gnu 封版）：CI 的 windows-latest msvc 岗换 ubuntu-latest 交叉岗（apt mingw-w64 条件步、Test 交叉岗跳过由 linux/mac 双岗覆盖、Package 后缀改判 triple），Windows 资产名改 `hst-<arch>-pc-windows-gnu.zip`（CRT 静态零 DLL 依赖，12.1MB 与 msvc 同量级，ohmycloud 裸环境交叉与 lan-win 实弹双实证）；`hst self update` 关键词梯子 `windows-gnu > windows-msvc > windows`（新源选 gnu、旧 release msvc-only 回落、通用词保底旧 msvc 二进制升级）；codex 复核四发现全收（S028 D47 追记、INDEX 标签、cfg 断言 CI 盲区记 TODO、README gnu 翻名随本封版落地）；herdr 共存实证同日落档（S035，状态栏 unknown 非冲突定性）。

### 里程碑 2026-09-15

- **断源自救与诊断正名 v1.2.0**（D48 至 D51 攒批，用户裁「要新封版」）：**D48 stable 镜像腿与限流自救**（ohmycloud 舰队撞 api.github.com 匿名 403 断源转需求）：`HST_MIRROR` 未设改 GitHub 优先、失败自动回退镜像腿默认基址（ark 三层读序参照，段随通道、dev 禁落 stable）、设值扩到 stable 通道 mirror-first、`GH_TOKEN` 在位附 Bearer（匿名 60 升 5000 次每时）；真网 e2e 两轮（mirror-first stable 腿下载替换与幂等 already-latest）；顺带根修 stable 通道镜像整段跳过的老缺口。**D49 技能名翻 hst**（两轮：先兼容窗、后用户令旧牌直接删除）：技能唯一名 `hst`，旧牌 `ohmyagents` 目录由 `skill --write`（`skill.retired=`）与 `init` 四处（`(retired)`）幂等退役，ours 识别（marker 家族或生成签名）用户手改不动，外科式只删 SKILL.md 目录空才收。**D50 doctor bypass 残余阻塞面**（S029 追记分类学落地）：新检查 `yolo.ask`（ask 规则 bypass 下照弹，三层）与 `yolo.readblock`（读沙箱键，不可静态分析命令即使 bypass 也问人，三层），既有 yolo 检查补 `settings.local.json` 层并修项目层 bypass-only 假阳性（claude 2.1.257 起项目层被忽略）。**D51 状态栏 clock 段**：第一行行尾年月日加当前时间（`yyyy-MM-dd HH:mm` 分钟精度，渲染事件重绘即活钟；`{datetime}` 占位与 `clock` 图标键可定制，冒号转义钉 ASCII 免区域文化替换）。S029 追记 bypass 残余阻塞分类学（官方七页文档加 changelog 取证，证伪 resume 还原旧记）与 S035 herdr 共存实证同窗落档。

### 里程碑 2026-09-16

- **治理迁移与四端实弹 v1.3.0**（D52 铁证根修至 REQ-007 攒批，总台统一封版令）：**D52 家目录守卫**（宿主反例三轮归因收敛：裸 init 于家目录 cwd 时项目级退役趟洗用户键，deploy_all 守卫 + --project-yolo 同罩，三端 fleet 实弹验）；读侧 BOM 容忍全覆盖（json 与 toml 五路）加 yolo full 双面落 blockReads=false 加 doctor yolo.parse 显式报。**D53 状态栏面并入 init**（fleet 实测脚本停旧版：deploy_script 加四家 merge 随全套部署，内容判等幂等，marker 保护自备）。**文档体系迁移 dev-evo**（REQ-001：AGENTS 五节合同、ADR 四件、REQ 登记、docs/README 地图承接 INDEX、PE-11 豁免机制化 PEVO_CHECK_ALLOW）。**aidoc 投影强制**（REQ-006：missing_docs deny 加 /// 契约注释 163 处加格式纪律 337 处补注、23 artifact 进 Git、--check --strict 漂移门禁、clippy 四 lint）。**全平台四端实弹矩阵**（REQ-007：cross-test.sh，linux 加 ubuntu 加 mac 加 win 回环，连接姿势 WSL 到宿主恒走 127.0.0.1）。S029 追记 bypass 残余阻塞分类学（归因三次反转教训入档）加 D50 doctor 诊断面（yolo.ask 与 yolo.readblock）随 v1.2.0 已发。

### 排后

- Linux/mac 接管（P0012 跨平台面）：资产与代码路径就绪；指令集 SIGILL 预备检测研究已备（S021）。
