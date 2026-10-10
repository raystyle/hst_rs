//! hst goal 自治面（REQ-045，REQ-019 演进；用户令 2026-10-11「hst 不再
//! 关联 goal 和 loop」「hst 自己用 hook 实现 goal」）：goal 文本由 hst
//! 自有存储承载（`~/.hst/state/goal/<slug>.json`，slug 规则同 trace 与
//! projyolo：路径非字母数字一律 `-`），与项目 `.claude/scheduled_tasks.json`
//! 的一切读写解除（该文件回归 Claude Code 原生 CronCreate 域，loop 面
//! v2.10.0 退役）。Stop 事件腿（`hst hook goal`，注册面 Claude Stop 事件
//! 与 state 腿双挂，仅 claude）：每轮 agent 停止 rounds 计数落盘（状态栏
//! goal 段的事件驱动数据源），自然停（payload 无 stop_hook_active）出
//! `decision: block` 加 reason 提醒注入一轮、续跑引发的再停放行（Claude
//! Code Stop 契约，[实证: code.claude.com/docs/en/hooks 2026-10-11 查：
//! stop_hook_active 在场须 exit 0 防死环；block 加 reason 续跑]），
//! 恒无死环。CLI 面 set/show/clear 语义承接 REQ-019 管理命令面（项目级，
//! --project 缺省当前目录；项目键 = CLI 侧项目根与 payload 侧
//! workspace.project_dir 同一 slug，子目录调用键不中，须在项目根跑或显式
//! 给 --project，边界同 projyolo marker 先例）。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};

/// goal 记录（自有存储单文件形，项目一键一文件）。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GoalRecord {
    /// goal 文本（trim 后非空；Stop 提醒与状态栏 goal 段显示源）。
    pub text: String,
    /// 设置时刻（epoch 秒）。
    pub set_at: u64,
    /// Stop 事件计数（Stop 腿每轮递增；状态栏 `{count}` 显示源）。
    pub rounds: u64,
    /// 最近一次 Stop 时刻（epoch 秒；0 = 设置后尚无停止事件）。
    pub last_stop_at: u64,
}

/// goal 面错误（承接 REQ-021 类型契约先例）：`Display` 面向人，
/// [`GoalError::code`] 给 agent 稳定短码（CLI 错误面 `goal error=<code>`）。
#[derive(Debug)]
pub enum GoalError {
    /// goal 文本 trim 后为空。
    EmptyText,
    /// 存储文件在场但不可解析（携带路径与原因）。
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

impl GoalError {
    /// agent 面稳定短码（CLI 错误输出 `error=<code>` 的取值源）。
    pub fn code(&self) -> &'static str {
        match self {
            GoalError::EmptyText => "empty_goal",
            GoalError::Corrupt { .. } => "corrupt",
            GoalError::Io { .. } => "io",
        }
    }
}

impl std::fmt::Display for GoalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalError::EmptyText => write!(f, "goal text is empty"),
            GoalError::Corrupt { path, cause } => write!(f, "{path}: {cause}"),
            GoalError::Io { path, cause } => write!(f, "{path}: {cause}"),
        }
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 项目键 slug（规则同 trace 与 projyolo marker：路径非字母数字一律 `-`）。
pub fn slug(root: &str) -> String {
    root.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// goal 存储目录（`<hst 自管根>/state/goal/`；测试经 HST_ROOT 缝注入）。
pub fn goal_dir(home: &Path) -> PathBuf {
    home.join("state").join("goal")
}

/// 项目 goal 文件路径（`<dir>/<slug>.json`）。
pub fn goal_path(home: &Path, root: &Path) -> PathBuf {
    goal_dir(home).join(format!("{}.json", slug(&root.display().to_string())))
}

fn read_record(path: &Path) -> Result<Option<GoalRecord>, GoalError> {
    if !path.exists() {
        return Ok(None);
    }
    // 复用 yolo 面的 BOM 容忍读；解析失败硬错上抛，绝不覆写真数据。
    let v = crate::yolo::read_json(path).map_err(|e| GoalError::Corrupt {
        path: path.display().to_string(),
        cause: e,
    })?;
    serde_json::from_value(v)
        .map(Some)
        .map_err(|e| GoalError::Corrupt {
            path: path.display().to_string(),
            cause: e.to_string(),
        })
}

fn write_record(path: &Path, rec: &GoalRecord) -> Result<(), GoalError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| GoalError::Io {
            path: dir.display().to_string(),
            cause: e.to_string(),
        })?;
    }
    let body = serde_json::to_string_pretty(rec).map_err(|e| GoalError::Corrupt {
        path: path.display().to_string(),
        cause: format!("serialize: {e}"),
    })? + "\n";
    // 原子写（tmp 加 rename，同 hook state 先例）：Stop 腿与状态栏读并发
    // 不见半文件。
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| GoalError::Io {
        path: tmp.display().to_string(),
        cause: e.to_string(),
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        GoalError::Io {
            path: path.display().to_string(),
            cause: e.to_string(),
        }
    })
}

