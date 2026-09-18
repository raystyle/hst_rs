# HST

[![CI](https://github.com/raystyle/hst_rs/actions/workflows/dev-release.yml/badge.svg)](https://github.com/raystyle/hst_rs/actions/workflows/dev-release.yml)
[![Release](https://img.shields.io/github/v/release/raystyle/hst_rs)](https://github.com/raystyle/hst_rs/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## 项目介绍

HST（Hooks, Statusline, Trace）是 agent 全平台部署配置与诊断 CLI：给 Claude Code、Codex、Grok、Kimi 四家 agent 做 hook 状态落盘、状态栏配置、只读对话 trace、可用性诊断与 yolo 无阻塞分级，Windows（含 WSL）/Linux/macOS 同一命令面。为在多机多 agent 环境里治理部署形态与排查「这文件谁改的、agent 配置为什么没生效」这类问题而生。

仓间分工：agent 二进制安装归 `ark`（`ark install claude`），舰队分发运维归 `omc`；hst 只管单机部署配置与诊断，不编排、不注入 token。与 Hipo 的 hst history picker 共存（本工具装用户目录，不覆盖 `/usr/bin/hst`）。

特性一览：

- 四家 agent 的 hook 注册与状态落盘（shim 自包含，零 hst 二进制依赖）加无头验收
- 状态栏幂等配置（四家各自配置面；支持自备脚本整替换与还原）
- 只读对话 trace 六视图（会话、时间线、四域检索、单文件轨迹、操作块、agent 面）
- 零网络只读体检（doctor）与活性诊断（diagnose，打真 API 烧最小 token）分家
- yolo 无阻塞分级 full|partial|off（用户级与项目级两级显式，一键清干扰键）
- kv|json|jsonl 三态输出加 `--filter-output` 信封键路径过滤（agent 友好）
- `--llms` 紧凑手册随活命令树自适应渲染，配 `--json` 出机器形态

## 部署

```powershell
# ark 装（推荐）
ark install hst

# 或独立分发域直下（三平台预编译，hst.ohmygh.com 已通；镜像腿 env.ohmygh.com 同形）
# Windows：https://hst.ohmygh.com/hst/stable/hst-x86_64-pc-windows-gnu.zip 解压进 PATH
# Linux/WSL：https://hst.ohmygh.com/hst/stable/hst-x86_64-unknown-linux-gnu.tar.gz
# macOS：https://hst.ohmygh.com/hst/stable/hst-aarch64-apple-darwin.tar.gz
```

二进制校验：GitHub Release 逐包带 `.sha256` 边车，`sha256sum -c hst-x86_64-unknown-linux-gnu.tar.gz.sha256`。

五端注意：状态栏运行时是 pwsh 7（缺了只是不渲染，不影响其它命令）；`~/.local/bin` 不在非登录 ssh 的默认 PATH（无头调用前 `export PATH="$HOME/.local/bin:$PATH"`）；Windows 侧勿用 powershell.exe 5.1 跑仓内脚本。

自升级通道：`hst self update`（dev 滚动源）、`hst self update --stable`（正式版；缺省镜像优先，失败自动回退 GitHub 官方）、`hst self update --git`（源码装）。

## 配置

环境变量（都可缺省）：

| 变量 | 作用 | 缺省 |
| --- | --- | --- |
| `HST_MIRROR` | 自升级镜像基址 | 未设 = 镜像优先（`https://env.ohmygh.com`，失败回退 GitHub）；设值 = 基址覆盖；空串 = 镜像全关 |
| `HST_GATEWAY_URL` / `HST_GATEWAY_KEY` | 活性诊断（diagnose）网关指向与凭据 | 未设（diagnose cache 才消费） |
| `HST_ISSUES_API` | issue 入口基址覆盖 | `https://issues.ohmygh.com` |
| `HST_AGENT_PATH` / `HST_<AGENT>_BIN` | agent 检测自定义路径 | PATH 与默认目录探测 |

配置文件位：hst 自管数据根 `~/.hst/`（hooks shim、statusline 脚本、state、selfupdate 记录）；状态栏定制模板 `hst statusline --example` 打印（落 `~/.hst/statusline.toml`）；四家 agent 的 hook 注册与 yolo 键写各家用户级配置（`~/.claude/settings.json` 等），AGENTS/CLAUDE 说明落项目目录。

密钥纪律：凭据只经环境变量与 agent 侧配置传递，不落 hst 输出与仓内明文；secretguard 在 hook 面做密钥拦截（block 级 exit 2 拒调用）。

## 使用方法

```bash
hst init                     # 全套部署（幂等）：用户级 yolo 键加四家 hook 加状态栏
hst doctor                   # 零网络只读体检（有 block 才退 1）
hst init --yolo=partial      # 分级：编辑自动过，危险操作仍确认
hst init --clear-project-yolo  # 清项目级对用户级 yolo 的干扰键
hst trace file src/main.rs   # 单文件轨迹：谁、何时、基于什么意图改的
hst statusline --example     # 状态栏定制模板
hst diagnose cache           # 网关缓存命中矩阵（打真 API 烧最小 token）
hst --json --filter-output blocked doctor   # 信封只留 blocked 键（agent 面）
```

发现缺陷一键反馈：`hst issue new "<标题>" --body "<细节>"`（自动带版本加平台加主机，issues.ohmygh.com 集中管理；`hst issue list` / `hst issue show <id>` 读面）。

agent 手册面：`hst --llms`（紧凑 markdown 手册，随活命令树自适应）；`hst --llms --json`（机器形态）。完整选项与子命令不内联本页，以 `hst --help` 与 `hst --llms` 为准。

## 许可

MIT，见 [LICENSE](LICENSE)。
