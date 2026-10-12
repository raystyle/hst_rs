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
//! 寿命、`.sweeper-lock` 单实例锁 pid 加 token 身份验主、tick 加最坏
//! heavy 两倍陈旧接管），`HST_GOAL_SWEEPER=off`
//! 逃生闸。wake 文本 v2 自包含五件（goal 文本加上轮收束态加即刻推进加
//! 防卡死防空转边界加用法）。
//!
//! goalhard-01（2026-10-12 goal 原语地基批）：舰队互视互改地基加固。
//! 写原语四硬化——唯一 tmp 名（并发写者不互吞，LWW 语义下 set/clear
//! 永不假错）、写后回读验非空（真源写后验教训）、unix fsync（断电窗
//! 耐久）、`.bak` 上次好档维护；审计道 `.goal-audit`（set/clear/repair
//! 管理动作留痕，host 加 user 加 pid 归属）；坏档自愈 = 隔离留证加
//! bak 晋升或判缺省（doctor 体检面保持只读纯读）；风暴耐久序 = 唤醒
//! 节流先落盘后发 prompt、yolo 治冷却先落盘后跑治序列（写盘故障不再
//! 引发每趟重试风暴）、sweeper 每趟至多一个治动作；增量路写前整档
//! 快照乐观守卫（并发 set 不被增量旧档吞，同秒只换文本亦判变）；机读面
//! `goal show` json 信封加 `goal list` 全机清单（插件 list --all 聚合消费）。

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

/// 读指定 goal 文件（缺失 = None；坏损 = Err 响亮）。并发 clear 竞态
///（读中文件被删）按缺失收（goalhard-01:is_file 判真与读之间被并发 clear
/// 删走不是坏档,整档 LWW 合法态）。
///
/// # Errors
///
/// 文件在场但不可读或不可解析时返回 [`GoalError::Corrupt`]。
pub fn read_record(path: &Path) -> Result<Option<GoalRecord>, GoalError> {
    if !path.is_file() {
        return Ok(None);
    }
    let corrupt = |cause: String| GoalError::Corrupt {
        path: path.display().to_string(),
        cause,
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        // 读中消失:并发 clear 已收,按缺失收。
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(corrupt(e.to_string())),
    };
    // BOM 容忍（同 yolo 面 D52 先例:舰队 PowerShell 常写 UTF-8 BOM,serde
    // 对 BOM 头整体拒收）。
    let text = text.trim_start_matches('\u{feff}');
    let v: Json = serde_json::from_str(text).map_err(|e| corrupt(e.to_string()))?;
    serde_json::from_value(v)
        .map(Some)
        .map_err(|e| corrupt(e.to_string()))
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
    // 上次好档维护（goalhard-01 坏档自愈的前置）：主档可解析才刷 .bak，
    // 坏档不覆好 bak；best-effort，不阻断主写。
    keep_last_good(path);
    // 原子写（tmp 加 rename）：Stop 腿与状态栏读并发
    // 不见半文件。tmp 名唯一（pid 加随机 token）：本地与远端（ssh/mesh
    // exec 落本机）并发写者互不吞 tmp，rename 即整档最后写胜，set/clear
    // 永不因 tmp 互撞假错。
    let tmp = path.with_extension(format!("json.{}.tmp", write_token()));
    std::fs::write(&tmp, &body).map_err(|e| GoalError::Io {
        path: tmp.display().to_string(),
        cause: e.to_string(),
    })?;
    fsync_path(&tmp);
    let rename_err = std::fs::rename(&tmp, path).err();
    if let Some(e) = rename_err {
        let _ = std::fs::remove_file(&tmp);
        return Err(GoalError::Io {
            path: path.display().to_string(),
            cause: e.to_string(),
        });
    }
    if let Some(dir) = path.parent() {
        fsync_path(dir);
    }
    // 写后验（goalhard-01，真源写后验非空教训）：回读解析且 text 非空，
    // 不通过即响亮报错，绝不静默留坏档；主位缺失 = 并发 clear 已收
    //（rename 已成的缺席只可能来自并发删除，整档 LWW 合法态）放行。
    match read_record(path) {
        Ok(Some(back)) if !back.text.trim().is_empty() => Ok(()),
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(GoalError::Io {
            path: path.display().to_string(),
            cause: "verify: 写后回读 text 为空".into(),
        }),
        Err(e) => Err(GoalError::Io {
            path: path.display().to_string(),
            cause: format!("verify: 写后回读失败 {e}"),
        }),
    }
}

/// 写者唯一 token（pid 加 getrandom，同 sweeper 锁身份先例）。
fn write_token() -> String {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    format!("{}-{:016x}", std::process::id(), u64::from_be_bytes(b))
}

/// 上次好档路径（`<slug>.json.bak`；修复面的晋升源）。
fn bak_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// 主档在场且可解析时刷 `.bak`（原子写同形，best-effort 静默）。
fn keep_last_good(path: &Path) {
    let Ok(body) = std::fs::read(path) else {
        return;
    };
    if serde_json::from_slice::<GoalRecord>(&body).is_err() {
        return;
    }
    let bak = bak_path(path);
    let tmp = bak.with_extension(format!("bak.{}.tmp", write_token()));
    if std::fs::write(&tmp, &body).is_ok() && std::fs::rename(&tmp, &bak).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// fsync 耐久（goalhard-01 断电窗）：unix 下尽力而为（网络文件系统可
/// 拒，失败不计）；Windows 无对应原语，跳过（rename 原子性仍在）。
#[cfg(unix)]
fn fsync_path(p: &Path) {
    let _ = std::fs::File::open(p).and_then(|f| f.sync_all());
}

#[cfg(not(unix))]
fn fsync_path(_p: &Path) {}

/// 乐观守卫写入（goalhard-01 并发语义，整档 CAS）：增量路（rounds/
/// streak 等读改写）落盘前重读全档与读取时快照比对——任一字段变（并发
/// set 换文本、同秒 set 只换文本也中、heal 换节流、他路计数）即弃写让位
/// 返回 false（他方写保全，增量不吞管理写）。验后 rename 前的 TOCTOU 窗
/// 已收窄到微秒级，残余竞争按整档 LWW 收敛（守卫不改原子性，只保语义）。
///
/// # Errors
///
/// 守卫重读或写盘失败时返回 [`GoalError`]。
fn write_record_guarded(
    path: &Path,
    expected: &GoalRecord,
    rec: &GoalRecord,
) -> Result<bool, GoalError> {
    match read_record(path)? {
        Some(cur) if &cur == expected => write_record(path, rec).map(|_| true),
        _ => Ok(false),
    }
}

/// 设置本项目 goal（覆盖旧 goal，rounds 归零重计；整档最后写胜语义，
/// 真写落审计道）。
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
    let path = goal_path(home, root);
    let prior_rounds = read_record(&path).ok().flatten().map(|p| p.rounds);
    write_record(&path, &rec)?;
    goal_audit_append(
        home,
        &slug(&root.display().to_string()),
        "set",
        &format!(
            "{} prior_rounds={} text={}",
            audit_source(),
            prior_rounds.unwrap_or(0),
            flatten_inline(&rec.text)
        ),
    );
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

/// 全机在册 goal 清单行（`hst goal list` 的数据面，goalhard-01 机读
/// 聚合原语：goal 插件 list --all 逐机消费）。
pub struct GoalListRow {
    /// 项目键 slug（文件名去 `.json`；项目路径的损压缩形，同键规则）。
    pub slug: String,
    /// 存储文件路径。
    pub file: PathBuf,
    /// goal 记录（None = 修复后判缺省或本无档）。
    pub rec: Option<GoalRecord>,
    /// 修复注记（本趟触发过坏档修复时在；对象形与 show 面同构，rev-codex
    /// G-4：action 加 quarantined 取证指针不丢）。
    pub repaired: Option<RepairNote>,
}

/// 列本机全部在册 goal（goalhard-01）：按统一收集器 goal_files 字典
/// 序逐档，坏档经修复面（隔离加 bak 晋升或判缺省）不卡清单。
///
/// # Errors
///
/// goal 目录不可读时返回 `String`（清单面整趟响亮，不静默截短）。
pub fn list_goals(home: &Path) -> Result<Vec<GoalListRow>, String> {
    let dir = goal_dir(home);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut rows = Vec::new();
    for path in goal_files(home) {
        let slug = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match read_record_repairing(home, &path) {
            Ok((rec, note)) => rows.push(GoalListRow {
                slug,
                file: path,
                rec,
                repaired: note,
            }),
            Err(e) => {
                return Err(format!("{}: {e}", path.display()));
            }
        }
    }
    Ok(rows)
}

/// 清空本项目 goal（删文件；返回是否真删，无 goal 零改动合法）。`.bak`
/// 同删（rev-codex G-3：clear 后再 set 不刷 bak，留着旧 bak 只会在新档
/// 坏损时复活已清目标；误清恢复由审计道 clear 行 text= 快照承载）。并发
/// clear 竞态（读档后文件被别家收走）按已清收。真删落审计道。
///
/// # Errors
///
/// IO 失败时返回 [`GoalError`]。
pub fn clear_goal(home: &Path, root: &Path) -> Result<bool, GoalError> {
    let path = goal_path(home, root);
    if !path.exists() {
        return Ok(false);
    }
    let key = slug(&root.display().to_string());
    let prior = read_record(&path).ok().flatten();
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        // 读档后已被并发 clear 收走：按已清收（两机同清不假错）。
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(GoalError::Io {
                path: path.display().to_string(),
                cause: e.to_string(),
            })
        }
    }
    let _ = std::fs::remove_file(bak_path(&path));
    goal_audit_append(
        home,
        &key,
        "clear",
        &format!(
            "{} text={}",
            audit_source(),
            prior
                .map(|p| flatten_inline(&p.text))
                .unwrap_or_else(|| "(unreadable)".into())
        ),
    );
    Ok(true)
}

