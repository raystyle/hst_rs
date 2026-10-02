---
id: REQ-043
title: hook壳烘焙安全序与init同步
status: implemented
priority: must
trace: shim 单测 shim_exe_path_falls_back_from_dev_forms 加 deploy_shims_writes_eight 告警断言改件；实弹三场景（debug 裸 init 回落装位加旗标 init 同步死路径加告警打点）
---

# REQ-043:hook壳烘焙安全序与init同步

## Scenario

总台派单（2026-10-02，实战爆雷实证）：本机 hook 壳六件（~/.hst/hooks/hst-state|token.{sh,ps1,cmd}）被生成时钉死仓内 debug 构建绝对路径 /mnt/wsl/repos/hst_rs/target/debug/hst；总台 cargo clean 扫掉 debug 产物后全部工具调用三连 hook 喷屏（No such file or directory）；重跑带旗标 init 未同步（REQ-032 反馈件四的旗标跳过整跳 hooks 面），壳头「rerun hst init to sync」承诺落空；总台 sed 应急修六件后派单三改进 `[实证: 派单原文加本机复现三场景]`。

## Criteria

验收判据,可检验、可勾选:

- [x] 改进 1（烘焙安全序）：current_exe 落仓内开发构建位（/target/debug/ 加 /target/release/ 族，正反斜杠）时不烘焙，回落装位探测（~/.local/bin 加 PATH 首个 hst）再裸名 PATH 相对；正装照旧绝对路径烘焙（REQ-032 原位替换设计保持）
- [x] 改进 2（init 同步兑现）：带旗标 init（compact 加 keys-only 加 clear-project-yolo 族）不再整跳壳面：shim 幂等同步照跑（打点 init.shims.sync），注册面照旧跳（设计语义保持）
- [x] 改进 3（debug 形告警）：生成器为开发构建位时 init.warns 打「shim exe fallback」告警（改进 1 的回落即防，告警供人知）
- [x] 测试：shim_exe_path 四断言加既有八件告警断言改件；实弹三场景（debug 裸 init 壳回落装位、旗标 init 同步死路径、warn 打点）

## 边界

- PATH 探测命中的 hst 仍为开发构建位时跳过续回落（防 PATH 首位即 debug 形）。
- 裸名回落形（全落空）依赖 PATH 在 agent hook 运行环境可用（既有 hook 契约本就走 PATH 解析面，风险等价）。
- 壳文件死路径的纠偏依赖 exe 解析变化触发内容判等重写；同路径存活态不主动体检（文件存在性自检候裁）。
