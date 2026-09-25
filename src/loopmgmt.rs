//! loop 与 goal 管理面（REQ-019、REQ-021 类型契约化试点）。hst 侧唯一
//! 真相 = 项目根 `.claude/scheduled_tasks.json`（Claude Code durable
//! CronCreate 的同源落盘形，实证 2026-09-23 探针：tasks[] 含 id / cron /
//! prompt / recurring / createdAt / createdBySessionId / createdByPid /
//! createdByProcStart）；hst 是该文件的读写管理面（set / list / del），
//! 状态栏 loop 与 goal 段同源只读消费，不自建第二状态文件。会话归属
//! 判据 = `createdBySessionId` 与当前会话 id 等值。goal 语义 = 任务
//! `prompt` 文本。并发面与 Claude Code 同款读改写（无锁，后写覆盖先写）；
//! 同一会话内混用 hst 与 agent 原生 cron 工具会互相覆盖盘上变更，管理
//! 纪律是单向（要么全经 hst，要么全经会话内工具）。已在跑会话不接管
//! 盘上外部写入（2026-09-26 探针实证阴性：agent 进程 inotify 盯着项目
//! .claude 目录但任务变更不被采纳，会话运行期调度器状态在内存注册表，盘上
//! 文件是重启装载的持久化镜像而非活输入通道）。重启装载面同日实证
//! 阳性：新会话启动装载本文件，missed 一次性任务以补触发形浮出（带
//! 用户确认门，prompt 即 goal 文本）并自删；hst 写盘后重开会话即生效。
//!
//! 类型契约（REQ-021）：节拍互斥由 Cadence 枚举在类型层表达（双缺
//! 或双给的检查上移 CLI 解析层）；任务行由 Task 结构体 typed 承载
//! （缺键容忍默认、错型响亮报错、未知键 flatten 保真）；错误由
//! LoopError 枚举承载（LoopError::code 给 agent 稳定短码，CLI 错误面
//! 「loop error=<code>:」前缀即 agent 判别契约；结构化短码字段面候裁
//! REQ-011，评审 G3 回填）。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};

use crate::pathutil::{forward_slash, user_home};

/// 节拍规格（REQ-021）：`--every` 周期形与 `--at` 一次性形的互斥枚举，
/// 非法组合（双缺或双给）在类型层不可表示，检查归 CLI 解析层。
pub enum Cadence<'a> {
    /// 周期形 `Nm`（1 至 59 分）或 `Nh`（1 至 23 小时）。
    Every(&'a str),
    /// 一次性形 `HH:MM`（24 小时制）。
    At(&'a str),
}

/// loop 与 goal 面错误（REQ-021）：`Display` 面向人，[`LoopError::code`]
/// 给 agent 稳定短码（机器判别不靠字符串前缀分流）。
#[derive(Debug)]
pub enum LoopError {
    /// goal 文本 trim 后为空。
    EmptyGoal,
    /// goal 文本 trim 后为空（goal 改写面专用；Display 带 `hst goal
    /// clear` 指引尾巴，评审 G1 回填：原 set_goal 可操作文案不丢）。
    EmptyGoalClear,
    /// `--every` 形坏或越界（携带原文）。
    BadEvery(String),
    /// `--at` 时刻形坏或越界（携带原文）。
    BadAt(String),
    /// 当前会话三级解析全落空（携带解析链说明）。
    SessionUnresolved(String),
    /// 本会话无任务（goal 改写面前置缺失）。
    NoOwnTask,
    /// 文件在场但不可解析：坏 JSON 或任务字段错型（携带路径与原因）。
    Corrupt {
        /// 坏损文件绝对路径。
        path: String,
        /// 解析失败原因。
        cause: String,
    },
    /// 目录创建或写盘失败（携带路径与原因）。
    Io {
        /// 失败目标绝对路径。
        path: String,
        /// IO 失败原因。
        cause: String,
    },
}

impl LoopError {
    /// agent 面稳定短码（CLI 错误输出 `error=<code>` 的取值源）。
    pub fn code(&self) -> &'static str {
        match self {
            LoopError::EmptyGoal | LoopError::EmptyGoalClear => "empty_goal",
            LoopError::BadEvery(_) => "bad_every",
            LoopError::BadAt(_) => "bad_at",
            LoopError::SessionUnresolved(_) => "no_session",
            LoopError::NoOwnTask => "no_own_task",
            LoopError::Corrupt { .. } => "corrupt",
            LoopError::Io { .. } => "io",
        }
    }
}

impl std::fmt::Display for LoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopError::EmptyGoal => write!(f, "goal text is empty"),
            LoopError::EmptyGoalClear => write!(
                f,
                "goal text is empty (to remove the goal use: hst goal clear)"
            ),
            LoopError::BadEvery(spec) => {
                write!(f, "invalid --every (expect Nm 1-59 or Nh 1-23): {spec}")
            }
            LoopError::BadAt(spec) => write!(f, "invalid --at (expect HH:MM 24h): {spec}"),
            LoopError::SessionUnresolved(msg) => write!(f, "{msg}"),
            LoopError::NoOwnTask => {
                write!(
                    f,
                    "no loop task for current session (create one with: hst loop set)"
                )
            }
            LoopError::Corrupt { path, cause } => write!(f, "{path}: {cause}"),
            LoopError::Io { path, cause } => write!(f, "{path}: {cause}"),
        }
    }
}

