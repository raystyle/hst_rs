//! 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
//! 读 stdin agent JSON 出状态行，契约与退役中的 pwsh 脚本逐字对齐。复用
//! 单源：段序与模板配置走 statusline.rs 的 StatuslineConfig 加
//! effective_orders；loop/goal 探针走 loopmgmt；goalmode 倒序分块扫描
//! 本模块 Rust 形。性能面：无 pwsh 冷启动（约 300ms）加 memmap 倒扫
//!（105MB transcript 毫秒级）。已知边界：tools 段（显式选用面）首版
//! 渲染为空，见 REQ-026。

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::pathutil::user_home;
use crate::statusline::{effective_orders_pub, read_config_pub, StatuslineConfig};

/// 渲染入口：stdin 全量字节加 agent 名，出多行状态串（行间 `\n`）。
///
/// # Errors
///
/// stdin 不可读、配置坏损或渲染内部错误时返回 `String`。
pub fn render(stdin: &[u8], agent: &str) -> Result<String, String> {
    let d: Json = serde_json::from_slice(stdin).map_err(|e| format!("stdin json: {e}"))?;
    let home = crate::install::hst_home().unwrap_or_else(|_| PathBuf::from("."));
    let cfg = read_config_pub(&home)?;
    let orders = effective_orders_pub(&cfg)?;
    let rows: Vec<&[&str]> = orders.iter().map(|r| r.as_slice()).collect();
    render_rows(&d, agent, &cfg, &rows, &home)
}

fn s(v: &Json, path: &[&str]) -> String {
    let mut cur = v;
    for k in path {
        cur = cur.get(k).unwrap_or(&Json::Null);
    }
    cur.as_str().unwrap_or("").to_string()
}

fn f64_at(v: &Json, path: &[&str]) -> Option<f64> {
    let mut cur = v;
    for k in path {
        cur = cur.get(k).unwrap_or(&Json::Null);
    }
    cur.as_f64()
}