/// 设置本项目 goal（覆盖旧 goal，rounds 归零重计）。
///
/// # Errors
///
/// 文本 trim 后为空、存储坏损或 IO 失败时返回 [`GoalError`]。
pub fn set_goal(home: &Path, root: &Path, text: &str) -> Result<GoalRecord, GoalError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(GoalError::EmptyText);
    }
    let rec = GoalRecord {
        text: text.to_string(),
        set_at: unix_secs(),
        rounds: 0,
        last_stop_at: 0,
    };
    write_record(&goal_path(home, root), &rec)?;
    Ok(rec)
}

/// 查看本项目 goal（无 goal 返回 None，查看面不受阻）。
///
/// # Errors
///
/// 存储坏损或 IO 失败时返回 [`GoalError`]。
pub fn show_goal(home: &Path, root: &Path) -> Result<Option<GoalRecord>, GoalError> {
    read_record(&goal_path(home, root))
}

/// 清空本项目 goal（删文件；返回是否真删，无 goal 零改动合法）。
///
/// # Errors
///
/// IO 失败时返回 [`GoalError`]。
pub fn clear_goal(home: &Path, root: &Path) -> Result<bool, GoalError> {
    let path = goal_path(home, root);
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).map_err(|e| GoalError::Io {
        path: path.display().to_string(),
        cause: e.to_string(),
    })?;
    Ok(true)
}

