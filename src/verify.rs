//! `hst agents verify`（D17）：四家 agent 的 hook 与状态栏全平台无头验收。
//! 两层判据（S033 源码实证底座；D28 起注册面全量用户级）：
//! - 状态栏：**验收已部署的面**（D37，2026-09-13 wsl 总台验收适配）：脚本
//!   本体直跑（mock 空 JSON 喂 stdin，断言 stdout 任一行含 `agent:state`
//!   机读标记（D42 三行布局起 agent 态在第二行；D46 起可带连字符版本形
//!   `agent-<version>:state`），S025；脚本由 verify 按
//!   需释放）；codex 无外部命令面（M045），断
//!   `~/.codex/config.toml` 的 `[tui] status_line` 含内置项 ID。面未部署
//!   （无 `[tui] status_line`）或可选运行时缺位（pwsh 不在 PATH）= skip
//!   不计败、带 CTA（状态栏是可选面，doctor 另有 warn）；已部署但断链
//!   （marker 缺、内置项缺 run-state 锚、脚本非零退出）仍 fail。
//! - hook：临时目录起无头会话，判据只押 SessionStart / UserPromptSubmit
//!   这类先于模型调用的事件——模型应答失败（无 token、网络错）不影响判
//!   定，state 文件落盘即 ok。D28 判据隔离：子进程带
//!   `HST_STATE_FILE` 指进临时目录（shim 与 hst hook 都认，env 经
//!   agent 进程继承给 hook 子进程）；万一某家不透传 env，回落扫用户级
//!   `~/.hst/state/` 里本轮窗口内新写的 `<agent>*.json`。注册走
//!   **真实用户级面**（四家同一形态，即产品面本身）：byte 备份五件配置、
//!   deploy、Drop 还原（kimi M058 实证全局触发面泛化到四家，2026-09-11）。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::agents;

/// verify 认识的四家（与 agents::SPECS 同集）。
pub const SUPPORTED: &[&str] = &["claude", "codex", "grok", "kimi"];
/// 单家无头会话缺省最长秒数。
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// 单层验收结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerVerdict {
    /// Ok：无头验收 verify公开项。
    Ok,
    /// codex 状态栏：内置项 ID 面，无外部命令可跑（M045）。
    Builtin,
    /// Skip(String),：无头验收 verify公开项。
    Skip(String),
    /// Fail {：无头验收 verify公开项。
    Fail {
        /// reason: String,：无头验收 verify公开项。
        reason: String,
        /// hint: Option<String>：无头验收 verify公开项。
        hint: Option<String>,
    },
}

/// 单家验收结果（两层各一条；skip 时两层不跑）。
#[derive(Debug)]
pub struct AgentOutcome {
    /// agent：无头验收 verify公开项。
    pub agent: String,
    /// 整机 skip 原因（not-installed）；Some 时两层不跑、不算失败。
    pub skip: Option<String>,
    /// statusline：无头验收 verify公开项。
    pub statusline: LayerVerdict,
    /// hook：无头验收 verify公开项。
    pub hook: LayerVerdict,
    /// hook 层观测到的四态（ok 时必有）。
    pub hook_state: Option<String>,
}

impl AgentOutcome {
    fn skipped(agent: &str, reason: &str) -> Self {
        AgentOutcome {
            agent: agent.to_string(),
            skip: Some(reason.to_string()),
            statusline: LayerVerdict::Skip(reason.to_string()),
            hook: LayerVerdict::Skip(reason.to_string()),
            hook_state: None,
        }
    }
}

fn wanted_names(names: &[String]) -> Result<Vec<String>, String> {
    if names.is_empty() {
        return Ok(SUPPORTED.iter().map(|s| s.to_string()).collect());
    }
    let unknown: Vec<&str> = names
        .iter()
        .filter(|n| !SUPPORTED.contains(&n.as_str()))
        .map(String::as_str)
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "verify supports claude/codex/grok/kimi only: {}",
            unknown.join(",")
        ));
    }
    Ok(names.to_vec())
}

/// 验收主流程：逐家两层，skip（未装）不算失败。
pub fn run(names: &[String], timeout_secs: u64) -> Result<Vec<AgentOutcome>, String> {
    let wanted = wanted_names(names)?;
    let reports = agents::detect();
    let home = crate::install::hst_home()?;
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hst"));
    let mut outcomes = Vec::new();
    for name in wanted {
        let hit = reports
            .iter()
            .find(|r| r.agent == name)
            .and_then(|r| r.hit.as_ref());
        let Some(hit) = hit else {
            outcomes.push(AgentOutcome::skipped(&name, "not-installed"));
            continue;
        };
        let statusline = verify_statusline(&name, &home);
        let (hook, hook_state) = verify_hook(&name, &hit.path, &exe, timeout_secs);
        outcomes.push(AgentOutcome {
            agent: name,
            skip: None,
            statusline,
            hook,
            hook_state,
        });
    }
    Ok(outcomes)
}

