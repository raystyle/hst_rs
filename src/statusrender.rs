//! 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
//! 读 stdin agent JSON 出状态行。REQ-038 起 PS1 载体完全淘汰，本引擎是
//! 唯一渲染载体（历史对版口径见 ADR-0010 与 REQ-026：与退役前 pwsh 脚本
//! 在 Ansi 形下逐字相同；管道缺省形态 PowerShell Host 渲染器剥 ANSI，旧
//! 载体在 agent 实际调用里无色，原生缺省带色，评审 F1）。复用单源：段序与模板配置走
//! statusline.rs 的 StatuslineConfig 加 effective_orders；loop/goal 探针走
//! loopmgmt；goalmode 与会话级 loop（ScheduleWakeup 源，REQ-036）倒序分块
//! 扫描本模块 Rust 形（流式倒扫）。性能面：无
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
            &apply_fmt(
                &tmpl_of(ctx, "dir"),
                &[("icon", icon_of(ctx, "dir")), ("path", ctx.dir.clone())],
            ),
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

/// shell 名归一：剥 `.exe` 尾（Windows 形）与首部登录杠（macOS `ps -o
/// comm=` 取 argv[0]，登录 shell 按惯例带 `-` 前缀，如 `-zsh`；Linux
/// /proc/comm 取可执行名恒无杠）。登录杠剥离后裸名形两渲染器同判；PS1
/// 匹配为锚定形，路径形 comm 属存量分歧。
fn normalize_shell_name(comm: &str) -> String {
    comm.trim_start_matches('-')
        .trim_end_matches(".exe")
        .to_string()
}

fn seg_shell(ctx: &Ctx) -> Option<String> {
    // Unix 祖先链：跳过 agent 本体，向上找最近 shell。匹配是包含语义
    // （路径形 comm 同样命中）；PS1 侧为 ^ 起始锚定，登录杠剥离后裸名
    // 形同判，路径形 comm 本侧出行而 PS1 回落 $SHELL，属存量分歧未随
    // 各批对齐。
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
        .map(|c| normalize_shell_name(&c));
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
            &[
                ("icon", icon_of(ctx, "git")),
                ("branch", branch_txt),
                ("flags", flag_txt),
            ],
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

/// hook 功能别名清单（用户令 2026-09-27 多轮收敛）：别名带属主进程前缀
/// （「别名 加上什么进程」，例序 herdr 在先），分隔符 ` | ` 与他行段分隔
/// 同形；预对齐解耦后命令名 `hst token` 与 `hst state`（REQ-028 候裁）。
/// 未收录的回落 stem 本名；清单含外来 hook（herdr 等）。
const HOOK_ALIASES: &[(&str, &str)] = &[
    // 别名带属主前缀（用户令 2026-09-27「别名 加上什么进程」，例序 herdr
    // 在先）；分隔符 ` | ` 与他行段分隔同形。REQ-028 拆条后一命令一脚本
    // 一别名：`hst hook token` 单对 hst-token.sh、`hst hook state` 单对
    // hst-state.sh；未来 `hst hook <x>` 循此式加条即入列。
    // herdr agent 状态监控 hook：claude/grok 面 SessionStart 会话登记推
    // herdr server（pane 与会话绑定），kimi 面每事件推 working/idle 态。
    ("herdr-agent-state", "herdr agent状态监控"),
    // hst token 护栏 hook（REQ-028）：PreToolUse/UserPromptSubmit 跑
    // secretguard 密钥拦截（S030：API key 命中 exit 2 阻断）。
    ("hst-token", "hst token护栏"),
    // hst 会话状态同步 hook（S025/D28）：事件映射四态写 ~/.hst/state
    // 会话键。
    ("hst-state", "hst 会话状态同步"),
];

/// 新增 hook 收录指引（评审 G3）：只改 HOOK_ALIASES 一处加 stem 判定回
/// 落即可；并确认该 agent 是否多文件注册面（grok 已知 ohmyagents-
/// state.json 与 herdr.json 双文件，注册面读全目录 *.json）。

/// hookstate 段（第 5 行，用户令 2026-09-26 起，2026-09-27 四令迭代）：
/// 注册面全部 hook 的功能别名清单独占一行（用户令「不需要working这些
/// 状态 显示hook功能的别名」：态文本退出缺省显示，仅以行色暗示；{state}
/// 占位符保留供自配）；出行门 = 状态文件在场或有挂载 hook 任一（评审
/// 快核 G1：刚装未触发的空窗期不隐清单）；双缺整行隐藏（零噪声）；
/// 注册面读不出 hook 时回落泛称 `hook` 保语义。
fn seg_hookstate(ctx: &Ctx) -> Option<String> {
    let (state, present) = hook_state(ctx);
    let alias_raw = hooked_aliases(&ctx.agent, ctx.d);
    // 出行门（评审快核 G1 裁）：状态文件在场或有挂载 hook 任一即出行——
    // 刚装未触发的空窗期（hook 已注册、state 未写）不应整行隐掉清单；
    // 状态缺报按 unknown 取行色。
    if !present && alias_raw.is_empty() {
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
    let alias = if alias_raw.is_empty() {
        "hook".to_string()
    } else {
        alias_raw
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "hookstate")),
                ("alias", alias),
                ("state", state),
            ],
        ),
        color,
    ))
}

/// 注册面 hook 别名清单：按 agent 定位注册文件，收集全部 hook 命令
///（ours 与外来都在场），stem 去重后按别名表序稳定排列（未收录 stem
/// 字典序殿后）映射别名，分隔符 ` | ` 与他行段分隔同形（用户令
/// 2026-09-27 两轮收敛）。claude 额外并读项目级注册面（REQ-039：payload
/// `workspace.project_dir` 下 `.claude/settings.json` 加
/// `settings.local.json`，项目守卫 hook 如 session-tool-guard 族入列）。
/// 文件缺失、坏损或零挂载返回空串。codex 虽无外部状态栏面，手动 render
/// 亦可得清单。
fn hooked_aliases(agent: &str, d: &Json) -> String {
    match user_home() {
        Ok(h) => hooked_aliases_at(&h, agent, &s(d, &["workspace", "project_dir"])),
        Err(_) => String::new(),
    }
}

/// hooked_aliases 的可测形（home 加项目根显式透传）。
fn hooked_aliases_at(home: &Path, agent: &str, project_root: &str) -> String {
    let mut cmds: Vec<String> = Vec::new();
    match agent {
        // claude/codex/grok 注册面同构（hooks.<Event>[].hooks[].command）。
        "claude" => {
            collect_json_commands(&home.join(".claude").join("settings.json"), &mut cmds);
            if !project_root.is_empty() {
                let pdotclaude = std::path::Path::new(project_root).join(".claude");
                collect_json_commands(&pdotclaude.join("settings.json"), &mut cmds);
                collect_json_commands(&pdotclaude.join("settings.local.json"), &mut cmds);
            }
        }
        "codex" => collect_json_commands(&home.join(".codex").join("hooks.json"), &mut cmds),
        // grok 是多文件注册面（评审 F：本机 herdr 的 grok 挂载在
        // ~/.grok/hooks/herdr.json 而非 hst 的 ohmyagents-state.json），
        // 全目录 *.json 合并收集。
        "grok" => {
            let dir = home.join(".grok").join("hooks");
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let mut files: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .is_some_and(|x| x.eq_ignore_ascii_case("json"))
                    })
                    .collect();
                files.sort();
                for f in files {
                    collect_json_commands(&f, &mut cmds);
                }
            }
        }
        // kimi 是 [[hooks]] 表项（event 加 command 平铺）。
        "kimi" => collect_toml_commands(&home.join(".kimi-code").join("config.toml"), &mut cmds),
        _ => {}
    }
    let stems: std::collections::BTreeSet<String> =
        cmds.iter().filter_map(|c| hook_stem(c)).collect();
    let mut out: Vec<&str> = HOOK_ALIASES
        .iter()
        .filter(|(stem, _)| stems.contains(*stem))
        .map(|(_, alias)| *alias)
        .collect();
    let known: Vec<&str> = HOOK_ALIASES.iter().map(|(s, _)| *s).collect();
    out.extend(
        stems
            .iter()
            .filter(|s| !known.contains(&s.as_str()))
            .map(String::as_str),
    );
    out.join(" | ")
}

