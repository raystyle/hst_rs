//! 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
//! 读 stdin agent JSON 出状态行，契约与退役中的 pwsh 脚本逐字对齐（对版
//! 口径：pwsh 侧强制 `$PSStyle.OutputRendering='Ansi'` 后逐字相同；管道
//! 缺省形态 PowerShell Host 渲染器会剥 ANSI，旧载体在 agent 实际调用里
//! 无色，原生缺省带色，评审 F1）。复用单源：段序与模板配置走
//! statusline.rs 的 StatuslineConfig 加 effective_orders；loop/goal 探针走
//! loopmgmt；goalmode 倒序分块扫描本模块 Rust 形（流式倒扫）。性能面：无
//! pwsh 冷启动（约 300ms）加流式倒扫（105MB transcript 毫秒级）。ANSI 退
//! 裸文本开关：`NO_COLOR` 或 `HST_STATUSLINE_NO_ANSI` 任一非空（评审 F1）。
//! 已知边界：tools 段（显式选用面）首版渲染为空、版本本地探测缓存面
//!（D46）未移植（payload version 字段归一承载）、REQ-014/REQ-017 哨兵面
//! 未移植，见 REQ-026。

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
    if no_ansi() {
        return text.to_string();
    }
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// ANSI 退裸文本开关（评审 F1）：NO_COLOR（标准约定）或
/// HST_STATUSLINE_NO_ANSI 任一非空即剥色。
fn no_ansi() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
        || std::env::var_os("HST_STATUSLINE_NO_ANSI").is_some_and(|v| !v.is_empty())
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
    // root 探测门控（评审 G5，pwsh COMMON 同判）：仅 dir/hst/mcp/hookstate
    // 消费 root（目录显示、状态文件定位）；无消费者的配置不 spawn git。
    let need_root = rows.iter().any(|r| {
        r.iter()
            .any(|id| matches!(*id, "dir" | "hst" | "mcp" | "hookstate"))
    });
    let root = if need_root {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };
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
            if let Some(p) = render_segment(&ctx, id)? {
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
    // 用户 [icons] 键级覆盖优先（评审 F3：pwsh 烘焙表 = 默认并覆盖），
    // 回落 statusline.rs 单一权威默认表。
    if let Some((_, v)) = ctx.cfg.icons.iter().find(|(k, _)| k == key) {
        return v.clone();
    }
    crate::statusline::default_icon(key)
}

fn tmpl_of(ctx: &Ctx, key: &str) -> String {
    // 用户模板覆盖优先，回落默认表。
    if let Some(t) = ctx.cfg.template.iter().find(|(k, _)| k == key) {
        return t.1.clone();
    }
    crate::statusline::default_template(key)
}

/// 段派发（评审二轮 G3：未知段 id 响亮报错对齐 pwsh 部署期硬错；配置
/// typo 不静默丢段）。已知 id 清单镜像 statusline.rs 段注册表。
fn render_segment(ctx: &Ctx, id: &str) -> Result<Option<String>, String> {
    const KNOWN: &[&str] = &[
        "shell",
        "dir",
        "hst",
        "model",
        "context",
        "tools",
        "mcp",
        "tokens",
        "duration",
        "loop",
        "goal",
        "goalmode",
        "hookstate",
        "git",
        "clock",
        "package",
        "python",
        "rust",
        "node",
        "zig",
        "go",
        "cpp",
    ];
    if !KNOWN.contains(&id) {
        return Err(format!(
            "unknown statusline segment: {id}（检查 ~/.hst/statusline.toml 的 segments 清单）"
        ));
    }
    Ok(segment_opt(ctx, id))
}