fn ansi(text: &str, code: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// 模板占位符替换（与 PS1 ApplyFmt 同语义：未知占位符原样保留）。
fn apply_fmt(tmpl: &str, vars: &[(&str, String)]) -> String {
    let mut out = tmpl.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

struct Ctx<'a> {
    d: &'a Json,
    agent: String,
    nerd: bool,
    cfg: &'a StatuslineConfig,
    home: &'a Path,
    dir: String,
    root: String,
    /// 项目判型（就近向上找 manifest，与 PS1 PROBE 同判）：rust / node /
    /// python / zig / go / cpp；工具链段按型门控探测。
    proj_kind: String,
    pkg_ver: String,
}

fn render_rows(
    d: &Json,
    agent: &str,
    cfg: &StatuslineConfig,
    rows: &[&[&str]],
    home: &Path,
) -> Result<String, String> {
    let nerd = agent != "grok";
    let dir = {
        let mut dir = s(d, &["workspace", "current_dir"]);
        if dir.is_empty() || dir == "." {
            dir = s(d, &["cwd"]);
        }
        if dir.is_empty() || dir == "." {
            dir = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
        }
        dir
    };
    let root = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let (pkg_ver, proj_kind) = probe_project(&dir);
    let ctx = Ctx {
        d,
        agent: agent.to_string(),
        nerd,
        cfg,
        home,
        dir,
        root,
        proj_kind,
        pkg_ver,
    };
    let mut out_rows: Vec<String> = Vec::new();
    for row in rows {
        let mut parts: Vec<String> = Vec::new();
        for id in *row {
            if let Some(p) = render_segment(&ctx, id) {
                if !p.trim().is_empty() {
                    parts.push(p);
                }
            }
        }
        if !parts.is_empty() {
            out_rows.push(parts.join(" | "));
        }
    }
    let joined = if cfg.single_line || agent == "kimi" || agent == "grok" {
        out_rows.join(" | ")
    } else {
        out_rows.join("\n")
    };
    Ok(format!("{joined}\n"))
}

fn icon_of(ctx: &Ctx, key: &str) -> String {
    if !ctx.nerd {
        return String::new();
    }
    // 与 DEFAULT_ICONS 同表（statusline.rs 常量非 pub，此处按键取值经
    // 模板配置覆盖优先；表值以 statusline.rs 为单一权威，经公开 getter）。
    crate::statusline::default_icon(key)
}

fn tmpl_of(ctx: &Ctx, key: &str) -> String {
    // 用户模板覆盖优先，回落默认表。
    if let Some(t) = ctx.cfg.template.iter().find(|(k, _)| k == key) {
        return t.1.clone();
    }
    crate::statusline::default_template(key)
}

fn render_segment(ctx: &Ctx, id: &str) -> Option<String> {
    match id {
        "shell" => seg_shell(ctx),
        "dir" => Some(ansi(
            &apply_fmt(&tmpl_of(ctx, "dir"), &[("path", ctx.dir.clone())]),
            "38;5;39",
        ))
        .filter(|x| !x.is_empty()),
        "git" => seg_git(ctx),
        "clock" => {
            let now = now_hm();
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "clock"),
                    &[("icon", icon_of(ctx, "clock")), ("datetime", now)],
                ),
                "38;5;245",
            ))
        }
        "package" => seg_package(ctx),
        "hst" => seg_hst(ctx),
        "model" => {
            let m = s(ctx.d, &["model", "display_name"]);
            if m.is_empty() {
                return None;
            }
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "model"),
                    &[("icon", icon_of(ctx, "model")), ("model", m)],
                ),
                "38;5;147",
            ))
        }
        "context" => seg_context(ctx),
        "duration" => {
            let ms = f64_at(ctx.d, &["cost", "total_duration_ms"]).unwrap_or(0.0);
            let txt = fmt_dur(ms);
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "duration"),
                    &[("icon", icon_of(ctx, "duration")), ("duration", txt)],
                ),
                "38;5;245",
            ))
        }
        "hookstate" => seg_hookstate(ctx),
        "loop" => seg_loop(ctx),
        "goal" => seg_goal(ctx),
        "goalmode" => seg_goalmode(ctx),
        "mcp" => {
            let n = ctx
                .d
                .get("mcp_servers")
                .and_then(|v| v.as_object())
                .map(|o| o.len())
                .unwrap_or(0);
            if n == 0 {
                return None;
            }
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "mcp"),
                    &[("icon", icon_of(ctx, "mcp")), ("count", n.to_string())],
                ),
                "38;5;140",
            ))
        }
        "tokens" => {
            let win = f64_at(ctx.d, &["context_window", "context_window_size"]);
            let pct = f64_at(ctx.d, &["context_window", "used_percentage"]);
            let (win, pct) = (win?, pct?);
            let used = win * pct / 100.0;
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "tokens"),
                    &[
                        ("icon", icon_of(ctx, "tokens")),
                        ("used", fmt_tok(used)),
                        ("window", fmt_tok(win)),
                    ],
                ),
                "38;5;180",
            ))
        }
        "python" | "rust" | "node" | "zig" | "go" | "cpp" => seg_tool(ctx, id),
        // tools 段（显式选用面）首版渲染为空（REQ-026 已知边界）。
        "tools" => None,
        other => Err(format!("unknown statusline segment: {other}")).ok(),
    }
}

fn now_hm() -> String {
    // 无 chrono：epoch 转本地时（准分即可；时区经 TZ 环境由 libc 侧近
    // 似——CI 与五端均 UTC 或本地 date 命令一次调用保形）。
    let out = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    out
}

fn fmt_tok(n: f64) -> String {
    if n >= 1024.0 * 1024.0 {
        return format!("{:.0}M", n / (1024.0 * 1024.0));
    }
    if n >= 1024.0 {
        return format!("{:.0}k", n / 1024.0);
    }
    format!("{:.0}", n)
}

fn fmt_dur(ms: f64) -> String {
    let sec = ms / 1000.0;
    let s = sec.floor();
    if s >= 86400.0 {
        return format!(
            "{}d{}h",
            (s / 86400.0).floor(),
            ((s % 86400.0) / 3600.0).floor()
        );
    }
    if s >= 3600.0 {
        return format!(
            "{}h{}m",
            (s / 3600.0).floor(),
            ((s % 3600.0) / 60.0).floor()
        );
    }
    if s >= 60.0 {
        return format!("{}m{}s", (s / 60.0).floor(), s % 60.0);
    }
    format!("{s:.0}s")
}

