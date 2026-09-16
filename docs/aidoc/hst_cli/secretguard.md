# hst-cli::secretguard

密钥拦截闸：模式表八层防误报、实值比对、block 级 exit 2（S030/D23）。
密钥 hook 安全拦截（S030）：oma hook 的第二职责，接管 ohmypwsh
secret-guard 的会话出口闸。误报八层防线（S030 误报策略节）：
① 精确前缀硬阻断 ② 实值比对零误报通道 ③ 熵值门（通用赋值类）
④ stopwords 占位符豁免 ⑤ 语料豁免（测试运行时拼接构造）⑥ warn-only
分级出口 ⑦ 日志掩码 ⑧ fail-open（异常不挡活）。

阻断语义对齐 ohmypwsh：PreToolUse / UserPromptSubmit 命中 block 级 →
调用方 exit 2；PostToolUse 只观察不阻断。

## Functions

- `guard` — guard 主判定（fail-open：任何一步拿不到文本都放行）。
- `mask` — 掩码（防线 7）：审计与 stderr 只见前 4 后 4。
- `scan` — 扫描：命中列表（含 warn 级，调用方分级处置）。
- `scan_text` — 从 hook payload 抽待扫描文本（claude/codex/kimi/grok 信封，snake_case

## Types

- `Finding` — Finding：密钥拦截闸的数据面。
- `GuardVerdict` — guard 判定：`block` 为真时调用方 exit 2；`reasons` 面向 agent stderr。
- `Tier` — Tier：密钥拦截闸的取值集。