fn segment_opt(ctx: &Ctx, id: &str) -> Option<String> {
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
            let m = model_name(ctx.d);
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
            let ms = duration_ms(ctx.d)?;
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "duration"),
                    &[
                        ("icon", icon_of(ctx, "duration")),
                        ("duration", fmt_dur(ms)),
                    ],
                ),
                "38;5;245",
            ))
        }
        "hookstate" => seg_hookstate(ctx),
        "loop" => seg_loop(ctx),
        "goal" => seg_goal(ctx),
        "goalmode" => seg_goalmode(ctx),
        "mcp" => {
            let cj = user_home()
                .ok()
                .map(|h| h.join(".claude.json"))
                .filter(|p| p.is_file());
            let mj = Path::new(&ctx.dir).join(".mcp.json");
            let mj = if mj.is_file() { Some(mj) } else { None };
            let n = mcp_count(ctx.d, cj.as_deref(), mj.as_deref());
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
                "38;5;117",
            ))
        }
        "node" => seg_node(ctx),
        "python" | "rust" | "zig" | "go" | "cpp" => seg_tool(ctx, id),
        // tools 段（显式选用面）首版渲染为空（REQ-026 已知边界）。
        "tools" => None,
        _ => None,
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
    // Unix 祖先链（与 PS1 同判）：跳过 agent 本体，向上找最近 shell；
    // 匹配是包含语义（PS1 -match 无锚，路径形 comm 同样命中）。
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
        // macOS/bsd 无 /proc（评审 G2）：单次 ps -ax 全表建 pid 加 ppid
        // 映射再走链，等价 PS1 的逐级 ps 兜底但不逐级 spawn。
        let out = std::process::Command::new("ps")
            .args(["-o", "pid=,ppid=,comm=", "-ax"])
            .output()
            .ok()?;
        let txt = String::from_utf8_lossy(&out.stdout);
        let mut map: std::collections::HashMap<u32, (u32, String)> =
            std::collections::HashMap::new();
        for l in txt.lines() {
            let mut it = l.split_whitespace();
            let (Some(pid), Some(ppid)) = (it.next(), it.next()) else {
                continue;
            };
            let (Ok(pid), Ok(ppid)) = (pid.parse::<u32>(), ppid.parse::<u32>()) else {
                continue;
            };
            let comm = it.collect::<Vec<_>>().join(" ").to_lowercase();
            map.insert(pid, (ppid, comm));
        }
        let mut cur = std::process::id();
        for _ in 0..8 {
            let Some((ppid, comm)) = map.get(&cur) else {
                break;
            };
            if *ppid <= 1 {
                break;
            }
            chain.push(comm.clone());
            cur = *ppid;
        }
    }
    let agent_idx = chain
        .iter()
        .position(|c| agent_stems.iter().any(|a| c.contains(a)));
    let search: &[String] = match agent_idx {
        Some(i) if i + 1 < chain.len() => &chain[i + 1..],
        _ => &chain,
    };
    let mut name = search
        .iter()
        .find(|c| shells.iter().any(|sh| c.contains(sh)))
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

/// porcelain 头行 ahead/behind 标记（评审三轮 F：token 带 `]` 尾随
/// `2]` 直接 parse 恒败按 0 处理，ahead/behind 标记整体丢失；取 token
/// 前导数字段再 parse，pwsh `ahead (\d+)` 同口径）。nerd 形重复箭头字
/// 形、ascii 形正负加数字。
fn ahead_behind_of(hdr: &str, key: &str, nerd: bool, glyph: &str, sign: char) -> String {
    let Some(tok) = hdr.split(key).nth(1) else {
        return String::new();
    };
    let digits: String = tok.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: usize = digits.parse().unwrap_or(0);
    if nerd {
        return glyph.repeat(n);
    }
    (n > 0).then(|| format!("{sign}{n}")).unwrap_or_default()
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
            ahead_behind.push_str(&ahead_behind_of(l, "ahead ", ctx.nerd, "\u{21e1}", '+'));
            ahead_behind.push_str(&ahead_behind_of(l, "behind ", ctx.nerd, "\u{21e3}", '-'));
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
                // pyproject 版本行与 build.zig.zon 的 .version（评审二轮
                // F-new4，pwsh PROBE 同源；requirements.txt 无版本面）。
                "pyproject.toml" | "requirements.txt" => {
                    let v = std::fs::read_to_string(&p).ok().and_then(|t| {
                        t.lines().find_map(|l| {
                            let rest = l.trim_start().strip_prefix("version")?.trim_start();
                            let rest = rest.strip_prefix('=')?.trim_start();
                            let rest = rest.strip_prefix('"')?;
                            let end = rest.find('"')?;
                            (!rest[..end].is_empty()).then(|| rest[..end].to_string())
                        })
                    });
                    Some((v.unwrap_or_default(), "python".into()))
                }
                "build.zig" => {
                    let zon = probe.join("build.zig.zon");
                    let v = std::fs::read_to_string(&zon).ok().and_then(|t| {
                        t.lines().find_map(|l| {
                            let rest = l.trim_start().strip_prefix(".version")?.trim_start();
                            let rest = rest.strip_prefix('=')?.trim_start();
                            let rest = rest.strip_prefix('"')?;
                            let end = rest.find('"')?;
                            (!rest[..end].is_empty()).then(|| rest[..end].to_string())
                        })
                    });
                    Some((v.unwrap_or_default(), "zig".into()))
                }
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

/// 工具链段（PS1 同判：projKind 门控加裸 spawn 加版本提取；提取器按各
/// 家正则口径手写，评审 F8：regex_lite 字面前缀法对 go/zig/cpp 三型失效）。
fn seg_tool(ctx: &Ctx, id: &str) -> Option<String> {
    if !ctx.nerd || ctx.proj_kind != id {
        return None;
    }
    let (bin, args, color): (&str, &[&str], &str) = match id {
        "python" => ("python", &["--version"], "38;5;143"),
        "rust" => ("rustc", &["--version"], "38;5;180"),
        "node" => ("node", &["--version"], "38;5;078"),
        "zig" => ("zig", &["version"], "38;5;178"),
        "go" => ("go", &["version"], "38;5;080"),
        "cpp" => ("c++", &["--version"], "38;5;110"),
        _ => return None,
    };
    let out = std::process::Command::new(bin)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // 正则口径逐家对齐：python/rustc 前缀后随版本（\s+ 空白容忍）、go
    // 逐出现位扫描（首个 go 后随数字串，"go version go1.27" 形跳过首个
    // go）、zig 串首锚（^）、node 首段数字串（v? 无点要求）、cpp 首段
    // 含点数字串（\d+\.\d+(\.\d+)?）。
    let ver = match id {
        "python" => after_prefix(&txt, "Python "),
        "rust" => after_prefix(&txt, "rustc "),
        "node" => first_num_run(&txt, false),
        "zig" => anchored_num_run(&txt),
        "go" => after_prefix(&txt, "go"),
        "cpp" => first_num_run(&txt, true),
        _ => None,
    }?;
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, id),
            &[("icon", icon_of(ctx, id)), ("version", format!("v{ver}"))],
        ),
        color,
    ))
}

