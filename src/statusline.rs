//! agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
//! - claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
//!   读改写，保留 env/permissions 等，只覆盖 statusLine 键）
//! - codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
//!   `status_line` 为内置项 ID 数组（Codex 无外部命令面，S016）
//! REQ-038：pwsh 脚本载体完全淘汰（用户令 2026-09-29「powershell 完全淘汰
//! 不用保留」，撤销 REQ-032 的弃用期保留一代）；渲染唯一载体 = 原生
//! `hst statusline --render`（ADR-0010），grok 的 thin `.cmd` 壳直调原生。
//! 用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
//! 包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；hst 段 = 当前
//! agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025）。

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::yolo::{read_toml, toml_write};

/// 状态栏脚本 HEAD：param、首尾强制 UTF-8（输出侧 CP936 控制台下 emoji 会被
/// 替换成字面 `??`，S024；输入侧重定向 stdin 默认按 OEM 码页解码，中文
/// cwd 会进来即花成「缁跨洘」形 GBK 误解码，D28 补钉）、stdin JSON 解析
/// （claude code 供给 model 等；codex 无 stdin 数据时退化）、Seg 与 FmtTok、
/// FmtDur 助手、`$parts` 收集器。
/// 默认第一行「身份与项目」（REQ-051 四行结构）：agent 类型加版本（首位，
/// 用户令）/ dir / git 分支 / clock（bash 与运行时环境版本段默认退役，段
/// id 保留可显式写回）。D18 定制面 `segments` 键缺省回落此序。
pub(crate) const DEFAULT_SEGMENTS: &[&str] = &["agent", "dir", "git", "clock"];

/// 默认第二行「运行时」（REQ-051）：模型 / context（四格条形加百分比加
/// token 绝对值，`▰▰▱▱ 42% (82k/195k)` 形）/ 耗时 / fleet 舰队计数（纯读
/// 快照零子进程）。agent 态段随 REQ-049 F-3 与 REQ-051 退役。
/// `segments2` 键缺省回落此序。
pub(crate) const DEFAULT_SEGMENTS2: &[&str] = &["model", "context", "duration", "fleet"];

/// 默认第三行 = hookstate 专属行（REQ-026 起，用户令 2026-09-27 行序
/// 裁定「应该在排在第3行」，自第五行前移）：hook 通道别名清单独占一行
///（原生渲染独占段，pwsh 弃用期不出）；双缺整行隐藏。`segments3` 键
/// 缺省回落此序。
pub(crate) const DEFAULT_SEGMENTS3: &[&str] = &["hookstate"];

/// 默认第四行 = task 专属行（REQ-045 起 goal 自治面,REQ-051 task 正名
/// 独占一行,goalmode 退化前缀旗;
/// loop 段随 loop 面退役出仓；REQ-024 至 REQ-042 时点口径为 loop 行）：
/// 本项目 goal 文本加 Stop 腿轮次；无 goal 时整行剔除（零噪声不变）。
/// tools / mcp / tokens 仍可显式写 `segments4` 换位。`segments4` 键缺省
/// 回落此序。
pub(crate) const DEFAULT_SEGMENTS4: &[&str] = &["goal"];

/// 默认第五行 = goalmode 专属行（REQ-025；2026-09-27 行序调整自第四行
/// 顺移）：`/goal` Goal Mode 条件文本加在役态（active/paused，会话
/// transcript 尾探）；无 goal 会话整行隐藏（零噪声）。`segments5` 键缺
/// 省回落此序。
pub(crate) const DEFAULT_SEGMENTS5: &[&str] = &[];

/// 内嵌默认模板（D18）。键 = 段 id；`context-ascii` 是 grok 的结构差异项
/// （nerd 版带 used/window 括号对，ascii 版只有百分比加 ctx 后缀）。
/// 各段可用占位符见 `hst statusline --example`；git 段默认前导空格在 branch / flags 变量里。
/// 原生渲染引擎取默认模板（单一权威表）。
pub fn default_template(key: &str) -> String {
    DEFAULT_TEMPLATES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    ("shell", "{icon}{name}"),
    ("dir", "{icon}{path}"),
    ("hst", "{icon}{agent}:{state}"),
    ("agent", "{icon}{agent}"),
    ("fleet", "{icon}{fleet}"),
    ("model", "{icon}{model}"),
    ("context", "{bar} {pct}% ({used}/{window})"),
    ("context-ascii", "{pct}% ({used}/{window})"),
    ("tools", "{icon}{count}"),
    ("tools-ascii", "{count}"),
    ("mcp", "{icon}{count}"),
    ("mcp-ascii", "{count}"),
    ("tokens", "{icon}{used}/{window}"),
    ("tokens-ascii", "{used}/{window}"),
    ("duration", "{icon}{duration}"),
    ("goal", "{icon} {mode}{goal} R{count} · {age}"),
    ("goalmode", "{icon}  {text}"),
    ("goal-ascii", "{mode}{goal} R{count} · {age}"),
    ("goalmode-ascii", "{text}"),
    ("hookstate", "{icon}  {alias}"),
    ("hookstate-ascii", "{alias}"),
    ("git", "{icon}{branch}{flags}"),
    ("clock", "{icon}{datetime}"),
    ("package", "{icon}{version}"),
    ("python", "{icon}{version}"),
    ("rust", "{icon}{version}"),
    ("node", "{icon}{version}"),
    ("ts", "{icon}{version}"),
    ("zig", "{icon}{version}"),
    ("go", "{icon}{version}"),
    ("cpp", "{icon}{version}"),
];

