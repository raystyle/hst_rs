//! 状态栏原生渲染引擎（ADR-0010、REQ-026）：`hst statusline render <agent>`
//! 读 stdin agent JSON 出状态行。REQ-038 起 PS1 载体完全淘汰，本引擎是
//! 唯一渲染载体（历史对版口径见 ADR-0010 与 REQ-026：与退役前 pwsh 脚本
//! 在 Ansi 形下逐字相同；管道缺省形态 PowerShell Host 渲染器剥 ANSI，旧
//! 载体在 agent 实际调用里无色，原生缺省带色，评审 F1）。复用单源：段序与模板配置走
//! statusline.rs 的 StatuslineConfig 加 effective_orders；goal 段走 goal
//! 自有存储（REQ-045，用户令 2026-10-11「hst 不再关联 goal 和 loop」，
//! loop 段随 loop 面退役出仓）；goalmode（/goal 原生态）倒序分块扫描本
//! 模块 Rust 形（流式倒扫）。性能面：无
//! pwsh 冷启动（约 300ms）加流式倒扫（105MB transcript 毫秒级）。ANSI 退
//! 裸文本开关：`NO_COLOR` 或 `HST_STATUSLINE_NO_ANSI` 任一非空（评审 F1）。
//! 已知边界：tools 段（显式选用面）首版渲染为空、版本本地探测缓存面
//!（D46）未移植（payload version 字段归一承载）、REQ-014/REQ-017 哨兵面
//! 未移植，见 REQ-026。

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::pathutil::user_home;
// 两组拆行避 rustfmt 双版互斥(仓门禁 edition 2021 与 evo-lint 静检
// edition 2024 对大小写混排排序互斥,单组混排必有一版红,豁免在案)。
use crate::statusline::StatuslineConfig;
use crate::statusline::{effective_orders_pub, read_config_pub};

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
    /// 窗宽（REQ-051 退化表输入）：env COLUMNS 检测，缺省 120 按满窗。
    width: usize,
}

