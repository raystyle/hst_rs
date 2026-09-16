//! hst trace：查询时联邦的四家会话日志检索（P0013，S019）。
//! 直接读各家原生会话库并归一化——零采集设施、可回溯 hst 部署前的历史。
//! 写库即归一化原则（S018 坑 3）：文件路径一律正斜杠；项目比较统一「正斜杠 + 小写」。

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// 意图截断上限（S018：按字符不按字节，中文按字节截会 panic）。
const MAX_INTENT_CHARS: usize = 200;
/// 分页 clamp（S018 参数形状）。
pub const DEFAULT_LIMIT: usize = 100;
/// trace 六视图的MAX_LIMIT面（细则见 R002 与模块文档）。
pub const MAX_LIMIT: usize = 1000;

#[derive(Debug, Clone)]
/// trace 六视图的TraceSession面（细则见 R002 与模块文档）。
pub struct TraceSession {
    /// 该字段承载trace 六视图的agent数据。
    pub agent: String,
    /// 该字段承载trace 六视图的id数据。
    pub id: String,
    /// 该字段承载trace 六视图的project数据。
    pub project: PathBuf,
    /// 该字段承载trace 六视图的file数据。
    pub file: PathBuf,
    /// 该字段承载trace 六视图的started_at数据。
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// trace 六视图的EditKind面（细则见 R002 与模块文档）。
pub enum EditKind {
    /// 该字段承载trace 六视图的Create数据。
    Create,
    /// 该字段承载trace 六视图的Modify数据。
    Modify,
    /// 该字段承载trace 六视图的Delete数据。
    Delete,
}

impl EditKind {
    /// trace 六视图的as_str面（细则见 R002 与模块文档）。
    pub fn as_str(self) -> &'static str {
        match self {
            EditKind::Create => "create",
            EditKind::Modify => "modify",
            EditKind::Delete => "delete",
        }
    }
}

/// trace 六视图的归一化事件面（细则见 R002 与模块文档）。
#[derive(Debug, Clone)]
pub struct TraceEvent {
    /// 该字段承载trace 六视图的agent数据。
    pub agent: String,
    /// 该字段承载trace 六视图的session_id数据。
    pub session_id: String,
    /// 该字段承载trace 六视图的call_id数据。
    pub call_id: Option<String>,
    /// 该字段承载trace 六视图的tool数据。
    pub tool: Option<String>,
    /// 该字段承载trace 六视图的file数据。
    pub file: Option<String>,
    /// 该字段承载trace 六视图的kind数据。
    pub kind: EditKind,
    /// 该字段承载trace 六视图的user_intent数据。
    pub user_intent: Option<String>,
    /// 该字段承载trace 六视图的op_intent数据。
    pub op_intent: Option<String>,
    /// 该字段承载trace 六视图的ts数据。
    pub ts: Option<String>,
    /// 排序用统一 epoch ms（四家时间源不同：claude/codex ISO、kimi 原生 ms、grok 会话 uuidv7 近似）。
    pub ts_ms: Option<u64>,
    /// 可检索正文（patch、Edit new_string、Write content 等）。
    pub patch: Option<String>,
}

impl TraceEvent {
    /// trace 六视图的operation_id面（细则见 R002 与模块文档）。
    pub fn operation_id(&self) -> String {
        format!(
            "{}:{}",
            self.session_id,
            self.call_id.as_deref().unwrap_or("-")
        )
    }
}

/// trace 六视图的意图操作块面（细则见 R002 与模块文档）。
#[derive(Debug, Clone)]
pub struct TraceBlock {
    /// 该字段承载trace 六视图的op数据。
    pub op: String,
    /// 该字段承载trace 六视图的agent数据。
    pub agent: String,
    /// 该字段承载trace 六视图的session_id数据。
    pub session_id: String,
    /// 该字段承载trace 六视图的files数据。
    pub files: Vec<String>,
    /// 该字段承载trace 六视图的edits数据。
    pub edits: usize,
    /// 该字段承载trace 六视图的kinds数据。
    pub kinds: Vec<String>,
    /// 该字段承载trace 六视图的first_ts数据。
    pub first_ts: Option<String>,
    /// 该字段承载trace 六视图的last_ts数据。
    pub last_ts: Option<String>,
    /// 该字段承载trace 六视图的user_intent数据。
    pub user_intent: Option<String>,
    /// 该字段承载trace 六视图的op_intent数据。
    pub op_intent: Option<String>,
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// 按首次出现顺序聚合事件为块（一个 operation_id 一块）。
pub fn group_blocks(events: &[TraceEvent]) -> Vec<TraceBlock> {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::BTreeMap<String, TraceBlock> = Default::default();
    for e in events {
        let op = e.operation_id();
        if !map.contains_key(&op) {
            order.push(op.clone());
            map.insert(
                op.clone(),
                TraceBlock {
                    op: op.clone(),
                    agent: e.agent.clone(),
                    session_id: e.session_id.clone(),
                    files: Vec::new(),
                    edits: 0,
                    kinds: Vec::new(),
                    first_ts: e.ts.clone(),
                    last_ts: e.ts.clone(),
                    user_intent: e.user_intent.clone(),
                    op_intent: e.op_intent.clone(),
                },
            );
        }
        let b = map.get_mut(&op).unwrap();
        b.edits += 1;
        if let Some(f) = &e.file {
            if !b.files.contains(f) {
                b.files.push(f.clone());
            }
        }
        let k = e.kind.as_str().to_string();
        if !b.kinds.contains(&k) {
            b.kinds.push(k);
        }
        b.last_ts = e.ts.clone().or_else(|| b.last_ts.clone());
    }
    order.into_iter().filter_map(|op| map.remove(&op)).collect()
}

/// trace 六视图的TraceFilter面（细则见 R002 与模块文档）。
pub struct TraceFilter<'a> {
    /// 该字段承载trace 六视图的agent数据。
    pub agent: Option<&'a str>,
    /// 该字段承载trace 六视图的file_glob数据。
    pub file_glob: Option<&'a str>,
    /// 该字段承载trace 六视图的limit数据。
    pub limit: usize,
    /// 翻页偏移（D26）：按视图输出序跳过前 offset 条（timeline / search /
    /// file 的窗口从最新往旧走，offset 即往更早翻页）。缺省 0。
    pub offset: usize,
}

