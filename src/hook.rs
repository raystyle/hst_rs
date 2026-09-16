use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as Json};

use crate::pathutil::keys_match;

/// Map a hook event (already normalized) to a four-state label.
pub fn map_event(event: &str) -> &'static str {
    match event {
        "session" | "sessionstart" | "idle" | "stop" | "interrupt" | "sessionend" => "idle",
        "userpromptsubmit" | "userpromptuse" | "pretooluse" | "posttooluse"
        | "posttoolusefailure" | "subagentstart" | "subagentstop" | "precompact"
        | "permissionresult" | "working" => "working",
        "permissionrequest" | "blocked" => "blocked",
        "notification" | "unknown" => "unknown",
        _ => "unknown",
    }
}

fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn event_from_payload(v: &Json) -> String {
    let raw = v
        .get("hook_event_name")
        .or_else(|| v.get("hookEventName"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if !raw.is_empty() {
        return normalize(raw);
    }
    // codex 信封无事件名字段：从 payload 形状推断（对齐 ohmypwsh
    // secret-guard 的 _detect_event）。
    if v.get("tool_response").is_some()
        || v.get("toolResponse").is_some()
        || v.get("output").is_some()
    {
        return "posttooluse".into();
    }
    if v.get("tool_name").is_some() || v.get("toolName").is_some() {
        return "pretooluse".into();
    }
    if v.get("prompt").is_some() || v.get("userPrompt").is_some() {
        return "userpromptsubmit".into();
    }
    String::new()
}

fn notification_kind(v: &Json) -> String {
    let raw = v
        .get("notification_type")
        .or_else(|| v.get("notificationType"))
        .or_else(|| v.get("matcher"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    normalize(raw)
}

/// Claude Notification is mixed (tips vs permission). Only permission-shaped
/// kinds count as blocked; the rest stay unknown so we do not spur idle.
pub fn state_for_payload(event: &str, payload: Option<&Json>) -> &'static str {
    if event == "notification" {
        if let Some(v) = payload {
            let kind = notification_kind(v);
            if kind.contains("permission") || kind.contains("elicitation") {
                return "blocked";
            }
        }
        return "unknown";
    }
    if event == "elicitation" || event == "elicitationresult" {
        return "blocked";
    }
    map_event(event)
}

fn env_nonempty(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

fn project_allows(payload: Option<&Json>) -> bool {
    let Some(project) = env_nonempty("HST_PROJECT") else {
        return true;
    };
    let Some(v) = payload else {
        return true;
    };
    let Some(cwd) = v.get("cwd").and_then(|x| x.as_str()) else {
        return true;
    };
    keys_match(cwd, &project)
        || cwd
            .replace('\\', "/")
            .starts_with(&project.replace('\\', "/"))
        || Path::new(cwd).starts_with(&project)
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn atomic_write(path: &Path, body: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
}

fn read_stdin_json() -> Option<Json> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// hook 出口：状态通道 + 密钥 guard（S030 第二职责）。
#[derive(Debug, Default)]
pub struct HookOutcome {
    /// 该字段承载hook 状态的state_file数据。
    pub state_file: Option<PathBuf>,
    /// None = 该事件不属 guard 扫描面；Some 内 block=true 时调用方 exit 2。
    pub guard: Option<crate::secretguard::GuardVerdict>,
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Hook entry: always exit-path friendly. `HST_STATE_FILE` 覆盖互斥
/// 单写（verify 与测试）；缺省走用户级 session 分键通道（D28）：写
/// `~/.hst/state/<agent>.json`（agent 最新）加 `<agent>-<session>.json`
/// （session 键，状态栏按当前会话直读；session 取 payload session_id /
/// sessionId，grok 回退 GROK_SESSION_ID env）。SessionEnd 删本 session 键
/// 文件（GC），顺带清扫同 agent 超 7 天的陈旧键文件（崩溃残留）。
pub fn run(event_arg: Option<&str>, agent_arg: Option<&str>) -> Result<HookOutcome, String> {
    let payload = if event_arg.is_some() {
        None
    } else {
        read_stdin_json()
    };
    run_with_payload(event_arg, agent_arg, payload)
}

/// 陈旧 session 键文件判定阈值（写入侧顺带清扫崩溃残留；SessionEnd 正常
/// 路径自删）。
const STALE_SESSION_SECS: u64 = 7 * 24 * 3600;

/// 清扫 `<dir>/<agent>-*.json` 中 mtime 超阈值的键文件（best-effort）。
fn sweep_stale_sessions(dir: &Path, agent: &str) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    let prefix = format!("{agent}-");
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(STALE_SESSION_SECS))
        .unwrap_or(0);
    for ent in rd.flatten() {
        let p = ent.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with(&prefix) || !name.ends_with(".json") {
            continue;
        }
        let stale = ent
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .is_some_and(|age| age.as_secs() < cutoff);
        if stale {
            let _ = fs::remove_file(&p);
        }
    }
}

/// Test seam: payload injected instead of read from stdin.
pub(crate) fn run_with_payload(
    event_arg: Option<&str>,
    agent_arg: Option<&str>,
    payload: Option<Json>,
) -> Result<HookOutcome, String> {
    if !project_allows(payload.as_ref()) {
        return Ok(HookOutcome::default());
    }
    let agent = env_nonempty("HST_AGENT")
        .or_else(|| agent_arg.map(str::to_string))
        .unwrap_or_default();
    let event = if let Some(arg) = event_arg {
        normalize(arg)
    } else if let Some(ref v) = payload {
        event_from_payload(v)
    } else {
        String::new()
    };
    if event.is_empty() {
        return Ok(HookOutcome::default());
    }
    // 密钥 guard（fail-open；与状态通道互相独立——state 文件推不出来也照拦）。
    let guard = if matches!(
        event.as_str(),
        "pretooluse" | "userpromptsubmit" | "posttooluse"
    ) {
        Some(crate::secretguard::guard(&event, payload.as_ref()))
    } else {
        None
    };
    let session = payload
        .as_ref()
        .and_then(|v| {
            v.get("session_id")
                .or_else(|| v.get("sessionId"))
                .and_then(|x| x.as_str())
                .map(str::to_string)
        })
        .or_else(|| env_nonempty("GROK_SESSION_ID"))
        .unwrap_or_default();
    let state = state_for_payload(&event, payload.as_ref());
    let record = json!({
        "state": state,
        "event": event,
        "agent": agent,
        "session": session,
        "ts": unix_secs(),
    });
    let body = serde_json::to_string(&record).map_err(|e| e.to_string())? + "\n";
    let wrote = if let Some(file) = env_nonempty("HST_STATE_FILE").map(PathBuf::from) {
        atomic_write(&file, &body)?;
        Some(file)
    } else if agent.is_empty() {
        None
    } else if let Ok(oma) = crate::install::hst_home() {
        // 用户级 session 分键通道（D28）：双写 agent 最新 + session 键。
        let dir = oma.join("state");
        let latest = dir.join(format!("{agent}.json"));
        atomic_write(&latest, &body)?;
        if !session.is_empty() {
            let keyed = dir.join(format!("{agent}-{session}.json"));
            atomic_write(&keyed, &body)?;
            // SessionEnd GC：会话已终，键文件即删（最新键保留终态）。
            if event == "sessionend" {
                let _ = fs::remove_file(&keyed);
            }
        }
        sweep_stale_sessions(&dir, &agent);
        Some(latest)
    } else {
        None
    };
    Ok(HookOutcome {
        state_file: wrote,
        guard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathutil::ENV_LOCK;
    use serde_json::json;

    #[test]
    fn maps_core_events() {
        assert_eq!(map_event("userpromptsubmit"), "working");
        assert_eq!(map_event("stop"), "idle");
        assert_eq!(map_event("permissionrequest"), "blocked");
        assert_eq!(map_event("notification"), "unknown");
        assert_eq!(map_event("sessionstart"), "idle");
    }

    #[test]
    fn permission_notification_is_blocked() {
        let v = json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt"
        });
        assert_eq!(state_for_payload("notification", Some(&v)), "blocked");
        let tips = json!({
            "hookEventName": "notification",
            "notificationType": "idle_prompt"
        });
        assert_eq!(state_for_payload("notification", Some(&tips)), "unknown");
    }

    #[test]
    fn grok_camel_case_normalizes() {
        let v = json!({ "hookEventName": "user_prompt_submit" });
        assert_eq!(event_from_payload(&v), "userpromptsubmit");
        assert_eq!(
            state_for_payload(&event_from_payload(&v), Some(&v)),
            "working"
        );
    }

    #[test]
    fn run_is_silent_without_env_or_agent() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("HST_STATE_FILE");
        env::remove_var("HST_AGENT");
        env::remove_var("HST_ROOT");
        // 无 agent 名即无状态文件可落（event arg 短路 stdin）。
        assert_eq!(run(Some("blocked"), None).unwrap().state_file, None);
    }

    #[test]
    fn run_writes_blocked() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir =
            std::env::temp_dir().join(format!("hst-hook-{}-{}", std::process::id(), unix_secs()));
        let file = dir.join("claude.json");
        env::set_var("HST_STATE_FILE", &file);
        env::set_var("HST_AGENT", "claude");
        let wrote = run(Some("PermissionRequest"), None).unwrap();
        env::remove_var("HST_STATE_FILE");
        env::remove_var("HST_AGENT");
        assert_eq!(wrote.state_file.as_deref(), Some(file.as_path()));
        let v: Json = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["state"], "blocked");
        assert_eq!(v["event"], "permissionrequest");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_writes_user_level_session_keyed_pair_without_env() {
        // D28 用户级 session 分键：HST_ROOT 缝注入临时根，双写
        // <agent>.json 加 <agent>-<session>.json；HookOutcome 报最新键。
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("HST_STATE_FILE");
        env::remove_var("HST_AGENT");
        let oma = std::env::temp_dir().join(format!(
            "hst-hook-user-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        env::set_var("HST_ROOT", &oma);
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "cwd": "D:\\anywhere",
            "session_id": "sess-42",
        });
        let wrote = run_with_payload(None, Some("claude"), Some(payload)).unwrap();
        let state = oma.join("state");
        assert_eq!(
            wrote.state_file.as_deref(),
            Some(state.join("claude.json").as_path())
        );
        let latest: Json =
            serde_json::from_str(&fs::read_to_string(state.join("claude.json")).unwrap()).unwrap();
        assert_eq!(latest["state"], "working");
        assert_eq!(latest["session"], "sess-42");
        let keyed: Json =
            serde_json::from_str(&fs::read_to_string(state.join("claude-sess-42.json")).unwrap())
                .unwrap();
        assert_eq!(
            keyed["state"], "working",
            "session-keyed twin carries state"
        );
        // grok 回退：payload 无 session 时取 GROK_SESSION_ID env。
        env::set_var("GROK_SESSION_ID", "g-sess-7");
        let payload = json!({ "hookEventName": "user_prompt_submit" });
        run_with_payload(None, Some("grok"), Some(payload)).unwrap();
        let keyed: Json =
            serde_json::from_str(&fs::read_to_string(state.join("grok-g-sess-7.json")).unwrap())
                .unwrap();
        assert_eq!(keyed["state"], "working");
        env::remove_var("GROK_SESSION_ID");
        env::remove_var("HST_ROOT");
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn session_end_removes_keyed_file_and_sweep_clears_stale() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("HST_STATE_FILE");
        env::remove_var("HST_AGENT");
        let oma = std::env::temp_dir().join(format!(
            "hst-hook-gc-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        env::set_var("HST_ROOT", &oma);
        // SessionEnd：键文件写入即删（终态留最新键）。
        let payload = json!({ "hook_event_name": "SessionEnd", "session_id": "s-end" });
        run_with_payload(None, Some("claude"), Some(payload)).unwrap();
        let state = oma.join("state");
        assert!(
            !state.join("claude-s-end.json").exists(),
            "keyed GC'd on SessionEnd"
        );
        let latest: Json =
            serde_json::from_str(&fs::read_to_string(state.join("claude.json")).unwrap()).unwrap();
        assert_eq!(latest["state"], "idle", "latest keeps the terminal state");
        // 陈旧清扫：mtime 拨回 8 天前的键文件被清，新鲜键文件与其它 agent 不动。
        let stale = state.join("claude-s-old.json");
        fs::write(&stale, "{}\n").unwrap();
        let fresh = state.join("claude-s-fresh.json");
        fs::write(&fresh, "{}\n").unwrap();
        let foreign = state.join("grok-s-old.json");
        fs::write(&foreign, "{}\n").unwrap();
        let old = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
        set_mtime(&stale, old);
        set_mtime(&foreign, old);
        let payload = json!({ "hook_event_name": "UserPromptSubmit", "session_id": "s-2" });
        run_with_payload(None, Some("claude"), Some(payload)).unwrap();
        assert!(!stale.exists(), "stale keyed file swept");
        assert!(fresh.exists(), "fresh keyed file survives");
        assert!(foreign.exists(), "other agents' files untouched");
        env::remove_var("HST_ROOT");
        let _ = fs::remove_dir_all(&oma);
    }

    /// mtime 拨回（仅测试用；Windows 与 Unix 都走 std FileTimes）。
    fn set_mtime(path: &Path, t: SystemTime) {
        let f = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open for set_times");
        f.set_modified(t).expect("set_modified");
    }

    #[test]
    fn codex_envelope_infers_event_from_payload_shape() {
        // codex 无 hook_event_name：tool_name → pretooluse（状态面 working）。
        let v = json!({ "tool_name": "Bash", "tool_input": { "command": "git status" } });
        assert_eq!(event_from_payload(&v), "pretooluse");
        let post = json!({ "tool_response": "ok" });
        assert_eq!(event_from_payload(&post), "posttooluse");
        let prompt = json!({ "prompt": "hi" });
        assert_eq!(event_from_payload(&prompt), "userpromptsubmit");
        // hook_event_name 恒优先。
        let named = json!({ "hook_event_name": "Stop", "tool_name": "Bash" });
        assert_eq!(event_from_payload(&named), "stop");
    }

    #[test]
    fn guard_blocks_secret_in_pretooluse_command() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("HST_STATE_FILE");
        env::remove_var("HST_AGENT");
        let oma = std::env::temp_dir().join(format!(
            "hst-hook-guard-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        env::set_var("HST_ROOT", &oma);
        // 运行时拼接构造 token（防线 5：测试语料不落字面密钥，oma 源码不
        // 被自家 guard 误伤）。
        let tok = format!("{}{}", "ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": format!("curl -H \"Authorization: Bearer {tok}\" https://x") },
        });
        let out = run_with_payload(None, Some("claude"), Some(payload)).unwrap();
        env::remove_var("HST_ROOT");
        let g = out.guard.expect("guard ran");
        assert!(g.block, "reasons: {:?}", g.reasons);
        let _ = fs::remove_dir_all(&oma);
    }
}
