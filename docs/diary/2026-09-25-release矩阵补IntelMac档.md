# 2026-09-25：release 矩阵补 Intel Mac 档与 v2.8.1 封版

> 派单：舰队新收编异网 Intel MacBook（macOS 12.7.6，x86_64，omc 第六端 agent 机），hst 2.8.0 资产只有 aarch64 档无件可装；release 矩阵补 x86_64-apple-darwin，滚 v2.8.1（纯发布面扩展）。既有三平台产物名与压缩形不动（omc catalog pin 对账面）。

## 流水

1. **形态定标**：ADR-0007 三段式的正式版资产产地在本地（`.tools/release.ps1`，lan-mac 实机编 darwin），CI 面只跑测试岗、dev 轻岗与播种；派单「CI 出全四平台资产」口径按「仓内流水线既有形态择优」边界校正为：CI 构建矩阵四档（出 dev prerelease 资产加交叉岗实证），正式版四档由本地直发 `[实证: dev-release.yml 事件条件与 release.ps1 全链]`。workflow 头注 2026-09-17「mac 只要 arm64 不要 intel」用户裁决随本单翻转，ADR-0007 后记在案。
2. **前置实弹**：lan-mac（arm64，macOS 26.6.2）交叉编 x86_64-apple-darwin 24.48 秒出 Mach-O x86_64（lipo 确证）；ssh config 既有 `macbook` 主机探明即新收编 intelMBP（x86_64，macOS 12.7.6），交叉件两跳投递实跑 `hst 2.8.0`。冒烟策略由初判的降级形状闸（本网无 x86_64 宿主）升为真机运行时闸 `[实证: 2026-09-25 本机两跳实弹]`。
3. **改面**：release.ps1（lan-mac 同机双档构建、publish 计数断言六件升八件、intel 冒烟宿主 macbook 加不可达显式弃保旗标同 win 腿形）、dev-release.yml（矩阵四档，arm64 runner 交叉编，Test 对交叉岗跳过同 win-gnu 先例）、环境事实（平台矩阵行、发布流行、仓外邻机行）、README 装机面 intel 行。selfupdate 零代码改动：资产名按 arch 动态推导，x86_64 mac 用例测试在册 `[实证: update.rs asset_name 加 tests]`。未立新 REQ：纯发布面运维扩展无产品面需求，trace 由 ADR 后记加 CHANGELOG 加本 diary 承载。
4. **封版 v2.8.1**：semver patch 判据（零功能零行为变化）；门禁全绿（fmt、clippy 零 error、222 单测加 48 集成、aidoc strict、md 四门禁、PEVO 合规 11 PASS）。
5. **发布实绩**：tag v2.8.1 = 2e28ab9（main 头）；release.ps1 全链绿，四端解包冒烟逐字对（linux 加 arm64 mac 加 intelMBP 实跑 `hst 2.8.1` 加 win）；gh 直发 `--latest` 八件。
6. **验收自取**：gh api 资产八件含 `hst-x86_64-apple-darwin.tar.gz`（digest `fea027e3` 开头）加 `.sha256` 边车；CI push run 36099146544（构建四岗含 intel 交叉岗、dev prerelease、seed dev）conclusion=success，release run 36099420164（stable 播种）conclusion=success；播种日志 `seed.count.version.2.8.1=8` 与 `seed.count.stable=8`，intel 件双段 Copied (new)；镜像面 env.ohmygh.com 与 hst.ohmygh.com 的 stable 段 intel 件及边车 HTTP 200，边车内容与 GitHub digest 逐字对 `[实证: gh api 加 run logs 加 curl，2026-09-25 自取]`。

## 自省

- 派单预设「CI 出资产」与仓内 ADR-0007 形态（本地直发）有出入：按仓内既有形态落本地直发，CI 面同步扩矩阵补齐派单验收面（run conclusion 自取）。预设与形态冲突时以仓内 ADR 为准，再把派单验收口径在两读序下都做实。
- 冒烟初判「本网无 x86_64 宿主」只扫了发布链既有宿主（wsl 加 lan-mac 加 win 回环），差点降级 lipo 形状闸；动手前扫 ssh config 全主机面才探明 macbook 即 intelMBP。降级设计前先穷举可达宿主面，再谈降级。
- PEVO_CHECK_ALLOW 是豁免清单形（标准命令钉在 docs/README 末节），随手 `=1` 会把 aidoc 生成物与存量 diary 债全亮红。工具门禁参数以 docs 标准命令为准，不凭记忆写。
