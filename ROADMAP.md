# ROADMAP

项目全局路线：**大里程碑**。状态：`未开始` / `进行中` / `已完成` / `排后`。细碎轨迹见 `docs/diary/`。方案细节已随 ADR-0006 清退（git 历史可考）。

## 阶段总览

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| 0 | 项目基础设施：对照 ohmypwsh 的文档与目录；定位为通用智能体多路复用任务编排器 | 已完成 |
| 1 | 各功能部件 POC 验证原型 + rmux pin | 已完成 |
| 2 | 项目级自动配置（init）+ 多路 session + CLI 编排 | 已完成 |
| 3 | 编排三传输：HTTP 编排面加可视化网页、MCP、REPL | 已完成 |
| 4 | Windows 全量收口：安装接管、联邦检索、输出与易用、指令集检测 | 已完成 |
| 5 | 编排产品化与部署安全面：web 镜像与看板、和解式编排、serve 守护化、四环境自适应、doctor 与登录引导、密钥与权限面 | 已完成 |
| 6 | Linux/mac 端点接管与验收 | 排后 |
| 7 | 重新定位（D15）：去编排，oma 退化为 Agent 全平台 token、hook 与状态栏部署配置工具 | 已完成（2026-09-08，P0035）；阶段 1 至 3 与 6 的编排面随之退出历史 |
| 8 | 与 omc / ome 集成的优先级序（D24，用户裁 2026-09-10）：诊断与检测首要、恢复与治愈次之、安装部署配置最后 | 进行中：doctor / detect 面在役（doctor、agents 检测、diagnose 活性诊断）；heal / recover 未开始；install / deploy / config 排最后 |

## 阶段 0：项目基础设施

AGENTS 四段职责、PRD/GOAL/TODO/PLAN 四原语（PRD 引入 2026-09-03）、方案 0001/0002/0004、research、references、rumdl 加自研扫描门禁。现役定位见 P0035（D15 前为 P0004）。

## 阶段 1：rmux 与脚手架

> 方案 P0005。

十四件 `examples/poc-*` Windows 范围全绿；`oma check` 按 `catalog/rmux.toml` 装 pin 的 rmux 0.10.0。

## 阶段 2：会话与编排

> 方案 P0006 至 P0010。

`init` 全套（hook/skill/yolo 键，幂等不动家目录）；项目专属会话 spawn/status/send/cleanup；三段式粘贴；`oma run` 状态门分派；真四路拉通与 settle 自愈信任。

## 阶段 3：编排三传输

> 方案 P0011 与 P0016。

api 传输无关层一份核心三消费：`oma serve`（六操作 RESTish、JSON 信封、可视化网页、SSE 渲染行画面、trace 端点）、`oma mcp`（stdio 九 tools）、REPL（裸 `oma` 内嵌编排面）。

## 阶段 4：Windows 全量收口

> 方案 P0012、P0013、P0014、P0017、P0018。

rmux 与四家 agent 的自适应安装接管（两层 pin、渠道序、装后探针）；联邦轨迹检索六视图（grok updates 权威日志）；send 回显间隔、SKILL 命令图、grok 无头、`--json` 信封、TTY 表格、completions、`oma mcp --print-config`；CPU 指令集能力检测与探针异常退出分类。

## 阶段 5：编排产品化与部署安全面

> 方案 P0021 至 P0028（2026-09-01 至 09-02）。

web 镜像三面接管与本地化、看板资源包化（嵌二进制首启释放）；spawn 和解三态与 `oma respawn`；serve 守护化与协议化停机；`oma task` 带产物等待；四环境部署自适应（PATH bare 与 codex 字段所有权）与状态栏重铸（`agent:state` 机读标记）；doctor 部署诊断与 `oma agents login` 设备码引导；密钥与权限面（hook 拦截闸、一钥两密文、提供商别名、bypass argv）；仓库更名 ohmyagents-rs 与四仓生态定调。三平台（Windows、macOS、WSL Linux）四家安装与真身全链验收。

## 阶段 6：跨平台

> 排后（用户定调 2026-08-31：先把 Windows 全量开发好）。

用户定调（2026-08-31）：先把 Windows 全量开发好。资产与代码路径就绪（Windows named pipe 与 POSIX unix socket 双形态已在端点层分流）；指令集 SIGILL 预备检测研究已备（S021 检测阶梯）；待环境切换后按 P0012 验收。

## 阶段 8：与 omc / ome 集成的优先级序

> 用户裁定 2026-09-10（PRD D24）：oma 与 omc / ome 集成的功能优先级 = 首要诊断与检测，其次恢复与治愈，最后才是安装部署配置。

- **诊断与检测（首要，在役）**：`oma doctor`（零网络只读体检）、`oma agents`（四家检测）、`oma diagnose cache|agents`（活性诊断）；与 omc agent doctor 的分工边界照 2026-09-10 共识（oma 单机运行时治理，omc 舰队编排面）。
- **恢复与治愈（次之，未开始）**：heal / recover 面待立项（方向：诊断结论到本机修复动作；新需求照 PRD 追问链）。
- **安装部署配置（最后）**：oma 自身安装部署面已随 D20 收窄，后续集成以与 ome（`ome install`）、omc（透传与金库真源）的协作面为主。
