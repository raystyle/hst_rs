---
id: REQ-055
title: ledger全线退役批
status: implemented
priority: must
trace: cargo test --locked（单测 239 集成 59 绿）加门禁全绿（fmt 加 clippy 加 aidoc strict 加 md 四门禁）加 grep src 零 ledger 残留
date: 2026-10-21
owner: raystyle
tags: ['ledger', '总台令', 'v3.0.0', 'REQ-010', 'REQ-018']
---

# REQ-055：ledger 全线退役批

## Scenario

总台令（2026-10-21，lan-ubuntu:wN:p1 下达）：ledger 全线淘汰核查。经实查，本仓 ledger 命中非文案注释而是 ledger.ohmygh.com 服务面实功能（issue 加 artifact 加密钥管理三命令族，ledger-client crate BASE_URL 即 `https://ledger.ohmygh.com`），按令整体剔除并发新版。

## Criteria

- [x] `src/ledger.rs` 模块、`pub mod ledger` 与 `ledger-client` git 依赖出仓；`base64` 依赖随 ledger 独占消费一并出仓
- [x] `hst issue`（new/list/show）、`hst artifact`（publish/attest/list）、`hst ledger`（keygen/verify）三命令族出仓（clap 枚举、派发臂、处理器全链）
- [x] `--llms` 手册常用例与子命令表随活命令树自适应收敛（issue 例句摘除）；README env 表三行（HST_ISSUES_API/HST_LEDGER_URL/HST_LEDGER_PRIVATE_KEY）与 issue 账本流段摘除
- [x] update.rs 三处错误提示语去 `hst issue new` 指引；AGENTS.md 遇缺陷即 hst issue new 的 Must 纪律行摘除（反馈通道停役待总台新令）
- [x] 源码注释 ledger 考古引用清理（lib.rs、compact.rs、doctor.rs、tests/cli.rs 的 ledger n6 与 roundtrip 引注）
- [x] REQ-010 与 REQ-018 各追加退役节；集成测试五件与帮助树四行出仓
- [x] 契约破裂按 semver 判据记 major：v3.0.0 承载（命令族与 pub 模块移除即公开契约破裂）

## trace

单测 244 降至 239（ledger.rs 五件出仓）、集成 64 降至 59（ledger 五件出仓）全绿；cargo aidoc 重生成后 docs/aidoc 零 ledger 投影；grep 全仓 src 零 ledger 残留（diary 与 CHANGELOG 历史记档层除外）。总台令原文指认的 statusrender.rs/selfupdate.rs 实查为零命中（selfupdate.rs 不存在，实名 update.rs），真面在 main.rs 与 ledger.rs，回执已向总台更正 `[实证: 2026-10-21 cargo test --locked 239 加 59 绿加门禁全绿]`
