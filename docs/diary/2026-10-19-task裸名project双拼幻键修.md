# 2026-10-19 task 裸名 project 双拼幻键修与状态栏刷新面答复 task-02

## 事

lan-pve（pve-harness 工位，hst 2.15.1）bug 反馈两件：①cwd=项目根下 `hst task set --project pve-harness` 落盘 `-home-ray-repos-pve-harness-pve-harness--claude.json`（路径 slug 加实参双拼），状态栏按 payload 根找单 slug 键不中、task 段不显示；②task set/clear 后状态栏要等下一轮交互才刷新，体感像没生效。要求定位修或回复设计意图。

## 修:报单①

- 根因：`project_root`（main.rs）按 M031 把相对 `--project` 一律 join 到 cwd，裸名单名在项目根内解析成 `<root>/<root 名>` 幻键；task set 落键成功但状态栏与 Stop 腿按 payload 项目根（真根）键控，永不可见（静默错位，非报错）。
- 修法：裸名单名（单 component）双解析:cwd 下同名先认（monorepo 子目录同名不受扰，现行相对语义不变）；不在场自 cwd 逐级上溯取首个 basename 同名祖先（项目根与任意深度子目录内 `--project <项目名>` 皆命中真根；上溯命中必在盘）。带分隔符相对路径与绝对路径语义不动。
- 兜底：task set 落根不在盘出 `task.warn=project root … 不在盘` 行（循 `init.warn=` 先例，warn 不拦写防破脚本面）；`--project` 帮助串 12 处同步「裸名单名就近上溯解析」。
- 测试：集成 `task_set_bare_project_name_resolves_ancestor_no_double_slug` 五断言（根内裸名单 slug 落盘、cwd 缺省 show 同键可中、子目录裸名上溯、兄弟目录现行语义不变、幻根 warn）；断言用 canonicalize 形（macOS TMPDIR 符号链接 getcwd 漂移，trace 测试先例注记）。

## 答复:报单②设计意图不修

- claude 面 bar 是 command 型（settings.json statusLine），由 Claude Code 自身在会话事件（消息提交与接收，最快 300ms 节流）时重跑；外部进程与 hook 腿均无通道触发 CC 重渲染，渲染节奏归各家 harness（grok 有 refresh_interval、kimi 1s 节流、codex 内置项，S025 矩阵在案）。[实证: code.claude.com/docs/en/statusline 契约加 S025 四家矩阵]
- hst 侧数据面零缓存：task 段每渲染直读 state/task 档，不存在 hst 自己的轮询延迟。缓解姿势：task set 在同一 Claude 会话内经 Bash 跑，工具回合结束即会话事件、下一拍重渲染即显；外部终端（另开 pane）设完则等该会话下一次交互自然刷新。

## 证

- 单测 244 加集成 64 绿（NO_COLOR=1 cargo test --locked，新增 1 件）；fmt 双版绿（2021 加 2024）；clippy 改动文件零新警（存量基线同形）；aidoc 26 artifact strict 0；md 四门禁绿（ref-scan 0 断链加 heading-scan 0 括号标题加 charlint 加 rumdl）。
- 报单原样复现修后复验：根内裸名落单 slug `-tmp-taskrepro-repos-pve-harness--claude.json`（修前双拼形）；幻根出 warn 行；子目录裸名 show present=true。2.15.2 patch 定名。

## 次

- 装机位热换与总台侧升级重验随发布链；CHANGELOG v2.13.0 节 U+1F411（绵羊形）存量字形留因由不改史节（承 v2.15.1 批同判）。

## 验收回执:lan-pve 侧当日闭环

- 报单方（lan-pve-claude-1）实机验收：v2.15.2 经 ark 管道滚 stable 已升；旧双拼档已删；task 重设单 slug 落位 present=true（状态栏随会话下一拍重渲染即显）；修法与 warn 兜底认可，报单② 缓解姿势知悉采用。task-02 闭环。