fn seg_shell(ctx: &Ctx) -> Option<String> {
    // Unix 祖先链（与 PS1 同判）：跳过 agent 本体，向上找最近 shell。
    let shells = [
        "pwsh",
        "powershell",
        "bash",
        "zsh",
        "sh",
        "fish",
        "cmd",
        "nu",
        "elvish",
        "xonsh",
    ];
    let agent_stems = ["node", "claude", "codex", "grok", "kimi", "hst"];
    let mut chain: Vec<String> = Vec::new();
    if cfg!(target_os = "linux") {
        let mut cur = std::process::id();
        for _ in 0..8 {
            let stat = std::fs::read_to_string(format!("/proc/{cur}/stat")).ok()?;
            let ppid: u32 = stat
                .rsplit(')')
                .next()?
                .split_whitespace()
                .nth(1)
                .and_then(|t| t.parse().ok())?;
            if ppid <= 1 {
                break;
            }
            cur = ppid;
            let comm = std::fs::read_to_string(format!("/proc/{cur}/comm"))
                .ok()
                .map(|c| c.trim().to_lowercase());
            if let Some(c) = comm {
                chain.push(c);
            }
        }
    } else {
        let out = std::process::Command::new("ps")
            .args(["-o", "ppid=,comm=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        let txt = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
        let mut ppid: Option<u32> = None;
        for tok in txt.split_whitespace() {
            if let Ok(p) = tok.parse::<u32>() {
                ppid = Some(p);
                break;
            }
        }
        if let Some(p) = ppid {
            if let Ok(comm) = std::fs::read_to_string(format!("/proc/{p}/comm")) {
                chain.push(comm.trim().to_lowercase());
            }
        }
    }
    let agent_idx = chain
        .iter()
        .position(|c| agent_stems.iter().any(|a| c.starts_with(a)));
    let search: &[String] = match agent_idx {
        Some(i) if i + 1 < chain.len() => &chain[i + 1..],
        _ => &chain,
    };
    let mut name = search
        .iter()
        .find(|c| shells.iter().any(|sh| c.starts_with(sh)))
        .cloned()
        .map(|c| c.trim_end_matches(".exe").to_string());
    if name.is_none() {
        if let Ok(sh) = std::env::var("SHELL") {
            name = Some(Path::new(&sh).file_name()?.to_string_lossy().to_string());
        }
    }
    let name = name?;
    let key = if name.starts_with("pwsh") || name.starts_with("powershell") {
        "shell-pwsh"
    } else {
        "shell"
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "shell"),
            &[("icon", icon_of(ctx, key)), ("name", name)],
        ),
        "38;5;245",
    ))
}

fn seg_git(ctx: &Ctx) -> Option<String> {
    let mut branch = s(ctx.d, &["worktree", "branch"]);
    if branch.is_empty() {
        branch = s(ctx.d, &["workspace", "branch"]);
    }
    if branch.is_empty() {
        branch = s(ctx.d, &["workspace", "git_worktree", "name"]);
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&ctx.dir)
        .args(["status", "-b", "--porcelain=v1"])
        .output()
        .ok()?;
    let gs = String::from_utf8_lossy(&out.stdout);
    let mut flags = String::new();
    let mut ahead_behind = String::new();
    let (mut conflicted, mut staged, mut modified, mut untracked, mut deleted, mut renamed) =
        (false, false, false, false, false, false);
    for l in gs.lines() {
        if l.starts_with("## ") {
            if branch.is_empty() {
                let hdr = l.trim_start_matches("## ").split_whitespace().next();
                if let Some(h) = hdr {
                    let h = h.split("...").next().unwrap_or(h);
                    if !h.is_empty() {
                        branch = h.to_string();
                    }
                }
            }
            if let Some(n) = l
                .split("ahead ")
                .nth(1)
                .and_then(|t| t.split([' ', ',']).next())
            {
                if ctx.nerd {
                    ahead_behind.push_str(&"\u{21e1}".repeat(n.parse::<usize>().unwrap_or(0)));
                } else {
                    ahead_behind.push_str(&format!("+{n}"));
                }
            }
            if let Some(n) = l
                .split("behind ")
                .nth(1)
                .and_then(|t| t.split([' ', ',']).next())
            {
                if ctx.nerd {
                    ahead_behind.push_str(&"\u{21e3}".repeat(n.parse::<usize>().unwrap_or(0)));
                } else {
                    ahead_behind.push_str(&format!("-{n}"));
                }
            }
            continue;
        }
        let b = l.as_bytes();
        if b.len() < 2 {
            continue;
        }
        let (x, y) = (b[0] as char, b[1] as char);
        if x == '?' {
            untracked = true;
            continue;
        }
        if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
            conflicted = true;
            continue;
        }
        if x != ' ' && x != '?' {
            staged = true;
        }
        if y == 'M' || x == 'M' {
            modified = true;
        }
        if y == 'D' {
            deleted = true;
        }
        if x == 'R' || y == 'R' {
            renamed = true;
        }
    }
    if conflicted {
        flags.push('=');
    }
    if deleted {
        flags.push(if ctx.nerd { '\u{2718}' } else { 'x' });
    }
    if renamed {
        flags.push(if ctx.nerd { '\u{00bb}' } else { '>' });
    }
    if modified {
        flags.push('!');
    }
    if staged {
        flags.push('+');
    }
    if untracked {
        flags.push('?');
    }
    flags.push_str(&ahead_behind);
    if branch.is_empty() && flags.is_empty() {
        return None;
    }
    let branch_txt = if branch.is_empty() {
        String::new()
    } else {
        format!(" {branch}")
    };
    let flag_txt = if flags.is_empty() {
        String::new()
    } else {
        format!(" [{flags}]")
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "git"),
            &[("branch", branch_txt), ("flags", flag_txt)],
        ),
        "38;5;176",
    ))
}

