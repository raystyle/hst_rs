# hst-cli::statusrender

 状态栏原生渲染引擎（ADR-0010、REQ-026）：hst statusline render 子命令
 的 Rust 实现，退役 pwsh 脚本载体。
 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
 读 stdin agent JSON 出状态行，契约与退役中的 pwsh 脚本逐字对齐（对版
 口径：pwsh 侧强制 `$PSStyle.OutputRendering='Ansi'` 后逐字相同；管道
 缺省形态 PowerShell Host 渲染器会剥 ANSI，旧载体在 agent 实际调用里
 无色，原生缺省带色，评审 F1）。复用单源：段序与模板配置走
 statusline.rs 的 StatuslineConfig 加 effective_orders；loop/goal 探针走
 loopmgmt；goalmode 倒序分块扫描本模块 Rust 形（流式倒扫）。性能面：无
 pwsh 冷启动（约 300ms）加流式倒扫（105MB transcript 毫秒级）。ANSI 退
 裸文本开关：`NO_COLOR` 或 `HST_STATUSLINE_NO_ANSI` 任一非空（评审 F1）。
 已知边界：tools 段（显式选用面）首版渲染为空、版本本地探测缓存面
（D46）未移植（payload version 字段归一承载）、REQ-014/REQ-017 哨兵面
 未移植，见 REQ-026。

## Functions

- `render` — 渲染入口：stdin 全量字节加 agent 名，出多行状态串（行间 `\n`）。

