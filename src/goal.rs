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
//! 给 --project）。goal-01（2026-10-11）两修：其一子目录键漂——hook
//! payload 常只带 cwd 而 agent 多在项目子目录工作，精确 slug 键不中，
//! 解析改就近上溯（goal_file_for）取最近在册 goal；其二自愈面
//! （run_heal，用户令「如果没用唤醒操作 herdr api 自愈唤醒一下」）
//! ——goal 在册而停摆未被唤醒过静默窗时经 herdr agent prompt 唤醒，
//! blocked 等人裁与频控风暴闸豁免，提醒与唤醒文本均附 hst goal 用法
//! （用户令「让 agent 自行操作」）。

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
    /// 最近一次自愈唤醒时刻（epoch 秒；0 = 未自愈过。goal-01）。
    #[serde(default)]
    pub last_heal_at: u64,
    /// 连续无 rounds 增长的自愈次数（Stop 事件回场即归零；风暴闸输入。
    /// goal-01）。
    #[serde(default)]
    pub heal_streak: u64,
    /// 上次自愈时的 rounds 快照（与现值等值 = 唤醒未转化为 Stop 事件。
    /// goal-01）。
    #[serde(default)]
    pub heal_rounds: u64,
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

/// 读指定 goal 文件（缺失 = None；坏损 = Err 响亮）。
///
/// # Errors
///
/// 文件在场但不可解析时返回 [`GoalError::Corrupt`]。
pub fn read_record(path: &Path) -> Result<Option<GoalRecord>, GoalError> {
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
        last_heal_at: 0,
        heal_streak: 0,
        heal_rounds: 0,
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
         若未达成，继续推进；若受阻或判断目标不该继续（推不动、无意义或失效），可自行 hst goal clear \
         停止并如实说明——goal 是你的助手不是约束，停机权在你，勿徒劳死循环。hst goal 用法：\
         hst goal set <新目标> 改目标、hst goal show 查看、hst goal clear 清空收束。",
        rec.rounds,
        flatten_inline(&rec.text)
    );
    serde_json::to_string(&json!({ "decision": "block", "reason": reason }))
        .map_err(|e| e.to_string())
}

/// 就近上溯解析（goal-01 根因 B 修）：hook payload 常只带 cwd 而 agent
/// 多在项目子目录工作（pi-rs 实证 cwd = 项目/practice/eval），精确 slug
/// 键不中致 Stop 提醒静默漏发；从 root 逐级上溯取最近一层在册 goal 文件
///（深者胜：子目录自有 goal 优先于项目根 goal）。CLI set/clear 仍按给定
/// 根精确键（管理面不动）。
pub fn goal_file_for(home: &Path, root: &Path) -> Option<PathBuf> {
    let mut cur = Some(root);
    while let Some(p) = cur {
        let f = goal_path(home, p);
        if f.is_file() {
            return Some(f);
        }
        cur = p.parent();
    }
    None
}

/// 事件是否 Stop（事件参数形或 payload hook_event_name 形；heal_sweep 的
/// 挂载判据同源）。
pub fn is_stop_event(payload: Option<&Json>, event_arg: Option<&str>) -> bool {
    match event_arg {
        Some(e) => e.trim().eq_ignore_ascii_case("stop"),
        None => payload
            .and_then(|v| {
                v.get("hook_event_name")
                    .or_else(|| v.get("hookEventName"))
                    .and_then(|x| x.as_str())
            })
            .is_some_and(|e| e.trim().eq_ignore_ascii_case("stop")),
    }
}

