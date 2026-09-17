# HST

HST（Hooks, Statusline, Trace）：Claude Code / Codex / Grok / Kimi 四家的 hook 落盘、状态栏、只读对话 trace、可用性诊断与 yolo 分级配置，Windows / macOS / Linux（含 WSL）同一命令面。CLI 名 `hst`。

hst 不做编排、不管 token 注入（密钥安全归 [ohmypwsh]）；agent 二进制安装归姊妹工具 `ark`（Agent Runtime Kit，`ark install claude`）。与 Hipo 的 hst history picker 共存：本工具装用户目录，不覆盖 `/usr/bin/hst`。

[ohmypwsh]: https://github.com/raystyle/ohmypwsh

## 安装

预编译二进制覆盖三平台。装好后把 `hst`（Windows 为 `hst.exe`）放进 PATH 上的任一目录即可（Windows 建议 `%USERPROFILE%\.hst\bin`）。

### Windows

```powershell
# 最新正式版（镜像直下把 URL 换 https://env.ohmygh.com/hst/stable/hst-x86_64-pc-windows-gnu.zip）
Invoke-WebRequest https://github.com/raystyle/hst_rs/releases/latest/download/hst-x86_64-pc-windows-gnu.zip -OutFile hst.zip
Expand-Archive hst.zip -DestinationPath $HOME\.hst\bin
Move-Item $HOME\.hst\bin\hst-x86_64-pc-windows-gnu\hst.exe $HOME\.hst\bin\
# 把 $HOME\.hst\bin 加进 PATH 后重开终端
hst --version
```

### macOS

```bash
curl -L https://github.com/raystyle/hst_rs/releases/latest/download/hst-aarch64-apple-darwin.tar.gz | tar xz
mkdir -p ~/.local/bin && mv hst-aarch64-apple-darwin/hst ~/.local/bin/
hst --version    # ~/.local/bin 需在 PATH
```

### Linux x86_64 与 WSL

```bash
curl -L https://github.com/raystyle/hst_rs/releases/latest/download/hst-x86_64-unknown-linux-gnu.tar.gz | tar xz
mkdir -p ~/.local/bin && mv hst-x86_64-unknown-linux-gnu/hst ~/.local/bin/
hst --version
```

### 源码安装与滚动更新

```bash
cargo install --git https://github.com/raystyle/hst_rs    # 源码
hst self update          # 自更新：缺省 dev 滚动源；--stable 走正式版
```

设 `HST_MIRROR=https://env.ohmygh.com` 后 `hst self update` 的 dev 通道走镜像（stable 通道直连 GitHub）。

### 前置

- 状态栏运行时是 pwsh（PowerShell 7）：装了才有状态栏，缺了只是不渲染，不影响其它命令
- hst 自管数据根 `~/.hst`（旧 `~/.oma` 首启自动迁移）；hook 注册与 yolo 键写各家用户级配置，AGENTS/CLAUDE 说明落项目目录（skill 面已退役 ADR-0005）

## 快速上手

```powershell
cd D:\my\proj          # 进你的项目，后续命令都不用再带路径
hst init               # 部署 hook / 状态栏 / yolo 键加旧数据根迁移 heal（幂等）
hst statusline         # 配置四家状态栏
hst doctor             # 体检：有 block 级问题才退出 1
```

全部命令支持 `--format kv|json|jsonl` 与 `--json` 信封输出。

## 部署与信任

```powershell
hst init                        # 全套：用户级 yolo 键加四家 hook 注册与状态栏加 heal 迁移
hst init --yolo                 # 仅用户级无阻塞键（全机生效，缺省 full 全 bypass）
hst init --yolo=partial         # 分级：编辑自动过，危险操作仍确认
hst init --yolo=off             # 全关：摘 hst 落的 yolo 键
hst init --project-yolo         # 仅项目级（项目覆盖用户级；=partial/off 可选级别）
hst init --pre-trust            # 额外预写家目录信任库（四家）
hst init --project D:\my\proj   # 不进目录也能指定项目
```

hook 注册常驻用户级 `~/.hst/hooks/` 自包含 shim，状态按 session 分键写 `~/.hst/state/`，零 hst 二进制依赖，可任意时刻无痛升级轮换；未 init 的项目也有状态数据。

## 诊断与验收

```powershell
hst doctor                  # yolo / 信任 / 登录态 / hook 形态 / 状态栏 / CPU 指令集
hst agents                  # 列四家检测：装没装、来源与版本（缺装 hint 指向 ark install）
hst agents verify           # 四家无头验收：hook 落盘加状态栏直跑
hst hook verify kimi --timeout 120     # 单家 hook 层验收
```

## 活性诊断

打真网关（api 缓存回归测试端点，配置注入）烧最小 token，与 doctor 的零网络体检分家；凭据读 agent 侧配置，也可用 `HST_GATEWAY_URL` / `HST_GATEWAY_KEY` 覆盖。

```powershell
hst diagnose cache                    # 全别名缓存命中矩阵（双连探测）
hst diagnose cache zy-gpt56sol-codex  # 只测指定别名
hst diagnose agents                   # 配置指向、别名在册、key 活性、thinking 上限
```

## 状态栏

```powershell
hst statusline                  # 四家全配（幂等）
hst statusline codex            # 只配一家
hst statusline --example        # 用户级定制模板（~/.hst/statusline.toml）
```

定制三层：段落显隐与顺序（`segments`）、段内模板与图标（`[template]` / `[icons]`）、整脚本替换（`hst statusline --script <路径>`，`--builtin` 还原）。改完配置重跑一次 `hst statusline` 生效。

## 对话历史检索

```powershell
hst trace sessions          # 项目内各 agent 的会话
hst trace search "重构"      # 按正则检索四家的修改与意图
hst trace file src\main.rs  # 单文件轨迹：谁、何时、为何改的
hst trace agent claude      # 某家 agent 的操作块时间线
```

只读直查四家原生会话库，六视图（sessions / timeline / blocks / agent / file / search）；timeline / blocks / agent / file / search 支持 `--limit` / `--offset` 分页，sessions 支持 `--limit`。

## 注意

`hst init` 的 yolo 面（缺省用户级）会关掉 agent 的审批与沙箱且**全机所有项目生效**，只在自己信任的机器与账户上用；要收窄：`--yolo=partial` 危险操作仍确认、`--yolo=off` 全关、或 `--project-yolo` 收到单项目。oma 更名过渡期已结束（v1.1.3 起）：oma 旧命令与 oma-* stub 资产退役，环境变量一律 HST_*（旧 OMA_* 不再读）。

## 更多文档

- 命令手册：`hst --llms`（CLI 紧凑手册，随命令树自适应）；库面契约：`docs/aidoc/` 投影
- 需求与设计：`docs/requirements/`（REQ 登记）与 `docs/adr/`（架构决策）；文档地图 `docs/README.md`
