use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value as Json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use toml::Value as Toml;

use crate::agents;
use crate::pathutil::{abs_display, forward_slash, keys_match, native_slash};
use crate::yolo::kimi_workspace_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 体检结论三态：ok、warn（部署缺口不计败）与 block（退出 1）。
pub enum Status {
    /// 该字段承载体检的Ok数据。
    Ok,
    /// Deploy-diagnosis gap that does not block an interactive run (login
    /// missing, statusline off): surfaced for `hst doctor`,
    /// never counted by `blocked()`.
    Warn,
    /// 该字段承载体检的Block数据。
    Block,
}

impl Status {
    /// 状态的 kv 标记取值（ok/warn/block）。
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Block => "block",
        }
    }
}

#[derive(Debug)]
/// 单条体检发现：agent、检查名、状态、路径与明细。
pub struct Finding {
    /// 该字段承载体检的agent数据。
    pub agent: String,
    /// 该字段承载体检的check数据。
    pub check: &'static str,
    /// 按 agent 与检查名取状态（消费面查单项）。
    pub status: Status,
    /// 该字段承载体检的path数据。
    pub path: String,
    /// 该字段承载体检的detail数据。
    pub detail: String,
}

#[derive(Debug)]
/// 体检结果集：findings 列表与阻断判定。
pub struct Diagnosis {
    /// 该字段承载体检的findings数据。
    pub findings: Vec<Finding>,
}

impl Diagnosis {
    /// 任一发现为 block 即真（doctor 退出码判据）。
    pub fn blocked(&self) -> bool {
        self.findings.iter().any(|f| f.status == Status::Block)
    }

    /// 按 agent 与检查名取状态（消费面查单项）。
    pub fn status(&self, agent: &str, check: &str) -> Option<Status> {
        self.findings
            .iter()
            .find(|f| f.agent == agent && f.check == check)
            .map(|f| f.status)
    }
}

fn json_file(path: &Path) -> Option<Json> {
    let text = fs::read_to_string(path).ok()?;
    // D52：BOM 容忍（舰队 PS 脚本常写 UTF-8 BOM，serde 整体拒收）。
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// 体检的D52面（细则见模块文档与集成测试）。
/// 假报 missing（宿主实弹：带 BOM 的 settings.json 被 doctor 报 missing
/// 误导排障）。
enum JsonFileState {
    Absent,
    Bad,
    Ok(Json),
}

fn json_file_state(path: &Path) -> JsonFileState {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        // codex F3：只有真不在算 Absent；读得出错（如 UTF-16 非 UTF-8）
        // 属 Bad——正是本批要消的「换了编码又假报 missing」形态。
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return JsonFileState::Absent,
        Err(_) => return JsonFileState::Bad,
    };
    match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(v) => JsonFileState::Ok(v),
        Err(_) => JsonFileState::Bad,
    }
}

fn toml_file(path: &Path) -> Option<Toml> {
    let text = fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

fn json_bool(v: &Json, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()) == Some(true)
}

fn push_binary(out: &mut Vec<Finding>, agent: &str) {
    match agents::find(agent) {
        Some(h) => {
            let mut detail = format!("source={}", h.source.as_str());
            if let Some(v) = &h.version {
                detail.push(' ');
                detail.push_str(v);
            }
            if !h.extras.is_empty() {
                detail.push_str(" extras=");
                detail.push_str(
                    &h.extras
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            push(out, agent, "binary", true, &h.path, detail);
        }
        None => push(
            out,
            agent,
            "binary",
            false,
            Path::new(agent),
            "not on PATH, HST_AGENT_PATH, HST_*_BIN, or default locations",
        ),
    }
}

fn push(
    out: &mut Vec<Finding>,
    agent: &str,
    check: &'static str,
    ok: bool,
    path: &Path,
    detail: impl Into<String>,
) {
    let status = if ok { Status::Ok } else { Status::Block };
    push_status(out, agent, check, status, path, detail);
}

fn push_status(
    out: &mut Vec<Finding>,
    agent: &str,
    check: &'static str,
    status: Status,
    path: &Path,
    detail: impl Into<String>,
) {
    out.push(Finding {
        agent: agent.to_string(),
        check,
        status,
        path: path.display().to_string(),
        detail: detail.into(),
    });
}

fn toml_str<'a>(t: &'a Toml, key: &str) -> Option<&'a str> {
    t.get(key).and_then(|v| v.as_str())
}

fn projects_trusted(toml: &Toml, root: &Path) -> bool {
    let Some(projects) = toml.get("projects").and_then(|v| v.as_table()) else {
        return false;
    };
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in projects {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return toml_str(v, "trust_level") == Some("trusted");
        }
    }
    false
}

fn claude_project_entry<'a>(cj: &'a Json, root: &Path) -> Option<&'a Json> {
    let projects = cj.get("projects")?.as_object()?;
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in projects {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return Some(v);
        }
    }
    None
}

fn has_hooks_settings(v: &Json) -> bool {
    v.get("hooks").and_then(|h| h.as_object()).is_some_and(|o| {
        o.values()
            .any(|x| x.as_array().is_some_and(|a| !a.is_empty()))
    })
}

fn project_mcp_configured(root: &Path, settings: Option<&Json>) -> bool {
    root.join(".mcp.json").is_file()
        || settings
            .and_then(|v| v.get("mcpServers"))
            .and_then(|m| m.as_object())
            .is_some_and(|o| !o.is_empty())
}

fn toml_mcp_configured(t: &Toml) -> bool {
    t.get("mcp_servers")
        .and_then(|v| v.as_table())
        .is_some_and(|m| !m.is_empty())
}

fn mcp_servers_approved(v: &Json) -> bool {
    json_bool(v, "enableAllProjectMcpServers")
        || v.get("enabledMcpjsonServers")
            .and_then(|x| x.as_array())
            .is_some_and(|a| a.iter().any(|s| s.as_str().is_some_and(|n| !n.is_empty())))
}

fn skill_dir_is_plugin(skills: &Path) -> bool {
    fs::read_dir(skills)
        .map(|rd| {
            rd.flatten().any(|e| {
                let p = e.path();
                p.join(".claude-plugin").join("plugin.json").is_file()
                    || p.join("plugin.json").is_file()
            })
        })
        .unwrap_or(false)
}

fn dir_nonempty(path: &Path) -> bool {
    path.is_dir()
        && fs::read_dir(path)
            .map(|rd| rd.flatten().next().is_some())
            .unwrap_or(false)
}

fn path_is_file(path: &Path) -> bool {
    path.is_file()
}

fn grok_folder_trusted(toml: &Toml, root: &Path) -> bool {
    let Some(folders) = toml.get("folders").and_then(|v| v.as_table()) else {
        return false;
    };
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in folders {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return v.get("trusted").and_then(|x| x.as_bool()) == Some(true);
        }
    }
    false
}

fn kimi_trust_ok(home: &Path, root: &Path) -> bool {
    let key = kimi_workspace_key(root);
    let file = home.join(".kimi-code").join("workspace-trust").join(&key);
    let Some(v) = json_file(&file) else {
        return false;
    };
    let Some(stored) = v.get("root").and_then(|x| x.as_str()) else {
        return false;
    };
    keys_match(stored, &native_slash(root)) || keys_match(stored, &forward_slash(root))
}

// ===== 登录态（S026 判据）与部署诊断扩展 =====

/// grok 登录态：`~/.grok/auth.json` 是 scope → 凭据 map。判据来自 S026
/// 体检的源码取证加本机文件结构实证面（细则见模块文档与集成测试）。
/// 过期看 `expires_at`（RFC3339），缺省按 `create_time + 30 天`兜底，提前
/// 300s 视过期；过期但 refresh_token 在则 agent 下次运行自动刷新。
fn grok_login_state(v: Option<&Json>, now: OffsetDateTime) -> (Status, String) {
    let Some(map) = v.and_then(|v| v.as_object()) else {
        return (
            Status::Warn,
            "auth.json missing; grok login --device-code".into(),
        );
    };
    for (scope, cred) in map {
        let has_key = cred
            .get("key")
            .and_then(|x| x.as_str())
            .is_some_and(|s| !s.is_empty());
        let has_refresh = cred
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .is_some_and(|s| !s.is_empty());
        if !has_key && !has_refresh {
            continue;
        }
        let raw_exp = cred.get("expires_at").and_then(|x| x.as_str());
        let expires = raw_exp
            .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
            .or_else(|| {
                let created = cred.get("create_time").and_then(|x| x.as_str())?;
                OffsetDateTime::parse(created, &Rfc3339)
                    .ok()
                    .map(|t| t + time::Duration::days(30))
            });
        return match expires {
            Some(e) if e > now + time::Duration::seconds(300) => (
                Status::Ok,
                raw_exp
                    .map(|s| format!("scope={scope} expires_at={s}"))
                    .unwrap_or_else(|| format!("scope={scope}")),
            ),
            Some(_) if has_refresh => (
                Status::Warn,
                format!("scope={scope} expired; refresh_token present (auto-refresh on next run)"),
            ),
            Some(_) => (
                Status::Warn,
                format!("scope={scope} expired, no refresh_token; grok login --device-code"),
            ),
            None => (
                Status::Ok,
                format!("scope={scope} no expiry timestamps (treated live)"),
            ),
        };
    }
    (
        Status::Warn,
        "no credential entries; grok login --device-code".into(),
    )
}

/// kimi 登录态：`~/.kimi-code/credentials/kimi-code.json`。判据来自 S026
/// 体检的源码取证面（细则见模块文档与集成测试）。
/// 阈值自动做）；空串是 401/403 墓碑（吊销态，需重登）；expires_at 是
/// Unix 秒。
fn kimi_login_state(v: Option<&Json>, now_secs: i64) -> (Status, String) {
    let Some(v) = v else {
        return (Status::Warn, "credentials file missing; kimi login".into());
    };
    match v.get("access_token").and_then(|x| x.as_str()) {
        Some(t) if !t.is_empty() => {
            let detail = match v.get("expires_at").and_then(|x| x.as_i64()) {
                Some(e) if e > now_secs => {
                    format!("access_token present, expires in {}s", e - now_secs)
                }
                Some(e) => format!(
                    "access_token present, expired {}s ago (auto-refresh threshold)",
                    now_secs - e
                ),
                None => "access_token present, no expires_at".to_string(),
            };
            (Status::Ok, detail)
        }
        Some(_) => (
            Status::Warn,
            "access_token empty-string tombstone (revoked); kimi login again".into(),
        ),
        None => (Status::Warn, "no access_token field; kimi login".into()),
    }
}