/// 内嵌默认图标（码位与拆段前脚本逐字对齐；hst 机器人宽字形跟两空格，
/// D45 前键名 oma）。
/// Grok ASCII 路径图标恒空串（M046）。
/// 原生渲染引擎取默认图标（单一权威表）。
pub fn default_icon(key: &str) -> String {
    DEFAULT_ICONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

const DEFAULT_ICONS: &[(&str, &str)] = &[
    ("shell-pwsh", "\u{ebc7} "),
    ("dir", "\u{f07b} "),
    ("git", "\u{e0a0} "),
    ("shell", "\u{ea85} "),
    ("hst", "\u{f06a9}  "),
    ("agent", "\u{2733} "),
    ("fleet", "\u{1f411} "),
    ("model", "\u{2726} "),
    ("context", "\u{f035b} "),
    ("tools", "\u{f0ad} "),
    ("mcp", "\u{f233} "),
    ("tokens", "\u{f080} "),
    ("duration", "\u{f0150} "),
    ("goal", "\u{f140} "),
    ("goalmode", "\u{f140}"),
    ("hookstate", "\u{f0f1}"),
    ("package", "\u{f03d7} "),
    ("python", "\u{f0320} "),
    ("rust", "\u{f1617} "),
    ("node", "\u{f0399} "),
    ("ts", "\u{f06e6} "),
    ("zig", "\u{e6a9} "),
    ("go", "\u{e627} "),
    ("cpp", "\u{e646} "),
    ("clock", "\u{f0954} "),
];

/// `~/.hst/statusline.toml` 用户级定制（D18）。键级缺省回落内嵌默认：
/// 没写的键用默认，写下的键生效；坏文件硬错退出 1。
#[derive(Debug, Default, PartialEq)]
pub struct StatuslineConfig {
    /// `segments`：第一排段 id 数组即全量序（显隐加顺序）；键缺省回落
    /// `DEFAULT_SEGMENTS`。
    pub segments: Option<Vec<String>>,
    /// `segments2`（D40）：第二行（agent 状态）段 id 数组；键缺省回落
    /// `DEFAULT_SEGMENTS2`，空数组 = 不出第二行。
    pub segments2: Option<Vec<String>>,
    /// `segments3`（D42）：第三行（loop/goal 专属行，REQ-024）段 id 数组；
    /// 键缺省回落 `DEFAULT_SEGMENTS3`，空数组 = 不出第三行。
    pub segments3: Option<Vec<String>>,
    /// `segments4`（REQ-025）：第四行（goalmode 专属行）段 id 数组；键
    /// 缺省回落 `DEFAULT_SEGMENTS4`，空数组 = 不出第四行。
    pub segments4: Option<Vec<String>>,
    /// `segments5`（REQ-026 起，2026-09-27 行序调整后为 goalmode 专属
    /// 行）段 id 数组；键缺省回落 `DEFAULT_SEGMENTS5`，空数组 = 不出第
    /// 五行。
    pub segments5: Option<Vec<String>>,
    /// `single_line`（D40）：退单排开关（两排段并一行；默认 false 双排）。
    /// kimi / grok 的多行渲染未实证时的逃生门。
    pub single_line: bool,
    /// `[template]`：段格式串（键 = 段 id；`<段>-ascii` 为 grok 结构差异项）；
    /// 键级回落 `DEFAULT_TEMPLATES`。
    pub template: Vec<(String, String)>,
    /// `[icons]`：图标映射（含 `ts`（node 段内部子部件，非独立段 id）、
    /// `shell-pwsh` 子项键）；键级回落
    /// `DEFAULT_ICONS`。Grok ASCII 路径图标恒空（M046）。
    pub icons: Vec<(String, String)>,
    /// `[codex] items`：codex 内置项 ID 子集（原样透传，未知 id codex 侧
    /// 静默跳过）；键缺省回落 `CODEX_STATUS_LINE_ITEMS`。
    pub codex_items: Option<Vec<String>>,
}

pub(crate) fn config_path(home: &Path) -> PathBuf {
    home.join("statusline.toml")
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 读用户定制配置；文件不存在回落全默认（不是错误）。
pub fn read_config(home: &Path) -> Result<StatuslineConfig, String> {
    read_config_pub_alias(home)
}

/// 原生渲染引擎（ADR-0010）复用口：等价 read_config。
pub fn read_config_pub(home: &Path) -> Result<StatuslineConfig, String> {
    read_config_pub_alias(home)
}

fn read_config_pub_alias(home: &Path) -> Result<StatuslineConfig, String> {
    let p = config_path(home);
    if !p.exists() {
        return Ok(StatuslineConfig::default());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse_config(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// 状态栏的纯函数面（细则见模块文档与集成测试）。
fn parse_config(text: &str) -> Result<StatuslineConfig, String> {
    let v: toml::Value = toml::from_str(text).map_err(|e| format!("parse: {e}"))?;
    let mut cfg = StatuslineConfig::default();
    if let Some(segs) = v.get("segments") {
        let arr = segs.as_array().ok_or("segments 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments 元素必须是字符串")?.to_string());
        }
        cfg.segments = Some(out);
    }
    if let Some(segs) = v.get("segments2") {
        let arr = segs.as_array().ok_or("segments2 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments2 元素必须是字符串")?.to_string());
        }
        cfg.segments2 = Some(out);
    }
    if let Some(segs) = v.get("segments3") {
        let arr = segs.as_array().ok_or("segments3 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments3 元素必须是字符串")?.to_string());
        }
        cfg.segments3 = Some(out);
    }
    if let Some(segs) = v.get("segments4") {
        let arr = segs.as_array().ok_or("segments4 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments4 元素必须是字符串")?.to_string());
        }
        cfg.segments4 = Some(out);
    }
    if let Some(segs) = v.get("segments5") {
        let arr = segs.as_array().ok_or("segments5 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments5 元素必须是字符串")?.to_string());
        }
        cfg.segments5 = Some(out);
    }
    if let Some(sl) = v.get("single_line") {
        cfg.single_line = sl.as_bool().ok_or("single_line 必须是布尔值")?;
    }
    for (key, slot) in [("template", &mut cfg.template), ("icons", &mut cfg.icons)] {
        if let Some(t) = v.get(key) {
            let t = t.as_table().ok_or_else(|| format!("{key} 必须是表"))?;
            for (k, val) in t {
                let s = val
                    .as_str()
                    .ok_or_else(|| format!("{key}.{k} 必须是字符串"))?;
                slot.push((k.clone(), s.to_string()));
            }
        }
    }
    if let Some(items) = v.get("codex").and_then(|c| c.get("items")) {
        let arr = items.as_array().ok_or("[codex] items 必须是字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(
                s.as_str()
                    .ok_or("[codex] items 元素必须是字符串")?
                    .to_string(),
            );
        }
        cfg.codex_items = Some(out);
    }
    Ok(cfg)
}

