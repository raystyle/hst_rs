# hst-cli::statusrender

 状态栏原生渲染引擎（ADR-0010、REQ-026）：hst statusline render 子命令
 的 Rust 实现，退役 pwsh 脚本载体。
 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
 读 stdin agent JSON 出状态行，契约与退役中的 pwsh 脚本逐字对齐。复用
 单源：段序与模板配置走 statusline.rs 的 StatuslineConfig 加
 effective_orders；loop/goal 探针走 loopmgmt；goalmode 倒序分块扫描
 本模块 Rust 形。性能面：无 pwsh 冷启动（约 300ms）加 memmap 倒扫
（105MB transcript 毫秒级）。已知边界：tools 段（显式选用面）首版
 渲染为空，见 REQ-026。

## Functions

- `render` — 渲染入口：stdin 全量字节加 agent 名，出多行状态串（行间 `\n`）。