/// scheduled_tasks.json 的任务行（REQ-021 typed 承载）：核心八字段加
/// flatten 未知键保真。五字段（createdAt/recurring/createdBy 三件）
/// `#[serde(default)]` 容忍缺键（历史文件可能短字段；外会话行经 hst
/// 写盘会补全默认值，属接受的可见变化，评审 G5 回填）；id/cron/prompt
/// 三必需字段缺失或错型均响亮报错（缺 id 会让 del 定位失准，不给默认，
/// 评审 F1 回填）。键序不承诺与原生落盘形同序（hst 写盘按结构体字段序
/// 重排、未知键居末，JSON 消费方不受影响，评审 G4 回填）；文件属主是
/// Claude Code，schema 可能长，未知键读改写幸存。
#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    /// 任务 id。
    pub id: String,
    /// cron 表达式（5 字段）。
    pub cron: String,
    /// goal 文本（prompt 原文，未截断）。
    pub prompt: String,
    /// 创建时刻（epoch ms）。
    #[serde(rename = "createdAt", default)]
    pub created_at: u64,
    /// 是否周期形（一次性 --at 形为 false）。
    #[serde(default)]
    pub recurring: bool,
    /// 创建会话 id。
    #[serde(rename = "createdBySessionId", default)]
    pub created_by_session_id: String,
    /// 属主进程 pid（会话外跑为 0）。
    #[serde(rename = "createdByPid", default)]
    pub created_by_pid: u64,
    /// 属主进程启动刻（Linux /proc 第 22 字段；非 Linux 或读不到为空）。
    #[serde(rename = "createdByProcStart", default)]
    pub created_by_proc_start: String,
    /// 未知键保真（flatten；读改写幸存，不自建第二 schema 真相）。
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Json>,
}

/// set 落盘报告：新任务的 id、cron、recurring、归属会话与文件路径。
pub struct SetReport {
    /// 新任务 id（8 位十六进制）。
    pub id: String,
    /// cron 表达式。
    pub cron: String,
    /// 是否周期形（一次性 --at 形为 false）。
    pub recurring: bool,
    /// 归属会话 id。
    pub session: String,
    /// 落盘文件绝对路径。
    pub file: PathBuf,
}

/// 任务行（list / del 判据用）：文件内每条任务的展开视图。
#[derive(Debug)]
pub struct TaskRow {
    /// 任务 id。
    pub id: String,
    /// cron 表达式。
    pub cron: String,
    /// 是否周期形。
    pub recurring: bool,
    /// 创建会话 id。
    pub session: String,
    /// goal 文本（prompt 原文，未截断）。
    pub goal: String,
    /// 是否当前会话的任务（会话不可解析时恒 false）。
    pub ours: bool,
}

/// 定时任务文件路径（项目根下 `.claude/scheduled_tasks.json`）。
pub fn scheduled_tasks_path(root: &Path) -> PathBuf {
    root.join(".claude").join("scheduled_tasks.json")
}

fn read_tasks(root: &Path) -> Result<Vec<Task>, LoopError> {
    let path = scheduled_tasks_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    // 复用 yolo 面的 BOM 容忍读；解析失败硬错上抛，绝不覆写真数据。
    let v = crate::yolo::read_json(&path).map_err(|e| LoopError::Corrupt {
        path: path.display().to_string(),
        cause: e,
    })?;
    // 评审二轮 G8 回填：顶层非对象（如裸数组）= 真数据在场，硬错绝不
    // 静默当无 tasks 处理（下一次写盘会整段重置掉它）。
    if !v.is_object() {
        return Err(LoopError::Corrupt {
            path: path.display().to_string(),
            cause: "top-level is not an object (refusing to overwrite real data)".to_string(),
        });
    }
    let arr = match v.get("tasks") {
        None => return Ok(Vec::new()),
        // 评审 F2 回填：tasks 键在场而非数组 = 真数据在场，硬错绝不
        // 覆写（静默当空集会让下一次写盘整段盖掉它）。
        Some(t) => t.as_array().cloned().ok_or_else(|| LoopError::Corrupt {
            path: path.display().to_string(),
            cause: "tasks key is present but not an array (refusing to overwrite \
                    real data)"
                .to_string(),
        })?,
    };
    arr.iter()
        .enumerate()
        .map(|(i, t)| {
            // 评审 F1 加 G2 回填：必需字段缺失或错型统一文案响亮报错，
            // 点名行下标供多行文件定位。
            serde_json::from_value(t.clone()).map_err(|e| LoopError::Corrupt {
                path: path.display().to_string(),
                cause: format!("bad task row at tasks[{i}]: {e}"),
            })
        })
        .collect()
}

