# AGENTS.md

本文件是协作规则的**最高约束**，四段职责依次为：**项目定位**、**工作规则**、**意图路由**、**资源索引**。

> 归档注记（2026-09-16）：本文件是旧四段协作规则留档，现行命令面以 R002 为准；`hst skill` 已随 ADR-0005 退役（`hst --llms` 手册面承接），下文 skill 相关行仅作历史记录。

## 一、项目定位

> 本项目的本质与边界。根为定位，下分本质、边界、管理对象、方案索引。

1. **本质**
   - HST（Hooks, Statusline, Trace，原 Oh My Agents，v0.6.0 更名过渡期，D29；版本线自 1.0.0 重开不接 oma 老线，D34）：Agent 全平台部署配置与诊断工具，专注五个功能（D20，2026-09-09 用户裁定）：agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置；不做编排（D15），不管 token 环境变量注入（D20 删除 secrets 与 providers 面，密钥安全归 ohmypwsh）；对话 trace 直读原生会话库，与 rmux 零耦合（D19）。

2. **边界**
   - 配置钉在启动的项目目录；不替代 ohmypwsh 五端环境总台，不替代各 agent 本体。
   - 不做编排（D15，2026-09-08）：spawn / send / status / serve / mcp 等编排命令与 rmux 运行时后端整体移除；原编排定位归档 P0004，本裁定归档 P0035。trace 初随 D15 连坐删除，D19 以只读检索面恢复（六视图全量）。
   - 四仓分工（2026-09-02 定调，D07 修正 agent 二进制归属，D09 钉种子不归 oma，D15 去编排，D20 去 token 注入，细目见 R001 四仓生态节）：ark-rs（原 ohmyenv-rs，`ark`，2026-09-12 三仓周知更名 Ark / Agent Runtime Kit，首版 1.0.0，镜像开 env.ohmygh.com/ark/ 段、ome/ 段兼容期保留；本地路径仍 `D:\ohmyenv-rs`）管工具、运行时依赖与 agent 二进制下装部署、本仓（`hst`）管诊断、hook、状态栏、trace 与 yolo、ohmypwsh 管五端总台与密钥安全（token 注入归此）、ohmycloud 管云端二进制分发与镜像种子；跨仓协作互相发 issue。
   - oma 不管种子、不管 agent 二进制下装（D09，2026-09-07）、不管 token 注入（D20，2026-09-09）：只管可用性诊断、hook、状态栏、trace 与 yolo。下载、安装、部署（五端）归 ark-rs：`ark install`（更名前 `ome install`，兼容期 ome 命令与镜像段保留）幂等检测安装（已装任何来源即跳过，存量原地纳管），数据权威 ome `catalog\tools.toml` agent 四节（D07 方向反转 2026-09-05，ohmyagents#5）。本仓 `hst agents install` / `update` / `secrets` / `providers` / `login` 已随 D20 整体移除（历史口径见 R002 与 P0040）。hst doctor 的登录态 / hook 形态 / 状态栏三类检查归 agents 域（二进制在位与版本归 ome doctor）。
   - 三活仓本地路径（用户裁 2026-09-10 登记，重叠功能互相 review 时直读对方仓代码）：ohmycloud = `D:\ohmycloud`、ark-rs（原 ohmyenv-rs）= `D:\ohmyenv-rs`、hst-rs（原 ohmyagents-rs，本地目录 2026-09-12 随 D30 更名）= `D:\hst_rs`（本仓）；review 重叠面（如 diagnose 与 omc agent doctor 分工）直接读 `D:\ohmycloud\src\` 对应文件。
   - hook、skill 部署只落启动目录；hook 注册与状态通道常驻用户级（D28：注册面四家用户层、shim 与 session 分键状态在 `~/.hst/`（D29 更名，旧 `~/.oma` 首启自动迁移），未 init 项目也有状态数据）；hst 自管应用数据根是 `~/.hst`（D29；旧 `~/.oma` 与 `~/.ohmyagents` 仅旧在则改名迁过去。agent 安装与本地 pin，P0012；D07 后安装域迁 ome，此根承载存量安装与配置数据），yolo 与 pretrust 键的写入面见 R002。

3. **管理对象**
   - 可注册的终端 agent（当前默认 claude / codex / grok / kimi，可扩展）。
   - 目标项目目录（cwd 或 `--project`）。

4. **方案索引**
   - 需求入口：`PRD.md`（新需求先入 PRD 走追问链，禁止静默假设）。
   - 定位：`docs\references\R001-项目定位-Agent全平台部署配置工具.md`
   - 定位变更：`docs\proven\P0035-项目重新定位-Agent全平台部署配置工具.md`；上一版 `docs\proven\P0004-项目重新定位-通用智能体多路复用任务编排器.md`
   - 首期切面：`docs\proven\P0001-四路会话工具-CLI控制面与网页观察面.md`
   - 研究：`docs\research\`（文件名即标题，按关键词搜）

## 二、工作规则

> 四类场景：**对话**（何时做什么）、**操作**（过程纪律）、**编码**（写什么按什么标准）、**文档**（写文档与脚本的标准）。每类下分可以与禁止；节末文档对齐义务表是各动作的对账底线。产品与 rmux 的行为约束不在此层，见意图路由与 `docs\research\`。

### 对话

1. **每轮对话**
   - 可以：先核对四原语 `PRD.md`、`GOAL.md`、`TODO.md`、`PLAN.md`；实质推进当场更新 todo 与 plan。
   - 禁止：不核对四原语就干活；偏离当前目标；推进了不更新 todo/plan。

2. **需求驱动**
   - 可以：新需求先入 `PRD.md`（状态新需求，走追问链澄清，禁止静默假设）；澄清采纳后立项，GOAL 锚点回指 D 编号。
   - 禁止：静默假设需求；GOAL 目标无需求来源。

### 操作

3. **踩坑时**
   - 可以：当场按当前最大号接编 MNNN，落 `docs\mistakes\` 对应分类文件一行（文件名即错误主题，分类表见 `INDEX.md`）；同根因或同型坑合并聚合进已有条目（保留最早编号与首踩日期），不必每踩必新增；主题深挖落 `docs\research\`。
   - 禁止：只留在对话里反复试错。

4. **发现问题时**
   - 可以：任何问题都参照现有文档逻辑、结构与 `INDEX.md` 自修正，走五步闭环（循环自迭代）：**定位**：先搜 INDEX 与相关文档，确认是否已有规则、研究或参考覆盖；**归类**：文档错修文档、规则缺补规则（AGENTS 或对应细则）、知识缺落研究（六态）、出错模式记 mistakes、验证过的做法沉淀 references；**修正**：改在源头，下游引用、索引与四原语同步；**验证**：`rumdl check .` 加 `.tools` 三个 md 扫描（`md-ref-scan.py` 断链、`md-heading-scan.py` 标题括号、`mdcharlint.py` 四类禁用字符封闭清单门禁），涉及结构再对账 INDEX 与磁盘；**提交**：一事一提交，diary 记钩子。
   - 禁止：跳过定位直接改（重复造已有规则）；只修表象不回写体系；问题只留在对话或记忆里；修完不跑验证；把临时补丁当最终方案不归档。

5. **交付变更时**
   - 可以：改代码同步对应文档，改文档同步索引与 `docs\diary\`；遵守命名标准；技术文档按文档标准细则写。
   - 禁止：只改代码不落文档；改了文档不更新索引。

6. **经验沉淀时（强规则，G004）**
   - 可以：成功的 plan 沉淀归 `docs\proven\`（方案与过程）；研究被实证后的做法与多次错误后沉淀成的正确工作流进 `docs\references\` 并挂意图路由或 R002。
   - 错误经验踩坑当场记 `docs\mistakes\`（同根因聚合）；同型坑二犯以上把正确处理升格成 references 工作流并互指。
   - 禁止：`[经验]` 断言只留在研究文档不落 references（检索不到等于没沉淀）；错误只记现象不记根因与处理；`[推断]`/`[假设]` 跳级进 references；一条知识两个权威落位互相重复。

7. **提交时**
   - 可以：`feat:` / `docs:` / `fix:` / `chore:` 前缀加中文描述；一次提交只做一件事。
   - 禁止：多事混一提交；未经指示推远端。

### 编码

8. **执行命令与写文件时**
   - 可以：Windows 命令用 PowerShell 7（`pwsh`），Linux / macOS / WSL 用该平台常规 shell；Markdown / Rust 源码 UTF-8；Windows 上需兼容 5.1 的脚本用 UTF-8 BOM。
   - 禁止：Windows 上默认用 `powershell.exe` 5.1；无 BOM 的中文 ps1 给 5.1 读。

9. **写 Rust 时**
   - 可以：先查 crates.io / docs.rs / GitHub 上是否已有最流行、最稳定、或已经覆盖本需求的库，检索走双通道细则 `docs\references\R005-选型研究细则-cratesio与github双通道.md`（crates.io 稳度四信号加 gh 流行活跃分辨，结论附证据）；选定后用最少代码接上，优先组合而不是自写协议、解压、HTTP、哈希、CLI 解析。
   - 禁止：在现成库已能稳定完成的前提下从零实现；为风格引入冷门或实验 crate；一次拉一堆用不上的依赖。

10. **写测试时**
    - 可以：遵守 `docs\references\R004-测试标准细则-分层断言与门禁流程.md`。分层按官方三层（单元 `#[cfg(test)]`、集成 `tests\*.rs`、doctest），集成优先于单元；意图对应方法（冒烟断退出码、回归用黄金文件、验收对照 oracle）；测试名写成可读规格，负例带 `dies_` 前缀；期望值必须来自独立来源（规范、黄金文件、属性），断言只写稳定字段（标记行、退出码），放过 pid 与时间戳；测试体用 `TestResult` 加 `?` 传播错误；rmux 依赖的测试按闸门 skip；测试设施收 `test-util` feature。
    - 禁止：重言式断言（期望值来自被测同款逻辑或镜像实现分支，AI 生成测试高发）；公开 API 测试塞 `mod tests{}` 不进 `tests\`；默认 mock（oma 拿真 daemon）；计时进断言；测试设施无 feature gate 进生产构建；只测 happy path。

11. **写临时脚本时**
    - 可以：按需自定义的 ps1 / py / Rust 工具，有复用价值即归档 `.tools\`（规则与清单见 `.tools\README.md`；Python 带 PEP 723 头，用 `uv run --script` 运行，py 选库走 `docs\references\R008-项目工具Python库选型细则-pypi与uv.md`；PowerShell 选模块走 `docs\references\R009`）；文档结构大改（改名、编号、移目录）后跑 `uv run --script .tools\md-ref-scan.py` 做断链回归。
    - 禁止：可复用脚本散落仓库根或只留在对话里；把 `pypi.org/search` 或抓网页当可编程选型接口；用 sed 批改中文与反斜杠路径（用 `md-replace.py`，见 M023）；归档不带自述与用法。

### 文档

12. **写文档时**
    - 可以：遵守 `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`（树形、标题干净、文件名即标题、rumdl）加 `docs\guide\G005-中英文技术文档字符与标点硬禁令.md`（四类禁用字符：破折号、箭头、emoji、非法全角；豁免区与替代写法；D10 后封闭清单已删，新文件强制合规）。
    - 禁止：标题带括号、口号或破折号（解释放标题下一行引用 `>`）；整段混杂不成树；豁免区外出现 G005 四类禁字。

13. **写研究与测试文档时**
    - 可以：事实性断言必须标六态之一：`[实证]`（本机实测）、`[推断]`（逻辑推出）、`[经验]`（历史惯例）、`[记忆]`（待复核）、`[假设]`（待验证）、`[直觉]`（主观倾向）；标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`；研究与测试的结论断言不标六态即视为未完成。
    - 禁止：把「没验证」写成「已验证」（实证滥用）；断言不标六态；用猜测冒充结论。