/// 命令到 hook 干 stem：已知 stem 子串直配（解释器前缀与引号都拦不住）；
/// 未收录的取脚本 token 的 basename 去末个扩展（评审 G1/G2：解释器后
/// 首个含路径分隔符 token 优先——参数位脚本路径不再反客为主，无路径分隔
/// 符时回落首个非解释器 token——裸名形与 .bat/.exe/.py/.js 载体不漏列）。
fn hook_stem(cmd: &str) -> Option<String> {
    for (stem, _) in HOOK_ALIASES {
        if cmd.contains(stem) {
            return Some(stem.to_string());
        }
    }
    const INTERPRETERS: &[&str] = &[
        "bash",
        "sh",
        "zsh",
        "dash",
        "ksh",
        "pwsh",
        "powershell",
        "powershell.exe",
        "python",
        "python3",
        "node",
        "cmd",
        "cmd.exe",
        "nu",
        "fish",
        "elvish",
    ];
    let toks: Vec<&str> = cmd
        .split_whitespace()
        .map(|t| t.trim_matches(['"', '\'']))
        .collect();
    let start = if !toks.is_empty() && INTERPRETERS.contains(&toks[0].to_lowercase().as_str()) {
        1
    } else {
        0
    };
    let rest = &toks[start.min(toks.len())..];
    // 分隔符须在首字符之后（`/c`、`-File` 旗标形首字符即分隔符，不是
    // 路径）；或 token 含点（裸名加扩展形 metric-bridge.sh）。
    let sep_after_first = |t: &str| t.chars().skip(1).any(|c| c == '/' || c == '\\');
    let tok = rest
        .iter()
        .find(|t| sep_after_first(t) || t.contains('.'))?;
    let name = tok.rsplit(['/', '\\']).next().unwrap_or(tok);
    let stem = match name.rsplit_once('.') {
        Some((base, _)) => base,
        None => name,
    };
    (!stem.is_empty()).then(|| stem.to_string())
}

/// JSON 注册面（claude/codex/grok）命令收集（ours 与外来都在场）。
fn collect_json_commands(path: &Path, cmds: &mut Vec<String>) {
    let Ok(v) = crate::yolo::read_json(path) else {
        return;
    };
    let Some(obj) = v.get("hooks").and_then(|h| h.as_object()) else {
        return;
    };
    for groups in obj.values() {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for g in groups {
            if let Some(hs) = g.get("hooks").and_then(|h| h.as_array()) {
                for h in hs {
                    if let Some(c) = h.get("command").and_then(|c| c.as_str()) {
                        cmds.push(c.to_string());
                    }
                }
            }
        }
    }
}

/// TOML 注册面（kimi [[hooks]]）命令收集：ours 判定同干 stem。
fn collect_toml_commands(path: &Path, cmds: &mut Vec<String>) {
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
        if let Some(c) = item.get("command").and_then(|c| c.as_str()) {
            cmds.push(c.to_string());
        }
    }
}

/// REQ-014 哨兵（原生侧，REQ-027）：unknown 时探注册面（按 agent 定位
/// 配置文件，标记串 hst-state 干 stem），缺位升格 `no-hook!`；节流窗
///（每 agent 1 小时）到期 best-effort 自愈 `hst hook init`（幂等重注
/// 册，仅 hook 面；不在 PATH 或失败静默，不阻塞渲染）。stamp 记 ts 内容
///（pwsh 形记 mtime；内容形可测）。
fn sentinel_no_hook(ctx: &Ctx, state: &str) -> Option<String> {
    if state != "unknown" {
        return None;
    }
    let home = match user_home() {
        Ok(h) => h,
        Err(_) => return None,
    };
    let reg = match ctx.agent.as_str() {
        "claude" => home.join(".claude").join("settings.json"),
        "codex" => home.join(".codex").join("hooks.json"),
        "grok" => home
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json"),
        "kimi" => home.join(".kimi-code").join("config.toml"),
        _ => return None,
    };
    let reg_ok = std::fs::read_to_string(&reg)
        .map(|t| t.contains("hst-state"))
        .unwrap_or(false);
    if reg_ok {
        return None;
    }
    // 节流自愈：stamp（~/.hst/state/.hookcheck-<agent>）记录上次尝试 ts，
    ///窗内不重试。
    if throttle_due(&ctx.home.join("state"), ".hookcheck-", &ctx.agent, 3600).is_some() {
        heal_spawn(&["hook", "init"]);
    }
    Some("no-hook!".to_string())
}

/// REQ-017 哨兵（原生侧，REQ-027）：projyolo marker（hit 且同项目且新
/// 鲜 2 倍窗内）升格 `proj-yolo!`（实时态有价值时并显 `working/…` 形）；
/// 节流 10 分钟到期 best-effort 跑 `hst yolo check --project <dir>`（同
/// pwsh 自愈先例）。
fn sentinel_proj_yolo(ctx: &Ctx, state: &str) -> Option<String> {
    if state == "no-hook!" || ctx.dir.is_empty() {
        return None;
    }
    let slug: String = ctx
        .dir
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = ctx.home.join("state").join("projyolo");
    let marker = dir.join(format!("{slug}.json"));
    let Ok(text) = std::fs::read_to_string(&marker) else {
        // 无 marker：仍走节流探针（10 分钟窗）。
        let _ = throttle_due(&dir, ".check-", &slug, 600);
        return None;
    };
    let v: Json = serde_json::from_str(&text).ok()?;
    let hit = v.get("hit").and_then(|x| x.as_bool()).unwrap_or(false);
    let ts = v.get("ts").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let fresh = now_epoch() - ts <= 1200.0;
    let matches = v.get("project").and_then(|x| x.as_str()) == Some(ctx.dir.as_str())
        || v.get("real").and_then(|x| x.as_str()) == Some(ctx.dir.as_str());
    if throttle_due(&dir, ".check-", &slug, 600).is_some() {
        let mut args: Vec<&str> = vec!["yolo", "check", "--project"];
        let dir = ctx.dir.clone();
        let _ = &mut args;
        heal_spawn_args(&["yolo", "check", "--project"], &dir);
    }
    if !(hit && matches && fresh) {
        return None;
    }
    Some(match state {
        "working" | "blocked" | "idle" => format!("{state}/proj-yolo!"),
        _ => "proj-yolo!".to_string(),
    })
}

/// 两哨兵组合（seg_hst 消费口）。
fn sentinel_escalate(ctx: &Ctx, state: String) -> String {
    if let Some(s) = sentinel_no_hook(ctx, &state) {
        return s;
    }
    if let Some(s) = sentinel_proj_yolo(ctx, &state) {
        return s;
    }
    state
}

/// 节流 stamp：`<dir>/<prefix><name>` 记 ts 内容；窗内返回 None，到期记
/// 新 ts 返回 Some（并发双写有界，幂等自愈可接受，同 pwsh 评审 G2）。
fn throttle_due(dir: &Path, prefix: &str, name: &str, window_secs: u64) -> Option<PathBuf> {
    let stamp = dir.join(format!("{prefix}{name}"));
    let now = now_epoch() as u64;
    if let Ok(text) = std::fs::read_to_string(&stamp) {
        if let Ok(last) = text.trim().parse::<u64>() {
            if now.saturating_sub(last) < window_secs {
                return None;
            }
            let _ = std::fs::write(&stamp, now.to_string());
            return Some(stamp);
        }
    }
    if std::fs::create_dir_all(dir).is_ok() {
        if std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stamp)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(now.to_string().as_bytes())
            })
            .is_ok()
        {
            return Some(stamp);
        }
        // 首创建竞争败者：视作他人刚触发。
        let _ = std::fs::write(&stamp, now.to_string());
    }
    None
}