/// Stop 腿入口（`hst hook goal`）：非 Stop 事件或本项目无 goal 返回
/// None（静默放行）；有 goal 则 rounds 递增落盘后，自然停（payload 无
/// stop_hook_active）返回提醒 JSON、续跑再停（stop_hook_active=true）返回
/// None。计数落盘恒执行（防环放行轮也计），状态栏 goal 段数据随之刷新。
/// goal-01 起 goal_file_for 就近上溯解析（子目录 cwd 容形）。
///
/// # Errors
///
/// 存储坏损或 IO 失败时返回 `String`（调用方 fail-open，不挡 agent 停止）。
pub fn stop_hook(
    home: &Path,
    payload: Option<&Json>,
    event_arg: Option<&str>,
) -> Result<Option<String>, String> {
    if !is_stop_event(payload, event_arg) {
        return Ok(None);
    }
    let root = payload_root(payload);
    if root.is_empty() {
        return Ok(None);
    }
    let Some(path) = goal_file_for(home, Path::new(&root)) else {
        return Ok(None);
    };
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

/// 自愈静默窗（goal-01 频控即此一闸）：goal 在役而 max(last_stop_at,
/// last_heal_at, set_at) 距今不足此值不判停摆——Stop 提醒链的正常节拍
/// 余量，也是两次自愈唤醒的最小间隔（防风暴）。
pub const HEAL_IDLE_SECS: u64 = 15 * 60;

/// 风暴闸：连续无 rounds 增长的自愈唤醒次数上限（Stop 事件回场即归零），
/// 超过即停手转 report（疑驱动链坏死，须人工重开会话或核 doctor）。
pub const HEAL_STORM_CAP: u64 = 3;

/// herdr 格状态面（`herdr agent list` 的 result.agents 投影，消费面只读
/// 三字段）。
pub struct PaneFace {
    /// 格位 id（唤醒目标，如 `w2T:p1`）。
    pub pane_id: String,
    /// agent 名（claude/codex/kimi/grok）。
    pub agent: String,
    /// 生命周期态（idle/working/blocked/done）。
    pub status: String,
    /// 格位工作目录（与 goal 项目键做上溯匹配）。
    pub cwd: String,
}

/// `herdr agent list` stdout 解析（纯函数面，测试注入 JSON 串）。
///
/// # Errors
///
/// 输出非 JSON 或形不符（无 result.agents 数组）时返回 `String`。
pub fn parse_agent_list(text: &str) -> Result<Vec<PaneFace>, String> {
    let v: Json = serde_json::from_str(text.trim())
        .map_err(|e| format!("herdr agent list 输出非 JSON: {e}"))?;
    let arr = v
        .get("result")
        .and_then(|r| r.get("agents"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| "herdr agent list 形不符（缺 result.agents）".to_string())?;
    Ok(arr
        .iter()
        .filter_map(|a| {
            Some(PaneFace {
                pane_id: a.get("pane_id")?.as_str()?.to_string(),
                agent: a
                    .get("agent")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                status: a
                    .get("agent_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                cwd: a
                    .get("cwd")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect())
}

/// goal 项目键与格位目录的归属判据：格位 cwd 逐级上溯的任一层 slug 等值
/// 即归属（与 [`goal_file_for`] 同 walk，容子目录工作形）。
fn pane_belongs(goal_slug: &str, cwd: &str) -> bool {
    let mut cur = Some(Path::new(cwd));
    while let Some(p) = cur {
        if slug(&p.display().to_string()) == goal_slug {
            return true;
        }
        cur = p.parent();
    }
    false
}

/// 自愈判定两态族（纯函数面；输入 = goal 在册态加 herdr 格状态加当下时
/// 刻）。豁免口径（用户令「agent 自己用 hst 停止的不用自愈」与防风暴）：
/// goal 已清即无文件不入本判；blocked 等人裁态不唤醒；静默窗内不唤醒；
/// 风暴闸到顶不唤醒。
pub enum HealVerdict {
    /// 唤醒该格（pane id）。
    Wake {
        /// 唤醒目标格位。
        pane: String,
    },
    /// 有格在 working：驱动链活或正忙，不扰。
    SkipWorking {
        /// 归属格数。
        panes: usize,
    },
    /// 有格 blocked 等人裁：豁免。
    SkipBlocked,
    /// 静默窗内（Stop 或上次唤醒未过期）：频控跳过。
    SkipThrottled,
    /// 风暴闸到顶（连续唤醒无 rounds 增长）：转人工。
    SkipStormCapped,
    /// 无归属格：goal 孤儿（会话已关未清），report 面。
    NoAgent,
}

/// 自愈判定（纯函数；检测形态 = goal-01 裁量口径：rounds 无增长加 Stop 后
/// idle 时限，idle/blocked 态取自 herdr 格状态而非墙钟推断）。
pub fn heal_verdict(
    rec: &GoalRecord,
    goal_slug: &str,
    panes: &[PaneFace],
    now: u64,
) -> HealVerdict {
    let mine: Vec<&PaneFace> = panes
        .iter()
        .filter(|p| pane_belongs(goal_slug, &p.cwd))
        .collect();
    if mine.is_empty() {
        return HealVerdict::NoAgent;
    }
    if mine.iter().any(|p| p.status == "working") {
        return HealVerdict::SkipWorking { panes: mine.len() };
    }
    if mine.iter().any(|p| p.status == "blocked") {
        return HealVerdict::SkipBlocked;
    }
    let last = rec.last_stop_at.max(rec.last_heal_at).max(rec.set_at);
    if now.saturating_sub(last) < HEAL_IDLE_SECS {
        return HealVerdict::SkipThrottled;
    }
    if rec.heal_streak >= HEAL_STORM_CAP {
        return HealVerdict::SkipStormCapped;
    }
    let pane = mine
        .iter()
        .find(|p| p.status == "idle" || p.status == "done")
        .map(|p| p.pane_id.clone())
        .unwrap_or_default();
    HealVerdict::Wake { pane }
}

/// 自愈唤醒文本（herdr agent prompt 载体；附 hst goal 用法，用户令「让
/// agent 自行操作」）。
pub fn wake_text(rec: &GoalRecord, project: &str) -> String {
    format!(
        "hst goal 自愈唤醒：项目 {} 的 goal 在役（第 {} 轮）而你会话停摆未被 Stop 提醒唤醒\
         （疑会话早于 hst init 注册、子目录键漂或 goal check 超时）。goal 文本：「{}」。\
         若已达成，请运行 hst goal clear 收束并如实简报；若未达成，继续推进；若受阻或判断目标\
         不该继续（推不动、无意义或失效），可自行 hst goal clear 停止并如实说明——停机权在你，\
         勿徒劳死循环。hst goal 用法：hst goal set <新目标> 改目标、hst goal show 查看、\
         hst goal clear 清空收束。",
        project,
        rec.rounds,
        flatten_inline(&rec.text)
    )
}

/// 单 goal 的自愈回执行（kv 面）。
pub struct HealReport {
    /// goal 项目键 slug。
    pub slug: String,
    /// 动作（woke/skip-working/skip-blocked/skip-throttled/skip-storm-capped/no-agent/skip-corrupt）。
    pub action: String,
    /// 明细（格位、原因、计数）。
    pub detail: String,
}

/// herdr 子进程有界执行（goal-01 澄清令「可控」面）：挂死不拖垮 hook 腿
///（Stop 注册 timeout 10s，子进程预算收紧），超时杀掉报错由调用方分流。
fn herdr_output_bounded(args: &[&str], budget_ms: u128) -> Result<String, String> {
    let mut child = std::process::Command::new("herdr")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("herdr spawn: {e}（须在 herdr 会话格内跑，或 PATH 有 herdr）"))?;
    use std::io::Read as _;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out_buf = String::new();
                if let Some(mut h) = stdout {
                    let _ = h.read_to_string(&mut out_buf);
                }
                let mut err_buf = String::new();
                if let Some(mut h) = stderr {
                    let _ = h.read_to_string(&mut err_buf);
                }
                if !status.success() {
                    return Err(format!(
                        "herdr {} 退出 {:?}: {}",
                        args.join(" "),
                        status.code(),
                        err_buf.trim()
                    ));
                }
                return Ok(out_buf);
            }
            Ok(None) => {
                if start.elapsed().as_millis() >= budget_ms {
                    let _ = child.kill();
                    return Err(format!(
                        "herdr {} 超时 {}ms（挂起，放弃本轮自愈）",
                        args.join(" "),
                        budget_ms
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(e) => return Err(format!("herdr wait: {e}")),
        }
    }
}

fn herdr_output(args: &[&str]) -> Result<String, String> {
    let budget: u128 = if args.first().is_some_and(|a| *a == "agent")
        && args.get(1).is_some_and(|a| *a == "list")
    {
        4_000
    } else {
        8_000
    };
    herdr_output_bounded(args, budget)
}

/// 自愈扫描入口（`hst goal heal`）：扫在册 goal（`only` 给定则单项目），
/// 逐条判定后对 Wake 形经 `herdr agent prompt` 唤醒（无 --wait，提交即
/// 回；下一轮扫描自会复核），并落 last_heal_at 加风暴闸计数；dry_run 只
/// 报判定不唤醒不落盘。herdr 消费面只读 agent list 加单发 prompt，不改
/// herdr 仓（goal-01 边界）。
///
/// # Errors
///
/// herdr 不可达或输出形不符时返回 `String`（整趟拒跑，不做半吊子自愈）。
pub fn run_heal(
    home: &Path,
    only: Option<&Path>,
    dry_run: bool,
) -> Result<Vec<HealReport>, String> {
    let panes = parse_agent_list(&herdr_output(&["agent", "list"])?)?;
    let now = unix_secs();
    let dir = goal_dir(home);
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_stem()
                    .is_some_and(|s| !s.to_string_lossy().ends_with(".tmp"))
        })
        .collect();
    files.sort();
    let only_slug = only.map(|r| slug(&r.display().to_string()));
    let mut out = Vec::new();
    for path in files {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(want) = &only_slug {
            if &stem != want {
                continue;
            }
        }
        let mut rec = match read_record(&path) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => {
                out.push(HealReport {
                    slug: stem,
                    action: "skip-corrupt".into(),
                    detail: e.to_string(),
                });
                continue;
            }
        };
        match heal_verdict(&rec, &stem, &panes, now) {
            HealVerdict::Wake { pane } => {
                if dry_run {
                    out.push(HealReport {
                        slug: stem,
                        action: "would-wake".into(),
                        detail: format!(
                            "pane={pane} rounds={} streak={}",
                            rec.rounds, rec.heal_streak
                        ),
                    });
                    continue;
                }
                let project = panes
                    .iter()
                    .find(|p| p.pane_id == pane)
                    .map(|p| p.cwd.clone())
                    .unwrap_or_else(|| stem.clone());
                herdr_output(&["agent", "prompt", &pane, &wake_text(&rec, &project)])?;
                rec.last_heal_at = now;
                rec.heal_streak = if rec.rounds == rec.heal_rounds {
                    rec.heal_streak + 1
                } else {
                    1
                };
                rec.heal_rounds = rec.rounds;
                write_record(&path, &rec).map_err(|e| e.to_string())?;
                out.push(HealReport {
                    slug: stem,
                    action: "woke".into(),
                    detail: format!(
                        "pane={pane} rounds={} streak={}",
                        rec.rounds, rec.heal_streak
                    ),
                });
            }
            HealVerdict::SkipWorking { panes } => out.push(HealReport {
                slug: stem,
                action: "skip-working".into(),
                detail: format!("panes={panes}"),
            }),
            HealVerdict::SkipBlocked => out.push(HealReport {
                slug: stem,
                action: "skip-blocked".into(),
                detail: "等人裁态豁免".into(),
            }),
            HealVerdict::SkipThrottled => out.push(HealReport {
                slug: stem,
                action: "skip-throttled".into(),
                detail: format!("静默窗 {}s 内", HEAL_IDLE_SECS),
            }),
            HealVerdict::SkipStormCapped => out.push(HealReport {
                slug: stem,
                action: "skip-storm-capped".into(),
                detail: format!(
                    "连续 {} 次唤醒无 rounds 增长，转人工（重开会话或核 hst doctor check=goal）",
                    rec.heal_streak
                ),
            }),
            HealVerdict::NoAgent => out.push(HealReport {
                slug: stem,
                action: "no-agent".into(),
                detail: "无归属格（会话已关未清？确认后 hst goal clear 收尾）".into(),
            }),
        }
    }
    Ok(out)
}

/// 扫描盖章限频（goal-01 澄清令「事件驱动、零轮询」面的频控外闸）：
/// heal_sweep 挂在每轮 Stop 事件流上，任一 Stop 至多触发一次全量扫描
/// 每 [`SWEEP_MIN_GAP_SECS`]（盖章文件 `.heal-sweep` 存上次扫描时刻；非
/// 轮询 daemon，纯事件到点判）。
pub const SWEEP_MIN_GAP_SECS: u64 = 5 * 60;

fn sweep_stamp(home: &Path) -> PathBuf {
    goal_dir(home).join(".heal-sweep")
}

fn sweep_due(home: &Path, now: u64) -> bool {
    match std::fs::read_to_string(sweep_stamp(home))
        .ok()
        .and_then(|t| t.trim().parse::<u64>().ok())
    {
        Some(last) => now.saturating_sub(last) >= SWEEP_MIN_GAP_SECS,
        None => true,
    }
}

/// 自愈留痕（goal-01 澄清令「可观测」面）：单行追加 `.heal-log`
///（ts 加 slug 加 action 加 detail），best-effort。
fn heal_log_append(home: &Path, slug: &str, action: &str, detail: &str) {
    let p = goal_dir(home).join(".heal-log");
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .and_then(|mut f| {
            use std::io::Write as _;
            writeln!(f, "{} {} {} {}", unix_secs(), slug, action, detail)
        });
}

/// hook 事件流挂载的自愈扫描入口（goal-01 澄清令：自愈须事件驱动实时，
/// 挂 Stop 事件流当场判定，零 cron 零落盘轮询零外部定时）：goal 腿处理完
/// 本会话提醒并已输出后调用；盖章限频内直接让位（零子进程开销），
/// 过章才扫（herdr list 有界 4s，唤醒 prompt 有界 8s，全 fail-open 不影
/// 响主链）。`HST_GOAL_HEAL=off` 为逃生闸（诊断/测试面）。返回 Some(回执
/// 行数) = 本轮实扫，None = 未到点或关闸。
///
/// # Errors
///
/// herdr 不可达或扫描内部失败时返回 `String`（调用方仅 verbose 打点）。
pub fn heal_sweep(home: &Path) -> Result<Option<usize>, String> {
    if std::env::var_os("HST_GOAL_HEAL").is_some_and(|v| v == "off") {
        return Ok(None);
    }
    let now = unix_secs();
    if !sweep_due(home, now) {
        return Ok(None);
    }
    // 先盖章后扫（并发 Stop 竞态最多多扫一趟，per-goal 频控兜底）。
    if let Some(d) = sweep_stamp(home).parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(sweep_stamp(home), now.to_string());
    let rows = run_heal(home, None, false)?;
    for r in &rows {
        if matches!(
            r.action.as_str(),
            "woke" | "skip-storm-capped" | "no-agent" | "skip-corrupt"
        ) {
            heal_log_append(home, &r.slug, &r.action, &r.detail);
        }
    }
    Ok(Some(rows.len()))
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
        // goal-01 件 3：提醒附用法（agent 自行操作面）；插队件：受阻分支
        // 补自主停机指引（停机权在 agent，勿徒劳死循环）。
        assert!(out.contains("hst goal 用法"), "{out}");
        assert!(out.contains("hst goal set"), "{out}");
        assert!(out.contains("停机权在你"), "{out}");
        assert!(out.contains("徒劳死循环"), "{out}");
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
        // goal-01 根因 B 修：子目录 cwd 上溯命中项目根 goal（计数落同一文件）。
        let subdir = json!({ "hook_event_name": "Stop", "cwd": "/w/p/sub/deep" });
        let out2 = stop_hook(&home, Some(&subdir), None).unwrap().unwrap();
        assert!(
            out2.contains("闭环 REQ 收尾"),
            "subdir walk hits parent goal: {out2}"
        );
        assert_eq!(show_goal(&home, root).unwrap().unwrap().rounds, 3);
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
    fn legacy_record_without_heal_fields_loads_with_defaults() {
        // goal-01：存量 v2.10.0 记录无 heal 三字段，serde default 兼容载入。
        let home = fresh_home("lg");
        let root = Path::new("/w/old");
        let p = goal_path(&home, root);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(
            &p,
            r#"{"text":"旧记录","set_at":100,"rounds":5,"last_stop_at":200}"#,
        )
        .unwrap();
        let rec = show_goal(&home, root).unwrap().unwrap();
        assert_eq!(rec.rounds, 5);
        assert_eq!(rec.last_heal_at, 0);
        assert_eq!(rec.heal_streak, 0);
        assert_eq!(rec.heal_rounds, 0);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn goal_file_for_walks_ancestors_deepest_first() {
        let home = fresh_home("wk");
        let proj = Path::new("/w/proj");
        let sub_goal = Path::new("/w/proj/sub");
        set_goal(&home, proj, "项目根 goal").unwrap();
        assert_eq!(
            goal_file_for(&home, Path::new("/w/proj/a/b")),
            Some(goal_path(&home, proj)),
            "无更深层 goal 时上溯命中项目根"
        );
        assert_eq!(goal_file_for(&home, Path::new("/w/other")), None);
        // 深者胜：子目录自有 goal 优先。
        set_goal(&home, sub_goal, "子目录 goal").unwrap();
        assert_eq!(
            goal_file_for(&home, Path::new("/w/proj/sub/x")),
            Some(goal_path(&home, sub_goal))
        );
        assert_eq!(
            goal_file_for(&home, proj),
            Some(goal_path(&home, proj)),
            "根路径仍键根"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn parse_agent_list_projects_three_fields() {
        let raw = r#"{"id":"cli:agent:list","result":{"agents":[
            {"agent":"claude","agent_status":"idle","cwd":"/mnt/wsl/repos/pi-rs","pane_id":"w2T:p1"},
            {"agent":"kimi","agent_status":"blocked","cwd":"/x","pane_id":"w1:p2"},
            {"agent":"grok","agent_status":"done","cwd":"/y","pane_id":"w1:p3"}
        ]}}"#;
        let v = parse_agent_list(raw).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].pane_id, "w2T:p1");
        assert_eq!(v[0].status, "idle");
        assert!(parse_agent_list("not-json").is_err());
        assert!(parse_agent_list(r#"{"result":{}}"#).is_err());
    }

    #[test]
    fn heal_verdict_exemptions_and_wake_matrix() {
        let mk = |stop: u64, heal_at: u64, streak: u64| GoalRecord {
            text: "t".into(),
            set_at: 1,
            rounds: 1,
            last_stop_at: stop,
            last_heal_at: heal_at,
            heal_streak: streak,
            heal_rounds: 1,
        };
        let now = 10_000u64;
        let slug = "-w-proj";
        let pane = |status: &str, cwd: &str| PaneFace {
            pane_id: "w1:p1".into(),
            agent: "claude".into(),
            status: status.into(),
            cwd: cwd.into(),
        };
        // working：不扰。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("working", "/w/proj")], now),
            HealVerdict::SkipWorking { panes: 1 }
        ));
        // blocked 等人裁：豁免（用户令）。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("blocked", "/w/proj")], now),
            HealVerdict::SkipBlocked
        ));
        // 静默窗内（last_stop 距今不足 HEAL_IDLE_SECS）：频控。
        assert!(matches!(
            heal_verdict(
                &mk(now - HEAL_IDLE_SECS + 60, 1, 0),
                slug,
                &[pane("idle", "/w/proj")],
                now
            ),
            HealVerdict::SkipThrottled
        ));
        // 风暴闸到顶：转人工。
        assert!(matches!(
            heal_verdict(
                &mk(1, 1, HEAL_STORM_CAP),
                slug,
                &[pane("done", "/w/proj")],
                now
            ),
            HealVerdict::SkipStormCapped
        ));
        // 停摆过窗：唤醒（子目录格位经上溯归属）。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("idle", "/w/proj/sub/deep")], now),
            HealVerdict::Wake { .. }
        ));
        // 无归属格：孤儿。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("idle", "/w/elsewhere")], now),
            HealVerdict::NoAgent
        ));
    }

    #[test]
    fn wake_text_carries_goal_and_usage() {
        let rec = GoalRecord {
            text: "解完 t 题".into(),
            set_at: 1,
            rounds: 2,
            last_stop_at: 0,
            last_heal_at: 0,
            heal_streak: 0,
            heal_rounds: 0,
        };
        let t = wake_text(&rec, "/mnt/wsl/repos/pi-rs");
        assert!(t.contains("/mnt/wsl/repos/pi-rs"), "{t}");
        assert!(t.contains("解完 t 题"), "{t}");
        assert!(
            t.contains("hst goal clear") && t.contains("hst goal 用法"),
            "{t}"
        );
        assert!(
            t.contains("停机权在你") && t.contains("徒劳死循环"),
            "插队件自主停机指引: {t}"
        );
        assert!(!t.contains('\n'), "单行契约: {t}");
    }

    #[cfg(unix)]
    #[test]
    fn run_heal_end_to_end_with_fake_herdr() {
        use std::io::Write as _;
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = fresh_home("rh");
        let proj = Path::new("/w/proj");
        set_goal(&home, proj, "闭环 t 题").unwrap();
        // 拨老记录：刚设的 goal 在静默窗内（正确不唤），测试直取停摆态。
        let mut aged = show_goal(&home, proj).unwrap().unwrap();
        aged.set_at = 1;
        aged.last_stop_at = 1;
        write_record(&goal_path(&home, proj), &aged).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hst-goal-herdr-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let list = dir.join("list.json");
        let log = dir.join("prompt.log");
        std::fs::write(
            &list,
            format!(
                r#"{{"result":{{"agents":[{{"agent":"claude","agent_status":"idle","cwd":"{}","pane_id":"w1:p9"}}]}}}}"#,
                proj.display()
            ),
        )
        .unwrap();
        let script = bin.join("herdr");
        std::fs::write(
            &script,
            concat!(
                "#!/bin/sh\n",
                "if [ \"$1 $2\" = \"agent list\" ]; then cat \"$HERDR_FAKE_LIST\"; exit 0; fi\n",
                "if [ \"$1 $2\" = \"agent prompt\" ]; then printf '%s\\n' \"$4\" >> \"$HERDR_FAKE_LOG\"; fi\n",
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        std::env::set_var("HERDR_FAKE_LIST", &list);
        std::env::set_var("HERDR_FAKE_LOG", &log);
        std::env::set_var("PATH", &path);
        // dry-run：只报不唤不落盘。
        let rows = run_heal(&home, Some(proj), true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "would-wake", "{:?}", rows[0].detail);
        assert!(!log.exists());
        // 实跑：唤醒一次，计数落盘。
        let rows = run_heal(&home, Some(proj), false).unwrap();
        assert_eq!(rows[0].action, "woke", "{:?}", rows[0].detail);
        let woke_log = std::fs::read_to_string(&log).unwrap();
        assert!(woke_log.contains("闭环 t 题") && woke_log.contains("hst goal clear"));
        let rec = show_goal(&home, proj).unwrap().unwrap();
        assert_eq!(rec.heal_streak, 1);
        assert!(rec.last_heal_at > 0);
        // 紧跟第二趟：静默窗频控跳过（无新 prompt）。
        let rows = run_heal(&home, Some(proj), false).unwrap();
        assert_eq!(rows[0].action, "skip-throttled", "{:?}", rows[0].detail);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), woke_log);
        // 风暴闸：伪造 streak 到顶后转人工。
        let mut rec = show_goal(&home, proj).unwrap().unwrap();
        rec.heal_streak = HEAL_STORM_CAP;
        rec.last_heal_at = 1;
        write_record(&goal_path(&home, proj), &rec).unwrap();
        let rows = run_heal(&home, Some(proj), false).unwrap();
        assert_eq!(rows[0].action, "skip-storm-capped", "{:?}", rows[0].detail);
        // heal_sweep（goal-01 澄清令事件驱动面）：off 闸让位、实扫唤醒加
        // 留痕（.heal-log 与 .heal-sweep）、盖章限频二连让位。
        std::env::set_var("HST_GOAL_HEAL", "off");
        assert_eq!(heal_sweep(&home).unwrap(), None, "off 闸让位");
        std::env::remove_var("HST_GOAL_HEAL");
        let _ = std::fs::remove_file(goal_dir(&home).join(".heal-sweep"));
        let mut rec = show_goal(&home, proj).unwrap().unwrap();
        rec.last_stop_at = 1;
        rec.last_heal_at = 0;
        rec.heal_streak = 0;
        write_record(&goal_path(&home, proj), &rec).unwrap();
        assert_eq!(heal_sweep(&home).unwrap(), Some(1), "首扫实跑唤醒");
        assert!(goal_dir(&home).join(".heal-sweep").is_file(), "盖章落盘");
        let log_text = std::fs::read_to_string(goal_dir(&home).join(".heal-log")).unwrap();
        assert!(log_text.contains("woke"), "留痕: {log_text}");
        assert_eq!(heal_sweep(&home).unwrap(), None, "盖章期内二连让位");
        std::env::remove_var("HERDR_FAKE_LIST");
        std::env::remove_var("HERDR_FAKE_LOG");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
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