/// 当下登录态（grok/kimi）：只读诊断落盘凭据（登录引导面已随 D20 移除）。
pub(crate) fn login_state(agent: &str) -> Option<(Status, String)> {
    let home = crate::pathutil::user_home().ok()?;
    match agent {
        "grok" => {
            let p = home.join(".grok").join("auth.json");
            Some(grok_login_state(
                json_file(&p).as_ref(),
                OffsetDateTime::now_utc(),
            ))
        }
        "kimi" => {
            let p = home
                .join(".kimi-code")
                .join("credentials")
                .join("kimi-code.json");
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            Some(kimi_login_state(json_file(&p).as_ref(), now))
        }
        _ => None,
    }
}

// ===== 状态栏形态（S025 落位） =====

/// 体检的状态栏脚本标记面（细则见模块文档与集成测试）。
const STATUSLINE_MARKER: &str = "hst-statusline";
const STATUSLINE_MARKER_LEGACY: &str = "oma-statusline";

fn has_statusline_marker(c: &str) -> bool {
    c.contains(STATUSLINE_MARKER) || c.contains(STATUSLINE_MARKER_LEGACY)
}

fn claude_statusline_on(home: &Path) -> bool {
    json_file(&home.join(".claude").join("settings.json"))
        .as_ref()
        .and_then(|v| v.get("statusLine"))
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| has_statusline_marker(c))
}

/// Codex `[tui].status_line` kinds. Command-argv is the oma 2026-09-01
/// misfit: Codex only accepts built-in item IDs and skips the rest, so
/// the bar goes empty (M045).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexStatusline {
    Builtin,
    CommandArgv,
    Missing,
}

fn looks_like_codex_command_argv(s: &str) -> bool {
    has_statusline_marker(s) || s == "pwsh" || s == "-NoProfile" || s == "-File" || s == "command"
}

fn is_codex_builtin_status_item(s: &str) -> bool {
    matches!(
        s,
        "run-state"
            | "status"
            | "model-name"
            | "model-with-reasoning"
            | "reasoning"
            | "current-dir"
            | "project-root"
            | "project"
            | "project-name"
            | "git-branch"
            | "pull-request-number"
            | "branch-changes"
            | "permissions"
            | "approval-mode"
            | "approval"
            | "context-remaining"
            | "context-used"
            | "context-window-size"
            | "used-tokens"
            | "total-input-tokens"
            | "total-output-tokens"
            | "five-hour-limit"
            | "weekly-limit"
            | "codex-version"
            | "thread-credits"
            | "estimated-thread-cost"
            | "session-id"
            | "thread-id"
            | "fast-mode"
            | "raw-output"
            | "thread-title"
            | "workspace-headline"
            | "task-progress"
    )
}

fn codex_statusline_state(home: &Path) -> CodexStatusline {
    let Some(parts) = toml_file(&home.join(".codex").join("config.toml"))
        .as_ref()
        .and_then(|t| t.get("tui"))
        .and_then(|tui| tui.get("status_line"))
        .and_then(|sl| sl.as_array())
        .cloned()
    else {
        return CodexStatusline::Missing;
    };
    if parts.is_empty() {
        return CodexStatusline::Missing;
    }
    if parts
        .iter()
        .any(|p| p.as_str().is_some_and(looks_like_codex_command_argv))
    {
        return CodexStatusline::CommandArgv;
    }
    if parts
        .iter()
        .any(|p| p.as_str().is_some_and(is_codex_builtin_status_item))
    {
        return CodexStatusline::Builtin;
    }
    CodexStatusline::Missing
}

fn kimi_statusline_on(home: &Path) -> bool {
    toml_file(&home.join(".kimi-code").join("tui.toml"))
        .as_ref()
        .and_then(|t| t.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| has_statusline_marker(c))
}

/// Grok `[ui.status_line].command` kinds. A `pwsh -File "..."` shell line
/// contains quotes, so Windows `Command::new(entire_string)` returns
/// ERROR_INVALID_NAME 123 and never falls back to a shell (M048).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokStatusline {
    CmdPath,
    PwshFile,
    Missing,
}

fn grok_statusline_state(home: &Path) -> GrokStatusline {
    let toml = toml_file(&home.join(".grok").join("config.toml"));
    let Some(cmd) = toml
        .as_ref()
        .and_then(|t| t.get("ui"))
        .and_then(|ui| ui.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|c| c.contains(STATUSLINE_MARKER))
    else {
        return GrokStatusline::Missing;
    };
    if cmd.ends_with("hst-statusline-grok.cmd") && !cmd.contains("pwsh") {
        return GrokStatusline::CmdPath;
    }
    if cmd.contains("pwsh") && cmd.contains("-File") {
        return GrokStatusline::PwshFile;
    }
    GrokStatusline::Missing
}

fn push_statusline(
    out: &mut Vec<Finding>,
    agent: &str,
    on: bool,
    cfg: &Path,
    script_ok: bool,
    pwsh_missing: bool,
) {
    let (status, mut detail) = if !on {
        (
            Status::Warn,
            format!("not configured; hst statusline {agent}"),
        )
    } else if !script_ok {
        (
            Status::Warn,
            "configured but hst-statusline.ps1 missing; rerun hst statusline".into(),
        )
    } else {
        (Status::Ok, "hst bar configured".into())
    };
    if pwsh_missing {
        detail.push_str("; pwsh not on PATH (bar will not render)");
    }
    push_status(out, agent, "statusline", status, cfg, detail);
}

// ===== hook 注册形态（P0027 口径） =====

/// 命令串程序位解析（剥调用操作符与包裹引号；D39 包裹形态经
/// `deploy::program_token` 取解释器后的程序位，codex review F2）：
/// `& "D:/x/hst-state.cmd" codex`、`D:/x/hst-state.cmd codex` 与
/// `powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:/x/hst-state.ps1 claude`
/// 都得指向 shim 脚本本体。
fn command_target(c: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(crate::deploy::program_token(c))
}

/// JSON 形 hook 注册（claude settings、grok ohmyagents-state.json）的 oma
/// 体检的形态面（细则见模块文档与集成测试）。
/// 体检的缺失面（细则见模块文档与集成测试）。
/// absolute（单环境）/ args（M047 病理）/ none。
fn json_hooks_form(v: Option<&Json>) -> &'static str {
    let Some(events) = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object()) else {
        return "none";
    };
    let mut ours = false;
    let mut shim = false;
    let mut shim_dead = false;
    let mut bare = false;
    let mut has_args = false;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
                continue;
            };
            if crate::deploy::is_ours(c) {
                ours = true;
                if h.get("args")
                    .and_then(|a| a.as_array())
                    .is_some_and(|a| !a.is_empty())
                {
                    has_args = true;
                }
                if c.contains("hst-state") || c.contains("oma-state") {
                    if command_target(c).is_file() {
                        shim = true;
                    } else {
                        shim_dead = true;
                    }
                }
                if !c.contains('/') && !c.contains('\\') {
                    bare = true;
                }
            }
        }
    }
    if has_args {
        "args"
    } else if shim_dead && !shim {
        "shim-dead"
    } else if shim {
        "shim"
    } else if bare {
        "bare"
    } else if ours {
        "absolute"
    } else {
        "none"
    }
}

/// codex `.codex/hooks.json` 里 ours 处理器占据的 per-OS 字段：
/// command（Unix 侧）/ commandWindows（Windows 侧）。绝对路径是设计态
/// （hook exec 环境不继承 PATH，P0027）。
fn codex_hooks_sides(v: Option<&Json>) -> (bool, bool) {
    let Some(events) = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object()) else {
        return (false, false);
    };
    let mut unix_side = false;
    let mut win_side = false;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            if h.get("command")
                .and_then(|c| c.as_str())
                .is_some_and(crate::deploy::is_ours)
            {
                unix_side = true;
            }
            if h.get("commandWindows")
                .and_then(|c| c.as_str())
                .is_some_and(crate::deploy::is_ours)
            {
                win_side = true;
            }
        }
    }
    (unix_side, win_side)
}

/// codex 各侧有效字段（Windows 看 commandWindows、Unix 看 command）的形态
/// 分类（review 验收补：此前不分 shim / bare / absolute 一律 ok，死链也
/// ok，D08「写了配置仍 ok」同族）。None = 该侧无 ours 条目；多条 ours 取
/// 最差形态（shim 好、bare/absolute 次、shim-dead 最差）。
fn codex_side_form(v: Option<&Json>, windows_side: bool) -> Option<&'static str> {
    let events = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object())?;
    let key = if windows_side {
        "commandWindows"
    } else {
        "command"
    };
    let rank = |f: &str| match f {
        "shim" => 0,
        "bare" | "absolute" => 1,
        _ => 2, // shim-dead
    };
    let mut form: Option<&'static str> = None;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            let Some(c) = h.get(key).and_then(|c| c.as_str()) else {
                continue;
            };
            if !crate::deploy::is_ours(c) {
                continue;
            }
            let f = if c.contains("hst-state") || c.contains("oma-state") {
                if command_target(c).is_file() {
                    "shim"
                } else {
                    "shim-dead"
                }
            } else if !c.contains('/') && !c.contains('\\') {
                "bare"
            } else {
                "absolute"
            };
            form = Some(match form {
                None => f,
                Some(prev) if rank(prev) >= rank(f) => prev,
                Some(_) => f,
            });
        }
    }
    form
}

/// D29 迁移告警：ours 注册仍带 oma 纪元形态（oma-state / .oma 路径）→
/// warn 指引 `hst init` 完成数据根与注册改写（heal）。
fn push_hooks_migrate(out: &mut Vec<Finding>, agent: &str, v: Option<&Json>, path: &Path) {
    let legacy = v
        .and_then(|v| v.get("hooks"))
        .and_then(|h| h.as_object())
        .is_some_and(|events| {
            events
                .values()
                .filter_map(|g| g.as_array())
                .flatten()
                .any(|grp| {
                    grp.get("hooks")
                        .and_then(|h| h.as_array())
                        .is_some_and(|hs| {
                            hs.iter().any(|h| {
                                ["command", "commandWindows"].iter().any(|k| {
                                    h.get(*k).and_then(|c| c.as_str()).is_some_and(|c| {
                                        crate::deploy::is_ours(c)
                                            && (c.contains("oma-state") || c.contains(".oma"))
                                    })
                                })
                            })
                        })
                })
        });
    if legacy {
        push_status(
            out,
            agent,
            "hooks.migrate",
            Status::Warn,
            path,
            "pre-0.6.0 registration points at ~/.oma; run `hst init` to migrate root and rewrite (D29)",
        );
    }
}

