# HST

> HST（Hooks, Statusline, Trace）：Agent 全平台部署配置与诊断工具，专注五功能（agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置）。公开契约以类型签名与 `///` 契约注释为准。本项目会话由 hst 自治理（hook 与状态栏经 `hst init` 部署，agent 状态写用户级 `~/.hst/state/`）。

## Commands

- `cargo test --locked` 全测试（单元加集成）
- `cargo fmt --all -- --check` 加 `cargo clippy --locked` 提交前必跑
- `cargo aidoc` 后提交 `docs/aidoc/`（改 pub 项或 /// 后先重生成再提交；`cargo aidoc --check --strict` 漂移门禁）
- `uv run --script .tools/md-ref-scan.py` 加 `.tools/md-heading-scan.py` 加 `.tools/mdcharlint.py` 加 `rumdl check .` md 四门禁
- `cargo build --release --locked --target x86_64-pc-windows-gnu --bins` Windows 交叉构建
- `PEVO_CHECK_ALLOW` 加 `uv run /mnt/wsl/repos/project-evo/plugins/evo-adr/skills/code-kit/scripts/check.py .` 骨架合规自检（标准命令见 docs/README 末节）
- `./target/debug/hst doctor` 本机体检；`./target/debug/hst init` dogfood 部署
- `bash .tools/cross-test.sh` 全平台实弹矩阵（5端4机，细则见 guides/环境事实）
- `pwsh -NoProfile -File .tools/release.ps1 [-DryRun]` 本地发布链（ADR-0007 三段式：版本闸加测试闸加本地交叉与 mac 实机构建加打包冒烟加 gh 直发 --latest）
- `///` 契约注释覆盖公开项（missing_docs = deny，CI 必红）；doctest 走集成测试 tests/ 承载

## Must

- 改命令面同步：clap 注释加集成测试加 `cargo aidoc` 重生（`hst --llms` 手册随命令树自适应，无生成物落盘，ADR-0005）
- 不可逆技术选择先立 `docs/adr/`；新需求先立 `docs/requirements/` REQ
- 事实性断言标六态（定义见 docs/README 写作规范节）；实证滥用即未完成
- 踩坑当场记 diary 当天笔（同型二犯升格 ADR 或 guides 工作流）
- self update 与部署改动经 herdr 会话知会 ohmycloud（跨仓周知不发 issue）
- 遇缺陷即 `hst issue new "<标题>" --body <细节>` 一键反馈（issues.ohmygh.com 统一入口，自动带版本加平台加主机，REQ-010/总台 REQ-057 契约）
- 每次提交 diary 当天记钩子；一事一提交（feat/docs/fix/chore 前缀）
- 版本载体唯一权威 Cargo.toml（semver 判据见 guides/环境事实）

## Must not

- 手改生成物（状态栏脚本、docs/aidoc 生成面）
- 另写第二真相（命令契约在 clap 与 tests，标记契约在 fmtio `///`；references 老文档层已清退，ADR-0006）
- emoji、破折号、Unicode 箭头、非法全角（四类禁字，mdcharlint 与 rumdl 门禁，豁免区外零容忍）
- 未经指示推远端；编排面与 token 注入面回流（ADR-0001）
- Windows 侧默认 powershell.exe 5.1；sed 批改中文与反斜杠路径

## Read first

- 检索阶梯（渐进披露，不整读）：`hst --llm`（CLI 手册投影，REQ-060 标准面）到 `docs/aidoc/llms.txt`（库投影）到模块 `///` 与源码；ADR 仅在改对应决策时读
- 文档地图：`docs/README.md`；需求队列：`docs/requirements/README.md`；平台矩阵与连接姿势：`docs/guides/环境事实.md`

## 环境

- 三平台矩阵（CI 三岗含 gnu 交叉岗）；本仓唯一工作根 WSL `~/repos/hst_rs`；跨仓路径、5端4机矩阵与分发体系长事实见 `docs/guides/环境事实.md`
- 评审闸门惯例：交付批经 herdr 右侧 codex 会话对齐后推 main