/// 段序生效值（D42 三行、D44 曾默认两行、REQ-024 起默认三行、REQ-045 起
/// 第四行 goal 自治专属，无任务时行空即剔除）：各行取用户清单或默认序（未知与
/// 跨行重复 id 由拼装器拒，空行由拼装器剔除）。**老配置原样升级语义**
/// （codex D42 评审 F1/H1 两轮收口）：用户显式写过 `segments` 而未写任何
/// 后续行键时，未写的行**不补默认**（D18 老单排与 v1.1.0 双排两种老形态
/// 原样升级，行为零漂移；新装用户键全缺省得两行默认）；用户写过任一后续
/// 行键（segments2 或 segments3）时，未写的行**补默认并去重**（显式段
/// id 从默认行剔除，防跨行重复硬错）。显式写的行不去重（跨行重复仍由拼
/// 装器拒）。
/// 原生渲染引擎（ADR-0010）复用口：等价 effective_orders。
pub fn effective_orders_pub(cfg: &StatuslineConfig) -> Result<Vec<Vec<&str>>, String> {
    effective_orders(cfg)
}

fn effective_orders(cfg: &StatuslineConfig) -> Result<Vec<Vec<&str>>, String> {
    let rows = [
        (&cfg.segments, DEFAULT_SEGMENTS),
        (&cfg.segments2, DEFAULT_SEGMENTS2),
        (&cfg.segments3, DEFAULT_SEGMENTS3),
        (&cfg.segments4, DEFAULT_SEGMENTS4),
        (&cfg.segments5, DEFAULT_SEGMENTS5),
    ];
    let any_later_row_written = cfg.segments2.is_some()
        || cfg.segments3.is_some()
        || cfg.segments4.is_some()
        || cfg.segments5.is_some();
    // 老配置形态 = 用户写过 segments 且没写任何后续行键 → 后续缺省行不补
    // （原样单行升级）。新装（segments 也缺省）与写过后续键的配置照常补
    // 默认（去重）。
    let legacy_single_row =
        cfg.segments.as_ref().is_some_and(|v| !v.is_empty()) && !any_later_row_written;
    // 用户显式写过的段 id 全集：补默认的行剔除这些 id。
    let user_ids: std::collections::HashSet<&str> = rows
        .iter()
        .filter_map(|(user, _)| user.as_ref())
        .flat_map(|segs| segs.iter().map(String::as_str))
        .collect();
    let orders: Vec<Vec<&str>> = rows
        .iter()
        .enumerate()
        .map(|(i, (user, default))| match user {
            Some(segs) => segs.iter().map(String::as_str).collect(),
            None if i > 0 && legacy_single_row => Vec::new(),
            None => default
                .iter()
                .copied()
                .filter(|id| !user_ids.contains(id))
                .collect(),
        })
        .filter(|row| !row.is_empty())
        .collect();
    // 全量评审 G1：跨行重复 id 拒（同 id 双显且 git 类双 spawn；单行内
    // 重复由渲染期拼装与 UNKNOWN 校验面共同拦）。
    let mut seen = std::collections::HashSet::new();
    for id in orders.iter().flat_map(|r| r.iter()) {
        if !seen.insert(*id) {
            return Err(format!("duplicate statusline segment across rows: {id}"));
        }
    }
    Ok(orders)
}

/// 部署方 hst 二进制绝对路径（ADR-0010 statusline 命令锚）：current_exe
/// 失败回落裸 `hst`（PATH 形）。
pub(crate) fn hst_bin_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hst".to_string())
}

pub(crate) fn grok_cmd_path(home: &Path) -> PathBuf {
    home.join("statusline").join("hst-statusline-grok.cmd")
}

/// REQ-038：PS1 载体完全淘汰的退役清扫（幂等）。摘除弃用期保留的
/// `hst-statusline.ps1` 与自备脚本标记 `.custom`（grok thin `.cmd` 壳调
/// 原生渲染，保留）；返回被摘除的文件名清单（kv 面回显，空 = 无残余）。
pub fn cleanup_legacy_script(home: &Path) -> Vec<String> {
    let mut removed = Vec::new();
    for name in ["hst-statusline.ps1", "hst-statusline.ps1.custom"] {
        let p = home.join("statusline").join(name);
        if p.is_file() && std::fs::remove_file(&p).is_ok() {
            removed.push(name.to_string());
        }
    }
    removed
}