fn write_tasks(root: &Path, tasks: &[Task]) -> Result<PathBuf, LoopError> {
    let path = scheduled_tasks_path(root);
    // 评审 G1：读改写保顶层兄弟键（该文件属主是 Claude Code，schema 可能
    // 长，只换 tasks 键，与 apply_pct 对 settings 的纪律同款）。
    let mut v = if path.exists() {
        crate::yolo::read_json(&path).map_err(|e| LoopError::Corrupt {
            path: path.display().to_string(),
            cause: e,
        })?
    } else {
        json!({})
    };
    // 评审二轮 G8 回填：顶层非对象由读侧硬错拦截，写侧不再静默重置
    //（原 `v = json!({})` 会把 [1,2,3] 类真数据整段盖掉）。
    if !v.is_object() {
        return Err(LoopError::Corrupt {
            path: path.display().to_string(),
            cause: "top-level is not an object (refusing to overwrite real data)".to_string(),
        });
    }
    let rows: Vec<Json> = tasks
        .iter()
        .map(|t| {
            serde_json::to_value(t).map_err(|e| LoopError::Corrupt {
                path: path.display().to_string(),
                cause: format!("serialize task: {e}"),
            })
        })
        .collect::<Result<_, _>>()?;
    v["tasks"] = Json::Array(rows);
    let text = serde_json::to_string_pretty(&v).map_err(|e| LoopError::Corrupt {
        path: path.display().to_string(),
        cause: format!("serialize file: {e}"),
    })? + "\n";
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| LoopError::Io {
            path: dir.display().to_string(),
            cause: e.to_string(),
        })?;
    }
    std::fs::write(&path, text).map_err(|e| LoopError::Io {
        path: path.display().to_string(),
        cause: e.to_string(),
    })?;
    Ok(path)
}

/// 解析「当前会话」：显式 `--session` 优先，其次 agent 会话内环境变量
/// `CLAUDE_CODE_SESSION_ID`，再次 `~/.claude.json` 项目表 `lastSessionId`
/// （键 = 项目根正斜杠形，与 pretrust 面同判）；全落空报错并提示显式给。
///
/// # Errors
///
/// 三级解析全落空时返回 [`LoopError::SessionUnresolved`]（提示显式给
/// --session）。
pub fn resolve_session(explicit: Option<&str>, root: &Path) -> Result<String, LoopError> {
    if let Some(s) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(s.to_string());
    }
    if let Some(v) = std::env::var("CLAUDE_CODE_SESSION_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return Ok(v.trim().to_string());
    }
    let key = forward_slash(root);
    let cj = user_home()
        .map_err(|e| LoopError::Io {
            path: "user home".into(),
            cause: e,
        })?
        .join(".claude.json");
    if cj.exists() {
        let v = crate::yolo::read_json(&cj).map_err(|e| LoopError::Corrupt {
            path: cj.display().to_string(),
            cause: e,
        })?;
        if let Some(sid) = v
            .get("projects")
            .and_then(|p| p.get(&key))
            .and_then(|p| p.get("lastSessionId"))
            .and_then(|s| s.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            return Ok(sid.trim().to_string());
        }
    }
    Err(LoopError::SessionUnresolved(format!(
        "cannot resolve current session (no --session, no CLAUDE_CODE_SESSION_ID, no \
         ~/.claude.json projects[{key}].lastSessionId); pass --session <id> explicitly"
    )))
}

/// 属主进程(pid, procStart)：会话内跑时取 CLAUDE_PID（agent 本体），
/// 否则 pid=0（归属未知）；procStart 读 Linux `/proc/<pid>/stat` 第 22
/// 字段（Claude Code liveness 同源形），非 Linux 或读不到留空。
fn owner_proc() -> (u64, String) {
    let pid: u64 = std::env::var("CLAUDE_PID")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let mut start = String::new();
    if pid > 0 && cfg!(target_os = "linux") {
        if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            // comm 字段可含空格括号，取末个 ')' 之后 token 19（全行第 22 字段）。
            if let Some(rest) = stat.rsplit(')').next() {
                if let Some(tok) = rest.split_whitespace().nth(19) {
                    start = tok.to_string();
                }
            }
        }
    }
    (pid, start)
}

static ID_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn new_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = ID_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let digest = Sha256::digest(format!("{n}-{seq}-{}", std::process::id()).as_bytes());
    format!("{digest:x}")[..8].to_string()
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 周期间隔转 cron：`Nm`（1 至 59）得 `*/N * * * *`；`Nh`（1 至 23）得
/// `M */N * * *`（评审 F2：N 必须进小时步进位，否则 6h 静默放大成每小
/// 时），M 取落盘时刻分钟位并避开 0 与 30（舰队避整点半点纪律）。恒为
/// recurring 形（由 [`Cadence::Every`] 变体决定，不再以布尔元组携带）。
///
/// # Errors
///
/// 形坏或数值越界时返回 [`LoopError::BadEvery`]。
pub fn every_to_cron(every: &str) -> Result<String, LoopError> {
    let spec = every.trim();
    if let Some(n) = spec.strip_suffix('m').and_then(|d| d.parse::<u32>().ok()) {
        if (1..=59).contains(&n) {
            return Ok(format!("*/{n} * * * *"));
        }
        return Err(LoopError::BadEvery(spec.to_string()));
    }
    if let Some(n) = spec.strip_suffix('h').and_then(|d| d.parse::<u32>().ok()) {
        if (1..=23).contains(&n) {
            let mut m = unix_millis() / 60_000 % 60;
            if m == 0 {
                m = 7;
            } else if m == 30 {
                m = 37;
            }
            return Ok(format!("{m} */{n} * * *"));
        }
        return Err(LoopError::BadEvery(spec.to_string()));
    }
    Err(LoopError::BadEvery(spec.to_string()))
}