fn now_epoch() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// 自愈探针 spawn（防测试递归闸）：单测载荷（cfg!(test)）与
/// HST_SENTINEL_HEAL=off（集成测试与手动排障）不 spawn——单测里
/// current_exe 是测试二进制，重入即无限递归（实弹挂起抓获）。
fn heal_spawn(args: &[&str]) {
    if cfg!(test) || std::env::var_os("HST_SENTINEL_HEAL").is_some_and(|v| v == "off") {
        return;
    }
    let _ = std::process::Command::new(
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("hst")),
    )
    .args(args)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status();
}

/// heal_spawn 的带尾参形（--project <dir>）。
fn heal_spawn_args(prefix: &[&str], tail: &str) {
    if cfg!(test) || std::env::var_os("HST_SENTINEL_HEAL").is_some_and(|v| v == "off") {
        return;
    }
    let _ = std::process::Command::new(
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("hst")),
    )
    .args(prefix)
    .arg(tail)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status();
}

fn seg_hst(ctx: &Ctx) -> Option<String> {
    // 状态读序（D28）：与 hookstate 段共享 hook_state（会话键加闸全序）；
    // 版本取 payload version 字段归一（D46）。
    let (state, _present) = hook_state(ctx);
    // REQ-027（原生侧哨兵回填）：REQ-014 no-hook! 加 REQ-017 proj-yolo!，
    // 与 pwsh 载体同判（issue #31 候裁件收口）。
    let state = sentinel_escalate(ctx, state);
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
    let (mut count, mut cadence, mut goal) = loop_probe(ctx)?;
    // REQ-041：{next} 缺省 = 节拍（durable 面无单点下次时刻），会话面由
    // 回落层覆写为活倒计时（每帧走到点，行随渲染刷新）。
    let mut next = cadence.trim_start_matches('×').to_string();
    if count == 0 {
        // REQ-036 会话级回落：零 durable 任务时取 /loop 自调度
        //（ScheduleWakeup）态，loop 行补齐「本会话实际在跑的 loop」语义
        // 三层：durable 等值、durable 收养（REQ-035）、会话自调度。
        if let Some((c, g, n)) = session_loop_probe(ctx) {
            count = 1;
            cadence = c;
            goal = g;
            next = n;
        }
    }
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
                ("next", next),
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
/// 60）。项目根解析序与 PS1 同（workspace.project_dir 回落链）。归属判据
/// 双层（REQ-035 收养回落）：`createdBySessionId` 等值优先；本会话零自有
/// 任务时回落取项目文件全量。durable 任务项目作用域存活，创建会话终结后
/// 触发仍落本项目活会话而 `createdBySessionId` 不被改写（2026-09-28
/// prs_c2coe 工位重启实证：任务 15:37 仍发、创建会话 14:29 已终），
/// 单看等值会话栏误报无 loop。代价：同项目并行双会话各自都显示项目
/// loop（接受，见 REQ-035 边界节）。
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
    let mut mine: Vec<&Json> = tasks
        .iter()
        .filter(|t| {
            t.get("createdBySessionId")
                .and_then(|x| x.as_str())
                .map(|x| x == sid)
                .unwrap_or(false)
        })
        .collect();
    if mine.is_empty() {
        // REQ-035 收养回落：本会话零自有任务时取项目全量（判据见函数
        // 注）；全量也空才真零命中。
        mine = tasks.iter().collect();
    }
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
/// paused 系统事件加 clear 指令加设标形（REQ-037：2.1.270 的 /goal 以
/// queue-operation 主链 content 加 queued_command 附件 prompt 落盘，常稳
/// 运转不产 check-in 标记，缺此形新设 goal 行恒隐）；512B 跨界重叠；
/// 状态与文本独立回溯。REQ-040 补三形：TUI 斜杠路径回执（local-
/// command-stdout 载体的 Goal set 加 Goal cleared）与 goal_status 权威态
/// 附件（met:true 判终；缺 cleared 形则清 goal 后行冻在旧设标文本）。
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

/// 会话级 loop 探针（REQ-036）：`/loop` 动态自调度（ScheduleWakeup）
/// 态。源 = 会话 transcript（定位序与 goalmode 探针同：项目根 slug 加
/// session_id）倒序分块反扫（4MB 块加 512B 跨界重叠，新者先中即锁），
/// 末条工具调用取 delaySeconds 与 prompt（剥 `/loop ` 前缀截 60）。
/// 判终两形：`stop:true` 显式终；就近回取条目 timestamp，now 超 ts 加
/// 两倍心跳未续期视为弃约滞隐（timestamp 取不到不判龄，乐观显；
/// prs_c2coe 实机 4 例均按续期或 stop 闭环）。durable 任务在 seg_loop
/// 上层优先，本探针只在零 durable 时被消费。
fn session_loop_probe(ctx: &Ctx) -> Option<(String, String, String)> {
    let sid = s(ctx.d, &["session_id"]);
    if sid.is_empty() {
        return None;
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
        return None;
    }
    let mut fs = std::fs::File::open(&file).ok()?;
    use std::io::{Read, Seek, SeekFrom};
    let len = fs.metadata().ok()?.len();
    // REQ-042：源族三标记取新者——ScheduleWakeup（自调度）加 CronCreate
    //（会话级 cron，durable 缺省 false 只活会话内存，/loop <interval> 形
    // 走此道，2026-10-02 prs_c2coe fc60bb08 实证文件恒空）加 CronDelete
    //（取消判终，新于创建即隐）。
    let sw_marker = "\"name\":\"ScheduleWakeup\",\"input\":{";
    let cc_marker = "\"name\":\"CronCreate\",\"input\":{";
    let cd_marker = "\"name\":\"CronDelete\"";
    let chunk_sz: u64 = 4 * 1024 * 1024;
    let ovl: u64 = 512;
    let mut pos = len;
    let mut prev_head: Vec<u8> = Vec::new();
    while pos > 0 {
        let take = chunk_sz.min(pos);
        pos -= take;
        fs.seek(SeekFrom::Start(pos)).ok()?;
        let mut buf = vec![0u8; take as usize];
        fs.read_exact(&mut buf).ok()?;
        let mut comb = buf.clone();
        comb.extend_from_slice(&prev_head);
        prev_head = buf[..(ovl as usize).min(buf.len())].to_vec();
        let chunk = String::from_utf8_lossy(&comb);
        let sw = chunk.rfind(sw_marker);
        let cc = chunk.rfind(cc_marker);
        let cd = chunk.rfind(cd_marker);
        let newest = [sw, cc, cd]
            .into_iter()
            .flatten()
            .max_by_key(|&i| i)
            .map(|i| (i, sw == Some(i), cc == Some(i), cd == Some(i)));
        if let Some((idx, is_sw, is_cc, is_cd)) = newest {
            if is_sw {
                return wakeup_verdict(&chunk, idx).into();
            }
            if is_cd {
                return None;
            }
            // CronCreate：durable:true 属文件层（durable 探针管辖，本探针
            // 只在文件层零命中时被咨询）；会话级取 cron 加 prompt。
            return cron_verdict(&chunk, idx).into();
        }
    }
    None
}

/// 末条 ScheduleWakeup 判定两态（REQ-041，用户裁定空输入 = 取消）：
/// Terminal = stop 显式终、滞隐、或空输入退化调用（无 delaySeconds 无
/// stop——布了空定时即「不再排程」，循环即止行立隐，防取消后挂起；
/// 2026-10-01 prs_c2coe 实证）；Active = (cadence, goal, next)，next 为
/// 活倒计时（ts 加 delay 减 now，负值钳 0，timestamp 取不到回落节拍）。
enum WakeVerdict {
    Active((String, String, String)),
    Terminal,
}

impl From<WakeVerdict> for Option<(String, String, String)> {
    fn from(v: WakeVerdict) -> Self {
        match v {
            WakeVerdict::Active(t) => Some(t),
            WakeVerdict::Terminal => None,
        }
    }
}

fn wakeup_verdict(chunk: &str, marker_idx: usize) -> WakeVerdict {
    let obj = object_span(&chunk[marker_idx..]);
    if obj.contains("\"stop\":true") {
        return WakeVerdict::Terminal;
    }
    let delay: u64 = match obj
        .find("\"delaySeconds\":")
        .and_then(|i| {
            let t = &obj[i + 15..];
            let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
            u64::from_str_radix(&t[..d], 10).ok()
        })
        .filter(|d| *d > 0)
    {
        Some(d) => d,
        None => return WakeVerdict::Terminal,
    };
    let prompt = obj
        .find("\"prompt\":\"")
        .and_then(|i| json_string_at(&obj[i + 9..]));
    let ts = chunk[..marker_idx]
        .rfind("\"timestamp\":\"")
        .and_then(|i| iso_epoch(&chunk[i + 13..]));
    let mut next = delay_cadence(delay);
    if let Some(ts) = ts {
        if epoch_now() > ts.saturating_add(delay.saturating_mul(2)) {
            return WakeVerdict::Terminal;
        }
        let remain = ts.saturating_add(delay).saturating_sub(epoch_now());
        next = delay_cadence(remain);
    }
    let mut goal = prompt
        .map(|p| p.strip_prefix("/loop ").unwrap_or(&p).to_string())
        .unwrap_or_default()
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string();
    if goal.chars().count() > 60 {
        let cut: String = goal.chars().take(60).collect();
        goal = format!("{cut}…");
    }
    WakeVerdict::Active((delay_cadence(delay), goal, next))
}

/// 会话级 CronCreate 判定（REQ-042）：`durable:true` 属文件层归 None
///（本探针只在文件层零命中时被咨询）；会话级取 cron 节拍（cron_to_
/// cadence 剥 × 对齐 SW 形）加 prompt 文本，next 回落节拍（会话 cron 无
/// 单点下次时刻）。无 cron 字段的退化输入归 None。
fn cron_verdict(chunk: &str, marker_idx: usize) -> CronOutcome {
    let obj = object_span(&chunk[marker_idx..]);
    if obj.contains("\"durable\":true") {
        return CronOutcome::FileLayer;
    }
    let cron = obj
        .find("\"cron\":\"")
        .and_then(|i| json_string_at(&obj[i + 7..]))
        .unwrap_or_default();
    if cron.is_empty() {
        return CronOutcome::FileLayer;
    }
    let prompt = obj
        .find("\"prompt\":\"")
        .and_then(|i| json_string_at(&obj[i + 9..]));
    let cadence = cron_to_cadence(&cron).trim_start_matches('×').to_string();
    let mut goal = prompt
        .map(|p| p.strip_prefix("/loop ").unwrap_or(&p).to_string())
        .unwrap_or_default()
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string();
    if goal.chars().count() > 60 {
        let cut: String = goal.chars().take(60).collect();
        goal = format!("{cut}…");
    }
    CronOutcome::Session((cadence.clone(), goal, cadence))
}

enum CronOutcome {
    Session((String, String, String)),
    FileLayer,
}

impl From<CronOutcome> for Option<(String, String, String)> {
    fn from(v: CronOutcome) -> Self {
        match v {
            CronOutcome::Session(t) => Some(t),
            CronOutcome::FileLayer => None,
        }
    }
}

/// 取首个 `{` 起的配对对象跨（字符串感知：值内花括号与引号不计数）；
/// 跨界劈开或失配时宽容取到串尾（上层字段缺失兜底为零命中）。
fn object_span(t: &str) -> &str {
    let start = match t.find('{') {
        Some(i) => i,
        None => return "",
    };
    let b = t.as_bytes();
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (j, &c) in b.iter().enumerate().skip(start) {
        if esc {
            esc = false;
            continue;
        }
        if c == b'\\' && in_str {
            esc = true;
            continue;
        }
        match c {
            b'"' => in_str = !in_str,
            b'{' if !in_str => depth += 1,
            b'}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return &t[start..=j];
                }
            }
            _ => {}
        }
    }
    &t[start..]
}