fn push_hooks_form(out: &mut Vec<Finding>, agent: &str, form: &str, path: &Path) {
    match form {
        "shim" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Ok,
            path,
            "form=shim (self-contained state writer in ~/.hst/hooks; user-level \
             registration, zero hst dependency, D27/D28)",
        ),
        "shim-dead" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=shim-dead (registration points at a missing ~/.hst/hooks script; \
             rerun hst init)",
        ),
        "bare" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=bare (pre-D27 registration pins the oma binary; hst init upgrades to shim)",
        ),
        "args" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "command+args is PowerShell ParserError under Grok (M047); hst init",
        ),
        "absolute" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=absolute (pre-D27 registration pins the oma binary; hst init upgrades to shim)",
        ),
        _ => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "no hst hooks; hst init deploys",
        ),
    }
}

/// kimi `[[hooks]]`（用户级 config.toml）的 oma 形态（D28：kimi 的注册面
/// 只有用户级；F8 补 shim 在位探针，对齐 claude / grok 面辨形）。
fn kimi_hooks_form(toml: Option<&Toml>) -> &'static str {
    let Some(arr) = toml.and_then(|t| t.get("hooks")).and_then(|h| h.as_array()) else {
        return "none";
    };
    let mut ours = false;
    let mut shim = false;
    let mut shim_dead = false;
    for h in arr {
        let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
            continue;
        };
        if !crate::deploy::is_ours(c) {
            continue;
        }
        ours = true;
        if c.contains("hst-state") || c.contains("oma-state") {
            if command_target(c).is_file() {
                shim = true;
            } else {
                shim_dead = true;
            }
        }
    }
    if shim_dead && !shim {
        "shim-dead"
    } else if shim {
        "shim"
    } else if ours {
        "bare"
    } else {
        "none"
    }
}

