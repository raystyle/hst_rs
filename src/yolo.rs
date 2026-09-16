use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use toml::Value as Toml;

use crate::pathutil::{abs_display, forward_slash, native_slash};

/// 键落盘报告：写入路径清单。
pub struct ApplyReport {
    /// 该字段承载yolo 键的wrote数据。
    pub wrote: Vec<String>,
}

pub(crate) fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path)?;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn read_json(path: &Path) -> Result<Json, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    // D52：BOM 容忍——舰队 PowerShell 脚本常写 UTF-8 BOM，serde_json 对带
    // BOM 头整体拒收（doctor 假报 missing、init 硬错），读侧剥 U+FEFF。
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &Json) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())? + "\n";
    write_text(path, &text)
}

pub(crate) fn read_toml(path: &Path) -> Result<Toml, String> {
    if !path.exists() {
        return Ok(Toml::Table(toml::map::Map::new()));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Toml::Table(toml::map::Map::new()));
    }
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_toml(path: &Path, value: &Toml) -> Result<(), String> {
    let text = toml::to_string_pretty(value).map_err(|e| e.to_string())?;
    write_text(path, &text)
}

pub(crate) fn toml_write(path: &Path, value: &Toml) -> Result<(), String> {
    write_toml(path, value)
}

fn table_mut(v: &mut Toml) -> Result<&mut toml::map::Map<String, Toml>, String> {
    match v {
        Toml::Table(t) => Ok(t),
        _ => Err("expected TOML table".into()),
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// yolo 分级（D33，2026-09-13 ohmycloud 协调批）：full = 现行全 bypass；
/// partial = 危险操作仍确认（编辑类自动过、命令执行与 MCP 审批保留确认：
/// ours 落的 enableAllProjectMcpServers 随降级摘除，同批或重跑 --pre-trust
/// 会再开；enabledMcpjsonServers 名单含 agent 原生用户审批与 hst 落值、
/// 无法归因落写者故一律保留，彻底恢复 MCP 确认需手动清名单）；
/// off = 全关（摘 hst 落键恢复各家默认确认，用户自设值保留）。
/// 四家 partial 取值：claude `acceptEdits`（官方 permission-mode 取值）、
/// codex `workspace-write` 加 `on-request`（官方 config 文档）、kimi `auto`
/// （doctor 既有 ok 值）、grok `auto`（grok-build `permissions.rs`
/// `parse_permission_mode_canonical` 值集 always-approve|auto|ask，未知串
/// 回落 ask 安全向 [实证: 2026-09-13 上游源码加本机 --help 复核]）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum YoloLevel {
    /// 全 bypass：编辑与命令执行全放行（现行 --yolo 行为）
    Full,
    /// yolo 键的危险操作仍确认面（细则见 R002 与模块文档）。
    Partial,
    /// yolo 键的全关面（细则见 R002 与模块文档）。
    Off,
}

impl YoloLevel {
    /// yolo 键的as_str面（细则见 R002 与模块文档）。
    pub fn as_str(self) -> &'static str {
        match self {
            YoloLevel::Full => "full",
            YoloLevel::Partial => "partial",
            YoloLevel::Off => "off",
        }
    }
}

/// 每级的四家落键值（off 无写入键，走退役路径）。
struct LevelKeys {
    claude_mode: &'static str,
    codex_sandbox: &'static str,
    codex_approval: &'static str,
    kimi_mode: &'static str,
    grok_mode: &'static str,
}

fn level_keys(level: YoloLevel) -> LevelKeys {
    match level {
        YoloLevel::Full => LevelKeys {
            claude_mode: "bypassPermissions",
            codex_sandbox: "danger-full-access",
            codex_approval: "never",
            kimi_mode: "yolo",
            grok_mode: "always-approve",
        },
        YoloLevel::Partial => LevelKeys {
            claude_mode: "acceptEdits",
            codex_sandbox: "workspace-write",
            codex_approval: "on-request",
            kimi_mode: "auto",
            grok_mode: "auto",
        },
        YoloLevel::Off => unreachable!("off has no write keys"),
    }
}

