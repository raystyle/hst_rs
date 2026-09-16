# 项目定位：Agent 全平台部署配置工具

> AGENTS「项目定位」的展开。现役定位以本文件为准。演进：方案 0001 是首期切面，方案 0002 与 0004 是前两版编排定位，方案 0035 定调本定位（D15，2026-09-08 用户裁定：去编排，退化为纯部署配置工具）。

## 本质

HST（Hooks, Statusline, Trace；原 Oh My Agents / oma，v0.6.0 更名过渡期，D29；版本线自 1.0.0 重开，D34；仓库名 `hst_rs`（`hst-rs` 为改名重定向别名），CLI 二进制 `hst`，数据根 `~/.hst`）是 **Agent 全平台部署配置与诊断工具**，专注五个功能（D20，2026-09-09 用户裁定）。

- **agent 可用性诊断**：`hst doctor` 只读体检（yolo / 信任 / 二进制 / state / 登录态 / hook 形态 / 状态栏 / CPU 能力），warn 与 block 分层；`hst agents` 四源检测；`hst diagnose` 活性诊断打真网关（D21）。
- **hook 设置**：`hst init` 部署 hook 注册（四家用户级常驻 `~/.hst/hooks/` shim，D28）与状态栏面，幂等合并（skill 面已退役 ADR-0005，ours 技能目录幂等清扫）；`hst hook` 状态落盘加密钥拦截。
- **状态栏设置**：`hst statusline` 四家写入面加用户级定制（D18；`agents statusline` 隐藏别名）。
- **对话 trace**：`hst trace` 六视图联邦读四家原生会话库（P0013/P0014），只读、与 rmux 零耦合；D15 曾连坐删除，D19（2026-09-08 用户裁定）全量恢复为只读检索面。
- **yolo 不阻塞设置**：`hst init --yolo[=full|partial|off]` 分级无阻塞键（full 全 bypass / partial 危险操作仍确认 / off 全关，D33）与 `--pre-trust` 信任预写。
- **全平台**：Windows / macOS / Linux（含 WSL）同一命令面；四环境自适应（P0027，矩阵见 S024）。
- **通用**：智能体集合可扩展。当前默认适配 Claude、Codex、Grok、Kimi，不是产品上限。

## 边界

- 配置钉在启动的那个项目目录，不替代 ohmypwsh 的五端环境总台。
- 不替代 claude / codex / grok / kimi 本体，只配置它们、诊断它们的部署形态。
- 不做编排（D15）：不拉起会话、不发任务、不做 web 镜像与传输面；rmux 不再是运行时后端。编排时代定位见归档 P0004 与 P0001 至 P0034。
- 不管 token 环境变量注入（D20，2026-09-09 用户裁定）：secrets（一钥两密文与四 shell 懒注入，S031）、providers 别名簿（S027）、login 设备码引导（S026）整体移除；密钥安全归 ohmypwsh。历史口径见 R002 移除面注记与 P0040。

## 四仓生态

> 2026-09-02 用户定调。四仓互相发 issue 协作（`https://github.com/raystyle/<仓>/issues`）；本仓是 agent 侧。

| 仓库 | 本地 | CLI | 职责 |
| --- | --- | --- | --- |
| ark-rs（原 ohmyenv-rs） | `D:\ohmyenv-rs` | `ark`（原 `ome`） | 本机部署、管理、验收；agent 二进制下装与版本、工具与运行时依赖。2026-09-12 更名 Ark / Agent Runtime Kit，首版 1.0.0，镜像 env.ohmygh.com/ark/ 段、ome/ 段兼容期保留 |
| hst_rs（原 ohmyagents-rs） | `D:\hst_rs` | `hst`（原 `oma`） | 可用性诊断、hook、状态栏、trace 与 yolo（本仓）；不管种子、不管 agent 二进制下装（D09）、不管编排（D15）、不管 token 注入（D20） |
| ohmypwsh | `D:\ohmypwsh` | 无单一 CLI | Windows 元主机操作台：初始主机密钥配置、兄弟仓 git 密钥扫描安全、集成 ark 与 hst 做本地与远程的主机操作、部署、检查、诊断 |
| ohmycloud | `D:\ohmycloud` | `omc` | 云端基础设施：工具、运行时、agent 各版本二进制的 S3 存储与 web 分发（专用域名加 Cloudflare）；镜像种子归此仓 |