/// 项目级 ours 注册残留提示（D28 退役迁移判据：v0.5.3 前的项目部署遗留）。
fn push_project_residue(out: &mut Vec<Finding>, agent: &str, ours: bool, path: &Path) {
    if ours {
        push_status(
            out,
            agent,
            "hooks.retired",
            Status::Warn,
            path,
            "project-level oma hooks remain; rerun hst init to retire (D28)",
        );
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Read-only. Does not attach, send-keys, or wait on TUI.
pub fn diagnose(root: &Path) -> Result<Diagnosis, String> {
    let root = abs_display(root);
    let home = crate::pathutil::user_home()?;
    let mut findings = Vec::new();

    // 部署诊断共享事实：状态栏脚本与 pwsh 探测一次（S025），登录态用统一
    // 时间基准（S026）。
    let oma_root = crate::install::hst_home().ok();
    let sl_script_ok = oma_root
        .as_deref()
        .map(crate::statusline::script_path)
        .is_some_and(|p| p.is_file());
    let sl_grok_ok = sl_script_ok
        && oma_root
            .as_deref()
            .map(crate::statusline::grok_cmd_path)
            .is_some_and(|p| p.is_file());
    let sl_pwsh_missing = !crate::statusline::pwsh_on_path();
    let now = OffsetDateTime::now_utc();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // CPU 能力段（S021）：Bun 系要 AVX/AVX2、Rust 原生常要 AVX-512，缺了
    // 表现为 agent 启动即崩——先摆出事实面，探针异常退出另有分类。
    let caps = crate::caps::detect();
    findings.push(Finding {
        agent: "cpu".into(),
        check: "caps".into(),
        status: Status::Ok,
        path: caps.arch.into(),
        detail: crate::caps::caps_line(&caps),
    });

    // D28 第 3 轮：yolo 两级显式（--yolo 用户级、--project-yolo 项目级），
    // doctor 双级接受；项目键在场时按 agent 分层规则遮蔽用户键（报告项目
    // 面为准）。D50：项目层补 settings.local.json 读取（local 优先于
    // shared，对齐 settings 优先级栈，S029 原案 Allow 堆积即在 local 层）；
    // 项目层 bypass-only 不再判 ok（claude 2.1.257 起项目与 local 层
    // bypassPermissions 被忽略，运行时回退用户层，S029 追记）。
    let claude_shared = root.join(".claude").join("settings.json");
    let claude_local_settings = root.join(".claude").join("settings.local.json");
    let claude_user_yolo = home.join(".claude").join("settings.json");
    let claude_mode_at = |p: &Path| -> Option<String> {
        json_file(p)
            .as_ref()
            .and_then(|v| v.get("permissions"))
            .and_then(|pp| pp.get("defaultMode"))
            .and_then(|m| m.as_str())
            .map(str::to_string)
    };
    // D52：解析失败显式报（BOM 已容忍后的残余防线：真坏文件点名）。
    for p in [
        &claude_local_settings,
        &claude_shared,
        &home.join(".claude").join("settings.json"),
    ] {
        if matches!(json_file_state(p), JsonFileState::Bad) {
            push_status(
                &mut findings,
                "claude",
                "yolo.parse",
                Status::Warn,
                p,
                "settings file exists but is not valid JSON (check encoding/\
                 content); hst cannot read its keys",
            );
        }
    }
    let claude_proj_mode = claude_mode_at(&claude_local_settings)
        .map(|m| (m, claude_local_settings.clone()))
        .or_else(|| claude_mode_at(&claude_shared).map(|m| (m, claude_shared.clone())));
    let claude_user_mode = claude_mode_at(&claude_user_yolo);
    // D33：yolo 判据分级接受（full=bypassPermissions、partial=acceptEdits）。
    let claude_yolo_ok = |m: &str| matches!(m, "bypassPermissions" | "acceptEdits");
    match (&claude_proj_mode, &claude_user_mode) {
        // 项目层 bypass（2.1.257 起被忽略，不生效）：实际生效层是用户层，
        // 按用户层判；不构成遮蔽（codex F3：p=bypass 加 u=acceptEdits 实际
        // 生效 acceptEdits，D33 下 ok，走 conflict 臂属误报）。
        (Some((p, _)), Some(u)) if p == "bypassPermissions" => push(
            &mut findings,
            "claude",
            "yolo",
            claude_yolo_ok(u),
            &claude_user_yolo,
            format!(
                "defaultMode={u} (user level; project bypassPermissions copy \
                 ignored since claude 2.1.257)"
            ),
        ),
        // 双级冲突（D28 第 4 令；D50 补 local 层）：非忽略的项目 mode
        //（acceptEdits / default / auto 等）遮蔽用户层即静默降级（S029 追
        // 记：项目层优先级高于用户层，bypass 名义下全线提示回潮），warn
        // 加对齐 CTA。
        (Some((p, ppath)), Some(u)) if p != u => push_status(
            &mut findings,
            "claude",
            "yolo",
            Status::Warn,
            ppath,
            format!(
                "conflict: project defaultMode={p} shadows user {u} (silent \
                 downgrade, S029); align via `hst init --clear-project-yolo` \
                 (one-click strip of project-level interference, REQ-009) or `hst init \
                 --project-yolo=<level>` or drop one level (D28 r4)"
            ),
        ),
        (Some((p, ppath)), Some(_)) => push(
            &mut findings,
            "claude",
            "yolo",
            claude_yolo_ok(p),
            ppath,
            format!("defaultMode={p} (project level)"),
        ),
        // 项目层 bypass-only：2.1.257 起被忽略，运行时无 bypass（D50 修
        // 假阳性：此前判 ok）。
        (Some((p, ppath)), None) if p == "bypassPermissions" => push_status(
            &mut findings,
            "claude",
            "yolo",
            Status::Warn,
            ppath,
            "project-only defaultMode=bypassPermissions is ignored since claude \
             2.1.257 (session starts manual); set user level via `hst init --yolo`",
        ),
        (Some((p, ppath)), None) => push(
            &mut findings,
            "claude",
            "yolo",
            claude_yolo_ok(p),
            ppath,
            format!("defaultMode={p} (project level)"),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "claude",
            "yolo",
            claude_yolo_ok(u),
            &claude_user_yolo,
            format!("defaultMode={u} (user level)"),
        ),
        // codex F4：用户级文件在但解析失败时，yolo 行不再假报 missing 加
        // block（误导且误计 exit 1）；点名 unparseable 并指向 yolo.parse 行。
        (None, None)
            if matches!(json_file_state(&claude_user_yolo), JsonFileState::Bad) =>
        {
            push_status(
                &mut findings,
                "claude",
                "yolo",
                Status::Warn,
                &claude_user_yolo,
                "user settings unparseable; cannot read defaultMode (see yolo.parse)",
            )
        }
        (None, None) => push(
            &mut findings,
            "claude",
            "yolo",
            false,
            &claude_user_yolo,
            "missing permissions.defaultMode=bypassPermissions|acceptEdits (tool prompt will block)",
        ),
    }

    // D50：项目层显式 ask 规则（S029 追记第 2 类）——bypass 只跳 allow 层，
    // ask 照弹；项目层按优先级栈盖用户层。warn 级不 block（部署缺口）。
    let ask_count = |p: &Path| -> usize {
        json_file(p)
            .as_ref()
            .and_then(|v| v.get("permissions"))
            .and_then(|pp| pp.get("ask"))
            .and_then(|a| a.as_array())
            .map_or(0, |a: &Vec<Json>| a.len())
    };
    for (path, layer) in [
        (&claude_local_settings, "project-local"),
        (&claude_shared, "project"),
        // codex F4：用户级 ask 同样 bypass 下照弹，口径与 readblock 对齐
        //（三层全查）。
        (&claude_user_yolo, "user"),
    ] {
        let n = ask_count(path);
        if n > 0 {
            push_status(
                &mut findings,
                "claude",
                "yolo.ask",
                Status::Warn,
                path,
                format!(
                    "permissions.ask has {n} rule(s) in {layer} layer; ask rules \
                     still prompt even in bypassPermissions (S029); remove or narrow"
                ),
            );
        }
    }
    // D50：读沙箱读块（S029 追记标本类）——blockReadsOutsideWorkingDirectories
    // 在场时，shell 解析器无法静态分析的命令（内联 python -c 等）即使
    // bypass 也问人。任一层为 true 即 warn。
    let read_block = |p: &Path| -> bool {
        json_file(p)
            .as_ref()
            .and_then(|v| v.get("permissions"))
            .and_then(|pp| pp.get("blockReadsOutsideWorkingDirectories"))
            .and_then(|b| b.as_bool())
            .unwrap_or(false)
    };
    for path in [&claude_local_settings, &claude_shared, &claude_user_yolo] {
        if read_block(path) {
            push_status(
                &mut findings,
                "claude",
                "yolo.readblock",
                Status::Warn,
                path,
                "permissions.blockReadsOutsideWorkingDirectories=true: shell \
                 commands the parser cannot analyze still prompt even in \
                 bypassPermissions (S029)",
            );
        }
    }

    let claude_local = root.join(".claude").join("settings.local.json");
    let user_claude = home.join(".claude").join("settings.json");
    let skip = json_file(&claude_local)
        .as_ref()
        .map(|v| json_bool(v, "skipDangerousModePermissionPrompt"))
        .unwrap_or(false)
        || json_file(&user_claude)
            .as_ref()
            .map(|v| json_bool(v, "skipDangerousModePermissionPrompt"))
            .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "skip_prompt",
        skip,
        if claude_local.exists() {
            &claude_local
        } else {
            &user_claude
        },
        if skip {
            "skipDangerousModePermissionPrompt=true"
        } else {
            "missing skipDangerousModePermissionPrompt (bypass confirm dialog)"
        },
    );

    let claude_json = home.join(".claude.json");
    let cj = json_file(&claude_json);
    let claude_entry = cj.as_ref().and_then(|v| claude_project_entry(v, &root));
    let folder_ok = claude_entry
        .map(|e| json_bool(e, "hasTrustDialogAccepted"))
        .unwrap_or(false);
    let onboard_ok = cj
        .as_ref()
        .map(|v| json_bool(v, "hasCompletedOnboarding"))
        .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "trust.project",
        folder_ok,
        &claude_json,
        if folder_ok {
            "hasTrustDialogAccepted (folder; parent path also counts)"
        } else {
            "folder TrustDialog would block (hasTrustDialogAccepted)"
        },
    );
    let claude_has_hooks = json_file(&claude_shared)
        .as_ref()
        .map(has_hooks_settings)
        .unwrap_or(false)
        || json_file(&claude_local)
            .as_ref()
            .map(has_hooks_settings)
            .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "trust.hooks",
        !claude_has_hooks || folder_ok,
        &claude_json,
        if !claude_has_hooks {
            "n/a no project hooks in .claude/settings*.json"
        } else if folder_ok {
            "covered_by trust.project (interactive holds settings hooks until folder trusted)"
        } else {
            "project hooks present; interactive would hold them until hasTrustDialogAccepted"
        },
    );
    let claude_mcp = project_mcp_configured(&root, json_file(&claude_shared).as_ref())
        || project_mcp_configured(&root, json_file(&claude_local).as_ref());
    let user_mcp_ok = json_file(&user_claude)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let local_mcp_ok = json_file(&claude_local)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let project_mcp_ok = json_file(&claude_shared)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let mcp_ok = user_mcp_ok || (folder_ok && (local_mcp_ok || project_mcp_ok));
    push(
        &mut findings,
        "claude",
        "trust.mcp",
        if claude_mcp { mcp_ok } else { true },
        if user_mcp_ok {
            &user_claude
        } else if claude_local.exists() {
            &claude_local
        } else {
            &claude_shared
        },
        if !claude_mcp {
            "n/a no project MCP servers"
        } else if user_mcp_ok {
            "enableAll/enabledMcpjsonServers in user settings (honored while untrusted)"
        } else if folder_ok && (local_mcp_ok || project_mcp_ok) {
            "enableAll/enabledMcpjsonServers after workspace trust"
        } else if local_mcp_ok || project_mcp_ok {
            "project/local MCP approval ignored until workspace trusted (v2.1.196+)"
        } else {
            "MCPServerApprovalDialog pending"
        },
    );
    let claude_skills_dir = root.join(".claude").join("skills");
    let claude_commands_dir = root.join(".claude").join("commands");
    let claude_skills = dir_nonempty(&claude_skills_dir) || dir_nonempty(&claude_commands_dir);
    let claude_skill_plugin = skill_dir_is_plugin(&claude_skills_dir);
    push(
        &mut findings,
        "claude",
        "trust.skill",
        !claude_skills || folder_ok,
        if dir_nonempty(&claude_skills_dir) {
            &claude_skills_dir
        } else {
            &claude_commands_dir
        },
        if !claude_skills {
            "n/a no .claude/skills or .claude/commands"
        } else if folder_ok && claude_skill_plugin {
            "covered_by trust.project (skills-dir plugin + folder trust)"
        } else if folder_ok {
            "covered_by trust.project (skills/commands load after folder trust)"
        } else if claude_skill_plugin {
            "skills-dir plugin and project skills blocked until hasTrustDialogAccepted"
        } else {
            "project skills/commands blocked until hasTrustDialogAccepted"
        },
    );
    push(
        &mut findings,
        "claude",
        "onboarding",
        onboard_ok,
        &claude_json,
        if onboard_ok {
            "hasCompletedOnboarding"
        } else {
            "onboarding dialog would block"
        },
    );
    push_binary(&mut findings, "claude");
    // D28：hook 注册用户级；形态看 ~/.claude/settings.json（项目级文件只查
    // 残留）。
    let claude_user_settings = home.join(".claude").join("settings.json");
    push_hooks_form(
        &mut findings,
        "claude",
        json_hooks_form(json_file(&claude_user_settings).as_ref()),
        &claude_user_settings,
    );
    push_hooks_migrate(
        &mut findings,
        "claude",
        json_file(&claude_user_settings).as_ref(),
        &claude_user_settings,
    );
    let claude_proj_form = {
        let form = json_hooks_form(json_file(&claude_shared).as_ref());
        if form != "none" {
            form
        } else {
            json_hooks_form(json_file(&claude_local).as_ref())
        }
    };
    push_project_residue(
        &mut findings,
        "claude",
        claude_proj_form != "none",
        &claude_shared,
    );
    push_statusline(
        &mut findings,
        "claude",
        claude_statusline_on(&home),
        &home.join(".claude").join("settings.json"),
        sl_script_ok,
        sl_pwsh_missing,
    );

    let codex_proj = root.join(".codex").join("config.toml");
    let codex_user = home.join(".codex").join("config.toml");
    let proj_toml = toml_file(&codex_proj);
    let user_toml = toml_file(&codex_user);
    let codex_pair_at = |t: Option<&Toml>| -> Option<(String, String)> {
        let t = t?;
        Some((
            toml_str(t, "sandbox_mode")?.to_string(),
            toml_str(t, "approval_policy")?.to_string(),
        ))
    };
    let codex_proj_pair = codex_pair_at(proj_toml.as_ref());
    let codex_user_pair = codex_pair_at(user_toml.as_ref());
    let codex_yolo_pair = |p: &(String, String)| {
        // D33：full = danger-full-access/never，partial = workspace-write/on-request。
        p == &("danger-full-access".to_string(), "never".to_string())
            || p == &("workspace-write".to_string(), "on-request".to_string())
    };
    match (&codex_proj_pair, &codex_user_pair) {
        // 双级冲突（D28 第 4 令）：项目遮蔽用户，warn 加对齐 CTA。
        (Some(p), Some(u)) if p != u => push_status(
            &mut findings,
            "codex",
            "yolo",
            Status::Warn,
            &codex_proj,
            format!(
                "conflict: project sandbox/approval={p:?} shadows user {u:?}; align via \
                 `hst init --clear-project-yolo` (one-click strip of project-level \
                 interference, REQ-009) or `hst init --project-yolo=<level>` or drop one \
                 level (D28 r4)"
            ),
        ),
        (Some(p), _) => push(
            &mut findings,
            "codex",
            "yolo",
            codex_yolo_pair(p),
            &codex_proj,
            format!("sandbox/approval (project level): {} {}", p.0, p.1),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "codex",
            "yolo",
            codex_yolo_pair(u),
            &codex_user,
            format!("sandbox/approval (user level): {} {}", u.0, u.1),
        ),
        (None, None) => push(
            &mut findings,
            "codex",
            "yolo",
            false,
            &codex_user,
            "missing sandbox/approval yolo keys",
        ),
    }
    // Codex only loads the project config layer after the user store trusts the
    // path. Project `.codex/config.toml` [projects] does not clear the dialog.
    let trust_codex = user_toml
        .as_ref()
        .map(|t| projects_trusted(t, &root))
        .unwrap_or(false);
    push(
        &mut findings,
        "codex",
        "trust.project",
        trust_codex,
        &codex_user,
        if trust_codex {
            "user projects trust_level=trusted"
        } else {
            "untrusted project skips .codex layer and shows trust dialog"
        },
    );
    // D28：hook 注册用户级 ~/.codex/hooks.json；信任预种在 ~/.codex/config.toml
    // 的 [hooks.state]。判据逐键对账（codex review F2 硬化）：期望键源 =
    // hooks.json 路径、期望哈希按宿主侧现算（与 deploy 同源）；「有任一
    // trusted_hash 即 ok」会假绿。项目 hooks.json 只查残留。
    let codex_user_hooks_path = home.join(".codex").join("hooks.json");
    let codex_user_hooks = json_file(&codex_user_hooks_path);
    let codex_user_ours = codex_hooks_sides(codex_user_hooks.as_ref()) != (false, false);
    let expected_trust = crate::deploy::codex_trust_entries(
        codex_user_hooks
            .as_ref()
            .unwrap_or(&serde_json::Value::Null),
        &codex_user_hooks_path,
    )
    .unwrap_or_default();
    let trust_gap: Vec<String> = expected_trust
        .iter()
        .filter(|(k, want)| {
            user_toml
                .as_ref()
                .and_then(|t| t.get("hooks"))
                .and_then(|h| h.get("state"))
                .and_then(|s| s.get(k))
                .and_then(|e| e.get("trusted_hash"))
                .and_then(|h| h.as_str())
                != Some(want.as_str())
        })
        .map(|(k, _)| k.rsplit(':').nth(2).unwrap_or(k).to_string())
        .collect();
    push(
        &mut findings,
        "codex",
        "trust.hooks",
        if codex_user_ours {
            trust_gap.is_empty()
        } else {
            true
        },
        &codex_user_hooks_path,
        if !codex_user_ours {
            "n/a no oma hooks in user hooks.json (yolo does not bypass hook trust)".to_string()
        } else if trust_gap.is_empty() {
            "user [hooks.state] keys+hashes match (hooks.json source)".to_string()
        } else {
            format!(
                "hook trust missing/mismatched for {}; rerun hst init or --dangerously-bypass-hook-trust",
                trust_gap.join(",")
            )
        },
    );
    let codex_skills = dir_nonempty(&root.join(".agents").join("skills"))
        || dir_nonempty(&root.join(".codex").join("skills"));
    push(
        &mut findings,
        "codex",
        "trust.skill",
        if codex_skills { trust_codex } else { true },
        &root.join(".agents").join("skills"),
        if !codex_skills {
            "n/a no .agents/skills or .codex/skills"
        } else if trust_codex {
            "covered_by trust.project"
        } else {
            "project skills skipped until trust.project"
        },
    );
    let codex_mcp_json = root.join(".mcp.json");
    let codex_mcp =
        path_is_file(&codex_mcp_json) || proj_toml.as_ref().is_some_and(toml_mcp_configured);
    push(
        &mut findings,
        "codex",
        "trust.mcp",
        !codex_mcp || trust_codex,
        if path_is_file(&codex_mcp_json) {
            &codex_mcp_json
        } else {
            &codex_proj
        },
        if !codex_mcp {
            "n/a no project MCP servers"
        } else if trust_codex {
            "covered_by trust.project (Codex skips project MCP until trusted)"
        } else {
            "project MCP skipped until trust.project"
        },
    );
    push_binary(&mut findings, "codex");
    let codex_json = codex_user_hooks.clone();
    let (codex_unix, codex_win) = codex_hooks_sides(codex_json.as_ref());
    if codex_unix || codex_win {
        // 分侧辨形（review 验收补）：warn 判据只看宿主侧字段——非宿主侧在
        // 单环境部署下恒为 M055 bare 兜底（init 修不掉，warn 即空转）；两
        // 侧 shim-dead/absolute 都是真缺陷仍降 warn。
        let host_windows = cfg!(windows);
        let unix_form = codex_side_form(codex_json.as_ref(), false);
        let win_form = codex_side_form(codex_json.as_ref(), true);
        // 宿主侧必须 shim；非宿主侧 bare 是 M055 兜底设计态（不 warn），
        // absolute / shim-dead 任何一侧都是真缺陷（warn）。
        let side_ok = |f: &str, is_host: bool| {
            if is_host {
                f == "shim"
            } else {
                f != "shim-dead" && f != "absolute"
            }
        };
        let mut sides = Vec::new();
        let mut ok = true;
        if codex_unix {
            let f = unix_form.unwrap_or("absolute");
            ok &= side_ok(f, !host_windows);
            sides.push(format!("command(unix)={f}"));
        }
        if codex_win {
            let f = win_form.unwrap_or("absolute");
            ok &= side_ok(f, host_windows);
            sides.push(format!("commandWindows(windows)={f}"));
        }
        push_status(
            &mut findings,
            "codex",
            "hooks.form",
            if ok { Status::Ok } else { Status::Warn },
            &codex_user_hooks_path,
            format!(
                "per-OS fields ours: {} (shim by design, D27/D28; host side must be shim, \
                 dead/absolute anywhere = rerun hst init)",
                sides.join(", ")
            ),
        );
    } else {
        push_hooks_form(&mut findings, "codex", "none", &codex_user_hooks_path);
    }
    push_hooks_migrate(
        &mut findings,
        "codex",
        codex_user_hooks.as_ref(),
        &codex_user_hooks_path,
    );
    push_project_residue(
        &mut findings,
        "codex",
        codex_hooks_sides(json_file(&root.join(".codex").join("hooks.json")).as_ref())
            != (false, false),
        &root.join(".codex").join("hooks.json"),
    );
    {
        let cfg = home.join(".codex").join("config.toml");
        match codex_statusline_state(&home) {
            CodexStatusline::Builtin => push_status(
                &mut findings,
                "codex",
                "statusline",
                Status::Ok,
                &cfg,
                "oma bar configured (built-in item IDs)",
            ),
            CodexStatusline::CommandArgv => push_status(
                &mut findings,
                "codex",
                "statusline",
                Status::Warn,
                &cfg,
                "status_line is command-argv; Codex only accepts built-in item IDs (S016); hst statusline",
            ),
            CodexStatusline::Missing => push_statusline(
                &mut findings,
                "codex",
                false,
                &cfg,
                sl_script_ok,
                sl_pwsh_missing,
            ),
        }
    }

    let kimi_user = home.join(".kimi-code").join("config.toml");
    let kimi_proj = root.join(".kimi-code").join("config.toml");
    let kimi_mode_at = |p: &Path| -> Option<String> {
        toml_file(p).and_then(|t| {
            t.get("default_permission_mode")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
    };
    let kimi_proj_mode = kimi_mode_at(&kimi_proj);
    let kimi_user_mode = kimi_mode_at(&kimi_user);
    let kimi_ok = |m: &str| matches!(m, "yolo" | "auto");
    match (&kimi_proj_mode, &kimi_user_mode) {
        (Some(p), Some(u)) if p != u => push_status(
            &mut findings,
            "kimi",
            "yolo",
            Status::Warn,
            &kimi_proj,
            format!(
                "conflict: project default_permission_mode={p} shadows user {u}; align via \
                 `hst init --clear-project-yolo` (one-click strip of project-level \
                 interference, REQ-009) or `hst init --project-yolo=<level>` or drop one \
                 level (D28 r4)"
            ),
        ),
        (Some(p), _) => push(
            &mut findings,
            "kimi",
            "yolo",
            kimi_ok(p),
            &kimi_proj,
            format!("default_permission_mode={p} (project level)"),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "kimi",
            "yolo",
            kimi_ok(u),
            &kimi_user,
            format!("default_permission_mode={u} (user level)"),
        ),
        (None, None) => push(
            &mut findings,
            "kimi",
            "yolo",
            false,
            &kimi_user,
            "missing default_permission_mode auto|yolo",
        ),
    }
    let trust_kimi = kimi_trust_ok(&home, &root);
    let kimi_trust_path = home
        .join(".kimi-code")
        .join("workspace-trust")
        .join(kimi_workspace_key(&root));
    push(
        &mut findings,
        "kimi",
        "trust.project",
        trust_kimi,
        &kimi_trust_path,
        if trust_kimi {
            "workspace-trust file for this root"
        } else {
            "folder trust dialog defaults to Don't trust"
        },
    );
    push(
        &mut findings,
        "kimi",
        "trust.hooks",
        true,
        &kimi_user,
        "n/a no hook-trust dialog (hooks live in user config.toml)",
    );
    let kimi_skills_dir = root.join(".kimi-code").join("skills");
    let agents_skills_dir = root.join(".agents").join("skills");
    let kimi_skills = dir_nonempty(&kimi_skills_dir) || dir_nonempty(&agents_skills_dir);
    push(
        &mut findings,
        "kimi",
        "trust.skill",
        !kimi_skills || trust_kimi,
        if dir_nonempty(&kimi_skills_dir) {
            &kimi_skills_dir
        } else {
            &agents_skills_dir
        },
        if !kimi_skills {
            "n/a no project skills"
        } else if trust_kimi {
            "covered_by trust.project"
        } else {
            "project skills present; folder trust dialog defaults to Don't trust"
        },
    );
    push_binary(&mut findings, "kimi");
    let kimi_cred = home
        .join(".kimi-code")
        .join("credentials")
        .join("kimi-code.json");
    let (kimi_login_st, kimi_login_detail) =
        kimi_login_state(json_file(&kimi_cred).as_ref(), now_secs);
    push_status(
        &mut findings,
        "kimi",
        "login",
        kimi_login_st,
        &kimi_cred,
        kimi_login_detail,
    );
    // D28：kimi 注册面只有用户级 ~/.kimi-code/config.toml [[hooks]]（F8 起
    // 与其它家同款辨形，shim-dead 死链可见）。
    push_hooks_form(
        &mut findings,
        "kimi",
        kimi_hooks_form(toml_file(&kimi_user).as_ref()),
        &kimi_user,
    );
    push_statusline(
        &mut findings,
        "kimi",
        kimi_statusline_on(&home),
        &home.join(".kimi-code").join("tui.toml"),
        sl_script_ok,
        sl_pwsh_missing,
    );

    let grok_cfg = home.join(".grok").join("config.toml");
    let grok_tf = home.join(".grok").join("trusted_folders.toml");
    let grok_mode = toml_file(&grok_cfg).as_ref().and_then(|t| {
        t.get("ui")
            .and_then(|ui| toml_str(ui, "permission_mode"))
            .or_else(|| toml_str(t, "permission_mode"))
            .map(|s| s.to_string())
    });
    // D33：grok 分级接受（full=always-approve、partial=auto；canonical 值集
    // always-approve|auto|ask，grok-build permissions.rs）。
    let yolo_grok = matches!(grok_mode.as_deref(), Some("always-approve") | Some("auto"));
    push(
        &mut findings,
        "grok",
        "yolo",
        yolo_grok,
        &grok_cfg,
        match grok_mode.as_deref() {
            Some(m) => format!("permission_mode={m} (user config only)"),
            None => {
                "missing [ui] permission_mode=always-approve|auto in ~/.grok/config.toml".into()
            }
        },
    );
    let trust_grok = toml_file(&grok_tf)
        .as_ref()
        .map(|t| grok_folder_trusted(t, &root))
        .unwrap_or(false);
    let grok_markers = path_is_file(&root.join(".mcp.json"))
        || path_is_file(&root.join(".envrc"))
        || path_is_file(&root.join(".cursor").join("mcp.json"))
        || path_is_file(&root.join(".cursor").join("hooks.json"))
        || path_is_file(&root.join(".grok").join("lsp.json"))
        || path_is_file(&claude_shared)
        || path_is_file(&claude_local)
        || dir_nonempty(&root.join(".grok").join("hooks"))
        || dir_nonempty(&root.join(".grok").join("plugins"))
        || dir_nonempty(&root.join(".grok").join("agents"))
        || dir_nonempty(&root.join(".claude").join("agents"))
        || dir_nonempty(&root.join(".grok").join("roles"))
        || dir_nonempty(&root.join(".grok").join("personas"))
        || dir_nonempty(&root.join(".grok").join("workflows"));
    push(
        &mut findings,
        "grok",
        "trust.project",
        trust_grok || !grok_markers,
        &grok_tf,
        if trust_grok {
            "trusted_folders.toml trusted=true (MCP/LSP/hooks/plugins share this store)"
        } else if !grok_markers {
            "n/a no repo-local code-exec configs; Grok skips the prompt"
        } else {
            "folder trust missing; --trust writes the same store"
        },
    );
    let grok_hooks = dir_nonempty(&root.join(".grok").join("hooks"))
        || path_is_file(&root.join(".cursor").join("hooks.json"));
    push(
        &mut findings,
        "grok",
        "trust.hooks",
        if grok_hooks { trust_grok } else { true },
        &root.join(".grok").join("hooks"),
        if !grok_hooks {
            "n/a no project .grok/hooks (empty repo skips the prompt)"
        } else if trust_grok {
            "covered_by trust.project"
        } else {
            "project hooks silently skipped until folder trusted"
        },
    );
    let grok_mcp = path_is_file(&root.join(".mcp.json"))
        || path_is_file(&root.join(".cursor").join("mcp.json"));
    push(
        &mut findings,
        "grok",
        "trust.mcp",
        if grok_mcp { trust_grok } else { true },
        &root.join(".mcp.json"),
        if !grok_mcp {
            "n/a no .mcp.json (other mcp markers still use folder store)"
        } else if trust_grok {
            "covered_by trust.project"
        } else {
            "repo-local MCP gated by folder trust"
        },
    );
    let grok_skills = dir_nonempty(&root.join(".grok").join("skills"));
    push(
        &mut findings,
        "grok",
        "trust.skill",
        true,
        &root.join(".grok").join("skills"),
        if grok_skills {
            "n/a skills are not a folder-trust trigger (source kinds omit skills)"
        } else {
            "n/a no .grok/skills"
        },
    );
    push_binary(&mut findings, "grok");
    let grok_auth = home.join(".grok").join("auth.json");
    let (grok_login_st, grok_login_detail) = grok_login_state(json_file(&grok_auth).as_ref(), now);
    push_status(
        &mut findings,
        "grok",
        "login",
        grok_login_st,
        &grok_auth,
        grok_login_detail,
    );
    // D28：grok 注册面在全局层 ~/.grok/hooks/*.json；项目文件只查残留。
    let grok_state_json = home
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    push_hooks_form(
        &mut findings,
        "grok",
        json_hooks_form(json_file(&grok_state_json).as_ref()),
        &grok_state_json,
    );
    push_hooks_migrate(
        &mut findings,
        "grok",
        json_file(&grok_state_json).as_ref(),
        &grok_state_json,
    );
    push_project_residue(
        &mut findings,
        "grok",
        json_hooks_form(
            json_file(
                &root
                    .join(".grok")
                    .join("hooks")
                    .join("ohmyagents-state.json"),
            )
            .as_ref(),
        ) != "none",
        &root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json"),
    );
    match grok_statusline_state(&home) {
        GrokStatusline::CmdPath => {
            #[cfg(windows)]
            push_statusline(
                &mut findings,
                "grok",
                true,
                &grok_cfg,
                sl_grok_ok,
                sl_pwsh_missing,
            );
            #[cfg(not(windows))]
            push_status(
                &mut findings,
                "grok",
                "statusline",
                Status::Warn,
                &grok_cfg,
                "status_line is a Windows .cmd path; hst statusline grok",
            );
        }
        GrokStatusline::PwshFile => {
            #[cfg(windows)]
            push_status(
                &mut findings,
                "grok",
                "statusline",
                Status::Warn,
                &grok_cfg,
                "status_line is pwsh -File shell line; Grok Command::new paints os error 123 (M048); hst statusline grok",
            );
            #[cfg(not(windows))]
            push_statusline(
                &mut findings,
                "grok",
                true,
                &grok_cfg,
                sl_script_ok,
                sl_pwsh_missing,
            );
        }
        GrokStatusline::Missing => push_statusline(
            &mut findings,
            "grok",
            false,
            &grok_cfg,
            sl_grok_ok,
            sl_pwsh_missing,
        ),
    }

    let state_dir = crate::pathutil::project_dir(&root).join("state");
    if state_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&state_dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let blocked = json_file(&p)
                    .as_ref()
                    .and_then(|v| v.get("state"))
                    .and_then(|s| s.as_str())
                    == Some("blocked");
                let agent = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                findings.push(Finding {
                    agent: agent.to_string(),
                    check: "state",
                    status: if blocked { Status::Block } else { Status::Ok },
                    path: p.display().to_string(),
                    detail: if blocked {
                        "state=blocked".into()
                    } else {
                        "state not blocked".into()
                    },
                });
            }
        }
    }

    // （行序：项目面状态行在前——status() 取首个，项目归因的 Block 优先
    // 于用户级行。）
    // D28 用户级状态面：`~/.hst/state/*.json`。用户级状态无法归因到本项
    // 目（可能来自任何项目的会话），blocked 一律 warn 不 block——doctor
    // 的 Block 语义仍只对本项目交互阻塞负责（项目级旧状态文件照旧扫，
    // blocked = Block）。
    if let Some(user_state) = crate::install::hst_home().ok().map(|h| h.join("state")) {
        if user_state.is_dir() {
            if let Ok(rd) = fs::read_dir(&user_state) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if !name.ends_with(".json") {
                        continue;
                    }
                    let Some(agent) = ["claude", "codex", "grok", "kimi"].iter().find(|a| {
                        name == format!("{a}.json") || name.starts_with(&format!("{a}-"))
                    }) else {
                        continue;
                    };
                    let keyed = name.contains('-');
                    let blocked = json_file(&p)
                        .as_ref()
                        .and_then(|v| v.get("state"))
                        .and_then(|s| s.as_str())
                        == Some("blocked");
                    findings.push(Finding {
                        agent: agent.to_string(),
                        check: "state",
                        status: if blocked { Status::Warn } else { Status::Ok },
                        path: p.display().to_string(),
                        detail: if blocked {
                            if keyed {
                                "state=blocked (session-keyed, project unknown)".into()
                            } else {
                                "state=blocked (user-level latest, project unknown)".into()
                            }
                        } else {
                            "state not blocked".into()
                        },
                    });
                }
            }
        }
    }

    Ok(Diagnosis { findings })
}

