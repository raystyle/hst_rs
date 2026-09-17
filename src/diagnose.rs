//! oma diagnose：活性诊断族（D21，hst-rs#8 / ohmycloud D45 配套）。
//! 与 doctor 的契约分家：这里打真 API（api 缓存回归测试端点，配置注入：
//! `HST_GATEWAY_URL` / agent 侧配置，端点不落仓面明文）、烧最小
//! token（每别名两条极短 prompt）、有网络延迟；doctor 保持零网络零 token。
//! 凭据只读 agent 侧原生配置（D45 模板下发形态），不新建存储（D20 口径）：
//! claude `~/.claude/settings.json` env（ANTHROPIC_BASE_URL / AUTH_TOKEN）、
//! codex `~/.codex/config.toml`（model_providers.base_url）加 auth.json
//! （OPENAI_API_KEY）；`HST_GATEWAY_URL` / `HST_GATEWAY_KEY` 环境覆盖（联
//! 调与测试通道）。

use std::io::Read;

use serde_json::{json, Value};

/// 活性诊断的探测用长文本面（细则见模块文档与集成测试）。
/// 确定性生成（同 payload 双连才能命中前缀缓存）。
fn cache_text() -> String {
    let para = "oma cache probe deterministic filler block. prefix caching requires a stable long system prefix repeated verbatim across two connections. ";
    // 96 块约 2000 token：稳过两线缓存门槛（claude ephemeral 与 responses
    // 前缀缓存都要求 1024 token 起；实测 40 块 = 1005 token 差 1 不命中）。
    let mut s = String::with_capacity(para.len() * 96);
    for i in 0..96 {
        s.push_str(&format!("block-{i:03} {para}"));
    }
    s
}

/// 已实测 thinking budget 上限（ohmycloud D45 阶梯实测：fable5 与 opus48
/// 8192 至 65536 全收、已顶格 65536；其余别名上限未知，只报 info）。
fn thinking_cap(model: &str) -> Option<u64> {
    let base = model.trim_end_matches("-claude").trim_end_matches("-codex");
    match base {
        "zy-claudefable5" | "zy-claudeopus48" => Some(65536),
        _ => None,
    }
}

/// 网关凭据与入口。
pub struct Gateway {
    /// 该字段承载活性诊断的base_url数据。
    pub base_url: String,
    /// 该字段承载活性诊断的key数据。
    pub key: String,
    /// 该字段承载活性诊断的source数据。
    pub source: &'static str,
}

/// 活性诊断的网关发现面（细则见模块文档与集成测试）。
fn read_env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 活性诊断的discover_gateway面（细则见模块文档与集成测试）。
pub fn discover_gateway() -> Result<Gateway, String> {
    // D45 oma 遗产清扫：旧 OMA_GATEWAY_* 兼容读已删（1.1.0 窗口已过）。
    let env_url = read_env_nonempty("HST_GATEWAY_URL");
    let env_key = read_env_nonempty("HST_GATEWAY_KEY");
    if let (Some(u), Some(k)) = (env_url, env_key) {
        return Ok(Gateway {
            base_url: u.trim_end_matches('/').to_string(),
            key: k,
            source: "env",
        });
    }
    let env_map = claude_env();
    let cu = env_map.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str());
    let ck = env_map.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str());
    if let (Some(u), Some(k)) = (cu, ck) {
        if !u.is_empty() && !k.is_empty() {
            return Ok(Gateway {
                base_url: u.trim_end_matches('/').to_string(),
                key: k.to_string(),
                source: "claude-config",
            });
        }
    }
    if let Some((base, key)) = codex_gateway() {
        return Ok(Gateway {
            base_url: base,
            key,
            source: "codex-config",
        });
    }
    Err(
        "no gateway config found: set HST_GATEWAY_URL/HST_GATEWAY_KEY, or point claude/codex config at the gateway (ohmycloud D45 模板下发) 后重跑"
            .into(),
    )
}

/// claude `~/.claude/settings.json` 的 env 表（整表读，diagnose agents 要多键）。
pub fn claude_env() -> serde_json::Map<String, Value> {
    let Some(home) = dirs::home_dir() else {
        return serde_json::Map::new();
    };
    let path = home.join(".claude").join("settings.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return serde_json::Map::new();
    };
    // D52：BOM 容忍（同 yolo.rs read_json）。
    let Ok(v) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
        return serde_json::Map::new();
    };
    v.get("env")
        .and_then(|e| e.as_object())
        .cloned()
        .unwrap_or_default()
}