/// 任一非跳过项失败即 true（进程退出 1 的判据）。
pub fn any_fail(outcomes: &[AgentOutcome]) -> bool {
    outcomes.iter().any(|o| {
        matches!(o.statusline, LayerVerdict::Fail { .. })
            || matches!(o.hook, LayerVerdict::Fail { .. })
    })
}

/// kv marker 行渲染（风格对齐 statusline/install 等现有命令）。
pub fn render(outcomes: &[AgentOutcome]) -> Vec<String> {
    let mut lines = Vec::new();
    for o in outcomes {
        if let Some(reason) = &o.skip {
            lines.push(format!("verify.{}=skip reason={reason}", o.agent));
            continue;
        }
        let failed = matches!(o.statusline, LayerVerdict::Fail { .. })
            || matches!(o.hook, LayerVerdict::Fail { .. });
        lines.push(format!(
            "verify.{}={}",
            o.agent,
            if failed { "fail" } else { "ok" }
        ));
        render_layer(&mut lines, &o.agent, "statusline", &o.statusline);
        render_layer(&mut lines, &o.agent, "hook", &o.hook);
        if let Some(state) = &o.hook_state {
            lines.push(format!("verify.{}.hook.state={state}", o.agent));
        }
    }
    lines.push(format!("verify.ok={}", !any_fail(outcomes)));
    lines
}

fn render_layer(lines: &mut Vec<String>, agent: &str, layer: &str, verdict: &LayerVerdict) {
    match verdict {
        LayerVerdict::Ok => lines.push(format!("verify.{agent}.{layer}=ok")),
        LayerVerdict::Builtin => lines.push(format!("verify.{agent}.{layer}=builtin")),
        LayerVerdict::Skip(reason) => {
            lines.push(format!("verify.{agent}.{layer}=skip reason={reason}"))
        }
        LayerVerdict::Fail { reason, hint } => {
            lines.push(format!("verify.{agent}.{layer}=fail reason={reason}"));
            if let Some(h) = hint {
                lines.push(format!("verify.{agent}.{layer}.hint={h}"));
            }
        }
    }
}

// ── 层 1：状态栏 ──

fn verify_statusline(agent: &str, home: &Path) -> LayerVerdict {
    if agent == "codex" {
        return codex_statusline_builtin();
    }
    let script = match crate::statusline::deploy_script(home) {
        Ok(p) => p,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("deploy-script: {e}"),
                hint: None,
            }
        }
    };
    if !crate::statusline::pwsh_on_path() {
        // D37：pwsh 是可选运行时（README 前置：缺了只是不渲染），未装不
        // 计败，skip 带 CTA；doctor 的状态栏 warn 面另行覆盖。
        return LayerVerdict::Skip(
            "pwsh-not-on-path（状态栏可选运行时缺位：装 PowerShell 7 后重跑 hst statusline 与 verify）".into(),
        );
    }
    // D46（codex F7/F1）：mock 空 JSON 无 version 必走本地探，钉 HST_VER_CACHE_DIR
    // 到系统临时目录的本轮专用子目录（用完即删），不读不写真实 ~/.hst/cache、
    // 不在数据根留常驻子件（pristine 迁移判据不受扰；冷缓存每家一次
    // --version spawn 的代价口径见 R002）。
    let ver_cache =
        std::env::temp_dir().join(format!("hst-verify-ver-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ver_cache);
    let mut child = match Command::new("pwsh")
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script)
        .arg(agent)
        .env("HST_VER_CACHE_DIR", &ver_cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("pwsh-spawn: {e}"),
                hint: None,
            }
        }
    };
    // 脚本对空/无 stdin 有容错；喂 `{}` 走 claude 形态的 JSON 解析路径。
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"{}");
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&ver_cache);
            return LayerVerdict::Fail {
                reason: format!("pwsh-wait: {e}"),
                hint: None,
            };
        }
    };
    // 探针隔离目录用完即删（临时目录防呆，失败不致命）。
    let _ = std::fs::remove_dir_all(&ver_cache);
    if !out.status.success() {
        return LayerVerdict::Fail {
            reason: format!("script-exit-{}", out.status.code().unwrap_or(-1)),
            hint: None,
        };
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if statusline_marker_ok(agent, &stdout) {
        LayerVerdict::Ok
    } else {
        LayerVerdict::Fail {
            reason: "marker-missing".into(),
            hint: Some(format!(
                "状态栏脚本 stdout 应含机读标记 {agent}[-<version>]:<state>（S025；D42 起 agent 态在第二行，D46 起可带连字符版本形）"
            )),
        }
    }
}