/// Windows Grok `[ui.status_line].command` must be a single spawnable path.
/// grok-build `command.rs` does `Command::new(entire_string)` first and only
/// falls back to a shell on `NotFound` (or Unix ENOEXEC). A `pwsh -File "..."`
/// line contains quotes and slashes, so Windows returns ERROR_INVALID_NAME
/// 123 and paints `[status line: could not start the script: ...]` (M048).
/// REQ-032：grok Windows 状态栏壳改 thin cmd 透传（烘焙部署方 exe 绝对
/// 路径，stdin 继承；pwsh 退役）。
fn statusline_grok_cmd() -> String {
    format!(
        "@echo off\r\nrem generated by hst init (REQ-032 thin)\r\n\"{}\" statusline --render grok\r\n",
        hst_bin_path()
    )
}

fn grok_command_line(cmd: &str) -> String {
    #[cfg(windows)]
    {
        // M048：单路径形（正斜杠归一）。cmd 由 merge_grok 从其 home 形参
        // 同源生成（评审 F1：不自取 hst_home，防形参脱钩）。
        cmd.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        // ADR-0010：grok 同指原生渲染（部署方绝对路径）。
        let _ = cmd;
        format!("\"{}\" statusline --render grok", hst_bin_path())
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的claude面（细则见模块文档与集成测试）。
/// D53（codex F1）：user_home 显式透传（测试密闭，不读 env）。
pub fn merge_claude(_home: &Path, user_home: &Path) -> Result<String, String> {
    let settings = user_home.join(".claude").join("settings.json");
    let mut v: serde_json::Value = if settings.exists() {
        let text = std::fs::read_to_string(&settings)
            .map_err(|e| format!("{}: {e}", settings.display()))?;
        // D52（codex F1）：BOM 容忍（与 yolo::read_json 同类，舰队 PS 脚本）。
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{}: corrupt: {e}", settings.display()))?
    } else {
        json!({})
    };
    // ADR-0010：原生渲染（hst 二进制 stdin/stdout）；REQ-038：PS1 载体
    // 完全淘汰。命令用部署方 hst 绝对路径（PATH 上旧版 hst 不带 --render
    // 面；stable v2.9.1 起全体带面后可回 hook 先例的裸 PATH 形）。
    let cmd = format!("\"{}\" statusline --render claude", hst_bin_path());
    v["statusLine"] = json!({ "type": "command", "command": cmd });
    let body = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n";
    // D53（codex F2）：内容判等幂等（init 重跑不搅 mtime）。
    if std::fs::read_to_string(&settings).ok().as_deref() != Some(body.as_str()) {
        std::fs::write(&settings, body).map_err(|e| format!("{}: {e}", settings.display()))?;
    }
    Ok(settings.display().to_string())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的kimi面（细则见模块文档与集成测试）。
/// cmd/sh 执行，首行接管 footer；300ms 超时由 kimi 侧约束，超时自动回退
/// 内置布局——S025）。其它表保留。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_kimi(_home: &Path, user_home: &Path) -> Result<String, String> {
    let config = user_home.join(".kimi-code").join("tui.toml");
    let mut toml = read_toml(&config)?;
    if apply_kimi_status_line(&mut toml)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[status_line].command` 幂等落位；返回是否变更（可测纯函数）。
fn apply_kimi_status_line(toml: &mut toml::Value) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi tui.toml is not a table".into()),
    };
    let status_line = table
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("kimi [status_line] is not a table".into()),
    };
    // ADR-0010：kimi 同指原生渲染（部署方绝对路径）。
    let command = format!("\"{}\" statusline --render kimi", hst_bin_path());
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str());
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
    }
    Ok(changed)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的grok面（细则见模块文档与集成测试）。
/// Windows 写 `.cmd` 单路径（M048）；Unix 仍写 `pwsh -File` 命令行（NotFound
/// 才回落 sh -c）。其它表保留。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_grok(home: &Path, user_home: &Path) -> Result<String, String> {
    // REQ-038：PS1 淘汰后 grok 的 thin `.cmd` 壳（直调原生渲染）是唯一
    // 状态栏落盘件，由本面幂等落位（内容判等，M048 单路径约束）。
    let cmd = grok_cmd_path(home);
    if let Some(dir) = cmd.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if std::fs::read_to_string(&cmd).ok().as_deref() != Some(statusline_grok_cmd().as_str()) {
        std::fs::write(&cmd, statusline_grok_cmd())
            .map_err(|e| format!("{}: {e}", cmd.display()))?;
    }
    let config = user_home.join(".grok").join("config.toml");
    let cmd_str = cmd.display().to_string();
    let mut toml = read_toml(&config)?;
    if apply_grok_status_line(&mut toml, &cmd_str)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[ui.status_line]` 幂等落位（type=command + command 串）；返回是否变更
/// （可测纯函数）。
fn apply_grok_status_line(toml: &mut toml::Value, cmd: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("grok config.toml is not a table".into()),
    };
    let ui = table
        .entry("ui".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let ui = match ui {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui] is not a table".into()),
    };
    let status_line = ui
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui.status_line] is not a table".into()),
    };
    let command = grok_command_line(cmd);
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str())
        || sl.get("type").and_then(|v| v.as_str()) != Some("command");
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
        sl.insert("type".into(), toml::Value::String("command".into()));
    }
    Ok(changed)
}

