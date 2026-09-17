# hst-cli::diagnose

`hst diagnose` 活性诊断族：网关缓存命中矩阵与配置活性（D21）。
oma diagnose：活性诊断族（D21，hst-rs#8 / ohmycloud D45 配套）。
与 doctor 的契约分家：这里打真 API（api 缓存回归测试端点，配置注入：
`HST_GATEWAY_URL` / agent 侧配置，端点不落仓面明文）、烧最小
token（每别名两条极短 prompt）、有网络延迟；doctor 保持零网络零 token。
凭据只读 agent 侧原生配置（D45 模板下发形态），不新建存储（D20 口径）：
claude `~/.claude/settings.json` env（ANTHROPIC_BASE_URL / AUTH_TOKEN）、
codex `~/.codex/config.toml`（model_providers.base_url）加 auth.json
（OPENAI_API_KEY）；`HST_GATEWAY_URL` / `HST_GATEWAY_KEY` 环境覆盖（联
调与测试通道）。

## Functions

- `claude_env` — claude `~/.claude/settings.json` 的 env 表（整表读，diagnose agents 要多键）。
- `codex_gateway` — codex 网关：config.toml 激活 provider 的 base_url（剥 /v1 尾）加 auth.json
- `codex_key` — codex auth.json 的 key（活性单独探测用）。
- `codex_view` — codex 激活模型与 provider base_url（diagnose agents 检测面）。
- `discover_gateway` — # Errors
- `line_of` — 活性诊断的别名线归属面（细则见模块文档与集成测试）。
- `list_models` — # Errors
- `probe_alias` — 活性诊断的探测一个别名面（细则见模块文档与集成测试）。
- `render_cache_rows` — kv 行渲染（`hst diagnose cache` 输出面）。
- `run_agents` — # Errors
- `run_cache` — # Errors
- `verdict_from_usages` — 活性诊断的纯函数面（细则见模块文档与集成测试）。

## Types

- `CacheVerdict` — 活性诊断的CacheVerdict面（细则见模块文档与集成测试）。
- `Gateway` — 网关凭据与入口。
- `Line` — 活性诊断的Line面（细则见模块文档与集成测试）。