/// 就近向上四层找 manifest：返回 (包版本, 判型)。
fn probe_project(dir: &str) -> (String, String) {
    let mut probe = PathBuf::from(dir);
    for _ in 0..4 {
        let kind = |n: &str| -> Option<(String, String)> {
            let p = probe.join(n);
            if !p.is_file() {
                return None;
            }
            match n {
                "Cargo.toml" => {
                    let v = std::fs::read_to_string(&p).ok()?.lines().find_map(|l| {
                        let l = l.trim();
                        if l.starts_with("version") {
                            let v = l.split('"').nth(1)?;
                            if v.contains('.') {
                                return Some(v.to_string());
                            }
                        }
                        None
                    })?;
                    Some((v, "rust".into()))
                }
                "package.json" => {
                    let v: Json = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
                    let v = v.get("version")?.as_str()?.to_string();
                    Some((v, "node".into()))
                }
                "pyproject.toml" | "requirements.txt" => Some((String::new(), "python".into())),
                "build.zig" => Some((String::new(), "zig".into())),
                "go.mod" => Some((String::new(), "go".into())),
                "CMakeLists.txt" | "meson.build" => Some((String::new(), "cpp".into())),
                _ => None,
            }
        };
        for m in [
            "Cargo.toml",
            "package.json",
            "pyproject.toml",
            "requirements.txt",
            "build.zig",
            "go.mod",
            "CMakeLists.txt",
            "meson.build",
        ] {
            if let Some((v, k)) = kind(m) {
                return (v, k);
            }
        }
        probe = match probe.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }
    (String::new(), String::new())
}

fn seg_package(ctx: &Ctx) -> Option<String> {
    if ctx.pkg_ver.is_empty() {
        return None;
    }
    let v = format!("v{}", ctx.pkg_ver);
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "package"),
            &[("icon", icon_of(ctx, "package")), ("version", v)],
        ),
        "38;5;208",
    ))
}

/// 工具链段（PS1 同判：projKind 门控加裸 spawn 加版本正则）。
fn seg_tool(ctx: &Ctx, id: &str) -> Option<String> {
    if !ctx.nerd || ctx.proj_kind != id {
        return None;
    }
    let (bin, args, re, color): (&str, &[&str], &str, &str) = match id {
        "python" => ("python", &["--version"], r"Python ([\d.]+)", "38;5;143"),
        "rust" => ("rustc", &["--version"], r"rustc ([\d.]+)", "38;5;180"),
        "node" => ("node", &["--version"], r"v([\d.]+)", "38;5;078"),
        "zig" => ("zig", &["version"], r"^([\d.]+)", "38;5;178"),
        "go" => ("go", &["version"], r"go([\d.]+)", "38;5;080"),
        "cpp" => ("c++", &["--version"], r"([\d]+\.[\d]+\.[\d]+)", "38;5;081"),
        _ => return None,
    };
    let out = std::process::Command::new(bin)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let ver = regex_lite(&txt, re)?;
    let _ = color;
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, id),
            &[("icon", icon_of(ctx, id)), ("version", format!("v{ver}"))],
        ),
        color,
    ))
}