/// node 段（评审二轮 F-new1）：node 版本段加 ts 子段（pwsh node 块内
/// 探针：就近四层找 node_modules/typescript/package.json 读版本，不起
/// tsc 子进程；ts 非独立段 id，是 node 块的第二 part，两 part 各自包
/// ANSI 后以行分隔符拼回，渲染逐字等形）。
fn seg_node(ctx: &Ctx) -> Option<String> {
    let node_seg = seg_tool(ctx, "node")?;
    let mut ts_probe = PathBuf::from(&ctx.dir);
    for _ in 0..4 {
        let ts_pj = ts_probe
            .join("node_modules")
            .join("typescript")
            .join("package.json");
        if let Ok(v) = crate::yolo::read_json(&ts_pj) {
            if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                let ts = ansi(
                    &apply_fmt(
                        &tmpl_of(ctx, "ts"),
                        &[("icon", icon_of(ctx, "ts")), ("version", format!("v{ver}"))],
                    ),
                    "38;5;067",
                );
                if !ts.is_empty() {
                    return Some(format!("{node_seg} | {ts}"));
                }
            }
            break;
        }
        match ts_probe.parent() {
            Some(p) => ts_probe = p.to_path_buf(),
            None => break,
        }
    }
    Some(node_seg)
}

/// 前缀后随数字点串（pwsh `prefix\s+([\d.]+)` 口径）：逐出现位找首个
/// 前缀，跳过后随空白取数字点串；该位不中续找下一出现位（go 的
/// "go version go1.27" 形靠此跳过首个 go）。
fn after_prefix(hay: &str, prefix: &str) -> Option<String> {
    let mut from = 0usize;
    while let Some(rel) = hay[from..].find(prefix) {
        let rest = hay[from + rel + prefix.len()..].trim_start();
        let end = rest
            .char_indices()
            .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        if end > 0 {
            return Some(rest[..end].to_string());
        }
        from += rel + 1;
    }
    None
}

/// 首段数字点串（pwsh `([\d.]+)` / `v?([\d.]+)` 口径）：need_dot 时只认
/// 含点串（cpp 的 `\d+\.\d+` 形），否则首段数字串即中（node 形）。
fn first_num_run(hay: &str, need_dot: bool) -> Option<String> {
    let b = hay.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            let cand = &hay[start..i];
            if !need_dot || cand.contains('.') {
                return Some(cand.to_string());
            }
        } else {
            i += 1;
        }
    }
    None
}

