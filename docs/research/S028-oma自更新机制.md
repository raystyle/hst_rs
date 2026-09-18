# S028-oma自更新机制

> 2026-09-02。用户定调：oma 应有 update 功能自己去 GitHub 升级新版本；目前本地测试、**还不封版**（无 releases），命令面用 `oma self update`。

## 需求

- oma 自更新：查 GitHub Releases、按平台取资产、自替换；封版前 releases 为空要体面降级（`--git` 源码安装为主路径）。

## 关键结论

### 1. 机制

- **查询**：`GET api.github.com/repos/<owner>/<repo>/releases/latest`（GH_TOKEN 自动附带）；404 = 无 releases，打 `update.release=unavailable` 与 `--git` 提示，退出码 0（封版前常态，不算错）。
- **版本比较**：tag 容忍 `v` 前缀，按点分数值逐段比（非数字段按 0）；不新于当前则 `update.ok=already-latest`（`--force` 跳过）。
- **资产约定**：**资产名即编译目标** `oma-<target-triple>.zip`（Windows）/ `.tar.gz`（Unix），按本机 OS 关键词（windows-msvc / apple-darwin / linux-gnu）加架构（x86_64 / aarch64）匹配；无平台匹配兜底任一 oma 资产（打日志供人工核对）。版本判据：dev 走资产 sha256，stable 走 release tag。
- **自替换**：新二进制先 copy 到旁路暂存；Windows 走 rename 舞步（运行中 exe 不能覆盖但可改名：当前改名 `.old`，新就位，再删 `.old`；失败回滚旧件保持可启动），Unix 直接 rename 覆盖（原子）。
- **`--git`**：`cargo install --git https://github.com/<repo>.git --force`（PATH 探针找 cargo）：封版前主路径。
- **`--repo owner/name`**：私有 fork / 改仓时覆盖（缺省 `raystyle/ohmyagents-rs`，更名自 OhMyAgents 2026-09-02，GitHub 旧名 URL 自动重定向）。

### 2. CI 滚动 dev release 与部署位

- 工作流 `.github/workflows/dev-release.yml`：main 每推则三平台**构建加测试**（Windows x86_64-msvc、Linux x86_64-gnu、macOS **仅 arm64 不要 Intel**，用户定调），再覆盖发布 prerelease tag `dev`（delete + recreate，资产带 `.sha256` 附带文件）。
- **正式版靠版本触发**（用户定调）：`v*` tag 推送则同一矩阵出正式 release（`--latest`），`oma self update --stable` 消费。
- **部署位切换**：`oma self update` 缺省通道 = **dev 滚动源**（`releases/tags/dev`）；dev 通道判新 = **资产 digest 对上次安装记录**（`~/.ohmyagents/selfupdate.json`；实测纠正：digest 是压缩包哈希，与 exe 哈希不可比；缺记录或缺摘要保守更新）；stable（`--stable`）按 release tag 版本比较。
- 资产名即编译目标：`oma-<target-triple>.zip|.tar.gz`（用户定稿；版本判据走 release tag，dev 走 sha256）。

### 3. 实测

[实证: 本机 2026-09-02]

- `oma self update` 于无 releases 仓实跑：`update.current=0.1.0`，再 `update.release=unavailable`（404 路径），再 hint `--git`，退出码 0。
- 单测：版本比较（含 v 前缀/双位/非数字后缀）、资产匹配（三平台断言 + 兜底）。

## 待办

- 工作流推上 GitHub 后首跑验证（gh release delete/create 路由、资产 digest 字段是否随 API 返回）
- 封版时：release workflow 产 `oma-<triple>.(zip|tar.gz)` 资产 + sha256 附带文件（download 后校验，install.rs sha256_file 复用位已留）。
- `oma agents update`（agent 层）与 `oma self update`（自身）语义对照进 R002。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本机 | `oma self update` 实跑（无 releases 404 路径） | 2026-09-02 | 降级行为实证 |
| web | GitHub REST releases/latest 语义（install.rs 既用同 API 族） | 2026-09-02 | 查询与资产字段 |

## 追记：镜像通道落地

D16，2026-09-08。

[实证： 2026-09-08 本机 cargo test]

- `OMA_MIRROR=<基址>` 环境变量接入 dev 通道：资产名按 host 三元组确定性构造（与 dev-release.yml 命名约定同表），取 `<基址>/oma/dev/<资产>.sha256` 边车判新（免 manifest；边车裸哈希归一 `sha256:<hex>`，与 GitHub digest 记录互认）。
- 下载后强制 sha256 校验：不符属安全问题，报错不回落 [实证： 单测覆盖解析与判等；2026-09-08 镜像三分支端到端绿]。
- 网络类失败（边车或下载）打 `update.mirror=failed` 加 `update.fallback=github` 回落 GitHub dev 路径；stable 通道不吃镜像（D16 时口径，D48 起扩到 stable，见文末 D48 追记；`update.mirror=skipped channel=stable`）。
- 镜像契约来自 ohmycloud 对账（2026-09-08 五点回执）：路径 `<tool>/<version>/<asset>` 加 `.sha256` 边车即锚，无 manifest；oma 段分 dev 与 stable 两段各回各段（ome #8/#9 教训），stable 段待首个 v* tag。