/// payload 项目根解析序（同旧 loop 探针链，REQ-019 时点口径承接）：
/// `workspace.project_dir` 回落 `workspace.current_dir` 回落 `cwd`；
/// 事件参数形（无 payload）回落进程当前目录。
fn payload_root(payload: Option<&Json>) -> String {
    if let Some(v) = payload {
        let mut s = v
            .get("workspace")
            .and_then(|w| w.get("project_dir"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if s.is_empty() || s == "." {
            s = v
                .get("workspace")
                .and_then(|w| w.get("current_dir"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
        }
        if s.is_empty() || s == "." {
            s = v.get("cwd").and_then(|x| x.as_str()).unwrap_or("");
        }
        if !s.is_empty() && s != "." {
            return s.to_string();
        }
    }
    std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

/// 指令文本卫生（单源先例承 herdr arm 面，该面已随 loop 退役）：换行与
/// 回车压空格，trim 收边（reason 单行契约）。
fn flatten_inline(s: &str) -> String {
    s.chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Stop 提醒 JSON（Claude Code Stop 契约：`decision: block` 加 `reason`
/// 续跑一轮，reason 即注入下轮的提醒文本）。
///
/// # Errors
///
/// 序列化失败时返回 `String`（纯字符串组合，实跑不可达，类型面完整）。
pub fn reminder_json(rec: &GoalRecord) -> Result<String, String> {
    let reason = format!(
        "hst goal 在役（第 {} 轮）：「{}」。若目标已达成，请运行 hst goal clear 收束并如实简报；\
         若未达成，继续推进；若受阻，如实说明阻塞后停。",
        rec.rounds,
        flatten_inline(&rec.text)
    );
    serde_json::to_string(&json!({ "decision": "block", "reason": reason }))
        .map_err(|e| e.to_string())
}

/// Stop 腿入口（`hst hook goal`）：非 Stop 事件或本项目无 goal 返回
/// None（静默放行）；有 goal 则 rounds 递增落盘后，自然停（payload 无
/// stop_hook_active）返回提醒 JSON、续跑再停（stop_hook_active=true）返回
/// None。计数落盘恒执行（防环放行轮也计），状态栏 goal 段数据随之刷新。
///
/// # Errors
///
/// 存储坏损或 IO 失败时返回 `String`（调用方 fail-open，不挡 agent 停止）。
pub fn stop_hook(
    home: &Path,
    payload: Option<&Json>,
    event_arg: Option<&str>,
) -> Result<Option<String>, String> {
    let is_stop = match event_arg {
        Some(e) => e.trim().eq_ignore_ascii_case("stop"),
        None => payload
            .and_then(|v| {
                v.get("hook_event_name")
                    .or_else(|| v.get("hookEventName"))
                    .and_then(|x| x.as_str())
            })
            .is_some_and(|e| e.trim().eq_ignore_ascii_case("stop")),
    };
    if !is_stop {
        return Ok(None);
    }
    let root = payload_root(payload);
    if root.is_empty() {
        return Ok(None);
    }
    let path = goal_path(home, Path::new(&root));
    let mut rec = match read_record(&path).map_err(|e| e.to_string())? {
        Some(r) => r,
        None => return Ok(None),
    };
    rec.rounds += 1;
    rec.last_stop_at = unix_secs();
    write_record(&path, &rec).map_err(|e| e.to_string())?;
    if payload.is_some_and(|v| v.get("stop_hook_active").and_then(|x| x.as_bool()) == Some(true)) {
        return Ok(None);
    }
    reminder_json(&rec).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_home(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-goal-{tag}-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn slug_matches_trace_and_projyolo_rule() {
        assert_eq!(slug("/home/ray/repos/hst_rs"), "-home-ray-repos-hst-rs");
        assert_eq!(
            slug("C:\\Users\\ray\\proj"),
            "C--Users-ray-proj",
            "非字母数字（含盘符冒号与反斜杠）一律 -"
        );
    }

    #[test]
    fn set_show_clear_roundtrip_and_overwrite_resets_rounds() {
        let home = fresh_home("rt");
        let root = Path::new("/w/proj");
        assert!(show_goal(&home, root).unwrap().is_none());
        let r = set_goal(&home, root, "盯发布窗口").unwrap();
        assert_eq!(r.text, "盯发布窗口");
        assert_eq!(r.rounds, 0);
        assert_eq!(goal_path(&home, root), goal_dir(&home).join("-w-proj.json"));
        let got = show_goal(&home, root).unwrap().unwrap();
        assert_eq!(got.text, "盯发布窗口");
        // rounds 拨高后覆盖设置：重置归零（新 goal 新计数）。
        let mut aged = got.clone();
        aged.rounds = 9;
        write_record(&goal_path(&home, root), &aged).unwrap();
        let r2 = set_goal(&home, root, "换目标").unwrap();
        assert_eq!(r2.rounds, 0);
        assert_eq!(show_goal(&home, root).unwrap().unwrap().text, "换目标");
        // clear：删文件、再 clear 零改动合法。
        assert!(clear_goal(&home, root).unwrap());
        assert!(show_goal(&home, root).unwrap().is_none());
        assert!(!clear_goal(&home, root).unwrap());
        // 空文本短码承接。
        assert_eq!(
            set_goal(&home, root, "  ").unwrap_err().code(),
            "empty_goal"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn stop_hook_counts_and_reminds_only_on_natural_stop() {
        let home = fresh_home("sh");
        let root = Path::new("/w/p");
        set_goal(&home, root, "闭环 REQ\n收尾").unwrap();
        // 非 Stop 事件：零动作零输出。
        let other = json!({ "hook_event_name": "PreToolUse", "cwd": "/w/p" });
        assert!(stop_hook(&home, Some(&other), None).unwrap().is_none());
        assert_eq!(show_goal(&home, root).unwrap().unwrap().rounds, 0);
        // 自然停：计数加一加提醒注入（文本换行压平）。
        let stop = json!({ "hook_event_name": "Stop", "cwd": "/w/p", "session_id": "s1" });
        let out = stop_hook(&home, Some(&stop), None).unwrap().unwrap();
        assert!(out.contains("\"decision\":\"block\""), "{out}");
        assert!(out.contains("闭环 REQ 收尾"), "{out}");
        assert!(out.contains("hst goal clear"), "{out}");
        let rec = show_goal(&home, root).unwrap().unwrap();
        assert_eq!(rec.rounds, 1);
        assert!(rec.last_stop_at > 0);
        // 续跑再停（stop_hook_active）：计数仍落盘、输出放行。
        let again = json!({
            "hook_event_name": "Stop",
            "cwd": "/w/p",
            "stop_hook_active": true
        });
        assert!(stop_hook(&home, Some(&again), None).unwrap().is_none());
        assert_eq!(show_goal(&home, root).unwrap().unwrap().rounds, 2);
        // 无 goal 项目：静默。
        let elsewhere = json!({ "hook_event_name": "Stop", "cwd": "/w/other" });
        assert!(stop_hook(&home, Some(&elsewhere), None).unwrap().is_none());
        // 事件参数形（无 payload）：项目根回落进程 cwd，本测试 cwd 无 goal
        // 文件即静默。
        assert!(stop_hook(&home, None, Some("stop")).unwrap().is_none());
        assert!(stop_hook(&home, None, Some("idle")).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn payload_root_prefers_project_dir_chain() {
        let v = json!({
            "workspace": { "project_dir": "/w/a", "current_dir": "/w/a/sub" },
            "cwd": "/w/a/sub/deeper"
        });
        assert_eq!(payload_root(Some(&v)), "/w/a");
        let v2 = json!({ "cwd": "/w/b" });
        assert_eq!(payload_root(Some(&v2)), "/w/b");
        // 点值（"."）跳过续走链。
        let v3 = json!({ "workspace": { "project_dir": "." }, "cwd": "/w/c" });
        assert_eq!(payload_root(Some(&v3)), "/w/c");
    }

    #[test]
    fn corrupt_store_is_loud_not_silent() {
        let home = fresh_home("cp");
        let root = Path::new("/w/x");
        let p = goal_path(&home, root);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "not-json").unwrap();
        assert_eq!(show_goal(&home, root).unwrap_err().code(), "corrupt");
        // Stop 腿遇坏损：上抛（调用方 fail-open），不静默覆写。
        let stop = json!({ "hook_event_name": "Stop", "cwd": "/w/x" });
        assert!(stop_hook(&home, Some(&stop), None).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "not-json");
        let _ = std::fs::remove_dir_all(&home);
    }
}