/// 一次性时刻转 cron：`HH:MM`（24 小时制）得 `M H * * *`（到点触发后由
/// 运行时自动删除；非 recurring 形由 [`Cadence::At`] 变体决定）。
///
/// # Errors
///
/// 时刻形坏或越界时返回 [`LoopError::BadAt`]。
pub fn at_to_cron(at: &str) -> Result<String, LoopError> {
    let spec = at.trim();
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (
            parts[0].trim().parse::<u32>(),
            parts[1].trim().parse::<u32>(),
        ) {
            if h < 24 && m < 60 {
                return Ok(format!("{m} {h} * * *"));
            }
        }
    }
    Err(LoopError::BadAt(spec.to_string()))
}

/// 设置当前会话 loop：节拍由 [`Cadence`] 枚举承载（互斥在类型层成立），
/// goal 文本 trim 后非空；读改写保留既有任务（含外会话与 agent 原生
/// 落盘项）。
///
/// # Errors
///
/// goal 空、节拍形坏或越界、会话不可解析、文件坏损或 IO 失败时返回
/// [`LoopError`]。
pub fn set_loop(
    root: &Path,
    goal: &str,
    cadence: Cadence<'_>,
    session: Option<&str>,
) -> Result<SetReport, LoopError> {
    let goal = goal.trim();
    if goal.is_empty() {
        return Err(LoopError::EmptyGoal);
    }
    let cron = match cadence {
        Cadence::Every(e) => every_to_cron(e)?,
        Cadence::At(a) => at_to_cron(a)?,
    };
    let recurring = matches!(cadence, Cadence::Every(_));
    let session = resolve_session(session, root)?;
    let (pid, proc_start) = owner_proc();
    let id = new_id();
    let task = Task {
        id: id.clone(),
        cron: cron.clone(),
        prompt: goal.to_string(),
        created_at: unix_millis(),
        recurring,
        created_by_session_id: session.clone(),
        created_by_pid: pid,
        created_by_proc_start: proc_start,
        extra: serde_json::Map::new(),
    };
    let mut tasks = read_tasks(root)?;
    tasks.push(task);
    let file = write_tasks(root, &tasks)?;
    Ok(SetReport {
        id,
        cron,
        recurring,
        session,
        file,
    })
}

fn task_row(t: &Task, session: &Option<String>) -> TaskRow {
    TaskRow {
        id: t.id.clone(),
        cron: t.cron.clone(),
        recurring: t.recurring,
        session: t.created_by_session_id.clone(),
        goal: t.prompt.clone(),
        ours: session
            .as_deref()
            .is_some_and(|s| *s == t.created_by_session_id),
    }
}

/// 列出项目定时任务（全量，不按会话过滤；`ours` 标当前会话归属，
/// 会话解析失败时返回 None 且 ours 恒 false，列表本身不受阻）。
///
/// # Errors
///
/// 文件坏损（含任务字段错型）或 IO 失败时返回 [`LoopError`]（会话解析
/// 失败不算错）。
pub fn list_tasks(
    root: &Path,
    session: Option<&str>,
) -> Result<(Vec<TaskRow>, Option<String>), LoopError> {
    let resolved = resolve_session(session, root).ok();
    let tasks = read_tasks(root)?;
    Ok((
        tasks.iter().map(|t| task_row(t, &resolved)).collect(),
        resolved,
    ))
}

/// goal 改写报告：被改任务的 id 与 cron。
#[derive(Debug)]
pub struct GoalReport {
    /// 被改任务 id。
    pub id: String,
    /// 被改任务 cron（goal 面不动节奏，原样回显）。
    pub cron: String,
}

/// 本会话 createdAt 最新任务的下标（同刻取后入者，与 del latest 同判）。
fn latest_own_index(tasks: &[Task], sid: &str) -> Option<usize> {
    tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.created_by_session_id == sid)
        .max_by_key(|(i, t)| (t.created_at, *i))
        .map(|(i, _)| i)
}

/// 设置当前会话最新 loop 的 goal 文本（prompt 就地改写，节奏与属主不动；
/// 无本会话任务报错，先 `hst loop set` 建任务）。
///
/// # Errors
///
/// 文本空、无本会话任务、文件坏损或 IO 失败时返回 [`LoopError`]。
pub fn set_goal(root: &Path, text: &str, session: Option<&str>) -> Result<GoalReport, LoopError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(LoopError::EmptyGoalClear);
    }
    let sid = resolve_session(session, root)?;
    let mut tasks = read_tasks(root)?;
    let idx = latest_own_index(&tasks, &sid).ok_or(LoopError::NoOwnTask)?;
    let report = GoalReport {
        id: tasks[idx].id.clone(),
        cron: tasks[idx].cron.clone(),
    };
    tasks[idx].prompt = text.to_string();
    write_tasks(root, &tasks)?;
    Ok(report)
}

