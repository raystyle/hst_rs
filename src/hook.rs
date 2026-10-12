use std::env;
use std::io::{self, Read};
use std::path::Path;

use serde_json::Value as Json;

use crate::pathutil::keys_match;

/// Map a hook event (already normalized) to a four-state label.
pub fn map_event(event: &str) -> &'static str {
    match event {
        "session" | "sessionstart" | "idle" | "stop" | "interrupt" | "sessionend" => "idle",
        "userpromptsubmit" | "userpromptuse" | "pretooluse" | "posttooluse"
        | "posttoolusefailure" | "subagentstart" | "subagentstop" | "precompact"
        | "permissionresult" | "working" => "working",
        "permissionrequest" | "blocked" => "blocked",
        // REQ-030：非权限类通知映射 idle（payload 细分见 state_for_payload）。
        "notification" => "idle",
        "unknown" => "unknown",
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

/// 事件标签公共解析（REQ-047 goal 快道共用）：事件参数形直 normalize,
/// 无参数走 payload 形状推断（同 state 腿),两路皆空得空串。
pub fn event_label(event_arg: Option<&str>, payload: Option<&Json>) -> String {
    if let Some(arg) = event_arg {
        normalize(arg)
    } else if let Some(v) = payload {
        event_from_payload(v)
    } else {
        String::new()
    }
}

/// Claude Notification is mixed (tips vs permission). Only permission-shaped
/// kinds count as blocked; the rest map idle（REQ-030：干完活等人看）.
pub fn state_for_payload(event: &str, payload: Option<&Json>) -> &'static str {
    if event == "notification" {
        if let Some(v) = payload {
            let kind = notification_kind(v);
            if kind.contains("permission") || kind.contains("elicitation") {
                return "blocked";
            }
        }
        // REQ-030：非权限类通知（任务完成等）映射 idle（用户令 2026-09-27
        //「为什么是灰色」——unknown 灰数小时误导，干完活等人看是 idle）。
        return "idle";
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

/// 读 stdin 全量为 JSON（空串与坏形返回 None；REQ-045 起共用给 goal 腿
/// 入口）。
pub fn read_stdin_json() -> Option<Json> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// token 腿入口（`hst hook token`）：只做 secretguard 密钥扫描（block 级
/// 由调用方 exit 2），不写盘。entropy-01（2026-10-12 用户令）：hst-state
/// 腿（四态写盘 `~/.hst/state/`）整体退役——herdr 的 herdr-agent-state
/// hook 已覆盖会话状态同步，本模块只保留事件解析公共件与 guard 面。
///
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
pub fn run_token(
    event_arg: Option<&str>,
    agent_arg: Option<&str>,
) -> Result<Option<crate::secretguard::GuardVerdict>, String> {
    let payload = if event_arg.is_some() {
        None
    } else {
        read_stdin_json()
    };
    Ok(run_token_with_payload(event_arg, agent_arg, payload))
}

/// Test seam: token 腿（REQ-028）。None = 该事件不属 guard 扫描面；Some
/// 内 block=true 时调用方 exit 2。
pub(crate) fn run_token_with_payload(
    event_arg: Option<&str>,
    _agent_arg: Option<&str>,
    payload: Option<Json>,
) -> Option<crate::secretguard::GuardVerdict> {
    if !project_allows(payload.as_ref()) {
        return None;
    }
    let event = if let Some(arg) = event_arg {
        normalize(arg)
    } else if let Some(ref v) = payload {
        event_from_payload(v)
    } else {
        String::new()
    };
    if event.is_empty() {
        return None;
    }
    // 密钥 guard（fail-open；与状态通道互相独立，state 文件推不出来也照拦）。
    if matches!(
        event.as_str(),
        "pretooluse" | "userpromptsubmit" | "posttooluse"
    ) {
        Some(crate::secretguard::guard(&event, payload.as_ref()))
    } else {
        None
    }
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
        assert_eq!(map_event("notification"), "idle");
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
        // REQ-030：非权限类通知映射 idle（原 unknown，用户实报灰数小时）。
        assert_eq!(state_for_payload("notification", Some(&tips)), "idle");
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
        // 运行时拼接构造 token（防线 5：测试语料不落字面密钥，oma 源码不
        // 被自家 guard 误伤）。
        let tok = format!("{}{}", "ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": format!("curl -H \"Authorization: Bearer {tok}\" https://x") },
        });
        let g = run_token_with_payload(None, Some("claude"), Some(payload)).expect("guard ran");
        assert!(g.block, "reasons: {:?}", g.reasons);
    }
}