/// `hst statusline --example` 打印的带注释全量示例（存到
/// `~/.hst/statusline.toml` 生效）。
pub const EXAMPLE_TOML: &str = r#"# ~/.hst/statusline.toml —— 状态栏用户级定制（D18）
# 原生渲染每帧直读本文件（改完即生效，无需重跑部署）。
# 键级缺省回落：没写的键用内嵌默认；坏文件硬错退出 1。

# 段落清单（四行结构 REQ-051：L1 身份与项目、L2 运行时、L3 hook、L4 task）：
# segments = L1 身份与项目行（agent 类型加版本 / 目录 / git 分支 / 时钟；
# bash 与运行时环境版本段默认退役，段 id 保留可显式写回）、
# segments2 = L2 运行时行（模型 / context 条形加百分比加 token 绝对值 /
# 耗时 / fleet 舰队计数，fleet 纯读 ~/.hst/state/fleet-snap.json 零子进程）、
# segments3 = L3 hook 专属行（注册面 hook 功能别名清单，清单内部 `|` 细分，
# 双缺整行隐藏）、
# segments4 = L4 task 专属行（hst task 自治面：文本加 R 轮次加循环时间
# （距上轮 age 的 d/h/m 形），goalmode 退化为行内前缀旗（active 加
# paused，在役才显），无 task 整行隐藏）、
# segments5 = 第五行保留（默认空；显式写 goalmode 等段可启五行走旧形）、
# 段 id 数组即全量（显隐加顺序）；tools / mcp / tokens 三段仍可显式写入
# 后续行与专属段同线换位，如：
#   segments4 = ["goal", "tools", "mcp"]
# task 经 `hst task set "task 文本"` 设置、`hst task show` 查看、
# `hst task clear` 清空（REQ-045 起 goal 正名 task，自治面）。
# 例（隐藏时钟段、git 提到目录前）：
#   segments = ["agent", "dir", "git"]
#   segments2 = ["model", "context", "duration", "fleet"]
#   segments3 = []
# 退单行（kimi 运行时自动恒单行紧凑形；grok 1.0.50 起多行；显式退单行用）：
#   single_line = true
segments = ["agent", "dir", "git", "clock"]
segments2 = ["model", "context", "duration", "fleet"]
segments3 = ["hookstate"]
segments4 = ["goal"]
segments5 = []

# 段内模板（[template]）：每段一条格式串；`<段>-ascii` 是 ASCII 形
#（缺省同用 nerd 模板、图标恒空）。可用占位符：
#   agent {icon}{agent}（L1 身份段：agent 名或 agent-版本归一形）
#   fleet {icon}{fleet}（L2 舰队段：在线 agent 计数 N，总台第六笔回执收敛形；
#   龄超 5 分钟灰显带龄、超 30 分钟隐藏）
#   shell {icon}{name} / dir {icon}{path} / hst {icon}{agent}{state}
#   goalmode {icon}  {text}（/goal 原始参数文本；可自配 {state}）
#   model {icon}{model} / context {icon}{bar}{pct}{used}{window}（bar = 四格
#   条形，窄窗自动去括号对）
#   tools {icon}{count} / mcp {icon}{count} / tokens {icon}{used}{window}
#   duration {icon}{duration} / git {icon}{branch}{flags}
#   goal {icon} {mode}{goal} R{count} · {age}（L4 task 行；{mode} = goalmode
#   前缀旗（active 加 paused，在役才显）、{goal} 截 60、{count} = Stop 腿
#   轮次、{age} = 距上轮循环时间 d/h/m 形）
#   hookstate {icon}  {alias}（L3 hook 行；注册面全部 hook 的功能别名清单，
#   英文 id 直用（hst-token 映射 hst-token-guard），未收录 hook 回落 stem
#   本名，全局插件 hook 的 statusMessage 含中文时回落纯插件名（REQ-051
#   hook 零中文），清单空回落泛称 hook；窄窗截首三个至首一个）
#   package 与六工具链段（python 加 rust 加 node 加 zig 加 go 加 cpp）{icon}{version}
#   （ts 不是独立段 id：它是 node 段内部第二部件，[icons] 的 ts 键只管该部件字形，写进 segments 会响亮报错）
#   clock {icon}{datetime}（D51：年月日加当前时间，分钟精度）
# 例（hst 段去图标改方括号态）：
# [template]
# hst = "{agent}[{state}]"

# 图标映射（[icons]）：键级回落；hst 机器人宽字形默认跟两空格。
# 可用键：shell / shell-pwsh / dir / git / hst / agent（REQ-051 L1 身份段）
#         / fleet（REQ-051 L2 舰队段）
#         / model / context / tools / mcp / tokens
#         / duration / goal（REQ-019 加，REQ-045 起自有存储源）
#         / goalmode（REQ-025 加）
#         / package / python / rust / node / ts / zig / go / cpp
#         / clock（D51 加）
# 例：
# [icons]
# rust = "R "

# codex 内置项子集（[codex] items）：替换写入 ~/.codex/config.toml 的
# [tui].status_line 内置项 ID 清单；未知 id codex 侧静默跳过（S016）。
# codex 只有内置项面（无外部命令 statusline）；REQ-051 换序对齐 L1 至 L2
# 语义（版本到目录到分支到模型到 context 三键），run-state 出默认集；
# fleet 与 hook 与 task 不上 codex（无对应内置项，不硬造）。
# 例（只要目录与分支）：
# [codex]
# items = ["current-dir", "git-branch"]
"#;