/// 体检的print_diagnosis面（细则见模块文档与集成测试）。
pub fn print_diagnosis(d: &Diagnosis) {
    for f in &d.findings {
        println!(
            "agent={} check={} status={} path={} detail={}",
            f.agent,
            f.check,
            f.status.as_str(),
            f.path,
            f.detail.replace('\n', " ")
        );
    }
    println!("doctor.blocked={}", d.blocked());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn state_blocked_is_a_finding() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let oma = temp_root("pstate");
        std::env::set_var("HST_ROOT", &oma);
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let state = crate::pathutil::project_dir(&root).join("state");
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("codex.json"), r#"{"state":"blocked"}"#).unwrap();
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_ROOT");
        assert_eq!(d.status("codex", "state"), Some(Status::Block));
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn user_level_state_never_blocks_project_diagnosis() {
        // D28 用户级状态面：HST_ROOT 缝注入（共享 env 锁与 hook 测试互斥）。
        // 用户级 blocked（latest 与 session 键两种）不升 Block——无法归因本
        // 项目；项目级旧文件 blocked 仍 Block。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let oma = temp_root("ustate");
        fs::create_dir_all(oma.join("state")).unwrap();
        fs::write(
            oma.join("state").join("claude.json"),
            r#"{"state":"blocked"}"#,
        )
        .unwrap();
        fs::write(
            oma.join("state").join("kimi-s9.json"),
            r#"{"state":"blocked"}"#,
        )
        .unwrap();
        fs::write(
            oma.join("state").join("grok.json"),
            r#"{"state":"working"}"#,
        )
        .unwrap();
        std::env::set_var("HST_ROOT", &oma);
        let root = temp_root("ustate-proj");
        let proj_state = crate::pathutil::project_dir(&root).join("state");
        fs::create_dir_all(&proj_state).unwrap();
        fs::write(proj_state.join("codex.json"), r#"{"state":"blocked"}"#).unwrap();
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_ROOT");
        // 用户级 blocked：warn 不 block。
        assert_eq!(d.status("claude", "state"), Some(Status::Warn));
        let kimi_rows: Vec<&Finding> = d
            .findings
            .iter()
            .filter(|f| f.agent == "kimi" && f.check == "state")
            .collect();
        assert!(
            kimi_rows.iter().any(|f| f.status == Status::Warn),
            "keyed blocked row is warn: {kimi_rows:?}"
        );
        // 正常态照旧 ok。
        assert_eq!(d.status("grok", "state"), Some(Status::Ok));
        // 项目级旧文件 blocked 仍 Block（本项目归因）。
        assert_eq!(d.status("codex", "state"), Some(Status::Block));
        assert!(d.blocked());
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_project_splits_trust_kinds() {
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-kinds-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(&root).unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(d.status("claude", "trust.hooks"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Ok));
        assert_eq!(d.status("kimi", "trust.hooks"), Some(Status::Ok));
        assert_eq!(d.status("grok", "trust.skill"), Some(Status::Ok));
        assert_eq!(d.status("grok", "trust.project"), Some(Status::Ok));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn claude_mcp_project_approval_ignored_until_folder_trust() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-mcp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(
            root.join(".mcp.json"),
            r#"{"mcpServers":{"demo":{"command":"echo"}}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"enableAllProjectMcpServers":true}"#,
        )
        .unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        let user = dirs::home_dir()
            .unwrap()
            .join(".claude")
            .join("settings.json");
        let user_ok = json_file(&user)
            .as_ref()
            .map(mcp_servers_approved)
            .unwrap_or(false);
        if user_ok {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        } else {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Block));
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn regular_claude_skills_block_without_folder_trust() {
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-skill-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude").join("skills").join("demo")).unwrap();
        fs::write(
            root.join(".claude")
                .join("skills")
                .join("demo")
                .join("SKILL.md"),
            "# demo\n",
        )
        .unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Block));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yolo_alone_does_not_clear_mcp_or_skill_trust() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("yolo-user");
        fs::create_dir_all(&user).unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-yolo-gates-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude").join("skills").join("demo")).unwrap();
        fs::write(
            root.join(".claude")
                .join("skills")
                .join("demo")
                .join("SKILL.md"),
            "# demo\n",
        )
        .unwrap();
        fs::write(
            root.join(".mcp.json"),
            r#"{"mcpServers":{"demo":{"command":"echo"}}}"#,
        )
        .unwrap();
        crate::yolo::apply_user_yolo_with(&user).expect("yolo");
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_USER_HOME");
        assert_eq!(d.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Block));
        // 用户级 enableAll 已由 yolo 面写入（缝内家），项目 MCP 由此覆盖。
        assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    // ===== 部署诊断扩展（S025/S026 判据） =====

    fn temp_root(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oma-doctor-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ))
    }

    #[test]
    fn grok_login_covers_s026_rules() {
        let now = OffsetDateTime::parse("2026-09-02T12:00:00Z", &Rfc3339).unwrap();
        let live =
            json!({"https://auth.x.ai::u": {"key": "k", "expires_at": "2026-09-02T13:00:00Z"}});
        assert_eq!(grok_login_state(Some(&live), now).0, Status::Ok);
        let refreshable =
            json!({"s": {"key": "k", "refresh_token": "r", "expires_at": "2026-09-02T11:00:00Z"}});
        let (st, detail) = grok_login_state(Some(&refreshable), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("refresh_token"));
        // 提前 300s 视过期：now+299s 落过期分支，now+301s 存活
        let edge_in = json!({"s": {"key": "k", "expires_at": "2026-09-02T12:04:59Z"}});
        assert_eq!(grok_login_state(Some(&edge_in), now).0, Status::Warn);
        let edge_out = json!({"s": {"key": "k", "expires_at": "2026-09-02T12:05:01Z"}});
        assert_eq!(grok_login_state(Some(&edge_out), now).0, Status::Ok);
        // 无 expires_at 按 create_time + 30 天兜底（2026-08-05 加 30 天 = 09-04）
        let created_live = json!({"s": {"key": "k", "create_time": "2026-08-05T00:00:00Z"}});
        assert_eq!(grok_login_state(Some(&created_live), now).0, Status::Ok);
        let created_stale = json!({"s": {"key": "k", "create_time": "2026-07-01T00:00:00Z"}});
        let (st, detail) = grok_login_state(Some(&created_stale), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("grok login"));
        assert_eq!(grok_login_state(None, now).0, Status::Warn);
        let no_creds = json!({"s": {"email": "x@y"}});
        assert_eq!(grok_login_state(Some(&no_creds), now).0, Status::Warn);
    }

    #[test]
    fn kimi_login_tombstone_is_distinct_from_missing() {
        let now = 1_800_000_000i64;
        let live = json!({"access_token": "t", "refresh_token": "r", "expires_at": now + 3600});
        assert_eq!(kimi_login_state(Some(&live), now).0, Status::Ok);
        // hasToken 不看过期（S026）：过期仍有 token 仍是登录态，刷新自动做
        let expired = json!({"access_token": "t", "expires_at": now - 10});
        assert_eq!(kimi_login_state(Some(&expired), now).0, Status::Ok);
        let tombstone = json!({"access_token": "", "expires_at": now});
        let (st, detail) = kimi_login_state(Some(&tombstone), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("revoked"));
        assert_eq!(kimi_login_state(None, now).0, Status::Warn);
        let no_field = json!({"refresh_token": "r"});
        assert_eq!(kimi_login_state(Some(&no_field), now).0, Status::Warn);
    }

    #[test]
    fn hooks_form_classifies_shim_bare_absolute_none() {
        // shim 判定含在位探针：夹具真文件判 shim，死链判 shim-dead（review
        // 验收补：此前死链也 ok）。
        let dir = std::env::temp_dir().join(format!(
            "oma-doctor-shim-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let alive = dir.join(".oma").join("hooks").join("hst-state.cmd");
        std::fs::create_dir_all(alive.parent().unwrap()).unwrap();
        std::fs::write(&alive, "@echo off\r\n").unwrap();
        let alive_fwd = alive.to_string_lossy().replace('\\', "/");
        let shim = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!("{alive_fwd} claude")}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&shim)), "shim");
        let shim_grok = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!("& \"{alive_fwd}\" grok")}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&shim_grok)), "shim");
        let dead = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "D:/moved-away/.oma/hooks/hst-state.cmd claude"}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&dead)), "shim-dead");
        // D39 包裹形态（codex review F2）：程序位是 -File 后的 shim 脚本，
        // 不是解释器路径；真文件判 shim、死链判 shim-dead。
        let alive_ps1 = dir.join(".oma").join("hooks").join("hst-state.ps1");
        std::fs::write(&alive_ps1, "# ps1\n").unwrap();
        let ps1_fwd = alive_ps1.to_string_lossy().replace('\\', "/");
        let wrapped = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!(
                "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {ps1_fwd} claude"
            )}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&wrapped)), "shim");
        let wrapped_dead = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!(
                "powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:/gone/hst-state.ps1 claude"
            )}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&wrapped_dead)), "shim-dead");
        let _ = std::fs::remove_dir_all(&dir);
        let bare = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "oma hook --agent claude"}]}]}});
        assert_eq!(json_hooks_form(Some(&bare)), "bare");
        let absolute =
            json!({"hooks": {"SessionStart": [{"hooks": [{"command": "D:\\x\\oma.exe hook"}]}]}});
        assert_eq!(json_hooks_form(Some(&absolute)), "absolute");
        let foreign = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "echo hi"}]}]}});
        assert_eq!(json_hooks_form(Some(&foreign)), "none");
        assert_eq!(json_hooks_form(None), "none");
        let args = json!({"hooks": {"SessionStart": [{"hooks": [{
            "command": "oma",
            "args": ["hook", "--agent", "claude"]
        }]}]}});
        assert_eq!(
            json_hooks_form(Some(&args)),
            "args",
            "Grok loads Claude settings; command+args is ParserError (M047)"
        );
    }

    #[test]
    fn codex_side_form_classifies_shim_bare_dead() {
        // 期望来自注册形态语义（M059/M055）：shim 需脚本在位；bare 无路径
        // 分隔符；shim-dead 指 oma-state 但文件缺失；absolute 其余带路径。
        let dir = std::env::temp_dir().join(format!(
            "oma-doctor-codex-form-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let alive = dir.join(".oma").join("hooks").join("hst-state.cmd");
        std::fs::create_dir_all(alive.parent().unwrap()).unwrap();
        std::fs::write(&alive, "@echo off\r\n").unwrap();
        let fwd = alive.to_string_lossy().replace('\\', "/");
        let shim_win = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "oma hook --agent codex", "commandWindows": format!("{fwd} codex")}
        ]}]}});
        assert_eq!(codex_side_form(Some(&shim_win), true), Some("shim"));
        // M055 兜底：command 是 bare（设计态，非宿主侧不 warn 的判据基础）。
        assert_eq!(codex_side_form(Some(&shim_win), false), Some("bare"));
        let dead = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "\"/p/x/.oma/hooks/hst-state.sh\" codex"}
        ]}]}});
        assert_eq!(codex_side_form(Some(&dead), false), Some("shim-dead"));
        let absolute = json!({"hooks": {"SessionStart": [{"hooks": [
            {"commandWindows": "\"D:\\old\\oma.exe\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_side_form(Some(&absolute), true), Some("absolute"));
        // 多条 ours 取最差：shim 加 dead 共存判 dead。
        let mixed = json!({"hooks": {"SessionStart": [
            {"hooks": [{"commandWindows": format!("{fwd} codex")}]},
            {"hooks": [{"commandWindows": "D:/moved/.oma/hooks/hst-state.cmd codex"}]}
        ]}});
        assert_eq!(codex_side_form(Some(&mixed), true), Some("shim-dead"));
        assert_eq!(codex_side_form(None, true), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn codex_hooks_sides_read_per_os_fields() {
        let both = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command",
             "command": "\"/home/ray/.cargo/bin/oma\" hook --agent codex",
             "commandWindows": "& \"D:\\cargo\\oma.exe\" hook --agent codex",
             "timeout": 10}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&both)), (true, true));
        let unix_only = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "command": "\"/home/ray/.cargo/bin/oma\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&unix_only)), (true, false));
        let win_only = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "commandWindows": "& \"D:\\cargo\\oma.exe\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&win_only)), (false, true));
        let foreign = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "command": "echo hi"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&foreign)), (false, false));
        assert_eq!(codex_hooks_sides(None), (false, false));
    }

    #[test]
    fn codex_statusline_state_classifies_argv_builtin_missing() {
        let root = temp_root("codex-sl");
        fs::create_dir_all(root.join(".codex")).unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::Missing);
        fs::write(
            root.join(".codex").join("config.toml"),
            "[tui]\nstatus_line = [\"command\", \"pwsh\", \"-File\", \"C:/x/hst-statusline.ps1\"]\n",
        )
        .unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::CommandArgv);
        fs::write(
            root.join(".codex").join("config.toml"),
            "[tui]\nstatus_line = [\"run-state\", \"git-branch\"]\n",
        )
        .unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::Builtin);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn grok_statusline_state_classifies_cmd_pwsh_missing() {
        // Oracle: grok-build command.rs Command::new(entire string); Windows
        // shell line with quotes is ERROR_INVALID_NAME 123 (M048).
        let root = temp_root("grok-sl");
        fs::create_dir_all(root.join(".grok")).unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::Missing);
        fs::write(
            root.join(".grok").join("config.toml"),
            "[ui.status_line]\ntype = \"command\"\ncommand = \"pwsh -NoProfile -File \\\"C:/x/hst-statusline.ps1\\\" grok\"\n",
        )
        .unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::PwshFile);
        fs::write(
            root.join(".grok").join("config.toml"),
            "[ui.status_line]\ntype = \"command\"\ncommand = \"C:/Users/ray/.ohmyagents/statusline/hst-statusline-grok.cmd\"\n",
        )
        .unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::CmdPath);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn codex_user_hooks_trust_needs_full_key_match() {
        // D28 加 F2 硬化：判据逐键对账（键源 = hooks.json 路径）。oracle =
        // 真 deploy 行为（种真哈希断 Ok；篡改任一键断 Block），不用被测
        // 同款逻辑现算期望（反重言式）。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("codex-user-trust");
        let oma = temp_root("codex-user-trust-oma");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&oma).unwrap();
        let mut report = crate::deploy::DeployReport::default();
        crate::deploy::deploy_user_hooks_with(&user, &oma, crate::deploy::host_side(), &mut report)
            .unwrap();
        let root = temp_root("codex-user-trust-proj");
        fs::create_dir_all(&root).unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(
            d.status("codex", "trust.hooks"),
            Some(Status::Ok),
            "deploy-seeded keys+hashes must pass the per-key check"
        );
        // 篡改任一键哈希 → Block（「有任一 trusted_hash」旧判据在此假绿）。
        let cfg_path = user.join(".codex").join("config.toml");
        let mut cfg = crate::yolo::read_toml(&cfg_path).unwrap();
        let tampered = {
            let Some(state) = cfg
                .get_mut("hooks")
                .and_then(|h| h.get_mut("state"))
                .and_then(|s| s.as_table_mut())
            else {
                panic!("deploy must seed [hooks.state]");
            };
            let mut touched = false;
            for (_k, v) in state.iter_mut() {
                if let Some(entry) = v.as_table_mut() {
                    entry.insert(
                        "trusted_hash".into(),
                        Toml::String("sha256:tampered".into()),
                    );
                    touched = true;
                    break;
                }
            }
            touched
        };
        assert!(tampered, "state table has at least one entry");
        crate::yolo::toml_write(&cfg_path, &cfg).unwrap();
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_USER_HOME");
        assert_eq!(
            d.status("codex", "trust.hooks"),
            Some(Status::Block),
            "tampered hash must fail the per-key check"
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_local_layer_shadows_user_bypass() {
        // D50：项目 settings.local.json 层优先于 settings.json 与用户层
        //（settings 优先级栈，S029 原案 Allow 堆积即在 local 层）——local
        // 层 defaultMode=default 遮蔽用户层 bypass，yolo 检查须 warn。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("mask-user");
        let root = temp_root("mask-proj");
        fs::create_dir_all(user.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        crate::yolo::apply_user_yolo_with(&user).unwrap();
        // 项目 shared 层同 bypass（旧判 ok 的形态），local 层才是遮蔽源。
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"permissions": {"defaultMode": "bypassPermissions"}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude").join("settings.local.json"),
            r#"{"permissions": {"defaultMode": "default"}}"#,
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_USER_HOME");
        let f = d
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo")
            .expect("yolo row");
        assert_eq!(f.status, Status::Warn, "{:?}", f.detail);
        assert!(f.detail.contains("shadows user"), "{:?}", f.detail);
        assert!(
            f.path.ends_with("settings.local.json"),
            "mask source must be the local layer: {}",
            f.path
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_only_bypass_warns_ignored_since_257() {
        // D50 假阳性修正：项目层 bypass-only（用户层无 yolo）此前判 ok；
        // claude 2.1.257 起项目层 bypassPermissions 被忽略，运行时手动模式，
        // 须 warn 带 CTA。项目 + 用户层同为 bypass 仍 ok（回退用户层生效）。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("pbo-user");
        let root = temp_root("pbo-proj");
        let both = temp_root("pbo-both");
        fs::create_dir_all(user.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(both.join(".claude")).unwrap();
        let bypass_proj = r#"{"permissions": {"defaultMode": "bypassPermissions"}}"#;
        fs::write(root.join(".claude").join("settings.json"), bypass_proj).unwrap();
        fs::write(both.join(".claude").join("settings.json"), bypass_proj).unwrap();
        // u=acceptEdits 加 p=bypass：项目层被忽略不构成遮蔽，实际生效
        // acceptEdits（D33 ok），不得走 conflict 误报（codex F3）。
        let partial_user = temp_root("pbo-partial");
        fs::create_dir_all(partial_user.join(".claude")).unwrap();
        fs::write(
            partial_user.join(".claude").join("settings.json"),
            r#"{"permissions": {"defaultMode": "acceptEdits"}}"#,
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        let f = d
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo")
            .expect("yolo row");
        std::env::set_var("HST_USER_HOME", &partial_user);
        let f_partial = diagnose(&root).expect("diagnose partial-user");
        std::env::set_var("HST_USER_HOME", &user);
        crate::yolo::apply_user_yolo_with(&user).unwrap();
        let f_both = diagnose(&both).expect("diagnose both");
        std::env::remove_var("HST_USER_HOME");
        assert_eq!(f.status, Status::Warn, "{:?}", f.detail);
        assert!(
            f.detail.contains("ignored since claude 2.1.257"),
            "{:?}",
            f.detail
        );
        let fp = f_partial
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo")
            .expect("yolo row (partial user)");
        assert_eq!(fp.status, Status::Ok, "{:?}", fp.detail);
        assert!(fp.detail.contains("user level"), "{:?}", fp.detail);
        let fb = f_both
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo")
            .expect("yolo row (both bypass)");
        assert_eq!(fb.status, Status::Ok, "{:?}", fb.detail);
        assert!(
            fb.detail.contains("project bypassPermissions copy ignored"),
            "{:?}",
            fb.detail
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&both);
        let _ = fs::remove_dir_all(&partial_user);
    }

    #[test]
    fn bom_parses_silent_and_garbage_reports_yolo_parse() {
        // D52：带 BOM 的 settings.json 三面（doctor 判据、readblock 读值）
        // 全容忍不假报；真坏文件显式 yolo.parse warn 不再报 missing。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("bom-user");
        let root = temp_root("bom-proj");
        let bad = temp_root("bom-bad");
        for h in [&user, &bad] {
            fs::create_dir_all(h.join(".claude")).unwrap();
        }
        fs::create_dir_all(root.join(".claude")).unwrap();
        crate::yolo::apply_user_yolo_with(&user).unwrap();
        // 用户级文件加 BOM（模拟舰队 PS 脚本重写）。
        let raw = fs::read_to_string(user.join(".claude").join("settings.json")).unwrap();
        fs::write(
            user.join(".claude").join("settings.json"),
            format!("\u{feff}{raw}"),
        )
        .unwrap();
        // 坏文件：正文非法 JSON。
        fs::write(bad.join(".claude").join("settings.json"), "{not json").unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        std::env::set_var("HST_USER_HOME", &bad);
        let b = diagnose(&root).expect("diagnose bad");
        std::env::remove_var("HST_USER_HOME");
        // BOM 面：yolo 判 ok（不假报 missing）、无 yolo.parse 行。
        assert_eq!(
            d.status("claude", "yolo"),
            Some(Status::Ok),
            "BOM tolerated"
        );
        assert!(
            !d.findings.iter().any(|f| f.check == "yolo.parse"),
            "no parse warn for BOM file"
        );
        // 坏文件：yolo.parse warn 点名，不静默。
        let pf = b
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo.parse")
            .expect("yolo.parse row for garbage file");
        assert_eq!(pf.status, Status::Warn, "{:?}", pf.detail);
        assert!(pf.path.ends_with("settings.json"), "{}", pf.path);
        // codex F4：坏文件下 yolo 行不再假报 missing 加 block。
        let yb = b
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo")
            .expect("yolo row for bad file");
        assert_eq!(yb.status, Status::Warn, "{:?}", yb.detail);
        assert!(yb.detail.contains("unparseable"), "{:?}", yb.detail);
        // codex F3：UTF-16（读得出错非 NotFound）也判 Bad 不假报 missing。
        let utf16: Vec<u8> = "{".encode_utf16().flat_map(|w| w.to_le_bytes()).collect();
        fs::write(bad.join(".claude").join("settings.json"), utf16).unwrap();
        std::env::set_var("HST_USER_HOME", &bad);
        let u16d = diagnose(&root).expect("diagnose utf16");
        std::env::remove_var("HST_USER_HOME");
        assert!(
            u16d.findings.iter().any(|f| f.check == "yolo.parse"),
            "utf16 file must surface yolo.parse, not silent missing"
        );
        for h in [user, root, bad] {
            let _ = fs::remove_dir_all(h);
        }
    }

    #[test]
    fn ask_rules_and_read_block_warn_as_residual_blockers() {
        // D50（S029 追记第 1、2 类）：项目层 permissions.ask 非空与
        // blockReadsOutsideWorkingDirectories=true 都是 bypass 名义下的
        // 残余阻塞源，各打 warn；干净配置不出现两检查行。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("res-user");
        let root = temp_root("res-proj");
        let clean = temp_root("res-clean");
        fs::create_dir_all(user.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(clean.join(".claude")).unwrap();
        crate::yolo::apply_user_yolo_with(&user).unwrap();
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"permissions": {
                "ask": ["Bash(rm:*)"],
                "blockReadsOutsideWorkingDirectories": true
            }}"#,
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        let c = diagnose(&clean).expect("diagnose clean");
        std::env::remove_var("HST_USER_HOME");
        let ask = d
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo.ask")
            .expect("yolo.ask row");
        assert_eq!(ask.status, Status::Warn, "{:?}", ask.detail);
        assert!(ask.detail.contains("1 rule"), "{:?}", ask.detail);
        let rb = d
            .findings
            .iter()
            .find(|f| f.agent == "claude" && f.check == "yolo.readblock")
            .expect("yolo.readblock row");
        assert_eq!(rb.status, Status::Warn, "{:?}", rb.detail);
        assert!(
            !c.findings
                .iter()
                .any(|f| f.check == "yolo.ask" || f.check == "yolo.readblock"),
            "clean project must not carry the residual-blocker checks"
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&clean);
    }

    #[test]
    fn kimi_hooks_form_flags_dead_shim() {
        // F8 回归钉：kimi 用户级 [[hooks]] 指向缺失 shim 判 shim-dead（与
        // claude / grok 面同款在位探针）。
        let dir = temp_root("kimi-form");
        let dead = dir.join(".oma").join("hooks").join("hst-state.cmd");
        fs::create_dir_all(dead.parent().unwrap()).unwrap();
        let fwd = dead.to_string_lossy().replace('\\', "/");
        let cfg_text = format!(
            "[[hooks]]
event = \"Stop\"
command = \"{fwd} kimi\"
timeout = 10
"
        );
        let cfg = toml::from_str::<Toml>(&cfg_text).unwrap();
        assert_eq!(kimi_hooks_form(Some(&cfg)), "shim-dead");
        fs::write(
            &dead,
            "@echo off
",
        )
        .unwrap();
        let cfg = toml::from_str::<Toml>(&cfg_text).unwrap();
        assert_eq!(kimi_hooks_form(Some(&cfg)), "shim");
        assert_eq!(kimi_hooks_form(None), "none");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dual_level_yolo_conflict_warns_with_cta() {
        // D28 第 4 令：用户级与项目级并存且值不同 → warn 加对齐 CTA（项目
        // 遮蔽用户）。缝注入双根（共享 env 锁）。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = temp_root("cta-user");
        let root = temp_root("cta-proj");
        fs::create_dir_all(user.join(".claude")).unwrap();
        fs::create_dir_all(user.join(".codex")).unwrap();
        fs::create_dir_all(user.join(".kimi-code")).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::create_dir_all(root.join(".kimi-code")).unwrap();
        // 用户级 = 全 yolo。
        crate::yolo::apply_user_yolo_with(&user).unwrap();
        // 项目级 = 收紧值（用户自设形态）。
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"permissions": {"defaultMode": "acceptEdits"}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex").join("config.toml"),
            "sandbox_mode = \"read-only\"
approval_policy = \"on-request\"
",
        )
        .unwrap();
        fs::write(
            root.join(".kimi-code").join("config.toml"),
            "default_permission_mode = \"manual\"
",
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_USER_HOME");
        for agent in ["claude", "codex", "kimi"] {
            let f = d
                .findings
                .iter()
                .find(|f| f.agent == agent && f.check == "yolo")
                .unwrap_or_else(|| panic!("yolo row for {agent}"));
            assert_eq!(f.status, Status::Warn, "{agent}: {:?}", f.detail);
            assert!(
                f.detail.contains("conflict") && f.detail.contains("hst init --project-yolo"),
                "CTA present: {}",
                f.detail
            );
        }
        // 同值双级（项目对齐用户）不告警。
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"permissions": {"defaultMode": "bypassPermissions"}}"#,
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("HST_USER_HOME");
        assert_eq!(d.status("claude", "yolo"), Some(Status::Ok));
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deploy_diagnosis_rows_never_block() {
        // 契约：登录态/状态栏/hook 形态行是部署诊断面，只 ok|warn，
        // 不得混入 block——blocked() 语义仍只对交互阻塞负责。
        let root = temp_root("warn");
        fs::create_dir_all(&root).unwrap();
        let d = diagnose(&root).expect("diagnose");
        for f in &d.findings {
            if matches!(f.check, "login" | "statusline" | "hooks.form") {
                assert_ne!(
                    f.status,
                    Status::Block,
                    "{} {} must not block",
                    f.agent,
                    f.check
                );
            }
        }
    }
}