/// 串首锚数字点串（pwsh `^([\d.]+)` 口径：zig version 输出直起版本）。
fn anchored_num_run(hay: &str) -> Option<String> {
    let end = hay
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
        .map(|(i, _)| i)
        .unwrap_or(hay.len());
    (end > 0).then(|| hay[..end].to_string())
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
    for p in cands.iter() {
        // 会话闸对全部候选生效（评审 F2：pwsh 同判；会话键文件按构造
        // session 恒匹配，序号免闸在会话键缺位时把用户级 agent 键错放
        // 成免闸候选，跨会话态泄漏）。
        if let Some(st) = read_state_gated(p, &sid, true) {
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

/// hookstate 段（第 5 行，用户令 2026-09-26，同日二令迭代）：hst 状态
/// hook 实际挂载的事件清单加通道状态独占一行（用户令「hook了什么事件」）；
/// 出行门 = 状态文件在场或有挂载事件任一（评审快核 G1：刚装未触发的
/// 空窗期不隐清单，状态缺报回落 unknown）；双缺整行隐藏（零噪声）；
/// 注册面读不出事件时回落泛称 `hook` 保语义。
fn seg_hookstate(ctx: &Ctx) -> Option<String> {
    let (state, present) = hook_state(ctx);
    let events_raw = hooked_events(&ctx.agent);
    // 出行门（评审快核 G1 裁）：状态文件在场或有挂载事件任一即出行——
    // 刚装未触发的空窗期（hook 已注册、state 未写）不应整行隐掉清单；
    // 状态缺报回落 unknown（hst 段同判）。
    if !present && events_raw.is_empty() {
        return None;
    }
    let state = if present {
        state
    } else {
        "unknown".to_string()
    };
    let color = state_color(&state);
    let key = if ctx.nerd {
        "hookstate"
    } else {
        "hookstate-ascii"
    };
    let events = if events_raw.is_empty() {
        "hook".to_string()
    } else {
        events_raw
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "hookstate")),
                ("events", events),
                ("state", state),
            ],
        ),
        color,
    ))
}

/// 注册面事件清单（用户令 2026-09-26「hook了什么事件」）：按 agent 定位
/// 注册文件，取 hst 状态 hook（command 含 `hst-state` 干 stem，REQ-014
/// 同源标记）挂载的事件名，按部署事件序稳定排列（未知事件名字典序殿后）。
/// 文件缺失、坏损或零挂载返回空串。codex 虽无外部状态栏面，手动 render
/// 亦可得清单。
fn hooked_events(agent: &str) -> String {
    match user_home() {
        Ok(h) => hooked_events_at(&h, agent),
        Err(_) => String::new(),
    }
}

/// hooked_events 的可测形（home 显式透传）。
fn hooked_events_at(home: &Path, agent: &str) -> String {
    const ORDER: &[&str] = &[
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let mut found: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    match agent {
        // claude/codex/grok 注册面同构（hooks.<Event>[].hooks[].command）。
        "claude" => collect_json_events(&home.join(".claude").join("settings.json"), &mut found),
        "codex" => collect_json_events(&home.join(".codex").join("hooks.json"), &mut found),
        "grok" => collect_json_events(
            &home
                .join(".grok")
                .join("hooks")
                .join("ohmyagents-state.json"),
            &mut found,
        ),
        // kimi 是 [[hooks]] 表项（event 加 command 平铺）。
        "kimi" => collect_toml_events(&home.join(".kimi-code").join("config.toml"), &mut found),
        _ => {}
    }
    let mut out: Vec<&str> = ORDER
        .iter()
        .filter(|e| found.contains(**e))
        .copied()
        .collect();
    out.extend(
        found
            .iter()
            .filter(|e| !ORDER.contains(&e.as_str()))
            .map(String::as_str),
    );
    out.join(" ")
}

/// JSON 注册面（claude/codex/grok）事件收集：ours 判定 = 任一分组
/// hooks[].command 含 hst-state。
fn collect_json_events(path: &Path, found: &mut std::collections::BTreeSet<String>) {
    let Ok(v) = crate::yolo::read_json(path) else {
        return;
    };
    let Some(obj) = v.get("hooks").and_then(|h| h.as_object()) else {
        return;
    };
    for (event, groups) in obj {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        let ours = groups.iter().any(|g| {
            g.get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .is_some_and(|c| c.contains("hst-state"))
                    })
                })
                .unwrap_or(false)
        });
        if ours {
            found.insert(event.clone());
        }
    }
}

