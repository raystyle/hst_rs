# INDEX：项目总索引

> 角色：全仓**唯一索引**，只做定位：编号表、目录结构、代码文件位置。搜索与分析方法（rg、mq、ast-grep 怎么配合本索引）见 `AGENTS.md` 四、资源索引。规则权威源见 `AGENTS.md`；命名与编号规则见 `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`。

> 迁移注记（2026-09-15）：索引职责由 `docs/README.md` 地图承接（dev-evo 体系）；本文件迁移期保留，编号表与代码定位表仍有效，新文档不再登记。

## 一、编号体系

**前缀定位**：`D`（PRD 需求清单，2 位）；`P`（proven，已完成 plan 归档，4 位）；`S`（research，研究原型过程，3 位）；`R`（references，开发测试参考，3 位）；`G`（guide，元规范，3 位）；`M`（mistakes，分类文件 M1xx、行级错误 M0xx 全局递增不复用）。根目录四原语：`PRD`（需求清单）/ `GOAL`（目标轨迹）/ `PLAN`（当前目标方案，基于研究与参考）/ `TODO`（进度清单）。

**目录职能**：`proven` 已完成 plan 归档；`diary` 一天一篇总结与自省；`research` 研究原型过程（为什么，六态对齐，规范见 G002）；`references` 开发测试参考（要做什么怎么做，六态溯源）；`guide` 元规范（含 `template.md`）；`mistakes` 出错怎么纠（与 references 是经验教训的两面）；`web` 前端资源包输入区（D15 起退役：消费者 build.rs 与 webassets 已删，目录留作历史资产，不再维护）。

新文档按类别落位，编号接当前最大号，登记进本索引对应节；编号退役或断号留注记（见 R003 与 P0020 先例），不复用。

## 二、目录结构与代码文件位置

