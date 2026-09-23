//! loop 与 goal 管理面（REQ-019）。唯一真相 = 项目根
//! `.claude/scheduled_tasks.json`（Claude Code durable CronCreate 的同源
//! 落盘形，实证 2026-09-23 探针：tasks[] 含 id / cron / prompt /
//! recurring / createdAt / createdBySessionId / createdByPid /
//! createdByProcStart）；hst 是该文件的读写管理面（set / list / del），
//! 状态栏 loop 与 goal 段同源只读消费，不自建第二状态文件。会话归属
//! 判据 = `createdBySessionId` 与当前会话 id 等值。goal 语义 = 任务
//! `prompt` 文本。并发面与 Claude Code 同款读改写（无锁，后写覆盖先写）；
//! 同一会话内混用 hst 与 agent 原生 cron 工具会互相覆盖盘上变更，管理
//! 纪律是单向（要么全经 hst，要么全经会话内工具）。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};

use crate::pathutil::{forward_slash, user_home};

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

fn read_tasks(root: &Path) -> Result<Vec<Json>, String> {
    let path = scheduled_tasks_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    // 复用 yolo 面的 BOM 容忍读；解析失败硬错上抛，绝不覆写真数据。
    let v = crate::yolo::read_json(&path)?;
    Ok(v.get("tasks")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default())
}

