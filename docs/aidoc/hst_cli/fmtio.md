# hst-cli::fmtio

全局输出三态（kv/json/jsonl）与结构化错误出口契约。
全局输出格式（issue #1 总台集成契约，与 ome S003 同构）：
`--format kv|json|jsonl`，`--json` 为 json 简写（互斥）。kv 是人读
marker 行（缺省）；json 出 `{ok,data|error,meta}` 信封（P0015 三传输
同形——oma 与 ome 裸数据裁决的分道点，三传输复用优先，契约文档记档）；
jsonl 是列表型数据的逐行对象（数据即数据，无信封）。

结构化模式（json/jsonl）下的失败回执分两形：经信封出口的失败（业务
失败转发与 `--filter-output` 响错）双道，stdout 吐 ok:false 信封加 stderr
单行 JSON `{"code":"error","message":...}`；不经信封的失败只走 stderr
单行，stdout 无输出。两者退出码 1；kv 模式错误 `hst: <e>`。
serde_json 开 preserve_order：JSON 字段序与 kv 行序一致（ome S003 实证
教训——默认 BTreeMap 字母序会打乱）。

## Functions

- `envelope` — 响应信封（S016 吸收，原 api.rs；P0011 删除后归位本模块）：CLI `--json`
- `error_exit` — main 错误出口：结构化模式 stderr 单行 JSON，kv 模式人称行；退出码 1。
- `filter_output` — `--filter-output <keys>`：json 信封 data 的键路径过滤（cli-docs 采纳轮）。
- `init` — # Errors
- `mode` — 当前输出格式（kv 缺省）。
- `print_jsonl` — jsonl 模式：逐行对象（无信封）。列表型命令用；非列表命令 jsonl 视同
- `structured` — 结构化模式（错误走单行 JSON、stdout 纯数据）。

## Types

- `Format` — 输出格式三态：kv 人读 marker 行、json 信封、jsonl 逐行对象。

