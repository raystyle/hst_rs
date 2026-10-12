//! hst goal 自治面（REQ-045，REQ-019 演进；用户令 2026-10-11「hst 不再
//! 关联 goal 和 loop」「hst 自己用 hook 实现 goal」）：goal 文本由 hst
//! 自有存储承载（`~/.hst/state/goal/<slug>.json`，slug 规则同 trace 与
//! projyolo：路径非字母数字一律 `-`），与项目 `.claude/scheduled_tasks.json`
//! 的一切读写解除（该文件回归 Claude Code 原生 CronCreate 域，loop 面
//! v2.10.0 退役）。Stop 事件腿（`hst hook goal`，注册面 Claude Stop 事件
//! 与 state 腿双挂，仅 claude）：每轮 agent 停止 rounds 计数落盘（状态栏
//! goal 段的事件驱动数据源），自然停（payload 无 stop_hook_active）出
//! additionalContext 提醒注入一轮（goal-02 干净显示形，续跑同 decision
//! block 语义）、续跑引发的再停放行（Claude
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
//!
//! REQ-047（2026-10-12 架构批）：自驱机制升两维态。检测三路（Stop 事件
//! 流加 herdr 格态加 Notification/PermissionRequest 带内快道）；判定矩阵
//! 六序（no-agent、working 不扰、blocked 分型、idle 唤醒、频控、风暴）；
//! blocked 不再静默豁免，alarm 报警留痕（`.heal-log` 加 show 面），预授权
//! （`hst goal yolo-heal on`）下反复 blocked 触发 yolo 治（落键加重启格加
//! 续 goal）；间隔自驱 = hook 起 sweeper 有界子进程（60 秒 tick、2 小时
//! 寿命、`.sweeper-lock` 单实例、90 秒陈旧接管），`HST_GOAL_SWEEPER=off`
//! 逃生闸。wake 文本 v2 自包含五件（goal 文本加上轮收束态加即刻推进加
//! 防卡死防空转边界加用法）。

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
    /// 连续 blocked 目击数（rounds 增长即归零；yolo 治阈值输入。REQ-047）。
    #[serde(default)]
    pub blocked_streak: u64,
    /// 上次 blocked 目击时的 rounds 快照（增长判归零依据。REQ-047）。
    #[serde(default)]
    pub blocked_rounds: u64,
    /// 最近一次 blocked 报警时刻（epoch 秒；0 = 未报过。防抖窗输入。
    /// REQ-047）。
    #[serde(default)]
    pub blocked_alarm_at: u64,
    /// 最近一次 yolo 治时刻（epoch 秒；0 = 未治过。冷却窗输入。REQ-047）。
    #[serde(default)]
    pub yolo_heal_at: u64,
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
        blocked_streak: 0,
        blocked_rounds: 0,
        blocked_alarm_at: 0,
        yolo_heal_at: 0,
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

/// Stop 提醒 JSON（goal-02 输出面调优）：`hookSpecificOutput.additionalContext`
/// 形（Claude Code Stop 契约，2.1.163 起），续跑语义与 `decision: block`
/// 同（同 `stop_hook_active` 防环加 8 连续续跑帽），但 transcript 标注
/// `Stop hook feedback` 且不发 hook error 通知——消除「Stop hook error:」
/// 观感（用户报 2026-10-11，文档引文在 REQ-046 goal-02 注记）。
///
/// # Errors
///
/// 序列化失败时返回 `String`（纯字符串组合，实跑不可达，类型面完整）。
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
    serde_json::to_string(&json!({
        "hookSpecificOutput": {
            "hookEventName": "Stop",
            "additionalContext": reason,
        }
    }))
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
/// stop_hook_active）返回 additionalContext 提醒 JSON（续跑同 decision
/// block 语义，goal-02 干净显示形）、续跑再停（stop_hook_active=true）
/// 返回 None。计数落盘恒执行（防环放行轮也计），状态栏 goal 段数据随之
/// 刷新。
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

/// blocked 报警防抖窗（REQ-047）：同 goal 两次报警的最小间隔（streak 仍
/// 计数，报警静默）。
pub const BLOCKED_ALARM_SECS: u64 = 15 * 60;

/// yolo 治阈值（REQ-047）：blocked_streak 到此值即「反复 blocked」，预授权
/// 下触发治序列。
pub const BLOCKED_STORM_THRESHOLD: u64 = 2;

/// yolo 治冷却窗（REQ-047）：两次治动作的最小间隔（成败同计，防锤）。
pub const YOLO_HEAL_COOLDOWN_SECS: u64 = 6 * 3600;