| 类别 | 目录 | 说明 |
| --- | --- | --- |
| 文档 | `docs\`（proven/diary/research/guide/references/mistakes/web）+ 根目录 PRD/GOAL/PLAN/TODO/INDEX/AGENTS/README/CHANGELOG/ROADMAP | 见上节职能 |
| 代码 | `src\` | Rust CLI `hst`（catalog 目录已随 D20 移除，pin 数据权威在 ark-rs 原 ome） |
| 运行时产物 | 目标项目下 `.hst\`（D29 更名） | gitignore 项目态；工具前缀不进仓 |

**代码文件位置**：

| 文件 | 职责 |
| --- | --- |
| `.tools\` | 项目自定义脚本工具归档（ps1 / py / Rust；`README.md` 含清单与规则；`uv run --script` 载体） |
| `.tools\md-ref-scan.py` | markdown 仓内引用断链扫描（文档大改后回归门禁；豁免清单 `md-ref-allow.txt`） |
| `.tools\md-replace.py` | 中文与反斜杠路径安全的字面批量替换（规避 sed 坑 M023） |
| `.tools\md-heading-scan.py` | 标题括号规范扫描（G001 标题干净的机检项；代码围栏内的注释不计） |
| `.tools\mdcharlint.py` | 四类禁用字符检查（G005：破折号、箭头、emoji、非法全角；掩豁免区后逐字符扫） |
| `src\main.rs` | CLI 入口与子命令分发（init/doctor/agents/hook/self/completions）；`--json` 信封出口；D49 skill --write 单名 hst 加旧牌幂等退役；D52 --project-yolo 家目录守卫） |
| `src\lib.rs` | 模块声明 |
| `src\archive.rs` | 通用归档工具：sha256 校验、zip / tar.gz 解包、目录复制、host os/arch（D15 自 rmux.rs 剥离） |
| `src\hook.rs` | `hst hook`：事件到四态映射与用户级 session 分键 state 落盘（D28），加密钥拦截分流 |
| `src\agents.rs` | `hst agents`：PATH / 环境变量 / 默认目录探测 |
| `src\doctor.rs` | `hst doctor`：只读诊断（yolo / 信任 / 二进制 / 登录态 / hook 形态 / 状态栏 / 用户级与项目级状态面，D28；会话健康随 D15 移除；D50 yolo 面补 bypass 残余阻塞信号：项目层 local 优先读取、项目层 bypass-only 假阳性修正、yolo.ask 与 yolo.readblock 检查；D52 yolo.parse 显式报加 json_file BOM 容忍） |
| `src\yolo.rs` | `hst init --yolo[=full|partial|off]`：四家分级落盘、ours 退役（用户级与项目级）与 pretrust（D33）；D52 full 落 blockReads=false 加 off 等值摘、read_json BOM 容忍） |
| `src\deploy.rs` | `hst init` hook/skill 部署层：hook 注册四家用户级（D28：claude/codex/grok/kimi 用户层，codex trusted_hash 预种，kimi `[[hooks]]` 合并），项目级 ours 注册与 shim 退役，幂等合并（M059 无引号正斜杠形态，同形去重）；SKILL.md 由 COMMAND_MAP 命令图生成（标记覆写三态；D49 四处翻 skills/hst/ 加旧牌 ohmyagents 幂等退役；D52 家目录守卫：root==用户家目录时项目级退役趟整组跳过；D53 状态栏面并入全套部署） |
| `src\shim.rs` | D27 自包含状态 shim 加 D28 用户级常驻与 session 分键：hst-state.cmd（jq 首选加 findstr 回落，PATH 探 jq）、hst-state.ps1（D39 sh 兼容载体，Windows 注册指向）、hst-state.sh（bash 或 mac zsh）、grok baked 包装；落 `~/.hst/hooks/`，双写 agent 最新加 session 键，SessionEnd GC |
| `src\install.rs` | hst 根解析（hst_home）加自管根存量探测（managed_binaries/version）加共享下载件 download_asset（self update 复用）；安装机器已随 D20 删除 |
| `src\statusline.rs` | `hst statusline`：四家状态栏写入面幂等合并（S025 矩阵）；projKind 含 rust/node/python/zig/go/cpp（P0032）；脚本拆段拼装加用户级定制烘焙（~/.hst/statusline.toml 分层键加 --script 整替换，D18；D42 三行、D43 五点精修、D44 默认两行、D45 段名 hst；D46 agent 版本段：payload version 优先否则本地探加 mtime 键控缓存，标记两形 `agent[-<version>]:state`，codex 内置项 codex-version 并裁去 context-remaining 回十二项；D51 clock 段：第一行行尾年月日加当前时间分钟精度） |
| `src\update.rs` | `hst self update`：dev 滚动源与正式版判新、sha256 取证、Windows rename 舞步（S028）；HST_MIRROR 镜像通道加缓存击穿（D16；D45 资产名钉 hst-*、旧 OMA_MIRROR 读已删；D47 Windows 资产名与关键词切 gnu 带 msvc 回落；D48 双通道镜像腿、缺省 GitHub 失败自动回退默认基址、GH_TOKEN Bearer） |
| `src\trace.rs` | `hst trace` 六视图：联邦读四家原生会话库归一检索（P0013/P0014，S018/S019/S020；D15 连坐删除，D19 全量恢复） |
| `src\verify.rs` | `hst agents verify`：四家无头验收两层判据（D17，S033）；状态栏 mock 直跑（D46 标记两形判据加 HST_VER_CACHE_DIR 隔离）加 hook 用户级注册 byte 备份 Drop 还原与 env 隔离判据（D28）；grok trusted_folders 种子加 Drop 摘除 |
| `src\diagnose.rs` | `hst diagnose cache\|agents`：活性诊断族（D21）：网关发现（env 覆盖大于 claude env 大于 codex provider）、缓存双连探测加 ds 特判、配置指向加在册加 key 活性加 thinking 对照；D52 claude env 读侧 BOM 容忍） |
| `src\skillgen.rs` | `hst skill`：从 clap 活命令树自适应渲染 SKILL.md（D22；frontmatter 按 Agent Skills 标准，--write 落用户级技能目录；D49 名翻 hst 唯一名加旧牌幂等退役） |
| `src\secretguard.rs` | `hst hook` 密钥拦截闸（S030）：模式表八层防误报、实值比对通道、PreToolUse/UserPromptSubmit 阻断 exit 2 |
| `src\fmtio.rs` | 全局输出三态（kv/json/jsonl）与结构化错误出口（issue #1 契约，R011）；JSON 信封函数（D15 自 api.rs 迁入） |
| `src\caps.rs` | CPU 指令集能力与探针退出形态分类（S021/P0018：is_x86_feature_detected 加 0xC000001D 识别） |
| `src\pathutil.rs` | 路径工具；家目录与 hst 根解析（D29 起 `~/.hst`）；`user_home`（HST_USER_HOME 隔离缝，D28） |
| `tests\cli.rs` | CLI 集成冒烟（assert_cmd；agents/hook/doctor/init/statusline 部署配置面加 trace 只读检索面） |

（D20 移除：`src\secrets.rs` / `providers.rs` / `login.rs` / `catalog.rs`、`catalog\agents.toml`（token 注入面与 install/update 兼容层，密钥安全归 ohmypwsh）；D15 移除：`src\rmux.rs` / `rmuxpoc.rs` / `orch.rs` / `task.rs` / `servectl.rs` / `api.rs` / `mcp.rs` / `server.rs` / `repl.rs` / `webassets.rs`、`build.rs`、`catalog\rmux.toml`、`examples\poc-*.rs` 十四件、`.tools\share-view-probe.py`、`.tools\review-round.py`；`trace.rs` 后经 D19 恢复；历史见 git。）

## 三、方案归档

> 位置 `docs\proven\`。

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| P0001 | `P0001-四路会话工具-CLI控制面与网页观察面.md` | 首期切面 |
| P0002 | `P0002-项目重新定位-通用多Agents自动配置和任务编排器.md` | 上一版定位 |
| P0003 | `P0003-rmux检测版本哈希与全平台安装.md` | `oma check` |
| P0004 | `P0004-项目重新定位-通用智能体多路复用任务编排器.md` | 现役定位 |
| P0005 | `P0005-各功能部件POC验证原型.md` | 各功能部件 POC 验证 |
| P0006 | `P0006-产品命令最小闭环-spawn状态send与cleanup.md` | 产品命令最小闭环 |
| P0007 | `P0007-send多行粘贴与label端点融合.md` | send 多行粘贴与 label 端点融合 |
| P0008 | `P0008-oma-run委派与任务映射.md` | oma run 委派与任务映射 |
| P0009 | `P0009-真四路拉通验收.md` | 真四路拉通验收 |
| P0010 | `P0010-settle自愈信任-自检测与自动确认.md` | settle 自愈信任 |
| P0011 | `P0011-三传输编排面-http-api与mcp与网页可视化.md` | 三传输编排面 |
| P0012 | `P0012-自适应本机安装部署-rmux与四家agent接管.md` | 自适应本机安装部署 |
| P0013 | `P0013-agent意图操作块与编辑轨迹检索.md` | 意图操作块与编辑轨迹检索 |
| P0014 | `P0014-grok权威日志升级.md` | grok 权威日志升级 |
| P0015 | `P0015-S016吸收件收口.md` | S016 吸收件收口 |
| P0016 | `P0016-REPL与编排面内嵌.md` | REPL 与编排面内嵌 |
| P0017 | `P0017-Windows全量收口.md` | Windows 全量收口 |
| P0018 | `P0018-Windows侧指令集检测落地.md` | Windows 侧指令集检测 |
| P0019 | `P0019-产品完备收口与四家真路验收.md` | 产品完备收口与四家真路验收 |
| P0021 | `P0021-官方web镜像集成.md` | 官方 web 镜像集成 |
| P0022 | `P0022-web镜像本地化与主页化.md` | web 镜像本地化与主页化 |
| P0023 | `P0023-看板资源包化.md` | 看板资源包化 |
| P0024 | `P0024-agent实例和解式编排.md` | agent 实例和解式编排 |
| P0025 | `P0025-serve守护化与协议化停机.md` | serve 守护化与协议化停机 |
| P0026 | `P0026-code-review修复-并发安全与健壮性.md` | code review 修复 |
| P0027 | `P0027-四环境部署自适应-hook形态与状态栏.md` | 四环境部署自适应 |
| P0028 | `P0028-agent-doctor部署诊断与登录引导.md` | agent doctor 部署诊断与登录引导 |
| P0029 | `P0029-D07迁册批-agents下装归ome.md` | D07 迁册批：agents install/update deprecated 指向 ome、agents.toml 冻结历史锚、doctor 四类归 agents 域 |
| P0030 | `P0030-D08-doctor检查面补形态.md` | D08 doctor 检查面：Grok 状态栏 command 三态与 hook args 形态 |
| P0031 | `P0031-D10-G005存量字符清理.md` | D10 G005：SKIP_DIRS 外四类禁字清零，封闭清单删除 |
| P0032 | `P0032-D11-状态栏工具链段扩展-zig-go-cpp.md` | D11 状态栏 projKind 扩展 zig / go / cpp |
| P0033 | `P0033-D12-任务目录孤儿收敛.md` | D12 删除误落 `.ohmyagents/t006/`；协议尾注路径显式 tasks/ |
| P0034 | `P0034-D14-数据目录改名为oma.md` | D14 项目与家目录 `.ohmyagents` 改 `.oma` |
| P0035 | `P0035-项目重新定位-Agent全平台部署配置工具.md` | D15 去编排：oma 退化为 Agent 全平台 token、hook 与状态栏部署配置工具 |
| P0036 | `P0036-D16-self-update镜像通道.md` | D16 `OMA_MIRROR` 镜像通道：dev 段边车判新、sha256 强制校验、网络失败回落 GitHub |
| P0037 | `P0037-D19-trace六视图全量恢复.md` | D19 trace 六视图全量恢复：只读检索面回归，与 rmux 零耦合 |
| P0038 | `P0038-D17-agents-verify无头验收.md` | D17 `hst agents verify`：状态栏 mock 直跑加 hook 无头落盘两层判据；本机四家全绿 |
| P0039 | `P0039-D18-状态栏用户级定制.md` | D18 状态栏用户级定制：拆段拼装加生成时烘焙（segments / template / icons / codex items 三层键、--script 整替换） |
| P0040 | `P0040-D20-去token注入收窄.md` | D20 去 token 注入收窄：五功能收敛，secrets / providers / login / install / update 五面删除，secretguard 只看环境变量 |
| P0041 | `P0041-D21-活性诊断族.md` | D21 `hst diagnose cache\|agents`：网关发现三源、缓存双连探测加 ds 特判、配置指向加在册加 key 活性加 thinking 对照 |
| P0042 | `P0042-D22-自适应技能生成.md` | D22 `hst skill [--write]`：clap 活命令树自适应渲染 SKILL.md |
| P0043 | `P0043-D23-secretguard完整token匹配.md` | D23 实值比对完整 token 边界匹配加脱敏前缀（#9） |
| P0044 | `P0044-D27-hook与oma二进制解耦全平台shim.md` | D27 自包含状态 shim（cmd jq/findstr 两级加 bash/zsh）加 M057 codex PS 调用操作符根修 |
| P0048 | `P0048-ohmycloud协调批与版本线重开.md` | D31 至 D35 ohmycloud 协调批 / v1.0.0 版本线重开：域名清扫、--pre-trust、yolo 分级 full/partial/off、README 重写；herdr codex 三轮评审闸门 |
| P0047 | `P0047-D30更名收尾批.md` | D30 更名收尾批 / v0.6.1：data_dir pristine 合并、退役环与 marker 双收复活、技能身份回 ohmyagents、串面清扫 |
| P0046 | `P0046-D29-oma更名HST破坏性迁移.md` | D29 更名 HST / hst / v0.6.0：crate hst-cli、`~/.hst` 根迁移、heal 即重部署、oma stub 与镜像双段 |
| P0045 | `P0045-D28-hook用户级常驻与session分键状态.md` | D28 hook 注册与 shim 用户级常驻（四家统一）、`~/.oma/state/` session 分键双写、状态栏会话读序、项目面退役 |

（P0020 断号：编号已预留未使用，不复用。）

## 四、项目日记

> 位置 `docs\diary\`；一天一篇总结自省。

- `2026-08-29-对照ohmypwsh建立文档骨架.md`
- `2026-08-31-研究体系与POC全绿.md`
- `2026-09-01-和解式编排与资源包化.md`
- `2026-09-02-四环境自适应与状态栏重铸.md`
- `2026-09-03-仓库清理与文档体系重构.md`
- `2026-09-05-D06吸收合并agent二进制下装部署.md`
- `2026-09-08-D15去编排oma退化为纯部署配置工具.md`
- `2026-09-09-D18状态栏用户级定制.md`
- `2026-09-09-D20去token注入收窄.md`
- `2026-09-09-D21活性诊断族.md`
- `2026-09-10-D22D23技能与守卫修复.md`
- `2026-09-11-D28hook用户级与session分键.md`
- `2026-09-12-D29更名HST.md`
- `2026-09-12-D30更名收尾批.md`
- `2026-09-13-D31D35协调批与1.0.0发版.md`
- `2026-09-14-herdr共存实证与D47gnu交叉切换.md`
- `2026-09-15-bypass残余阻塞分类学研究.md`

## 五、研究文档

> 位置 `docs\research\`；S 编号。

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| S001 | `S001-四路会话的控制面与观察面.md` | CLI 控四路、网页只镜像；三份 demo 拆解 |
| S002 | `S002-跨平台与无浏览器.md` | 端点平台口径、WMI breakaway、默认不弹窗 |
| S003 | `S003-rmux-sdk最佳开发实践与验证poc.md` | SDK 0.10 一手核查；POC-0..8 |
| S004 | `S004-win-rmux既有rmux研究吸收.md` | 进程模型、hardened-guard 八条、命令核验表 |
| S005 | `S005-drive铁律与三段式粘贴.md` | 五铁律与 paste 三段式 |
| S006 | `S006-信任阻塞门-四家种类与官方口径.md` | 四家门总表；Claude 官方分路 |
| S007 | `S007-yolo与无阻塞启动-配置落盘与无头分路.md` | 四家落盘表；无头分路；doctor 判据 |
| S008 | `S008-项目级hook与skill.md` | 四家项目级发现规则；init 部署树 |
| S009 | `S009-agent状态判断-通道与分层.md` | 四层含 1b 模型；文件总线；事件映射 |
| S010 | `S010-clum等待原语作为hook兜底状态.md` | `terminal_state` 分类器；SDK 超时真相 |
| S011 | `S011-Command-LineRust测试方法论与oma测试分层.md` | 三源测试对照（已固化 R004） |
| S012 | `S012-ponytail懒人阶梯与oma编码经验.md` | 实现取舍七档阶梯 |
| S013 | `S013-选型研究双通道实证-cratesio与github.md` | 两落地法证据链（已固化 R005） |
| S014 | `S014-检测已装agent-PATH与默认目录与环境变量.md` | 四家二进制探测路径表 |
| S015 | `S015-四家hook注册一手形态-官方文档与源码核实.md` | 注册落点/schema/事件全集/部署矩阵（poc-init 依据） |
| S016 | `S016-incurs命令输出与帮助经验吸收.md` | 双层源码研究：输出信封/CTA/帮助/三传输参照（P0011 依据） |
| S017 | `S017-ohmypwsh安装配置机制与四家agent渠道取证.md` | ohmypwsh 安装配置蓝本与四家渠道 checksum 取证（P0012 依据） |
| S018 | `S018-aitrace意图轨迹机制研究与oma检索映射.md` | aitrace 三源关联机制、裁决表与坑清单（P0013 依据） |
| S019 | `S019-四家会话日志格式与联邦检索取证.md` | 四家会话库四要素钉死与三仓源码纠偏（P0013 依据） |
| S020 | `S020-grok权威日志updates与method分类学.md` | updates 信封两流分类学与四要素定位（P0014 依据） |
| S021 | `S021-linux预备检测-指令集SIGILL问题类与检测阶梯.md` | AVX-512/AVX2 SIGILL 问题类、四级检测阶梯与 oma 探针落点（P0012 预备） |
| S022 | `S022-rust程序自带资源包的三路线与释放裁决.md` | include_bytes 对 rust-embed 对嵌入归档加释放；指纹目录口径（P0023 依据） |
| S023 | `S023-rmux在windows的进程树与原语实测.md` | 活体进程树加源码核实；三纠偏与原语表 |
| S024 | `S024-四环境部署自适应矩阵.md` | PATH bare 形态与 codex 字段所有权（P0027 依据）；状态栏 UTF-8、信任键双族共存 |
| S025 | `S025-四家状态栏配置矩阵与机读标记.md` | 四家状态栏命令驱动矩阵；oma 机读标记 agent:state 与会话闸；projKind zig/go/cpp cmap |
| S026 | `S026-grok与kimi的OAuth登录流.md` | 两家设备码流取证、凭据落盘与登录态纯文件检测 |
| S027 | `S027-提供商别名注入矩阵.md` | zhipu/deepseek x claude/codex 四格官方端点与注入形态；sops 托管密钥 |
| S028 | `S028-oma自更新机制.md` | releases/latest 查询、资产约定、Windows rename 舞步自替换、封版前 --git 主路径 |
| S029 | `S029-bypassPermissions会话层失效与命令面注入.md` | 模式取值顺序与 2.1.257 项目层忽略；oma 命令面 argv 注入与 respawn 口径；2026-09-15 追记 bypass 后残余阻塞分类学（读沙箱读块、ask 规则、灾难删除网、folder trust、MCP 双键、resume 不还原证伪旧记）与 hst doctor 候选面 |
| S030 | `S030-密钥hook安全拦截接管与同类Rust实现.md` | ohmypwsh secret-guard 语义、kingfisher/rtk 取证、oma 零依赖接管落点 |
| S031 | `S031-密钥一钥两密文存储与四shell懒注入接管.md` | ohmycloud D20 keystore 与 ohmypwsh 懒注入取证；oma secrets 子树设计 |
| S032 | `S032-GitHub发版与分支合并标准流程调研.md` | 三分支模型、合并三式、分支保护与发版自动化选型；本仓适用结论 |
| S033 | `S033-四家agent无头模式hook触发与状态栏渲染矩阵.md` | D17 前置：无头入口、hook 触发、状态栏渲染四行矩阵；状态栏无头四家均不可验，判据须两层分离 |
| S034 | `S034-状态栏HUD流行设置与多行context构成.md` | D36 前置：ccstatusline/CCometixLine 等流行 HUD 盘点、官方 stdin JSON 契约（context_window 与多行）、MCP/tools 计数数据源缺口与设计候选 |
| S035 | `S035-herdr与hst的hook注册共存实证.md` | 状态栏 unknown 兜底非冲突：herdr 安装器外来保留取证（装机 v9 matcher 星号对上游 v10 收窄）、hst merge 双向共存落地、跨侧注册搬家 shim-dead 与重跑 init 自愈 |

## 六、开发测试参考

> 位置 `docs\references\`；R 编号。

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| R001 | `R001-项目定位-Agent全平台部署配置工具.md` | 现役定位展开（D15：oma 去编排，只管诊断、配置、hook、状态栏、token） |
| R002 | `R002-常用命令与管理流程-从项目init到部署诊断.md` | oma 命令手册（命令面唯一权威） |
| R004 | `R004-测试标准细则-分层断言与门禁流程.md` | 测试分层、断言、闸门 |
| R005 | `R005-选型研究细则-cratesio与github双通道.md` | 选库检索双通道 |
| R006 | `R006-rmux开发参考-连接会话布局与驱动.md` | 退役（D15 移除 rmux 后端；留作历史参考） |
| R007 | `R007-agent信任与无阻塞参考-四家配置与检测.md` | 写 init/doctor/hook 时查 |
| R008 | `R008-项目工具Python库选型细则-pypi与uv.md` | py 工具选库与 uv 工作流 |
| R009 | `R009-项目工具PowerShell模块选型细则-psgallery与psresourceget.md` | ps 模块选型与 ohmypwsh 统一管理 |
| R010 | `R010-Windows到Linux交接清单.md` | 退役（D15 去编排后 Linux 欠账口径失效；留作历史参考） |
| R011 | `R011-Agent友好IO契约-format三态信封退出码与冻结面.md` | --format 三态、信封与双通道错误、退出码表、冻结面（issue #1） |

（R003 退役：原全量清单并入本索引，编号不复用。）

## 七、元规范

> 位置 `docs\guide\`；G 编号。

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| G001 | `G001-文档标准细则-命名写作规范与rumdl检查.md` | 命名与编号、写作、rumdl |
| G002 | `G002-研究标准细则-结构与六态标记.md` | 研究结构与六态 |
| G003 | `G003-工作流标准细则-从登记到归档五步.md` | 五步工作流与优先级 |
| G004 | `G004-经验沉淀细则-成功与错误经验分治.md` | proven=成功 plan 沉淀；references=实证做法与多次错误后升格的正确工作流；mistakes=纠偏（当场记、二犯升格） |
| G005 | `G005-中英文技术文档字符与标点硬禁令.md` | 四类禁用字符、豁免区、空格混排、标点结构、mdcharlint 校验（分期：存量清零前进门禁不强制） |
| 不编号 | `template.md` | 方案模板 |

## 八、错误速查

> 位置 `docs\mistakes\`；分类文件 M1xx，行级 M0xx。

| 编号 | 分类文件 | 覆盖关键词 | 行级编号段 |
| --- | --- | --- | --- |
| M101 | `M101-drive与paste错误.md` | send-keys、Enter、`C-c`、bracketed paste、marker 假阳性 | M001、M008、M027、M038-M039 |
| M102 | `M102-信任与hook配置错误.md` | 信任框、trust、pretrust、init、yolo、ENOENT、共享目录 | M002、M009-M011、M042、M044-M048、M051、M055-M061 |
| M103 | `M103-文档与命名错误.md` | 命名、显示名、CLI 名、六态、diary、标题规范、索引登记、长行编辑锚 | M003-M005、M013-M014、M030、M043、M064 |
| M104 | `M104-rmux安装与CLI调用错误.md` | 安装、`-V`、`-S`、`-L`、`cmd()`、`-t` 前缀匹配 | M006-M007、M016、M020、M029 |
| M105 | `M105-agent检测与状态判断错误.md` | PATH、which、idle、Quiet、CPU | M012、M018-M019、M040 |
| M106 | `M106-Windows进程与daemon启动错误.md` | os error 5、Job Object、WMI、exit-empty、pane cwd | M015、M017、M021-M022、M031、M041 |
| M107 | `M107-工具链与脚本错误.md` | sed、grep、PowerShell、中文路径、测试临时目录 | M023-M026、M028、M032-M037、M049-M050、M052-M055、M062-M063、M065 |

迭代规则：踩坑按当前最大号接编 MNNN 进对应分类文件（M0xx 行级、新分类用 M1xx 接编）；一行一事；同根因或同型坑**可合并聚合**进已有条目（保留最早编号与首踩日期，聚合后的正解写全），避免同型条目无限线性追加；反复踩落 `docs\research\`；改「正确处理」不删历史行；新分类文件登记本节。**分类文件新增行级条目时，本表该行「行级编号段」当轮同步延长，漏延长即登记债（见 M043）。**

## 九、阶段与版本

- `ROADMAP.md`：阶段路线
- `CHANGELOG.md`：版本里程碑