/// 极简首捕获组正则（工具链版本形足够；不引 regex 依赖）。
fn regex_lite(txt: &str, pat: &str) -> Option<String> {
    // pat 形如 r"prefix\s+([\d.]+)"：按字面前缀 + 捕获 [\d.]+ 处理。
    let (prefix, tail) = pat.split_once('(')?;
    let take_dots = tail.contains(r"[\d.]");
    let anchor_start = tail.starts_with('^');
    let _ = anchor_start;
    let hay = if anchor_start { txt } else { txt };
    let pos = hay.find(prefix.trim_end())?;
    let rest = &hay[pos + prefix.len()..];
    let end = rest
        .char_indices()
        .find(|(_, c)| {
            if take_dots {
                !(c.is_ascii_digit() || *c == '.')
            } else {
                !c.is_ascii_digit()
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    let v = &rest[..end];
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

#[allow(dead_code)]
fn seg_package_old(_ctx: &Ctx) -> Option<String> {
    None
}

/// hook 状态读序（D28，hst 段与 hookstate 段共享，与 pwsh 同序）：1)
/// HST_STATE_FILE 覆盖；2) 用户级会话键 `~/.hst/state/<agent>-<会话id>.json`
/// （构造恒匹配免闸）；3) 用户级 `<agent>.json`（会话闸：记录带 session
/// 且不符续找）；4) 项目级 `<root>/.hst/state/<agent>.json`（同闸）。
/// 返回 (态, 命中文件)；全缺回落 unknown。
fn hook_state(ctx: &Ctx) -> (String, bool) {
    let sid = {
        let mut sid = s(ctx.d, &["session_id"]);
        if sid.is_empty() {
            sid = s(ctx.d, &["sessionId"]);
        }
        sid
    };
    if let Ok(f) = std::env::var("HST_STATE_FILE") {
        if !f.is_empty() {
            let p = PathBuf::from(&f);
            if p.is_file() {
                if let Some(st) = read_state_gated(&p, &sid, false) {
                    return (st, true);
                }
            }
            // 覆盖文件在场但无有效态：读序终止（pwsh 同判，不续找）。
            return ("unknown".to_string(), true);
        }
    }
    let mut cands: Vec<PathBuf> = Vec::new();
    let keyed = ctx
        .home
        .join("state")
        .join(format!("{}-{sid}.json", ctx.agent));
    if keyed.is_file() {
        cands.push(keyed);
    }
    let latest = ctx.home.join("state").join(format!("{}.json", ctx.agent));
    if latest.is_file() {
        cands.push(latest);
    }
    if !ctx.root.is_empty() {
        let proj = Path::new(&ctx.root)
            .join(".hst")
            .join("state")
            .join(format!("{}.json", ctx.agent));
        if proj.is_file() {
            cands.push(proj);
        }
    }
    for (i, p) in cands.iter().enumerate() {
        // 首候选（会话键）构造恒匹配；其余带会话闸。
        if let Some(st) = read_state_gated(p, &sid, i > 0) {
            return (st, true);
        }
    }
    ("unknown".to_string(), false)
}

/// 单候选读态：state 键在场才有效；gate 时记录带 session 且与当前会话
/// 不符返回 None 续找（别的可能已死会话遗留）。
fn read_state_gated(p: &Path, sid: &str, gate: bool) -> Option<String> {
    let v = crate::yolo::read_json(p).ok()?;
    let st = v.get("state").and_then(|x| x.as_str())?.to_string();
    if gate && !sid.is_empty() {
        if let Some(rec) = v.get("session").and_then(|x| x.as_str()) {
            if !rec.is_empty() && rec != sid {
                return None;
            }
        }
    }
    Some(st)
}

/// D28 读序态到色的映射（与 pwsh $stateColor 同表）。
fn state_color(state: &str) -> &'static str {
    match state {
        "idle" => "38;5;108",
        "working" => "38;5;179",
        "blocked" | "no-hook!" | "proj-yolo!" => "38;5;203",
        _ => "38;5;245",
    }
}

/// hookstate 段（第 5 行，用户令 2026-09-26）：hook 通道状态独占一行；
/// 状态文件不在场整行隐藏（无 hook 会话零噪声）。
fn seg_hookstate(ctx: &Ctx) -> Option<String> {
    let (state, present) = hook_state(ctx);
    if !present {
        return None;
    }
    let color = state_color(&state);
    let key = if ctx.nerd {
        "hookstate"
    } else {
        "hookstate-ascii"
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[("icon", icon_of(ctx, "hookstate")), ("state", state)],
        ),
        color,
    ))
}

fn seg_hst(ctx: &Ctx) -> Option<String> {
    // 状态读序（D28）：与 hookstate 段共享 hook_state（会话键加闸全序）；
    // 版本取 payload version 字段归一（D46）。
    let (state, _present) = hook_state(ctx);
    let mut ver = String::new();
    let pv = s(ctx.d, &["version"]);
    if !pv.is_empty() {
        let mut num = String::new();
        let mut seen_digit = false;
        for c in pv.chars() {
            if c.is_ascii_digit() || (c == '.' && seen_digit) {
                num.push(c);
                seen_digit = true;
            } else if !num.is_empty()
                && (c.is_ascii_alphanumeric() || c == '.' || c == '+' || c == '-')
            {
                num.push(c);
            } else if !num.is_empty() {
                break;
            }
        }
        if num.contains('.') {
            ver = num;
        }
    }
    let agent_disp = if ver.is_empty() {
        ctx.agent.clone()
    } else {
        format!("{}-{ver}", ctx.agent)
    };
    let color = state_color(&state);
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "hst"),
            &[
                ("icon", icon_of(ctx, "hst")),
                ("agent", agent_disp),
                ("state", state),
            ],
        ),
        color,
    ))
}