/// 留痕日志上限（goalhard-01）：`.heal-log` 与 `.goal-audit` 追加前查
/// 尺寸，超限先滚 `.1`（keep-one，超限旧档弃），长役不无限增长。
pub const AUDIT_LOG_CAP_BYTES: u64 = 1024 * 1024;

/// 审计道（goalhard-01 互视互改审计）：`.goal-audit` 单行追加
/// （ts 加 slug 加 action 加 detail），set/clear/repair 管理动作留痕
/// （heal 域动作在 `.heal-log`，不重记）。best-effort。
fn goal_audit_append(home: &Path, slug: &str, action: &str, detail: &str) {
    append_capped(
        &goal_dir(home).join(".goal-audit"),
        &format!("{} {} {} {}", unix_secs(), slug, action, detail),
    );
}

/// 带轮转帽的单行追加（`.heal-log` 与 `.goal-audit` 共用；并发轮转竞态
/// 下 rename 失败方直接续写原档，容忍偶超限）。单 write_all 含换行:unix
/// O_APPEND 单次写原子定位,并发追加不交错行（writeln! 两笔写会在行中
/// 拼进他行,实证 2026-10-12 并发锤测试）。best-effort。
fn append_capped(path: &Path, line: &str) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(md) = std::fs::metadata(path) {
        if md.len() >= AUDIT_LOG_CAP_BYTES {
            let _ = std::fs::rename(path, path.with_extension("1"));
        }
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| {
            use std::io::Write as _;
            f.write_all(format!("{line}\n").as_bytes())
        });
}

/// 写者归属（审计行内 host 加 user 加 pid；远端 ssh/mesh exec 落本机时
/// 进程身份即调用方代理身份，跨机聚合按各机 `.goal-audit` 自证）。
fn audit_source() -> String {
    format!(
        "host={} user={} pid={}",
        hostname_or_dash(),
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "-".into()),
        std::process::id()
    )
}

fn hostname_or_dash() -> String {
    let direct = if cfg!(unix) {
        std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        std::env::var("COMPUTERNAME").ok()
    };
    direct
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "-".into())
}

/// 坏档修复注记（读面自愈的回执：隔离留证 + 晋升或判缺省）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairNote {
    /// 动作（promote = bak 晋升回主档；drop = 无好档判缺省）。
    pub action: String,
    /// 坏档隔离后的路径（取证面）。
    pub quarantined: String,
}