/// 纯函数：stdout 任一行含机读标记，两形兼容（D42 三行布局起 agent 态
/// 在第二行；D46 起版本并入 agent 名）。旧形 `<agent>:<state>`；新形
/// `<agent>-<version>:<state>`（用户裁连字符拼接），版本 token 数字起头
/// 且含点（probe 正则与 payload version 都至少一段 `.N`；点门槛同时压
/// 「claude-2026:」类无点数字误报）。探不到版本时脚本回落旧形，两形都
/// 必须过。
pub fn statusline_marker_ok(agent: &str, stdout: &str) -> bool {
    // D42 三行布局：agent 态在第二行（一行 = 项目状态），机读标记从
    // 「首行含」放宽为「任一行含」（消费面 = verify 与 grep，S025 契约
    // 随布局修订；kimi / grok 运行时并单行不受影响）。
    stdout.lines().any(|l| marker_line_hit(agent, l))
}

/// 单行标记判据：行内找 `<agent>` 出现位，其后直跟 `:`（旧形），或跟
/// `-<version>:`（D46 新形，版本后必须收在 `:`）。
fn marker_line_hit(agent: &str, line: &str) -> bool {
    let mut from = 0;
    while let Some(pos) = line[from..].find(agent) {
        let rest = &line[from + pos + agent.len()..];
        if rest.starts_with(':') {
            return true;
        }
        if let Some(r) = rest.strip_prefix('-') {
            let tok = r.split([':', ' ']).next().unwrap_or("");
            if tok.starts_with(|c: char| c.is_ascii_digit())
                && tok.contains('.')
                && r[tok.len()..].starts_with(':')
            {
                return true;
            }
        }
        from += pos + agent.len();
    }
    false
}

/// codex：`[tui] status_line` 含内置项 ID 即 Builtin；键未部署 = Skip（D37，
/// 部署归 `hst statusline` 面，init 不写）；已部署但缺 run-state 锚 = Fail。
fn codex_statusline_builtin() -> LayerVerdict {
    let config = match crate::pathutil::user_home() {
        Ok(h) => h.join(".codex").join("config.toml"),
        Err(_) => {
            return LayerVerdict::Fail {
                reason: "no-home".into(),
                hint: None,
            }
        }
    };
    let text = match fs::read_to_string(&config) {
        Ok(t) => t,
        Err(_) => {
            // 无 config（连 init 都没跑过）或不可读：面未部署，skip。
            return LayerVerdict::Skip(
                "statusline-not-deployed（[tui] status_line 未部署：跑 hst statusline codex）"
                    .into(),
            );
        }
    };
    codex_statusline_from_text(&text)
}

/// 纯函数：按 config.toml 文本判 codex 状态栏层（D37 测试面）。
fn codex_statusline_from_text(text: &str) -> LayerVerdict {
    if !codex_statusline_key_present(text) {
        return LayerVerdict::Skip(
            "statusline-not-deployed（[tui] status_line 未部署：跑 hst statusline codex）".into(),
        );
    }
    if codex_builtin_statusline_ok(text) {
        LayerVerdict::Builtin
    } else {
        LayerVerdict::Fail {
            reason: "status_line-missing-builtin-ids".into(),
            hint: Some("跑 `hst statusline codex` 写入 [tui] 内置项 ID".into()),
        }
    }
}

/// 纯函数：`[tui]` 段是否有 `status_line` 键（单行或多行数组起笔都算）。
fn codex_statusline_key_present(text: &str) -> bool {
    let mut in_tui = false;
    for ln in text.lines() {
        let t = ln.trim();
        if t.starts_with('[') {
            in_tui = t == "[tui]";
            continue;
        }
        if in_tui && t.starts_with("status_line") {
            return true;
        }
    }
    false
}

