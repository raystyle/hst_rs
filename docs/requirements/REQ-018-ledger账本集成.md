---
id: REQ-018
title: ledger账本集成
status: implemented
priority: must
trace: ledger.rs 单测六件加 cli 集成三件加实弹三验（开单加列表加 artifact publish）
---

# REQ-018：ledger账本集成

## Scenario

总台令（2026-09-20）：集成 ledger 标准的 issue 与 artifact 命令族，替代原 issues.ohmygh.com issue 面，完毕封小版本。契约真源 = 总台 REQ-063（Phase 3 CLI 原生集成）加 ledger worker 实现码（字段名与投影形逐面提取）。

## Criteria

- [x] 签名道：写入五头（Idempotency-Key 加 X-Key-Id 加 X-Timestamp 加 X-Nonce 加 X-Signature）；签名基七行换行连（v1 加 POST 加路径加 ts 加 nonce 加 idem 加 body sha256hex，与服务端 signatureBase 同源）；Ed25519 私钥运行时从 env `HST_LEDGER_PRIVATE_KEY`（base64url seed）或密档 `~/.hst/ledger/ed25519.key`（0600）读，不进仓不进 argv；公钥 JWK 常量内置，kid = sha256hex(常量)（单一真相不另存字面量）
- [x] 密钥面：`hst ledger keygen` 落密档（0600）打印 kid 与 JWK（私钥不打印）；密钥对生成经一次性 example 完成，私钥只在提交侧
- [x] issue 族：`issue new`（title 加 kind=bug|improvement 缺省 bug 加 acceptance 必填）、`issue list`（家族标准：缺省 100 加 before keyset 游标加 has_more 权威饱和判定加 count 语义加截断提示）、`issue show`（projection 加 timeline）、`issue close`（result 引 digest 先行加 status=done 收尾，digest 本地校验 sha256:<64hex 小写>）
- [x] artifact 族：`artifact publish`（name 加 kind 十五种加 digest 加 version 加 git_range 加 deps[] 加 summary 加 outcome 加 git_sha）、`artifact attest --type`（六种事件）、`artifact promote`（promote 糖）、`artifact list --current --env`
- [x] 替代面：旧 issue.rs 客户端（issues.ohmygh.com 直连）整体退役，家族三件（钳制加饱和判定加提示）迁 ledger.rs 且提示改指账本网页面；组级 help 注明真源 ledger.ohmygh.com 加旧服务过渡保役
- [x] GET 不签名不受配额；写入 per-key 50 条每 UTC 日幂等命中不耗（服务端契约面，客户端透传 429 与 Retry-After 语义）
- [x] env：`HST_LEDGER_URL` 覆盖基址（测与灰度）

## trace

ledger.rs 单测六件（签名基向量、kid 常量自检、digest 校验、kind 集对服务端契约、keygen 加 load 双通道往返隔离、提示指账本面）；cli 集成三件（publish 本地校验秒红、close digest 校验、keygen 隔离落档不泄 seed）；实弹三验（总台在册后：issue new 201 加 list 投影对加 artifact publish experience 201，回执在 diary）。

- [x] 收口附注（2026-09-20 总台修正令）：权限只增（close 面移除、attest 收窄三验证型、promote 移除）（关删与状态推进归 omc 工作台）；自研客户端退役改引 ledger-client crate（v0.1.1，URL 缺斜杠缺陷本仓实弹报出后总台即修）；密档迁 hex 形 `[实证: crate 面 issue list 投影对加 attest_dev seq=157]`