- 分工裁决：agent（claude / codex / grok / kimi）二进制下装归 ark-rs（原 ome；D07，ohmyagents#5）；镜像种子归 ohmycloud（`env.ohmygh.com`，ohmycloud D36 / P0014）；hst 不管种子、不管 token 注入，只管可用性诊断、hook、状态栏、trace 与 yolo（D09 加 D15 加 D20 裁定）。旧裁决「安装归本仓、不进 ome 名录」（2026-08-31）已被 D07 反转；token 注入域曾属本仓（secrets / providers，S027 / S031）已随 D20 移除。
- 诊断分工共识（2026-09-10 跨仓对齐，ohmycloud 提案本仓确认）：hst 管本机运行时治理（hook / 状态栏 / trace / yolo，加模型缓存与网关连通的单机诊断即 `hst diagnose`）；omc 管舰队编排面（多端装态、配置对账、网关探活，配置真源在 omc 金库）。边界：hst 不做多端配置一致性检测，omc 不做 hook / trace 治理。omc 顶层有 `omc ome` / `omc oma` 透传命令（argv 原样透传；ome 更名 ark 后以 omc 侧口径为准），agent deploy 委托其安装命令（原 `ome install claude codex sops age`，更名后 `ark install`）；hst 正式 release 后经 herdr 会话知会 ohmycloud 更新 tool status 镜像锚（跨仓周知与回执一律走 herdr 会话同步、不发 issue，用户裁 2026-09-10）。omc tool 域的 oma 段吃 oma/stable 镜像（hst 更名后主段 hst/，oma/ 段兼容期同播）。同文件协调三点已裁（2026-09-10 herdr 回执，D28 起口径修订）：omc 渲染器读改写语义天然保留 hst 状态栏段（statusLine 与 [tui]，测试五断言钉死，omc 模板永不碰状态栏面）；yolo 托管端与 hst 的协调改由用户级同一文件幂等合并承载（D28 用户级化后收敛为同文件共识，项目级覆盖用户级语义保留）；pretrust 信任预写归属 hst init --pre-trust，omc / ark 不双写。集成功能优先级（用户裁 2026-09-10，PRD D24）：诊断与检测首要、恢复与治愈次之、安装部署配置最后（ROADMAP 阶段 8）。
- 分发通道：官方源与 `env.ohmygh.com` 镜像由 ark `download_asset_with_mirror` 消费；hst 不维护渠道序、不参与种子；`agents install` / `update` 兼容层已随 D20 删除（历史曾 deprecated 指向 ome）。
- 密钥：密钥体系主权与跨仓密钥扫描安全归 ohmypwsh，集成而非自建；hst 只保留 hook 面密钥拦截闸（S030，实值比对只看环境变量）。

## 约束在一个项目

启动 cwd 或 `--project` 就是唯一工作区。AGENTS / CLAUDE.md 说明落项目目录（skill 面已退役 ADR-0005）；hook 注册与 shim 常驻用户级（`~/.hst/hooks/`，D28），状态按 session 分键写 `~/.hst/state/`（供状态栏 `agent:state` 机读标记消费，S025，未 init 项目也有状态数据），状态栏脚本释放 `~/.hst/statusline/`，agent 版本探针缓存 `~/.hst/cache/`（D46，按 agent 单文件 `agent-version-<agent>.json`）。信任库（各 agent 记在用户家）可以预写（`--pre-trust`）。
