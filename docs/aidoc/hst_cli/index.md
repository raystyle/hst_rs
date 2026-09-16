# hst-cli 2.0.0

hst-cli：HST（Hooks, Statusline, Trace）库面——四家 agent（claude、
codex、grok、kimi）的部署配置与诊断内核。承载五功能：hook 状态落盘
（用户级 session 分键）、状态栏部署与拆段拼装、只读 trace 六视图联邦
检索、doctor 只读体检与 yolo 无阻塞键分级管理。自更新走 dev 滚动与
stable 双通道（镜像腿带缺省回退）。命令面唯一权威见 R002；行为动机见
docs/research 与 docs/adr；输出信封契约见 R011。

## Modules

- [`agents`](agents.md): 四家 agent 探测（PATH、env、hst 自管根、默认目录四源）。
- [`archive`](archive.md): 的通用归档工具面（细则见 R002 与模块文档）。
- [`caps`](caps.md): CPU 指令集能力与探针退出形态分类（S021/P0018）。
- [`deploy`](deploy.md): `hst init` 部署层：hook 注册四家用户级、shim 落位、状态栏面与 ours 技能目录清扫（D53/ADR-0005）。
- [`diagnose`](diagnose.md): `hst diagnose` 活性诊断族：网关缓存命中矩阵与配置活性（D21）。
- [`doctor`](doctor.md): `hst doctor` 只读体检：yolo、信任、二进制、登录态、hook 形态、状态栏与状态面。
- [`fmtio`](fmtio.md): 全局输出三态（kv/json/jsonl）与结构化错误出口（R011 契约）。
- [`hook`](hook.md): `hst hook`：事件到四态映射、用户级 session 分键 state 落盘与密钥拦截分流（D28/S030）。
- [`install`](install.md): hst 根解析与共享下载件（self update 复用）。
- [`pathutil`](pathutil.md): 的路径工具面（细则见 R002 与模块文档）。
- [`secretguard`](secretguard.md): 的密钥拦截闸面（细则见 R002 与模块文档）。
- [`shim`](shim.md): shim 三形态自包含状态写入器：cmd/ps1/sh 加 grok 包装（D27/D28/D39）。
- [`statusline`](statusline.md): `hst statusline`：四家状态栏写入面幂等合并与拆段拼装（S025/D18/D42 至 D51）。
- [`trace`](trace.md): trace 六视图：联邦读四家原生会话库归一检索（P0013/P0014，D19）。
- [`update`](update.md): `hst self update`：dev 滚动与 stable 正式版自更新、镜像腿与缺省回退（S028/D16/D48）。
- [`verify`](verify.md): `hst agents verify`：四家无头验收两层判据（D17/S033）。
- [`yolo`](yolo.md): `hst init --yolo`：四家分级落盘、ours 退役与 pretrust（D33/D52）。