fn render_rows(
    d: &Json,
    agent: &str,
    cfg: &StatuslineConfig,
    rows: &[&[&str]],
    home: &Path,
) -> Result<String, String> {
    // REQ-051(grok 评审):grok 1.0.50 支持多行与图标,旧剥离约束退役;
    // 图标与分隔符按总台令第四笔全量保留。
    let nerd = true;
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
    // REQ-051 窗宽检测:agent TUI 传 COLUMNS 则退化,缺省按满窗 120。
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(120);
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
        width,
    };
    // kimi 恒单行(REQ-051 四家承接):固定形一行,不走段行组装。
    if agent == "kimi" {
        return Ok(render_kimi_line(&ctx));
    }
    // REQ-051 退化表 40 列档:只保 L1(agent 加 dir)与 L2 的 context。
    let fallback_rows: [&[&str]; 2] = [&["agent", "dir"], &["context"]];
    let rows: &[&[&str]] = if width < 40 { &fallback_rows } else { rows };
    let mut out_rows: Vec<String> = Vec::new();
    for row in rows {
        let mut parts: Vec<String> = Vec::new();
        for id in *row {
            // REQ-051 退化表右起逐段弃:clock 小于 100 弃、fleet 小于 80 弃、
            // git 与 model 小于 60 弃。
            if (ctx.width < 100 && *id == "clock")
                || (ctx.width < 80 && *id == "fleet")
                || (ctx.width < 60 && (*id == "git" || *id == "model"))
            {
                continue;
            }
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
    // kimi 恒单行(2.1.1 只取首行)已在上方专用道;single_line 显式退单行保留。
    let joined = if cfg.single_line {
        out_rows.join(" | ")
    } else {
        out_rows.join("\n")
    };
    Ok(format!("{joined}\n"))
}

/// kimi 恒单行(REQ-051 四家承接):2.1.1 只取 stdout 首行(300ms 超时杀,
/// 非 0 退整行回落内置 footer),固定形一行 = 版本加目录 basename 加分支
/// (直读 payload gitBranch,可 null 藏)加模型加 context(pct% used/window
/// 无条形)加 task 紧凑形(R3·18m)加 hook 首 id;fleet 与 clock 与 duration
/// 不上;零子进程(分支不起 git,目录不探测)。
fn render_kimi_line(ctx: &Ctx) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = seg_agent(ctx) {
        parts.push(p);
    }
    let base = dir_basename(&ctx.dir);
    if !base.is_empty() {
        parts.push(ansi(&base, "38;5;39"));
    }
    let br = s(ctx.d, &["gitBranch"]);
    if !br.is_empty() {
        parts.push(ansi(&br, "38;5;176"));
    }
    let m = model_name(ctx.d);
    if !m.is_empty() {
        parts.push(ansi(
            &apply_fmt(
                &tmpl_of(ctx, "model"),
                &[("icon", icon_of(ctx, "model")), ("model", m)],
            ),
            "38;5;147",
        ));
    }
    if let Some((pct, used, win)) = context_kimi(ctx.d) {
        parts.push(ansi(
            &format!("{pct}% {}/{}", fmt_tok(used), fmt_tok(win)),
            "38;5;116",
        ));
    }
    if let Some(rec) = task_record(ctx) {
        let last = if rec.last_stop_at > 0 {
            rec.last_stop_at
        } else {
            rec.set_at
        };
        parts.push(ansi(
            &format!(
                "R{}·{}",
                rec.rounds,
                fmt_age(now_unix().saturating_sub(last))
            ),
            "38;5;179",
        ));
    }
    if let Some(first) = hooked_aliases(&ctx.agent, ctx.d).first() {
        parts.push(ansi(first, "38;5;143"));
    }
    format!("{}\n", parts.join(" | "))
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
        "agent",
        "fleet",
        "shell",
        "dir",
        "hst",
        "model",
        "context",
        "tools",
        "mcp",
        "tokens",
        "duration",
        "goal",
        "task",
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
        "agent" => seg_agent(ctx),
        "fleet" => seg_fleet(ctx),
        "shell" => seg_shell(ctx),
        "dir" => {
            // REQ-051 退化表:小于 80 列取 basename(双分隔符兼容 Windows 路径)。
            let path = if ctx.width < 80 {
                dir_basename(&ctx.dir)
            } else {
                ctx.dir.clone()
            };
            Some(ansi(
                &apply_fmt(
                    &tmpl_of(ctx, "dir"),
                    &[("icon", icon_of(ctx, "dir")), ("path", path)],
                ),
                "38;5;39",
            ))
            .filter(|x| !x.is_empty())
        }
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
        "goal" | "task" => seg_goal(ctx),
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
        // 全量评审 G5：链上任一环读取失败（进程退出竞态）break 走
        // `$SHELL` 回落，装饰段不因瞬时竞态整段消失。
        let mut cur = std::process::id();
        for _ in 0..8 {
            let stat = match std::fs::read_to_string(format!("/proc/{cur}/stat")) {
                Ok(s) => s,
                Err(_) => break,
            };
            let ppid: u32 = match stat
                .rsplit(')')
                .next()
                .and_then(|s| s.split_whitespace().nth(1))
                .and_then(|t| t.parse().ok())
            {
                Some(p) => p,
                None => break,
            };
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
        let txt = match std::process::Command::new("ps")
            .args(["-o", "pid=,ppid=,comm=", "-ax"])
            .output()
        {
            // G5：ps 失败走空链，回落 $SHELL 兜底（不整段消失）。
            Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
            Err(_) => String::new(),
        };
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
/// 会话态档新鲜度闸(hst-rev F-3 裁定:态显示随 state 腿退役):写入者已
/// 退役(SessionEnd GC 同亡),陈旧档超此窗按缺位收,防旧机升级后僵态永显;
/// 升级初窗(48h)内旧档仍显示,渐进入 unknown 稳态。
pub const STATE_STALE_SECS: u64 = 48 * 3600;

fn state_fresh(p: &Path) -> bool {
    p.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .is_some_and(|d| d.as_secs() + STATE_STALE_SECS > now_unix())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn hook_state(ctx: &Ctx) -> (String, bool) {
    let sid = {
        let mut sid = s(ctx.d, &["session_id"]);
        if sid.is_empty() {
            sid = s(ctx.d, &["sessionId"]);
        }
        sid
    };
    // HST_STATE_FILE 覆盖缝(hst-rev G-7):state 写入者已退役,本缝保留为
    // 遗留档显与测试缝(verify 与单测注入),生面孔无写入者。
    if let Ok(f) = std::env::var("HST_STATE_FILE") {
        if !f.is_empty() {
            let p = PathBuf::from(&f);
            if p.is_file() && state_fresh(&p) {
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
    if keyed.is_file() && state_fresh(&keyed) {
        cands.push(keyed);
    }
    let latest = ctx.home.join("state").join(format!("{}.json", ctx.agent));
    if latest.is_file() && state_fresh(&latest) {
        cands.push(latest);
    }
    if !ctx.root.is_empty() {
        let proj = Path::new(&ctx.root)
            .join(".hst")
            .join("state")
            .join(format!("{}.json", ctx.agent));
        if proj.is_file() && state_fresh(&proj) {
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

/// hook 功能别名清单（entropy-01 2026-10-12 用户令：段名纯英文，中文显
/// 示名全撤直用英文 id；hst-state 腿已整体退役不收录）。未收录的回落
/// stem 本名；清单含外来 hook（herdr 等）。
const HOOK_ALIASES: &[(&str, &str)] = &[
    // herdr agent 状态 hook：claude/grok 面 SessionStart 会话登记推
    // herdr server（pane 与会话绑定），kimi 面每事件推 working/idle 态。
    ("herdr-agent-state", "herdr-agent-state"),
    // hst token 护栏 hook：PreToolUse/UserPromptSubmit 跑 secretguard
    // 密钥拦截（API key 命中 exit 2 阻断）。
    ("hst-token", "hst-token-guard"),
    // hst goal 腿：Claude Code Stop 等四事件提醒注入与 goal 自驱。
    ("hst-task", "hst-task"),
];

/// 新增 hook 收录指引（评审 G3）：只改 HOOK_ALIASES 一处加 stem 判定回
/// 落即可；并确认该 agent 是否多文件注册面（grok 已知 ohmyagents-
/// state.json 与 herdr.json 双文件，注册面读全目录 *.json）。
/// hookstate 段（第 5 行，用户令 2026-09-26 起，2026-09-27 四令迭代）：
/// 注册面全部 hook 的功能别名清单独占一行（用户令「不需要working这些
/// 状态 显示hook功能的别名」：态文本退出缺省显示，仅以行色暗示；{state}
/// 占位符保留供自配）；出行门 = 状态文件在场或有挂载 hook 任一（评审
/// 快核 G1：刚装未触发的空窗期不隐清单）；双缺整行隐藏（零噪声）；
/// 注册面读不出 hook 时回落泛称 `hook` 保语义。注册面三源（用户级
/// settings 加项目级 settings 族加全局插件面，REQ-044）。恒单行不折
///（REQ-044 折行面上线即裁撤，用户令 2026-10-03「不要换行」：超宽由
/// 终端截断，Claude Code 渲染层行为，hst 不代劳）。
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
    // REQ-051 退化表:清单内部用 `|`;小于 80 列截首三个,小于 60 列只留
    // 首一个。
    let cap = if ctx.width < 60 {
        1
    } else if ctx.width < 80 {
        3
    } else {
        usize::MAX
    };
    let alias = if alias_raw.is_empty() {
        "hook".to_string()
    } else {
        alias_raw
            .into_iter()
            .take(cap)
            .collect::<Vec<_>>()
            .join("|")
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
/// 字典序殿后）映射别名为序表（REQ-051：清单内部用 `|` 细分，截断与
/// 拼接归调用方；用户令 2026-09-27 两轮收敛）。claude 额外并读项目级
/// 注册面（REQ-039：payload `workspace.project_dir` 下
/// `.claude/settings.json` 加 `settings.local.json`，项目守卫 hook 如
/// session-tool-guard 族入列）与插件注册面（REQ-044：`~/.claude/plugins/`
/// 全局插件 hook，显示形走覆写表）。文件缺失、坏损或零挂载返回空表。
/// codex 虽无外部状态栏面，手动 render 亦可得清单。
fn hooked_aliases(agent: &str, d: &Json) -> Vec<String> {
    match user_home() {
        Ok(h) => hooked_aliases_at(&h, agent, &s(d, &["workspace", "project_dir"])),
        Err(_) => Vec::new(),
    }
}

/// hooked_aliases 的可测形（home 加项目根显式透传）。
fn hooked_aliases_at(home: &Path, agent: &str, project_root: &str) -> Vec<String> {
    let mut cmds: Vec<String> = Vec::new();
    // stem 到显示名覆写表（REQ-044 插件面：插件名连字符加 statusMessage，
    // 自带前缀不双拼）。
    let mut display: std::collections::BTreeMap<String, String> = Default::default();
    match agent {
        // claude/codex/grok 注册面同构（hooks.<Event>[].hooks[].command）。
        "claude" => {
            collect_json_commands(&home.join(".claude").join("settings.json"), &mut cmds);
            if !project_root.is_empty() {
                let pdotclaude = std::path::Path::new(project_root).join(".claude");
                collect_json_commands(&pdotclaude.join("settings.json"), &mut cmds);
                collect_json_commands(&pdotclaude.join("settings.local.json"), &mut cmds);
            }
            collect_plugin_hooks(home, project_root, &mut cmds, &mut display);
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
    let mut out: Vec<String> = HOOK_ALIASES
        .iter()
        .filter(|(stem, _)| stems.contains(*stem))
        .map(|(_, alias)| (*alias).to_string())
        .collect();
    let known: Vec<&str> = HOOK_ALIASES.iter().map(|(s, _)| *s).collect();
    out.extend(
        stems
            .iter()
            .filter(|s| !known.contains(&s.as_str()))
            .map(|s| {
                // 覆写表只作用于未收录 stem（已知表 stem 是舰队公共别名，表形
                // 优先不覆写）。
                display.get(s).cloned().unwrap_or_else(|| s.clone())
            }),
    );
    out
}

/// claude 插件注册面收集（REQ-044）：`~/.claude/plugins/installed_plugins.json`
///（v2 形 plugins 名到条目数组，名形 `plugin@marketplace`）；enabled 门控
/// 见 `plugin_enabled`；条目 user 作用域全收，project 作用域仅 projectPath
/// 命中当前根者收；逐条读 installPath 下 `hooks/hooks.json`。显示形 =
/// 插件名连字符加 statusMessage（hook 自带态文案，缺席回落干 stem；文案
/// 总台纯名令起恒显纯插件名,statusMessage 不上栏）。
/// installed_plugins.json 缺席加坏损零命中不炸。
fn collect_plugin_hooks(
    home: &Path,
    project_root: &str,
    cmds: &mut Vec<String>,
    display: &mut std::collections::BTreeMap<String, String>,
) {
    let ip = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    let Ok(v) = crate::yolo::read_json(&ip) else {
        return;
    };
    let Some(plugins) = v.get("plugins").and_then(|p| p.as_object()) else {
        return;
    };
    for (name, entries) in plugins {
        let Some(entries) = entries.as_array() else {
            continue;
        };
        if !plugin_enabled(home, project_root, name) {
            continue;
        }
        let short = name.split('@').next().unwrap_or(name);
        for e in entries {
            let scope = e.get("scope").and_then(|x| x.as_str()).unwrap_or("");
            if scope == "project" {
                let pp = e.get("projectPath").and_then(|x| x.as_str()).unwrap_or("");
                if project_root.is_empty() || std::path::Path::new(pp) != Path::new(project_root) {
                    continue;
                }
            }
            let Some(install) = e.get("installPath").and_then(|x| x.as_str()) else {
                continue;
            };
            let hj = Path::new(install).join("hooks").join("hooks.json");
            let Ok(hv) = crate::yolo::read_json(&hj) else {
                continue;
            };
            // 总台令「只显式 evo-lint」:插件 hook 恒显纯插件名,statusMessage
            // 文案不上栏(REQ-044 双拼形与 REQ-051 CJK 回落一并退役)。
            walk_json_hooks(&hv, |c, _m| {
                cmds.push(c.to_string());
                if let Some(stem) = hook_stem(c) {
                    display.entry(stem).or_insert_with(|| short.to_string());
                }
            });
        }
    }
}

/// enabledPlugins 门控（REQ-044）：项目 local 大于项目大于用户级首见即
/// 用（Claude Code settings 层叠序）；无显式项判未启用（安装动作落
/// true、禁用动作落 false，缺省关闭）。企业 managed settings 面不入源
///（REQ-044 边界）。
fn plugin_enabled(home: &Path, project_root: &str, name: &str) -> bool {
    let mut files = vec![home.join(".claude").join("settings.json")];
    if !project_root.is_empty() {
        let pdotclaude = Path::new(project_root).join(".claude");
        files.push(pdotclaude.join("settings.json"));
        files.push(pdotclaude.join("settings.local.json"));
    }
    // 逆序遍历：最具体作用域优先（local > project > user）。
    for f in files.iter().rev() {
        if let Ok(v) = crate::yolo::read_json(f) {
            if let Some(b) = v
                .get("enabledPlugins")
                .and_then(|p| p.get(name))
                .and_then(|x| x.as_bool())
            {
                return b;
            }
        }
    }
    false
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
    walk_json_hooks(&v, |c, _| cmds.push(c.to_string()));
}

/// hooks.<Event>[].hooks[] 遍历（REQ-044 抽出共用）：对每条命令回调
///（command 加 statusMessage，statusMessage 缺席空串）。
fn walk_json_hooks(v: &Json, mut f: impl FnMut(&str, &str)) {
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
                        let m = h
                            .get("statusMessage")
                            .and_then(|m| m.as_str())
                            .unwrap_or("");
                        f(c, m);
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

/// REQ-014 哨兵（原生侧，REQ-027）：unknown 时探注册面，缺位升格
/// `no-hook!`；节流窗（每 agent 1 小时）到期 best-effort 自愈
/// `hst hook init`（幂等重注册，仅 hook 面；不在 PATH 或失败静默，不阻塞
/// 渲染）。REQ-052 起仅 claude 有腿（task 四事件），codex 与 grok 与
/// kimi 零 hst 钩为常态不哨。stamp 记 ts 内容（pwsh 形记 mtime；内容形
/// 可测）。
fn sentinel_no_hook(ctx: &Ctx, state: &str) -> Option<String> {
    if state != "unknown" {
        return None;
    }
    let home = match user_home() {
        Ok(h) => h,
        Err(_) => return None,
    };
    // REQ-052:token 腿退钩,唯 claude 挂腿(task 四事件);他家零钩常态
    // 不哨(entropy-01 判据的 token 分量保留认存量未重 init 机)。
    if ctx.agent != "claude" {
        return None;
    }
    let reg = home.join(".claude").join("settings.json");
    let reg_ok = std::fs::read_to_string(&reg)
        .map(|t| t.contains("hst-task") || t.contains("hst-token") || t.contains("hst-goal"))
        .unwrap_or(false);
    if reg_ok {
        return None;
    }
    // 节流自愈：stamp（~/.hst/state/.hookcheck-<agent>）记录上次尝试 ts，
    // 窗内不重试。
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

/// REQ-051 L1 身份段:agent 类型加版本(版本归一承 seg_hst 逻辑;grok 与
/// kimi 的 payload version 同路,缺键只显 agent 名)。
fn seg_agent(ctx: &Ctx) -> Option<String> {
    let ver = agent_version(ctx.d);
    let agent_disp = if ver.is_empty() {
        ctx.agent.clone()
    } else {
        format!("{}-{ver}", ctx.agent)
    };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "agent"),
            &[("icon", icon_of(ctx, "agent")), ("agent", agent_disp)],
        ),
        "38;5;250",
    ))
}

/// payload version 归一(seg_hst 先例:取数字加点形)。
fn agent_version(d: &Json) -> String {
    let pv = s(d, &["version"]);
    if pv.is_empty() {
        return String::new();
    }
    let mut num = String::new();
    let mut seen_digit = false;
    for c in pv.chars() {
        if c.is_ascii_digit() || (c == '.' && seen_digit) {
            num.push(c);
            seen_digit = true;
        } else if !num.is_empty() && (c.is_ascii_alphanumeric() || c == '.' || c == '+' || c == '-')
        {
            num.push(c);
        } else if !num.is_empty() {
            break;
        }
    }
    if num.contains('.') {
        num
    } else {
        String::new()
    }
}

/// REQ-051 L1 fleet 段:快照档纯读(零网络零子进程)。总台第九笔回执
/// 「18 前面用不同颜色显示在线 3/18」:恢复 W/N 形且在跑数异色
///(38;5;114 绿,零在跑不加色回落基色);!B 加 Dn 记号保持退役(第六笔);
/// 龄降级 = 5 分钟外灰显带龄,30 分钟外段隐藏。段级色与在跑数色分段
/// 施加(ansi 复位后基色重开,避免嵌套复位吞色)。
fn seg_fleet(ctx: &Ctx) -> Option<String> {
    let snap = ctx.home.join("state").join("fleet-snap.json");
    let text = std::fs::read_to_string(&snap).ok()?;
    let v: Json = serde_json::from_str(&text).ok()?;
    let total = v.get("total").and_then(|x| x.as_u64())?;
    if total == 0 {
        return None;
    }
    let ts = v.get("ts").and_then(|x| x.as_u64()).unwrap_or(0);
    let age = now_unix().saturating_sub(ts);
    if age > 30 * 60 {
        return None; // 超龄整段隐藏
    }
    let working = v.get("working").and_then(|x| x.as_u64()).unwrap_or(0);
    let stale = age > 5 * 60;
    let (color, age_txt) = if stale {
        ("38;5;245", format!(" ·{}m", age / 60))
    } else {
        ("38;5;117", String::new())
    };
    // r3 G-1 收裁:陈旧快照整段灰,在跑数不抢眼(快照虽旧在跑照实的
    // 读法弃,视觉一致性优先)。
    let w_txt = if working > 0 && !stale {
        ansi(&working.to_string(), "38;5;114")
    } else {
        working.to_string()
    };
    let body = format!("{w_txt}{}", ansi(&format!("/{total}{age_txt}"), color));
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, "fleet"),
            &[("icon", icon_of(ctx, "fleet")), ("fleet", body)],
        ),
        color,
    ))
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
    // REQ-051 键族按 agent:grok 三键严形(used_percentage 加 context_tokens
    // 加 context_window_size 全在场才出段,grok 评审禁百分比反推不画 0%);
    // kimi 比值族(contextUsage/contextTokens/maxContextTokens);claude 形
    // used 缺时按取整百分比反推(旧口径兼容)。窄窗退化:小于 80 列去
    // token 括号对,小于 40 列单格条加百分比。
    let shaped = match ctx.agent.as_str() {
        "grok" => context_grok(ctx.d)?,
        "kimi" => context_kimi(ctx.d)?,
        _ => context_shaped(ctx.d).or_else(|| context_kimi(ctx.d))?,
    };
    let (pct, used, win) = shaped;
    let body = if ctx.width < 40 {
        format!("{} {pct}%", bar_cells(pct, 1))
    } else if ctx.width < 80 {
        format!("{} {pct}%", bar_cells(pct, 4))
    } else {
        let key = if ctx.nerd { "context" } else { "context-ascii" };
        apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "context")),
                ("bar", bar_cells(pct, 4)),
                ("pct", pct.to_string()),
                ("used", fmt_tok(used)),
                ("window", fmt_tok(win)),
                ("mix", String::new()),
            ],
        )
    };
    Some(ansi(&body, "38;5;116"))
}

/// context 条形格(▰ 实加▱ 空):filled = ceil(pct*n/100) 截 0..n。
fn bar_cells(pct: u64, n: usize) -> String {
    let filled = (pct as usize * n).div_ceil(100).min(n);
    format!("{}{}", "▰".repeat(filled), "▱".repeat(n - filled))
}

/// 目录 basename(双分隔符兼容 Windows 路径;空串原样)。
fn dir_basename(dir: &str) -> String {
    dir.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(dir)
        .to_string()
}

/// grok 评审三键严形:used_percentage 加 context_tokens 加 context_window_size
/// 三键缺任一整段隐藏(禁百分比反推)。
fn context_grok(d: &Json) -> Option<(u64, f64, f64)> {
    let pct = f64_at(d, &["context_window", "used_percentage"])?.floor();
    let used = f64_at(d, &["context_window", "context_tokens"])?;
    let win = f64_at(d, &["context_window", "context_window_size"])?;
    Some((pct as u64, used, win))
}

/// claude 与 grok 的 context_window 三键形:(pct, used, window)。
fn context_shaped(d: &Json) -> Option<(u64, f64, f64)> {
    let win = f64_at(d, &["context_window", "context_window_size"])?;
    let pct = match f64_at(d, &["context_window", "used_percentage"]) {
        Some(u) => u.floor(),
        None => 100.0 - f64_at(d, &["context_window", "remaining_percentage"])?.floor(),
    };
    // grok 评审:used 直读 context_tokens,反推只作回落。
    let used = f64_at(d, &["context_window", "context_tokens"]).unwrap_or(win * pct / 100.0);
    Some((pct as u64, used, win))
}

/// kimi 的比值形:contextUsage(0.42 表 42%)加 contextTokens 加 maxContextTokens。
fn context_kimi(d: &Json) -> Option<(u64, f64, f64)> {
    let ratio = f64_at(d, &["contextUsage"])?;
    let used = f64_at(d, &["contextTokens"])?;
    let win = f64_at(d, &["maxContextTokens"])?;
    if win <= 0.0 {
        return None;
    }
    Some(((ratio * 100.0).floor() as u64, used, win))
}

/// model 段名（pwsh 同判：display_name 优先回落 id，双缺空串）。
fn model_name(d: &Json) -> String {
    // REQ-051 kimi 承接:model 键为字符串直取(kimi 契约);claude 形
    // display_name 优先回落 id。
    if let Some(m) = d.get("model").and_then(|m| m.as_str()) {
        return m.to_string();
    }
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

/// REQ-045：goal 段改 hst 自有存储源（`~/.hst/state/goal/<slug>.json`，
/// slug 同 trace 与 projyolo 规则；Stop 腿 rounds 事件驱动刷新）。项目根
/// 解析序承接旧 loop 探针链（workspace.project_dir 回落 current_dir 回落
/// dir）；四家同源显示，不再锚 claude session。存储坏损容忍隐行（读侧
/// 只读不修，doctor check=goal 报 warn）。
fn seg_goal(ctx: &Ctx) -> Option<String> {
    let rec = task_record(ctx)?;
    let mut text = rec.text.replace(['\r', '\n'], " ").trim().to_string();
    if text.is_empty() {
        return None;
    }
    // REQ-051 退化表:task 文本满窗截 60 字,小于 100 列截 16 字,小于 60
    // 列截 8 字(省略号尾)。
    let cap = if ctx.width < 60 {
        8
    } else if ctx.width < 100 {
        16
    } else {
        60
    };
    if text.chars().count() > cap {
        let cut: String = text.chars().take(cap).collect();
        text = format!("{cut}…");
    }
    // REQ-051 task 循环时间:距上轮活动(last_stop_at,无则 set_at)的
    // d/h/m 形;零新 IO 同源读档。
    let last = if rec.last_stop_at > 0 {
        rec.last_stop_at
    } else {
        rec.set_at
    };
    let age = fmt_age(now_unix().saturating_sub(last));
    // REQ-051 goalmode 退化前缀旗:在役态(active 加 paused)才显,余隐藏。
    let mode = match goalmode_probe(ctx) {
        Some((st, _)) if st == "active" || st == "paused" => format!("{st} "),
        _ => String::new(),
    };
    let key = if ctx.nerd { "goal" } else { "goal-ascii" };
    Some(ansi(
        &apply_fmt(
            &tmpl_of(ctx, key),
            &[
                ("icon", icon_of(ctx, "goal")),
                ("goal", text),
                ("count", rec.rounds.to_string()),
                ("age", age),
                ("mode", mode),
            ],
        ),
        "38;5;179",
    ))
}

/// 就近上溯取在册 task 记录(段 id goal 与 task 同源;kimi 紧凑形同用)。
fn task_record(ctx: &Ctx) -> Option<crate::task::TaskRecord> {
    let root = {
        let mut root = s(ctx.d, &["workspace", "project_dir"]);
        if root.is_empty() || root == "." {
            root = s(ctx.d, &["workspace", "current_dir"]);
        }
        if root.is_empty() || root == "." {
            root = ctx.dir.clone();
        }
        root
    };
    crate::task::task_file_for(ctx.home, Path::new(&root))
        .and_then(|p| crate::task::read_record(&p).ok().flatten())
}

/// 距上轮活动的 d/h/m 形(59s 内为 now,59 分内为 Xm,其余 Xh,超 24h 为 Xd)。
fn fmt_age(secs: u64) -> String {
    if secs < 60 {
        "now".into()
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 24 * 3600 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
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
    // REQ-042 评审 G：retry 形 system informational（goal check could not
    // complete · retrying）是活态信号（paused 后无 check-in 的复活支），
    // 产 Active 态（text 回溯更早 set 或 check-in）。
    // 全量评审 N10 收紧：带 system informational 前缀与 paused 对称。
    let retry_act = r#""type":"system","subtype":"informational","content":"Goal still active"#;
    let retry_act_sp =
        r#""type": "system", "subtype": "informational", "content": "Goal still active"#;
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
        } else if let Some(p) = anchor_len(chunk, i, retry_act, retry_act_sp) {
            found = Some((
                p,
                Marker {
                    kind: MarkerKind::Active,
                    text: None,
                },
            ));
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
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"\"/x/.hst/hooks/hst-task.sh\" claude"}]}],"SessionStart":[{"matcher":"*","hooks":[{"type":"command","command":"bash '/other/herdr-agent-state.sh' session"}]}],"Notification":[{"matcher":"*","hooks":[{"type":"command","command":"bash /x/.hst/hooks/hst-token.sh claude"}]}]}}"#,
        )
        .unwrap();
        // 别名表序（herdr 在先，用户例序），未收录 metric-bridge 回落本名殿后。
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", "").join("|"),
            "herdr-agent-state|hst-token-guard|hst-task"
        );
        // kimi TOML 面：ours 加外来同列。
        let kimi_dir = tmp.join(".kimi-code");
        std::fs::create_dir_all(&kimi_dir).unwrap();
        std::fs::write(
            kimi_dir.join("config.toml"),
            "[[hooks]]\nevent = \"PreToolUse\"\ncommand = \"/x/.hst/hooks/hst-token.sh kimi\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"/y/herdr-agent-state.sh\"\n",
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "kimi", "").join("|"),
            "herdr-agent-state|hst-token-guard"
        );
        // grok 多文件注册面（评审 F）：hst 的 ohmyagents-state.json 与
        // herdr 的 herdr.json 双文件合并收集。
        let grok_dir = tmp.join(".grok").join("hooks");
        std::fs::create_dir_all(&grok_dir).unwrap();
        std::fs::write(
            grok_dir.join("ohmyagents-state.json"),
            r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"/x/hst-task.sh grok"}]}]}}"#,
        )
        .unwrap();
        std::fs::write(
            grok_dir.join("herdr.json"),
            r#"{"hooks":{"SessionStart":[{"matcher":"*","hooks":[{"type":"command","command":"/z/herdr-agent-state.sh session"}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "grok", "").join("|"),
            "herdr-agent-state|hst-task"
        );
        // REQ-039：claude 项目级注册面并入（payload project_dir 下
        // .claude/settings.json 加 settings.local.json），未收录 stem
        //（项目守卫）字典序殿后；与用户级同 stem 去重。
        let proj = tmp.join("proj");
        let pdot = proj.join(".claude");
        std::fs::create_dir_all(&pdot).unwrap();
        std::fs::write(
            pdot.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-tool-guard.sh\""}]}],"Stop":[{"hooks":[{"type":"command","command":"/x/.hst/hooks/hst-task.sh claude"}]}]}}"#,
        )
        .unwrap();
        std::fs::write(
            pdot.join("settings.local.json"),
            r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/knowledge-recall.sh\""}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", proj.to_str().unwrap()).join("|"),
            "herdr-agent-state|hst-token-guard|hst-task|knowledge-recall|session-tool-guard"
        );
        // 坏损 JSON 零命中不炸。
        std::fs::write(claude_dir.join("settings.json"), "{ not json").unwrap();
        assert!(hooked_aliases_at(&tmp, "claude", "").is_empty());
        // 缺文件零命中空表（段内回落泛称 hook）。
        std::fs::remove_file(claude_dir.join("settings.json")).unwrap();
        assert!(hooked_aliases_at(&tmp, "claude", "").is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn hooked_aliases_plugin_face_enabled_gated_and_display() {
        // REQ-044：全局插件 hook 入列——installed_plugins.json v2 形收
        // user 作用域条目；enabledPlugins 门控（user true 收、false 拒、
        // 无显式项缺省关、项目 local 覆写用户级）；project 作用域仅
        // projectPath 命中当前根者收；显示 = 插件名连字符加 statusMessage
        //（缺席回落干 stem；自带插件名前缀不双拼，修复单 2026-10-12）；
        // 坏损 installed_plugins.json 零命中不炸。
        let tmp = std::env::temp_dir().join(format!("hst-hpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let claude_dir = tmp.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let proj = tmp.join("proj");
        let pdot = proj.join(".claude");
        std::fs::create_dir_all(&pdot).unwrap();
        // 插件缓存面：a（user 作用域，带 statusMessage）、b（user 作用域，
        // 无 statusMessage 回落 stem）、d（project 作用域钉 proj 根）、
        // e（user 作用域，statusMessage 自带插件名前缀，双拼防回归件）。
        // hooks.json 用 json! 构（命令含引号与 ${} 面，format! 直插会产
        // 坏 JSON）。
        let mk_hooks = |rel: &str, cmd: &str, msg: &str| {
            let dir = tmp.join(rel).join("hooks");
            std::fs::create_dir_all(&dir).unwrap();
            let mut h = serde_json::json!({"hooks":{"PostToolUse":[{"hooks":[{"command":cmd}]}]}});
            if !msg.is_empty() {
                h["hooks"]["PostToolUse"][0]["hooks"][0]["statusMessage"] = serde_json::json!(msg);
            }
            std::fs::write(dir.join("hooks.json"), h.to_string()).unwrap();
        };
        mk_hooks(
            "plugins-cache/a/1",
            "uv run --no-project \"${CLAUDE_PLUGIN_ROOT}/skills/code-kit/scripts/md-guard.py\"",
            "md 禁字门禁",
        );
        mk_hooks(
            "plugins-cache/b/1",
            "bash ${CLAUDE_PLUGIN_ROOT}/hooks/plain-guard.sh",
            "",
        );
        mk_hooks(
            "plugins-cache/d/1",
            "bash ${CLAUDE_PLUGIN_ROOT}/hooks/proj-guard.sh",
            "项目守卫",
        );
        mk_hooks(
            "plugins-cache/e/1",
            "uv run --no-project \"${CLAUDE_PLUGIN_ROOT}/scripts/prefix-guard.py\"",
            "plug-e 前缀静检",
        );
        let ip = serde_json::json!({
            "version": 2,
            "plugins": {
                "plug-a@mk": [{ "scope": "user", "installPath": tmp.join("plugins-cache/a/1") }],
                "plug-b@mk": [{ "scope": "user", "installPath": tmp.join("plugins-cache/b/1") }],
                "plug-c@mk": [{ "scope": "user", "installPath": tmp.join("plugins-cache/c/1") }],
                "plug-d@mk": [{ "scope": "project", "projectPath": proj, "installPath": tmp.join("plugins-cache/d/1") }],
                "plug-e@mk": [{ "scope": "user", "installPath": tmp.join("plugins-cache/e/1") }]
            }
        });
        let plugdir = tmp.join(".claude").join("plugins");
        std::fs::create_dir_all(&plugdir).unwrap();
        std::fs::write(
            plugdir.join("installed_plugins.json"),
            serde_json::to_string(&ip).unwrap(),
        )
        .unwrap();
        // 用户级 settings：ours hook + enabledPlugins（a 显式 true、b
        // 显式 false、c 无显式项缺省关、d 依赖项目根命中、e 显式 true）。
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"Stop":[{"hooks":[{"command":"/x/.hst/hooks/hst-task.sh claude"}]}]},"enabledPlugins":{"plug-a@mk":true,"plug-b@mk":false,"plug-e@mk":true}}"#,
        )
        .unwrap();
        // 总台纯名令:插件 hook 恒显纯插件名(statusMessage 一律不上
        // 栏);a 入列,b 拒,c 缺省关,d 项目作用域无根不收。
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", "").join("|"),
            "hst-task|plug-a|plug-e"
        );
        // 项目根命中：d 入列（projectPath 同根；项目作用域插件由项目级
        // settings 落 enabled，项目档入链）；项目 local 把 b 覆写 true
        //（local 大于项目大于用户级）——b 恒显纯插件名。
        std::fs::write(
            pdot.join("settings.json"),
            r#"{"enabledPlugins":{"plug-d@mk":true}}"#,
        )
        .unwrap();
        std::fs::write(
            pdot.join("settings.local.json"),
            r#"{"enabledPlugins":{"plug-b@mk":true}}"#,
        )
        .unwrap();
        assert_eq!(
            hooked_aliases_at(&tmp, "claude", proj.to_str().unwrap()).join("|"),
            "hst-task|plug-a|plug-b|plug-e|plug-d"
        );
        // 坏损 installed_plugins.json 零命中不炸（回落 settings 面）。
        std::fs::write(
            tmp.join(".claude")
                .join("plugins")
                .join("installed_plugins.json"),
            "{ not json",
        )
        .unwrap();
        assert_eq!(hooked_aliases_at(&tmp, "claude", "").join("|"), "hst-task");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn agent_segment_normalizes_payload_version() {
        // REQ-051 L1 身份段:payload version 归一承 seg_hst 逻辑,归一出
        // 点形才拼 agent-版本,缺键只显 agent 名。
        let home = std::env::temp_dir().join(format!("hst-sr-agent-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let cfg = StatuslineConfig::default();
        let mk = |payload: &str| -> Option<String> {
            let d: Json = serde_json::from_str(payload).unwrap();
            let ctx = Ctx {
                d: &d,
                agent: "claude".to_string(),
                nerd: false,
                cfg: &cfg,
                home: &home,
                dir: "/w/p".to_string(),
                root: String::new(),
                proj_kind: String::new(),
                pkg_ver: String::new(),
                width: 120,
            };
            seg_agent(&ctx)
        };
        assert!(
            mk(r#"{"version":"2.1.163 (Claude Code)"}"#)
                .unwrap()
                .contains("claude-2.1.163"),
            "带尾巴的 CLI 版本归一成点形"
        );
        assert!(
            mk(r#"{"version":"2.1.163"}"#)
                .unwrap()
                .contains("claude-2.1.163"),
            "裸版本形"
        );
        let bare = mk("{}").unwrap();
        assert!(
            bare.contains("claude") && !bare.contains("claude-"),
            "缺键只显 agent 名不造形: {bare}"
        );
        let nopoint = mk(r#"{"version":"unknown"}"#).unwrap();
        assert!(
            nopoint.contains("claude") && !nopoint.contains("claude-"),
            "无点版本不拼: {nopoint}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn fleet_segment_reads_snapshot_with_age_degradation() {
        // REQ-051 fleet 段:纯读快照零子进程;总台第九笔回执 W/N 形
        // 且在跑数绿色异显(零在跑回落基色);!B 加 Dn 记号保持退役;
        // 5 分钟外灰显带龄,30 分钟外
        // 整段隐藏;total=0 与缺档隐藏。评审 r1 F-1:断言面押 ANSI 色码,
        // 须 ENV_LOCK 互斥加剥双开关(NO_COLOR 与 HST_STATUSLINE_NO_ANSI
        // 任一在场即剥色必红),反例锁 HST_STATUSLINE_NO_ANSI=1 剥色形。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("NO_COLOR");
        std::env::remove_var("HST_STATUSLINE_NO_ANSI");
        let home = std::env::temp_dir().join(format!("hst-sr-fleet-{}", std::process::id()));
        std::fs::create_dir_all(home.join("state")).unwrap();
        let cfg = StatuslineConfig {
            template: vec![("fleet".to_string(), "{fleet}".to_string())],
            ..Default::default()
        };
        let d: Json = serde_json::from_str("{}").unwrap();
        let snap = |body: &str| {
            std::fs::write(home.join("state").join("fleet-snap.json"), body).unwrap();
        };
        macro_rules! mk_ctx {
            ($home:expr) => {
                Ctx {
                    d: &d,
                    agent: "claude".to_string(),
                    nerd: false,
                    cfg: &cfg,
                    home: $home,
                    dir: "/w/p".to_string(),
                    root: String::new(),
                    proj_kind: String::new(),
                    pkg_ver: String::new(),
                    width: 120,
                }
            };
        }
        let now = now_unix();
        snap(&format!(
            r#"{{"ts":{},"total":12,"working":1,"idle":10,"blocked":0,"done":1,"hosts":[]}}"#,
            now
        ));
        let fresh = seg_fleet(&mk_ctx!(&home)).unwrap();
        assert!(
            fresh.contains("\u{1b}[38;5;114m1\u{1b}[0m"),
            "在跑数绿色异显: {fresh}"
        );
        assert!(fresh.contains("/12"), "总数随段基色: {fresh}");
        assert!(fresh.contains("38;5;117"), "段基色: {fresh}");
        snap(&format!(
            r#"{{"ts":{},"total":5,"working":1,"idle":3,"blocked":1,"done":0,"hosts":[]}}"#,
            now
        ));
        assert!(
            seg_fleet(&mk_ctx!(&home)).unwrap().contains("/5"),
            "blocked 计入总数,记号不上栏"
        );
        // 6 分钟龄:灰显形带龄。
        snap(&format!(
            r#"{{"ts":{},"total":9,"working":2,"idle":7,"blocked":0,"done":0,"hosts":[]}}"#,
            now - 6 * 60
        ));
        let aged = seg_fleet(&mk_ctx!(&home)).unwrap();
        assert!(aged.contains("/9 ·6m"), "龄 6 分钟灰显带龄: {aged}");
        assert!(aged.contains("38;5;245"), "灰显色: {aged}");
        assert!(
            !aged.contains("38;5;114"),
            "r3 G-1 收裁:陈旧快照整段灰,在跑数不抢眼: {aged}"
        );
        // 31 分钟龄:整段隐藏。
        snap(&format!(
            r#"{{"ts":{},"total":9,"working":2,"idle":7,"blocked":0,"done":0,"hosts":[]}}"#,
            now - 31 * 60
        ));
        assert!(seg_fleet(&mk_ctx!(&home)).is_none(), "超龄隐藏");
        // total=0 与缺档:隐藏。
        snap(r#"{"ts":1,"total":0,"working":0,"idle":0,"blocked":0,"done":0,"hosts":[]}"#);
        assert!(seg_fleet(&mk_ctx!(&home)).is_none());
        let _ = std::fs::remove_file(home.join("state").join("fleet-snap.json"));
        assert!(seg_fleet(&mk_ctx!(&home)).is_none());
        // 评审 r1 F-1 反例:HST_STATUSLINE_NO_ANSI=1 剥色,色码断言面不现。
        std::env::set_var("HST_STATUSLINE_NO_ANSI", "1");
        snap(&format!(
            r#"{{"ts":{},"total":12,"working":1,"idle":10,"blocked":0,"done":1,"hosts":[]}}"#,
            now
        ));
        let stripped = seg_fleet(&mk_ctx!(&home)).unwrap();
        assert_eq!(stripped, "1/12", "剥色后裸文本: {stripped}");
        std::env::remove_var("HST_STATUSLINE_NO_ANSI");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn fmt_age_and_dir_basename_shapes() {
        // REQ-051 task 循环时间 d/h/m 形与目录 basename 双分隔符。
        assert_eq!(fmt_age(59), "now");
        assert_eq!(fmt_age(60), "1m");
        assert_eq!(fmt_age(18 * 60), "18m");
        assert_eq!(fmt_age(3 * 3600), "3h");
        assert_eq!(fmt_age(2 * 86_400), "2d");
        assert_eq!(dir_basename("/home/ray/repos/hst_rs"), "hst_rs");
        assert_eq!(dir_basename("D:\\repos\\ai-ccoe"), "ai-ccoe");
        assert_eq!(dir_basename("/w/x/"), "x", "尾分隔符剥");
        assert_eq!(dir_basename(""), "");
    }

    #[test]
    fn context_three_shapes_and_bar() {
        // REQ-051 三形:grok 三键严形(缺任一整段隐藏禁反推);kimi 比值族;
        // claude 形 used 缺时百分比反推回落;bar 四格 ceil。
        let d_grok: Json = serde_json::from_str(
            r#"{"context_window":{"used_percentage":42.7,"context_tokens":84000,"context_window_size":200000}}"#,
        )
        .unwrap();
        assert_eq!(context_grok(&d_grok), Some((42, 84000.0, 200000.0)));
        let no_used: Json = serde_json::from_str(
            r#"{"context_window":{"used_percentage":42.0,"context_window_size":200000}}"#,
        )
        .unwrap();
        assert!(
            context_grok(&no_used).is_none(),
            "grok 缺 context_tokens 整段隐藏"
        );
        let d_kimi: Json = serde_json::from_str(
            r#"{"contextUsage":0.42,"contextTokens":84000,"maxContextTokens":200000}"#,
        )
        .unwrap();
        assert_eq!(context_kimi(&d_kimi), Some((42, 84000.0, 200000.0)));
        let d_claude: Json = serde_json::from_str(
            r#"{"context_window":{"used_percentage":46.0,"context_window_size":977000}}"#,
        )
        .unwrap();
        // used 缺:取整百分比反推(46% * 977k)。
        assert_eq!(context_shaped(&d_claude), Some((46, 449420.0, 977000.0)));
        assert_eq!(bar_cells(0, 4), "▱▱▱▱");
        assert_eq!(bar_cells(42, 4), "▰▰▱▱");
        assert_eq!(bar_cells(100, 4), "▰▰▰▰");
        assert_eq!(bar_cells(30, 1), "▰");
        assert_eq!(bar_cells(0, 1), "▱");
    }

    #[test]
    fn kimi_renders_single_compact_line() {
        // REQ-051 kimi 恒单行:必留五件(版本加 basename 加分支加模型加
        // context)加 task 紧凑形 R3·18m 加 hook 首 id;分支直读 payload
        // 不起 git;fleet 与 clock 不上。
        let home = std::env::temp_dir().join(format!("hst-sr-kimi-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let cfg = StatuslineConfig {
            single_line: false,
            ..Default::default()
        };
        let d: Json = serde_json::from_str(
            r#"{"version":"2.1.1","cwd":"/home/ray/repos/hst_rs","gitBranch":"lane/statusbar-v2","model":"kimi-k2","contextUsage":0.42,"contextTokens":84000,"maxContextTokens":200000}"#,
        )
        .unwrap();
        let ctx = Ctx {
            d: &d,
            agent: "kimi".to_string(),
            nerd: false,
            cfg: &cfg,
            home: &home,
            dir: "/home/ray/repos/hst_rs".to_string(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
            width: 120,
        };
        let line = render_kimi_line(&ctx);
        assert!(!line.contains('\n') || line.ends_with('\n'), "恒单行");
        assert!(line.contains("kimi-2.1.1"), "版本: {line}");
        assert!(line.contains("hst_rs"), "basename: {line}");
        assert!(line.contains("lane/statusbar-v2"), "分支直读: {line}");
        assert!(line.contains("kimi-k2"), "模型: {line}");
        assert!(line.contains("42%"), "context pct: {line}");
        assert!(!line.contains("\u{1f411}"), "fleet 不上: {line}");
        assert!(!line.contains("󰥔"), "clock 不上: {line}");
        // task 在册:紧凑形 R0·(age)。
        crate::task::set_task(
            &home,
            std::path::Path::new("/home/ray/repos/hst_rs"),
            "eval 批",
        )
        .unwrap();
        let line2 = render_kimi_line(&ctx);
        assert!(line2.contains("R0·"), "task 紧凑形: {line2}");
        assert!(!line2.contains("eval 批"), "task 文本不上 kimi 行: {line2}");
        let _ = std::fs::remove_dir_all(&home);
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
        // 注册在场不升格；非 unknown 不升格；stamp 节流窗内不重触发。
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
            width: 120,
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
        // 注册面在场(现行腿):不升格(hst-rev F-1:判据改押 hst-token 与
        // hst-task 任一)。
        std::fs::write(
            user.join(".claude").join("settings.json"),
            "\"command\": \"/x/.hst/hooks/hst-task.sh\"",
        )
        .unwrap();
        assert_eq!(sentinel_no_hook(&ctx, "unknown"), None);
        std::fs::write(
            user.join(".claude").join("settings.json"),
            "\"command\": \"/x/.hst/hooks/hst-token.sh\"",
        )
        .unwrap();
        assert_eq!(sentinel_no_hook(&ctx, "unknown"), None);
        // 旧形 state-only 注册不认(自愈升级向量):升格并落 stamp。
        std::fs::write(
            user.join(".claude").join("settings.json"),
            "\"command\": \"/x/.hst/hooks/hst-state.sh\"",
        )
        .unwrap();
        assert_eq!(
            sentinel_no_hook(&ctx, "unknown").as_deref(),
            Some("no-hook!")
        );
        // 非 unknown：不升格。
        assert_eq!(sentinel_no_hook(&ctx, "working"), None);
        // REQ-052:唯 claude 挂腿,他家零钩常态不哨(注册面缺位也不升格)。
        let ctx_kimi = Ctx {
            d: &d,
            agent: "kimi".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &hst_home,
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
            width: 120,
        };
        assert_eq!(
            sentinel_no_hook(&ctx_kimi, "unknown"),
            None,
            "kimi 零钩不哨"
        );
        let ctx_codex = Ctx {
            d: &d,
            agent: "codex".to_string(),
            nerd: true,
            cfg: &cfg,
            home: &hst_home,
            dir: String::new(),
            root: String::new(),
            proj_kind: String::new(),
            pkg_ver: String::new(),
            width: 120,
        };
        assert_eq!(
            sentinel_no_hook(&ctx_codex, "unknown"),
            None,
            "codex 零钩不哨"
        );
        std::env::remove_var("HOME");
        drop(_env);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sentinel_proj_yolo_faces() {
        // REQ-027 原生侧：marker 命中同项目且新鲜升格（复合态并显）、项目
        // 不符与过期不升格、no-hook! 优先不叠加。
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
            width: 120,
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
    fn context_shaped_floor_and_remaining_fallback() {
        // 评审 F5：used 向下取整（46.7 出 46 非 47）；缺 used 时
        // remaining 回退 100 减其向下取整；used 缺时取整百分比反推
        //（REQ-051 claude 形回落道,直读优先）。REQ-051 起 pwsh 同口径
        // 函数 ctx_pct_win 收敛入 context_shaped。
        let d: Json = serde_json::from_str(
            r#"{"context_window":{"context_window_size":977000,"used_percentage":46.7}}"#,
        )
        .unwrap();
        assert_eq!(context_shaped(&d), Some((46, 449_420.0, 977_000.0)));
        let d: Json = serde_json::from_str(
            r#"{"context_window":{"context_window_size":1000,"remaining_percentage":30.9}}"#,
        )
        .unwrap();
        assert_eq!(context_shaped(&d).unwrap().0, 70, "100 - floor(30.9)");
        let d: Json =
            serde_json::from_str(r#"{"context_window":{"context_window_size":1000}}"#).unwrap();
        assert!(context_shaped(&d).is_none(), "both missing hides segment");
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
            width: 120,
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
    fn scan_markers_still_active_retry_form() {
        // REQ-042 评审 G：retry 形 system informational 产 Active 态（text
        // 回溯），paused 后无 check-in 的复活支不再冻在 paused。
        let both = r#"{"type":"queue-operation","content":"Goal set: 目标甲"}
{"type":"system","subtype":"informational","content":"Goal paused · the goal check timed out"}
{"type":"system","subtype":"informational","content":"Goal still active · the goal check could not complete · retrying"}
"#;
        let ms = scan_markers(both);
        assert_eq!(ms.len(), 3);
        assert!(matches!(ms[ms.len() - 1].kind, MarkerKind::Active));
        assert_eq!(ms[ms.len() - 1].text, None, "text backtracks to set");
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
    fn goal_segment_reads_own_storage_and_truncates() {
        // REQ-045：goal 段改 hst 自有存储源（四家同源，不再锚 claude
        // session）；{count} 承 Stop 腿轮次；截 60；缺 goal 隐行；payload
        // 项目根缺席回落 dir。
        let home = std::env::temp_dir().join(format!(
            "hst-sr-goal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&home).unwrap();
        let root = std::path::Path::new("/w/hst-proj");
        crate::task::set_task(&home, root, "盯发布窗口").unwrap();
        let cfg = StatuslineConfig {
            template: vec![("goal-ascii".to_string(), "{goal}|{count}".to_string())],
            ..Default::default()
        };
        macro_rules! mk_ctx {
            ($d:expr, $dir:expr) => {
                Ctx {
                    d: $d,
                    agent: "claude".to_string(),
                    nerd: false,
                    cfg: &cfg,
                    home: &home,
                    dir: $dir.to_string(),
                    root: String::new(),
                    proj_kind: String::new(),
                    pkg_ver: String::new(),
                    width: 120,
                }
            };
        }
        // payload 带项目根：键中自有存储。
        let with_root: Json = serde_json::from_str(
            r#"{"session_id":"s1","workspace":{"project_dir":"/w/hst-proj"}}"#,
        )
        .unwrap();
        let row = seg_goal(&mk_ctx!(&with_root, "/elsewhere")).unwrap();
        assert!(row.contains("盯发布窗口|0"), "{row}");
        // payload 项目根缺席：回落 dir（后续断言共用该形）。
        let bare: Json = serde_json::from_str(r#"{"session_id":"s1"}"#).unwrap();
        let row = seg_goal(&mk_ctx!(&bare, "/w/hst-proj")).unwrap();
        assert!(row.contains("盯发布窗口"), "dir fallback: {row}");
        // goal-01 根因 B 修：dir 为项目子目录时上溯命中项目根 goal。
        let row = seg_goal(&mk_ctx!(&bare, "/w/hst-proj/sub/deep")).unwrap();
        assert!(row.contains("盯发布窗口"), "subdir walk: {row}");
        // 轮次随 Stop 腿计数联动。
        let stop = serde_json::json!({ "hook_event_name": "Stop", "cwd": "/w/hst-proj" });
        crate::task::stop_hook(&home, Some(&stop), None).unwrap();
        let row = seg_goal(&mk_ctx!(&bare, "/w/hst-proj")).unwrap();
        assert!(row.contains("盯发布窗口|1"), "{row}");
        // 超 60 字截断。
        let long = "长".repeat(72);
        crate::task::set_task(&home, root, &long).unwrap();
        let row = seg_goal(&mk_ctx!(&bare, "/w/hst-proj")).unwrap();
        assert!(row.contains(&format!("{}…", "长".repeat(60))), "{row}");
        // 清空隐行。
        crate::task::clear_task(&home, root).unwrap();
        assert!(seg_goal(&mk_ctx!(&bare, "/w/hst-proj")).is_none());
        let _ = std::fs::remove_dir_all(&home);
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
            icons: vec![("goal".to_string(), "GOAL ".to_string())],
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
            width: 120,
        };
        assert_eq!(icon_of(&ctx, "goal"), "GOAL ");
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
            width: 120,
        };
        assert_eq!(icon_of(&ctx_ascii, "loop"), "");
    }
}