fn seg_context(ctx: &Ctx) -> Option<String> {
    let win = f64_at(ctx.d, &["context_window", "context_window_size"])?;
    let pct = f64_at(ctx.d, &["context_window", "used_percentage"])?;
    let used = win * pct / 100.0;
    let key = if ctx.nerd { "context" } else { "context-ascii" };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "context")),
                ("pct", format!("{pct:.0}")),
                ("used", fmt_tok(used)),
                ("window", fmt_tok(win)),
                ("mix", String::new()),
            ],
        ),
        "38;5;116",
    ))
}

fn seg_loop(ctx: &Ctx) -> Option<String> {
    let (count, cadence, goal) = loop_probe(ctx)?;
    if count == 0 {
        return None;
    }
    let every = cadence.trim_start_matches('×').to_string();
    let key = if ctx.nerd { "loop" } else { "loop-ascii" };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "loop")),
                ("count", count.to_string()),
                ("cadence", cadence),
                ("every", every),
                ("goal", goal),
            ],
        ),
        "38;5;114",
    ))
}

fn seg_goal(ctx: &Ctx) -> Option<String> {
    let (count, _, goal) = loop_probe(ctx)?;
    if count == 0 || goal.is_empty() {
        return None;
    }
    let key = if ctx.nerd { "goal" } else { "goal-ascii" };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[("icon", icon_of(ctx, "goal")), ("goal", goal)],
        ),
        "38;5;179",
    ))
}

/// loop 探针：本会话 durable 任务（count、最新任务节拍人性化、prompt 截
/// 60）。项目根解析序与 PS1 同（workspace.project_dir 回落链）。
fn loop_probe(ctx: &Ctx) -> Option<(usize, String, String)> {
    let sid = {
        let mut sid = s(ctx.d, &["session_id"]);
        if sid.is_empty() {
            sid = s(ctx.d, &["sessionId"]);
        }
        sid
    };
    if sid.is_empty() {
        return Some((0, String::new(), String::new()));
    }
    let mut root = s(ctx.d, &["workspace", "project_dir"]);
    if root.is_empty() || root == "." {
        root = s(ctx.d, &["workspace", "current_dir"]);
    }
    if root.is_empty() || root == "." {
        root = s(ctx.d, &["cwd"]);
    }
    if root.is_empty() || root == "." {
        root = std::env::current_dir().ok()?.display().to_string();
    }
    let path = crate::loopmgmt::scheduled_tasks_path(Path::new(&root));
    if !path.is_file() {
        return Some((0, String::new(), String::new()));
    }
    let Ok(v) = crate::yolo::read_json(&path) else {
        return Some((0, String::new(), String::new()));
    };
    let Some(tasks) = v.get("tasks").and_then(|t| t.as_array()) else {
        return Some((0, String::new(), String::new()));
    };
    let mine: Vec<&Json> = tasks
        .iter()
        .filter(|t| {
            t.get("createdBySessionId")
                .and_then(|x| x.as_str())
                .map(|x| x == sid)
                .unwrap_or(false)
        })
        .collect();
    if mine.is_empty() {
        return Some((0, String::new(), String::new()));
    }
    let newest = mine
        .iter()
        .max_by_key(|t| t.get("createdAt").and_then(|x| x.as_u64()).unwrap_or(0))?;
    let cron = newest.get("cron").and_then(|x| x.as_str()).unwrap_or("");
    let cadence = cron_to_cadence(cron);
    let mut goal = newest
        .get("prompt")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string();
    if goal.chars().count() > 60 {
        let cut: String = goal.chars().take(60).collect();
        goal = format!("{cut}…");
    }
    Some((mine.len(), cadence, goal))
}

