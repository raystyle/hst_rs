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
- `discover_gateway` — discover_gateway：活性诊断族的公开入口（行为细则与 marker 见 R002）。
- `line_of` — 别名线归属：`-codex` 尾走 /v1/responses，其余（`-claude` 尾或裸名）走
- `list_models` — GET `<base>/v1/models`，返回别名 id 清单（同时就是 key 活性判据）。
- `probe_alias` — 探测一个别名：同 payload 至多三连（网关两连可能异区只写不读——上游
- `render_cache_rows` — kv 行渲染（`hst diagnose cache` 输出面）。
- `run_agents` — `hst diagnose agents` 主流程：配置指向、别名在册、key 活性、thinking
- `run_cache` — `hst diagnose cache` 主流程。aliases 为空 = /v1/models 全量。
- `verdict_from_usages` — 纯函数：多连 usage 判 verdict（读到即 hit；只写不断连 = write-only，

## Types

- `CacheVerdict` — CacheVerdict：活性诊断族的取值集。
- `Gateway` — 网关凭据与入口。
- `Line` — Line：活性诊断族的取值集。