/// 清空当前会话最新 loop 的 goal（prompt 置空、任务与节奏保留；返回被改
/// 任务 id，无本会话任务返回 None）。
///
/// # Errors
///
/// 会话不可解析、文件坏损或 IO 失败时返回 [`LoopError`]。
pub fn clear_goal(root: &Path, session: Option<&str>) -> Result<Option<String>, LoopError> {
    let sid = resolve_session(session, root)?;
    let mut tasks = read_tasks(root)?;
    match latest_own_index(&tasks, &sid) {
        None => Ok(None),
        Some(idx) => {
            let id = tasks[idx].id.clone();
            tasks[idx].prompt = String::new();
            write_tasks(root, &tasks)?;
            Ok(Some(id))
        }
    }
}

/// 查看当前会话最新 loop 的 goal（会话不可解析或无任务返回 None，查看面
/// 不受阻）。
///
/// # Errors
///
/// 文件坏损（含任务字段错型）或 IO 失败时返回 [`LoopError`]（会话不可
/// 解析返回 None 不算错）。
pub fn show_goal(root: &Path, session: Option<&str>) -> Result<Option<TaskRow>, LoopError> {
    let sid = match resolve_session(session, root) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let tasks = read_tasks(root)?;
    Ok(latest_own_index(&tasks, &sid).map(|i| task_row(&tasks[i], &Some(sid))))
}

/// 删除任务：target 为任务 id（精确匹配任意任务，不限会话）、`latest`
/// （本会话 createdAt 最新一条）或 `all`（本会话全部）；latest / all
/// 需要可解析的当前会话。返回被删 id 清单（空 = 零改动，合法回执）。
///
/// # Errors
///
/// latest/all 会话不可解析、文件坏损或 IO 失败时返回 [`LoopError`]。
pub fn del_loops(
    root: &Path,
    target: &str,
    session: Option<&str>,
) -> Result<Vec<String>, LoopError> {
    let tasks = read_tasks(root)?;
    let spec = target.trim();
    // 评审二轮 G4 遗留：drop 下标集单源驱动 removed 与 kept（值等值反推
    // 在字节相同的重复任务上会失真，评审实证两同刻任务 del latest 静默
    // 无操作）。
    let drop_idx: std::collections::HashSet<usize> =
        if spec.eq_ignore_ascii_case("all") || spec.eq_ignore_ascii_case("latest") {
            let sid = resolve_session(session, root)?;
            let mine: Vec<(usize, u64)> = tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| t.created_by_session_id == sid)
                .map(|(i, t)| (i, t.created_at))
                .collect();
            if mine.is_empty() {
                return Ok(Vec::new());
            }
            if spec.eq_ignore_ascii_case("latest") {
                // max_by_key 同刻取后入者（与 latest_own_index 同判）。
                let Some((drop, _)) = mine.iter().max_by_key(|(_, c)| *c) else {
                    return Ok(Vec::new());
                };
                std::iter::once(*drop).collect()
            } else {
                mine.iter().map(|(i, _)| *i).collect()
            }
        } else {
            tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| t.id == spec)
                .map(|(i, _)| i)
                .collect()
        };
    let removed: Vec<String> = tasks
        .iter()
        .enumerate()
        .filter(|(i, _)| drop_idx.contains(i))
        .map(|(_, t)| t.id.clone())
        .collect();
    if !removed.is_empty() {
        let kept: Vec<Task> = tasks
            .iter()
            .enumerate()
            .filter(|(i, _)| !drop_idx.contains(i))
            .map(|(_, t)| t.clone())
            .collect();
        write_tasks(root, &kept)?;
    }
    Ok(removed)
}

/// doctor 消费的 loop 面健康快照（REQ-020）：计数面；坏损走 Err 不在
/// 快照里表达。
pub struct LoopHealth {
    /// 任务总数（文件缺失或空 tasks 为 0，零任务合法态）。
    pub total: usize,
    /// 本会话任务数（会话不可解析为 None，doctor 注记不告警）。
    pub ours: Option<usize>,
    /// 死属主任务数（pid 非零且进程不在或 procStart 不匹配）。
    pub dead_owner: usize,
    /// 是否做了属主活性判（仅 Linux；false 时 doctor 该子面降 ok 加
    /// 无判据注记，Status 模型无 info 档，同 compact 信息型先例）。
    pub owner_check: bool,
}

/// loop 面健康快照（REQ-020）：读项目 scheduled_tasks.json 计总数、本
/// 会话数与死属主；文件缺失或空任务合法返回零计数；坏损（顶层非对象、
/// tasks 键非数组、任务行缺必需字段或错型）返回 Err 带 LoopError 文案。
///
/// # Errors
///
/// 文件坏损或 IO 失败时返回 `String` 错误（LoopError 的 Display 形）。
pub fn health(root: &Path) -> Result<LoopHealth, String> {
    let tasks = read_tasks(root).map_err(|e| e.to_string())?;
    let owner_check = cfg!(target_os = "linux");
    let dead_owner = if owner_check {
        tasks
            .iter()
            .filter(|t| !owner_alive(t.created_by_pid, &t.created_by_proc_start))
            .count()
    } else {
        0
    };
    let ours = resolve_session(None, root).ok().map(|sid| {
        tasks
            .iter()
            .filter(|t| t.created_by_session_id == sid)
            .count()
    });
    Ok(LoopHealth {
        total: tasks.len(),
        ours,
        dead_owner,
        owner_check,
    })
}