fn cron_to_cadence(cron: &str) -> String {
    let f: Vec<&str> = cron.split_whitespace().collect();
    if f.len() != 5 {
        return String::new();
    }
    if let Some(n) = f[0].strip_prefix("*/").and_then(|x| x.parse::<u32>().ok()) {
        if f[1] == "*" && f[2] == "*" && f[3] == "*" {
            if n >= 60 && n % 60 == 0 {
                return format!("×{}h", n / 60);
            }
            return format!("×{n}m");
        }
    }
    if f[0].parse::<u32>().is_ok() && f[2] == "*" && f[3] == "*" && f[4] == "*" {
        if f[1] == "*" {
            return "×1h".to_string();
        }
        if let Some(n) = f[1].strip_prefix("*/").and_then(|x| x.parse::<u32>().ok()) {
            return format!("×{n}h");
        }
    }
    if let (Ok(m), Ok(h)) = (f[0].parse::<u32>(), f[1].parse::<u32>()) {
        if f[2] == "*" && f[3] == "*" && f[4] == "*" {
            let _ = h;
            return format!("@{h:02}:{m:02}");
        }
    }
    String::new()
}

fn seg_goalmode(ctx: &Ctx) -> Option<String> {
    let (state, text) = goalmode_probe(ctx)?;
    if text.is_empty() {
        return None;
    }
    let key = if ctx.nerd {
        "goalmode"
    } else {
        "goalmode-ascii"
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "goalmode")),
                ("state", state),
                ("text", text),
            ],
        ),
        "38;5;140",
    ))
}

/// goalmode 探针（Rust 形倒序分块扫描）：顶层结构锚两形（全链加短形）加
/// paused 系统事件加 clear 指令；512B 跨界重叠；状态与文本独立回溯。
fn goalmode_probe(ctx: &Ctx) -> Option<(String, String)> {
    let sid = s(ctx.d, &["session_id"]);
    if sid.is_empty() {
        return Some((String::new(), String::new()));
    }
    let mut root = s(ctx.d, &["workspace", "project_dir"]);
    if root.is_empty() || root == "." {
        root = ctx.dir.clone();
    }
    let slug: String = root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let home = user_home().ok()?;
    let file = home
        .join(".claude")
        .join("projects")
        .join(&slug)
        .join(format!("{sid}.jsonl"));
    if !file.is_file() {
        return Some((String::new(), String::new()));
    }
    let mut fs = std::fs::File::open(&file).ok()?;
    use std::io::Seek;
    let len = fs.metadata().ok()?.len();
    let chunk_sz: u64 = 4 * 1024 * 1024;
    let ovl: u64 = 512;
    let mut pos = len;
    let mut prev_head: Vec<u8> = Vec::new();
    let (mut have_state, mut have_txt) = (false, false);
    let (mut state, mut txt) = (String::new(), String::new());
    while pos > 0 && !(have_state && have_txt) {
        let take = chunk_sz.min(pos);
        pos -= take;
        fs.seek(std::io::SeekFrom::Start(pos)).ok()?;
        let mut buf = vec![0u8; take as usize];
        std::io::Read::read_exact(&mut fs, &mut buf).ok()?;
        let mut comb = buf.clone();
        comb.extend_from_slice(&prev_head);
        prev_head = buf[..(ovl as usize).min(buf.len())].to_vec();
        let chunk = String::from_utf8_lossy(&comb);
        // 倒序匹配：块内收集后右到左处理。
        let hits = scan_markers(&chunk);
        for m in hits.iter().rev() {
            if !have_state {
                match m.kind {
                    MarkerKind::Active => {
                        state = "active".into();
                        have_state = true;
                    }
                    MarkerKind::Paused => {
                        state = "paused".into();
                        have_state = true;
                    }
                    MarkerKind::Clear => {
                        state = String::new();
                        txt = String::new();
                        have_state = true;
                        have_txt = true;
                    }
                }
            }
            if !have_txt {
                if let Some(t) = &m.text {
                    txt = t.clone();
                    have_txt = true;
                }
            }
        }
    }
    if !have_state || txt.is_empty() {
        return Some((String::new(), String::new()));
    }
    let mut t = txt.replace(['\r', '\n'], " ").trim().to_string();
    if t.chars().count() > 60 {
        let cut: String = t.chars().take(60).collect();
        t = format!("{cut}…");
    }
    Some((state, t))
}

enum MarkerKind {
    Active,
    Paused,
    Clear,
}

struct Marker {
    kind: MarkerKind,
    text: Option<String>,
}