### 文档对齐义务表

> 对齐义务：什么动作后、到什么状态、必须对齐撰写哪些文档。漏对齐即登记债。

| 动作 | 状态时机 | 文档义务 |
| --- | --- | --- |
| 每轮对话 | 每轮开工前 | 核对四原语（PRD / GOAL / PLAN / TODO）；实质推进当场更新 TODO 与 PLAN |
| 新需求提出 | 提出时 | PRD 登记新需求行（状态新需求，走追问链） |
| 追问链澄清 | 澄清完成 | PRD 状态流转加澄清轮次与裁定 |
| 目标立项 | 开工前 | GOAL 起点与锚点（回指 D 编号）、PLAN 方案、TODO 清单 |
| 选型与调研 | 研究完成 | S 文档（六态）加 INDEX 研究节 |
| 交付变更 | 改动完成 | 改代码同步对应文档；命令面变化四处同步（COMMAND_MAP 加行、重跑 `hst init` 重生 SKILL、R002 加行、本文件三节加行）；INDEX 代码表同步 |
| 写测试 | 新层或新面 | R004 落点同步、INDEX 代码表 tests 行 |
| 写脚本 | 归档时 | `.tools\README.md` 清单行、INDEX 代码表 .tools 行 |
| 踩坑 | 当场 | `docs\mistakes\` 对应分类文件接编一行；INDEX 错误速查节行级编号段当轮同步延长 |
| 发现问题 | 当场 | 五步闭环（定位 / 归类 / 修正 / 验证 / 提交），改在源头、下游与四原语同步 |
| 经验沉淀（强规则，G004） | 验收后 | proven 归档（P 编号）；`[实证]` 做法升 references 并挂意图路由或 R002；mistakes 同型二犯升格 |
| 方案达成 | 验收全绿 | proven 归档、GOAL 历史行、INDEX 归档节、TODO 起新清单 |
| 每次提交 | 提交后 | diary 当天记钩子 |
| 版本级成果 / 文档结构变更 | 阶段完成 / 改名移目录后 | CHANGELOG 里程碑与 ROADMAP 阶段状态；INDEX 同步且 `md-ref-scan.py` 断链回归必跑；正式 release 后经 herdr 会话知会 ohmycloud 更新 tool status 镜像锚（跨仓周知一律走 herdr 会话同步、不发 issue，用户裁 2026-09-10） |

## 三、意图路由

> 需求意图到命令的映射（摘要层）。每条命令的行为细则、机理出处、marker 行、退出码与落地状态的唯一权威见 `docs\references\R002-常用命令与管理流程-从项目init到部署诊断.md`。

- **无阻塞诊断**：`hst doctor`（只读体检；登录态、hook 形态、状态栏三类归 agents 域，D07；warn 与 block 分层，block 才退出 1）
- **检测已装 agent**：`hst agents`（PATH / 环境变量 / hst 自管根 / 默认目录四源；缺装 hint 指向 `ark install`，D20）
- **配置状态栏**：`hst statusline [名] [--example]`（四家写入面幂等；用户级定制 `~/.hst/statusline.toml`：段落开关加模板图标加 codex 子集，`--script` 整脚本替换加 `--builtin` 还原，D18）
- **hook 写状态加密钥拦截**：`hst hook`（状态落盘；block 级密钥 exit 2 拒调用）
- **部署项目全套**：`hst init [--project PATH]`（yolo 加 hook/skill 加状态栏面，幂等，D53 状态栏脚本随 init 刷新（marker 保护自备）；hook 注册与 shim 常驻用户级 `~/.hst/hooks/`，状态按 session 分键写 `~/.hst/state/`，项目旧注册自动退役，零 hst 依赖可无痛轮换，D27/D28）
- **部署 yolo 与非阻塞键**：`hst init --yolo[=full|partial|off]`（用户级）或 `hst init --project-yolo[=级别]`（项目级，两级显式互斥，D28 第 3 轮；D33 分级：full 全 bypass、partial 危险操作仍确认、off 摘 hst 落键，级别缺省 full）；`--pre-trust` 追加按项目信任库预写
- **oma 自更新**：`hst self update [--stable] [--git]`（缺省 dev 滚动源，Windows rename 舞步；设 `HST_MIRROR=<基址>` 走镜像 dev 段（D45 起旧 OMA_* env 兼容读已删），边车 sha256 判新，网络失败回落 GitHub，D16）
- **生成 hst 自身技能**：`hst skill [--write]`（从 clap 活命令树自适应渲染 SKILL.md，新命令自动出现；--write 落用户级 ~/.claude/skills/hst/；D49 起技能名翻 hst 唯一名，旧牌 ohmyagents 目录由 skill --write 与 init 幂等退役（ours 识别，用户手改跳过），翻案 D14/D22/D45 旧牌保留裁定）
- **生成补全**：`hst completions <shell>`
- **检索对话历史**：`hst trace sessions|timeline|blocks|agent|file|search`（六视图联邦读四家原生会话库，只读，D19 恢复）
- **无头验收 agent**：`hst agents verify [名] [--timeout N]`（状态栏 mock 直跑加 hook 无头落盘两层判据，D17/S033；skip 不计败，fail 退出 1）
- **活性诊断**：`hst diagnose cache [别名...]` 加 `hst diagnose agents`（网关缓存命中矩阵加配置指向加别名在册加 key 活性加 thinking 对照，D21；打真 API 烧最小 token，与 doctor 的零网络体检分家）
- **输出格式契约**：全局 `--format kv|json|jsonl` 加 `--json` 简写（信封冻结面见 R011）

编排命令（check / spawn / respawn / status / send / key / run / task / settle / cleanup / REPL / web / serve / mcp）已随 D15 移除；token 注入面（agents secrets / providers / login）与 install / update 兼容层已随 D20 移除；历史口径见归档 P0001 至 P0034 与 P0040。新想法走 G003 五步再立项，禁止把未验收口径写成已可跑。

## 四、资源索引

> 定位看 `INDEX.md`（项目根目录，唯一索引：编号表、目录结构、代码文件位置）。本节是**配合 INDEX 的搜索与分析方法**。

**速记**：前缀定位 `D`（PRD 需求）/ `P`（proven 归档）/ `S`（research 研究）/ `R`（references 开发测试参考）/ `G`（guide 元规范）/ `M`（mistakes 错误；文件 M1xx、行级 M0xx）；根目录四原语 `PRD` / `GOAL` / `PLAN` / `TODO`。

**外部数据源**：`gh` / `git`（源码、issue 与同类仓检索取证）；`browser-harness`（浏览器搜索与任意网站抓取，ark-rs 安装）；`reader`（本地文档与电子书读取，ark-rs 安装）。

**查文档**：先搜 `INDEX.md` 定位编号，再读文件。

**项目工具**：`.tools\`（自定义脚本归档；Python 用 `uv run --script .tools\<名>.py`，清单见 `.tools\README.md`；py 选库细则 `docs\references\R008`）；文档验证三件套：断链回归 `md-ref-scan.py`、标题括号 `md-heading-scan.py`、`rumdl check .`。

**搜索方法（文档）**：

```powershell
rg -n "关键词" INDEX.md                        # 1 先搜总索引，定位编号或文件
rg --files docs | rg 关键词                     # 2 按文件名搜文档
rg -n "关键词" docs\research docs\references    # 3 全文搜研究参考
rg -n "关键词" docs\mistakes\                   # 4 搜错误处理