impl<'a> TraceFilter<'a> {
    /// trace 六视图的clamp_limit面（细则见 R002 与模块文档）。
    pub fn clamp_limit(&self) -> usize {
        self.limit.clamp(1, MAX_LIMIT)
    }
}

fn normalize_compare(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn normalize_file(f: &str) -> String {
    f.replace('\\', "/")
}

/// 项目内路径相对化（比较用小写、返回保留原大小写的正斜杠相对形）。
fn relativize(file: &str, project: &Path) -> String {
    let pn = normalize_compare(project);
    let pn = pn.trim_end_matches('/');
    let fl = file.to_lowercase();
    if let Some(rest) = fl.strip_prefix(&format!("{pn}/")) {
        let cut = file.len() - rest.len();
        return file[cut..].to_string();
    }
    file.to_string()
}

fn truncate_chars(s: &str) -> String {
    s.chars().take(MAX_INTENT_CHARS).collect()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_intent(s: &str) -> String {
    truncate_chars(&collapse_ws(s))
}

// ---- 时间统一：ISO RFC3339 子集与 epoch ms 互转（Howard Hinnant 算法，无 chrono 依赖） ----

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = ((m + 9) % 12) as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// "2026-08-31T09:24:32.851Z" 形（毫秒可选）→ epoch ms；纯数字串原样解析。
pub fn ts_to_ms(s: &str) -> Option<u64> {
    let t = s.trim();
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        return t.parse().ok();
    }
    if t.len() < 19 || t.as_bytes()[4] != b'-' {
        return None;
    }
    let num = |a: usize, b: usize| t.get(a..b).and_then(|x| x.parse::<u64>().ok());
    let (y, mo, d) = (num(0, 4)? as i64, num(5, 7)? as u32, num(8, 10)? as u32);
    let (h, mi, se) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let ms = if t.as_bytes().get(19) == Some(&b'.') {
        num(20, 23).unwrap_or(0)
    } else {
        0
    };
    let days = days_from_civil(y, mo, d);
    Some((days as u64 * 86_400 + h * 3600 + mi * 60 + se) * 1000 + ms)
}

/// trace 六视图的ms_to_iso面（细则见 R002 与模块文档）。
pub fn ms_to_iso(ms: u64) -> String {
    let days = (ms / 86_400_000) as i64;
    let rem = ms % 86_400_000;
    let (y, mo, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y,
        mo,
        d,
        rem / 3_600_000,
        (rem / 60_000) % 60,
        (rem / 1000) % 60,
        rem % 1000
    )
}

/// grok 会话 uuid v7 前 48 位（12 个 hex 字）是 unix ms（官方文档口径）——
/// 事件无行级时间时的会话起点近似。
fn grok_uuid_v7_ms(id: &str) -> Option<u64> {
    let hex: String = id.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() < 12 {
        return None;
    }
    u64::from_str_radix(&hex[..12], 16).ok()
}

/// 按行读 jsonl，坏行跳过（append-only 容错：截断尾行不崩检索）。
fn read_json_lines(path: &Path) -> Vec<serde_json::Value> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

// ---- 会话发现：四家原生库 ----

/// `~/.claude/projects/<slug>/`：slug 规则是路径串里非字母数字一律换 `-`
/// （D:\ohmyagents → D--ohmyagents）。
pub fn claude_project_slug(project: &Path) -> String {
    project
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// trace 六视图的claude_sessions_in面（细则见 R002 与模块文档）。
pub fn claude_sessions_in(dir: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        out.push(TraceSession {
            agent: "claude".into(),
            id: id.to_string(),
            // D26 #5：started 取 jsonl 首行 timestamp（会话最早条目）。
            started_at: first_line_timestamp(&p),
            project: project.to_path_buf(),
            file: p,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// jsonl 首个带 timestamp 的行（claude 会话开始时间；首行常是 mode 元
/// 数据行无时间，向前扫封顶 50 行；全无返回 None）。
fn first_line_timestamp(path: &Path) -> Option<String> {
    let f = fs::File::open(path).ok()?;
    let mut reader = io::BufReader::new(f);
    for _ in 0..50 {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => return None,
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(ts) = v.get("timestamp").and_then(|x| x.as_str()) {
                return Some(ts.to_string());
            }
        }
    }
    None
}

/// codex rollout：首行 session_meta.payload.cwd 决定项目归属。
pub fn codex_sessions_under(root: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !name.starts_with("rollout-") {
                    continue;
                }
                if let Some(meta) = codex_session_meta(&p) {
                    let cwd = meta.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
                    if normalize_compare(Path::new(cwd)) == normalize_compare(project) {
                        let id = meta
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let started = meta
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        out.push(TraceSession {
                            agent: "codex".into(),
                            id,
                            project: project.to_path_buf(),
                            file: p,
                            started_at: started,
                        });
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

fn codex_session_meta(file: &Path) -> Option<serde_json::Value> {
    let file = fs::File::open(file).ok()?;
    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                return v.get("payload").cloned();
            }
            // 首行不是 session_meta 就不必再扫（meta 恒在首行）。
            return None;
        }
    }
    None
}

/// trace 六视图的grok面（细则见 R002 与模块文档）。
/// 权威日志是 updates.jsonl（S020：chat_history 是派生缓存，compaction 会重建）；
/// 缺 updates 的旧会话退 chat_history。started_at：updates 首行 timestamp（秒），
/// 退 uuid v7 生成时刻近似。
pub fn grok_sessions_in(root: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let want = normalize_compare(project);
    let Ok(rd) = fs::read_dir(root) else {
        return out;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if normalize_compare(Path::new(&percent_decode(name))) != want {
            continue;
        }
        let Ok(inner) = fs::read_dir(&p) else {
            continue;
        };
        for sdir in inner.flatten() {
            let spath = sdir.path();
            let updates = spath.join("updates.jsonl");
            let file = if updates.is_file() {
                updates
            } else {
                let hist = spath.join("chat_history.jsonl");
                if hist.is_file() {
                    hist
                } else {
                    continue;
                }
            };
            let Some(id) = spath.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let started = if file.file_name().and_then(|s| s.to_str()) == Some("updates.jsonl") {
                grok_updates_first_ms(&file).or_else(|| grok_uuid_v7_ms(id))
            } else {
                grok_uuid_v7_ms(id)
            };
            out.push(TraceSession {
                agent: "grok".into(),
                id: id.to_string(),
                project: project.to_path_buf(),
                file,
                started_at: started.map(ms_to_iso),
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// updates.jsonl 首行信封的 timestamp（秒）转 ms；读不出返回 None。
fn grok_updates_first_ms(file: &Path) -> Option<u64> {
    let first = std::fs::read_to_string(file)
        .ok()?
        .lines()
        .next()?
        .trim()
        .to_string();
    let v: serde_json::Value = serde_json::from_str(&first).ok()?;
    let secs = v.get("timestamp")?.as_u64()?;
    Some(secs * 1000)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// trace 六视图的kimi面（细则见 R002 与模块文档）。
/// append-only 索引带墓碑行（{sessionId, deleted:true}），后行覆盖前行。
pub fn kimi_sessions_in(index: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let want = normalize_compare(project);
    let mut seen: std::collections::BTreeMap<String, Option<PathBuf>> = Default::default();
    for v in read_json_lines(index) {
        let id = v
            .get("sessionId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            continue;
        }
        // 墓碑行只有 {sessionId, deleted:true}（无 workDir），删除标记先于项目过滤。
        if v.get("deleted").and_then(|x| x.as_bool()).unwrap_or(false) {
            seen.insert(id, None);
            continue;
        }
        let work = v.get("workDir").and_then(|x| x.as_str()).unwrap_or("");
        if normalize_compare(Path::new(work)) != want {
            continue;
        }
        let file = v
            .get("sessionDir")
            .and_then(|x| x.as_str())
            .map(PathBuf::from)
            .map(|d| d.join("agents").join("main").join("wire.jsonl"));
        seen.insert(id, file);
    }
    for (id, file) in seen {
        let Some(file) = file else { continue };
        if !file.is_file() {
            continue;
        }
        out.push(TraceSession {
            agent: "kimi".into(),
            id,
            project: project.to_path_buf(),
            file,
            started_at: None,
        });
    }
    out
}

// ---- 事件抽取 ----

/// trace 六视图的会话库根面（细则见 R002 与模块文档）。
/// 同形——Windows 的家目录解析走 SHGetKnownFolderPath，HOME / USERPROFILE
/// 环境重定向对 dirs 无效，故显式开一扇门）。
fn sessions_home() -> PathBuf {
    std::env::var_os("HST_TRACE_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// trace 六视图的四家环境入口面（细则见 R002 与模块文档）。
pub fn list_sessions(project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let home = sessions_home();
    out.extend(claude_sessions_in(
        &home
            .join(".claude")
            .join("projects")
            .join(claude_project_slug(project)),
        project,
    ));
    out.extend(codex_sessions_under(
        &home.join(".codex").join("sessions"),
        project,
    ));
    out.extend(grok_sessions_in(
        &home.join(".grok").join("sessions"),
        project,
    ));
    out.extend(kimi_sessions_in(
        &home.join(".kimi-code").join("session_index.jsonl"),
        project,
    ));
    out
}

/// trace 六视图的四家环境入口面（细则见 R002 与模块文档）。
pub fn timeline(project: &Path) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    for s in list_sessions(project) {
        match s.agent.as_str() {
            "claude" => out.extend(claude_events(&s)),
            "codex" => out.extend(codex_events(&s)),
            "grok" => out.extend(grok_events(&s)),
            "kimi" => out.extend(kimi_events(&s)),
            _ => {}
        }
    }
    out.sort_by(|a, b| a.ts_ms.cmp(&b.ts_ms));
    out
}

/// claude transcript：父链近似为行序——tool_use 前最近的 assistant text 是操作意图、
/// 最近的真实 user text 是用户意图（S018 算法的顺序文件版）。
pub fn claude_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for v in read_json_lines(&session.file) {
        let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        match t {
            "user" => {
                if v.get("isMeta").and_then(|x| x.as_bool()).unwrap_or(false) {
                    continue;
                }
                let msg = v.get("message").cloned().unwrap_or_default();
                if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
                    if !text.trim().is_empty() {
                        user_intent = Some(clean_intent(text));
                    }
                }
            }
            "assistant" => {
                let msg = v.get("message").cloned().unwrap_or_default();
                let ts = v
                    .get("timestamp")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                let ts_ms = ts.as_deref().and_then(ts_to_ms);
                if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
                    for b in blocks {
                        let bt = b.get("type").and_then(|x| x.as_str()).unwrap_or("");
                        if bt == "text" {
                            let text = b.get("text").and_then(|x| x.as_str()).unwrap_or("");
                            if !text.trim().is_empty() {
                                op_intent = Some(clean_intent(text));
                            }
                        } else if bt == "tool_use" {
                            let name = b.get("name").and_then(|x| x.as_str()).unwrap_or("");
                            if !matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
                                continue;
                            }
                            let input = b.get("input").cloned().unwrap_or_default();
                            let file = input
                                .get("file_path")
                                .and_then(|x| x.as_str())
                                .map(|f| relativize(&normalize_file(f), &session.project));
                            let patch = input
                                .get("new_string")
                                .or_else(|| input.get("content"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());
                            out.push(TraceEvent {
                                agent: session.agent.clone(),
                                session_id: session.id.clone(),
                                call_id: b
                                    .get("id")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                tool: Some(name.to_string()),
                                file,
                                kind: EditKind::Modify,
                                user_intent: user_intent.clone(),
                                op_intent: op_intent.clone(),
                                ts: ts.clone(),
                                ts_ms,
                                patch,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// codex rollout：编辑主源是 `event_msg/item_completed` 的 `FileChange` item（绝对路径 +
/// add/delete/update + 内容或 unified_diff + call_id + completed_at_ms，S019 源码核实）；
/// 旧版无 FileChange 时退回 `custom_tool_call(apply_patch)` 补丁头解析。意图按行序回溯。
pub fn codex_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    let mut fc: Vec<TraceEvent> = Vec::new();
    let mut ct: Vec<TraceEvent> = Vec::new();
    for line in read_json_lines(&session.file) {
        let lt = line.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let payload = line.get("payload").cloned().unwrap_or_default();
        match lt {
            "response_item" => {
                let pt = payload.get("type").and_then(|x| x.as_str()).unwrap_or("");
                if pt == "message" {
                    let role = payload.get("role").and_then(|x| x.as_str()).unwrap_or("");
                    let text = payload
                        .get("content")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|b| b.get("text"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    if text.trim().is_empty() {
                        continue;
                    }
                    match role {
                        "user" => {
                            if is_codex_injected_context(text) {
                                continue;
                            }
                            user_intent = Some(clean_intent(text));
                        }
                        "assistant" => op_intent = Some(clean_intent(text)),
                        _ => {}
                    }
                } else if pt == "custom_tool_call" {
                    let name = payload.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    if name != "apply_patch" {
                        continue;
                    }
                    let input = payload.get("input").and_then(|x| x.as_str()).unwrap_or("");
                    let ts = line
                        .get("timestamp")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                    for (kind, file) in parse_apply_patch(input) {
                        ct.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: payload
                                .get("call_id")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            tool: Some("apply_patch".into()),
                            file: Some(relativize(&file, &session.project)),
                            kind,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ts.clone(),
                            ts_ms: ts.as_deref().and_then(ts_to_ms),
                            patch: Some(input.to_string()),
                        });
                    }
                }
            }
            "event_msg" => {
                if payload.get("type").and_then(|x| x.as_str()) != Some("item_completed") {
                    continue;
                }
                let item = payload.get("item").cloned().unwrap_or_default();
                if item.get("type").and_then(|x| x.as_str()) != Some("FileChange") {
                    continue;
                }
                let call_id = item
                    .get("id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                let ms = payload
                    .get("completed_at_ms")
                    .and_then(|x| x.as_u64())
                    .or_else(|| payload.get("started_at_ms").and_then(|x| x.as_u64()));
                if let Some(changes) = item.get("changes").and_then(|c| c.as_object()) {
                    for (path, change) in changes {
                        let ct_tag = change.get("type").and_then(|x| x.as_str()).unwrap_or("");
                        let kind = match ct_tag {
                            "add" => EditKind::Create,
                            "delete" => EditKind::Delete,
                            _ => EditKind::Modify,
                        };
                        let patch = change
                            .get("unified_diff")
                            .or_else(|| change.get("content"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string());
                        fc.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: call_id.clone(),
                            tool: Some("apply_patch".into()),
                            file: Some(relativize(&normalize_file(path), &session.project)),
                            kind,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ms.map(ms_to_iso),
                            ts_ms: ms,
                            patch,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    if fc.is_empty() {
        ct
    } else {
        fc
    }
}

/// codex 的环境注入 user message（指令/环境块）不是用户意图；marker 清单对齐
/// codex 源码 `CONTEXTUAL_USER_FRAGMENT_MATCHERS` 的主要项（S019 核实）。
fn is_codex_injected_context(text: &str) -> bool {
    let head = text.trim_start();
    head.starts_with("# AGENTS.md")
        || head.starts_with("<environment_context>")
        || head.starts_with("<user_instructions>")
        || head.starts_with("<turn_context>")
        || head.starts_with("<turn_aborted>")
        || head.starts_with("<user_shell_command>")
        || head.starts_with("<subagent_notification>")
        || head.starts_with("<current_time_reminder>")
}

/// 解析 apply_patch 补丁头的 Add/Update/Delete File 行。
pub fn parse_apply_patch(input: &str) -> Vec<(EditKind, String)> {
    let mut out = Vec::new();
    for line in input.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("*** Add File: ") {
            out.push((EditKind::Create, normalize_file(rest.trim())));
        } else if let Some(rest) = line.strip_prefix("*** Update File: ") {
            out.push((EditKind::Modify, normalize_file(rest.trim())));
        } else if let Some(rest) = line.strip_prefix("*** Delete File: ") {
            out.push((EditKind::Delete, normalize_file(rest.trim())));
        }
    }
    out
}

/// grok 双源分发（S020）：updates.jsonl 是权威日志，chat_history.jsonl 是派生缓存
/// （compaction 触发整体重建）。会话发现层已按存在性选源，这里按文件名分发。
pub fn grok_events(session: &TraceSession) -> Vec<TraceEvent> {
    if session.file.file_name().and_then(|s| s.to_str()) == Some("updates.jsonl") {
        return grok_events_from_updates(session);
    }
    grok_events_from_chat_history(session)
}

/// grok updates.jsonl（权威日志，S020）：信封 `{timestamp:秒, method, params:{sessionId,
/// update:{sessionUpdate,...}}}`。method 两流：`session/update` 管内容（user/agent 分片、
/// tool_call），`_x.ai/session/update` 管遥测（hook/turn/compaction）——内容只读前者。
/// trace 六视图的四要素面（细则见 R002 与模块文档）。
/// 是用户意图；agent_message_chunk 连续拼接是操作意图；tool_call 的 `_meta` 下 `x.ai/tool`
/// 带 kind（write/edit/read，判写族免名字硬编码），`rawInput` 是现成对象；时间用信封
/// timestamp（秒）——每事件真实时间，替代 v1 的会话起点近似。
fn grok_events_from_updates(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for v in read_json_lines(&session.file) {
        if v.get("method").and_then(|x| x.as_str()) != Some("session/update") {
            continue;
        }
        let Some(u) = v.get("params").and_then(|p| p.get("update")) else {
            continue;
        };
        let secs = u_ms(v.get("timestamp"));
        match u
            .get("sessionUpdate")
            .and_then(|x| x.as_str())
            .unwrap_or("")
        {
            "user_message_chunk" => {
                // 合成闸门：hideFromScrollback 标记的注入（system-reminder 等）不是用户意图。
                let hidden = u
                    .pointer("/_meta/hideFromScrollback")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                if hidden {
                    continue;
                }
                let text = u
                    .pointer("/content/text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if !text.trim().is_empty() {
                    // 同一 prompt 可能分多片：连续拼接；新 prompt 重置操作意图。
                    user_intent = Some(match user_intent.take() {
                        Some(prev) => format!("{prev}\n{}", clean_intent(text)),
                        None => clean_intent(text),
                    });
                    op_intent = None;
                }
            }
            "agent_message_chunk" => {
                let text = u
                    .pointer("/content/text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if !text.trim().is_empty() {
                    op_intent = Some(match op_intent.take() {
                        Some(prev) => format!("{prev}\n{}", clean_intent(text)),
                        None => clean_intent(text),
                    });
                }
            }
            "tool_call" => {
                let meta = u.pointer("/_meta/x.ai~1tool").cloned().unwrap_or_default();
                let name = meta
                    .get("name")
                    .and_then(|x| x.as_str())
                    .or_else(|| u.get("title").and_then(|x| x.as_str()))
                    .unwrap_or("");
                let kind = meta.get("kind").and_then(|x| x.as_str()).unwrap_or("");
                let write_family = matches!(kind, "write" | "edit")
                    || (kind.is_empty()
                        && matches!(
                            name,
                            "search_replace" | "write" | "edit" | "hashline_edit" | "apply_patch"
                        ));
                if !write_family {
                    continue;
                }
                let args = u.get("rawInput").cloned().unwrap_or_default();
                let file = args
                    .get("file_path")
                    .or_else(|| args.get("path"))
                    .or_else(|| args.get("target_file"))
                    .and_then(|x| x.as_str())
                    .map(|f| relativize(&normalize_file(f), &session.project));
                let patch = args
                    .get("new_string")
                    .or_else(|| args.get("content"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                out.push(TraceEvent {
                    agent: session.agent.clone(),
                    session_id: session.id.clone(),
                    call_id: u
                        .get("toolCallId")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string()),
                    tool: Some(name.to_string()),
                    file,
                    kind: EditKind::Modify,
                    user_intent: user_intent.clone(),
                    op_intent: op_intent.clone(),
                    ts: secs.map(ms_to_iso),
                    ts_ms: secs,
                    patch,
                });
            }
            _ => {}
        }
    }
    out
}

/// 信封 timestamp（秒，int 或 float）转 ms。
fn u_ms(v: Option<&serde_json::Value>) -> Option<u64> {
    match v.and_then(|x| x.as_u64()) {
        Some(s) => Some(s * 1000),
        None => v.and_then(|x| x.as_f64()).map(|f| (f * 1000.0) as u64),
    }
}

/// grok chat_history.jsonl（ConversationItem 行，S019 源码核实）：真实 user 行
/// （`synthetic_reason == null`）更用户意图；assistant.content 是操作意图；
/// 编辑在 `assistant.tool_calls[]`（name 属写文件族，arguments 是 JSON 串取 file_path）。
/// 行无时间戳，ts 用会话 uuid v7 的生成时刻近似（官方口径前 48 位 unix ms）。
/// 已被 updates.jsonl 主源取代（S020），留作缺 updates 的旧会话兜底。
fn grok_events_from_chat_history(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    let session_ms = grok_uuid_v7_ms(&session.id);
    let ts = session_ms.map(ms_to_iso);
    for v in read_json_lines(&session.file) {
        match v.get("type").and_then(|x| x.as_str()).unwrap_or("") {
            "user" => {
                if !v
                    .get("synthetic_reason")
                    .map(|s| s.is_null())
                    .unwrap_or(true)
                {
                    continue;
                }
                let text = concat_text_parts(v.get("content"));
                if !text.trim().is_empty() {
                    user_intent = Some(clean_intent(&text));
                }
            }
            "assistant" => {
                if let Some(text) = v.get("content").and_then(|c| c.as_str()) {
                    if !text.trim().is_empty() {
                        op_intent = Some(clean_intent(text));
                    }
                }
                if let Some(calls) = v.get("tool_calls").and_then(|c| c.as_array()) {
                    for tc in calls {
                        let name = tc.get("name").and_then(|x| x.as_str()).unwrap_or("");
                        if !matches!(
                            name,
                            "search_replace" | "write" | "edit" | "hashline_edit" | "apply_patch"
                        ) {
                            continue;
                        }
                        let args = tc
                            .get("arguments")
                            .and_then(|x| x.as_str())
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                            .unwrap_or_default();
                        let file = args
                            .get("file_path")
                            .or_else(|| args.get("path"))
                            .and_then(|x| x.as_str())
                            .map(|f| relativize(&normalize_file(f), &session.project));
                        let patch = args
                            .get("new_string")
                            .or_else(|| args.get("content"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string());
                        out.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: tc.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                            tool: Some(name.to_string()),
                            file,
                            kind: EditKind::Modify,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ts.clone(),
                            ts_ms: session_ms,
                            patch,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// kimi wire.jsonl（协议 1.5，S019 源码核实）：`turn.prompt` 且 `origin.kind=="user"` 是
/// 用户意图权威源；`context.append_message` role=assistant 的 text part 是操作意图；
/// 编辑 = loop event `tool.call`（name 属 Edit/Write，args.path 是路径键）。时间原生 epoch ms。
pub fn kimi_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for v in read_json_lines(&session.file) {
        let ms = v.get("time").and_then(|x| x.as_u64());
        let ts = ms.map(ms_to_iso);
        match v.get("type").and_then(|x| x.as_str()).unwrap_or("") {
            "turn.prompt" => {
                let origin = v
                    .get("origin")
                    .and_then(|o| o.get("kind"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if origin != "user" {
                    continue;
                }
                let text = concat_text_parts(v.get("input"));
                if !text.trim().is_empty() {
                    user_intent = Some(clean_intent(&text));
                }
            }
            "context.append_message" => {
                let msg = v.get("message").cloned().unwrap_or_default();
                if msg.get("role").and_then(|x| x.as_str()) != Some("assistant") {
                    continue;
                }
                let text = concat_text_parts(msg.get("content"));
                if !text.trim().is_empty() {
                    op_intent = Some(clean_intent(&text));
                }
            }
            "context.append_loop_event" => {
                let ev = v.get("event").cloned().unwrap_or_default();
                if ev.get("type").and_then(|x| x.as_str()) != Some("tool.call") {
                    continue;
                }
                let name = ev.get("name").and_then(|x| x.as_str()).unwrap_or("");
                if !matches!(name, "Edit" | "Write") {
                    continue;
                }
                let args = ev.get("args").cloned().unwrap_or_default();
                let file = args
                    .get("path")
                    .or_else(|| args.get("file_path"))
                    .and_then(|x| x.as_str())
                    .map(|f| relativize(&normalize_file(f), &session.project));
                let patch = args
                    .get("new_string")
                    .or_else(|| args.get("content"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                out.push(TraceEvent {
                    agent: session.agent.clone(),
                    session_id: session.id.clone(),
                    call_id: ev
                        .get("toolCallId")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string()),
                    tool: Some(name.to_string()),
                    file,
                    kind: EditKind::Modify,
                    user_intent: user_intent.clone(),
                    op_intent: op_intent.clone(),
                    ts: ts.clone(),
                    ts_ms: ms,
                    patch,
                });
            }
            _ => {}
        }
    }
    out
}

/// ContentPart 数组（[{type:"text",text},...]）取 text 拼接；跳过 think/blob_ref/媒体 part。
fn concat_text_parts(v: Option<&serde_json::Value>) -> String {
    let Some(v) = v else { return String::new() };
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    let Some(arr) = v.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for p in arr {
        if p.get("type").and_then(|x| x.as_str()) == Some("text") {
            if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                out.push_str(t);
            }
        }
    }
    out
}

// ---- 过滤与检索 ----

/// trace 六视图的apply_filter面（细则见 R002 与模块文档）。
pub fn apply_filter(events: Vec<TraceEvent>, filter: &TraceFilter) -> Vec<TraceEvent> {
    apply_filter_counted(events, filter).0
}

/// 过滤加窗口计数（D26）：返回（窗口内事件，过滤后总数）。事件流为时间
/// 升序；窗口从最新端取 `[n-offset-limit, n-offset)`，窗内保持升序——
/// offset 即向更早翻页，limit 为窗口大小（clamp 1 至 MAX_LIMIT）。
pub fn apply_filter_counted(
    events: Vec<TraceEvent>,
    filter: &TraceFilter,
) -> (Vec<TraceEvent>, usize) {
    let out: Vec<TraceEvent> = events
        .into_iter()
        .filter(|e| match filter.agent {
            Some(a) => e.agent == a,
            None => true,
        })
        .filter(|e| match filter.file_glob {
            Some(g) => e
                .file
                .as_deref()
                .map(|f| file_matches(f, g))
                .unwrap_or(false),
            None => true,
        })
        .collect();
    let n = out.len();
    let end = n.saturating_sub(filter.offset);
    let start = end.saturating_sub(filter.clamp_limit());
    let window: Vec<TraceEvent> = out.into_iter().skip(start).take(end - start).collect();
    (window, n)
}

/// trace 六视图的文件过滤面（细则见 R002 与模块文档）。
pub fn file_matches(file: &str, pattern: &str) -> bool {
    if let Ok(g) = glob::Pattern::new(pattern) {
        return g.matches(file);
    }
    file.contains(pattern)
}

/// trace 六视图的检索面（细则见 R002 与模块文档）。
pub fn search_matches(event: &TraceEvent, query: &str) -> bool {
    let re = regex::Regex::new(query).ok();
    let fields = [
        event.patch.as_deref().unwrap_or(""),
        event.file.as_deref().unwrap_or(""),
        event.user_intent.as_deref().unwrap_or(""),
        event.op_intent.as_deref().unwrap_or(""),
    ];
    match &re {
        Some(re) => fields.iter().any(|f| re.is_match(f)),
        None => fields.iter().any(|f| f.contains(query)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("trace")
            .join(name)
    }

    #[test]
    fn filter_offset_windows_do_not_overlap_and_count_total() {
        // D26：窗口从最新端取，offset 向更早翻页；total 是过滤后全量。
        let mk = |i: u64| TraceEvent {
            agent: "claude".into(),
            session_id: format!("s{i}"),
            call_id: Some(format!("c{i}")),
            tool: Some("Edit".into()),
            file: Some(format!("src/a{i}.rs")),
            kind: EditKind::Modify,
            user_intent: None,
            op_intent: None,
            ts: Some(format!("2026-08-31T00:00:{i:02}Z")),
            ts_ms: Some(1_787_407_600_000 + i * 1000),
            patch: None,
        };
        let events: Vec<TraceEvent> = (0..10).map(mk).collect();
        let f = |limit: usize, offset: usize| TraceFilter {
            agent: None,
            file_glob: None,
            limit,
            offset,
        };
        let (p1, total) = apply_filter_counted(events.clone(), &f(3, 0));
        assert_eq!(total, 10);
        assert_eq!(p1.len(), 3);
        assert_eq!(
            p1[0].call_id.as_deref(),
            Some("c7"),
            "窗口=最新 3 条，ASC 展示"
        );
        let (p2, _) = apply_filter_counted(events.clone(), &f(3, 3));
        let ids: Vec<_> = p2.iter().filter_map(|e| e.call_id.clone()).collect();
        assert_eq!(
            ids,
            vec!["c4".to_string(), "c5".to_string(), "c6".to_string()]
        );
        assert!(!p1.iter().any(|e| p2.iter().any(|x| x.call_id == e.call_id)));
        // 越界 offset 安全：空窗不减。
        let (p9, _) = apply_filter_counted(events.clone(), &f(3, 100));
        assert!(p9.is_empty());
    }

    #[test]
    fn first_line_timestamp_skips_meta_lines() {
        // D26 #5：claude jsonl 首行是 mode 元数据（无 timestamp），扫描应
        // 前进到首个带 timestamp 的行；全无返回 None。
        let dir = std::env::temp_dir().join(format!("hst-trace-ts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("s1.jsonl");
        fs::write(
            &p,
            concat!(
                "{\"mode\":\"default\",\"sessionId\":\"s1\",\"type\":\"meta\"}\n",
                "{\"type\":\"user\",\"timestamp\":\"2026-09-04T12:02:35.633Z\"}\n"
            ),
        )
        .unwrap();
        assert_eq!(
            first_line_timestamp(&p).as_deref(),
            Some("2026-09-04T12:02:35.633Z")
        );
        let p2 = dir.join("s2.jsonl");
        fs::write(&p2, "{\"type\":\"meta\"}\n").unwrap();
        assert_eq!(first_line_timestamp(&p2), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn claude_slug_matches_observed_dirs() {
        assert_eq!(
            claude_project_slug(Path::new(r"D:\ohmyagents")),
            "D--ohmyagents"
        );
        assert_eq!(
            claude_project_slug(Path::new(r"C:\Users\ray")),
            "C--Users-ray"
        );
    }

    #[test]
    fn time_conversions_round_trip() {
        let ms = ts_to_ms("2026-08-31T09:24:32.851Z").expect("iso");
        assert!(ms > 1_700_000_000_000);
        assert_eq!(ms_to_iso(ms), "2026-08-31T09:24:32.851Z");
        assert_eq!(ts_to_ms("1787407580274"), Some(1_787_407_580_274));
        assert_eq!(ts_to_ms("not-a-time"), None);
    }

    #[test]
    fn grok_uuid_v7_ms_extracts_unix_time() {
        let ms = grok_uuid_v7_ms("01a04d17-eb80-72b3-93e8-7988431e5f8c").expect("v7");
        assert!(ms > 1_700_000_000_000 && ms < 2_000_000_000_000, "{ms}");
        assert_eq!(grok_uuid_v7_ms("not-a-uuid"), None);
    }

    #[test]
    fn claude_events_extract_edit_with_dual_intent() {
        let dir = fixture("claude");
        let sessions = claude_sessions_in(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        let events = claude_events(&sessions[0]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.agent, "claude");
        assert_eq!(e.tool.as_deref(), Some("Edit"));
        assert!(e.file.as_deref().unwrap().ends_with("src/lib.rs"));
        assert_eq!(e.kind, EditKind::Modify);
        assert_eq!(e.user_intent.as_deref(), Some("把 greet 改成中文"));
        assert_eq!(e.op_intent.as_deref(), Some("我来修改 greet 函数返回中文"));
        assert!(e.call_id.as_deref().unwrap().starts_with("call_"));
        assert!(e.ts.as_deref().unwrap().starts_with("2026-"));
        assert!(e.ts_ms.is_some());
        assert!(e.patch.as_deref().unwrap().contains("你好"));
        assert!(e.operation_id().contains(":call_"));
    }

    #[test]
    fn codex_events_prefer_filechange_over_custom_tool_call() {
        let dir = fixture("codex");
        let mut sessions = codex_sessions_under(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 2);
        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        // A：有 FileChange item_completed → 主源（changes 绝对路径 + add/update 双键）。
        // serde_json 的 map 按键字典序迭代，不假设文件顺序。
        let events = codex_events(&sessions[0]);
        assert_eq!(events.len(), 2);
        let by_file = |f: &str| {
            events
                .iter()
                .find(|e| e.file.as_deref() == Some(f))
                .unwrap()
        };
        assert_eq!(by_file("notes.md").kind, EditKind::Create);
        assert_eq!(by_file("README.md").kind, EditKind::Modify);
        assert!(by_file("notes.md")
            .patch
            .as_deref()
            .unwrap()
            .contains("hello"));
        for e in &events {
            assert_eq!(e.tool.as_deref(), Some("apply_patch"));
            assert!(e.call_id.as_deref().unwrap().starts_with("call_"));
            assert!(e.ts_ms.is_some());
            assert_eq!(
                e.user_intent.as_deref(),
                Some("加一个笔记文件并更新 README")
            );
        }
        // B：无 FileChange（旧版形状）→ 退回 custom_tool_call 补丁头解析。
        let events = codex_events(&sessions[1]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EditKind::Delete);
        assert!(events[0].file.as_deref().unwrap().ends_with("old.txt"));
    }

    #[test]
    fn apply_patch_parser_handles_three_kinds() {
        let patch = "*** Begin Patch\n*** Add File: a\\b.txt\n+x\n*** Update File: c/d.rs\n-y\n+z\n*** Delete File: e.txt\n*** End Patch\n";
        let parsed = parse_apply_patch(patch);
        assert_eq!(
            parsed,
            vec![
                (EditKind::Create, "a/b.txt".to_string()),
                (EditKind::Modify, "c/d.rs".to_string()),
                (EditKind::Delete, "e.txt".to_string()),
            ]
        );
    }

    #[test]
    fn codex_injected_context_is_recognized() {
        assert!(is_codex_injected_context(
            "# AGENTS.md instructions for D:\\x"
        ));
        assert!(is_codex_injected_context("<environment_context>"));
        assert!(is_codex_injected_context("<user_shell_command>"));
        assert!(!is_codex_injected_context("正常用户输入"));
    }

    #[test]
    fn grok_events_extract_search_replace_with_synthetic_filter() {
        let dir = fixture("grok");
        let sessions = grok_sessions_in(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        let events = grok_events(&sessions[0]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.tool.as_deref(), Some("search_replace"));
        assert!(e.file.as_deref().unwrap().ends_with("src/app.rs"));
        assert!(e.call_id.as_deref().unwrap().starts_with("call-"));
        // synthetic_reason 的注入 user 行不能污染用户意图。
        assert_eq!(e.user_intent.as_deref(), Some("把标题改成中文"));
        assert_eq!(e.op_intent.as_deref(), Some("修改 app.rs 的标题渲染"));
        assert!(e.ts_ms.is_some());
        assert!(e.patch.as_deref().unwrap().contains("你好"));
    }

    #[test]
    fn grok_sessions_prefer_updates_log_with_real_start() {
        // 有 updates.jsonl 的会话选权威日志，started_at 用首行信封秒时间戳。
        let dir = fixture("grok-updates");
        let sessions = grok_sessions_in(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0]
            .file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .ends_with("updates.jsonl"));
        // 1787999890 秒 = 2026-08-29T10:38:10Z（fixture 契约）。
        assert_eq!(
            sessions[0].started_at.as_deref(),
            Some("2026-08-29T10:38:10.000Z")
        );
    }

    #[test]
    fn grok_updates_events_use_kind_meta_and_real_times() {
        let dir = fixture("grok-updates");
        let sessions = grok_sessions_in(&dir, Path::new(r"D:\demo"));
        let events = grok_events(&sessions[0]);
        // read_file（kind=read）与 tool_call_update 不产事件；写族两条。
        assert_eq!(events.len(), 2);
        let edit = events
            .iter()
            .find(|e| e.tool.as_deref() == Some("search_replace"))
            .expect("search_replace event");
        assert_eq!(edit.file.as_deref(), Some("src/app.rs"));
        assert!(edit.call_id.as_deref().unwrap().ends_with("-1"));
        // hideFromScrollback 的 system-reminder 分片不能污染用户意图。
        assert_eq!(edit.user_intent.as_deref(), Some("把标题改成中文"));
        assert_eq!(
            edit.op_intent.as_deref(),
            Some("我来修改 app.rs 的标题渲染")
        );
        // 每事件真实时间：信封秒 * 1000。
        assert_eq!(edit.ts_ms, Some(1_787_999_900_000));
        assert!(edit
            .ts
            .as_deref()
            .unwrap()
            .starts_with("2026-08-29T10:38:20"));
        assert!(edit.patch.as_deref().unwrap().contains("你好"));
        let write = events
            .iter()
            .find(|e| e.tool.as_deref() == Some("write"))
            .expect("write event");
        assert!(write.file.as_deref().unwrap().ends_with("docs/notes.md"));
        assert!(write.patch.as_deref().unwrap().contains("# 笔记"));
        assert_eq!(write.ts_ms, Some(1_787_999_910_000));
    }

    #[test]
    fn kimi_events_extract_edit_with_origin_filter() {
        let dir = fixture("kimi");
        let index = dir.join("session_index.jsonl");
        let sessions = kimi_sessions_in(&index, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        let events = kimi_events(&sessions[0]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.tool.as_deref(), Some("Write"));
        assert!(e.file.as_deref().unwrap().ends_with("out.md"));
        assert!(e.call_id.as_deref().unwrap().starts_with("tool_"));
        // origin.kind != user 的 turn.prompt（系统触发）不能污染用户意图。
        assert_eq!(e.user_intent.as_deref(), Some("生成一个说明文件"));
        assert_eq!(e.op_intent.as_deref(), Some("我来写说明文件"));
        assert_eq!(e.ts_ms, Some(1_787_407_580_274));
    }

    #[test]
    fn kimi_index_tombstones_hide_sessions() {
        let dir = fixture("kimi");
        let index = dir.join("session_index_deleted.jsonl");
        let sessions = kimi_sessions_in(&index, Path::new(r"D:\demo"));
        assert!(sessions.is_empty(), "墓碑行应隐藏会话");
    }

    #[test]
    fn group_blocks_aggregates_multi_file_operation() {
        let dir = fixture("codex");
        let mut sessions = codex_sessions_under(&dir, Path::new(r"D:\demo"));
        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        let events = codex_events(&sessions[0]);
        let blocks = group_blocks(&events);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.edits, 2);
        let mut files = b.files.clone();
        files.sort();
        assert_eq!(files, vec!["README.md".to_string(), "notes.md".to_string()]);
        let mut kinds = b.kinds.clone();
        kinds.sort();
        assert_eq!(kinds, vec!["create".to_string(), "modify".to_string()]);
        assert_eq!(
            b.user_intent.as_deref(),
            Some("加一个笔记文件并更新 README")
        );
        assert!(b.op.ends_with(":call_xyz789"));
    }

    #[test]
    fn filter_and_search_clamp_and_match() {
        let mk = |i: usize| TraceEvent {
            agent: if i % 2 == 0 { "claude" } else { "codex" }.into(),
            session_id: format!("s{i}"),
            call_id: Some(format!("call_{i}")),
            tool: Some("Edit".into()),
            file: Some(format!("src/mod{i}.rs")),
            kind: EditKind::Modify,
            user_intent: Some(format!("任务{i}")),
            op_intent: None,
            ts: Some(format!("2026-08-31T00:00:{i:02}Z")),
            ts_ms: Some(1_787_407_600_000 + i as u64 * 1000),
            patch: Some(format!("fn v{i}() {{}}")),
        };
        let events: Vec<TraceEvent> = (0..10).map(mk).collect();
        let f = TraceFilter {
            agent: Some("claude"),
            file_glob: Some("src/*.rs"),
            limit: 999,
            offset: 0,
        };
        let got = apply_filter(events.clone(), &f);
        assert!(got.iter().all(|e| e.agent == "claude"));
        assert_eq!(got.len(), 5);
        let f = TraceFilter {
            agent: None,
            file_glob: None,
            limit: 3,
            offset: 0,
        };
        assert_eq!(apply_filter(events.clone(), &f).len(), 3);
        // 正则与字面退路。
        let re_hit = events.iter().find(|e| search_matches(e, r"fn v3")).unwrap();
        assert_eq!(re_hit.session_id, "s3");
        // "v3(" 是非法正则（未闭合分组），退字面子串仍命中 patch 里的 "fn v3() {}"。
        assert!(events.iter().any(|e| search_matches(e, "v3(")));
        assert_eq!(f.clamp_limit(), 3);
    }
}