/// TOML 注册面（kimi [[hooks]]）事件收集：ours 判定同干 stem。
fn collect_toml_events(path: &Path, found: &mut std::collections::BTreeSet<String>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(v) = toml::from_str::<toml::Value>(&text) else {
        return;
    };
    let Some(items) = v.get("hooks").and_then(|h| h.as_array()) else {
        return;
    };
    for item in items {
        let ours = item
            .get("command")
            .and_then(|c| c.as_str())
            .is_some_and(|c| c.contains("hst-state"));
        if ours {
            if let Some(ev) = item.get("event").and_then(|e| e.as_str()) {
                found.insert(ev.to_string());
            }
        }
    }
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
    let (pct, win) = ctx_pct_win(ctx.d)?;
    let used = win * pct as f64 / 100.0;
    let key = if ctx.nerd { "context" } else { "context-ascii" };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "context")),
                ("pct", pct.to_string()),
                ("used", fmt_tok(used)),
                ("window", fmt_tok(win)),
                ("mix", String::new()),
            ],
        ),
        "38;5;116",
    ))
}

/// context 段取值（pwsh 同口径，评审 F5）：used_percentage 向下取整
/// 显示；缺 used 时 remaining_percentage 回退 100 减其向下取整；token
/// 绝对值由取整后百分比反推（显示与计算同源）。
fn ctx_pct_win(d: &Json) -> Option<(u64, f64)> {
    let win = f64_at(d, &["context_window", "context_window_size"])?;
    let pct = match f64_at(d, &["context_window", "used_percentage"]) {
        Some(u) => u.floor(),
        None => 100.0 - f64_at(d, &["context_window", "remaining_percentage"])?.floor(),
    };
    Some((pct as u64, win))
}

/// model 段名（pwsh 同判：display_name 优先回落 id，双缺空串）。
fn model_name(d: &Json) -> String {
    let m = s(d, &["model", "display_name"]);
    if m.is_empty() {
        return s(d, &["model", "id"]);
    }
    m
}

/// duration 段取值（pwsh 同判：字段在场且不低于 1000ms 才出段）。
fn duration_ms(d: &Json) -> Option<f64> {
    f64_at(d, &["cost", "total_duration_ms"]).filter(|ms| *ms >= 1000.0)
}