/// sweeper tick 间隔（REQ-047 间隔自驱）：有界子进程的检查节拍。
pub const SWEEPER_TICK_SECS: u64 = 60;

/// sweeper 单次寿命（REQ-047 有界性）：到点即退，续命须新 hook 事件
/// ensure（非 daemon）。
pub const SWEEPER_LIFE_SECS: u64 = 2 * 3600;

/// sweeper 锁陈旧阈（REQ-047 单实例）：心跳重写超此时长即判死锁接管。
pub const SWEEPER_STALE_SECS: u64 = 90;

/// herdr 格状态面（`herdr agent list` 的 result.agents 投影，消费面只读
/// 四字段）。
pub struct PaneFace {
    /// 格位 id（唤醒目标，如 `w2T:p1`）。
    pub pane_id: String,
    /// agent 名（claude/codex/kimi/grok）。
    pub agent: String,
    /// 生命周期态（idle/working/blocked/done）。
    pub status: String,
    /// 格位工作目录（与 goal 项目键做上溯匹配）。
    pub cwd: String,
    /// herdr 格名（yolo 治 fresh 起复用；无名的格为空。REQ-047）。
    pub name: String,
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
                name: a
                    .get("name")
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
/// 刻）。判定口径：goal 已清即无文件不入本判；working 不扰；blocked 分型
/// 处置（REQ-047：不再静默豁免，报警加预授权 yolo 治由引擎面承接）；静默
/// 窗内不唤醒；风暴闸到顶不唤醒。
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
    /// blocked 维命中：工位卡审批或选择屏（REQ-047 分型处置面）。
    Blocked {
        /// 卡阻格位。
        pane: String,
        /// 格位 agent 名（claude/codex/kimi/grok，yolo 治 kind 面）。
        agent: String,
        /// herdr 格名（fresh 起复用；无名格为空）。
        name: String,
    },
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
    if let Some(p) = mine.iter().find(|p| p.status == "blocked") {
        return HealVerdict::Blocked {
            pane: p.pane_id.clone(),
            agent: p.agent.clone(),
            name: p.name.clone(),
        };
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

/// 自愈唤醒文本 v2（REQ-047 自包含五件：goal 文本加上轮收束态加即刻推进
/// 指令加防卡死防空转边界加用法；herdr agent prompt 载体，单行契约）。
pub fn wake_text(rec: &GoalRecord, project: &str, now: u64) -> String {
    let last = rec.last_stop_at.max(rec.last_heal_at).max(rec.set_at);
    let idle_min = now.saturating_sub(last) / 60;
    format!(
        "hst goal 自驱唤醒:项目 {} 的 goal 在役:「{}」。上轮收束态:第 {} 轮,上次活动距今约 {} 分钟,\
         期间自愈唤醒 {} 次未转化。即刻推进:立即选定 goal 的下一个最小可行动子项并开始执行,\
         不要等待也不要只做计划;若 goal 已达成,请运行 hst goal clear 收束并如实简报;\
         若受阻,说明受阻点与所需人裁。防卡死防空转边界:同一失败路径不得重复尝试超过一次;\
         禁止只汇报不行动;若连续两轮无实质推进(无代码或测试或结论产出),判定为空转,\
         请自行 hst goal clear 停止并如实说明,停机权在你,勿徒劳死循环。\
         hst goal 用法:hst goal set <新目标> 改目标、hst goal show 查看、hst goal clear 清空收束。",
        project,
        flatten_inline(&rec.text),
        rec.rounds,
        idle_min,
        rec.heal_streak
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
    let budget: u128 = if args.first().is_some_and(|a| *a == "agent") {
        match args.get(1).copied() {
            Some("list") => 4_000,
            // start 等交互就绪默认 30s，预算放宽到 35s（治序列专用，只出
            // 现在 sweeper 与手动面，不进 hook 事件路径）。
            Some("start") => 35_000,
            _ => 8_000,
        }
    } else {
        8_000
    };
    herdr_output_bounded(args, budget)
}

/// `herdr agent get` 输出的格态投影（None = 无格态，含输出坏形与格已退
/// 场两义；治序列等格回 shell 的轮询判据）。
fn agent_status_of(text: &str) -> Option<String> {
    let v: Json = serde_json::from_str(text.trim()).ok()?;
    v.get("result")
        .and_then(|r| r.get("agent"))
        .and_then(|a| a.get("agent_status"))
        .and_then(|x| x.as_str())
        .map(str::to_string)
}

/// yolo 治预授权标记路径（`~/.hst/state/goal/.yolo-heal-armed`）。
fn yolo_heal_armed_path(home: &Path) -> PathBuf {
    goal_dir(home).join(".yolo-heal-armed")
}

/// yolo 治预授权读侧（REQ-047）：标记文件在场即 armed。
pub fn yolo_heal_armed(home: &Path) -> bool {
    yolo_heal_armed_path(home).is_file()
}

/// yolo 治预授权写侧（`hst goal yolo-heal on|off`）。返回是否真改动
///（重复 on 或重复 off 零改动合法）。
///
/// # Errors
///
/// IO 失败时返回 `String`。
pub fn set_yolo_heal_armed(home: &Path, on: bool) -> Result<bool, String> {
    let p = yolo_heal_armed_path(home);
    if on {
        if p.is_file() {
            return Ok(false);
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(&p, unix_secs().to_string()).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(true)
    } else if p.is_file() {
        std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// yolo 治序列（REQ-047,参考 actl 面 herdr yolo 原语）:落键加重启格加
/// fresh 起加续 goal。blocked 格 prompt 被 herdr 拒（agent_blocked），重启
/// 走原始键序 esc 加 /exit 加 enter;有界轮询 agent get 至格态消失（回
/// shell）,fresh 起复用格名（无名格按 slug 造 goal-heal 前缀名）,新会话
/// prompt 注入 wake v2 续跑 goal。
///
/// # Errors
///
/// 任一步失败返回 `String`（调用方落冷却防锤并留痕）。
fn yolo_heal_sequence(
    user_home: &Path,
    pane: &str,
    agent: &str,
    name: &str,
    rec: &GoalRecord,
    project: &str,
    now: u64,
) -> Result<String, String> {
    // 1 落键:四家用户级 full(即 hst init --yolo full 的落键半体)。
    crate::yolo::apply_user_yolo_level_with(user_home, crate::yolo::YoloLevel::Full)
        .map_err(|e| format!("落键: {e}"))?;
    // 2 重启格:原始键序退出(blocked 态 prompt 通道被拒)。
    herdr_output(&["agent", "send-keys", pane, "esc"])?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    herdr_output(&["agent", "send-keys", pane, "/exit"])?;
    herdr_output(&["agent", "send-keys", pane, "enter"])?;
    // 有界轮询等格回 shell(格态消失即回)。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let gone = herdr_output(&["agent", "get", pane])
            .ok()
            .and_then(|t| agent_status_of(&t))
            .is_none();
        if gone {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("等格回 shell 超时(15s)".into());
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    // 3 fresh 起:复用格名,无名格造名。
    let start_name = if name.is_empty() {
        let s = slug(project);
        let cut = s.len().min(24);
        format!("goal-heal-{}", &s[..cut])
    } else {
        name.to_string()
    };
    herdr_output(&[
        "agent",
        "start",
        &start_name,
        "--kind",
        agent,
        "--pane",
        pane,
    ])
    .map_err(|e| format!("fresh 起: {e}"))?;
    // 4 续 goal:wake v2 注入新会话。
    herdr_output(&["agent", "prompt", pane, &wake_text(rec, project, now)])
        .map_err(|e| format!("续 goal: {e}"))?;
    Ok(format!("pane={pane} name={start_name}"))
}

/// 自愈扫描入口（`hst goal heal`）：扫在册 goal（`only` 给定则单项目），
/// 逐条判定后分型处置（REQ-047 两维态矩阵）：Wake 形经 `herdr agent
/// prompt` 唤醒并落 last_heal_at 加风暴闸计数；Blocked 形报警留痕
///（streak 递增加 `.heal-log` alarm-blocked 行），预授权（arm 标记）且
/// streak 到阈值且工位非 yolo 且过冷却时走 yolo 治序列（`heavy` 为真才
/// 执行：治动作含重启格至多 50 秒，hook 事件路径传 false 只报警让位
/// sweeper 或手动面执行）。dry_run 只报判定不落盘不唤不发。herdr 消费面
/// 只读 agent list 加单发 prompt 加治序列原语（send-keys/get/start），
/// 不改 herdr 仓。
///
/// # Errors
///
/// herdr 不可达或输出形不符时返回 `String`（整趟拒跑，不做半吊子自愈）。
pub fn run_heal(
    home: &Path,
    user_home: &Path,
    only: Option<&Path>,
    dry_run: bool,
    heavy: bool,
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
                herdr_output(&["agent", "prompt", &pane, &wake_text(&rec, &project, now)])?;
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
            HealVerdict::Blocked { pane, agent, name } => {
                // streak：rounds 有增长先归零（新活动周期），再目击递增。
                bump_blocked_streak(&mut rec);
                let alarm_due = now.saturating_sub(rec.blocked_alarm_at) >= BLOCKED_ALARM_SECS;
                let eligible = heavy
                    && yolo_heal_armed(home)
                    && rec.blocked_streak >= BLOCKED_STORM_THRESHOLD
                    && !crate::yolo::user_yolo_active_with(user_home, &agent)
                    && now.saturating_sub(rec.yolo_heal_at) >= YOLO_HEAL_COOLDOWN_SECS;
                let mut detail = format!("pane={pane} agent={agent} streak={}", rec.blocked_streak);
                // 报警先行（委托令「blocked 即先报警留痕」）：防抖窗过即报，
                // 与是否治互不 gating。
                if alarm_due && !dry_run {
                    rec.blocked_alarm_at = now;
                    heal_log_append(home, &stem, "alarm-blocked", &detail);
                }
                let action = if eligible {
                    if dry_run {
                        "would-yolo-heal"
                    } else {
                        let project = panes
                            .iter()
                            .find(|p| p.pane_id == pane)
                            .map(|p| p.cwd.clone())
                            .unwrap_or_else(|| stem.clone());
                        match yolo_heal_sequence(
                            user_home, &pane, &agent, &name, &rec, &project, now,
                        ) {
                            Ok(d) => {
                                rec.yolo_heal_at = now;
                                detail.push_str(&format!(" heal=ok:{d}"));
                                heal_log_append(home, &stem, "yolo-heal", &detail);
                                "yolo-heal"
                            }
                            Err(e) => {
                                // 失败同落冷却（防锤），下窗再试。
                                rec.yolo_heal_at = now;
                                detail.push_str(&format!(" heal=fail:{e}"));
                                heal_log_append(home, &stem, "yolo-heal-fail", &detail);
                                "yolo-heal-fail"
                            }
                        }
                    }
                } else if alarm_due {
                    if dry_run {
                        "would-alarm-blocked"
                    } else {
                        "alarm-blocked"
                    }
                } else {
                    // 防抖窗内：streak 已计，报警静默。
                    "blocked-muted"
                };
                if !dry_run {
                    write_record(&path, &rec).map_err(|e| e.to_string())?;
                }
                out.push(HealReport {
                    slug: stem,
                    action: action.into(),
                    detail,
                });
            }
            HealVerdict::SkipWorking { panes } => out.push(HealReport {
                slug: stem,
                action: "skip-working".into(),
                detail: format!("panes={panes}"),
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

/// blocked 目击计数（纯函数）：rounds 较上次目击快照有增长即归零重计
///（新活动周期），随后目击递增（快照同步落）。
fn bump_blocked_streak(rec: &mut GoalRecord) {
    if rec.rounds != rec.blocked_rounds {
        rec.blocked_streak = 0;
        rec.blocked_rounds = rec.rounds;
    }
    rec.blocked_streak += 1;
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
/// 挂 Stop 事件流当场判定；REQ-047 扩 SessionEnd 收束事件）：goal 腿处理完
/// 本会话提醒并已输出后调用；盖章限频内直接让位（零子进程开销），
/// 过章才扫（herdr list 有界 4s，唤醒 prompt 有界 8s，全 fail-open 不影
/// 响主链）。治序列不在此路执行（heavy=false，hook 10s 预算装不下重启
/// 格，让位 sweeper 与手动面）。`HST_GOAL_HEAL=off` 为逃生闸（诊断/测试
/// 面）。返回 Some(回执行数) = 本轮实扫，None = 未到点或关闸。
///
/// # Errors
///
/// herdr 不可达或扫描内部失败时返回 `String`（调用方仅 verbose 打点）。
pub fn heal_sweep(home: &Path, user_home: &Path) -> Result<Option<usize>, String> {
    if heal_face_off() {
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
    let rows = run_heal(home, user_home, None, false, false)?;
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

/// heal 面总逃生闸（既有）：off 关整个自愈面（sweep、快道、sweeper）。
fn heal_face_off() -> bool {
    std::env::var_os("HST_GOAL_HEAL").is_some_and(|v| v == "off")
}

/// sweeper 逃生闸（REQ-047）：off 只关间隔子进程面（ensure 不起、loop
/// 即退），事件 sweep 与手动 heal 不受影响。
fn sweeper_off() -> bool {
    std::env::var_os("HST_GOAL_SWEEPER").is_some_and(|v| v == "off")
}

/// goal 目录是否有在册 goal（sweeper 续命与起动的前置判）。
fn goal_dir_has_goals(home: &Path) -> bool {
    std::fs::read_dir(goal_dir(home))
        .map(|rd| {
            rd.flatten().any(|e| {
                let p = e.path();
                p.extension().is_some_and(|x| x == "json")
                    && p.file_stem()
                        .is_some_and(|s| !s.to_string_lossy().starts_with('.'))
            })
        })
        .unwrap_or(false)
}

/// sweeper 单实例锁（`.sweeper-lock`，内容 = 心跳 epoch 秒）。
fn sweeper_lock_path(home: &Path) -> PathBuf {
    goal_dir(home).join(".sweeper-lock")
}

/// 锁心跳时刻（无锁或坏形 = None）。
fn sweeper_lock_ts(home: &Path) -> Option<u64> {
    std::fs::read_to_string(sweeper_lock_path(home))
        .ok()
        .and_then(|t| t.trim().parse::<u64>().ok())
}

/// 锁是否鲜活（心跳在陈旧阈内 = 有活 sweeper 在场）。
fn sweeper_lock_fresh(home: &Path, now: u64) -> bool {
    sweeper_lock_ts(home).is_some_and(|ts| now.saturating_sub(ts) < SWEEPER_STALE_SECS)
}

/// 抢锁（单实例防重入）：create_new 原子建；已在场且鲜活即让位，陈旧即
/// 接管（删后重试一次）。
fn sweeper_lock_acquire(home: &Path, now: u64) -> bool {
    let p = sweeper_lock_path(home);
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    use std::io::Write as _;
    let try_once = || -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)?;
        writeln!(f, "{now}")
    };
    if try_once().is_ok() {
        return true;
    }
    if sweeper_lock_fresh(home, now) {
        return false;
    }
    // 陈旧接管：删旧重试（竞态下两家同删，create_new 只成一家）。
    let _ = std::fs::remove_file(&p);
    try_once().is_ok()
}

/// 锁心跳重写（每 tick 落一次，续鲜）。
fn sweeper_heartbeat(home: &Path, now: u64) {
    let _ = std::fs::write(sweeper_lock_path(home), now.to_string());
}

/// sweeper 自有盖章（`.sweeper-sweep`，与事件路 `.heal-sweep` 分钟各表：
/// 两驱动各按 5 分钟限频，互不挤兑）。
fn sweeper_stamp_path(home: &Path) -> PathBuf {
    goal_dir(home).join(".sweeper-sweep")
}

fn sweeper_due(home: &Path, now: u64) -> bool {
    match std::fs::read_to_string(sweeper_stamp_path(home))
        .ok()
        .and_then(|t| t.trim().parse::<u64>().ok())
    {
        Some(last) => now.saturating_sub(last) >= SWEEP_MIN_GAP_SECS,
        None => true,
    }
}

/// hook 起 sweeper（REQ-047 间隔自驱的起动面）：goal 在册且无鲜活锁即
/// detach 起有界子进程（`hst goal heal --interval`），unix 下独立进程组
/// （hook 超时组杀不连坐）。零目标零锁鲜零动作;失败静默（fail-open）。
pub fn ensure_sweeper(home: &Path) {
    if heal_face_off() || sweeper_off() || !goal_dir_has_goals(home) {
        return;
    }
    let now = unix_secs();
    if sweeper_lock_fresh(home, now) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["goal", "heal", "--interval"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        // 独立进程组:hook 腿被超时组杀时 sweeper 不连坐。
        cmd.process_group(0);
    }
    let _ = cmd.spawn();
}

/// sweeper 有界主循环（`hst goal heal --interval` 子进程体）：抢锁让位即
/// 退;tick 心跳,寿命到点或 goal 清空即退;到点实扫（heavy=true,治序列
/// 在此路执行）。非 daemon:单次寿命 [`SWEEPER_LIFE_SECS`],续命须新 hook
/// 事件 ensure。
///
/// # Errors
///
/// 扫描内部失败时返回 `String`（子进程 stderr 落幕,锁仍释放）。
pub fn sweeper_loop(home: &Path, user_home: &Path) -> Result<(), String> {
    if sweeper_off() {
        return Ok(());
    }
    let now = unix_secs();
    if !sweeper_lock_acquire(home, now) {
        return Ok(());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(SWEEPER_LIFE_SECS);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(SWEEPER_TICK_SECS));
        let now = unix_secs();
        sweeper_heartbeat(home, now);
        if !goal_dir_has_goals(home) || std::time::Instant::now() >= deadline {
            break;
        }
        if sweeper_due(home, now) {
            if let Some(d) = sweeper_stamp_path(home).parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(sweeper_stamp_path(home), now.to_string());
            let rows = run_heal(home, user_home, None, false, true)?;
            for r in &rows {
                if matches!(
                    r.action.as_str(),
                    "woke" | "skip-storm-capped" | "no-agent" | "skip-corrupt"
                ) {
                    heal_log_append(home, &r.slug, &r.action, &r.detail);
                }
            }
        }
    }
    let _ = std::fs::remove_file(sweeper_lock_path(home));
    Ok(())
}

/// blocked 带内快道（REQ-047 两维态的实时腿）：goal 腿挂上 Notification 与
/// PermissionRequest 后,payload 权限形（复用 hook 面 state_for_payload 判
/// blocked）即对该项目 goal 报警留痕加 streak 递增,零 herdr 依赖零
/// stdout（不扰权限决策面）。防抖窗内只计 streak 不重报。返回 Some(回执
/// 行) = 本事件实报,None = 不属 blocked 形或防抖让位。
///
/// # Errors
///
/// 存储坏损时返回 `String`（调用方 fail-open）。
pub fn blocked_fastpath(
    home: &Path,
    payload: Option<&Json>,
    event_arg: Option<&str>,
) -> Result<Option<String>, String> {
    if heal_face_off() {
        return Ok(None);
    }
    let ev = crate::hook::event_label(event_arg, payload);
    if crate::hook::state_for_payload(&ev, payload) != "blocked" {
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
    let now = unix_secs();
    bump_blocked_streak(&mut rec);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let session = payload
        .and_then(|v| {
            v.get("session_id")
                .or_else(|| v.get("sessionId"))
                .and_then(|x| x.as_str())
        })
        .unwrap_or("-");
    let detail = format!("session={session} streak={}", rec.blocked_streak);
    if now.saturating_sub(rec.blocked_alarm_at) >= BLOCKED_ALARM_SECS {
        rec.blocked_alarm_at = now;
        write_record(&path, &rec).map_err(|e| e.to_string())?;
        heal_log_append(home, &stem, "alarm-blocked-fast", &detail);
        Ok(Some(format!("{stem} alarm-blocked-fast {detail}")))
    } else {
        write_record(&path, &rec).map_err(|e| e.to_string())?;
        Ok(None)
    }
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
        assert!(
            out.contains("\"hookSpecificOutput\"") && out.contains("\"additionalContext\""),
            "goal-02 干净形: {out}"
        );
        assert!(out.contains("\"hookEventName\":\"Stop\""), "{out}");
        assert!(!out.contains("\"decision\""), "{out}");
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
        // REQ-047 四新字段同走 serde default 兼容。
        assert_eq!(rec.blocked_streak, 0);
        assert_eq!(rec.blocked_rounds, 0);
        assert_eq!(rec.blocked_alarm_at, 0);
        assert_eq!(rec.yolo_heal_at, 0);
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
            blocked_streak: 0,
            blocked_rounds: 0,
            blocked_alarm_at: 0,
            yolo_heal_at: 0,
        };
        let now = 10_000u64;
        let slug = "-w-proj";
        let pane = |status: &str, cwd: &str| PaneFace {
            pane_id: "w1:p1".into(),
            agent: "claude".into(),
            status: status.into(),
            cwd: cwd.into(),
            name: "goal-p1".into(),
        };
        // working：不扰。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("working", "/w/proj")], now),
            HealVerdict::SkipWorking { panes: 1 }
        ));
        // blocked 维命中（REQ-047 分型处置面）：带格位加 agent 加格名。
        assert!(matches!(
            heal_verdict(&mk(1, 1, 0), slug, &[pane("blocked", "/w/proj")], now),
            HealVerdict::Blocked {
                ref pane,
                ref agent,
                ref name,
            } if pane == "w1:p1" && agent == "claude" && name == "goal-p1"
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
    fn bump_blocked_streak_resets_on_rounds_growth() {
        let mut rec = GoalRecord {
            text: "t".into(),
            set_at: 1,
            rounds: 5,
            last_stop_at: 1,
            last_heal_at: 0,
            heal_streak: 0,
            heal_rounds: 0,
            blocked_streak: 2,
            blocked_rounds: 5,
            blocked_alarm_at: 0,
            yolo_heal_at: 0,
        };
        // rounds 无增长：目击递增。
        bump_blocked_streak(&mut rec);
        assert_eq!(rec.blocked_streak, 3);
        assert_eq!(rec.blocked_rounds, 5);
        // rounds 增长（新活动周期）：归零重计。
        rec.rounds = 9;
        bump_blocked_streak(&mut rec);
        assert_eq!(rec.blocked_streak, 1);
        assert_eq!(rec.blocked_rounds, 9);
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
            blocked_streak: 0,
            blocked_rounds: 0,
            blocked_alarm_at: 0,
            yolo_heal_at: 0,
        };
        let t = wake_text(&rec, "/mnt/wsl/repos/pi-rs", 1 + 3600);
        assert!(t.contains("/mnt/wsl/repos/pi-rs"), "{t}");
        assert!(t.contains("解完 t 题"), "{t}");
        // REQ-047 v2 五件：上轮收束态加即刻推进加防卡死防空转。
        assert!(t.contains("第 2 轮"), "上轮收束态轮次: {t}");
        assert!(t.contains("距今约 60 分钟"), "上轮收束态时长: {t}");
        assert!(t.contains("自愈唤醒 0 次未转化"), "{t}");
        assert!(t.contains("即刻推进"), "{t}");
        assert!(t.contains("最小可行动子项"), "{t}");
        assert!(t.contains("防卡死防空转边界"), "{t}");
        assert!(t.contains("同一失败路径不得重复"), "{t}");
        assert!(t.contains("空转"), "{t}");
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
        let user = fresh_home("rh-user");
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
        let rows = run_heal(&home, &user, Some(proj), true, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "would-wake", "{:?}", rows[0].detail);
        assert!(!log.exists());
        // 实跑：唤醒一次，计数落盘。
        let rows = run_heal(&home, &user, Some(proj), false, false).unwrap();
        assert_eq!(rows[0].action, "woke", "{:?}", rows[0].detail);
        let woke_log = std::fs::read_to_string(&log).unwrap();
        assert!(woke_log.contains("闭环 t 题") && woke_log.contains("hst goal clear"));
        let rec = show_goal(&home, proj).unwrap().unwrap();
        assert_eq!(rec.heal_streak, 1);
        assert!(rec.last_heal_at > 0);
        // 紧跟第二趟：静默窗频控跳过（无新 prompt）。
        let rows = run_heal(&home, &user, Some(proj), false, false).unwrap();
        assert_eq!(rows[0].action, "skip-throttled", "{:?}", rows[0].detail);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), woke_log);
        // 风暴闸：伪造 streak 到顶后转人工。
        let mut rec = show_goal(&home, proj).unwrap().unwrap();
        rec.heal_streak = HEAL_STORM_CAP;
        rec.last_heal_at = 1;
        write_record(&goal_path(&home, proj), &rec).unwrap();
        let rows = run_heal(&home, &user, Some(proj), false, false).unwrap();
        assert_eq!(rows[0].action, "skip-storm-capped", "{:?}", rows[0].detail);
        // heal_sweep（goal-01 澄清令事件驱动面）：off 闸让位、实扫唤醒加
        // 留痕（.heal-log 与 .heal-sweep）、盖章限频二连让位。
        std::env::set_var("HST_GOAL_HEAL", "off");
        assert_eq!(heal_sweep(&home, &user).unwrap(), None, "off 闸让位");
        std::env::remove_var("HST_GOAL_HEAL");
        let _ = std::fs::remove_file(goal_dir(&home).join(".heal-sweep"));
        let mut rec = show_goal(&home, proj).unwrap().unwrap();
        rec.last_stop_at = 1;
        rec.last_heal_at = 0;
        rec.heal_streak = 0;
        write_record(&goal_path(&home, proj), &rec).unwrap();
        assert_eq!(heal_sweep(&home, &user).unwrap(), Some(1), "首扫实跑唤醒");
        assert!(goal_dir(&home).join(".heal-sweep").is_file(), "盖章落盘");
        let log_text = std::fs::read_to_string(goal_dir(&home).join(".heal-log")).unwrap();
        assert!(log_text.contains("woke"), "留痕: {log_text}");
        assert_eq!(heal_sweep(&home, &user).unwrap(), None, "盖章期内二连让位");
        std::env::remove_var("HERDR_FAKE_LIST");
        std::env::remove_var("HERDR_FAKE_LOG");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&user);
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

    #[test]
    fn yolo_heal_arm_switch_roundtrip() {
        let home = fresh_home("arm");
        assert!(!yolo_heal_armed(&home), "缺省 off");
        assert!(set_yolo_heal_armed(&home, true).unwrap(), "on 真改动");
        assert!(yolo_heal_armed(&home));
        assert!(!set_yolo_heal_armed(&home, true).unwrap(), "重复 on 零改动");
        assert!(set_yolo_heal_armed(&home, false).unwrap(), "off 真改动");
        assert!(!yolo_heal_armed(&home));
        assert!(
            !set_yolo_heal_armed(&home, false).unwrap(),
            "重复 off 零改动"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sweeper_lock_acquire_fresh_and_stale_takeover() {
        let home = fresh_home("lk");
        let now = 10_000u64;
        assert!(sweeper_lock_acquire(&home, now), "无锁首抢成功");
        assert!(!sweeper_lock_acquire(&home, now), "鲜活锁第二家让位");
        assert!(sweeper_lock_fresh(&home, now));
        // 心跳陈旧：接管成功。
        let stale = now + SWEEPER_STALE_SECS + 10;
        assert!(!sweeper_lock_fresh(&home, stale));
        assert!(sweeper_lock_acquire(&home, stale), "陈旧锁接管");
        assert!(sweeper_lock_fresh(&home, stale));
        // 坏形锁（非数字）：判陈旧可接管。
        std::fs::write(sweeper_lock_path(&home), "junk").unwrap();
        assert!(!sweeper_lock_fresh(&home, stale));
        assert!(sweeper_lock_acquire(&home, stale));
        // 空目录判无 goal：sweeper 不续命。
        assert!(!goal_dir_has_goals(&home));
        set_goal(&home, Path::new("/w/p"), "t").unwrap();
        assert!(goal_dir_has_goals(&home));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn blocked_fastpath_alarms_and_debounces() {
        let home = fresh_home("fp");
        let root = Path::new("/w/p");
        set_goal(&home, root, "盯部署").unwrap();
        // 非权限形通知：零动作。
        let tips = json!({
            "hook_event_name": "Notification",
            "notification_type": "tip",
            "cwd": "/w/p",
            "session_id": "s1"
        });
        assert!(blocked_fastpath(&home, Some(&tips), None)
            .unwrap()
            .is_none());
        assert_eq!(show_goal(&home, root).unwrap().unwrap().blocked_streak, 0);
        // 权限形通知：报警留痕加 streak。
        let perm = json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_request",
            "cwd": "/w/p",
            "session_id": "s1"
        });
        let line = blocked_fastpath(&home, Some(&perm), None).unwrap();
        assert!(
            line.as_ref()
                .is_some_and(|l| { l.contains("alarm-blocked-fast") && l.contains("session=s1") }),
            "{line:?}"
        );
        let rec = show_goal(&home, root).unwrap().unwrap();
        assert_eq!(rec.blocked_streak, 1);
        assert!(rec.blocked_alarm_at > 0);
        let log = std::fs::read_to_string(goal_dir(&home).join(".heal-log")).unwrap();
        assert!(log.contains("alarm-blocked-fast"), "{log}");
        // 防抖窗内二连：streak 递增、不重报。
        assert!(blocked_fastpath(&home, Some(&perm), None)
            .unwrap()
            .is_none());
        assert_eq!(show_goal(&home, root).unwrap().unwrap().blocked_streak, 2);
        assert_eq!(
            std::fs::read_to_string(goal_dir(&home).join(".heal-log"))
                .unwrap()
                .lines()
                .count(),
            1,
            "防抖窗内不重报"
        );
        // PermissionRequest 事件参数形同判 blocked。
        assert!(
            blocked_fastpath(&home, None, Some("permissionrequest"))
                .unwrap()
                .is_none(),
            "防抖窗内（项目根回落进程 cwd,本测试 cwd 无 goal 即静默）"
        );
        // heal 面总闸：off 让位。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HST_GOAL_HEAL", "off");
        let mut aged = show_goal(&home, root).unwrap().unwrap();
        aged.blocked_alarm_at = 1;
        write_record(&goal_path(&home, root), &aged).unwrap();
        assert!(blocked_fastpath(&home, Some(&perm), None)
            .unwrap()
            .is_none());
        std::env::remove_var("HST_GOAL_HEAL");
        // 过窗再报：报警回执回场。
        let line2 = blocked_fastpath(&home, Some(&perm), None).unwrap();
        assert!(line2.is_some(), "过防抖窗重报");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sweeper_gates_respect_escape_hatches() {
        let home = fresh_home("sg");
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // HST_GOAL_SWEEPER=off:loop 即退（零副作用零锁残留）。
        std::env::set_var("HST_GOAL_SWEEPER", "off");
        sweeper_loop(&home, Path::new("/nonexistent-user")).unwrap();
        assert!(!sweeper_lock_path(&home).exists(), "off 即退不留锁");
        std::env::remove_var("HST_GOAL_SWEEPER");
        let _ = std::fs::remove_dir_all(&home);
    }
}