/// Codex `[tui].status_line` is an ordered list of built-in item IDs
/// (ohmypwsh S016, openai/codex 0.148+). Unknown strings are silently
/// skipped, so a command argv (`"command", "pwsh", "-File", ...`) empties
/// the bar. hst cannot inject a custom script here.
/// codex 内置项默认集（D40 对齐增强：codex 只有内置项面，无外部命令
/// statusline（openai/codex#17827/#20244 未实现），可达上限就是富内置项
/// 清单——源码 status_line_setup.rs 全量约 30 项）。token 细分（used/
/// input/output/window）是 codex 侧对 D40「token 用量」要素的承载；
/// tools 计数、MCP 计数、context 构成三要素 codex 能力面不可达（差距
/// 说明见 S034 追记）。D46 加 `codex-version`（源码实证
/// `StatusLineItem::CodexVersion`，strum kebab_case ID），codex 侧版本
/// 显示走内置项，与 pwsh 面版本并入 agent 名同要素；同轮用户裁去
/// `context-remaining`（渲染成 left 百分比，与 `context-used` 的
/// `Context N% used` 重复占宽，只留 used 形，缺省集回十二项）。
/// 2026-09-28 用户令（目标形 `Context 35% · 996K window · used 373K`）：
/// 去 `total-input-tokens` 与 `total-output-tokens`（in 与 out 两项占宽），
/// 并把 `context-window-size` 提到 `used-tokens` 前，读序固定为百分比到
/// 窗口到用量（缺省集十项）。注：codex 内置项文案是固定形（`Context N%
/// used` 与 `N used`），故实际渲染比用户给的字面多一处 used 词，项选择面
/// 不可再收（hst 只能选 id，不能改 codex 的渲染）。
const CODEX_STATUS_LINE_ITEMS: &[&str] = &[
    "codex-version",
    "current-dir",
    "git-branch",
    "model-with-reasoning",
    "context-used",
    "context-window-size",
    "used-tokens",
];

fn render_codex_tui_section(items: &[&str]) -> String {
    let mut lines = vec!["[tui]".to_string(), "status_line = [".to_string()];
    let last = items.len().saturating_sub(1);
    for (i, id) in items.iter().enumerate() {
        let comma = if i == last { "" } else { "," };
        lines.push(format!("  \"{id}\"{comma}"));
    }
    lines.push("]".into());
    lines.push("status_line_use_colors = true".into());
    lines.join("\n")
}

/// 表头归一名（全量评审 F1）：剥 BOM 加括号内空白加行尾注释后取表名；
/// 非表头行归 None。`[tui]` 加 `[ tui ]` 加 `[tui] # 注释` 加 BOM 首行
/// 皆归 "tui"；`[tui.x]` 子表归 "tui.x"（不误剥）。
fn table_header_name(ln: &str) -> Option<String> {
    let s = ln.trim_start_matches('\u{feff}').trim();
    let s = s.split('#').next().unwrap_or("").trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim().to_string())
}

/// 键名归一（全量评审 F2）：`=` 前段 trim；非键行归 None。
fn key_name(ln: &str) -> Option<String> {
    let (k, _) = ln.split_once('=')?;
    Some(k.trim().to_string())
}