/// mcp 计数（pwsh 同序，评审 F9）：stdin mcp_servers 优先（数组取长、
/// 对象取键数；pwsh PSCustomObject 管道坑把对象形计 1，原生按真实键数，
/// 口径差记 REQ-026），回落 `~/.claude.json` mcpServers 键数加项目
/// `.mcp.json` 键数（用户级加项目级合计）。
fn mcp_count(d: &Json, claude_json: Option<&Path>, mcp_json: Option<&Path>) -> usize {
    let mut n = match d.get("mcp_servers") {
        Some(Json::Array(a)) => a.len(),
        Some(Json::Object(o)) => o.len(),
        _ => 0,
    };
    if n == 0 {
        if let Some(p) = claude_json {
            if let Ok(v) = crate::yolo::read_json(p) {
                if let Some(o) = v.get("mcpServers").and_then(|x| x.as_object()) {
                    n += o.len();
                }
            }
        }
    }
    if let Some(p) = mcp_json {
        if let Ok(v) = crate::yolo::read_json(p) {
            if let Some(o) = v.get("mcpServers").and_then(|x| x.as_object()) {
                n += o.len();
            }
        }
    }
    n
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
    // pwsh 正则逐字对齐（评审 F10 松锚假阳回修）：全链形要求 summary 段
    // （[^"]*）加闭合链 `</summary>…</task-notification>…<system-reminder>`
    // 加 `Goal check-in: «»» is still active` 尾缀；短形同样要求
    // `» is still active` 尾缀。头对而链不全、尾缀缺位的引文一律不中。
    let full_head = r#""role":"user","content":"<task-notification>\n<summary>Goal check-in:"#;
    let full_mid = r#"</summary>\n</task-notification>\n<system-reminder>\nGoal check-in: «"#;
    let short_head = r#""role":"user","content":"Goal check-in: «"#;
    let still = "» is still active";
    let paused = r#""type":"system","subtype":"informational","content":"Goal paused"#;
    let clear = r#""role":"user","content":"/goal "#;
    let mut i = 0;
    let b = chunk.as_bytes();
    while i < b.len() {
        let mut found: Option<(usize, Marker)> = None;
        if chunk[i..].starts_with(full_head) {
            // summary 段 [^"]* 后随闭合链：首个 full_mid 出现位之前不得
            // 有引号（[^"]* 不跨引号；跨过引号的链是别处的引文）。
            let rest = &chunk[i + full_head.len()..];
            if let Some(m) = rest.find(full_mid) {
                if !rest[..m].contains('"') {
                    let t = &rest[m + full_mid.len()..];
                    if let Some(e) = t.find('»') {
                        if t[e..].starts_with(still) {
                            found = Some((
                                full_head.len() + m + full_mid.len() + e + still.len(),
                                Marker {
                                    kind: MarkerKind::Active,
                                    text: Some(t[..e].to_string()),
                                },
                            ));
                        }
                    }
                }
            }
        } else if chunk[i..].starts_with(short_head) {
            let t = &chunk[i + short_head.len()..];
            if let Some(e) = t.find('»') {
                if t[e..].starts_with(still) {
                    found = Some((
                        short_head.len() + e + still.len(),
                        Marker {
                            kind: MarkerKind::Active,
                            text: Some(t[..e].to_string()),
                        },
                    ));
                }
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
    fn git_ahead_behind_and_ascii_forms() {
        // 评审三轮 F 回归锁：token 带 `]` 尾（"2]"）直接 parse 恒败，
        // ahead/behind 标记整体丢失；前导数字段提取后三形俱全。
        let hdr = "## main...origin/main [ahead 2]";
        assert_eq!(
            ahead_behind_of(hdr, "ahead ", true, "\u{21e1}", '+'),
            "\u{21e1}\u{21e1}"
        );
        assert_eq!(ahead_behind_of(hdr, "behind ", true, "\u{21e3}", '-'), "");
        assert_eq!(ahead_behind_of(hdr, "ahead ", false, "\u{21e1}", '+'), "+2");
        let hdr = "## main...origin/main [behind 3, ahead 1]";
        assert_eq!(
            ahead_behind_of(hdr, "behind ", true, "\u{21e3}", '-'),
            "\u{21e3}\u{21e3}\u{21e3}"
        );
        assert_eq!(
            ahead_behind_of(hdr, "ahead ", true, "\u{21e1}", '+'),
            "\u{21e1}"
        );
        assert_eq!(
            ahead_behind_of(hdr, "behind ", false, "\u{21e3}", '-'),
            "-3"
        );
        let hdr = "## main...origin/main";
        assert_eq!(ahead_behind_of(hdr, "ahead ", true, "\u{21e1}", '+'), "");
        // 数字缺失的坏形按 0 处理不出标记。
        let hdr = "## main...origin/main [ahead x]";
        assert_eq!(ahead_behind_of(hdr, "ahead ", true, "\u{21e1}", '+'), "");
    }

    #[test]
    fn hooked_events_reads_registration_and_orders() {
        // 用户令「hook了什么事件」：注册面实读（ours = hst-state 干 stem），
        // 外来 hook 不混入，部署事件序稳定输出，缺文件零命中。
        let tmp = std::env::temp_dir().join(format!("hst-hev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let claude_dir = tmp.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"\"/x/.hst/hooks/hst-state.sh\" claude"}]}],"SessionStart":[{"matcher":"*","hooks":[{"type":"command","command":"bash /other/herdr-agent-state.sh session"},{"type":"command","command":"\"/x/.hst/hooks/hst-state.sh\" claude"}]}],"UserPromptSubmit":[{"matcher":"*","hooks":[{"type":"command","command":"bash /only/foreign.sh"}]}]}}"#,
        )
        .unwrap();
        // 部署序稳定（文件内 Stop 在前、纯外来 UserPromptSubmit 不算）。
        assert_eq!(hooked_events_at(&tmp, "claude"), "SessionStart Stop");
        // kimi TOML 面：ours 表项命中、外来表项排除。
        let kimi_dir = tmp.join(".kimi-code");
        std::fs::create_dir_all(&kimi_dir).unwrap();
        std::fs::write(
            kimi_dir.join("config.toml"),
            "[[hooks]]\nevent = \"PreToolUse\"\ncommand = \"/x/.hst/hooks/hst-state.sh kimi\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"/foreign/other.sh\"\n",
        )
        .unwrap();
        assert_eq!(hooked_events_at(&tmp, "kimi"), "PreToolUse");
        // grok 面（JSON 同构另一路径）加未知事件名字典序殿后。
        let grok_dir = tmp.join(".grok").join("hooks");
        std::fs::create_dir_all(&grok_dir).unwrap();
        std::fs::write(
            grok_dir.join("ohmyagents-state.json"),
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"/x/hst-state.sh grok"}]}],"CustomEvent":[{"matcher":"*","hooks":[{"type":"command","command":"/x/hst-state.sh grok"}]}],"AheadEvent":[{"matcher":"*","hooks":[{"type":"command","command":"/x/hst-state.sh grok"}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_events_at(&tmp, "grok"),
            "Stop AheadEvent CustomEvent",
            "unknown events lexicographic after canonical order"
        );
        // 坏损 JSON 零命中不炸。
        std::fs::write(claude_dir.join("settings.json"), "{ not json").unwrap();
        assert_eq!(hooked_events_at(&tmp, "claude"), "");
        // 缺文件零命中空串（段内回落泛称 hook）。
        std::fs::remove_file(claude_dir.join("settings.json")).unwrap();
        assert_eq!(hooked_events_at(&tmp, "claude"), "");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn fmt_forms() {
        assert_eq!(fmt_tok(449_152.0), "439k");
        assert_eq!(fmt_dur(295_200_000.0), "3d10h");
    }

    #[test]
    fn version_extractors_six_toolchains() {
        // 评审 F8 回归锁：go/zig/cpp 三型在 regex_lite 时代恒不出。
        assert_eq!(
            after_prefix("go version go1.27.0 linux/amd64", "go").as_deref(),
            Some("1.27.0")
        );
        assert_eq!(
            anchored_num_run("0.16.0\n").as_deref(),
            Some("0.16.0"),
            "zig anchored form"
        );
        assert!(anchored_num_run("zig 0.16").is_none(), "non-digit head");
        assert_eq!(
            first_num_run("Apple clang version 15 (c++ 13.3.0)", true).as_deref(),
            Some("13.3.0"),
            "cpp skips version-less runs (need dot)"
        );
        assert_eq!(
            after_prefix("rustc 1.98.1 (abc) stable", "rustc ").as_deref(),
            Some("1.98.1")
        );
        assert_eq!(
            after_prefix("Python 3.12.8", "Python ").as_deref(),
            Some("3.12.8")
        );
        assert_eq!(
            first_num_run("v24.3.0", false).as_deref(),
            Some("24.3.0"),
            "node v-prefix form"
        );
    }

    #[test]
    fn scan_markers_strict_rejections() {
        // 评审 F10 松锚假阳回归锁：短形无尾缀、全链头对链不全、链外引文
        // 三形一律不中（ghost 防线）。
        let loose_short = r#"{"role":"user","content":"Goal check-in: «松锚引文» 别的文本"}"#;
        assert!(scan_markers(loose_short).is_empty(), "no suffix");
        let head_only = r#"{"role":"user","content":"<task-notification>\n<summary>Goal check-in: continuing</summary>\n</task-notification>\n后续无 system-reminder"}"#;
        assert!(scan_markers(head_only).is_empty(), "head without chain");
        let off_chain = r#"{"role":"user","content":"<task-notification>\n<summary>Goal check-in: x</summary>\n</task-notification>\n<system-reminder>\n别的话«链外引文»尾缀不接» is still active"}"#;
        assert!(
            scan_markers(off_chain).is_empty(),
            "chain without Goal-check-in tail"
        );
        // 跨值劈链：头在上一 JSON 值、链在下一值（rest[..m] 含引号），
        // 头不自中、链不被借；下一值自身完整的真标记照常命中。
        let cross_line = "{\"role\":\"user\",\"content\":\"<task-notification>\\n<summary>Goal check-in: 截断\"}\n{\"role\":\"user\",\"content\":\"<task-notification>\\n<summary>Goal check-in: ok</summary>\\n</task-notification>\\n<system-reminder>\\nGoal check-in: «真目标» is still active\"}";
        let ms = scan_markers(cross_line);
        assert_eq!(ms.len(), 1, "only the complete chain matches");
        assert_eq!(ms[0].text.as_deref(), Some("真目标"));
        // 真全链仍中（链内 «» 后紧跟尾缀）。
        let real = r#"{"role":"user","content":"<task-notification>\n<summary>Goal check-in: c</summary>\n</task-notification>\n<system-reminder>\nGoal check-in: «真目标» is still active."}"#;
        let ms = scan_markers(real);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].text.as_deref(), Some("真目标"));
    }

    #[test]
    fn ctx_pct_win_floor_and_remaining_fallback() {
        // 评审 F5：used 向下取整（46.7 出 46 非 47）；缺 used 时
        // remaining 回退 100 减其向下取整。
        let d: Json = serde_json::from_str(
            r#"{"context_window":{"context_window_size":977000,"used_percentage":46.7}}"#,
        )
        .unwrap();
        assert_eq!(ctx_pct_win(&d), Some((46, 977_000.0)));
        let d: Json = serde_json::from_str(
            r#"{"context_window":{"context_window_size":1000,"remaining_percentage":30.9}}"#,
        )
        .unwrap();
        assert_eq!(ctx_pct_win(&d), Some((70, 1000.0)), "100 - floor(30.9)");
        let d: Json =
            serde_json::from_str(r#"{"context_window":{"context_window_size":1000}}"#).unwrap();
        assert!(ctx_pct_win(&d).is_none(), "both missing hides segment");
    }

    #[test]
    fn duration_threshold_and_model_fallback() {
        // 评审 F4/F6：<1000ms 或字段缺失整段隐；model.id 回落。
        let d: Json = serde_json::from_str(r#"{"cost":{"total_duration_ms":999}}"#).unwrap();
        assert!(duration_ms(&d).is_none());
        let d: Json = serde_json::from_str(r#"{"cost":{"total_duration_ms":1000}}"#).unwrap();
        assert_eq!(duration_ms(&d), Some(1000.0));
        assert!(duration_ms(&Json::Null).is_none());
        let d: Json = serde_json::from_str(r#"{"model":{"id":"claude-x-1"}}"#).unwrap();
        assert_eq!(model_name(&d), "claude-x-1");
        let d: Json =
            serde_json::from_str(r#"{"model":{"display_name":"GLM","id":"glm-5"}}"#).unwrap();
        assert_eq!(model_name(&d), "GLM");
        assert_eq!(model_name(&Json::Null), "");
    }

    #[test]
    fn hook_state_gates_all_candidates_and_keeps_order() {
        // 评审 F2：会话闸对全部候选生效（会话键缺位时用户级 agent 键
        // 不得免闸，跨会话遗留态续找不泄漏）。
        let tmp = std::env::temp_dir().join(format!("hst-hs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let home = tmp.join("hsthome");
        std::fs::create_dir_all(home.join("state")).unwrap();
        std::fs::write(
            home.join("state").join("claude.json"),
            r#"{"state":"idle","session":"OTHER"}"#,
        )
        .unwrap();
        std::env::remove_var("HST_STATE_FILE");
        let cfg = StatuslineConfig::default();
        let d: Json = serde_json::from_str(r#"{"session_id":"s2"}"#).unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &home,
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        assert_eq!(hook_state(&ctx), ("unknown".to_string(), false));
        // 会话键命中优先于 agent 键。
        std::fs::write(
            home.join("state").join("claude-s2.json"),
            r#"{"state":"working","session":"s2"}"#,
        )
        .unwrap();
        assert_eq!(
            hook_state(&ctx),
            ("working".to_string(), true),
            "session-keyed wins"
        );
        // 无 session 字段的 agent 键不被闸（pwsh 同判：闸只看记录带
        // session 且不符）。
        let _ = std::fs::remove_file(home.join("state").join("claude-s2.json"));
        std::fs::write(
            home.join("state").join("claude.json"),
            r#"{"state":"blocked"}"#,
        )
        .unwrap();
        assert_eq!(
            hook_state(&ctx),
            ("blocked".to_string(), true),
            "record without session field is ungated"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mcp_count_falls_back_to_config_files() {
        // 评审 F9：payload 缺 mcp_servers 时回落 ~/.claude.json 加项目
        // .mcp.json 键数合计。
        let tmp = std::env::temp_dir().join(format!("hst-mcp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cj = tmp.join("claude.json");
        std::fs::write(&cj, r#"{"mcpServers":{"a":{},"b":{}}}"#).unwrap();
        let mj = tmp.join("mcp.json");
        std::fs::write(&mj, r#"{"mcpServers":{"c":{}}}"#).unwrap();
        let d: Json = serde_json::from_str(r#"{"mcp_servers":["x"]}"#).unwrap();
        // pwsh 同判：payload 在场短路用户级回落，项目 .mcp.json 无条件叠加。
        assert_eq!(
            mcp_count(&d, Some(&cj), Some(&mj)),
            2,
            "payload 1 + project 1"
        );
        let d: Json = serde_json::from_str("{}").unwrap();
        assert_eq!(mcp_count(&d, Some(&cj), Some(&mj)), 3, "user 2 + project 1");
        assert_eq!(mcp_count(&d, None, None), 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn icon_of_honors_user_override() {
        // 评审 F3：[icons] 键级覆盖优先于默认表；非 nerd 恒空。
        let cfg = StatuslineConfig {
            icons: vec![("loop".to_string(), "LOOP ".to_string())],
            ..Default::default()
        };
        let d: Json = serde_json::from_str("{}").unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: Path::new("/tmp"),
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        assert_eq!(icon_of(&ctx, "loop"), "LOOP ");
        assert_eq!(icon_of(&ctx, "hst"), "\u{f06a9}  ", "default survives");
        let ctx_ascii = Ctx {
            d: &d,
            agent: "grok".to_string(),
            nerd: false,
            cfg: &cfg,
            home: Path::new("/tmp"),
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        assert_eq!(icon_of(&ctx_ascii, "loop"), "");
    }
}