/// 纯函数：config.toml 的 `[tui]` 段 `status_line`（单行或多行数组）含
/// run-state 锚（hst 部署的内置项清单恒含，M045 无外部命令面）。
pub fn codex_builtin_statusline_ok(text: &str) -> bool {
    let mut in_tui = false;
    let mut in_list = false;
    for ln in text.lines() {
        let t = ln.trim();
        if t.starts_with('[') && !in_list {
            in_tui = t == "[tui]";
            continue;
        }
        if !in_tui {
            continue;
        }
        if t.starts_with("status_line") && t.contains('[') {
            in_list = true;
        }
        if in_list {
            if t.contains("run-state") {
                return true;
            }
            if t.contains(']') {
                in_list = false;
            }
        }
    }
    false
}

// ── 层 2：hook 无头落盘 ──

fn verify_hook(
    agent: &str,
    bin: &Path,
    exe: &Path,
    timeout_secs: u64,
) -> (LayerVerdict, Option<String>) {
    let tmp = std::env::temp_dir().join(format!(
        "hst-verify-{agent}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let result = verify_hook_in(agent, bin, exe, timeout_secs, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    result
}

fn verify_hook_in(
    agent: &str,
    bin: &Path,
    _exe: &Path,
    timeout_secs: u64,
    tmp: &Path,
) -> (LayerVerdict, Option<String>) {
    let fail = |reason: String| {
        (
            LayerVerdict::Fail {
                reason,
                hint: hook_hint(agent),
            },
            None,
        )
    };
    if let Err(e) = fs::create_dir_all(tmp) {
        return fail(format!("tmpdir: {e}"));
    }
    if let Err(e) = git_init(tmp) {
        return fail(format!("git-init: {e}"));
    }
    // D28：注册走真实用户级面（四家同一形态，byte 备份 + deploy + Drop 还原；
    // 真实家目录直取 dirs::home_dir，不受 HST_USER_HOME 隔离缝影响——agent
    // 本体读的是真实家）。
    let _user_guard = match UserHooksGuard::seed() {
        Ok(g) => g,
        Err(e) => return fail(format!("seed-user-hooks: {e}")),
    };
    // grok 项目源 hooks 受 folder trust 门禁（S033：xai-grok-config loader）：
    // 验收临时目录先种子 ~/.grok/trusted_folders.toml，Drop 时摘除（已信任不动）。
    let _grok_guard = if agent == "grok" {
        match grok_trust_add(tmp) {
            Ok(g) => Some(g),
            Err(e) => return fail(format!("grok-trust: {e}")),
        }
    } else {
        None
    };
    // 判据隔离：state 落进临时目录（env 经 agent 进程继承给 hook 子进程）。
    let state_file = tmp.join("hst-verify-state").join(format!("{agent}.json"));
    if let Some(dir) = state_file.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            return fail(format!("state-dir: {e}"));
        }
    }
    let started = std::time::SystemTime::now();
    let argv = match headless_argv(agent, bin, tmp) {
        Some(a) => a,
        None => return fail(format!("unknown agent {agent}")),
    };
    let mut child = match Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(tmp)
        .env("HST_STATE_FILE", &state_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return fail(format!("spawn: {e}")),
    };
    // 超时杀进程不算失败本身：判据只看 state 文件落盘。
    let _exited = wait_with_timeout(&mut child, Duration::from_secs(timeout_secs));
    match fs::read_to_string(&state_file) {
        Ok(text) => match parse_state(&text) {
            Ok(state) => (LayerVerdict::Ok, Some(state)),
            Err(e) => fail(format!("bad-state: {e}")),
        },
        Err(_) => {
            // 回落：某家不透传 env 时，扫用户级状态目录本轮窗口内的新文件。
            match freshest_new_user_state(agent, started) {
                Some(text) => match parse_state(&text) {
                    Ok(state) => (LayerVerdict::Ok, Some(state)),
                    Err(e) => fail(format!("bad-state(user): {e}")),
                },
                None => fail("no-state-file".into()),
            }
        }
    }
}

/// 用户级状态目录里本轮窗口内新写的 `<agent>*.json`（取最新 mtime）。
/// 只读不删：键文件归 SessionEnd GC 与写侧清扫管（并发的活会话不碰）。
/// 真实家目录直取（shim 写 `%USERPROFILE%\.hst\state` 不看 HST_ROOT）。
fn freshest_new_user_state(agent: &str, started: std::time::SystemTime) -> Option<String> {
    let home = verify_real_home().ok()?;
    let dir = crate::pathutil::data_dir(&home).join("state");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for ent in fs::read_dir(&dir).ok()?.flatten() {
        let p = ent.path();
        let name = p.file_name().and_then(|n| n.to_str())?;
        if !name.starts_with(agent) || !name.ends_with(".json") {
            continue;
        }
        let Ok(mtime) = ent.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if mtime < started {
            continue;
        }
        if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
            best = Some((mtime, p));
        }
    }
    let (_, p) = best?;
    fs::read_to_string(p).ok()
}

