---
id: ADR-0007
title: 发布流水线对齐build-release标准
status: accepted
date: 2026-09-17
deciders:
  - raystyle
supersedes: []
superseded_by: None
tags: ['release', 'build-release', 'D57']
---

# ADR-0007:发布流水线对齐build-release标准

## Context

总台定稿全仓编译打包发布流程标准（project-evo build-release skill，2026-09-17 对齐单），三段式正源 = 本地编译打包到 gh release 直发到 Action 自动播种，编译与打包面不在 CI。hst 现状是 CI 全包（matrix 编译加 CI 内建 Release 加 artifacts 灌段），对齐自查九条差距（回执在案）。总台核准批 A 产地迁移、批 B 版本段 immutable 加双段红灯、批 C dispatch tag 补推加版本闸加解包冒烟，优先序照案；批 D 两裁定（2026-09-17 总台核准单）：一，dev 滚动 Release 保留例外（dev 段随仓裁标准明言，CI 轻岗建 dev prerelease 豁免入本 ADR，不迁镜像段，退役条件 = 舰队判新读序统一镜像优先时再启）；二，自升级读序 ADR-0004（GitHub 优先加镜像回退，用户裁 2026-09-15，403 限流背景）豁免在册保留非漂移，标准镜像优先为无 ADR 仓缺省；GitHub 旧响应 digest 缺省判新降级钉死随迁移批。

## Decision

批 A 产地迁移：本地编译打包走 `.tools/release.ps1`（wsl 交叉 linux 加 win-gnu、lan-mac 实机 ssh 构建；测试闸先行；包形 = 单顶层目录二进制加 README 加 LICENSE，win 形 zip 他形 tar.gz，逐包 sha256 边车；版本一致性闸加解包三端冒烟内置），正式版 `gh release create <tag> --latest` 本地直发禁 draft；`dev-release.yml` 瘦身：CI 面保留 docs-gate 与测试岗与 dev 轻岗（push main 建 dev prerelease，豁免在册），tag 推送不再触发 CI 构建，播种从 Release 资产拉（stable 走 release published 事件，dev 轻岗链内触发因 GITHUB_TOKEN 建档不触发 release 事件）。批 B：播种段制补 `hst/<版本>/` immutable 段（copy 加长缓存头，版本号 bare 无 v 前缀，与存量 `hst/2.0.0/` 同形）加 stable 滚动段 sync --delete-excluded；零上传红灯两段报数（版本段与 stable 段分别清点，零对象即红）。批 C：播种 workflow 挂 workflow_dispatch 带 tag 入参补推口（从 Release 重灌不依赖重跑链）；digest 钉死：GitHub 腿 digest 归一（bare hex 或 sha256:hex 统一 sha256:hex）加缺省时取 Release 边车资产判新加安装后按下载件实算哈希写记录。

## Consequences

- 好：编译产地自控可复现（工具链不受 runner 漂移）；CI 面瘦（测试岗加播种）；镜像段制完整（版本段 immutable 加 stable 滚动加 dev 随仓裁）；护栏三件齐（一致性闸加零上传红灯加 dispatch 补推）；自升级判新全锚边车无降级面。
- 坏：本地发布依赖主开发机与 lan-mac 在线（mac 实机构建链 ssh 单点）；发布人切换需重配 gh 与 rclone 环境；dev 轻岗豁免是标准外形态，舰队读序统一时需回收。