/// 读面自愈（goalhard-01 坏档重建路径）：主档坏损时不再卡死——隔离坏档
/// 留证（`<slug>.json.corrupt-<ts>-<token>`），`.bak` 有好档即晋升回主档
/// （写后验同形），无好档即判缺省态；动作落 `.goal-audit`。doctor 体检
/// 面不走此口（只读契约），走纯 [`read_record`] 响亮报坏。
///
/// # Errors
///
/// 隔离失败（权限或竞态，坏档维持原样不覆写）或 bak 晋升写盘失败时返回
/// 原因 [`GoalError`]。
pub fn read_record_repairing(
    home: &Path,
    path: &Path,
) -> Result<(Option<GoalRecord>, Option<RepairNote>), GoalError> {
    let err = match read_record(path) {
        Ok(v) => return Ok((v, None)),
        Err(e) => e,
    };
    let GoalError::Corrupt { .. } = err else {
        return Err(err);
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let quarantined = path.with_extension(format!("json.corrupt-{}-{}", unix_secs(), {
        let mut b = [0u8; 4];
        let _ = getrandom::fill(&mut b);
        format!("{:08x}", u32::from_be_bytes(b))
    }));
    if std::fs::rename(path, &quarantined).is_err() {
        // 隔离不动手：维持响亮，绝不覆写真数据。
        return Err(err);
    }
    match read_record(&bak_path(path)) {
        Ok(Some(rec)) if !rec.text.trim().is_empty() => {
            write_record(path, &rec)?;
            goal_audit_append(
                home,
                &stem,
                "repair",
                &format!(
                    "{} action=promote quarantined={}",
                    audit_source(),
                    quarantined.display()
                ),
            );
            Ok((
                Some(rec),
                Some(RepairNote {
                    action: "promote".into(),
                    quarantined: quarantined.display().to_string(),
                }),
            ))
        }
        _ => {
            goal_audit_append(
                home,
                &stem,
                "repair",
                &format!(
                    "{} action=drop quarantined={}",
                    audit_source(),
                    quarantined.display()
                ),
            );
            Ok((
                None,
                Some(RepairNote {
                    action: "drop".into(),
                    quarantined: quarantined.display().to_string(),
                }),
            ))
        }
    }
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
    // 读面自愈（goalhard-01）：坏档先修复再用，坏档不再卡死提醒链。
    let mut rec = match read_record_repairing(home, &path).map_err(|e| e.to_string())? {
        (Some(r), _) => r,
        (None, _) => return Ok(None),
    };
    let expected = rec.clone();
    rec.rounds += 1;
    rec.last_stop_at = unix_secs();
    // 乐观守卫（goalhard-01）：计数落盘时并发 set/clear 已换档即弃写让位
    //（他方新 goal 从零计，本轮增量无意义；clear 已收即不复活），本轮
    // 不出提醒（下一轮自然停接续）。
    if !write_record_guarded(&path, &expected, &rec).map_err(|e| e.to_string())? {
        return Ok(None);
    }
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

/// sweeper 单趟 heavy 最坏时长（rev-codex G-1 判据锚）：herdr 预算和
/// 取整 = agent list 4s 加治序列（send-keys 三发各 8s 加回 shell 轮询
/// 15s 加 start 35s 加续发 8s，约 86s）。blocked 治另受风暴阈加 6h 冷却
/// 双闸；goalhard-01 起 sweeper 路每趟治动作上帽一个（heavy_cap），单趟
/// heavy 与 goal 数无关地有界，不叠破陈旧阈；叠破时接管让位（锁验主保
/// 后继锁不误删）兜底仍在。
const SWEEP_HEAVY_WORST_SECS: u64 = 90;

/// sweeper 锁陈旧阈（REQ-047 单实例）：心跳重写超此时长即判死锁接管。
/// rev-codex G-1 自 90 抬位 = tick 加最坏 heavy 两倍：heavy 跑程锁不
/// 心跳，鲜活窗须罩住「上一跳后整 tick 等待加整趟 heavy」再留一倍余量。
pub const SWEEPER_STALE_SECS: u64 = SWEEPER_TICK_SECS + 2 * SWEEP_HEAVY_WORST_SECS;

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
#[derive(Debug)]
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
/// shell;rev-codex G-2:herdr 报错不算回 shell,故障续轮询到超时）,fresh
/// 起复用格名（无名格按 slug 造 goal-heal 前缀名）,新会话
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
    // 有界轮询等格回 shell(格态消失即回;rev-codex G-2:herdr 报错与
    // 无状态区分,报错不算回 shell,续轮询到超时由冷却防锤兜底)。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let gone = match herdr_output(&["agent", "get", pane]) {
            Ok(t) => agent_status_of(&t).is_none(),
            Err(_) => false,
        };
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
/// 逐条判定后分型处置（REQ-047 两维态矩阵）：Wake 形先落 last_heal_at
/// 加风暴闸计数再经 `herdr agent prompt` 唤醒（goalhard-01 耐久序：节流
/// 先于 prompt 落盘，写盘故障不再引发每趟重唤风暴）；Blocked 形报警留痕
///（streak 递增加 `.heal-log` alarm-blocked 行），预授权（arm 标记）且
/// streak 到阈值且工位非 yolo 且过冷却时走 yolo 治序列——冷却先落盘再跑
/// 治序列（goalhard-01：治序列失败也持有冷却，防每趟 sweep 重启格风暴；
/// 写盘失败即不治）。wake-fail（prompt 发出失败）同吃 heal_streak 预算
///（rev-codex G-7 裁定保持：传输层连坏即驱动链坏死，连续 3 次转人工等
/// 人裁，恢复面 = agent 真活动（rounds 增长））。`heavy` 为真才可治（hook 事件路径传 false 只报警
/// 让位 sweeper 或手动面执行）；`heavy_cap` 限单趟治动作数（sweeper 路传
/// `Some(1)`：单趟 heavy 有界，多 goal 同 blocked 不叠破锁陈旧阈；手动
/// 面传 `None` 不帽）。并发 set 撞增量写即 `skip-raced` 让位（乐观守卫）；
/// 单档写盘失败折 `skip-write-fail` 行不弃整趟。dry_run 只报判定不落盘
/// 不唤不发。坏档经修复面（隔离加 bak 晋升或判缺省）不入 skip-corrupt
/// 死档。herdr 消费面只读 agent list 加单发 prompt 加治序列原语
/// （send-keys/get/start），不改 herdr 仓。
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
    heavy_cap: Option<usize>,
) -> Result<Vec<HealReport>, String> {
    let panes = parse_agent_list(&herdr_output(&["agent", "list"])?)?;
    let now = unix_secs();
    sweep_stale_tmp(home, now);
    let files = goal_files(home);
    let only_slug = only.map(|r| slug(&r.display().to_string()));
    let mut heavy_done = 0usize;
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
        // 坏档先修复（goalhard-01）：修复即继续本趟判定，不再死档。
        let mut rec = match read_record_repairing(home, &path) {
            Ok((Some(r), note)) => {
                if let Some(n) = note {
                    out.push(HealReport {
                        slug: stem.clone(),
                        action: format!("repair-{}", n.action),
                        detail: n.quarantined,
                    });
                }
                r
            }
            Ok((None, note)) => {
                if let Some(n) = note {
                    out.push(HealReport {
                        slug: stem,
                        action: format!("repair-{}", n.action),
                        detail: n.quarantined,
                    });
                }
                continue;
            }
            Err(e) => {
                out.push(HealReport {
                    slug: stem,
                    action: "skip-corrupt".into(),
                    detail: e.to_string(),
                });
                continue;
            }
        };
        let expected = rec.clone();
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
                // 节流先落盘（goalhard-01 耐久序）：last_heal_at 加 streak
                // 持久化先于 prompt，prompt 失败只丢本唤不引重试风暴。
                rec.last_heal_at = now;
                rec.heal_streak = if rec.rounds == rec.heal_rounds {
                    rec.heal_streak + 1
                } else {
                    1
                };
                rec.heal_rounds = rec.rounds;
                match write_record_guarded(&path, &expected, &rec) {
                    Ok(true) => {}
                    Ok(false) => {
                        out.push(HealReport {
                            slug: stem,
                            action: "skip-raced".into(),
                            detail: "并发 set/clear 让位，本趟不唤".into(),
                        });
                        continue;
                    }
                    Err(e) => {
                        out.push(HealReport {
                            slug: stem,
                            action: "skip-write-fail".into(),
                            detail: e.to_string(),
                        });
                        continue;
                    }
                }
                let project = panes
                    .iter()
                    .find(|p| p.pane_id == pane)
                    .map(|p| p.cwd.clone())
                    .unwrap_or_else(|| stem.clone());
                match herdr_output(&["agent", "prompt", &pane, &wake_text(&rec, &project, now)]) {
                    Ok(_) => out.push(HealReport {
                        slug: stem,
                        action: "woke".into(),
                        detail: format!(
                            "pane={pane} rounds={} streak={}",
                            rec.rounds, rec.heal_streak
                        ),
                    }),
                    Err(e) => out.push(HealReport {
                        slug: stem,
                        action: "wake-fail".into(),
                        detail: format!("pane={pane} 节流已落盘，下窗再试: {e}"),
                    }),
                }
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
                let cap_open = heavy_cap.is_none_or(|cap| heavy_done < cap);
                let mut detail = format!("pane={pane} agent={agent} streak={}", rec.blocked_streak);
                // 报警先行（委托令「blocked 即先报警留痕」）：防抖窗过即报，
                // 与是否治互不 gating。
                if alarm_due && !dry_run {
                    rec.blocked_alarm_at = now;
                    heal_log_append(home, &stem, "alarm-blocked", &detail);
                }
                if eligible && !cap_open {
                    // 单趟治名额已用（dry 与实跑同判）：让位下趟 tick。streak
                    // 与报警态先落盘（rev-codex G-2：不让位丢持久化，防抖与
                    // 计数窗口连续），dry 零副作用不写。
                    if !dry_run {
                        let _ = write_record_guarded(&path, &expected, &rec);
                    }
                    out.push(HealReport {
                        slug: stem,
                        action: "heavy-deferred".into(),
                        detail: format!(
                            "{detail} 本趟治名额已用（cap={}），下趟 tick 再治",
                            heavy_cap.unwrap_or(0)
                        ),
                    });
                    continue;
                }
                if dry_run {
                    let action = if eligible {
                        // dry 也占治名额（判据与实跑同源,cap 语义一致）。
                        heavy_done += 1;
                        "would-yolo-heal"
                    } else if alarm_due {
                        "would-alarm-blocked"
                    } else {
                        "blocked-muted"
                    };
                    out.push(HealReport {
                        slug: stem,
                        action: action.into(),
                        detail,
                    });
                    continue;
                }
                if eligible {
                    // 冷却先落盘（goalhard-01 耐久序）：yolo_heal_at 持久化
                    // 先于治序列，治失败也持有冷却（防锤），写失败即不治。
                    rec.yolo_heal_at = now;
                }
                let action = match write_record_guarded(&path, &expected, &rec) {
                    Ok(true) => {
                        if eligible {
                            heavy_done += 1;
                            let project = panes
                                .iter()
                                .find(|p| p.pane_id == pane)
                                .map(|p| p.cwd.clone())
                                .unwrap_or_else(|| stem.clone());
                            match yolo_heal_sequence(
                                user_home, &pane, &agent, &name, &rec, &project, now,
                            ) {
                                Ok(d) => {
                                    detail.push_str(&format!(" heal=ok:{d}"));
                                    heal_log_append(home, &stem, "yolo-heal", &detail);
                                    "yolo-heal"
                                }
                                Err(e) => {
                                    // 失败已持有冷却（先落盘），下窗再试。
                                    detail.push_str(&format!(" heal=fail:{e}"));
                                    heal_log_append(home, &stem, "yolo-heal-fail", &detail);
                                    "yolo-heal-fail"
                                }
                            }
                        } else if alarm_due {
                            "alarm-blocked"
                        } else {
                            // 防抖窗内：streak 已计，报警静默。
                            "blocked-muted"
                        }
                    }
                    Ok(false) => {
                        detail.push_str(" 并发 set/clear 让位");
                        "skip-raced"
                    }
                    Err(e) => {
                        detail.push_str(&format!(" 写盘失败:{e}"));
                        "skip-write-fail"
                    }
                };
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
///（ts 加 slug 加 action 加 detail），best-effort；goalhard-01 起过
/// [`AUDIT_LOG_CAP_BYTES`] 先滚 `.1`（keep-one）。
fn heal_log_append(home: &Path, slug: &str, action: &str, detail: &str) {
    append_capped(
        &goal_dir(home).join(".heal-log"),
        &format!("{} {} {} {}", unix_secs(), slug, action, detail),
    );
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
    let rows = run_heal(home, user_home, None, false, false, None)?;
    for r in &rows {
        if report_logworthy(&r.action) {
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

/// 在册 goal 文件清单（goalhard-01 统一收集器：run_heal 扫描与
/// `goal list` 共用）。收录形 = 扩展名 `json` 且非隐藏文件（`.heal-log`
/// 等治理件与 `.bak`、`.corrupt-*`、`*.tmp` 一律排除），字典序稳定。
fn goal_files(home: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(goal_dir(home))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|x| x == "json")
                        && p.file_stem().is_some_and(|s| {
                            let s = s.to_string_lossy();
                            !s.starts_with('.') && !s.ends_with(".tmp")
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// 崩溃残件 tmp 的清扫阈（goalhard-01）：唯一 tmp 名后写者不再互踩，
/// 残件只占位；超此时长（mtime 距今）的 `*.tmp` 在扫描趟顺手清。
pub const TMP_STALE_SECS: u64 = 3600;

fn sweep_stale_tmp(home: &Path, now: u64) {
    let Ok(rd) = std::fs::read_dir(goal_dir(home)) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "tmp") {
            let stale = p
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .is_some_and(|d| now.saturating_sub(d.as_secs()) > TMP_STALE_SECS);
            if stale {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

/// 回执行是否值得 `.heal-log` 留痕（heal_sweep 与 sweeper 两驱动共用判据，
/// goalhard-01 收口单源）：唤醒族与转人工族必留，新 wake-fail（节流已
/// 落盘的唤失败）与 repair 族同留（可观测）。
fn report_logworthy(action: &str) -> bool {
    matches!(
        action,
        "woke"
            | "wake-fail"
            | "skip-storm-capped"
            | "no-agent"
            | "skip-corrupt"
            | "skip-write-fail"
            | "repair-promote"
            | "repair-drop"
    )
}

/// sweeper 单实例锁（`.sweeper-lock`，内容 = `pid token 心跳epoch秒`
/// 三段：pid 加随机 token 双身份，心跳重写时保身不变，接管与释放均验主
/// （内容非我即他人所持，不动））。
fn sweeper_lock_path(home: &Path) -> PathBuf {
    goal_dir(home).join(".sweeper-lock")
}

/// 本 sweeper 自身份（进程 pid 加 getrandom token，一次生成全程用）。
fn sweeper_identity() -> String {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    format!("{} {:016x}", std::process::id(), u64::from_be_bytes(b))
}

/// 锁身份与心跳（无锁或坏形 = None）。
fn sweeper_lock_parse(home: &Path) -> Option<(String, u64)> {
    let text = std::fs::read_to_string(sweeper_lock_path(home)).ok()?;
    let mut it = text.split_whitespace();
    let pid = it.next()?.to_string();
    let token = it.next()?.to_string();
    let ts = it.next()?.parse::<u64>().ok()?;
    Some((format!("{pid} {token}"), ts))
}

/// 锁是否鲜活（心跳在陈旧阈内 = 有活 sweeper 在场）。
fn sweeper_lock_fresh(home: &Path, now: u64) -> bool {
    sweeper_lock_parse(home).is_some_and(|(_, ts)| now.saturating_sub(ts) < SWEEPER_STALE_SECS)
}

/// 抢锁（单实例防重入）：create_new 原子建（落自身份）；已在场且鲜活即
/// 让位（None），陈旧即接管（删后重试一次）。成功返回自身份（心跳与
/// 释放验主用）。
fn sweeper_lock_acquire(home: &Path, now: u64) -> Option<String> {
    let p = sweeper_lock_path(home);
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let me = sweeper_identity();
    use std::io::Write as _;
    let try_once = || -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)?;
        writeln!(f, "{me} {now}")
    };
    if try_once().is_ok() {
        return Some(me);
    }
    if sweeper_lock_fresh(home, now) {
        return None;
    }
    // 陈旧接管：删旧重试（竞态下两家同删，create_new 只成一家）。
    let _ = std::fs::remove_file(&p);
    try_once().is_ok().then_some(me)
}

/// 锁心跳重写（每 tick 落一次，续鲜）。rev-codex G-1 验主：只重写自持
/// 锁；锁不在场或非我身份（已被接管）即返 false，主循环让位退（不续写
/// 不误抹接管者锁，退出路径也不走释放）。
fn sweeper_heartbeat(home: &Path, me: &str, now: u64) -> bool {
    let p = sweeper_lock_path(home);
    if !sweeper_lock_parse(home).is_some_and(|(id, _)| id == me) {
        return false;
    }
    let _ = std::fs::write(&p, format!("{me} {now}"));
    true
}

/// 锁释放（收尾统一走此口，rev-codex F-2）：只删自持锁（验主），非我
/// 身份即他人接管中，不动。
fn sweeper_lock_release(home: &Path, me: &str) {
    if sweeper_lock_parse(home).is_some_and(|(id, _)| id == me) {
        let _ = std::fs::remove_file(sweeper_lock_path(home));
    }
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
/// 扫描内部失败时返回 `String`（子进程 stderr 落幕）。rev-codex F-2：
/// 扫描报错不走 `?` 提前返回（会跳过收尾），统一收尾释放自持锁后传播。
pub fn sweeper_loop(home: &Path, user_home: &Path) -> Result<(), String> {
    sweeper_loop_impl(home, user_home, SWEEPER_TICK_SECS)
}

/// 循环实现体（tick 秒数可注入，生面孔恒 [`SWEEPER_TICK_SECS`]，测试
/// 传小值快进；G-1 验主：心跳见非我身份即被接管，让位退不删他人锁）。
fn sweeper_loop_impl(home: &Path, user_home: &Path, tick: u64) -> Result<(), String> {
    if sweeper_off() {
        return Ok(());
    }
    let now = unix_secs();
    let Some(me) = sweeper_lock_acquire(home, now) else {
        return Ok(());
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(SWEEPER_LIFE_SECS);
    let mut sweep_err: Option<String> = None;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(tick));
        let now = unix_secs();
        if !sweeper_heartbeat(home, &me, now) {
            // 锁已被接管或不在场：让位退（他人持锁，不走释放）。
            return Ok(());
        }
        if !goal_dir_has_goals(home) || std::time::Instant::now() >= deadline {
            break;
        }
        if sweeper_due(home, now) {
            if let Some(d) = sweeper_stamp_path(home).parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(sweeper_stamp_path(home), now.to_string());
            // rev-codex F-2：结果先存不 `?`，报错也要走到收尾释放锁。
            // goalhard-01：heavy_cap=Some(1)——单趟治动作有界，多 goal 同
            // blocked 不叠破锁陈旧阈（每 tick 让位下一档，自然轮转）。
            let res = run_heal(home, user_home, None, false, true, Some(1));
            if let Ok(rows) = &res {
                for r in rows {
                    if report_logworthy(&r.action) {
                        heal_log_append(home, &r.slug, &r.action, &r.detail);
                    }
                }
            }
            if let Err(e) = res {
                sweep_err = Some(e);
                break;
            }
        }
    }
    sweeper_lock_release(home, &me);
    match sweep_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
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
    let mut rec = match read_record_repairing(home, &path).map_err(|e| e.to_string())? {
        (Some(r), _) => r,
        (None, _) => return Ok(None),
    };
    let expected = rec.clone();
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
    // 报警态先入档（goalhard-01 防抖耐久:与守卫写同笔落盘）。
    let alarm_due = now.saturating_sub(rec.blocked_alarm_at) >= BLOCKED_ALARM_SECS;
    if alarm_due {
        rec.blocked_alarm_at = now;
    }
    // 乐观守卫（goalhard-01）：并发 set/clear 已换档即弃写让位（streak
    // 属旧 goal，无意义），本事件零动作。
    if !write_record_guarded(&path, &expected, &rec).map_err(|e| e.to_string())? {
        return Ok(None);
    }
    if alarm_due {
        heal_log_append(home, &stem, "alarm-blocked-fast", &detail);
        Ok(Some(format!("{stem} alarm-blocked-fast {detail}")))
    } else {
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
        // clear：删文件、再 clear 零改动合法；.bak 同删（rev-codex G-3：旧
        // bak 复活已清目标是真风险）。
        assert!(clear_goal(&home, root).unwrap());
        assert!(show_goal(&home, root).unwrap().is_none());
        assert!(
            !bak_path(&goal_path(&home, root)).exists(),
            "clear 同删 bak"
        );
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
        let rows = run_heal(&home, &user, Some(proj), true, false, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "would-wake", "{:?}", rows[0].detail);
        assert!(!log.exists());
        // 实跑：唤醒一次，计数落盘。
        let rows = run_heal(&home, &user, Some(proj), false, false, None).unwrap();
        assert_eq!(rows[0].action, "woke", "{:?}", rows[0].detail);
        let woke_log = std::fs::read_to_string(&log).unwrap();
        assert!(woke_log.contains("闭环 t 题") && woke_log.contains("hst goal clear"));
        let rec = show_goal(&home, proj).unwrap().unwrap();
        assert_eq!(rec.heal_streak, 1);
        assert!(rec.last_heal_at > 0);
        // 紧跟第二趟：静默窗频控跳过（无新 prompt）。
        let rows = run_heal(&home, &user, Some(proj), false, false, None).unwrap();
        assert_eq!(rows[0].action, "skip-throttled", "{:?}", rows[0].detail);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), woke_log);
        // 风暴闸：伪造 streak 到顶后转人工。
        let mut rec = show_goal(&home, proj).unwrap().unwrap();
        rec.heal_streak = HEAL_STORM_CAP;
        rec.last_heal_at = 1;
        write_record(&goal_path(&home, proj), &rec).unwrap();
        let rows = run_heal(&home, &user, Some(proj), false, false, None).unwrap();
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
        // goalhard-01 读面自愈契约：Stop 腿遇坏档修复（隔离留证，无好档判
        // 缺省），不再响亮 Err 卡死提醒链；原坏字节保存在隔离档不丢。
        let stop = json!({ "hook_event_name": "Stop", "cwd": "/w/x" });
        assert!(stop_hook(&home, Some(&stop), None).unwrap().is_none());
        assert!(!p.exists(), "无好档判缺省，主位空");
        let dir = goal_dir(&home);
        let quarantined: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|f| {
                f.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("-w-x.json.corrupt-"))
            })
            .collect();
        assert_eq!(quarantined.len(), 1, "坏档隔离留证: {quarantined:?}");
        assert_eq!(
            std::fs::read_to_string(&quarantined[0]).unwrap(),
            "not-json",
            "原坏字节不丢"
        );
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
        assert!(sweeper_lock_acquire(&home, now).is_some(), "无锁首抢成功");
        assert!(
            sweeper_lock_acquire(&home, now).is_none(),
            "鲜活锁第二家让位"
        );
        assert!(sweeper_lock_fresh(&home, now));
        // 心跳陈旧：接管成功。
        let stale = now + SWEEPER_STALE_SECS + 10;
        assert!(!sweeper_lock_fresh(&home, stale));
        assert!(sweeper_lock_acquire(&home, stale).is_some(), "陈旧锁接管");
        assert!(sweeper_lock_fresh(&home, stale));
        // 坏形锁（非三段身份形）：判陈旧可接管。
        std::fs::write(sweeper_lock_path(&home), "junk").unwrap();
        assert!(!sweeper_lock_fresh(&home, stale));
        assert!(sweeper_lock_acquire(&home, stale).is_some());
        // 空目录判无 goal：sweeper 不续命。
        assert!(!goal_dir_has_goals(&home));
        set_goal(&home, Path::new("/w/p"), "t").unwrap();
        assert!(goal_dir_has_goals(&home));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sweeper_lock_heartbeat_and_release_verify_owner() {
        let home = fresh_home("lk2");
        let now = 10_000u64;
        let me = sweeper_lock_acquire(&home, now).unwrap();
        // 他家身份：心跳不写（ts 不动）加释放不删（rev-codex G-1 验主）。
        let other = "99999 deadbeefdeadbeef";
        assert!(!sweeper_heartbeat(&home, other, now + 10));
        assert_eq!(
            sweeper_lock_parse(&home).map(|(_, ts)| ts),
            Some(now),
            "他家心跳不得改写锁"
        );
        sweeper_lock_release(&home, other);
        assert!(sweeper_lock_path(&home).exists(), "他家释放不得删锁");
        // 自家身份：心跳续鲜加释放删锁。
        assert!(sweeper_heartbeat(&home, &me, now + 60));
        assert_eq!(sweeper_lock_parse(&home).map(|(_, ts)| ts), Some(now + 60));
        sweeper_lock_release(&home, &me);
        assert!(!sweeper_lock_path(&home).exists(), "自家释放删锁");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sweeper_loop_releases_lock_on_heal_error() {
        let home = fresh_home("lk3");
        set_goal(&home, Path::new("/w/p"), "t").unwrap();
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // PATH 清空 → herdr spawn 即败 → run_heal 首步报错；锁仍须释放
        //（rev-codex F-2：不走 ? 提前返回，收尾统一释放）。
        let saved = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "");
        let res = sweeper_loop_impl(&home, Path::new("/nonexistent-user"), 0);
        std::env::set_var("PATH", &saved);
        let err = res.unwrap_err();
        assert!(err.contains("herdr"), "{err}");
        assert!(!sweeper_lock_path(&home).exists(), "报错路径收尾仍释放锁");
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

    // ===== goalhard-01 三族回归（并发写 / 崩溃注入 / 风暴闸）加机读面 =====

    #[test]
    fn concurrent_writers_lww_never_corrupt_never_spurious_error() {
        // goalhard-01 并发族:双写者(本地与远端 ssh 落本机的同构多进程面)
        // 加清者加读者四线程锤同键。不变量:读者恒见有效档或缺失(绝不坏
        // 档);写者全 Ok(唯一 tmp,固定 tmp 名先例下两写者互吞必现假错);
        // 终态有效或缺失(整档 LWW);审计道行数与真发生的管理动作对账。
        let home = std::sync::Arc::new(fresh_home("cc"));
        let root = Path::new("/w/proj");
        let path = goal_path(&home, root);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut writers = Vec::new();
        for tag in ["A", "B"] {
            let (h, b) = (home.clone(), barrier.clone());
            writers.push(std::thread::spawn(move || {
                b.wait();
                for i in 0..60 {
                    set_goal(&h, root, &format!("{tag}-{i}")).unwrap();
                }
            }));
        }
        let clearer = {
            let (h, b) = (home.clone(), barrier.clone());
            std::thread::spawn(move || {
                b.wait();
                let mut removed = 0usize;
                for _ in 0..30 {
                    if clear_goal(&h, root).unwrap() {
                        removed += 1;
                    }
                }
                removed
            })
        };
        let reader = {
            let (p, b) = (path.clone(), barrier.clone());
            std::thread::spawn(move || {
                b.wait();
                // 读者跑固定预算:写者们 120 发 set 期间持续抽查不变量。
                for _ in 0..4000 {
                    match read_record(&p) {
                        Ok(Some(r)) => assert!(!r.text.is_empty(), "半档: {r:?}"),
                        Ok(None) => {}
                        Err(e) => panic!("读者见坏档: {e}"),
                    }
                }
            })
        };
        for h in writers {
            if let Err(e) = h.join() {
                std::panic::resume_unwind(e);
            }
        }
        let cleared = clearer.join().unwrap();
        reader.join().unwrap();
        let final_rec = read_record(&path).unwrap();
        assert!(
            final_rec.as_ref().is_none_or(|r| !r.text.is_empty()),
            "终态整档 LWW: {final_rec:?}"
        );
        // 审计道对账:120 发 set 每发一行;clear 只真删才留行。
        let audit =
            std::fs::read_to_string(goal_dir(&home).join(".goal-audit")).unwrap_or_default();
        let count = |action: &str| {
            audit
                .lines()
                .filter(|l| l.split_whitespace().nth(2) == Some(action))
                .count()
        };
        assert_eq!(count("set"), 120, "{audit}");
        assert_eq!(count("clear"), cleared, "{audit}");
        let _ = std::fs::remove_dir_all(&*home);
    }

    #[test]
    fn write_verify_bak_promote_and_drop_repair_paths() {
        // goalhard-01 坏档族:bak 维护(上次好档)、坏档隔离晋升、无好档判
        // 缺省、修复幂等、doctor 纯读面不修复、审计留痕。
        let home = fresh_home("bk");
        let root = Path::new("/w/p");
        set_goal(&home, root, "第一目标").unwrap();
        set_goal(&home, root, "第二目标").unwrap();
        let p = goal_path(&home, root);
        // bak = 上次好档(第一目标;第二次 set 前主档可解析才刷)。
        assert_eq!(
            read_record(&bak_path(&p)).unwrap().unwrap().text,
            "第一目标"
        );
        // 坏主档:修复晋升,主档复活,坏档隔离留证。
        std::fs::write(&p, "half-writ{").unwrap();
        let (rec, note) = read_record_repairing(&home, &p).unwrap();
        assert_eq!(rec.as_ref().unwrap().text, "第一目标");
        let note = note.unwrap();
        assert_eq!(note.action, "promote");
        assert!(Path::new(&note.quarantined).is_file(), "坏档隔离留证");
        assert_eq!(read_record(&p).unwrap().unwrap().text, "第一目标");
        // 幂等:再读无注记。
        let (_, note2) = read_record_repairing(&home, &p).unwrap();
        assert!(note2.is_none());
        // 审计道:promote 行在册(此节目录未清时点检)。
        let audit = std::fs::read_to_string(goal_dir(&home).join(".goal-audit")).unwrap();
        assert!(audit.contains(" action=promote "), "{audit}");
        // 无好档判缺省:坏主档加坏 bak → drop,主位空,隔离留证。
        let _ = std::fs::remove_dir_all(goal_dir(&home));
        set_goal(&home, root, "孤档").unwrap();
        std::fs::write(&p, "x").unwrap();
        std::fs::write(&bak_path(&p), "bad-bak").unwrap();
        let (rec3, note3) = read_record_repairing(&home, &p).unwrap();
        assert!(rec3.is_none(), "无好档判缺省: {rec3:?}");
        assert_eq!(note3.unwrap().action, "drop");
        assert!(!p.exists(), "主位空(坏档已隔离)");
        // 审计道:drop 行在册(本节目录未清时点检)。
        let audit = std::fs::read_to_string(goal_dir(&home).join(".goal-audit")).unwrap();
        assert!(audit.contains(" action=drop "), "{audit}");
        // doctor 纯读面契约:坏档上 show_goal 仍响亮 Corrupt,不修复。
        let _ = std::fs::remove_dir_all(goal_dir(&home));
        set_goal(&home, root, "t").unwrap();
        std::fs::write(&p, "x").unwrap();
        assert_eq!(show_goal(&home, root).unwrap_err().code(), "corrupt");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "x", "纯读面不动坏档");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn race_guard_full_record_cas_same_second_set_detected() {
        // goalhard-01 守卫族:整档 CAS——同秒并发 set 只换文本(set_at 同秒
        // 分不出)仍判变让位;快照未变即写成功;clear 后不复活。
        let home = fresh_home("cas");
        let root = Path::new("/w/r");
        let p = goal_path(&home, root);
        set_goal(&home, root, "原目标").unwrap();
        let snapshot = read_record(&p).unwrap().unwrap();
        // 同秒第二发 set:文本变即全档比对命中(旧 set_at 单键判据的盲区)。
        set_goal(&home, root, "新目标").unwrap();
        let mut bumped = snapshot.clone();
        bumped.rounds += 1;
        assert!(
            !write_record_guarded(&p, &snapshot, &bumped).unwrap(),
            "同秒换文本的并发 set 让增量让位"
        );
        assert_eq!(read_record(&p).unwrap().unwrap().text, "新目标");
        // 快照与盘上一致:写成功。
        let cur = read_record(&p).unwrap().unwrap();
        let mut next = cur.clone();
        next.rounds += 1;
        assert!(write_record_guarded(&p, &cur, &next).unwrap());
        assert_eq!(read_record(&p).unwrap().unwrap().rounds, 1);
        // clear 后:主位空,守卫让位不复活。
        clear_goal(&home, root).unwrap();
        assert!(
            !write_record_guarded(&p, &next, &next).unwrap(),
            "clear 后不复活"
        );
        assert!(!p.exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn stop_hook_and_fastpath_repair_corrupt_store() {
        // goalhard-01 读面自愈实弹:Stop 腿与 blocked 快道遇坏档先修复再走
        // (旧形响亮 Err 卡死提醒链);doctor 面(show_goal)仍纯读。
        let home = fresh_home("rp");
        let root = Path::new("/w/p");
        set_goal(&home, root, "第一目标").unwrap();
        set_goal(&home, root, "第二目标").unwrap();
        let p = goal_path(&home, root);
        // Stop 腿:坏档晋升 bak(第一目标)后计数加提醒照常。
        std::fs::write(&p, "garbage").unwrap();
        let stop = json!({ "hook_event_name": "Stop", "cwd": "/w/p" });
        let out = stop_hook(&home, Some(&stop), None).unwrap().unwrap();
        assert!(out.contains("第一目标"), "提醒载晋升后的 goal: {out}");
        let rec = read_record(&p).unwrap().unwrap();
        assert_eq!(rec.text, "第一目标");
        assert_eq!(rec.rounds, 1);
        // 快道:再坏档(重新 set 后破坏),blocked 事件修复加报警。
        set_goal(&home, root, "第三目标").unwrap();
        set_goal(&home, root, "第四目标").unwrap();
        std::fs::write(&p, "garbage2").unwrap();
        let perm = json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_request",
            "cwd": "/w/p",
            "session_id": "s9"
        });
        let line = blocked_fastpath(&home, Some(&perm), None).unwrap();
        assert!(
            line.as_ref()
                .is_some_and(|l| l.contains("alarm-blocked-fast")),
            "{line:?}"
        );
        let rec2 = read_record(&p).unwrap().unwrap();
        assert_eq!(rec2.text, "第三目标", "晋升上次好档");
        assert_eq!(rec2.blocked_streak, 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn wake_throttle_durable_before_prompt() {
        // goalhard-01 风暴闸族:prompt 失败时节流先落盘(旧形 prompt 先发
        // 写后落,写失败即下趟重唤风暴);第二趟 skip-throttled 不再发。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = fresh_home("wt");
        let user = fresh_home("wt-user");
        let proj = Path::new("/w/proj");
        set_goal(&home, proj, "盯发布").unwrap();
        let mut aged = show_goal(&home, proj).unwrap().unwrap();
        aged.set_at = 1;
        aged.last_stop_at = 1;
        write_record(&goal_path(&home, proj), &aged).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hst-goal-wakefail-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let list = dir.join("list.json");
        std::fs::write(
            &list,
            format!(
                r#"{{"result":{{"agents":[{{"agent":"claude","agent_status":"idle","cwd":"{}","pane_id":"w1:p5"}}]}}}}"#,
                proj.display()
            ),
        )
        .unwrap();
        // prompt 即败(list 必须通,唤醒发不出去);路径烤进脚本不走 env
        //（测试进程 env 是全局可变共享面,免并发串扰）。
        std::fs::write(
            &bin.join("herdr"),
            format!(
                concat!(
                    "#!/bin/sh\n",
                    "if [ \"$1 $2\" = \"agent list\" ]; then cat {list}; exit 0; fi\n",
                    "if [ \"$1 $2\" = \"agent prompt\" ]; then exit 1; fi\n",
                ),
                list = list.display(),
            ),
        )
        .unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin.join("herdr"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let saved_path = std::env::var("PATH").unwrap_or_default();
        let path_env = format!("{}:{}", bin.display(), saved_path);
        std::env::set_var("PATH", &path_env);
        let rows = run_heal(&home, &user, Some(proj), false, false, None).unwrap();
        assert_eq!(rows[0].action, "wake-fail", "{:?}", rows[0].detail);
        // 节流已落盘:last_heal_at 加 streak 持久,不因 prompt 失败回滚。
        let rec = show_goal(&home, proj).unwrap().unwrap();
        assert!(rec.last_heal_at > 0, "节流先落盘: {rec:?}");
        assert_eq!(rec.heal_streak, 1);
        // 第二趟:静默窗频控,不再发 prompt(风暴闸实证)。
        let rows2 = run_heal(&home, &user, Some(proj), false, false, None).unwrap();
        assert_eq!(rows2[0].action, "skip-throttled");
        std::env::set_var("PATH", &saved_path);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[cfg(unix)]
    #[test]
    fn yolo_heal_cooldown_durable_before_sequence() {
        // goalhard-01 风暴闸族:治序列失败时冷却已先落盘(旧形治后落,写
        // 失败即每趟 sweep 重启格风暴);第二趟不再治;手动面 heavy_deferred
        // 上帽单趟一个治动作。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = fresh_home("yc");
        let user = fresh_home("yc-user");
        let proj = Path::new("/w/proj");
        set_goal(&home, proj, "解阻塞").unwrap();
        set_yolo_heal_armed(&home, true).unwrap();
        let mut aged = show_goal(&home, proj).unwrap().unwrap();
        aged.set_at = 1;
        aged.last_stop_at = 1;
        aged.blocked_streak = 1;
        aged.blocked_rounds = 0;
        write_record(&goal_path(&home, proj), &aged).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hst-goal-yolocool-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let list = dir.join("list.json");
        let keys = dir.join("keys.log");
        std::fs::write(
            &list,
            format!(
                r#"{{"result":{{"agents":[{{"agent":"claude","agent_status":"blocked","cwd":"{}","pane_id":"w1:p5"}}]}}}}"#,
                proj.display()
            ),
        )
        .unwrap();
        // send-keys 计数后即败(治序列第一步即折,快路径);路径烤进脚本。
        std::fs::write(
            &bin.join("herdr"),
            format!(
                concat!(
                    "#!/bin/sh\n",
                    "if [ \"$1 $2\" = \"agent list\" ]; then cat {list}; exit 0; fi\n",
                    "if [ \"$1 $2\" = \"agent send-keys\" ]; then echo x >> {keys}; exit 1; fi\n",
                ),
                list = list.display(),
                keys = keys.display(),
            ),
        )
        .unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin.join("herdr"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let saved_path = std::env::var("PATH").unwrap_or_default();
        let path_env = format!("{}:{}", bin.display(), saved_path);
        std::env::set_var("PATH", &path_env);
        // 第一趟(streak bump 到 2 即阈值):治序列失败,冷却已先落盘。
        let rows = run_heal(&home, &user, Some(proj), false, true, None).unwrap();
        assert_eq!(rows[0].action, "yolo-heal-fail", "{:?}", rows[0].detail);
        let rec = show_goal(&home, proj).unwrap().unwrap();
        assert!(rec.yolo_heal_at > 0, "冷却先落盘: {rec:?}");
        // 第二趟:冷却窗(6h)内不再治,防抖窗内静默计数。
        let rows2 = run_heal(&home, &user, Some(proj), false, true, None).unwrap();
        assert_eq!(rows2[0].action, "blocked-muted", "{:?}", rows2[0].detail);
        assert_eq!(
            std::fs::read_to_string(&keys).unwrap().lines().count(),
            1,
            "治动作只发生一次(冷却耐久)"
        );
        std::env::set_var("PATH", &saved_path);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[cfg(unix)]
    #[test]
    fn sweeper_heavy_cap_one_action_per_pass() {
        // goalhard-01 风暴闸族:两档同 Eligibility,sweeper 形(cap=Some(1))
        // 单趟只治一档,另一档 heavy-deferred(dry 双证:两档保持 Eligible
        // 下仍只见一个 would-yolo-heal);实跑治序列落键级联(首档治后用户
        // 级 yolo 全落,次档非 yolo 前置失效)叠加帽后单趟仍只一键序。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = fresh_home("hc");
        let user = fresh_home("hc-user");
        let pa = Path::new("/w/a");
        let pb = Path::new("/w/b");
        for root in [pa, pb] {
            set_goal(&home, root, "解阻塞").unwrap();
            let mut aged = show_goal(&home, root).unwrap().unwrap();
            aged.set_at = 1;
            aged.last_stop_at = 1;
            aged.blocked_streak = 1;
            aged.blocked_rounds = 0;
            write_record(&goal_path(&home, root), &aged).unwrap();
        }
        set_yolo_heal_armed(&home, true).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hst-goal-heavycap-{}-{}",
            std::process::id(),
            unix_secs()
        ));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let list = dir.join("list.json");
        let keys = dir.join("keys.log");
        std::fs::write(
            &list,
            format!(
                r#"{{"result":{{"agents":[{{"agent":"claude","agent_status":"blocked","cwd":"{}","pane_id":"w1:p1"}},{{"agent":"claude","agent_status":"blocked","cwd":"{}","pane_id":"w1:p2"}}]}}}}"#,
                pa.display(),
                pb.display()
            ),
        )
        .unwrap();
        // send-keys 计数后即败(治序列第一步即折,快路径);路径烤进脚本。
        std::fs::write(
            &bin.join("herdr"),
            format!(
                concat!(
                    "#!/bin/sh\n",
                    "if [ \"$1 $2\" = \"agent list\" ]; then cat {list}; exit 0; fi\n",
                    "if [ \"$1 $2\" = \"agent send-keys\" ]; then echo x >> {keys}; exit 1; fi\n",
                ),
                list = list.display(),
                keys = keys.display(),
            ),
        )
        .unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin.join("herdr"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let saved_path = std::env::var("PATH").unwrap_or_default();
        let path_env = format!("{}:{}", bin.display(), saved_path);
        std::env::set_var("PATH", &path_env);
        // dry 双证:两档均 Eligible,cap=1 下只一档 would-yolo-heal,另一档
        // heavy-deferred(dry 零副作用,两档 Eligibility 不因治变)。
        let rows = run_heal(&home, &user, None, true, true, Some(1)).unwrap();
        let mut actions: Vec<&str> = rows.iter().map(|r| r.action.as_str()).collect();
        actions.sort_unstable();
        assert_eq!(
            actions,
            vec!["heavy-deferred", "would-yolo-heal"],
            "{rows:?}"
        );
        // 实跑:首档治序列(键序即折),次档或被帽让位或因落键级联非
        // Eligible,单趟仍只一键序。
        let rows2 = run_heal(&home, &user, None, false, true, Some(1)).unwrap();
        assert!(
            rows2.iter().any(|r| r.action == "yolo-heal-fail"),
            "{rows2:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&keys).unwrap().lines().count(),
            1,
            "单趟只一键序(帽加落键级联双保险)"
        );
        std::env::set_var("PATH", &saved_path);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn goal_files_filter_and_stale_tmp_swept() {
        // goalhard-01 收集器:goal_files 只收在册主档(治理件加 bak 加
        // 坏档隔离件加 tmp 全排除);崩溃残件 tmp 超龄被扫描趟清走。
        let home = fresh_home("gf");
        let root = Path::new("/w/p");
        set_goal(&home, root, "在册").unwrap();
        let dir = goal_dir(&home);
        let main = goal_path(&home, root);
        for (name, body) in [
            (".heal-log", "1 a b c\n"),
            (".sweeper-lock", "1 2 3\n"),
            (
                "-w-p.json.bak",
                "{\"text\":\"旧\",\"set_at\":1,\"rounds\":0,\"last_stop_at\":0}",
            ),
            ("-w-p.json.corrupt-1-ab", "junk"),
            ("-w-p.json.123-abc.tmp", "half"),
            ("x.json.9-1.tmp", "half2"),
        ] {
            std::fs::write(dir.join(name), body).unwrap();
        }
        let files = goal_files(&home);
        assert_eq!(files, vec![main.clone()], "只收在册主档: {files:?}");
        // tmp 清扫:注入未来时刻,全部 tmp 超龄清走,主档与 bak 不动。
        sweep_stale_tmp(&home, unix_secs() + TMP_STALE_SECS + 60);
        assert!(!dir.join("-w-p.json.123-abc.tmp").exists());
        assert!(!dir.join("x.json.9-1.tmp").exists());
        assert!(main.exists());
        assert!(dir.join("-w-p.json.bak").exists(), "bak 不被 tmp 清扫误伤");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn heal_and_goal_audit_logs_rotate_at_cap() {
        // goalhard-01 留痕轮转:过 AUDIT_LOG_CAP_BYTES 先滚 .1(keep-one)
        // 再写,长役不无限增长。
        let home = fresh_home("rot");
        let dir = goal_dir(&home);
        std::fs::create_dir_all(&dir).unwrap();
        let heal = dir.join(".heal-log");
        std::fs::write(&heal, "x".repeat(AUDIT_LOG_CAP_BYTES as usize + 1)).unwrap();
        heal_log_append(&home, "s", "woke", "d");
        assert!(
            std::fs::metadata(&heal).unwrap().len() < AUDIT_LOG_CAP_BYTES,
            "新档回小"
        );
        assert!(
            std::fs::metadata(dir.join(".heal-log.1")).unwrap().len() > AUDIT_LOG_CAP_BYTES,
            "旧档滚 .1"
        );
        assert_eq!(std::fs::read_to_string(&heal).unwrap().lines().count(), 1);
        let audit = dir.join(".goal-audit");
        std::fs::write(&audit, "y".repeat(AUDIT_LOG_CAP_BYTES as usize + 1)).unwrap();
        goal_audit_append(&home, "s", "set", "d");
        assert!(std::fs::metadata(dir.join(".goal-audit.1")).unwrap().len() > AUDIT_LOG_CAP_BYTES);
        assert_eq!(std::fs::read_to_string(&audit).unwrap().lines().count(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn list_goals_reports_rows_with_repair_notes() {
        // goalhard-01 机读面:list_goals 字典序逐档,坏档修复注记入行,
        // 空目录合法空清单。
        let home = fresh_home("lg2");
        assert!(list_goals(&home).unwrap().is_empty(), "空目录空清单");
        set_goal(&home, Path::new("/w/b"), "B 档").unwrap();
        // A 连发两遍:第二遍前主档可解析,刷出 .bak(晋升源)。
        set_goal(&home, Path::new("/w/a"), "A 旧档").unwrap();
        set_goal(&home, Path::new("/w/a"), "A 档").unwrap();
        let rows = list_goals(&home).unwrap();
        let slugs: Vec<&str> = rows.iter().map(|r| r.slug.as_str()).collect();
        assert_eq!(slugs, vec!["-w-a", "-w-b"], "字典序稳定: {slugs:?}");
        assert!(rows.iter().all(|r| r.rec.is_some() && r.repaired.is_none()));
        // 坏档:show 未触过,list 触修复(bak 晋升)注记入行。
        let pa = goal_path(&home, Path::new("/w/a"));
        std::fs::write(&pa, "bad").unwrap();
        let rows2 = list_goals(&home).unwrap();
        let row_a = rows2.iter().find(|r| r.slug == "-w-a").unwrap();
        assert!(
            row_a
                .repaired
                .as_ref()
                .is_some_and(|n| n.action == "promote" && !n.quarantined.is_empty()),
            "修复注记对象形: {:?}",
            row_a.repaired
        );
        assert!(row_a.rec.is_some(), "晋升后照常在册");
        let _ = std::fs::remove_dir_all(&home);
    }
}