/// 环境原因失败时的可行动 hint（不只打 fail）。
fn hook_hint(agent: &str) -> Option<String> {
    Some(match agent {
        "codex" => "codex 信任闸 exec 下静默跳过无提示（S033）：确认 \
            --dangerously-bypass-hook-trust 已带；D28 用户级注册的 \
            [hooks.state] trusted_hash 由 hst init 预种在 ~/.codex/config.toml。\
            hook 经会话环境 shell 执行（Windows 缺省 PowerShell，M057）：\
            commandWindows 为无引号正斜杠形态（M059，bash / PS / cmd 三吃）"
            .into(),
        "grok" => "grok hooks 走全局层 ~/.grok/hooks/（D28）：verify 已 byte 备份\
            并 deploy 用户级注册（结束还原）；项目源 folder trust 门禁条目也已\
            自动种子并摘除；仍失败检查 grok 版本对 global hooks 层的支持"
            .into(),
        "kimi" => "kimi print 模式只触发全局 [[hooks]]（S033 勘误，2026-09-10 实证）；\
            D28 注册面即用户级 ~/.kimi-code/config.toml，verify 已 byte 备份 deploy\
            并在结束还原；判据只押 SessionStart/UserPromptSubmit"
            .into(),
        _ => "确认 shim 在位（~/.hst/hooks/hst-state.*，hst init 部署）且 jq 在 \
            PATH（jq 缺位时 cmd shim 走 findstr 回落）；判据只看 state 落盘（env \
            隔离文件加用户级窗口回落双路），模型应答失败不影响"
            .into(),
    })
}

/// verify 的真实家取值（刻意不认 HST_USER_HOME 隔离缝：live 验收要验的
/// 就是真实用户面，agent 本体只读真实家；与 grok folder trust 走认缝的
/// user_home 是有意的两面，F6 记档）。
fn verify_real_home() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "no home".to_string())
}

