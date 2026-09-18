---
id: ADR-0004
title: self-update镜像腿与缺省回退
status: superseded
date: 2026-09-15
deciders:
  - raystyle
supersedes: []
superseded_by: ADR-0008
tags: ['self-update', 'D16', 'D48']
---

# ADR-0004:self-update镜像腿与缺省回退

## Context

舰队滚版撞 api.github.com 匿名 403（IP 限流）时 self update 断源（stable 通道镜像整段跳过、HST_MIRROR 未设时 dev 也纯 GitHub），四机靠镜像段手工锚装绕过（ohmycloud 2026-09-14 报修，D48）。

## Decision

读序三态：HST_MIRROR 设值=基址覆盖加 mirror-first 两通道；未设=GitHub 优先、失败自动回退镜像腿默认基址 env.ohmygh.com（不占缺省行为面，ark 先例）；空串=全关。stable 镜像腿与 dev 同机制（deterministic 名取段内边车 digest 对 selfupdate.json 记录判新，段随通道、dev 禁落 stable）；GH_TOKEN 在位附 Bearer。

## Consequences

舰队级生产实证（update.mirror=fallback-default 打点）；镜像另播 hst/版本段（catalog-seed 通道）。
