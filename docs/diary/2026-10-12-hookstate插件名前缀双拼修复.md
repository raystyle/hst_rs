# 2026-10-12：hookstate插件名前缀双拼修复

> 总台修复单：本机 claude statusline 末段渲染重复，显示「evo-lint-evo-lint 后缀静检」（其余段正常；statusline 即本仓产物 target/debug/hst）。任务定位 evo-lint 段标签拼接处根因、修复、cargo build 刷新、过闸提交；另面 lan-ubuntu 新开发台装机 2.10.2 的版本落差与升级道随回执注明。

## 流水

1. **根因一句**：REQ-044 插件面显示形无条件 `{插件名}-{statusMessage}` 前缀拼装，而 evo-lint 0.8.0 的 hooks.json statusMessage 本身即「evo-lint 后缀静检」（statusMessage 在 Claude Code 原生显示面是全文文案，插件作者按原生习惯把插件名写进首段），`evo-lint-` 加 `evo-lint 后缀静检` 即双拼 `[实证: 本机 /home/ray/.claude/plugins/cache/project-evo/evo-lint/0.8.0/hooks/hooks.json 对读加修前实弹渲染复现]`。修前链路复核：hook_stem 对 `uv run --no-project "${CLAUDE_PLUGIN_ROOT}/scripts/lint-hook.py"` 取干 stem `lint-hook`（未收录族），显示覆写表生效，即双拼出口。
2. **修法**：collect_plugin_hooks 拼装处换 plugin_display 纯函数：文案自带插件名前缀时原样用不双拼，前缀边界认全等加空格加连字符三形（`evo-lint` / `evo-lint 后缀静检` / `evo-lint-后缀静检`），无缝前缀（`plug` 对 `plugx`）不算命中照常拼；stem 回落腿同判（stem 恰以插件名连字符开头时同样免双拼，语义一致）。文档面同步：EXAMPLE_TOML hookstate 注释、collect_plugin_hooks 契约注、REQ-044 判据 2026-10-12 补注与 trace 回填。
3. **测试与实弹**：新件 plugin_display_dedups_short_prefix_in_label（边界三形加无缝反例）加集成件 hooked_aliases_plugin_face 扩 plug-e（statusMessage 自带前缀形）双断言（无根与带根两态）；全测 218 加 51 绿，fmt 净，clippy 新码零告警（存量四警告在册不动），aidoc --check --strict 绿（26 件），md 四门禁绿；PEVO 自检 11 PASS 1 FAIL，FAIL 全为存量 diary 禁字（2026-10-11 三行加 2026-10-02 两行，非本批引入，n7 先例同判豁免）。cargo build 刷新 target/debug/hst 后实弹：末段渲染 `herdr-agent状态监控 | hst-token护栏 | hst-会话状态同步 | hst-goal | evo-lint 后缀静检`，双拼消除，statusline 即时生效 `[实证: 本机渲染输出加全测日志]`。
4. **过程坑一笔（当场记，跨仓面）**：evo-lint 0.8.0 的 PostToolUse 静检 hook（恰是本单修显示的那位）对 .rs 文件硬编 `rustfmt --check --edition 2024`，本仓 edition 2021，混大小写 use 组排序两判据天然错位（statusrender.rs 的 statusline use 行每次编辑必拦）；仓内权威 `cargo fmt --all -- --check` 绿，照 hook 排序反改会把仓门禁改红，豁免留因由不照改。属插件侧缺陷（不读目标仓 Cargo.toml edition），hst issue new 开单 n8 反馈，修复归 project-evo 侧。
5. **lan-ubuntu 版本落差**：该工位接线 `~/.local/bin/hst statusline --render claude`，装位 2.10.2（镜像 stable，非本仓构建），无本修复；本修复在 main 待下一发布批（补丁判据：纯显示缺陷修复无契约变化），release.ps1 走版后镜像 stable 段到货即自愈，工位无需手动动作；急用临时道 = 本仓 linux 构建产物拷贝替换装位（未指示不动远端不发版）。
