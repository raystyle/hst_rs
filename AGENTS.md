# HST

> HST（Hooks, Statusline, Trace）：Agent 全平台部署配置与诊断工具，专注五功能（agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置）。公开契约以类型签名与 `///` 契约注释为准。本项目会话由 hst 自治理（hook 与状态栏经 `hst init` 部署，agent 状态写用户级 `~/.hst/state/`）。

## Commands

- `cargo test --locked` 全测试（单元加集成）
- `cargo fmt --all -- --check` 加 `cargo clippy --locked` 提交前必跑
- `uv run --script .tools/md-ref-scan.py` 断链回归（结构变更必跑）
- `uv run --script .tools/md-heading-scan.py` 加 `uv run --script .tools/mdcharlint.py` 加 `rumdl check .` md 门禁
- `cargo build --release --locked --target x86_64-pc-windows-gnu --bins` Windows 交叉构建
- `PEVO_CHECK_ALLOW` 加 `uv run /mnt/d/ProjectEvo/plugins/project-evo/skills/dev-evo/scripts/check.py .` 骨架合规自检（豁免正则与标准命令见 docs/README 存量禁字债口径节）
- `./target/debug/hst doctor` 本机体检；`./target/debug/hst init` dogfood 部署
- 裁定：本仓 bin-only 无公开库面，cargo test --doc 与 missing_docs 不适用（示例与行为锁走集成测试 tests/ 承载；对齐 dev-evo base-projection「无自有 API 面项目」范式：公开契约 = 生成物字节确定性，regenerate-and-diff 由 init_rerun_is_byte_idempotent 等测试钉）

## Must

- 改命令面四处同步：R002 行、重跑 `hst init` 重生 SKILL、COMMAND_MAP、INDEX/docs README
- 不可逆技术选择先立 `docs/adr/`；新需求先立 `docs/requirements/` REQ
- 事实性断言标六态（`[实证]` 至 `[直觉]`，规范见 G002）；实证滥用即未完成
- 踩坑当场记 `docs/mistakes/`（同根因聚合）；同型二犯升格 ADR 或 references
- self update 与部署改动经 herdr 会话知会 ohmycloud（跨仓周知不发 issue）
- 每次提交 diary 当天记钩子；一事一提交（feat/docs/fix/chore 前缀）

## Must not

- 手改生成物（SKILL.md、状态栏脚本、COMMAND_MAP 生成面）
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

- 三平台矩阵（Windows/Linux/macOS，CI 三岗含 gnu 交叉岗）；本仓双侧开发（Windows `D:\hst_rs` 加 WSL `/home/ray/hst_rs`）
- 评审闸门惯例：交付批经 herdr 右侧 codex 会话对齐后推 main
- 跨仓：ark-rs `D:\ohmyenv-rs`、ohmycloud `D:\ohmycloud`、本地 dev-evo 指导仓 `D:\ProjectEvo`
- WSL 上 Windows 命令用 pwsh 7；md 与 Rust 源 UTF-8；需兼容 PS 5.1 的脚本带 UTF-8 BOM
