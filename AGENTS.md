# HST

> HST（Hooks, Statusline, Trace）：Agent 全平台部署配置与诊断工具，专注五功能（agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置）。公开契约以类型签名与 `///` 契约注释为准。本项目会话由 hst 自治理（hook 与状态栏经 `hst init` 部署，agent 状态写用户级 `~/.hst/state/`）。

## Commands

- `cargo test --locked` 全测试（单元加集成）
- `cargo fmt --all -- --check` 加 `cargo clippy --locked` 提交前必跑
- `uv run --script .tools/md-ref-scan.py` 断链回归（结构变更必跑）
- `uv run --script .tools/md-heading-scan.py` 加 `uv run --script .tools/mdcharlint.py` 加 `rumdl check .` md 门禁
- `cargo build --release --locked --target x86_64-pc-windows-gnu --bins` Windows 交叉构建
- `PEVO_CHECK_ALLOW` 加 `uv run /mnt/d/ProjectEvo/plugins/project-evo/skills/dev-evo/scripts/check.py .` 骨架合规自检（豁免正则与标准命令见 docs/README 存量禁字债口径节；正则含 `^docs/aidoc/` 在册）
- `./target/debug/hst doctor` 本机体检；`./target/debug/hst init` dogfood 部署
- `pwsh -NoProfile -File <脚本>` 验收与运维脚本统一载体（五端 pwsh 7.6.6；非登录 shell PATH 兜底；仓内既有 bash 载体 cross-test.sh 属存量不强制迁移，新增验收运维面一律 pwsh，dev-evo env-platform 第十一节）
- `bash .tools/cross-test.sh` 全平台实弹矩阵，**5端4机**基建原语（wsl 加 lan-win 加 lan-mac 加 lan-ubuntu 加 lan-linux 五端；wsl 与 lan-win 同宿主机，2026-09-16 用户定）：wsl 就地跑、windows-gnu 本地交叉构建、mac 取 dev release 资产、lan 三端 mesh、lan-win 走宿主 127.0.0.1 回环（REQ-007）
- `cargo aidoc` 后提交 `docs/aidoc/`（改 pub 项或文档后先重生成再提交；`cargo aidoc --check --strict` 漂移门禁）
- `///` 契约注释覆盖公开项（missing_docs = deny，CI 必红）；doctest 走集成测试 tests/ 承载（dev-evo 第五十九批 ADR-0006：Rust 栈 aidoc 投影强制，bin-only 不豁免，受众是维护者与 agent）

## Must

- 改命令面两处同步：R002 行、docs README（agent 手册 `hst --llms` 随命令树自适应渲染，无生成物落盘，ADR-0005）
- 不可逆技术选择先立 `docs/adr/`；新需求先立 `docs/requirements/` REQ
- 事实性断言标六态（`[实证]` 至 `[直觉]`，规范见 G002）；实证滥用即未完成
- 踩坑当场记 `docs/mistakes/`（同根因聚合）；同型二犯升格 ADR 或 references
- self update 与部署改动经 herdr 会话知会 ohmycloud（跨仓周知不发 issue）
- 每次提交 diary 当天记钩子；一事一提交（feat/docs/fix/chore 前缀）

## Must not

- 手改生成物（状态栏脚本、docs/aidoc 生成面）
- 另写第二真相（命令细则唯一权威在 R002，标记契约在 S025/R011）
- emoji、破折号、Unicode 箭头、非法全角（G005 四类禁字，豁免区外零容忍）
- 未经指示推远端；编排面与 token 注入面回流（ADR-0001）
- Windows 侧默认 powershell.exe 5.1；sed 批改中文与反斜杠路径

## Read first

- 命令行为细则：`docs/references/R002`（唯一权威）到 `R007`（怎么做）到 `docs/research/S00x`（为什么）
- 文档地图：`docs/README.md`（全仓索引，承接旧 INDEX 职责）；历史定位与方案：`docs/adr/` 与 `docs/proven/`
- 代码定位：先 docs/README 代码表再 `ast-grep` 符号；抽文档节用 mq
- 需求与队列：`docs/requirements/README.md`；旧四段协作规则全文：`docs/guides/agents-legacy-four-sections.md`

## 环境

- 三平台矩阵（Windows/Linux/macOS，CI 三岗含 gnu 交叉岗）；本仓双侧开发（Windows `D:\hst_rs` 加 WSL `~/repos/hst_rs`）
- 评审闸门惯例：交付批经 herdr 右侧 codex 会话对齐后推 main
- 跨仓：ark-rs `D:\ohmyenv-rs`、ohmycloud `D:\ohmycloud`、本地 dev-evo 指导仓 `D:\ProjectEvo`
- WSL 上 Windows 命令用 pwsh 7；md 与 Rust 源 UTF-8；需兼容 PS 5.1 的脚本带 UTF-8 BOM
- 连接姿势：WSL 到宿主恒走 127.0.0.1 回环（ssh 加 interop 直调，不走宿主 mesh IP）；lan 三端（mac 与 ubuntu 与 linux）mesh 随时随地；口径全文见 dev-evo env-platform 第十节
- 版本载体唯一权威：Cargo.toml（semver：修复与文档批 patch、能力与行为变化 minor、契约破裂 major；载体外版本号即第二真相，dev-evo flow-release 第七节）
- 分发体系：omc 管分发运维、ark 管落地执行验收；本仓自升级三通道在册（GitHub 直连、镜像腿带缺省回退、--git 源码，S028/D48）