/// 用户级注册面的 byte 备份加 Drop 还原（D28，M058 kimi 全局面泛化到四
/// 家）：五件配置逐字节备份，deploy 后验收，Drop 时还原（原文件不在则整
/// 删）。
struct UserHooksGuard {
    backups: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl UserHooksGuard {
    fn seed() -> Result<Self, String> {
        let home = verify_real_home()?;
        let root = crate::install::hst_home()?;
        let rels = [
            ".claude/settings.json",
            ".codex/hooks.json",
            ".codex/config.toml",
            ".grok/hooks/ohmyagents-state.json",
            ".kimi-code/config.toml",
        ];
        let mut backups = Vec::new();
        for rel in rels {
            let p = home.join(rel);
            let backup = std::fs::read(&p).ok();
            backups.push((p, backup));
        }
        // F7：guard 先构造（备份已在对象内），deploy 失败提前返回也随
        // Drop byte 还原，不留 deploy 后的半程状态。
        let guard = UserHooksGuard { backups };
        let mut report = crate::deploy::DeployReport::default();
        crate::deploy::deploy_user_hooks_with(
            &home,
            &root,
            crate::deploy::host_side(),
            &mut report,
        )?;
        Ok(guard)
    }
}

impl Drop for UserHooksGuard {
    fn drop(&mut self) {
        for (p, backup) in &self.backups {
            match backup {
                Some(bytes) => {
                    let _ = std::fs::write(p, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }
}

/// grok folder trust 临时条目（S033 项目源 hooks 门禁）。Drop 时摘除：
/// 文件原本不在则整文件删掉；原本在则只摘我们加的 key。
struct GrokTrustGuard {
    file: PathBuf,
    key: String,
    file_existed: bool,
}

impl Drop for GrokTrustGuard {
    fn drop(&mut self) {
        if self.file_existed {
            if let Ok(mut toml) = crate::yolo::read_toml(&self.file) {
                grok_trust_remove(&mut toml, &self.key);
                let _ = crate::yolo::toml_write(&self.file, &toml);
            }
        } else {
            let _ = fs::remove_file(&self.file);
        }
    }
}

/// 纯函数：[folders.<key>] 落 trusted=true（对齐 yolo::apply_pretrust 的
/// grok 段）。返回是否为新增（已信任则不动，调用方也不该摘除）。
fn grok_trust_insert(toml: &mut toml::Value, key: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("grok trusted_folders.toml is not a table".into()),
    };
    let folders = table
        .entry("folders".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let folders = match folders {
        toml::Value::Table(t) => t,
        _ => return Err("grok [folders] is not a table".into()),
    };
    let already = folders
        .get(key)
        .and_then(|v| v.get("trusted"))
        .and_then(|v| v.as_bool())
        == Some(true);
    if already {
        return Ok(false);
    }
    let mut entry = toml::map::Map::new();
    entry.insert("trusted".into(), toml::Value::Boolean(true));
    let decided_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    entry.insert("decided_at".into(), toml::Value::Integer(decided_at));
    folders.insert(key.to_string(), toml::Value::Table(entry));
    Ok(true)
}

/// 纯函数：摘除 [folders.<key>]；folders 空了连表一起摘。
fn grok_trust_remove(toml: &mut toml::Value, key: &str) {
    let toml::Value::Table(table) = toml else {
        return;
    };
    let empty = match table.get_mut("folders") {
        Some(toml::Value::Table(folders)) => {
            folders.remove(key);
            folders.is_empty()
        }
        _ => false,
    };
    if empty {
        table.remove("folders");
    }
}

fn grok_trust_add(root: &Path) -> Result<Option<GrokTrustGuard>, String> {
    let home = crate::pathutil::user_home()?;
    let file = home.join(".grok").join("trusted_folders.toml");
    let file_existed = file.exists();
    let key = crate::pathutil::native_slash(root);
    let mut toml = crate::yolo::read_toml(&file)?;
    if !grok_trust_insert(&mut toml, &key)? {
        return Ok(None); // 已信任：不守卫、不摘除
    }
    crate::yolo::toml_write(&file, &toml)?;
    Ok(Some(GrokTrustGuard {
        file,
        key,
        file_existed,
    }))
}

/// hook 回退写盘的硬前置：cwd 向上 8 层内要有 .git（hook.rs）。
fn git_init(dir: &Path) -> Result<(), String> {
    match Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("exit {}", s.code().unwrap_or(-1))),
        // git 不在 PATH：回退只查 .git 存在性，手工落目录等价。
        Err(_) => {
            fs::create_dir_all(dir.join(".git")).map_err(|e| format!("{}: {e}", dir.display()))
        }
    }
}

/// 各家的无头命令行（S033 取证）。codex 的 bypass 旗标必须：否则 hook
/// 信任闸静默跳过无任何提示（假绿）。
pub fn headless_argv(agent: &str, bin: &Path, project: &Path) -> Option<Vec<String>> {
    let bin = bin.display().to_string();
    let prompt = "Reply with OK".to_string();
    match agent {
        "claude" => Some(vec![
            bin,
            "-p".into(),
            prompt,
            "--dangerously-skip-permissions".into(),
        ]),
        "codex" => Some(vec![
            bin,
            "exec".into(),
            "--dangerously-bypass-hook-trust".into(),
            prompt,
        ]),
        "grok" => Some(vec![
            bin,
            "-p".into(),
            prompt,
            "--always-approve".into(),
            "--cwd".into(),
            project.display().to_string(),
        ]),
        "kimi" => Some(vec![bin, "-p".into(), prompt]),
        _ => None,
    }
}

/// 等进程退出或超时；超时杀进程并等回收，返回是否自行退出。
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(200)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Err(_) => return true,
        }
    }
}