/// codex 网关：config.toml 激活 provider 的 base_url（剥 /v1 尾）加 auth.json
/// 的 OPENAI_API_KEY。任一缺失返回 None。
pub fn codex_gateway() -> Option<(String, String)> {
    let home = dirs::home_dir()?;
    let text = std::fs::read_to_string(home.join(".codex").join("config.toml")).ok()?;
    // D52（codex F1）：BOM 容忍（同 json 读侧）。
    let cfg: toml::Value = toml::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let provider = cfg.get("model_provider")?.as_str()?;
    let base = cfg
        .get("model_providers")?
        .get(provider)?
        .get("base_url")?
        .as_str()?;
    let auth: Value = serde_json::from_str(
        std::fs::read_to_string(home.join(".codex").join("auth.json"))
            .ok()?
            .trim_start_matches('\u{feff}'),
    )
    .ok()?;
    let key = auth.get("OPENAI_API_KEY")?.as_str()?;
    if base.is_empty() || key.is_empty() {
        return None;
    }
    let base = base
        .trim_end_matches('/')
        .trim_end_matches("/v1")
        .to_string();
    Some((base, key.to_string()))
}

/// codex 激活模型与 provider base_url（diagnose agents 检测面）。
pub fn codex_view() -> (Option<String>, Option<String>) {
    let Some(home) = dirs::home_dir() else {
        return (None, None);
    };
    let Ok(text) = std::fs::read_to_string(home.join(".codex").join("config.toml")) else {
        return (None, None);
    };
    let Ok(cfg) = toml::from_str::<toml::Value>(text.trim_start_matches('\u{feff}')) else {
        return (None, None);
    };
    let model = cfg
        .get("model")
        .and_then(|m| m.as_str())
        .map(str::to_string);
    let provider = cfg.get("model_provider").and_then(|p| p.as_str());
    let base = provider.and_then(|p| {
        cfg.get("model_providers")
            .and_then(|mp| mp.get(p))
            .and_then(|x| x.get("base_url"))
            .and_then(|b| b.as_str())
            .map(str::to_string)
    });
    (model, base)
}

// ---- HTTP ----

fn auth_headers(req: ureq::Request, key: &str, claude_line: bool) -> ureq::Request {
    let req = req.set("Authorization", &format!("Bearer {key}"));
    if claude_line {
        req.set("x-api-key", key)
            .set("anthropic-version", "2023-06-01")
    } else {
        req
    }
}