fn write_tasks(root: &Path, tasks: &[Json]) -> Result<PathBuf, String> {
    let path = scheduled_tasks_path(root);
    let text =
        serde_json::to_string_pretty(&json!({ "tasks": tasks })).map_err(|e| e.to_string())? + "\n";
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// 解析「当前会话」：显式 `--session` 优先，其次 agent 会话内环境变量
/// `CLAUDE_CODE_SESSION_ID`，再次 `~/.claude.json` 项目表 `lastSessionId`
/// （键 = 项目根正斜杠形，与 pretrust 面同判）；全落空报错并提示显式给。
pub fn resolve_session(explicit: Option<&str>, root: &Path) -> Result<String, String> {
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
    let cj = user_home()?.join(".claude.json");
    if cj.exists() {
        let v = crate::yolo::read_json(&cj)?;
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
    Err(format!(
        "cannot resolve current session (no --session, no CLAUDE_CODE_SESSION_ID, no \
         ~/.claude.json projects[{key}].lastSessionId); pass --session <id> explicitly"
    ))
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
/// `M * * * *`，M 取落盘时刻分钟位并避开 0 与 30（舰队避整点半点纪律）。
/// 恒为 recurring 形。
pub fn every_to_cron(every: &str) -> Result<(String, bool), String> {
    let spec = every.trim();
    if let Some(n) = spec.strip_suffix('m').and_then(|d| d.parse::<u32>().ok()) {
        if (1..=59).contains(&n) {
            return Ok((format!("*/{n} * * * *"), true));
        }
        return Err(format!("invalid --every minute range (1-59): {spec}"));
    }
    if let Some(n) = spec.strip_suffix('h').and_then(|d| d.parse::<u32>().ok()) {
        if (1..=23).contains(&n) {
            let mut m = unix_millis() / 60_000 % 60;
            if m == 0 {
                m = 7;
            } else if m == 30 {
                m = 37;
            }
            return Ok((format!("{m} * * * *"), true));
        }
        return Err(format!("invalid --every hour range (1-23): {spec}"));
    }
    Err(format!(
        "invalid --every (expect Nm 1-59 or Nh 1-23): {spec}"
    ))
}

/// 一次性时刻转 cron：`HH:MM`（24 小时制）得 `M H * * *` 加 recurring
/// false（到点触发后由运行时自动删除）。
pub fn at_to_cron(at: &str) -> Result<(String, bool), String> {
    let spec = at.trim();
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (
            parts[0].trim().parse::<u32>(),
            parts[1].trim().parse::<u32>(),
        ) {
            if h < 24 && m < 60 {
                return Ok((format!("{m} {h} * * *"), false));
            }
        }
    }
    Err(format!("invalid --at (expect HH:MM 24h): {spec}"))
}

/// 设置当前会话 loop：`--every` 与 `--at` 二选一（双缺或双给报错），
/// goal 文本 trim 后非空；读改写保留既有任务（含外会话与 agent 原生
/// 落盘项）。
pub fn set_loop(
    root: &Path,
    goal: &str,
    every: Option<&str>,
    at: Option<&str>,
    session: Option<&str>,
) -> Result<SetReport, String> {
    let goal = goal.trim();
    if goal.is_empty() {
        return Err("goal text is empty".to_string());
    }
    let (cron, recurring) = match (every, at) {
        (Some(e), None) => every_to_cron(e)?,
        (None, Some(a)) => at_to_cron(a)?,
        _ => {
            return Err(
                "exactly one of --every / --at is required (they are mutually exclusive)"
                    .to_string(),
            )
        }
    };
    let session = resolve_session(session, root)?;
    let (pid, proc_start) = owner_proc();
    let id = new_id();
    let task = json!({
        "id": id,
        "cron": cron,
        "prompt": goal,
        "createdAt": unix_millis(),
        "recurring": recurring,
        "createdBySessionId": session,
        "createdByPid": pid,
        "createdByProcStart": proc_start,
    });
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

fn task_str(t: &Json, k: &str) -> String {
    t.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn task_row(t: &Json, session: &Option<String>) -> TaskRow {
    let sid = task_str(t, "createdBySessionId");
    TaskRow {
        id: task_str(t, "id"),
        cron: task_str(t, "cron"),
        recurring: t
            .get("recurring")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        session: sid.clone(),
        goal: task_str(t, "prompt"),
        ours: session.as_deref().is_some_and(|s| *s == sid),
    }
}

/// 列出项目定时任务（全量，不按会话过滤；`ours` 标当前会话归属，
/// 会话解析失败时返回 None 且 ours 恒 false，列表本身不受阻）。
pub fn list_tasks(
    root: &Path,
    session: Option<&str>,
) -> Result<(Vec<TaskRow>, Option<String>), String> {
    let resolved = resolve_session(session, root).ok();
    let tasks = read_tasks(root)?;
    Ok((
        tasks.iter().map(|t| task_row(t, &resolved)).collect(),
        resolved,
    ))
}

/// goal 改写报告：被改任务的 id 与 cron。
pub struct GoalReport {
    /// 被改任务 id。
    pub id: String,
    /// 被改任务 cron（goal 面不动节奏，原样回显）。
    pub cron: String,
}

/// 本会话 createdAt 最新任务的下标（同刻取后入者，与 del latest 同判）。
fn latest_own_index(tasks: &[Json], sid: &str) -> Option<usize> {
    tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| task_str(t, "createdBySessionId") == sid)
        .max_by_key(|(i, t)| (t.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(0), *i))
        .map(|(i, _)| i)
}

/// 设置当前会话最新 loop 的 goal 文本（prompt 就地改写，节奏与属主不动；
/// 无本会话任务报错，先 `hst loop set` 建任务）。
pub fn set_goal(root: &Path, text: &str, session: Option<&str>) -> Result<GoalReport, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("goal text is empty (to remove the goal use: hst goal clear)".to_string());
    }
    let sid = resolve_session(session, root)?;
    let mut tasks = read_tasks(root)?;
    let idx = latest_own_index(&tasks, &sid).ok_or_else(|| {
        "no loop task for current session (create one with: hst loop set)".to_string()
    })?;
    let report = GoalReport {
        id: task_str(&tasks[idx], "id"),
        cron: task_str(&tasks[idx], "cron"),
    };
    tasks[idx]["prompt"] = json!(text);
    write_tasks(root, &tasks)?;
    Ok(report)
}

/// 清空当前会话最新 loop 的 goal（prompt 置空、任务与节奏保留；返回被改
/// 任务 id，无本会话任务返回 None）。
pub fn clear_goal(root: &Path, session: Option<&str>) -> Result<Option<String>, String> {
    let sid = resolve_session(session, root)?;
    let mut tasks = read_tasks(root)?;
    match latest_own_index(&tasks, &sid) {
        None => Ok(None),
        Some(idx) => {
            let id = task_str(&tasks[idx], "id");
            tasks[idx]["prompt"] = json!("");
            write_tasks(root, &tasks)?;
            Ok(Some(id))
        }
    }
}

/// 查看当前会话最新 loop 的 goal（会话不可解析或无任务返回 None，查看面
/// 不受阻）。
pub fn show_goal(root: &Path, session: Option<&str>) -> Result<Option<TaskRow>, String> {
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
pub fn del_loops(root: &Path, target: &str, session: Option<&str>) -> Result<Vec<String>, String> {
    let tasks = read_tasks(root)?;
    let target = target.trim();
    let keep: Vec<&Json> = if target.eq_ignore_ascii_case("all")
        || target.eq_ignore_ascii_case("latest")
    {
        let sid = resolve_session(session, root)?;
        let mut mine: Vec<(usize, u64)> = tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| task_str(t, "createdBySessionId") == sid)
            .map(|(i, t)| (i, t.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(0)))
            .collect();
        if mine.is_empty() {
            return Ok(Vec::new());
        }
        if target.eq_ignore_ascii_case("latest") {
            mine.sort_by_key(|(_, c)| *c);
            let drop = mine.last().unwrap().0;
            tasks
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != drop)
                .map(|(_, t)| t)
                .collect()
        } else {
            // all：本会话全删，保留外会话任务。
            let mine_idx: std::collections::HashSet<usize> = mine.iter().map(|(i, _)| *i).collect();
            tasks
                .iter()
                .enumerate()
                .filter(|(i, _)| !mine_idx.contains(i))
                .map(|(_, t)| t)
                .collect()
        }
    } else {
        tasks
            .iter()
            .filter(|t| task_str(t, "id") != target)
            .collect()
    };
    let removed: Vec<String> = tasks
        .iter()
        .filter(|t| !keep.iter().any(|k| *k == *t))
        .map(|t| task_str(t, "id"))
        .collect();
    if !removed.is_empty() {
        let kept: Vec<Json> = keep.into_iter().cloned().collect();
        write_tasks(root, &kept)?;
    }
    Ok(removed)
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

    #[test]
    fn set_list_del_roundtrip_preserves_foreign() {
        let root = fresh_dir("rt");
        // 外会话既有任务先行落盘（模拟 agent 原生写入）。
        write_tasks(
            &root,
            &[json!({
                "id": "foreign01", "cron": "*/9 * * * *", "prompt": "外会话任务",
                "createdAt": 1u64, "recurring": true,
                "createdBySessionId": "other-session", "createdByPid": 1u64,
                "createdByProcStart": "1",
            })],
        )
        .unwrap();
        let r1 = set_loop(&root, "盯CI发布", Some("5m"), None, Some("s1")).unwrap();
        assert_eq!(r1.cron, "*/5 * * * *");
        assert!(r1.recurring);
        assert_eq!(r1.session, "s1");
        assert_eq!(r1.id.len(), 8);
        let r2 = set_loop(&root, "十四点半提醒", None, Some("14:30"), Some("s1")).unwrap();
        assert_eq!(r2.cron, "30 14 * * *");
        assert!(!r2.recurring);
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
                json!({ "id": "old000001", "cron": "*/5 * * * *", "prompt": "旧",
                       "createdAt": 100u64, "recurring": true, "createdBySessionId": "s1" }),
                json!({ "id": "new000002", "cron": "*/7 * * * *", "prompt": "新",
                       "createdAt": 200u64, "recurring": true, "createdBySessionId": "s1" }),
                json!({ "id": "fgn000003", "cron": "*/9 * * * *", "prompt": "外",
                       "createdAt": 300u64, "recurring": true, "createdBySessionId": "s2" }),
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
        assert_eq!(
            every_to_cron("5m").unwrap(),
            ("*/5 * * * *".to_string(), true)
        );
        let (hc, recurring) = every_to_cron("2h").unwrap();
        assert!(recurring);
        assert!(hc.ends_with("* * * *"));
        assert!(hc.split(' ').next().unwrap().parse::<u32>().is_ok());
        assert!(every_to_cron("60m").is_err());
        assert!(every_to_cron("24h").is_err());
        assert!(every_to_cron("abc").is_err());
        assert_eq!(
            at_to_cron("09:05").unwrap(),
            ("5 9 * * *".to_string(), false)
        );
        assert!(at_to_cron("24:00").is_err());
        assert!(at_to_cron("9:5:30").is_err());
        let root = fresh_dir("val");
        assert!(set_loop(&root, "g", None, None, Some("s1")).is_err());
        assert!(set_loop(&root, "g", Some("5m"), Some("10:00"), Some("s1")).is_err());
        assert!(set_loop(&root, "  ", Some("5m"), None, Some("s1")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn goal_set_show_clear_roundtrip_on_latest_own() {
        let root = fresh_dir("gl");
        write_tasks(
            &root,
            &[
                json!({ "id": "old000001", "cron": "*/5 * * * *", "prompt": "旧",
                       "createdAt": 100u64, "recurring": true, "createdBySessionId": "s1" }),
                json!({ "id": "new000002", "cron": "*/7 * * * *", "prompt": "新",
                       "createdAt": 200u64, "recurring": true, "createdBySessionId": "s1" }),
                json!({ "id": "foreign01", "cron": "*/9 * * * *", "prompt": "外会话",
                       "createdAt": 300u64, "recurring": true, "createdBySessionId": "s2" }),
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
        assert!(clear_goal(&root, Some("nope")).unwrap().is_none());
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