/// 串内续解原语：入口已在 JSON 字符串内部（开引号已由锚消费），解转义
/// 直进到闭合引号；出（文本， 消费字节数含闭合引号）。`\u` 形含代理对
/// 拼合（孤立代理丢弃）；尾界失配（跨界劈开）宽容取到串尾。多字节字符
/// 按边界整段推进。
fn json_capture(t: &str) -> Option<(String, usize)> {
    let b = t.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i] as char;
        if c == '\\' {
            match b.get(i + 1).map(|&x| x as char) {
                Some('n') => {
                    out.push('\n');
                    i += 2;
                }
                Some('r') => {
                    out.push('\r');
                    i += 2;
                }
                Some('t') => {
                    out.push('\t');
                    i += 2;
                }
                Some('b') => {
                    out.push('\u{8}');
                    i += 2;
                }
                Some('f') => {
                    out.push('\u{c}');
                    i += 2;
                }
                Some('"') | Some('\\') | Some('/') => {
                    out.push(b[i + 1] as char);
                    i += 2;
                }
                Some('u') => {
                    let hex = t.get(i + 2..i + 6)?;
                    let code = u32::from_str_radix(hex, 16).ok()?;
                    let mut next = i + 6;
                    let mut val = code;
                    if (0xD800..=0xDBFF).contains(&code) {
                        if let (Some(bs), Some(h2)) =
                            (t.get(next..next + 2), t.get(next + 2..next + 6))
                        {
                            if bs == "\\u" {
                                if let Ok(lo) = u32::from_str_radix(h2, 16) {
                                    if (0xDC00..=0xDFFF).contains(&lo) {
                                        val = 0x10000 + ((code - 0xD800) << 10) + (lo - 0xDC00);
                                        next += 6;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(ch) = char::from_u32(val) {
                        out.push(ch);
                    }
                    i = next;
                }
                _ => i += 1,
            }
            continue;
        }
        if c == '"' {
            return Some((out, i + 1));
        }
        let mut e = i + 1;
        while e < b.len() && (b[e] & 0xC0) == 0x80 {
            e += 1;
        }
        out.push_str(&t[i..e]);
        i = e;
    }
    Some((out, t.len()))
}

/// 解一个 JSON 字符串字面量（首字符须为 `"`），出解转义内容。转义语义
/// 单源在 [`json_capture`]。
fn json_string_at(s: &str) -> Option<String> {
    if !s.starts_with('"') {
        return None;
    }
    json_capture(&s[1..]).map(|(t, _)| t)
}

/// ISO 8601 UTC 形（`2026-09-29T00:37:43.611Z`）手解为 epoch 秒（无
/// chrono 依赖；日历换算走 civil 算法）。
fn iso_epoch(ts: &str) -> Option<u64> {
    let y: i64 = ts.get(0..4)?.parse().ok()?;
    let mo: i64 = ts.get(5..7)?.parse().ok()?;
    let d: i64 = ts.get(8..10)?.parse().ok()?;
    let h: i64 = ts.get(11..13)?.parse().ok()?;
    let mi: i64 = ts.get(14..16)?.parse().ok()?;
    let se: i64 = ts.get(17..19)?.parse().ok()?;
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some((days * 86400 + h * 3600 + mi * 60 + se).max(0) as u64)
}

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 会话心跳节拍人性化：秒级 `Xs`、分级 `Xm`、时级整 `Xh` 带零头
/// `XhYm`、日级整 `Xd` 带零头 `XdYh`（durable 段的 every 同位占位符）。
fn delay_cadence(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs}s");
    }
    let m = secs / 60;
    if m < 60 {
        return format!("{m}m");
    }
    let (h, rm) = (m / 60, m % 60);
    if h < 24 {
        return if rm == 0 {
            format!("{h}h")
        } else {
            format!("{h}h{rm:02}m")
        };
    }
    let (d, rh) = (h / 24, h % 24);
    if rh == 0 {
        format!("{d}d")
    } else {
        format!("{d}d{rh:02}h")
    }
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

/// 锚命中（评审 G2 双形：紧凑与冒号后带空格两形同认）；返回命中锚长。
fn anchor_len(chunk: &str, i: usize, tight: &str, spaced: &str) -> Option<usize> {
    if chunk[i..].starts_with(tight) {
        Some(tight.len())
    } else if chunk[i..].starts_with(spaced) {
        Some(spaced.len())
    } else {
        None
    }
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
    // REQ-037 设标形：2.1.270 的 /goal 设标以排队件落盘（主链
    // queue-operation 的 content 加 queued_command 附件的 prompt，同事件
    // 多副本同文本幂等）；常稳运转的 goal 不产 check-in 标记，缺此形则
    // 新设 goal 行恒隐（2026-09-29 prs_c2coe 双会话实证）。
    let set_q = r#""content":"Goal set: "#;
    let set_a = r#""prompt":"Goal set: "#;
    let clear_q = r#""prompt":"/goal "#;
    // 评审 G2：同版本不同条目序列化器冒号后空格两形并存（020a4f1b 实证
    // queue-operation 带空格形），双形同认。
    let set_q_sp = r#""content": "Goal set: "#;
    let set_a_sp = r#""prompt": "Goal set: "#;
    let clear_q_sp = r#""prompt": "/goal "#;
    // REQ-040：TUI 斜杠路径回执（system 型 local-command-stdout 载体）与
    // goal_status 权威态附件（met:true 判终达成；met:false 活态确认不产
    // 标记）。缺 cleared 形则清 goal 后行冻在旧设标文本（2026-10-01
    // prs_c2coe 3e21eee2 实证：末态 Goal cleared 而行仍显旧文）。
    let lcs_set = r#""content":"<local-command-stdout>Goal set: "#;
    let lcs_clear = r#""content":"<local-command-stdout>Goal cleared: "#;
    let gs_met = r#""type":"goal_status","met":true"#;
    let lcs_set_sp = r#""content": "<local-command-stdout>Goal set: "#;
    let lcs_clear_sp = r#""content": "<local-command-stdout>Goal cleared: "#;
    let gs_met_sp = r#""type": "goal_status", "met": true"#;
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
        } else if let Some(p) =
            anchor_len(chunk, i, set_q, set_q_sp).or_else(|| anchor_len(chunk, i, set_a, set_a_sp))
        {
            // REQ-037：2.1.270 设标形（queue-operation 主链 content 加
            // queued_command 附件 prompt，同事件多副本幂等）；文本解转义
            // 取到闭合引号，active 态。
            if let Some((text, n)) = json_capture(&chunk[i + p..]) {
                found = Some((
                    p + n,
                    Marker {
                        kind: MarkerKind::Active,
                        text: Some(text),
                    },
                ));
            }
        } else if let Some(p) = anchor_len(chunk, i, clear_q, clear_q_sp) {
            // REQ-037：排队 clear 形（queued_command 附件 prompt 载体）。
            let rest = &chunk[i + p..];
            if rest.starts_with("clear") || rest.starts_with("off") || rest.starts_with("stop") {
                found = Some((
                    p,
                    Marker {
                        kind: MarkerKind::Clear,
                        text: None,
                    },
                ));
            }
        } else if let Some(p) = anchor_len(chunk, i, lcs_set, lcs_set_sp) {
            // REQ-040：TUI 斜杠路径设标回执（剥 local-command-stdout 尾）。
            if let Some((mut text, n)) = json_capture(&chunk[i + p..]) {
                if let Some(s) = text.strip_suffix("</local-command-stdout>") {
                    text = s.to_string();
                }
                found = Some((
                    p + n,
                    Marker {
                        kind: MarkerKind::Active,
                        text: Some(text),
                    },
                ));
            }
        } else if let Some(p) = anchor_len(chunk, i, lcs_clear, lcs_clear_sp) {
            // REQ-040：TUI 斜杠路径清 goal 回执，判终。
            found = Some((
                p,
                Marker {
                    kind: MarkerKind::Clear,
                    text: None,
                },
            ));
        } else if let Some(p) = anchor_len(chunk, i, gs_met, gs_met_sp) {
            // REQ-040：goal_status 权威态附件 met:true 判终（达成即无在役
            // goal，行隐）。
            found = Some((
                p,
                Marker {
                    kind: MarkerKind::Clear,
                    text: None,
                },
            ));
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
    fn hooked_aliases_list_known_foreign_and_fallback() {
        // 用户令「组织一个hook别名清单」：已知 stem 映射功能别名（表序
        // 稳定）、外来 hook（herdr）同列、未收录 stem 回落本名（字典序
        // 殿后）、kimi TOML 面、坏损与缺文件零命中。
        let tmp = std::env::temp_dir().join(format!("hst-hal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let claude_dir = tmp.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"\"/x/.hst/hooks/hst-state.sh\" claude"}]}],"SessionStart":[{"matcher":"*","hooks":[{"type":"command","command":"bash '/other/herdr-agent-state.sh' session"},{"type":"command","command":"\"/x/.hst/hooks/hst-state.sh\" claude"}]}],"Notification":[{"matcher":"*","hooks":[{"type":"command","command":"bash /x/.hst/hooks/hst-token.sh claude"}]}]}}"#,
        )
        .unwrap();
        // 别名表序（herdr 在先，用户例序），未收录 metric-bridge 回落本名殿后。
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", ""),
            "herdr agent状态监控 | hst token护栏 | hst 会话状态同步"
        );
        // kimi TOML 面：ours 加外来同列。
        let kimi_dir = tmp.join(".kimi-code");
        std::fs::create_dir_all(&kimi_dir).unwrap();
        std::fs::write(
            kimi_dir.join("config.toml"),
            "[[hooks]]\nevent = \"PreToolUse\"\ncommand = \"/x/.hst/hooks/hst-state.sh kimi\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"/y/herdr-agent-state.sh\"\n",
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "kimi", ""),
            "herdr agent状态监控 | hst 会话状态同步"
        );
        // grok 多文件注册面（评审 F）：hst 的 ohmyagents-state.json 与
        // herdr 的 herdr.json 双文件合并收集。
        let grok_dir = tmp.join(".grok").join("hooks");
        std::fs::create_dir_all(&grok_dir).unwrap();
        std::fs::write(
            grok_dir.join("ohmyagents-state.json"),
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"/x/hst-state.sh grok"}]}]}}"#,
        )
        .unwrap();
        std::fs::write(
            grok_dir.join("herdr.json"),
            r#"{"hooks":{"SessionStart":[{"matcher":"*","hooks":[{"type":"command","command":"/z/herdr-agent-state.sh session"}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "grok", ""),
            "herdr agent状态监控 | hst 会话状态同步"
        );
        // REQ-039：claude 项目级注册面并入（payload project_dir 下
        // .claude/settings.json 加 settings.local.json），未收录 stem
        //（项目守卫）字典序殿后；与用户级同 stem 去重。
        let proj = tmp.join("proj");
        let pdot = proj.join(".claude");
        std::fs::create_dir_all(&pdot).unwrap();
        std::fs::write(
            pdot.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-tool-guard.sh\""}]}],"Stop":[{"hooks":[{"type":"command","command":"/x/.hst/hooks/hst-state.sh claude"}]}]}}"#,
        )
        .unwrap();
        std::fs::write(
            pdot.join("settings.local.json"),
            r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/knowledge-recall.sh\""}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", proj.to_str().unwrap()),
            "herdr agent状态监控 | hst token护栏 | hst 会话状态同步 | knowledge-recall | session-tool-guard"
        );
        // 坏损 JSON 零命中不炸。
        std::fs::write(claude_dir.join("settings.json"), "{ not json").unwrap();
        assert_eq!(hooked_aliases_at(&tmp, "claude", ""), "");
        // 缺文件零命中空串（段内回落泛称 hook）。
        std::fs::remove_file(claude_dir.join("settings.json")).unwrap();
        assert_eq!(hooked_aliases_at(&tmp, "claude", ""), "");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn hook_stem_interpreter_then_path_heuristic() {
        // 评审 G1/G2：解释器后首个含路径分隔符 token 优先（参数位脚本
        // 路径不反客为主）；无路径分隔符回落首个带点 token；扩展白名单
        // 外载体（.bat/.exe/.py/.js）取 basename 去末扩展不漏列。
        assert_eq!(
            hook_stem("bash /x/foo.sh /y/bar.sh").as_deref(),
            Some("foo")
        );
        assert_eq!(hook_stem("/x/foo.sh --flag").as_deref(), Some("foo"));
        assert_eq!(
            hook_stem("node /x/hook.js").as_deref(),
            Some("hook"),
            "js carrier"
        );
        assert_eq!(
            hook_stem("cmd /c C:\\tools\\guard.bat").as_deref(),
            Some("guard"),
            "bat carrier with backslash path"
        );
        assert_eq!(
            hook_stem("metric-bridge.sh --on").as_deref(),
            Some("metric-bridge")
        );
        // 已知 stem 子串直配不受启发式影响（引号与解释器前缀都拦不住）。
        assert_eq!(
            hook_stem("bash '/opt/herdr/herdr-agent-state.sh' session").as_deref(),
            Some("herdr-agent-state")
        );
        // 纯裸命令无可判 token：不出（清单不编造）。
        assert!(hook_stem("echo hi").is_none());
    }

    #[test]
    fn sentinel_no_hook_faces() {
        // REQ-027 原生侧：unknown 且注册面缺 hst-state 标记 → no-hook!；
        ///注册在场不升格；非 unknown 不升格；stamp 节流窗内不重触发。
        let tmp = std::env::temp_dir().join(format!("hst-sent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let hst_home = tmp.join("hst");
        let user = tmp.join("user");
        std::fs::create_dir_all(hst_home.join("state")).unwrap();
        std::fs::create_dir_all(user.join(".claude")).unwrap();
        let cfg = StatuslineConfig::default();
        let d: Json = serde_json::from_str("{}").unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &hst_home,
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        // 注册面缺位：unknown 升格（user_home 经 HOME 钉 user——本测改用
        // 直读路径面：sentinel_no_hook 经 user_home()；钉 HOME 走 ENV 锁）。
        let _env = crate::pathutil::ENV_LOCK.lock().unwrap();
        std::env::set_var("HOME", &user);
        assert_eq!(
            sentinel_no_hook(&ctx, "unknown").as_deref(),
            Some("no-hook!")
        );
        // stamp 已写：窗内二次调用不升格路径重复自愈（升格仍出，探针节流
        // 由 stamp 内容窗保证，这里只锁 stamp 在场与内容形）。
        let stamp = hst_home.join("state").join(".hookcheck-claude");
        assert!(stamp.exists(), "throttle stamp written");
        // 注册面在场：不升格。
        std::fs::write(
            user.join(".claude").join("settings.json"),
            "\"command\": \"/x/.hst/hooks/hst-state.sh\"",
        )
        .unwrap();
        assert_eq!(sentinel_no_hook(&ctx, "unknown"), None);
        // 非 unknown：不升格。
        assert_eq!(sentinel_no_hook(&ctx, "working"), None);
        std::env::remove_var("HOME");
        drop(_env);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sentinel_proj_yolo_faces() {
        // REQ-027 原生侧：marker 命中同项目且新鲜升格（复合态并显）、项目
        ///不符与过期不升格、no-hook! 优先不叠加。
        let tmp = std::env::temp_dir().join(format!("hst-py-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let hst_home = tmp.join("hst");
        let cfg = StatuslineConfig::default();
        let d: Json = serde_json::from_str("{}").unwrap();
        let dir = "/proj/x";
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &hst_home,
            dir: dir.to_string(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        let mdir = hst_home.join("state").join("projyolo");
        std::fs::create_dir_all(&mdir).unwrap();
        let ts = now_epoch();
        // 命中：复合态并显。
        std::fs::write(
            mdir.join("-proj-x.json"),
            format!("{{\"hit\":true,\"project\":\"{dir}\",\"ts\":{ts}}}"),
        )
        .unwrap();
        assert_eq!(
            sentinel_proj_yolo(&ctx, "working").as_deref(),
            Some("working/proj-yolo!")
        );
        assert_eq!(
            sentinel_proj_yolo(&ctx, "unknown").as_deref(),
            Some("proj-yolo!")
        );
        // 项目不符：不升格。
        std::fs::write(
            mdir.join("-proj-x.json"),
            format!("{{\"hit\":true,\"project\":\"/elsewhere\",\"ts\":{ts}}}"),
        )
        .unwrap();
        assert_eq!(sentinel_proj_yolo(&ctx, "working"), None);
        // real 字段命中（评审 G1 双等值）。
        std::fs::write(
            mdir.join("-proj-x.json"),
            format!("{{\"hit\":true,\"project\":\"/elsewhere\",\"real\":\"{dir}\",\"ts\":{ts}}}"),
        )
        .unwrap();
        assert_eq!(
            sentinel_proj_yolo(&ctx, "idle").as_deref(),
            Some("idle/proj-yolo!")
        );
        // 过期（2 倍窗外）：不升格。
        std::fs::write(
            mdir.join("-proj-x.json"),
            format!(
                "{{\"hit\":true,\"project\":\"{dir}\",\"ts\":{}}}",
                ts - 1300.0
            ),
        )
        .unwrap();
        assert_eq!(sentinel_proj_yolo(&ctx, "working"), None);
        // no-hook! 优先：不叠加。
        assert_eq!(sentinel_proj_yolo(&ctx, "no-hook!"), None);
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
    fn loop_probe_adoption_fallback_prefers_own() {
        // REQ-035 收养回落：工位重启后创建会话已终而任务仍项目级发着
        //（2026-09-28 prs_c2coe 实证），零自有任务时显示面回落全量；
        // 自有任务在场时优先且不混入他话任务。
        let tmp = std::env::temp_dir().join(format!("hst-loop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".claude")).unwrap();
        // 收养面可见性：他话任务 createdAt 更新，回落时 newest 取它。
        std::fs::write(
            tmp.join(".claude").join("scheduled_tasks.json"),
            r#"{"tasks":[
                {"id":"own","cron":"*/15 * * * *","prompt":"本话盯发布","createdAt":200,"createdBySessionId":"s2"},
                {"id":"orphan","cron":"*/30 * * * *","prompt":"他话巡检","createdAt":300,"createdBySessionId":"dead"}
            ]}"#,
        )
        .unwrap();
        let cfg = StatuslineConfig::default();
        let cwd = tmp.display().to_string().replace('\\', "/");
        fn ctx_of<'a>(d: &'a Json, cfg: &'a StatuslineConfig, home: &'a Path) -> Ctx<'a> {
            Ctx {
                d,
                agent: "claude".to_string(),
                nerd: true,
                cfg,
                home,
                dir: String::new(),
                root: String::new(),
                proj_kind: String::new(),
                pkg_ver: String::new(),
            }
        }
        let d_of = |sid: &str| {
            serde_json::from_str::<Json>(&format!(r#"{{"session_id":"{sid}","cwd":"{cwd}"}}"#))
                .unwrap()
        };
        // 自有优先：只算本话任务，不混入 createdAt 更新的他话任务。
        let d2 = d_of("s2");
        assert_eq!(
            loop_probe(&ctx_of(&d2, &cfg, &tmp)).unwrap(),
            (1, "×15m".to_string(), "本话盯发布".to_string())
        );
        // 零自有回落：收养项目全量，newest 按 createdAt 取他话任务。
        let d3 = d_of("s3");
        assert_eq!(
            loop_probe(&ctx_of(&d3, &cfg, &tmp)).unwrap(),
            (2, "×30m".to_string(), "他话巡检".to_string())
        );
        // 全量也空才真零命中（行隐藏判据不变）。
        std::fs::write(
            tmp.join(".claude").join("scheduled_tasks.json"),
            r#"{"tasks":[]}"#,
        )
        .unwrap();
        let d4 = d_of("s3");
        assert_eq!(
            loop_probe(&ctx_of(&d4, &cfg, &tmp)).unwrap(),
            (0, String::new(), String::new())
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn normalize_shell_name_strips_login_dash_and_exe() {
        // macOS `ps -o comm=` 取 argv[0]，登录 shell 带登录杠前缀（-zsh
        // 原样上屏是 2026-09-28 mac 工位实弹缺陷）；.exe 剥除是既有行为。
        assert_eq!(normalize_shell_name("-zsh"), "zsh");
        assert_eq!(normalize_shell_name("zsh"), "zsh");
        assert_eq!(normalize_shell_name("-bash"), "bash");
        assert_eq!(normalize_shell_name("pwsh.exe"), "pwsh");
        assert_eq!(normalize_shell_name("-pwsh.exe"), "pwsh");
        // 全剥前导杠语义（评审 G1）：复数杠直通剥净；全杠归一为空但
        // 选入前须 contains 命中 shell token，空名不可达不上屏。
        assert_eq!(normalize_shell_name("--zsh"), "zsh");
        assert_eq!(normalize_shell_name("-"), "");
    }

    #[test]
    fn scan_markers_goal_set_forms() {
        // REQ-037：2.1.270 设标形入词表（queue-operation 主链 content 加
        // queued_command 附件 prompt），取 active 态加解转义文本；深层
        // 引文形不中；排队 clear 形判清态。
        let q = r#"{"type":"queue-operation","content":"Goal set: 完成所有题目 解题过程"}"#;
        let ms = scan_markers(q);
        assert_eq!(ms.len(), 1);
        assert!(matches!(ms[0].kind, MarkerKind::Active));
        assert_eq!(ms[0].text.as_deref(), Some("完成所有题目 解题过程"));
        let a = r#"{"type":"attachment","attachment":{"type":"queued_command","prompt":"Goal set: \"引号\"与\n换行"},"rendered":[]}"#;
        let ms = scan_markers(a);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].text.as_deref(), Some("\"引号\"与\n换行"));
        // 字符串值内嵌的引文形（再编码）不中。
        let quoted = r#"{"type":"user","content":"[{\"prompt\":\"Goal set: 假标\"}]"}"#;
        assert!(scan_markers(quoted).is_empty());
        // 排队 clear 形。
        let c = r#"{"type":"attachment","attachment":{"type":"queued_command","prompt":"/goal clear"}}"#;
        let ms = scan_markers(c);
        assert_eq!(ms.len(), 1);
        assert!(matches!(ms[0].kind, MarkerKind::Clear));
        // 评审 G2：冒号后带空格的序列化形同认（三载体全测）。
        let sp = r#"{"type": "queue-operation", "content": "Goal set: 带空格形"}"#;
        let ms = scan_markers(sp);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].text.as_deref(), Some("带空格形"));
        let sp_a = r#"{"type": "attachment", "attachment": {"type": "queued_command", "prompt": "Goal set: 附件空格形"}, "rendered": []}"#;
        let ms = scan_markers(sp_a);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].text.as_deref(), Some("附件空格形"));
        let sp_c = r#"{"type": "attachment", "attachment": {"type": "queued_command", "prompt": "/goal clear"}}"#;
        assert!(matches!(scan_markers(sp_c)[0].kind, MarkerKind::Clear));
    }

    #[test]
    fn goalmode_probe_terminal_forms_hide_row() {
        // REQ-040 评审 G3：probe 级终态 e2e——设标后末条 cleared 或
        // goal_status met:true,整行隐（用户可见判据上夹具）。
        let _env = crate::pathutil::ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("hst-gt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let proj = format!("{}/proj", tmp.display());
        let slug: String = proj
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let tdir = tmp.join(".claude").join("projects").join(&slug);
        std::fs::create_dir_all(&tdir).unwrap();
        std::env::set_var("HST_USER_HOME", &tmp);
        let cfg = StatuslineConfig::default();
        let d: Json =
            serde_json::from_str(&format!(r#"{{"session_id":"s1","cwd":"{proj}"}}"#)).unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &tmp,
            dir: proj.clone(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        let set_line = r#"{"type":"queue-operation","content":"Goal set: 目标文本"}"#;
        for tail in [
            r#"{"type":"system","content":"<local-command-stdout>Goal cleared: 目标文本</local-command-stdout>"}"#,
            r#"{"type":"attachment","attachment":{"type":"goal_status","met":true,"sentinel":true,"condition":"目标文本"}}"#,
        ] {
            std::fs::write(tdir.join("s1.jsonl"), format!("{set_line}\n{tail}\n")).unwrap();
            assert_eq!(
                goalmode_probe(&ctx),
                Some((String::new(), String::new())),
                "terminal tail hides row: {tail}"
            );
        }
        std::env::remove_var("HST_USER_HOME");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn session_cron_probe_reads_croncreate_family() {
        // REQ-042：/loop <interval> 形走会话级 CronCreate（durable 缺省
        // false 只活会话内存，文件恒空）；取 cron 节拍加 prompt；CronDelete
        // 新于创建判取消；durable:true 属文件层归 None。
        let _env = crate::pathutil::ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("hst-sc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let proj = format!("{}/proj", tmp.display());
        let slug: String = proj
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let tdir = tmp.join(".claude").join("projects").join(&slug);
        std::fs::create_dir_all(&tdir).unwrap();
        std::env::set_var("HST_USER_HOME", &tmp);
        let cfg = StatuslineConfig::default();
        let d: Json =
            serde_json::from_str(&format!(r#"{{"session_id":"s1","cwd":"{proj}"}}"#)).unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &tmp,
            dir: proj.clone(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        let cc = r#"{"timestamp":"2026-10-02T08:55:00.000Z","message":{"content":[{"type":"tool_use","name":"CronCreate","input":{"cron":"*/30 * * * *","prompt":"继续 按进度完成goal","recurring":true}}]}}"#;
        let cd = r#"{"timestamp":"2026-10-02T09:00:00.000Z","message":{"content":[{"type":"tool_use","name":"CronDelete","input":{"jobId":"fc60bb08"}}]}}"#;
        let cc_durable = r#"{"timestamp":"2026-10-02T09:05:00.000Z","message":{"content":[{"type":"tool_use","name":"CronCreate","input":{"cron":"*/5 * * * *","prompt":"durable 件","recurring":true,"durable":true}}]}}"#;
        // 会话级 CronCreate → 30m 节拍加 prompt（无 timestamp 依赖，next 回
        // 落节拍）。
        std::fs::write(tdir.join("s1.jsonl"), format!("{cc}\n")).unwrap();
        let probe = session_loop_probe(&ctx).unwrap();
        assert_eq!(
            probe,
            (
                "30m".to_string(),
                "继续 按进度完成goal".to_string(),
                "30m".to_string()
            )
        );
        // CronDelete 新于创建 → 取消判终。
        std::fs::write(tdir.join("s1.jsonl"), format!("{cc}\n{cd}\n")).unwrap();
        assert!(session_loop_probe(&ctx).is_none());
        // durable:true 属文件层（文件层零命中时被咨询 → None）。
        std::fs::write(tdir.join("s1.jsonl"), format!("{cc}\n{cc_durable}\n")).unwrap();
        assert!(session_loop_probe(&ctx).is_none());
        std::env::remove_var("HST_USER_HOME");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_markers_req040_terminal_forms() {
        // REQ-040：TUI 斜杠路径回执（Goal set 剥 local-command-stdout 尾取
        // 文本；Goal cleared 判终）与 goal_status 权威态附件（met:true 判
        // 终；met:false 不产标记）；set 后 cleared 的时序终态。
        let s = r#"{"type":"system","content":"<local-command-stdout>Goal set: 斜杠路径设标</local-command-stdout>"}"#;
        let ms = scan_markers(s);
        assert_eq!(ms.len(), 1);
        assert!(matches!(ms[0].kind, MarkerKind::Active));
        assert_eq!(ms[0].text.as_deref(), Some("斜杠路径设标"));
        let c = r#"{"type":"system","content":"<local-command-stdout>Goal cleared: 斜杠路径清除</local-command-stdout>"}"#;
        let ms = scan_markers(c);
        assert_eq!(ms.len(), 1);
        assert!(matches!(ms[0].kind, MarkerKind::Clear));
        let met = r#"{"type":"attachment","attachment":{"type":"goal_status","met":true,"sentinel":true,"condition":"达成条件"}}"#;
        assert!(matches!(scan_markers(met)[0].kind, MarkerKind::Clear));
        // met:false 是活态确认，不产标记（不干扰回溯的 set 文本）。
        let unmet = r#"{"type":"attachment","attachment":{"type":"goal_status","met":false,"condition":"未达成"}}"#;
        assert!(scan_markers(unmet).is_empty());
        // 时序：set 在前 cleared 在后 → 扫描器两标记都在，回溯态取新者
        //（cleared）。
        let both = r#"{"type":"queue-operation","content":"Goal set: 旧目标"}
{"type":"system","content":"<local-command-stdout>Goal cleared: 旧目标</local-command-stdout>"}
"#;
        let ms = scan_markers(both);
        assert_eq!(ms.len(), 2);
        assert!(matches!(ms[ms.len() - 1].kind, MarkerKind::Clear));
    }

    #[test]
    fn goalmode_probe_reads_goal_set_form() {
        // REQ-037 端到端：仅设标形（无常稳运转不产的 check-in 标记）也
        // 出 active 态行。夹具钉 HST_USER_HOME。
        let _env = crate::pathutil::ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("hst-gs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let proj = format!("{}/proj", tmp.display());
        let slug: String = proj
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let tdir = tmp.join(".claude").join("projects").join(&slug);
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(
            tdir.join("s1.jsonl"),
            r#"{"x":1}
{"type":"queue-operation","content":"Goal set: 完成所有题目 解题过程： browse交互逻辑分析"}
"#,
        )
        .unwrap();
        std::env::set_var("HST_USER_HOME", &tmp);
        let cfg = StatuslineConfig::default();
        let d: Json =
            serde_json::from_str(&format!(r#"{{"session_id":"s1","cwd":"{proj}"}}"#)).unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &tmp,
            dir: proj.clone(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        assert_eq!(
            goalmode_probe(&ctx),
            Some((
                "active".to_string(),
                "完成所有题目 解题过程： browse交互逻辑分析".to_string()
            ))
        );
        std::env::remove_var("HST_USER_HOME");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn session_loop_probe_verdicts_four_forms() {
        // REQ-036：末条心跳取胜（goal 剥 /loop 前缀）、stop:true 判终、
        // 两心跳未续期滞隐、durable 在场时会话不混入（seg_loop 集成）。
        let _env = crate::pathutil::ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("hst-sw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let proj = format!("{}/proj", tmp.display());
        let slug: String = proj
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let tdir = tmp.join(".claude").join("projects").join(&slug);
        std::fs::create_dir_all(&tdir).unwrap();
        let cfg = StatuslineConfig::default();
        let d: Json =
            serde_json::from_str(&format!(r#"{{"session_id":"s1","cwd":"{proj}"}}"#)).unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "claude".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &tmp,
            dir: proj.clone(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
        };
        std::env::set_var("HST_USER_HOME", &tmp);
        let sw = |delay: u64, prompt: &str, extra: &str, ts: &str| {
            format!(
                r#"{{"timestamp":"{ts}","message":{{"content":[{{"type":"tool_use","name":"ScheduleWakeup","input":{{"delaySeconds":{delay},"prompt":"{prompt}","reason":"r","noop":false{extra}}}}}]}}"}}"#,
            )
        };
        // 末条胜 + 前缀剥：ts 老、delay 30d → 滞隐窗外活态。
        std::fs::write(
            tdir.join("s1.jsonl"),
            format!(
                "{}\n{}\n",
                sw(600, "/loop 旧心跳", "", "2026-09-01T00:00:00.000Z"),
                sw(
                    2592000,
                    "/loop 确认解题后沉淀了步骤",
                    "",
                    "2026-09-01T00:30:00.000Z"
                ),
            ),
        )
        .unwrap();
        // REQ-041 起 probe 出三元组（cadence 加 goal 加 next 活倒计
        // 时），倒计时随时钟走不钉墙钟值，钉 cadence 加 goal。
        let probe = session_loop_probe(&ctx).unwrap();
        assert_eq!(probe.0, "30d");
        assert_eq!(probe.1, "确认解题后沉淀了步骤");
        // seg_loop 集成：零 durable + 会话活 → 行出（{next} 缺省模板）。
        let row = seg_loop(&ctx).unwrap();
        assert!(row.contains("/ 确认解题后沉淀了步骤"), "session row: {row}");
        // stop:true 判终。
        std::fs::write(
            tdir.join("s1.jsonl"),
            sw(
                2592000,
                "/loop 终态",
                ",\"stop\":true",
                "2026-09-01T00:30:00.000Z",
            ),
        )
        .unwrap();
        assert_eq!(session_loop_probe(&ctx), None);
        // 两心跳未续期滞隐（ts 2026-09-01 + 60s）。
        std::fs::write(
            tdir.join("s1.jsonl"),
            sw(60, "/loop 短心跳", "", "2026-09-01T00:30:00.000Z"),
        )
        .unwrap();
        assert_eq!(session_loop_probe(&ctx), None);
        // durable 在场：会话不混入，行只出 durable。
        let sdir = tmp.join("proj").join(".claude");
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(
            tdir.join("s1.jsonl"),
            sw(
                2592000,
                "/loop 会话心跳在场",
                "",
                "2026-09-01T00:30:00.000Z",
            ),
        )
        .unwrap();
        std::fs::write(
            sdir.join("scheduled_tasks.json"),
            r#"{"tasks":[{"id":"d1","cron":"*/15 * * * *","prompt":"durable 盯发布","createdAt":200,"createdBySessionId":"s1"}]}"#,
        )
        .unwrap();
        let row = seg_loop(&ctx).unwrap();
        assert!(row.contains("15m / durable 盯发布"), "durable wins: {row}");
        assert!(!row.contains("会话心跳在场"), "session not mixed: {row}");
        // REQ-041（用户裁定）：空输入 SW = 取消——即使更早布防仍在滞隐
        // 窗内，行也立隐（防取消后挂起）；stop 判终同收口。
        std::fs::remove_file(sdir.join("scheduled_tasks.json")).unwrap();
        let empty_sw = r#"{"timestamp":"2026-10-01T08:16:10.402Z","message":{"content":[{"type":"tool_use","name":"ScheduleWakeup","input":{}}]}}"#;
        std::fs::write(
            tdir.join("s1.jsonl"),
            format!(
                "{}
{}
",
                sw(5184000, "/loop 长心跳在途", "", "2026-09-25T00:00:00.000Z"),
                empty_sw,
            ),
        )
        .unwrap();
        assert!(
            session_loop_probe(&ctx).is_none(),
            "empty SW cancels even within staleness window"
        );
        std::fs::write(
            tdir.join("s1.jsonl"),
            format!(
                "{}
{}
",
                sw(
                    5184000,
                    "/loop 已停",
                    ",\"stop\":true",
                    "2026-09-25T00:00:00.000Z"
                ),
                empty_sw,
            ),
        )
        .unwrap();
        assert!(session_loop_probe(&ctx).is_none(), "stop terminal wins");
        std::env::remove_var("HST_USER_HOME");
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