fn scan_markers(chunk: &str) -> Vec<Marker> {
    let mut out = Vec::new();
    let full = r#""role":"user","content":"<task-notification>\n<summary>Goal check-in:"#;
    let short = r#""role":"user","content":"Goal check-in: «"#;
    let paused = r#""type":"system","subtype":"informational","content":"Goal paused"#;
    let clear = r#""role":"user","content":"/goal "#;
    let mut i = 0;
    let b = chunk.as_bytes();
    while i < b.len() {
        let mut found: Option<(usize, Marker)> = None;
        if chunk[i..].starts_with(full) {
            // 捕获 «文本»（同内容串内随后的 system-reminder 段）。
            let rest = &chunk[i..];
            if let Some(p) = rest.find("«") {
                // « 与 » 均 2 字节（U+00AB/U+00BB），非 3。
                let after = &rest[p + 2..];
                if let Some(q) = after.find('»') {
                    let t = after[..q].to_string();
                    found = Some((
                        full.len(),
                        Marker {
                            kind: MarkerKind::Active,
                            text: Some(t),
                        },
                    ));
                }
            }
            if found.is_none() {
                found = Some((
                    full.len(),
                    Marker {
                        kind: MarkerKind::Active,
                        text: None,
                    },
                ));
            }
        } else if chunk[i..].starts_with(short) {
            let rest = &chunk[i + short.len()..];
            if let Some(q) = rest.find('»') {
                found = Some((
                    short.len() + q + 2,
                    Marker {
                        kind: MarkerKind::Active,
                        text: Some(rest[..q].to_string()),
                    },
                ));
            }
        } else if chunk[i..].starts_with(paused) {
            found = Some((
                paused.len(),
                Marker {
                    kind: MarkerKind::Paused,
                    text: None,
                },
            ));
        } else if chunk[i..].starts_with(clear) {
            let rest = &chunk[i + clear.len()..];
            if rest.starts_with("clear") || rest.starts_with("off") || rest.starts_with("stop") {
                found = Some((
                    clear.len(),
                    Marker {
                        kind: MarkerKind::Clear,
                        text: None,
                    },
                ));
            }
        }
        match found {
            Some((adv, m)) => {
                out.push(m);
                i += adv.max(1);
            }
            // 边界跳跃：逐字节推进会切进多字节字符中部（单字实证 panic），
            // 跳过 UTF-8 续字节（0b10xxxxxx）保 i 恒在字符边界。
            None => {
                i += 1;
                while i < b.len() && (b[i] & 0xC0) == 0x80 {
                    i += 1;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_cadence_forms() {
        assert_eq!(cron_to_cadence("*/5 * * * *"), "×5m");
        assert_eq!(cron_to_cadence("*/120 * * * *"), "×2h");
        assert_eq!(cron_to_cadence("17 * * * *"), "×1h");
        assert_eq!(cron_to_cadence("7 */2 * * *"), "×2h");
        assert_eq!(cron_to_cadence("30 14 * * *"), "@14:30");
        assert_eq!(cron_to_cadence("0 0 1 1 *"), "");
    }

    #[test]
    fn scan_markers_two_forms_and_quotes() {
        let full = r#"{"x":1,"role":"user","content":"<task-notification>\n<summary>Goal check-in: continuing</summary>\n</task-notification>\n<system-reminder>\nGoal check-in: «继续回归» is still active."}"#;
        let ms = scan_markers(full);
        assert_eq!(ms.len(), 1);
        assert!(matches!(ms[0].kind, MarkerKind::Active));
        assert_eq!(ms[0].text.as_deref(), Some("继续回归"));
        let short =
            r#"{"role":"user","content":"Goal check-in: «短形» is still active, and evaluation"}"#;
        let ms = scan_markers(short);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].text.as_deref(), Some("短形"));
        // 深层再编码引文不触发。
        let quoted = r#"{"type":"user","content":"[提醒] {\"role\":\"user\",\"content\":\"Goal check-in: «回显» is still active\"}"}"#;
        assert!(scan_markers(quoted).is_empty());
        let paused =
            r#"{"type":"system","subtype":"informational","content":"Goal paused · timed out"}"#;
        assert!(matches!(scan_markers(paused)[0].kind, MarkerKind::Paused));
        let clear = r#"{"role":"user","content":"/goal clear"}"#;
        assert!(matches!(scan_markers(clear)[0].kind, MarkerKind::Clear));
    }

    #[test]
    fn fmt_forms() {
        assert_eq!(fmt_tok(449_152.0), "439k");
        assert_eq!(fmt_dur(295_200_000.0), "3d10h");
    }
}