/// 纯函数：state 文件 JSON 的 state 字段 ∈ 四态且 event 非空才作数。
/// event 空 = shim 没解析到 stdin payload（codex review G2：此前的判据
/// 盲区，读空也落 unknown-state 文件、verify 照样绿）。
pub fn parse_state(text: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("state json: {e}"))?;
    let state = v
        .get("state")
        .and_then(|s| s.as_str())
        .ok_or_else(|| "state field missing".to_string())?;
    match state {
        "idle" | "working" | "blocked" | "unknown" => {}
        other => return Err(format!("unexpected state {other}")),
    }
    let event = v
        .get("event")
        .and_then(|e| e.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if event.is_empty() {
        return Err("empty event (shim failed to parse stdin payload)".into());
    }
    Ok(state.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dies_codex_statusline_undeployed_is_skip_not_fail() {
        // D37：`[tui] status_line` 只有 hst statusline 才写，init 不写；
        // 全新 init 态（无键、无 [tui] 段、无 config 内容）是 skip 不计败。
        assert!(matches!(
            codex_statusline_from_text("model = \"gpt\"\n"),
            LayerVerdict::Skip(_)
        ));
        assert!(matches!(
            codex_statusline_from_text("[tui]\nnotifications = true\n"),
            LayerVerdict::Skip(_)
        ));
    }

    #[test]
    fn codex_statusline_deployed_but_broken_is_fail() {
        // 已部署（键在）但缺 run-state 锚：真断链，仍 fail。
        let deployed = "[tui]\nstatus_line = [\"custom-only\"]\n";
        assert!(codex_statusline_key_present(deployed));
        assert!(matches!(
            codex_statusline_from_text(deployed),
            LayerVerdict::Fail { .. }
        ));
        let ok_form = "[tui]\nstatus_line = [\"run-state\"]\n";
        assert!(matches!(
            codex_statusline_from_text(ok_form),
            LayerVerdict::Builtin
        ));
    }

    #[test]
    fn headless_argv_shapes_per_agent() {
        let bin = Path::new("C:/bin/agent.exe");
        let project = Path::new("C:/tmp/proj");
        let codex = headless_argv("codex", bin, project).unwrap();
        assert_eq!(codex[1], "exec");
        // S033：bypass 旗标必须，否则信任闸静默跳过（假绿）。
        assert!(codex.contains(&"--dangerously-bypass-hook-trust".to_string()));
        let grok = headless_argv("grok", bin, project).unwrap();
        assert!(grok.contains(&"--always-approve".to_string()));
        let cwd_at = grok.iter().position(|a| a == "--cwd").unwrap();
        assert_eq!(grok[cwd_at + 1], project.display().to_string());
        let claude = headless_argv("claude", bin, project).unwrap();
        assert!(claude.contains(&"--dangerously-skip-permissions".to_string()));
        let kimi = headless_argv("kimi", bin, project).unwrap();
        assert_eq!(kimi.len(), 3, "kimi -p 硬编码 auto 批准，无额外线标");
        assert!(headless_argv("bogus", bin, project).is_none());
    }

    #[test]
    fn parse_state_accepts_the_four_states() {
        for s in ["idle", "working", "blocked", "unknown"] {
            let text = format!("{{\"state\": \"{s}\", \"event\": \"sessionstart\"}}");
            assert_eq!(parse_state(&text).unwrap(), s);
        }
    }

    #[test]
    fn dies_parse_state_rejects_foreign_state_and_garbage() {
        // 期望来自 hook.rs 的四态封闭集（独立来源），非被测逻辑镜像。
        assert!(parse_state("{\"state\": \"running\"}").is_err());
        assert!(parse_state("{\"event\": \"stop\"}").is_err());
        assert!(parse_state("not json").is_err());
        // codex review G2：event 空 = shim 读空 stdin 的空解析产物，不作数。
        assert!(parse_state("{\"state\": \"unknown\", \"event\": \"\"}").is_err());
        assert!(parse_state("{\"state\": \"idle\"}").is_err());
    }

    #[test]
    fn statusline_marker_may_be_on_any_line_with_agent_prefix() {
        assert!(statusline_marker_ok("claude", "claude:idle | ~/proj\n"));
        // ANSI 色码包裹下标记仍在行内（脚本 Seg 输出）。
        assert!(statusline_marker_ok(
            "grok",
            "\u{1b}[38;5;245mgrok:unknown\u{1b}[0m | x"
        ));
        assert!(!statusline_marker_ok("kimi", "claude:idle"));
        assert!(!statusline_marker_ok("kimi", ""));
        // D42 三行布局：agent 态在第二行，任一行命中即算（一行 = 项目
        // 状态、二行 = agent 状态、三行 = 运行时状态）。
        assert!(statusline_marker_ok(
            "kimi",
            "bash | /p | main\nkimi:idle\n3 | 2"
        ));
    }

    #[test]
    fn d46_marker_accepts_versioned_form_and_rejects_lookalikes() {
        // D46 新形（用户裁连字符拼接）：`<agent>-<version>:<state>`，版本
        // token 数字起头且含点，与脚本归一化同源。负例压空格分隔、无点
        // 数字、非数字起头、缺冒号四族。
        assert!(statusline_marker_ok(
            "claude",
            "\u{1b}[38;5;179mclaude-2.1.268:working\u{1b}[0m | x"
        ));
        assert!(statusline_marker_ok("kimi", "kimi-1.2.3-beta.1:idle\n"));
        assert!(statusline_marker_ok("grok", "grok-1.0.30:unknown"));
        assert!(
            !statusline_marker_ok("claude", "claude 2.1.268:working"),
            "space form is not the canonical separator"
        );
        assert!(!statusline_marker_ok("claude", "claude-2026:x"), "no dot");
        assert!(
            !statusline_marker_ok("claude", "claude-v2.1:working"),
            "version token must start with a digit"
        );
        assert!(
            !statusline_marker_ok("claude", "claude-2.1 working"),
            "colon terminator required"
        );
        // 同名前缀不吃误报：`claudex` 行不判成 `claude` 命中。
        assert!(!statusline_marker_ok("claude", "claudex-2.1.0:working"));
    }

    #[test]
    fn codex_builtin_accepts_multiline_and_singleline_arrays() {
        let multiline =
            "model = \"gpt\"\n\n[tui]\nstatus_line = [\n  \"run-state\",\n  \"git-branch\"\n]\n";
        assert!(codex_builtin_statusline_ok(multiline));
        let singleline = "[tui]\nstatus_line = [\"run-state\", \"current-dir\"]\n";
        assert!(codex_builtin_statusline_ok(singleline));
        // 段外出现的 run-state 不算数。
        let wrong_section = "[hooks]\nx = \"run-state\"\n[tui]\nother = 1\n";
        assert!(!codex_builtin_statusline_ok(wrong_section));
        // status_line_use_colors 同前缀不触发。
        let colors_only = "[tui]\nstatus_line_use_colors = true\n";
        assert!(!codex_builtin_statusline_ok(colors_only));
    }

    #[test]
    fn skip_outcomes_render_skip_and_do_not_fail() {
        let outcomes = SUPPORTED
            .iter()
            .map(|a| AgentOutcome::skipped(a, "not-installed"))
            .collect::<Vec<_>>();
        assert!(!any_fail(&outcomes), "skip 不算失败（退出码契约）");
        let lines = render(&outcomes);
        assert!(lines.contains(&"verify.claude=skip reason=not-installed".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=true");
    }

    #[test]
    fn any_layer_fail_marks_agent_and_run_failed() {
        let mut o = AgentOutcome::skipped("codex", "not-installed");
        o.skip = None;
        o.statusline = LayerVerdict::Builtin;
        o.hook = LayerVerdict::Fail {
            reason: "no-state-file".into(),
            hint: Some("hint-text".into()),
        };
        let lines = render(&[o]);
        assert!(lines.contains(&"verify.codex=fail".to_string()));
        assert!(lines.contains(&"verify.codex.statusline=builtin".to_string()));
        assert!(lines.contains(&"verify.codex.hook=fail reason=no-state-file".to_string()));
        assert!(lines.contains(&"verify.codex.hook.hint=hint-text".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=false");
    }

    #[test]
    fn grok_trust_insert_remove_roundtrip() {
        // 期望对齐 yolo::apply_pretrust 的 grok 段：[folders.<key>] trusted=true。
        let mut toml = toml::Value::Table(toml::map::Map::new());
        assert!(grok_trust_insert(&mut toml, "D:\\tmp\\x").unwrap());
        let folders = toml.get("folders").unwrap().as_table().unwrap();
        assert_eq!(
            folders.get("D:\\tmp\\x").unwrap().get("trusted"),
            Some(&toml::Value::Boolean(true))
        );
        // 已信任再插返回 false（调用方不摘除用户既有信任）。
        assert!(!grok_trust_insert(&mut toml, "D:\\tmp\\x").unwrap());
        grok_trust_remove(&mut toml, "D:\\tmp\\x");
        assert!(toml.get("folders").is_none(), "空 folders 表连表摘除");
        // 既有别的 key 时只摘我们的。
        let mut t2 = toml::Value::Table(toml::map::Map::new());
        grok_trust_insert(&mut t2, "D:\\keep").unwrap();
        grok_trust_insert(&mut t2, "D:\\tmp\\x").unwrap();
        grok_trust_remove(&mut t2, "D:\\tmp\\x");
        let folders = t2.get("folders").unwrap().as_table().unwrap();
        assert!(folders.get("D:\\keep").is_some());
        assert!(folders.get("D:\\tmp\\x").is_none());
    }

    #[test]
    fn ok_hook_renders_observed_state() {
        let o = AgentOutcome {
            agent: "kimi".into(),
            skip: None,
            statusline: LayerVerdict::Ok,
            hook: LayerVerdict::Ok,
            hook_state: Some("idle".into()),
        };
        let lines = render(&[o]);
        assert!(lines.contains(&"verify.kimi.hook.state=idle".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=true");
    }
}