fn parse_body(resp: ureq::Response) -> Result<Value, String> {
    let mut buf = String::new();
    resp.into_reader()
        .take(1 << 20)
        .read_to_string(&mut buf)
        .map_err(|e| format!("read body: {e}"))?;
    serde_json::from_str(&buf).map_err(|e| format!("decode body: {e}"))
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// GET `<base>/v1/models`，返回别名 id 清单（同时就是 key 活性判据）。
pub fn list_models(gw: &Gateway) -> Result<Vec<String>, String> {
    let req = ureq::get(&format!("{}/v1/models", gw.base_url)).timeout(Duration::from_secs(30));
    let resp = auth_headers(req, &gw.key, false)
        .call()
        .map_err(|e| format!("GET /v1/models: {e}"))?;
    if resp.status() != 200 {
        return Err(format!("GET /v1/models -> HTTP {}", resp.status()));
    }
    let v = parse_body(resp)?;
    let mut out = Vec::new();
    if let Some(data) = v.get("data").and_then(|d| d.as_array()) {
        for m in data {
            if let Some(id) = m.get("id").and_then(|i| i.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    out.sort();
    Ok(out)
}

/// 活性诊断的别名线归属面（细则见模块文档与集成测试）。
/// /v1/messages。
pub fn line_of(alias: &str) -> Line {
    if alias.ends_with("-codex") {
        Line::Codex
    } else {
        Line::Claude
    }
}

/// 活性诊断的Line面（细则见模块文档与集成测试）。
pub enum Line {
    /// 该字段承载活性诊断的Claude数据。
    Claude,
    /// 该字段承载活性诊断的Codex数据。
    Codex,
}

impl Line {
    fn name(&self) -> &'static str {
        match self {
            Line::Claude => "claude",
            Line::Codex => "codex",
        }
    }
}

/// 活性诊断的CacheVerdict面（细则见模块文档与集成测试）。
pub enum CacheVerdict {
    /// Hit {：活性诊断族公开项。
    Hit {
        /// created: u64,：活性诊断族公开项。
        created: u64,
        /// read: u64,：活性诊断族公开项。
        read: u64,
    },
    /// WriteOnly {：活性诊断族公开项。
    WriteOnly {
        /// created: u64,：活性诊断族公开项。
        created: u64,
    },
    /// ds 官方 anthropic 端点走全自动前缀匹配，usage 不透传缓存字段：
    /// 判「自动前缀（不可见）」而非「无缓存」（D21 用户科普修正）。
    AutoPrefix,
    /// 该字段承载活性诊断的None数据。
    None,
    /// Error(String),：活性诊断族公开项。
    Error(String),
}

impl CacheVerdict {
    /// 活性诊断的label面（细则见模块文档与集成测试）。
    pub fn label(&self) -> String {
        match self {
            CacheVerdict::Hit { created, read } => {
                format!("hit created={created} read={read}")
            }
            CacheVerdict::WriteOnly { created } => format!("write-only created={created}"),
            CacheVerdict::AutoPrefix => "auto-prefix".into(),
            CacheVerdict::None => "none".into(),
            CacheVerdict::Error(e) => format!("error({e})"),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            CacheVerdict::Hit { .. } => "hit",
            CacheVerdict::WriteOnly { .. } => "write-only",
            CacheVerdict::AutoPrefix => "auto-prefix",
            CacheVerdict::None => "none",
            CacheVerdict::Error(_) => "error",
        }
    }
}

fn post_json(url: &str, key: &str, claude_line: bool, body: &Value) -> Result<Value, String> {
    let req = ureq::post(url).timeout(Duration::from_secs(60));
    let resp = auth_headers(req, key, claude_line)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("POST {url}: {e}"))?;
    if resp.status() >= 400 {
        return Err(format!("HTTP {}", resp.status()));
    }
    parse_body(resp)
}

/// 活性诊断的探测一个别名面（细则见模块文档与集成测试）。
/// geo 漂移实测；读到即停，判前缀缓存）。
pub fn probe_alias(gw: &Gateway, alias: &str, line: &Line) -> CacheVerdict {
    let text = cache_text();
    let (url, body) = match line {
        Line::Claude => (
            format!("{}/v1/messages", gw.base_url),
            json!({
                "model": alias,
                "max_tokens": 1,
                "system": [{ "type": "text", "text": text,
                             "cache_control": { "type": "ephemeral" } }],
                "messages": [{ "role": "user", "content": "cache probe" }]
            }),
        ),
        Line::Codex => (
            format!("{}/v1/responses", gw.base_url),
            json!({
                "model": alias,
                "instructions": text,
                "input": "cache probe",
                "max_output_tokens": 16
            }),
        ),
    };
    let mut usages: Vec<Value> = Vec::new();
    let mut last_err: Option<String> = None;
    for attempt in 0..3 {
        match post_json(&url, &gw.key, matches!(line, Line::Claude), &body) {
            Ok(v) => {
                usages.push(v.get("usage").cloned().unwrap_or(Value::Null));
                let v = verdict_from_usages(alias, line, &usages);
                if matches!(v, CacheVerdict::Hit { .. }) {
                    return v;
                }
            }
            Err(e) => last_err = Some(format!("attempt {}: {e}", attempt + 1)),
        }
    }
    if usages.is_empty() {
        return CacheVerdict::Error(last_err.unwrap_or_else(|| "no response".into()));
    }
    verdict_from_usages(alias, line, &usages)
}

/// 活性诊断的纯函数面（细则见模块文档与集成测试）。
/// 网关异区只写不读的实测形态；ds 特判只落在 claude 线）。
pub fn verdict_from_usages(alias: &str, line: &Line, usages: &[Value]) -> CacheVerdict {
    let is_ds = alias.starts_with("ds");
    let field = |u: &Value, k: &str| u.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
    let cached_codex = |u: &Value| {
        u.get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };
    match line {
        Line::Claude => {
            let read = usages
                .iter()
                .map(|u| field(u, "cache_read_input_tokens"))
                .max()
                .unwrap_or(0);
            let created = usages
                .iter()
                .map(|u| field(u, "cache_creation_input_tokens"))
                .max()
                .unwrap_or(0);
            if read > 0 {
                CacheVerdict::Hit { created, read }
            } else if created > 0 {
                CacheVerdict::WriteOnly { created }
            } else if is_ds {
                CacheVerdict::AutoPrefix
            } else {
                CacheVerdict::None
            }
        }
        Line::Codex => {
            let read = usages.iter().map(|u| cached_codex(u)).max().unwrap_or(0);
            let created = usages
                .iter()
                .map(|u| {
                    u.get("input_tokens_details")
                        .and_then(|d| d.get("cache_write_tokens"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            if read > 0 {
                CacheVerdict::Hit { created, read }
            } else if created > 0 {
                CacheVerdict::WriteOnly { created }
            } else {
                CacheVerdict::None
            }
        }
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// `hst diagnose cache` 主流程。aliases 为空 = /v1/models 全量。
pub fn run_cache(aliases: &[String]) -> Result<Vec<(String, Line, CacheVerdict)>, String> {
    let gw = discover_gateway()?;
    let targets: Vec<String> = if aliases.is_empty() {
        list_models(&gw)?
    } else {
        aliases.to_vec()
    };
    let mut out = Vec::with_capacity(targets.len());
    for alias in targets {
        let line = line_of(&alias);
        let verdict = probe_alias(&gw, &alias, &line);
        out.push((alias.clone(), line, verdict));
    }
    Ok(out)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// `hst diagnose agents` 主流程：配置指向、别名在册、key 活性、thinking
/// 对照。返回 kv 行（已排序的 (key, value) 对）。
pub fn run_agents() -> Result<Vec<(String, String)>, String> {
    let gw = discover_gateway()?;
    let models = list_models(&gw).unwrap_or_default();
    let mut rows: Vec<(String, String)> = Vec::new();
    let on_record = |m: &str| models.iter().any(|x| x == m);

    // claude 面
    let env = claude_env();
    let gw_base = gw.base_url.trim_end_matches('/');
    let cbase = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_end_matches('/');
    rows.push((
        "agents.claude.base_url".into(),
        format!(
            "{cbase} state={}",
            if cbase == gw_base {
                "ok"
            } else if cbase.is_empty() {
                "missing"
            } else {
                "warn-not-gateway"
            }
        ),
    ));
    for key in [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    ] {
        if let Some(m) = env.get(key).and_then(|v| v.as_str()) {
            rows.push((
                format!("agents.claude.{key}"),
                format!(
                    "{m} state={}",
                    if on_record(m) {
                        "ok"
                    } else {
                        "warn-not-on-record"
                    }
                ),
            ));
        }
    }
    if let Some(t) = env.get("MAX_THINKING_TOKENS").and_then(|v| v.as_str()) {
        let want: u64 = t.trim().parse().unwrap_or(0);
        let model = env
            .get("ANTHROPIC_MODEL")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let state = match (thinking_cap(model), want) {
            (Some(cap), w) if w > 0 && w <= cap => format!("ok cap={cap}"),
            (Some(cap), w) if w > cap => format!("warn-over-cap cap={cap}"),
            (_, 0) => "warn-unparsable".into(),
            _ => "info-cap-unknown".into(),
        };
        rows.push((
            "agents.claude.thinking".into(),
            format!("{t} state={state}"),
        ));
    }

    // codex 面（key 活性用 codex 自己的 auth.json 单独打一发）
    let (model, base) = codex_view();
    let cbase2 = base.as_deref().unwrap_or("").trim_end_matches('/');
    let cbase2 = cbase2.trim_end_matches("/v1");
    rows.push((
        "agents.codex.base_url".into(),
        format!(
            "{cbase2} state={}",
            if cbase2 == gw_base {
                "ok"
            } else if cbase2.is_empty() {
                "missing"
            } else {
                "warn-not-gateway"
            }
        ),
    ));
    if let Some(m) = &model {
        rows.push((
            "agents.codex.model".into(),
            format!(
                "{m} state={}",
                if on_record(m) {
                    "ok"
                } else {
                    "warn-not-on-record"
                }
            ),
        ));
    }
    let key_state = match codex_key() {
        Some(k) => match list_models(&Gateway {
            base_url: gw.base_url.clone(),
            key: k,
            source: "codex-auth",
        }) {
            Ok(_) => "alive".to_string(),
            Err(e) => format!("dead({e})"),
        },
        None => "missing".to_string(),
    };
    rows.push(("agents.codex.key".into(), key_state));
    rows.push(("diagnose.agents.models".into(), models.len().to_string()));
    Ok(rows)
}

/// codex auth.json 的 key（活性单独探测用）。
pub fn codex_key() -> Option<String> {
    let home = dirs::home_dir()?;
    let text = std::fs::read_to_string(home.join(".codex").join("auth.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("OPENAI_API_KEY")
        .and_then(|k| k.as_str())
        .map(str::to_string)
}

/// kv 行渲染（`hst diagnose cache` 输出面）。
pub fn render_cache_rows(out: &[(String, Line, CacheVerdict)]) -> Vec<String> {
    let mut lines = Vec::new();
    for (alias, line, v) in out {
        lines.push(format!(
            "cache.{alias}.line={} verdict={}",
            line.name(),
            v.label()
        ));
    }
    let hit = out.iter().filter(|(_, _, v)| v.kind() == "hit").count();
    let auto = out
        .iter()
        .filter(|(_, _, v)| v.kind() == "auto-prefix")
        .count();
    let errs = out.iter().filter(|(_, _, v)| v.kind() == "error").count();
    lines.push(format!(
        "cache.summary=hit {hit} auto-prefix {auto} of {} (errors {errs})",
        out.len()
    ));
    lines.push(format!("diagnose.cache.ok={}", errs == 0));
    lines
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_of_classifies_by_suffix() {
        assert!(matches!(line_of("zy-gpt56sol-codex"), Line::Codex));
        assert!(matches!(line_of("ds-v4-claude"), Line::Claude));
        assert!(matches!(line_of("km-k3"), Line::Claude));
    }

    #[test]
    fn verdict_claude_line_shapes() {
        // 期望值来自 ohmycloud D45 实测矩阵语义（issue #8 方法论）。
        let hit = json!({"cache_creation_input_tokens": 2000, "cache_read_input_tokens": 2000});
        let write_only = json!({"cache_creation_input_tokens": 2000});
        let plain = json!({"input_tokens": 2000});
        let mk =
            |v: &Value| verdict_from_usages("zy-x-claude", &Line::Claude, &[v.clone(), v.clone()]);
        assert!(matches!(mk(&hit), CacheVerdict::Hit { .. }));
        assert!(matches!(mk(&write_only), CacheVerdict::WriteOnly { .. }));
        assert!(matches!(mk(&plain), CacheVerdict::None));
        // ds 特判：claude 线无字段判自动前缀（不可见），不是无缓存。
        assert!(matches!(
            verdict_from_usages(
                "ds-v4-claude",
                &Line::Claude,
                &[plain.clone(), plain.clone()]
            ),
            CacheVerdict::AutoPrefix
        ));
    }

    #[test]
    fn verdict_claude_line_write_only_across_connects() {
        // 网关异区只写不读的实测形态：连续 created、read 恒 0 → write-only。
        let c_only = json!({"cache_creation_input_tokens": 1902, "cache_read_input_tokens": 0});
        assert!(matches!(
            verdict_from_usages(
                "zy-claudefable5-claude",
                &Line::Claude,
                &[c_only.clone(), c_only.clone()]
            ),
            CacheVerdict::WriteOnly { created: 1902 }
        ));
        // 第三连读到 → hit（读到即停语义的判据面）。
        let read = json!({"cache_creation_input_tokens": 0, "cache_read_input_tokens": 1792});
        assert!(matches!(
            verdict_from_usages("zy-claudefable5-claude", &Line::Claude, &[c_only, read]),
            CacheVerdict::Hit { read: 1792, .. }
        ));
    }

    #[test]
    fn verdict_codex_line_reads_cached_tokens() {
        let first = json!({"input_tokens_details": {"cached_tokens": 0}});
        let second = json!({"input_tokens_details": {"cached_tokens": 1800}});
        assert!(matches!(
            verdict_from_usages(
                "zy-gpt56sol-codex",
                &Line::Codex,
                &[first.clone(), second.clone()]
            ),
            CacheVerdict::Hit { read: 1800, .. }
        ));
        assert!(matches!(
            verdict_from_usages(
                "zy-gpt56sol-codex",
                &Line::Codex,
                &[first.clone(), first.clone()]
            ),
            CacheVerdict::None
        ));
    }

    #[test]
    fn thinking_cap_table_covers_measured_models_only() {
        assert_eq!(thinking_cap("zy-claudefable5-claude"), Some(65536));
        assert_eq!(thinking_cap("zy-claudeopus48-claude"), Some(65536));
        assert_eq!(thinking_cap("zy-gpt56sol-codex"), None);
        assert_eq!(thinking_cap("km-k3"), None);
    }

    #[test]
    fn cache_text_is_long_and_stable() {
        assert!(
            cache_text().len() > 4000,
            "must exceed the 1024-token floor"
        );
        assert_eq!(cache_text(), cache_text(), "deterministic across calls");
    }

    #[test]
    fn render_cache_rows_marks_errors_not_ok() {
        let out = vec![
            ("a-claude".to_string(), Line::Claude, CacheVerdict::None),
            (
                "b-codex".to_string(),
                Line::Codex,
                CacheVerdict::Error("x".into()),
            ),
        ];
        let rows = render_cache_rows(&out);
        assert!(rows.iter().any(|l| l.contains("verdict=none")));
        assert!(rows.iter().any(|l| l.contains("errors 1")));
        assert!(rows.iter().any(|l| l == "diagnose.cache.ok=false"));
    }
}