/// 属主活性判（liveness 同源 /proc 第 22 字段）：pid 为 0 = 归属未知不
/// 算死；进程不在算死；procStart 落盘非空且与活动进程第 22 字段不等
/// （pid 复用）算死；procStart 落盘为空只查进程在否。非 Linux 恒活
///（调用方以 owner_check 分流该子面）。
fn owner_alive(pid: u64, proc_start: &str) -> bool {
    if pid == 0 || !cfg!(target_os = "linux") {
        return true;
    }
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let Some(rest) = stat.rsplit(')').next() else {
        return false;
    };
    match rest.split_whitespace().nth(19) {
        Some(tok) => proc_start.is_empty() || tok == proc_start,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-loop-test-{}-{tag}-{:?}",
            std::process::id(),
            unix_millis()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// 测试夹具：json 形构造再 typed 解析（缺键走 serde default）。
    fn jt(v: Json) -> Task {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn set_list_del_roundtrip_preserves_foreign() {
        let root = fresh_dir("rt");
        // 外会话既有任务先行落盘（模拟 agent 原生写入）;顶层带外来兄弟键
        // version（评审 G1：读改写必须保留）。
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        std::fs::write(
            root.join(".claude").join("scheduled_tasks.json"),
            r#"{"version":7,"tasks":[{"id":"foreign01","cron":"*/9 * * * *","prompt":"外会话任务","createdAt":1,"recurring":true,"createdBySessionId":"other-session","createdByPid":1,"createdByProcStart":"1"}]}"#,
        )
        .unwrap();
        let r1 = set_loop(&root, "盯CI发布", Cadence::Every("5m"), Some("s1")).unwrap();
        assert_eq!(r1.cron, "*/5 * * * *");
        assert!(r1.recurring);
        assert_eq!(r1.session, "s1");
        assert_eq!(r1.id.len(), 8);
        let r2 = set_loop(&root, "十四点半提醒", Cadence::At("14:30"), Some("s1")).unwrap();
        assert_eq!(r2.cron, "30 14 * * *");
        assert!(!r2.recurring);
        // G1：顶层兄弟键在读写后幸存。
        let after: Json = serde_json::from_str(
            &std::fs::read_to_string(root.join(".claude").join("scheduled_tasks.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(after["version"], 7, "top-level siblings survive");
        // list：三行,ours 两行。
        let (rows, sid) = list_tasks(&root, Some("s1")).unwrap();
        assert_eq!(sid.as_deref(), Some("s1"));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|r| r.ours).count(), 2);
        assert!(rows.iter().any(|r| r.goal == "外会话任务" && !r.ours));
        // del all：本会话两条删,外会话保留。
        let removed = del_loops(&root, "all", Some("s1")).unwrap();
        assert_eq!(removed.len(), 2);
        let (rows2, _) = list_tasks(&root, Some("s1")).unwrap();
        assert_eq!(rows2.len(), 1);
        assert_eq!(rows2[0].id, "foreign01");
        // 再 del all：零改动合法。
        assert!(del_loops(&root, "all", Some("s1")).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn latest_targets_newest_own_task_only() {
        let root = fresh_dir("lt");
        write_tasks(
            &root,
            &[
                jt(
                    json!({ "id": "old000001", "cron": "*/5 * * * *", "prompt": "旧",
                       "createdAt": 100u64, "recurring": true, "createdBySessionId": "s1" }),
                ),
                jt(
                    json!({ "id": "new000002", "cron": "*/7 * * * *", "prompt": "新",
                       "createdAt": 200u64, "recurring": true, "createdBySessionId": "s1" }),
                ),
                jt(
                    json!({ "id": "fgn000003", "cron": "*/9 * * * *", "prompt": "外",
                       "createdAt": 300u64, "recurring": true, "createdBySessionId": "s2" }),
                ),
            ],
        )
        .unwrap();
        let removed = del_loops(&root, "latest", Some("s1")).unwrap();
        assert_eq!(removed, vec!["new000002".to_string()]);
        let (rows, _) = list_tasks(&root, Some("s1")).unwrap();
        assert_eq!(rows.len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_and_at_validation() {
        assert_eq!(every_to_cron("5m").unwrap(), "*/5 * * * *");
        let hc = every_to_cron("2h").unwrap();
        assert!(hc.contains("*/2"), "hour step must carry N: {hc}");
        assert_eq!(hc.split(' ').count(), 5);
        assert!(every_to_cron("60m").is_err());
        assert!(every_to_cron("24h").is_err());
        assert!(every_to_cron("abc").is_err());
        assert_eq!(at_to_cron("09:05").unwrap(), "5 9 * * *");
        assert!(at_to_cron("24:00").is_err());
        assert!(at_to_cron("9:5:30").is_err());
        // REQ-021：错误枚举带 agent 短码。
        assert_eq!(every_to_cron("60m").unwrap_err().code(), "bad_every");
        assert_eq!(at_to_cron("24:00").unwrap_err().code(), "bad_at");
        let root = fresh_dir("val");
        assert!(set_loop(&root, "  ", Cadence::Every("5m"), Some("s1")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn goal_set_show_clear_roundtrip_on_latest_own() {
        let root = fresh_dir("gl");
        write_tasks(
            &root,
            &[
                jt(
                    json!({ "id": "old000001", "cron": "*/5 * * * *", "prompt": "旧",
                       "createdAt": 100u64, "recurring": true, "createdBySessionId": "s1" }),
                ),
                jt(
                    json!({ "id": "new000002", "cron": "*/7 * * * *", "prompt": "新",
                       "createdAt": 200u64, "recurring": true, "createdBySessionId": "s1" }),
                ),
                jt(
                    json!({ "id": "foreign01", "cron": "*/9 * * * *", "prompt": "外会话",
                       "createdAt": 300u64, "recurring": true, "createdBySessionId": "s2" }),
                ),
            ],
        )
        .unwrap();
        // set：改最新本会话任务,节奏不动,外会话不碰。
        let r = set_goal(&root, "改盯发布窗口", Some("s1")).unwrap();
        assert_eq!(r.id, "new000002");
        assert_eq!(r.cron, "*/7 * * * *");
        let (rows, _) = list_tasks(&root, Some("s1")).unwrap();
        assert!(rows
            .iter()
            .any(|t| t.id == "new000002" && t.goal == "改盯发布窗口"));
        assert!(rows
            .iter()
            .any(|t| t.id == "foreign01" && t.goal == "外会话"));
        // show 回读最新。
        let s = show_goal(&root, Some("s1")).unwrap().unwrap();
        assert_eq!(s.goal, "改盯发布窗口");
        // clear 置空 prompt、任务保留。
        let id = clear_goal(&root, Some("s1")).unwrap().unwrap();
        assert_eq!(id, "new000002");
        assert_eq!(show_goal(&root, Some("s1")).unwrap().unwrap().goal, "");
        // 会话不可解析:show 不受阻返回 None。
        assert!(show_goal(&root, Some("nope")).unwrap().is_none());
        // 无本会话任务 set 报错;空文本报错;clear 无任务 None。
        assert!(set_goal(&root, "x", Some("nope")).is_err());
        assert!(set_goal(&root, "  ", Some("s1")).is_err());
        assert_eq!(
            set_goal(&root, "x", Some("nope")).unwrap_err().code(),
            "no_own_task"
        );
        assert!(clear_goal(&root, Some("nope")).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wrong_typed_field_is_loud_not_silent() {
        let root = fresh_dir("wt");
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        // createdAt 错型（字符串）：typed 承载必须响亮报错，不静默吞默认。
        std::fs::write(
            root.join(".claude").join("scheduled_tasks.json"),
            r#"{"tasks":[{"id":"x1","cron":"*/5 * * * *","prompt":"p","createdAt":"not-a-number","recurring":true,"createdBySessionId":"s1"}]}"#,
        )
        .unwrap();
        let err = list_tasks(&root, None).unwrap_err();
        assert_eq!(err.code(), "corrupt");
        assert!(
            err.to_string().contains("bad task row at tasks[0]"),
            "error should name row and cause: {err}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tasks_not_array_refuses_overwrite() {
        let root = fresh_dir("na");
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        let raw = r#"{"tasks":{"weird":1}}"#;
        let f = root.join(".claude").join("scheduled_tasks.json");
        std::fs::write(&f, raw).unwrap();
        // 只读面：硬错不静默当空集（评审 F2 实证复现面）。
        let err = list_tasks(&root, None).unwrap_err();
        assert_eq!(err.code(), "corrupt");
        assert!(err.to_string().contains("not an array"), "{err}");
        // 写面：拒绝覆写真数据，文件字节不动。
        assert!(set_loop(&root, "x", Cadence::Every("5m"), Some("s1")).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), raw);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_required_field_loud_with_row_index() {
        let root = fresh_dir("mr");
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        // 前行合法、后行缺 id：报错点名 tasks[1]（评审 F1 加 G2 回填）。
        std::fs::write(
            root.join(".claude").join("scheduled_tasks.json"),
            r#"{"tasks":[{"id":"ok1","cron":"* * * * *","prompt":"a"},{"cron":"* * * * *","prompt":"b"}]}"#,
        )
        .unwrap();
        let err = list_tasks(&root, None).unwrap_err();
        assert_eq!(err.code(), "corrupt");
        let msg = err.to_string();
        assert!(msg.contains("bad task row at tasks[1]"), "{msg}");
        assert!(msg.contains("missing field `id`"), "{msg}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn foreign_missing_optional_keys_get_defaulted_visibly() {
        let root = fresh_dir("fd");
        write_tasks(
            &root,
            &[jt(
                json!({ "id": "f1", "cron": "*/5 * * * *", "prompt": "外",
                     "createdBySessionId": "s2", "createdAt": 9u64 }),
            )],
        )
        .unwrap();
        // 评审 G5 回填：缺 recurring/pid/procStart 的外会话行经 hst 写盘
        // 补全默认（false/0/空串），属接受的可见变化。
        set_goal(&root, "改写", Some("s2")).unwrap();
        let after: Json = serde_json::from_str(
            &std::fs::read_to_string(root.join(".claude").join("scheduled_tasks.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(after["tasks"][0]["recurring"], false);
        assert_eq!(after["tasks"][0]["createdByPid"], 0);
        assert_eq!(after["tasks"][0]["createdByProcStart"], "");
        assert_eq!(after["tasks"][0]["prompt"], "改写");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn goal_empty_error_keeps_clear_hint() {
        // 评审 G1 回填：set_goal 空文本保留 hst goal clear 指引尾巴。
        let root = fresh_dir("gh");
        write_tasks(
            &root,
            &[jt(
                json!({ "id": "g1", "cron": "*/5 * * * *", "prompt": "p",
                     "createdAt": 1u64, "createdBySessionId": "s1" }),
            )],
        )
        .unwrap();
        let err = set_goal(&root, "  ", Some("s1")).unwrap_err();
        assert_eq!(err.code(), "empty_goal");
        assert!(
            err.to_string().contains("hst goal clear"),
            "guidance tail must survive: {err}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn g8_top_level_non_object_refuses_overwrite() {
        let root = fresh_dir("g8");
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        let raw = r#"[1,2,3]"#;
        let f = root.join(".claude").join("scheduled_tasks.json");
        std::fs::write(&f, raw).unwrap();
        // 评审二轮 G8 回填：顶层非对象读写两面都硬错，文件字节不动。
        let err = list_tasks(&root, None).unwrap_err();
        assert_eq!(err.code(), "corrupt");
        assert!(
            err.to_string().contains("top-level is not an object"),
            "{err}"
        );
        assert!(set_loop(&root, "x", Cadence::Every("5m"), Some("s1")).is_err());
        assert!(set_goal(&root, "y", Some("s1")).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), raw);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn health_zero_when_file_missing() {
        // REQ-020：文件缺失是合法零任务态，健康快照零计数非错误。
        let root = fresh_dir("hz");
        let h = health(&root).unwrap();
        assert_eq!(h.total, 0);
        assert_eq!(h.dead_owner, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn health_dead_owner_only_for_vanished_pid_linux() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let root = fresh_dir("hd");
        write_tasks(
            &root,
            &[
                jt(json!({ "id": "d1", "cron": "*/5 * * * *", "prompt": "a",
                         "createdAt": 1u64, "createdBySessionId": "s1",
                         "createdByPid": 4000000u64, "createdByProcStart": "1" })),
                jt(json!({ "id": "a0", "cron": "*/7 * * * *", "prompt": "b",
                         "createdAt": 2u64, "createdBySessionId": "s1",
                         "createdByPid": 0u64, "createdByProcStart": "" })),
                jt(json!({ "id": "me", "cron": "*/9 * * * *", "prompt": "c",
                         "createdAt": 3u64, "createdBySessionId": "s1",
                         "createdByPid": std::process::id() as u64,
                         "createdByProcStart": "" })),
            ],
        )
        .unwrap();
        let h = health(&root).unwrap();
        assert_eq!(h.total, 3);
        // 只有消失 pid 算死：pid=0 归属未知不算、本测试进程活着（procStart
        // 落盘为空只查进程在否）不算。
        assert_eq!(h.dead_owner, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn task_level_unknown_keys_survive_roundtrip() {
        let root = fresh_dir("uk");
        write_tasks(
            &root,
            &[jt(
                json!({ "id": "u1", "cron": "*/5 * * * *", "prompt": "p",
                     "createdAt": 42u64, "recurring": true, "createdBySessionId": "s1",
                     "futureField": {"nested": 7} }),
            )],
        )
        .unwrap();
        set_goal(&root, "改写后", Some("s1")).unwrap();
        let after: Json = serde_json::from_str(
            &std::fs::read_to_string(root.join(".claude").join("scheduled_tasks.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            after["tasks"][0]["futureField"]["nested"], 7,
            "unknown task-level keys survive read-modify-write"
        );
        assert_eq!(after["tasks"][0]["prompt"], "改写后");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn session_resolution_env_over_claude_json_over_err() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir("su");
        let root = fresh_dir("sr");
        std::env::set_var("HST_USER_HOME", &user);
        // 三级全空：报错。
        std::env::remove_var("CLAUDE_CODE_SESSION_ID");
        assert!(resolve_session(None, &root).is_err());
        // claude.json 项目表 lastSessionId 兜底。
        std::fs::write(
            user.join(".claude.json"),
            format!(
                "{{\"projects\": {{\"{}\": {{\"lastSessionId\": \"from-json\"}}}}}}",
                forward_slash(&root)
            ),
        )
        .unwrap();
        assert_eq!(resolve_session(None, &root).unwrap(), "from-json");
        // 环境变量优先于 claude.json。
        std::env::set_var("CLAUDE_CODE_SESSION_ID", "from-env");
        assert_eq!(resolve_session(None, &root).unwrap(), "from-env");
        // 显式旗标最优先。
        assert_eq!(
            resolve_session(Some("explicit"), &root).unwrap(),
            "explicit"
        );
        std::env::remove_var("CLAUDE_CODE_SESSION_ID");
        std::env::remove_var("HST_USER_HOME");
        let _ = std::fs::remove_dir_all(&user);
        let _ = std::fs::remove_dir_all(&root);
    }
}
