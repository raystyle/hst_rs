---
id: ADR-0008
title: selfupdate家族统一标准对齐
status: accepted
date: 2026-09-18
deciders:
  - raystyle
supersedes: [ADR-0004]
superseded_by: null
tags: ['self-update', '家族统一标准', 'REQ-013']
---

# ADR-0008:selfupdate家族统一标准对齐

## Context

总台家族自更新统一标准轮（2026-09-18 用户裁「采纳推广」，权威 = browse-rs REQ-005 加 build-release 公共契约第六节）：镜像 stable 滚动段优先、任一步失败整对回落官方、digest 锚硬校验不符拒回落、semver 只升不降本地领先报 localNewer 不动、自替换带锁与陈旧收割与 --version 自证五次重试与回滚复核、管理方布局让位。hst 的 D48 读序（ADR-0004：未设 = GitHub 优先、镜像仅回退腿）与标准缺省方向相反，且 GitHub 官方腿下载零锚校验、自替换无锁无自证无回滚复核、无管理方让位。

## Decision

对齐家族标准，全量承接 ADR-0004 仍有效件并翻缺省方向：HST_MIRROR 未设 = 镜像段优先（默认基址 env.ohmygh.com，stable 通道落 stable 滚动段），镜像腿任一步网络类失败整对回落 GitHub 官方（不回环）；设值 = 基址覆盖同读序；空串 = 全关。段随通道与 dev 禁落 stable 与 GH_TOKEN Bearer 沿用。GitHub 官方腿下载后对 API digest（缺省回落同 Release 边车资产）硬校验，不符拒装不回落。latest 通道判新分三态（already-latest、localNewer 不动、更新），双腿统一暂存件 --version 预检降级守卫。自替换：exe 旁更新锁（create_new 加 pid 活性判加陈旧收割加 drop 清锁）、陈旧暂存与备份件收割、旧件 pid 备份、入位后 --version 自证五次重试（期望版从 release tag 注入）、证败回滚并复核在位、回滚受阻报自救路径。管理方布局（exe 同目录 ark-managed 落痕或用户面 bin 符号链接指向本 exe）拦自更新，CTA 走 ark。

## Consequences

- 好：缺省读序与家族一致（browse 与 ark 同形），api.github.com 匿名限流面从首腿消失；官方腿补锚校验后两条腿同安全级；降级保护全覆盖；并发自更新互踩面收敛。
- 坏：缺省行为变化（未设 HST_MIRROR 的存量端从 GitHub 优先变镜像优先），kv 标记 fallback-default 形退役换 default-first 形；ark 落痕生产者契约待 ark 侧批次（未落痕的 ark 实文件安装检测不到，元数据对齐验收待 ark 批双报）。