## 追记：D47 Windows 构建切 gnu

[实证： 2026-09-14 本机交叉复验加门禁全绿；ohmycloud 侧裸环境实弹]

- 用户裁 2026-09-14 摆脱 VC：CI 的 windows-latest msvc 岗换 ubuntu-latest 交叉岗（apt mingw-w64），Windows 资产名改 `hst-<arch>-pc-windows-gnu.zip`；本机 WSL 复验 PE32+ console 12.13MB 与 msvc 同量级，ohmycloud 裸环境（零 CC/AR 配置）交叉编译与 lan-win 实跑全过，CRT 静态零 DLL 依赖。
- host_keywords 的 windows 臂改 `["windows-gnu","windows-msvc","windows"]` 梯子：新源选 gnu、旧 release 仅 msvc 资产时回落命中、通用 windows 词保底旧 msvc 二进制升级新源 [实证： 单测 gnu 主名与 msvc-only 回落两断言]。
- 过渡窗口（codex 复核确认）：push 后镜像 sync 完成前边车 404 走 MirrorStep::Fallback 回落 GitHub dev，旧 dev 的 msvc 资产由通用词兜住，下一轮即 gnu [推断： 依回落链路代码与镜像 sync 时序]。
- 交叉岗 Test 跳过（PE 不可在 linux 跑，测试面由 linux/mac 双岗覆盖）；cfg(windows) 分支断言在 linux CI 不再参与编译，gnu 字面量的 CI 校验缺口以取参纯函数重构记 TODO（codex 评审 F3，不阻断）。

## 追记：D48 stable 镜像腿与缺省回退

[实证： 2026-09-14 本机两轮 e2e（mirror-first stable 腿真网下载替换与幂等 already-latest）加 177 单测加 28 集成绿；ark selfupdate.rs 源码取证]

- 断源背景：ohmycloud 舰队滚 1.1.5 撞 api.github.com 匿名 403（IP 限流）时 self update 无镜像回退（stable 通道镜像整段跳过、`HST_MIRROR` 未设时 dev 也纯 GitHub），四机靠镜像段手工锚装绕过。
- GH_TOKEN 更正：本文件前文「GH_TOKEN 自动附带」与现码不符：D48 前的 fetch_release 只发 UA 与 Accept；D48 起在位附 `Authorization: Bearer`（ark resolve.rs 同款，匿名 60 升 5000 次每时）[实证]。
- 读序三态（D48，ark 先例）：`HST_MIRROR` 设值 = 基址覆盖加 mirror-first 两通道（失败回落 GitHub）；未设 = GitHub 优先、失败自动回退镜像腿用默认基址 `env.ohmygh.com`（不占缺省行为面）；空串 = 镜像全关。kv 标记 `update.mirror` 三值形 = 基址原值、`fallback-default:<基址>`、`off`。
- stable 镜像腿 = dev 机制参数化复用（via_mirror 抽段参）：deterministic 名取 `<基址>/hst/stable/<资产>.sha256` 边车、digest 对 `selfupdate.json` 记录判新（hst 是 zip 资产，digest 是压缩包哈希与 exe 哈希不可比，故不照抄 ark 的裸 exe 直比）、sha256 强校验、Windows rename 舞步复用；段随通道、dev 禁落 stable（防正式版装进滚动源）。
- e2e 两轮 [实证]：R1 mirror-first stable 腿真网下载 v1.1.5 linux 资产、校验替换、记录 tag `stable-mirror`；R2 边车 digest 与记录一致出 `update.ok=already-latest`。
- 舰队级生产实证 [实证： ohmycloud 2026-09-15 v1.2.0 收敛回执]：四机旧位二进制升级 1.2.0 时实际走缺省回退腿（`update.mirror=fallback-default` 打点实证），GitHub 不可用场景自愈按设计生效；镜像另播 `hst/1.2.0` 版本段（catalog-seed 通道，与 hst/stable 滚动段并存）。R1 替换的是运行中 debug 二进制，顺带实证旧 v1.1.5 二进制的旧行为（mirror 设值加 stable 仍 skipped），即舰队断腿的机器侧复现。
- ADR-0008 追记（2026-09-18 家族统一标准轮，本条以上文 D48 三态为准的更新）：缺省翻镜像优先（未设 HST_MIRROR = 默认基址镜像段优先，任一步网络类失败整对回落 GitHub 不回环；latest 落 stable 滚动段、dev 落 dev 滚动段同读序），kv 标记 `fallback-default` 形退役换 `default-first` 形；D48 守卫两函数（github_fail_falls_back_to_mirror 加 mirror_fallback_base）随「结构化不回环」退役；GitHub 官方腿补 API digest 加边车锚硬校验（锚不可得 fail-closed 拒装）；降级守卫命中按 localNewer 收束（rc=0 不装不回落）；自替换补更新锁（pid 活性判加 mtime 超窗兜底）、陈旧收割（.old 救援件 exe 缺位保留）、入位后自证五次重试与回滚复核、证毕清理残件；管理方布局让位（ark 落痕或带落痕的符号链接，用户自建便利链接不误拦）[实证: 2026-09-18 评审回执加实弹 dev 腿降级收束 rc=0]。