/// 重写 `[tui]` 表（全量评审 F1 加 F2）：表头按归一名判（不再精确串等，
/// 防非规范形旧表未剥重复表写坏 config 且重跑不自愈）；只替换
/// status_line 加 status_line_use_colors 两键，兄弟键原样保留并入新表
///（存量重复 `[tui]` 表顺带合并治伤）。
fn rewrite_tui_section(text: &str, items: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut siblings: Vec<String> = Vec::new();
    let mut in_tui = false;
    let mut skipping_target_value = false;
    let mut skip_depth: i32 = 0;
    for ln in text.lines() {
        if let Some(name) = table_header_name(ln) {
            in_tui = name == "tui";
            if in_tui {
                continue;
            }
        }
        if in_tui {
            if skipping_target_value {
                skip_depth += ln.matches('[').count() as i32;
                skip_depth -= ln.matches(']').count() as i32;
                if skip_depth <= 0 {
                    skipping_target_value = false;
                }
                continue;
            }
            if matches!(
                key_name(ln).as_deref(),
                Some("status_line") | Some("status_line_use_colors")
            ) {
                skip_depth = ln.matches('[').count() as i32 - ln.matches(']').count() as i32;
                if skip_depth > 0 {
                    skipping_target_value = true;
                }
                continue;
            }
            siblings.push(ln.to_string());
            continue;
        }
        out.push(ln.to_string());
    }
    let mut section = render_codex_tui_section(items);
    for s in &siblings {
        if !s.trim().is_empty() {
            section.push('\n');
            section.push_str(s);
        }
    }
    let kept = out.join("\n").trim_end().to_string();
    if kept.is_empty() {
        format!("{section}\n")
    } else {
        format!("{kept}\n\n{section}\n")
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Codex: replace the `[tui]` table with built-in item IDs (ohmypwsh S016).
/// Does not deploy the pwsh script; Codex has no command-backed status line.
/// `[codex] items`（D18）用户清单原样透传：codex 对未知 id 静默跳过，hst
/// 不校验清单合法性；键缺省回落内嵌推荐八项。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_codex(home: &Path, user_home: &Path) -> Result<String, String> {
    let cfg = read_config(home)?;
    let items: Vec<&str> = match &cfg.codex_items {
        Some(list) => list.iter().map(String::as_str).collect(),
        None => CODEX_STATUS_LINE_ITEMS.to_vec(),
    };
    let config = user_home.join(".codex").join("config.toml");
    let existing = if config.exists() {
        std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?
    } else {
        String::new()
    };
    let body = rewrite_tui_section(&existing, &items);
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // D53（codex F2）：内容判等幂等（init 重跑不搅 mtime）。
    if std::fs::read_to_string(&config).ok().as_deref() != Some(body.as_str()) {
        std::fs::write(&config, body).map_err(|e| format!("{}: {e}", config.display()))?;
    }
    Ok(config.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独占临时目录（单测内 fs 落盘判据用；用完即删）。
    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hst-sl-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_config_reads_segments_and_falls_back_on_missing_key() {
        // 期望值来自 D18 裁定：segments 写下即全量序，键缺省回落默认。
        assert_eq!(parse_config("").unwrap().segments, None);
        assert_eq!(
            parse_config("segments = [\"hst\", \"git\"]\n")
                .unwrap()
                .segments,
            Some(vec!["hst".to_string(), "git".to_string()])
        );
        // segments = [] 是显式空清单（空栏），不回落默认。
        assert_eq!(
            parse_config("segments = []\n").unwrap().segments,
            Some(vec![])
        );
    }

    #[test]
    fn dies_parse_config_rejects_malformed() {
        assert!(parse_config("segments = ").is_err(), "truncated toml");
        assert!(
            parse_config("segments = \"git\"\n").is_err(),
            "not an array"
        );
        assert!(parse_config("segments = [1]\n").is_err(), "not strings");
    }

    #[test]
    fn parse_config_reads_template_and_icons_tables() {
        let cfg = parse_config("[template]\nhst = '[{state}] {agent}'\n\n[icons]\nhst = '>'\n\n")
            .unwrap();
        assert_eq!(
            cfg.template,
            vec![("hst".to_string(), "[{state}] {agent}".to_string())]
        );
        assert_eq!(cfg.icons, vec![("hst".to_string(), ">".to_string())]);
    }

    #[test]
    fn dies_parse_config_rejects_non_string_template_or_icon_value() {
        assert!(parse_config("[template]\nhst = 1\n").is_err());
        assert!(parse_config("[icons]\nhst = true\n").is_err());
        assert!(parse_config("template = \"x\"\n").is_err(), "not a table");
    }

    #[test]
    fn codex_items_config_overrides_builtin_list() {
        // 期望值：用户清单原样透传（含未知 id——codex 侧静默跳过，hst 不拦）。
        let cfg =
            parse_config("[codex]\nitems = [\"current-dir\", \"git-branch\", \"nope\"]\n").unwrap();
        assert_eq!(
            cfg.codex_items,
            Some(vec![
                "current-dir".to_string(),
                "git-branch".to_string(),
                "nope".to_string()
            ])
        );
        let tui = render_codex_tui_section(&["current-dir", "git-branch", "nope"]);
        assert!(tui.contains("\"current-dir\""));
        assert!(tui.contains("\"nope\""));
        assert!(
            !tui.contains("run-state"),
            "default list replaced, not merged"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("nope"))
            .unwrap();
        assert!(!last.trim().ends_with(','), "no trailing comma: {last}");
        // 键缺省：无 [codex] items 时回落内嵌推荐八项。
        assert!(parse_config("").unwrap().codex_items.is_none());
    }

    #[test]
    fn dies_parse_config_rejects_malformed_codex_items() {
        assert!(parse_config("[codex]\nitems = \"x\"\n").is_err());
        assert!(parse_config("[codex]\nitems = [1]\n").is_err());
    }

    #[test]
    fn grok_windows_command_is_bare_cmd_path() {
        // Oracle: grok-build `Command::new(entire_string)`; a shell line with
        // quotes is ERROR_INVALID_NAME 123, which is not NotFound, so the
        // shell fallback never runs (M048)。评审 F1：cmd 路径由调用方同源
        // 传入，windows 臂原样回传（正斜杠归一）。
        let cmd = grok_command_line("C:\\hst\\statusline\\hst-statusline-grok.cmd");
        #[cfg(windows)]
        {
            assert_eq!(cmd, "C:/hst/statusline/hst-statusline-grok.cmd");
            assert!(
                !cmd.contains('"'),
                "quotes in the program name are 123: {cmd}"
            );
            assert!(
                !cmd.contains("pwsh"),
                "args after the path are not passed: {cmd}"
            );
        }
        #[cfg(not(windows))]
        {
            // ADR-0010：grok 指原生渲染（裸 PATH 形）。
            assert!(cmd.ends_with("statusline --render grok"), "{cmd}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn dies_windows_pwsh_shell_line_is_invalid_filename() {
        // Independent oracle: Win32 ERROR_INVALID_NAME = 123. grok-build
        // command.rs only shells out on NotFound, so this error is painted.
        let cmd = r#"pwsh -NoProfile -File "C:/Users/ray/.ohmyagents/statusline/hst-statusline.ps1" grok"#;
        let err = std::process::Command::new(cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect_err("shell line must not be a valid program name");
        assert_eq!(err.raw_os_error(), Some(123), "{err}");
        assert_ne!(err.kind(), std::io::ErrorKind::NotFound, "{err:?}");
    }

    #[test]
    fn kimi_and_grok_merges_are_idempotent_and_keep_other_tables() {
        // 期望来自 kimi/grok 官方 schema（S025）：kimi [status_line].command、
        // grok [ui.status_line] type=command；其它表必须存活。
        let mut kimi: toml::Value =
            toml::from_str("theme = \"dark\"\n[status_line]\nitems = [\"model\"]\n").unwrap();
        assert!(apply_kimi_status_line(&mut kimi).unwrap());
        assert!(!apply_kimi_status_line(&mut kimi).unwrap());
        let kimi_t = kimi.as_table().unwrap();
        assert_eq!(kimi_t.get("theme").unwrap().as_str(), Some("dark"));
        let sl = kimi_t.get("status_line").unwrap().as_table().unwrap();
        // ADR-0010：kimi 指原生渲染。
        let kc = sl.get("command").and_then(|v| v.as_str()).unwrap_or("");
        assert!(kc.ends_with("statusline --render kimi"), "{kc}");
        assert_eq!(
            sl.get("items").unwrap().as_array().unwrap().len(),
            1,
            "foreign [status_line] keys survive"
        );

        let mut grok: toml::Value =
            toml::from_str("model = \"x\"\n[ui]\npermission_mode = \"always-approve\"\n").unwrap();
        assert!(apply_grok_status_line(&mut grok, "C:/x/hst-statusline-grok.cmd").unwrap());
        assert!(!apply_grok_status_line(&mut grok, "C:/x/hst-statusline-grok.cmd").unwrap());
        let grok_t = grok.as_table().unwrap();
        assert_eq!(grok_t.get("model").unwrap().as_str(), Some("x"));
        let ui = grok_t.get("ui").unwrap().as_table().unwrap();
        assert_eq!(
            ui.get("permission_mode").unwrap().as_str(),
            Some("always-approve"),
            "yolo key in [ui] survives"
        );
        let sl = ui.get("status_line").unwrap().as_table().unwrap();
        assert_eq!(sl.get("type").unwrap().as_str(), Some("command"));
        let grok_cmd = sl.get("command").unwrap().as_str().unwrap();
        assert_eq!(
            grok_cmd,
            grok_command_line(
                &grok_cmd_path(std::path::Path::new("/tmp/x"))
                    .display()
                    .to_string()
            )
        );
        assert!(
            grok_cmd.ends_with("statusline --render grok"),
            "ADR-0010 native render: {grok_cmd}"
        );
    }

    #[test]
    fn codex_tui_section_is_builtin_ids_not_command_argv() {
        let tui = render_codex_tui_section(CODEX_STATUS_LINE_ITEMS);
        // REQ-051 换序对齐 L1 至 L2 语义:版本到目录到分支到模型到 context
        // 三键;run-state 出默认集(grok 邻格评审,态归 herdr 域)。
        let order: Vec<usize> = [
            "codex-version",
            "current-dir",
            "git-branch",
            "model-with-reasoning",
            "context-used",
            "context-window-size",
            "used-tokens",
        ]
        .iter()
        .map(|id| tui.find(&format!("\"{id}\"")).expect("item present"))
        .collect();
        assert!(
            order.windows(2).all(|w| w[0] < w[1]),
            "REQ-051 order: {order:?}"
        );
        assert!(
            !tui.contains("run-state") && !tui.contains("branch-changes"),
            "run-state 与 branch-changes 出默认集: {tui}"
        );
        assert!(tui.contains("status_line_use_colors = true"));
        assert!(
            !tui.contains("total-input-tokens") && !tui.contains("total-output-tokens"),
            "in/out items retired: {tui}"
        );
        assert!(
            !tui.contains("pwsh") && !tui.contains("oma-statusline"),
            "Codex silently skips unknown IDs; command argv empties the bar: {tui}"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("used-tokens"))
            .unwrap();
        assert!(
            !last.trim().ends_with(','),
            "TOML 1.0 rejects trailing commas: {last}"
        );
    }

    #[test]
    fn rewrite_tui_normalizes_headers_keeps_siblings_and_heals() {
        // 全量评审 F1 加 F2：表头归一（空格加注释加 BOM 形皆认）；兄弟键
        // 保留并入新表；重复 [tui] 合并治伤；幂等（重跑同文）。
        let items = ["a", "b"];
        let plain = "model = \"gpt\"\n[tui]\nstatus_line = [\"old\"]\nnotifications = true\n[sandbox]\nmode = \"rw\"\n";
        let out = rewrite_tui_section(plain, &items);
        assert!(out.contains("model = \"gpt\""));
        assert!(out.contains("[sandbox]"));
        assert!(!out.contains("\"old\""));
        assert_eq!(out.matches("[tui]").count(), 1);
        assert!(
            out.contains("notifications = true"),
            "sibling key kept: {out}"
        );
        // 非规范表头三形治坏：单表输出。
        for hdr in ["[ tui ]", "[tui] # codex ui", "\u{feff}[tui]"] {
            let bad = format!("x = 1\n{hdr}\nstatus_line = [\"old\"]\n");
            let out = rewrite_tui_section(&bad, &items);
            assert_eq!(out.matches("[tui]").count(), 1, "heals {hdr}: {out}");
            assert!(!out.contains("\"old\""));
        }
        // 重复 [tui]（F1 存量伤）合并。
        let dup = "x = 1\n[tui]\nstatus_line = [\"a\"]\n[tui]\nstatus_line = [\"b\"]\n";
        let out = rewrite_tui_section(dup, &items);
        assert_eq!(out.matches("[tui]").count(), 1, "merges dup: {out}");
        // 幂等：重跑同文。
        assert_eq!(out, rewrite_tui_section(&out, &items));
        // 子表不误剥。
        let sub = "[tui]\nstatus_line = [\"a\"]\n[tui.theme]\ncolor = \"dark\"\n";
        let out = rewrite_tui_section(sub, &items);
        assert!(out.contains("[tui.theme]"), "subtable kept: {out}");
    }
}