# mq（markdown 结构查询，D:\ohmyenv\mq\mq.exe，jq 风格；section 模块必须 -A）
mq -F grep '.h2' docs\research\*.md             # 跨文件按节标题定位（文件:行号:标题）
mq -A 'section::section(., "关键结论")' 文档     # 抽整节内容（含正文）
mq -A -F json '.h1' 文档                        # 结构化 JSON（类型/深度/位置）
```

**搜索方法（代码）**：

```powershell
ast-grep outline -l rs --json src\              # 模块符号表（INDEX 代码表配符号清单）
ast-grep run -p 'pub fn name($$$) $$$' -l rs    # 按名定位定义，免疫注释与调用行
ast-grep run -p 'fn $NAME($$$) -> Result<$RET, String> $$$' -l rs --json  # 签名表
```

坑速查：mq 的 `.h.1` 是层级值不是文本（节点用 `.h`/`.h1`）；无 `.s` 选择器（用 section 模块）；ast-grep 的 fn 模式必须带 body 通配 `$$$`、可见性要写进模式、JSON 变量取 `metaVariables.single.<VAR>.text`。详见 M107。

**分析路径**：改产品行为先读 `docs\references\R007`（怎么做）再回 `docs\research\S00x`（为什么）；踩坑查 `docs\mistakes\M1xx`；写码选库走 R005；测试规范 R004；新想法走 G003 五步；定位代码先 INDEX 模块表再 ast-grep 符号；抽文档节用 mq section。
