# S036-kimi2.2.0权限值语义对调与分级取值翻正

> 2026-10-17。总台修单立案（lan-win w19:p1）：`hst init --yolo` 后 kimi 格仍弹窗，用户报不持久。证据主件 = grok 研究单（share/lan-win/c58479c604，yolo-detect-flip.txt，实查日 2026-10-16）。一题一事：kimi 2.2.0 的 `default_permission_mode` 三值语义，与 hst 分级取值的对位翻正。

## 背景

hst 自 D33 起 yolo 分级：full 落 kimi `yolo`、partial 落 kimi `auto`。该映射定于 kimi-cli 旧时代（S007 时点 kimi 0.38.0）：旧站口径 `--yolo` 即全放行，故 full 对位 `yolo` 是当时的正确读法。kimi 2.2.0（现 kimi-code 文档站）值语义与旧站不是同一套，总台实测 `hst init --yolo` 后 kimi 格高危操作仍弹窗，立案复查。

## 关键结论

1. **kimi 2.2.0 三值语义**（配置 `~/.kimi-code/config.toml` 键 `default_permission_mode`，值集 manual|yolo|auto）：
   - `manual` = Always Ask（每步都问）；
   - `yolo` = Ask When Needed（常规修改与命令自动跑，高危操作、提问、计划退出仍会问；对应旗标 `--yolo`，隐藏别名 `--yes`/`--auto-approve`）；
   - `auto` = Never Ask（完全不打断，危险命令守卫也不再拦；对应旗标 `--auto`；非交互 `-p` 固定用 auto 且禁叠 `--yolo`/`--auto`/`--plan`，`--yolo` 与 `--auto` 互斥）。
   - 旧键名 `default_yolo` 属旧 kimi-cli，2.2.0 配方不以它当主档。[实证: 2026-10-16 grok 研究单 share/lan-win/c58479c604 帮助原文与文档站摘录（moonshotai.github.io/kimi-code 之 kimi-command、slash-commands、interaction、config-files、overrides 五页）加本机 kimi 2.2.0 --help]
2. **「yolo 当全跳过」是旧站残留误读**：旧 kimi-cli 的 `--yolo` 等于全放行，2.2.0 起同一词位变为 Ask When Needed。写读同 keys 互证会假绿（hst 写 `yolo` 读 `yolo` 判 ok，但实机语义是高危仍问），总台实测复现（init --yolo 后 kimi 格仍弹窗）。[实证: 2026-10-16 总台实机 + grok 研究单判据表]
3. **分级取值翻正**（v2.15.1）：full 落 `auto`（Never Ask 真全跳，恰是全 bypass 档）；partial 落 `yolo`（Ask When Needed，常规自动过、危险操作仍问，恰是半放行档）。doctor 读侧 ok 值集本就含两值（`auto|yolo` 均绿），读侧不分级故自动跟对；full 活性判（user_yolo_active_with）走 level_keys(Full) 同步翻为 `auto`。ours 值集 OURS_KIMI_MODES 仍为 `{yolo, auto}` 两代并收，off/退役面无需迁移：装机位旧值 `yolo` 仍在 ours 集内可摘，重跑 init 即落新值。[实证: 2026-10-17 本仓单测 244 加集成 63 绿（含翻正断言）]
4. **对照余波（同单判据表，hst 落值面无涉）**：codex `-a never` 仍走沙箱（全跳要 `never` 加 `danger-full-access` 两键齐，hst 现行落法已齐）；grok `auto` 档仍会问、全跳是 `always-approve`（hst 现行落法已对）；kimi 热翻 `/yolo`、`/auto` 都要先开模式列表再按 Enter（非无对话框热翻），冷翻落 config 即本仓路径。[实证: 2026-10-16 grok 研究单翻台矩阵节]

## 踩坑沉淀

| 坑 | 正解 |
| --- | --- |
| 把 kimi `yolo` 值当全跳过（旧站残留语义） | 2.2.0 起全跳是 `auto`；`yolo` 是 Ask When Needed |
| 写读同 keys 互证当验证 | 值语义随上游版本漂移，要对官方值表判据，不能自证 |
| kimi 无人值守配 `yolo` 档 | 用 `auto`（Never Ask），高危与守卫都不拦 |

## 待办

- 无（取值翻正随 v2.15.1 落地；装机位旧值经重跑 init 自愈）。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 键 | share/lan-win/c58479c604（yolo-detect-flip.txt，grok 研究单） | 2026-10-16 | 四家 yolo 检测判据表、kimi 2.2.0 帮助原文与三值语义、翻台矩阵、设置配方 |
| web | moonshotai.github.io/kimi-code（command、slash-commands、interaction、config-files、overrides） | 2026-10-16 | 值语义、别名、互斥与 headless 固定 auto 口径 |
| 本机 | kimi 2.2.0 --help / --version（grok 研究单内采） | 2026-10-16 | 旗标面与版本锚点 |
| 实机 | 总台（lan-win）init --yolo 后 kimi 格弹窗复现 | 2026-10-16 | 缺陷实锤与修单缘起 |

## 缺口

- kimi 状态栏「Always Ask / Ask When Needed / Never Ask」三串的字形实读未采（grok 单声明「状态栏字形未实读」），本仓 doctor 不依赖屏面正则，无碍。[记忆: 候后续单补屏面字形]
