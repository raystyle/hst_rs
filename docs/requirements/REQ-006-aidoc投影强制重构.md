---
id: REQ-006
title: aidoc投影强制重构
status: implemented
priority: must
trace: cargo aidoc --check --strict; cargo test --locked
---

# REQ-006:aidoc投影强制重构

## Scenario

dev-evo 第五十九批 ADR-0006 将 Rust 栈 aidoc 投影强制化（bin-only 不豁免，受众是维护者与 agent），本仓既有不适用裁定需撤换并落地全链。

## Criteria

- [x] 撤换 AGENTS 与地图的不适用裁定句，改引强制口径
- [x] REQ 登记（本件）
- [x] /// 契约注释覆盖公开项（162 处补齐）
- [x] missing_docs = deny 落地（Cargo.toml lints）
- [x] cargo aidoc 生成投影进 Git（docs/aidoc/ 23 artifact 含 llms.txt）
- [x] cargo aidoc --check --strict 入 AGENTS Commands 作漂移门禁
- [x] PEVO_CHECK_ALLOW 指 docs/aidoc/ 在册（tool-rust 豁免实务）