/// hst 落过的键值集（退役判 ours 用，含 full 与 partial 两代值）。
const OURS_CLAUDE_MODES: &[&str] = &["bypassPermissions", "acceptEdits"];
const OURS_CODEX_SANDBOX: &[&str] = &["danger-full-access", "workspace-write"];
const OURS_CODEX_APPROVAL: &[&str] = &["never", "on-request"];
const OURS_KIMI_MODES: &[&str] = &["yolo", "auto"];
const OURS_GROK_MODES: &[&str] = &["always-approve", "auto"];

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 用户级 yolo 与非阻塞键（D28 第 2 轮裁定 2026-09-11：yolo 模式与非阻塞
/// 模式也用户级，覆盖 D25「oma --yolo 面向项目级」口径；历史脉络 = 用户级
/// 需求自 POC 期「不改家目录」约束起被逐面翻正：状态栏 P0027、kimi M058、
/// hook D28、本面）。四家写入面：claude `~/.claude/settings.json`
/// （permissions.defaultMode 加 skipDangerousModePermissionPrompt 加
/// enableAllProjectMcpServers）、codex `~/.codex/config.toml`
/// （sandbox_mode 加 approval_policy）、kimi `~/.kimi-code/config.toml`
/// （default_permission_mode）、grok `~/.grok/config.toml`
/// （`[ui]` permission_mode）。项目级旧键由 init 退役（retire_project_yolo）。
/// D33 起 level 分级：partial 摘 ours 落的 enableAllProjectMcpServers（MCP
/// 审批恢复确认，--pre-trust 重跑可再开；codex 评审 F1 裁 a）；off 走
/// retire_user_yolo_with。
pub fn apply_user_yolo_with(user_home: &Path) -> Result<ApplyReport, String> {
    apply_user_yolo_level_with(user_home, YoloLevel::Full)
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 用户级分级落键（full / partial；off 调 retire_user_yolo_with）。
pub fn apply_user_yolo_level_with(
    user_home: &Path,
    level: YoloLevel,
) -> Result<ApplyReport, String> {
    assert!(
        level != YoloLevel::Off,
        "off goes through retire_user_yolo_with"
    );
    let keys = level_keys(level);
    let mut wrote = Vec::new();

    let claude_user = user_home.join(".claude").join("settings.json");
    let mut shared = read_json(&claude_user)?;
    if !shared.is_object() {
        shared = json!({});
    }
    {
        let obj = shared.as_object_mut().unwrap();
        let permissions = obj
            .entry("permissions".to_string())
            .or_insert_with(|| json!({}));
        if !permissions.is_object() {
            *permissions = json!({});
        }
        permissions
            .as_object_mut()
            .unwrap()
            .insert("defaultMode".into(), json!(keys.claude_mode));
        // D52 宿主令：full 落读块关（「非阻塞键落盘」承诺兑现；off 等值
        // 摘，partial 不动该键）。
        if level == YoloLevel::Full {
            permissions.as_object_mut().unwrap().insert(
                "blockReadsOutsideWorkingDirectories".into(),
                Json::Bool(false),
            );
        }
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        if level == YoloLevel::Full {
            // 用户级 enableAll 覆盖所有项目（per-project enabledMcpjsonServers
            // 名单是项目语义，不上用户级）。
            obj.insert("enableAllProjectMcpServers".into(), Json::Bool(true));
        } else {
            // partial 摘 ours 落的 enableAll（codex 评审 F1 裁 a，2026-09-13）：
            // full 降级后 MCP 审批要恢复确认；--pre-trust 面写的同值一并摘
            // （ours 判定不分落写者，同批或重跑 --pre-trust 会再开）。名单
            // enabledMcpjsonServers 含 agent 原生用户审批与 hst 落值、无法
            // 归因故一律保留（用户自设值保留纪律，codex 二轮 A 裁改口径）。
            if obj
                .get("enableAllProjectMcpServers")
                .and_then(|x| x.as_bool())
                == Some(true)
            {
                obj.remove("enableAllProjectMcpServers");
            }
        }
    }
    write_json(&claude_user, &shared)?;
    wrote.push(claude_user.display().to_string());

    let codex_user = user_home.join(".codex").join("config.toml");
    let mut ctoml = read_toml(&codex_user)?;
    {
        let t = table_mut(&mut ctoml)?;
        t.insert(
            "sandbox_mode".into(),
            Toml::String(keys.codex_sandbox.into()),
        );
        t.insert(
            "approval_policy".into(),
            Toml::String(keys.codex_approval.into()),
        );
    }
    write_toml(&codex_user, &ctoml)?;
    wrote.push(codex_user.display().to_string());

    let kimi_user = user_home.join(".kimi-code").join("config.toml");
    let mut ktoml = read_toml(&kimi_user)?;
    table_mut(&mut ktoml)?.insert(
        "default_permission_mode".into(),
        Toml::String(keys.kimi_mode.into()),
    );
    write_toml(&kimi_user, &ktoml)?;
    wrote.push(kimi_user.display().to_string());

    let grok_cfg = user_home.join(".grok").join("config.toml");
    let mut gc = read_toml(&grok_cfg)?;
    {
        let t = table_mut(&mut gc)?;
        let ui = t
            .entry("ui".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(ui)?.insert(
            "permission_mode".into(),
            Toml::String(keys.grok_mode.into()),
        );
    }
    write_toml(&grok_cfg, &gc)?;
    wrote.push(grok_cfg.display().to_string());

    Ok(ApplyReport { wrote })
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 项目级 yolo 与非阻塞键（D28 第 3 轮裁定 2026-09-11：yolo 命令分两级，
/// `hst init --project-yolo` 显式选项目级；v0.5.3 前的默认行为收编为显式
/// 旗标）。写入面：claude 项目 `.claude/settings.json` 加
/// `settings.local.json`、codex 项目 `.codex/config.toml`（yolo 键加项目
/// 信任预种）、kimi 项目 `.kimi-code/config.toml`；grok 无项目级面
/// （permission_mode 只能写用户级，S025）。D33 起 level 分级：partial 的
/// local 摘 ours 落的 enableAllProjectMcpServers（F1 裁 a，与用户级同纪律），
/// codex 项目信任预种不分级（信任门是另一面，项目 config 层能加载才有分级
/// 可言）。
pub fn apply_project_yolo(root: &Path) -> Result<ApplyReport, String> {
    apply_project_yolo_level(root, YoloLevel::Full)
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 项目级分级落键（full / partial；off 调 retire_project_yolo）。
pub fn apply_project_yolo_level(root: &Path, level: YoloLevel) -> Result<ApplyReport, String> {
    assert!(
        level != YoloLevel::Off,
        "off goes through retire_project_yolo"
    );
    let keys = level_keys(level);
    let root = abs_display(root);
    let mut wrote = Vec::new();

    let claude_shared = root.join(".claude").join("settings.json");
    let mut shared = read_json(&claude_shared)?;
    if !shared.is_object() {
        shared = json!({});
    }
    {
        let obj = shared.as_object_mut().unwrap();
        let permissions = obj
            .entry("permissions".to_string())
            .or_insert_with(|| json!({}));
        if !permissions.is_object() {
            *permissions = json!({});
        }
        permissions
            .as_object_mut()
            .unwrap()
            .insert("defaultMode".into(), json!(keys.claude_mode));
        // D52（codex F2）：项目 full 对称落读块关（与用户级同一承诺）。
        if level == YoloLevel::Full {
            permissions.as_object_mut().unwrap().insert(
                "blockReadsOutsideWorkingDirectories".into(),
                Json::Bool(false),
            );
        }
    }
    write_json(&claude_shared, &shared)?;
    wrote.push(claude_shared.display().to_string());

    let claude_local = root.join(".claude").join("settings.local.json");
    let mut local = read_json(&claude_local)?;
    if !local.is_object() {
        local = json!({});
    }
    {
        let obj = local.as_object_mut().unwrap();
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        if level == YoloLevel::Full {
            apply_mcp_approvals(obj, &root);
        } else if obj
            .get("enableAllProjectMcpServers")
            .and_then(|x| x.as_bool())
            == Some(true)
        {
            // partial 摘 ours 落的 enableAll（F1 裁 a，与用户级同纪律）；
            // 名单 enabledMcpjsonServers 无法归因落写者，一律保留。
            obj.remove("enableAllProjectMcpServers");
        }
    }
    write_json(&claude_local, &local)?;
    wrote.push(claude_local.display().to_string());

    let native = native_slash(&root);
    let codex = root.join(".codex").join("config.toml");
    let mut ctoml = read_toml(&codex)?;
    {
        let t = table_mut(&mut ctoml)?;
        t.insert(
            "sandbox_mode".into(),
            Toml::String(keys.codex_sandbox.into()),
        );
        t.insert(
            "approval_policy".into(),
            Toml::String(keys.codex_approval.into()),
        );
        let projects = t
            .entry("projects".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let projects = table_mut(projects)?;
        let key = if cfg!(windows) {
            native.to_ascii_lowercase()
        } else {
            native.clone()
        };
        let proj = projects
            .entry(key)
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(proj)?.insert("trust_level".into(), Toml::String("trusted".into()));
    }
    write_toml(&codex, &ctoml)?;
    wrote.push(codex.display().to_string());

    let kimi = root.join(".kimi-code").join("config.toml");
    let mut ktoml = read_toml(&kimi)?;
    table_mut(&mut ktoml)?.insert(
        "default_permission_mode".into(),
        Toml::String(keys.kimi_mode.into()),
    );
    write_toml(&kimi, &ktoml)?;
    wrote.push(kimi.display().to_string());

    Ok(ApplyReport { wrote })
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// yolo 键的生产入口面（细则见 R002 与模块文档）。
pub fn apply_user_yolo() -> Result<ApplyReport, String> {
    let home = crate::pathutil::user_home()?;
    apply_user_yolo_with(&home)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// yolo 键的生产入口面（细则见 R002 与模块文档）。
pub fn apply_user_yolo_level(level: YoloLevel) -> Result<ApplyReport, String> {
    let home = crate::pathutil::user_home()?;
    apply_user_yolo_level_with(&home, level)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// yolo 键的生产入口面（细则见 R002 与模块文档）。
pub fn retire_user_yolo() -> Result<Vec<String>, String> {
    let home = crate::pathutil::user_home()?;
    retire_user_yolo_with(&home)
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 项目级旧 yolo 键退役（D28 第 2 轮）：oma 写过的项目面键摘除（值等于
/// ours 落值才动，用户自设其它值保留），整文件只剩空对象/空表时删文件。
/// D33 起 ours 值集含 full 与 partial 两代。返回变更描述（deploy report 收录）。
pub fn retire_project_yolo(root: &Path) -> Result<Vec<String>, String> {
    let root = abs_display(root);
    let mut changed = Vec::new();

    // claude 项目 settings.json：permissions.defaultMode 是 ours 落值才摘；
    // D52（codex F2）：ours 落的 blockReads=false 对称等值摘。
    let claude_shared = root.join(".claude").join("settings.json");
    if claude_shared.exists() {
        let mut v = read_json(&claude_shared)?;
        if v.is_object() {
            let mut dirty = false;
            if v.get("permissions")
                .and_then(|p| p.get("defaultMode"))
                .and_then(|m| m.as_str())
                .is_some_and(|m| OURS_CLAUDE_MODES.contains(&m))
            {
                if let Some(p) = v.get_mut("permissions").and_then(|p| p.as_object_mut()) {
                    p.remove("defaultMode");
                    dirty = true;
                    if p.is_empty() {
                        v.as_object_mut().unwrap().remove("permissions");
                    }
                }
            }
            if v.get("permissions")
                .and_then(|p| p.get("blockReadsOutsideWorkingDirectories"))
                .and_then(|x| x.as_bool())
                == Some(false)
            {
                if let Some(p) = v.get_mut("permissions").and_then(|p| p.as_object_mut()) {
                    p.remove("blockReadsOutsideWorkingDirectories");
                    dirty = true;
                    if p.is_empty() {
                        v.as_object_mut().unwrap().remove("permissions");
                    }
                }
            }
            if dirty {
                if v.as_object().is_some_and(|o| o.is_empty()) {
                    fs::remove_file(&claude_shared)
                        .map_err(|e| format!("{}: {e}", claude_shared.display()))?;
                } else {
                    write_json(&claude_shared, &v)?;
                }
                changed.push(format!("{} (retired-yolo)", claude_shared.display()));
            }
        }
    }

    // claude 项目 settings.local.json：skip / enableAll 摘（ours 值才动）。
    let claude_local = root.join(".claude").join("settings.local.json");
    if claude_local.exists() {
        let mut v = read_json(&claude_local)?;
        if v.is_object() {
            let obj = v.as_object_mut().unwrap();
            let mut dirty = false;
            for key in [
                "skipDangerousModePermissionPrompt",
                "enableAllProjectMcpServers",
            ] {
                if obj.get(key).and_then(|x| x.as_bool()) == Some(true) {
                    obj.remove(key);
                    dirty = true;
                }
            }
            if dirty {
                if obj.is_empty() {
                    fs::remove_file(&claude_local)
                        .map_err(|e| format!("{}: {e}", claude_local.display()))?;
                } else {
                    write_json(&claude_local, &v)?;
                }
                changed.push(format!("{} (retired-yolo)", claude_local.display()));
            }
        }
    }

    // codex 项目 config.toml：sandbox/approval ours 值摘（项目信任键留给
    // pretrust 面的用户 store，不动）。
    let codex = root.join(".codex").join("config.toml");
    if codex.exists() {
        let mut t = read_toml(&codex)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            for (k, ours) in [
                ("sandbox_mode", OURS_CODEX_SANDBOX),
                ("approval_policy", OURS_CODEX_APPROVAL),
            ] {
                if map
                    .get(k)
                    .and_then(|v| v.as_str())
                    .is_some_and(|v| ours.contains(&v))
                {
                    map.remove(k);
                    dirty = true;
                }
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&codex).map_err(|e| format!("{}: {e}", codex.display()))?;
                } else {
                    write_toml(&codex, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", codex.display()));
            }
        }
    }

    // kimi 项目 config.toml：default_permission_mode ours 值摘。
    let kimi = root.join(".kimi-code").join("config.toml");
    if kimi.exists() {
        let mut t = read_toml(&kimi)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            if map
                .get("default_permission_mode")
                .and_then(|v| v.as_str())
                .is_some_and(|m| OURS_KIMI_MODES.contains(&m))
            {
                map.remove("default_permission_mode");
                dirty = true;
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&kimi).map_err(|e| format!("{}: {e}", kimi.display()))?;
                } else {
                    write_toml(&kimi, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", kimi.display()));
            }
        }
    }

    Ok(changed)
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 用户级 yolo 键退役（D33 off）：与 retire_project_yolo 同款 ours 等值
/// 摘除策略（hst 落值才动、用户自设值保留、整文件只剩空对象/空表时删）。
/// yolo 键的注意面（细则见 R002 与模块文档）。
/// 也由 pretrust 面写入，ours 判定无法区分落写者，off 一并摘除；需要 MCP
/// 直通请重跑 `--pre-trust`。grok 只摘 `[ui]` permission_mode（ours 值）。
pub fn retire_user_yolo_with(user_home: &Path) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();

    let claude_user = user_home.join(".claude").join("settings.json");
    if claude_user.exists() {
        let mut v = read_json(&claude_user)?;
        if v.is_object() {
            let mut dirty = false;
            if v.get("permissions")
                .and_then(|p| p.get("defaultMode"))
                .and_then(|m| m.as_str())
                .is_some_and(|m| OURS_CLAUDE_MODES.contains(&m))
            {
                if let Some(p) = v.get_mut("permissions").and_then(|p| p.as_object_mut()) {
                    p.remove("defaultMode");
                    dirty = true;
                    if p.is_empty() {
                        v.as_object_mut().unwrap().remove("permissions");
                    }
                }
            }
            // D52：ours 落的 blockReads=false 等值摘（摘后缺省亦 false，
            // 语义无差；ours 值集纪律同 skip 键）。
            if v.get("permissions")
                .and_then(|p| p.get("blockReadsOutsideWorkingDirectories"))
                .and_then(|x| x.as_bool())
                == Some(false)
            {
                if let Some(p) = v.get_mut("permissions").and_then(|p| p.as_object_mut()) {
                    p.remove("blockReadsOutsideWorkingDirectories");
                    dirty = true;
                    if p.is_empty() {
                        v.as_object_mut().unwrap().remove("permissions");
                    }
                }
            }
            let obj = v.as_object_mut().unwrap();
            for key in [
                "skipDangerousModePermissionPrompt",
                "enableAllProjectMcpServers",
            ] {
                if obj.get(key).and_then(|x| x.as_bool()) == Some(true) {
                    obj.remove(key);
                    dirty = true;
                }
            }
            if dirty {
                if v.as_object().is_some_and(|o| o.is_empty()) {
                    fs::remove_file(&claude_user)
                        .map_err(|e| format!("{}: {e}", claude_user.display()))?;
                } else {
                    write_json(&claude_user, &v)?;
                }
                changed.push(format!("{} (retired-yolo)", claude_user.display()));
            }
        }
    }

    let codex_user = user_home.join(".codex").join("config.toml");
    if codex_user.exists() {
        let mut t = read_toml(&codex_user)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            for (k, ours) in [
                ("sandbox_mode", OURS_CODEX_SANDBOX),
                ("approval_policy", OURS_CODEX_APPROVAL),
            ] {
                if map
                    .get(k)
                    .and_then(|v| v.as_str())
                    .is_some_and(|v| ours.contains(&v))
                {
                    map.remove(k);
                    dirty = true;
                }
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&codex_user)
                        .map_err(|e| format!("{}: {e}", codex_user.display()))?;
                } else {
                    write_toml(&codex_user, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", codex_user.display()));
            }
        }
    }

    let kimi_user = user_home.join(".kimi-code").join("config.toml");
    if kimi_user.exists() {
        let mut t = read_toml(&kimi_user)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            if map
                .get("default_permission_mode")
                .and_then(|v| v.as_str())
                .is_some_and(|m| OURS_KIMI_MODES.contains(&m))
            {
                map.remove("default_permission_mode");
                dirty = true;
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&kimi_user)
                        .map_err(|e| format!("{}: {e}", kimi_user.display()))?;
                } else {
                    write_toml(&kimi_user, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", kimi_user.display()));
            }
        }
    }

    let grok_cfg = user_home.join(".grok").join("config.toml");
    if grok_cfg.exists() {
        let mut t = read_toml(&grok_cfg)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            if let Some(ui) = map.get_mut("ui").and_then(|u| u.as_table_mut()) {
                if ui
                    .get("permission_mode")
                    .and_then(|v| v.as_str())
                    .is_some_and(|m| OURS_GROK_MODES.contains(&m))
                {
                    ui.remove("permission_mode");
                    dirty = true;
                }
            }
            if dirty {
                if map
                    .get("ui")
                    .and_then(|u| u.as_table())
                    .is_some_and(|u| u.is_empty())
                {
                    map.remove("ui");
                }
                if map.is_empty() {
                    fs::remove_file(&grok_cfg)
                        .map_err(|e| format!("{}: {e}", grok_cfg.display()))?;
                } else {
                    write_toml(&grok_cfg, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", grok_cfg.display()));
            }
        }
    }

    Ok(changed)
}

/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上。
/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Trust stores in the user home. Not hook registration.
pub fn apply_pretrust(root: &Path) -> Result<ApplyReport, String> {
    let root = abs_display(root);
    let home = crate::pathutil::user_home()?;
    let mut wrote = Vec::new();
    let native = native_slash(&root);
    let fwd = forward_slash(&root);

    let claude_json = home.join(".claude.json");
    let mut cj = read_json(&claude_json)?;
    if !cj.is_object() {
        cj = json!({});
    }
    {
        let obj = cj.as_object_mut().unwrap();
        obj.insert("hasCompletedOnboarding".into(), Json::Bool(true));
        let projects = obj
            .entry("projects".to_string())
            .or_insert_with(|| json!({}));
        if !projects.is_object() {
            *projects = json!({});
        }
        let entry = projects
            .as_object_mut()
            .unwrap()
            .entry(fwd.clone())
            .or_insert_with(|| json!({}));
        if let Some(m) = entry.as_object_mut() {
            m.insert("hasTrustDialogAccepted".into(), Json::Bool(true));
            m.insert("hasTrustDialogHooksAccepted".into(), Json::Bool(true));
        }
    }
    write_json(&claude_json, &cj)?;
    wrote.push(claude_json.display().to_string());

    let user_claude = home.join(".claude").join("settings.json");
    let mut us = read_json(&user_claude)?;
    if !us.is_object() {
        us = json!({});
    }
    {
        let obj = us.as_object_mut().unwrap();
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        apply_mcp_approvals(obj, &root);
    }
    write_json(&user_claude, &us)?;
    wrote.push(user_claude.display().to_string());

    let codex_user = home.join(".codex").join("config.toml");
    let mut cu = read_toml(&codex_user)?;
    {
        let t = table_mut(&mut cu)?;
        let projects = t
            .entry("projects".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let key = if cfg!(windows) {
            native.to_ascii_lowercase()
        } else {
            native.clone()
        };
        let proj = table_mut(projects)?
            .entry(key)
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(proj)?.insert("trust_level".into(), Toml::String("trusted".into()));
    }
    write_toml(&codex_user, &cu)?;
    wrote.push(codex_user.display().to_string());

    let kimi_home = home.join(".kimi-code");
    let key = kimi_workspace_key(&root);
    let ws_path = kimi_home.join("workspaces.json");
    let mut ws = read_json(&ws_path)?;
    if !ws.is_object() {
        ws = json!({ "version": 1, "workspaces": {} });
    }
    {
        let obj = ws.as_object_mut().unwrap();
        obj.entry("version".to_string())
            .or_insert(Json::Number(1.into()));
        let workspaces = obj
            .entry("workspaces".to_string())
            .or_insert_with(|| json!({}));
        if !workspaces.is_object() {
            *workspaces = json!({});
        }
        workspaces.as_object_mut().unwrap().insert(
            key.clone(),
            json!({
                "root": native,
                "name": root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
            }),
        );
    }
    write_json(&ws_path, &ws)?;
    wrote.push(ws_path.display().to_string());

    let trust_file = kimi_home.join("workspace-trust").join(&key);
    write_json(
        &trust_file,
        &json!({ "root": native, "trustedAt": unix_millis() as u64 }),
    )?;
    wrote.push(trust_file.display().to_string());

    let grok_tf = home.join(".grok").join("trusted_folders.toml");
    let mut gt = read_toml(&grok_tf)?;
    {
        let t = table_mut(&mut gt)?;
        let folders = t
            .entry("folders".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let folder = table_mut(folders)?
            .entry(native.clone())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let ft = table_mut(folder)?;
        ft.insert("trusted".into(), Toml::Boolean(true));
        ft.insert("decided_at".into(), Toml::Integer(unix_secs() as i64));
    }
    write_toml(&grok_tf, &gt)?;
    wrote.push(grok_tf.display().to_string());

    let grok_cfg = home.join(".grok").join("config.toml");
    let mut gc = read_toml(&grok_cfg)?;
    {
        let t = table_mut(&mut gc)?;
        let ui = t
            .entry("ui".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(ui)?.insert(
            "permission_mode".into(),
            Toml::String("always-approve".into()),
        );
    }
    write_toml(&grok_cfg, &gc)?;
    wrote.push(grok_cfg.display().to_string());

    Ok(ApplyReport { wrote })
}

fn mcp_json_names(root: &Path) -> Vec<String> {
    let v = read_json(&root.join(".mcp.json")).unwrap_or_else(|_| json!({}));
    v.get("mcpServers")
        .and_then(|m| m.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn apply_mcp_approvals(obj: &mut serde_json::Map<String, Json>, root: &Path) {
    obj.insert("enableAllProjectMcpServers".into(), Json::Bool(true));
    let mut names = mcp_json_names(root);
    if let Some(existing) = obj.get("enabledMcpjsonServers").and_then(|x| x.as_array()) {
        for s in existing {
            if let Some(n) = s.as_str() {
                if !n.is_empty() && !names.iter().any(|x| x == n) {
                    names.push(n.to_string());
                }
            }
        }
    }
    if !names.is_empty() {
        obj.insert(
            "enabledMcpjsonServers".into(),
            Json::Array(names.into_iter().map(Json::String).collect()),
        );
    }
}

/// yolo 键的kimi_workspace_key面（细则见 R002 与模块文档）。
pub fn kimi_workspace_key(root: &Path) -> String {
    let root = abs_display(root);
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let digest = Sha256::digest(native_slash(&root).as_bytes());
    let hex = format!("{digest:x}");
    format!("wd_{}_{}", name, &hex[..12])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::{diagnose, Status};

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn bom_settings_roundtrip_and_full_writes_readblock_false() {
        // D52 宿主实弹回归：舰队 PS 脚本写的 settings.json 带 UTF-8 BOM——
        // 读侧剥 U+FEFF 后 init/yolo 不再硬错；full 落 blockReads=false（非
        // 阻塞承诺）；既有键合并不洗；off 等值摘。
        let home = fresh_dir();
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::write(
            home.join(".claude").join("settings.json"),
            "\u{feff}{\"env\": {\"FOO\": \"bar\"}, \"permissions\": {\"defaultMode\": \"bypassPermissions\", \"blockReadsOutsideWorkingDirectories\": true}}",
        )
        .unwrap();
        apply_user_yolo_with(&home).unwrap();
        let raw = std::fs::read_to_string(home.join(".claude").join("settings.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["env"]["FOO"], "bar", "merge must preserve keys");
        assert_eq!(
            v["permissions"]["defaultMode"], "bypassPermissions",
            "defaultMode kept"
        );
        assert_eq!(
            v["permissions"]["blockReadsOutsideWorkingDirectories"], false,
            "full flips readblock off (D52)"
        );
        // off：ours 落的 false 等值摘（skip 与 enableAll 同摘）。
        retire_user_yolo_with(&home).unwrap();
        let v2: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert!(
            v2["permissions"]
                .get("blockReadsOutsideWorkingDirectories")
                .is_none(),
            "off removes ours-written readblock false"
        );
        assert_eq!(v2["env"]["FOO"], "bar", "retire keeps foreign keys");
        // D52（codex F2）：项目 full 对称落读块关、项目 off 等值摘。
        let proj = fresh_dir();
        std::fs::create_dir_all(proj.join(".claude")).unwrap();
        apply_project_yolo_level(&proj, YoloLevel::Full).unwrap();
        let pv: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(proj.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            pv["permissions"]["blockReadsOutsideWorkingDirectories"], false,
            "project full writes readblock false"
        );
        retire_project_yolo(&proj).unwrap();
        let p2 = proj.join(".claude").join("settings.json");
        if p2.exists() {
            let pv2: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&p2).unwrap()).unwrap();
            assert!(
                pv2["permissions"]
                    .get("blockReadsOutsideWorkingDirectories")
                    .is_none(),
                "project off removes ours-written readblock false"
            );
        }
        // 文件不在 = 整文件只剩 ours 键被删（retire 既有语义），同样通过。
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&proj);
    }

    fn fresh_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-yolo-test-{}-{}-{}",
            std::process::id(),
            unix_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn kimi_workspace_key_shape() {
        let root = fresh_dir();
        let key = kimi_workspace_key(&root);
        let name = root.file_name().unwrap().to_str().unwrap();
        assert!(key.starts_with(&format!("wd_{name}_")), "{key}");
        assert_eq!(key.len(), 3 + name.len() + 1 + 12, "{key}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn user_yolo_clears_file_prompt_blocks_and_retires_project_keys() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir();
        let root = fresh_dir();
        std::env::set_var("HST_USER_HOME", &user);
        let before = diagnose(&root).expect("diagnose");
        assert_eq!(before.status("claude", "yolo"), Some(Status::Block));
        assert_eq!(before.status("codex", "yolo"), Some(Status::Block));

        let report = apply_user_yolo_with(&user).expect("apply");
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("settings.json") && p.contains(".claude")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".codex")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".kimi-code")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".grok")));

        let shared = read_json(&user.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("bypassPermissions")
        );
        assert_eq!(
            shared["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert_eq!(shared["enableAllProjectMcpServers"].as_bool(), Some(true));

        let after = diagnose(&root).expect("diagnose after");
        assert_eq!(after.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("claude", "skip_prompt"), Some(Status::Ok));
        assert_eq!(after.status("codex", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("kimi", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("grok", "yolo"), Some(Status::Ok));
        // 信任面不被 yolo 清（pretrust 是另一个面）。
        assert_eq!(after.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(after.status("codex", "trust.project"), Some(Status::Block));

        // 项目级旧键退役：v0.5.3 形项目摘 ours 值、留用户自设值、空文件删。
        fs::create_dir_all(root.join(".claude")).unwrap();
        write_json(
            &root.join(".claude").join("settings.json"),
            &json!({"permissions": {"defaultMode": "bypassPermissions", "allow": ["Bash"]}}),
        )
        .unwrap();
        write_json(
            &root.join(".claude").join("settings.local.json"),
            &json!({"skipDangerousModePermissionPrompt": true}),
        )
        .unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            root.join(".codex").join("config.toml"),
            "sandbox_mode = \"danger-full-access\"
approval_policy = \"never\"
model = \"gpt\"
",
        )
        .unwrap();
        fs::create_dir_all(root.join(".kimi-code")).unwrap();
        fs::write(
            root.join(".kimi-code").join("config.toml"),
            "default_permission_mode = \"yolo\"
",
        )
        .unwrap();
        let changed = retire_project_yolo(&root).unwrap();
        assert_eq!(changed.len(), 4, "four files retired: {changed:?}");
        let v = read_json(&root.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            v["permissions"]["allow"][0].as_str(),
            Some("Bash"),
            "user-curated permission entries survive"
        );
        assert!(
            v["permissions"].get("defaultMode").is_none(),
            "ours key gone"
        );
        assert!(
            !root.join(".claude").join("settings.local.json").exists(),
            "ours-only local file deleted"
        );
        let codex = fs::read_to_string(root.join(".codex").join("config.toml")).unwrap();
        assert!(codex.contains("model"), "foreign key survives");
        assert!(!codex.contains("sandbox_mode"), "ours key gone");
        assert!(
            !root.join(".kimi-code").join("config.toml").exists(),
            "ours-only kimi project config deleted"
        );
        // 再跑幂等（无变更）。
        assert!(retire_project_yolo(&root).unwrap().is_empty());

        std::env::remove_var("HST_USER_HOME");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn partial_yolo_writes_confirm_still_keys_and_doctor_accepts() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir();
        let root = fresh_dir();
        std::env::set_var("HST_USER_HOME", &user);

        let report = apply_user_yolo_level_with(&user, YoloLevel::Partial).expect("apply");
        assert_eq!(report.wrote.len(), 4, "four user faces written");

        // claude：acceptEdits 加 skip；enableAll 不写（MCP 审批保留确认）。
        let shared = read_json(&user.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("acceptEdits")
        );
        assert_eq!(
            shared["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert!(
            shared.get("enableAllProjectMcpServers").is_none(),
            "partial must not auto-approve project MCP servers"
        );

        // codex：workspace-write / on-request（危险命令仍确认）。
        let codex = read_toml(&user.join(".codex").join("config.toml")).unwrap();
        assert_eq!(
            codex.get("sandbox_mode").and_then(|v| v.as_str()),
            Some("workspace-write")
        );
        assert_eq!(
            codex.get("approval_policy").and_then(|v| v.as_str()),
            Some("on-request")
        );

        // kimi / grok：auto。
        let kimi = read_toml(&user.join(".kimi-code").join("config.toml")).unwrap();
        assert_eq!(
            kimi.get("default_permission_mode").and_then(|v| v.as_str()),
            Some("auto")
        );
        let grok = read_toml(&user.join(".grok").join("config.toml")).unwrap();
        assert_eq!(
            grok.get("ui")
                .and_then(|u| u.get("permission_mode"))
                .and_then(|v| v.as_str()),
            Some("auto")
        );

        // doctor 分级判据：partial 不误报（四家 yolo 均 ok）。
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(d.status("claude", "skip_prompt"), Some(Status::Ok));
        assert_eq!(d.status("codex", "yolo"), Some(Status::Ok));
        assert_eq!(d.status("kimi", "yolo"), Some(Status::Ok));
        assert_eq!(d.status("grok", "yolo"), Some(Status::Ok));

        std::env::remove_var("HST_USER_HOME");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn user_yolo_off_retires_ours_and_keeps_foreign_keys() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir();
        std::env::set_var("HST_USER_HOME", &user);

        apply_user_yolo_with(&user).expect("full");
        // 混入用户自设键：ours 摘除必须幸存。
        {
            let p = user.join(".claude").join("settings.json");
            let mut v = read_json(&p).unwrap();
            v["permissions"]["allow"] = json!(["Bash"]);
            v["model"] = json!("opus");
            write_json(&p, &v).unwrap();
        }
        fs::write(
            user.join(".codex").join("config.toml"),
            "sandbox_mode = \"danger-full-access\"\napproval_policy = \"never\"\nmodel = \"gpt\"\n",
        )
        .unwrap();
        fs::write(
            user.join(".grok").join("config.toml"),
            "[ui]\npermission_mode = \"always-approve\"\nscreen_mode = \"minimal\"\n",
        )
        .unwrap();

        let changed = retire_user_yolo_with(&user).unwrap();
        assert_eq!(changed.len(), 4, "four user faces retired: {changed:?}");
        let v = read_json(&user.join(".claude").join("settings.json")).unwrap();
        assert!(
            v["permissions"].get("defaultMode").is_none(),
            "ours key gone"
        );
        assert_eq!(
            v["permissions"]["allow"][0].as_str(),
            Some("Bash"),
            "user-curated permission entries survive"
        );
        assert!(v.get("skipDangerousModePermissionPrompt").is_none());
        assert!(v.get("enableAllProjectMcpServers").is_none());
        let codex = fs::read_to_string(user.join(".codex").join("config.toml")).unwrap();
        assert!(codex.contains("model"), "foreign key survives");
        assert!(!codex.contains("sandbox_mode"), "ours key gone");
        assert!(
            !user.join(".kimi-code").join("config.toml").exists(),
            "ours-only kimi config deleted"
        );
        let grok = fs::read_to_string(user.join(".grok").join("config.toml")).unwrap();
        assert!(grok.contains("screen_mode"), "foreign key survives");
        assert!(!grok.contains("permission_mode"), "ours key gone");
        // 幂等（无变更）。
        assert!(retire_user_yolo_with(&user).unwrap().is_empty());

        // partial 值同样退役（ours 值集含两代）。
        apply_user_yolo_level_with(&user, YoloLevel::Partial).unwrap();
        let changed2 = retire_user_yolo_with(&user).unwrap();
        assert_eq!(changed2.len(), 4, "partial values retire too: {changed2:?}");

        std::env::remove_var("HST_USER_HOME");
        let _ = fs::remove_dir_all(&user);
    }

    #[test]
    fn yolo_full_to_partial_downgrade_removes_ours_mcp_key() {
        // codex 评审 F1（裁 a）：full 降 partial 时 ours 落的
        // enableAllProjectMcpServers 要摘（MCP 审批恢复确认），skip 保留、
        // defaultMode 换 acceptEdits；名单与用户自设键不动。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir();
        std::env::set_var("HST_USER_HOME", &user);

        apply_user_yolo_with(&user).expect("full");
        {
            // 模拟 --pre-trust 面同值键与用户自订名单并存。
            let p = user.join(".claude").join("settings.json");
            let mut v = read_json(&p).unwrap();
            v["enabledMcpjsonServers"] = json!(["mine"]);
            write_json(&p, &v).unwrap();
        }
        apply_user_yolo_level_with(&user, YoloLevel::Partial).expect("partial");
        let shared = read_json(&user.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("acceptEdits")
        );
        assert_eq!(
            shared["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true),
            "skip survives downgrade (inert under acceptEdits)"
        );
        assert!(
            shared.get("enableAllProjectMcpServers").is_none(),
            "ours enableAll removed on downgrade: {shared}"
        );
        assert_eq!(
            shared["enabledMcpjsonServers"][0].as_str(),
            Some("mine"),
            "name list may be user-curated, untouched"
        );

        std::env::remove_var("HST_USER_HOME");
        let _ = fs::remove_dir_all(&user);

        // 项目级同型：full 落 local 审批键，partial 降级摘除。
        let root = fresh_dir();
        apply_project_yolo_level(&root, YoloLevel::Full).expect("full");
        let full_local = read_json(&root.join(".claude").join("settings.local.json")).unwrap();
        assert_eq!(
            full_local["enableAllProjectMcpServers"].as_bool(),
            Some(true),
            "full project local writes mcp approvals"
        );
        apply_project_yolo_level(&root, YoloLevel::Partial).expect("partial");
        let local = read_json(&root.join(".claude").join("settings.local.json")).unwrap();
        assert_eq!(
            local["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert!(
            local.get("enableAllProjectMcpServers").is_none(),
            "ours enableAll removed on project downgrade: {local}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_yolo_partial_and_off_roundtrip() {
        let root = fresh_dir();
        let report = apply_project_yolo_level(&root, YoloLevel::Partial).expect("partial");
        assert_eq!(report.wrote.len(), 4);
        let shared = read_json(&root.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("acceptEdits")
        );
        let local = read_json(&root.join(".claude").join("settings.local.json")).unwrap();
        assert_eq!(
            local["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert!(
            local.get("enableAllProjectMcpServers").is_none(),
            "partial project local writes no MCP approvals"
        );
        let codex_text = fs::read_to_string(root.join(".codex").join("config.toml")).unwrap();
        assert!(codex_text.contains("workspace-write"));
        assert!(codex_text.contains("on-request"));
        assert!(
            codex_text.contains("trust_level"),
            "project trust seed is not leveled (trust is a separate face)"
        );
        let kimi = read_toml(&root.join(".kimi-code").join("config.toml")).unwrap();
        assert_eq!(
            kimi.get("default_permission_mode").and_then(|v| v.as_str()),
            Some("auto")
        );

        // off：ours 值（含 partial 值）摘除；codex 项目信任预种留给信任面。
        let changed = retire_project_yolo(&root).unwrap();
        assert_eq!(changed.len(), 4, "four project faces retired: {changed:?}");
        assert!(
            !root.join(".claude").join("settings.json").exists(),
            "ours-only claude project settings deleted"
        );
        assert!(
            !root.join(".claude").join("settings.local.json").exists(),
            "ours-only local deleted"
        );
        let codex = read_toml(&root.join(".codex").join("config.toml")).unwrap();
        assert!(codex.get("sandbox_mode").is_none());
        assert!(codex.get("approval_policy").is_none());
        assert!(
            codex.get("projects").is_some(),
            "trust seed survives off (pretrust face)"
        );
        assert!(
            !root.join(".kimi-code").join("config.toml").exists(),
            "ours-only kimi project config deleted"
        );
        // 幂等。
        assert!(retire_project_yolo(&root).unwrap().is_empty());

        let _ = fs::remove_dir_all(&root);
    }
}
