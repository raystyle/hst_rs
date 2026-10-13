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
//! - hook：临时目录起无头会话。entropy-01（2026-10-12 用户令）：hst-state
//!   腿退役（herdr 的 herdr-agent-state hook 已覆盖会话状态同步），运行时
//!   可观测腿只剩 claude 的 goal 面——claude 判据改押 task rounds（Stop
//!   事件端到端：临时 task 在册，无头会话收束后 rounds 递增即 ok，判据只
//!   看计数不看应答内容）；codex/grok/kimi 无运行时可观测腿，skip 带注
//!   （注册面形态由 hst doctor 的 hooks.form 验）。注册走**真实用户级
//!   面**：byte 备份五件配置、deploy、Drop 还原。

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
    /// 该字段承载无头验收的Ok数据。
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
    /// 该字段承载无头验收的agent数据。
    pub agent: String,
    /// 整机 skip 原因（not-installed）；Some 时两层不跑、不算失败。
    pub skip: Option<String>,
    /// 该字段承载无头验收的statusline数据。
    pub statusline: LayerVerdict,
    /// 该字段承载无头验收的hook数据。
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

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 无头验收的验收主流程面（细则见模块文档与集成测试）。
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

fn verify_statusline(agent: &str, _home: &Path) -> LayerVerdict {
    if agent == "codex" {
        return codex_statusline_builtin();
    }
    // REQ-038：PS1 载体完全淘汰，verify 直跑原生渲染（部署方 hst 二进制
    // `statusline --render <agent>`，stdin 喂 `{}` 走 JSON 解析路径）。
    // 原生渲染不本地探版本（payload version 归一承载，REQ-026 边界），
    // 无缓存面需隔离。
    let exe = crate::statusline::hst_bin_path();
    let mut child = match Command::new(&exe)
        .arg("statusline")
        .arg("--render")
        .arg(agent)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("render-spawn {exe}: {e}"),
                hint: None,
            }
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"{}");
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("render-wait: {e}"),
                hint: None,
            }
        }
    };
    if !out.status.success() {
        return LayerVerdict::Fail {
            reason: format!("render-exit-{}", out.status.code().unwrap_or(-1)),
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
                "原生渲染 stdout 应含机读标记 {agent} 或 {agent}-<version>（REQ-051 起 L1 身份段直立形，段界收尾；旧冒号形兼容）"
            )),
        }
    }
}

/// 无头验收的纯函数面（细则见模块文档与集成测试）。
/// REQ-051 起 L1 身份段直立形 `agent` / `agent-<version>`（态段退役无
/// 冒号尾，段界收尾；版本 token 数字起头且含点）；旧两形 `<agent>:<state>`
/// 与 `<agent>-<version>:<state>` 兼容认（装机位旧版 hst 渲染过渡期）。
pub fn statusline_marker_ok(agent: &str, stdout: &str) -> bool {
    // D42 三行布局：agent 态在第二行（一行 = 项目状态），机读标记从
    // 「首行含」放宽为「任一行含」（消费面 = verify 与 grep，S025 契约
    // 随布局修订；kimi / grok 运行时并单行不受影响）。
    stdout.lines().any(|l| marker_line_hit(agent, l))
}

/// 无头验收的单行标记判据面（细则见模块文档与集成测试）。
/// `-<version>:`（D46 新形，版本后必须收在 `:`）；REQ-051 起 L1 身份段
/// `agent` 或 `agent-<version>` 直立形（态段退役，无冒号尾）：前邻空格
/// （图标后）或行首，后随段界（空格、`|`、行尾或 ANSI 前缀）。
fn marker_line_hit(agent: &str, line: &str) -> bool {
    let mut from = 0;
    while let Some(pos) = line[from..].find(agent) {
        let at = from + pos;
        let rest = &line[at + agent.len()..];
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
        if identity_segment_hit(rest, &line[..at]) {
            return true;
        }
        from = at + agent.len();
    }
    false
}

/// REQ-051 L1 身份段直立形判据：rest = agent 之后的尾巴，before = agent
/// 之前的行前缀。尾巴可为空（行尾收）、段界起头（空格 / `|` / ANSI 转义
/// 前缀）或 `-<数字.数字>` 版本尾后接段界；前缀须以空格（图标后）、
/// ANSI 复位或行首收。路径形 `/x/claude`（前邻 `/`）与同前缀名
/// `claude-code`（`-` 后非版本形）不认。已知理论缝（评审 r1 G-3 注记）：
/// 模型 id 若恰呈 `claude-<数字.数字>` 形（现行 Anthropic id 皆
/// `claude-<名>-<版>` 多段形不触），L1 缺失时模型段可误过 marker,属可容
/// 启发式缝。
fn identity_segment_hit(rest: &str, before: &str) -> bool {
    let tail = match rest.strip_prefix('-') {
        Some(r) => {
            let tok: String = r
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
                .collect();
            if tok.starts_with(|c: char| c.is_ascii_digit()) && tok.contains('.') {
                &r[tok.len()..]
            } else {
                return false;
            }
        }
        None => rest,
    };
    let boundary = tail.is_empty()
        || tail.starts_with(' ')
        || tail.starts_with('|')
        || tail.starts_with("\x1b[");
    if !boundary {
        return false;
    }
    before.is_empty() || before.ends_with(' ') || before.ends_with("\x1b[0m")
}

/// 无头验收的codex面（细则见模块文档与集成测试）。
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

/// 无头验收的纯函数面（细则见模块文档与集成测试）。
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

/// 无头验收的纯函数面（细则见模块文档与集成测试）。
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

/// 无头验收的纯函数面（细则见模块文档与集成测试）。
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
    // entropy-01：hst 运行时可观测腿只剩 claude goal 面；他家 skip 带注
    //（注册面形态由 doctor hooks.form 验，token 腿无成功可观测副作用）。
    if agent != "claude" {
        return (
            LayerVerdict::Skip(
                "hst 运行时可观测腿仅 claude goal 面（state 腿已退役，herdr-agent-state 覆盖会话态）；注册面形态由 hst doctor 验".into(),
            ),
            None,
        );
    }
    if let Err(e) = fs::create_dir_all(tmp) {
        return fail(format!("tmpdir: {e}"));
    }
    if let Err(e) = git_init(tmp) {
        return fail(format!("git-init: {e}"));
    }
    // 注册走真实用户级面（byte 备份 + deploy + Drop 还原）。
    let _user_guard = match UserHooksGuard::seed() {
        Ok(g) => g,
        Err(e) => return fail(format!("seed-user-hooks: {e}")),
    };
    // 判据：临时 goal 在册，无头会话 Stop 后 rounds 递增（goal 腿端到端）。
    let home = match crate::install::hst_home() {
        Ok(h) => h,
        Err(e) => return fail(format!("hst-home: {e}")),
    };
    if let Err(e) = crate::task::set_task(
        &home,
        tmp,
        "hst 无头验收判据档:请勿 clear 本 task,直接简答 ok 结束",
    ) {
        return fail(format!("goal-set: {e}"));
    }
    let _goal_guard = scopeguard_clear_goal(&home, tmp);
    let argv = match headless_argv(agent, bin, tmp) {
        Some(a) => a,
        None => return fail(format!("unknown agent {agent}")),
    };
    let mut child = match Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return fail(format!("spawn: {e}")),
    };
    // 超时杀进程不算失败本身：判据只看 rounds。
    let _exited = wait_with_timeout(&mut child, Duration::from_secs(timeout_secs));
    match crate::task::show_task(&home, tmp) {
        Ok(Some(rec)) if rec.rounds >= 1 => {
            (LayerVerdict::Ok, Some(format!("rounds={}", rec.rounds)))
        }
        Ok(Some(rec)) => fail(format!("no-stop-event rounds={}", rec.rounds)),
        Ok(None) => fail("goal-record-vanished".into()),
        Err(e) => fail(format!("goal-read: {e}")),
    }
}

/// goal 收尾清理（RAII：判据读毕即 clear，不留验收残留）。
struct ClearGoal<'a> {
    home: &'a Path,
    root: &'a Path,
}

impl Drop for ClearGoal<'_> {
    fn drop(&mut self) {
        let _ = crate::task::clear_task(self.home, self.root);
    }
}

fn scopeguard_clear_goal<'a>(home: &'a Path, root: &'a Path) -> ClearGoal<'a> {
    ClearGoal { home, root }
}

/// 环境原因失败时的可行动 hint（不只打 fail）。
fn hook_hint(agent: &str) -> Option<String> {
    // entropy-01(hst-rev G-1)：判据重定后 hook 面只在 claude 实跑，hint 单臂
    // 化；他家走 skip 前置分支，本函数不再服务他臂。
    let _ = agent;
    Some(
        "确认 claude 用户级注册在位（hst hook init 部署，Stop 事件挂 hst-goal \
            shim）且无头会话真实收束；判据只看 task rounds 递增（Stop 事件端到 \
            端），模型应答内容不影响判定"
            .into(),
    )
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
        // REQ-051 起 L1 身份段直立形无冒号尾:空格段界与裸版本尾都是正形
        // (态段退役,冒号终结者要求随旧形只在兼容道保留)。
        assert!(
            statusline_marker_ok("claude", "claude 2.1.268:working"),
            "REQ-051 直立形:空格段界正形"
        );
        assert!(
            statusline_marker_ok("claude", "claude-2.1 working"),
            "REQ-051 直立形:版本尾接段界正形"
        );
        assert!(!statusline_marker_ok("claude", "claude-2026:x"), "no dot");
        assert!(
            !statusline_marker_ok("claude", "claude-v2.1:working"),
            "version token must start with a digit"
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

    #[test]
    fn marker_identity_form_and_rejections() {
        // REQ-051 L1 身份段直立形:agent 或 agent-<版本> 前邻空格(图标后)
        // 或行首,后随段界;路径形与同前缀名不认;旧冒号两形兼容。
        assert!(statusline_marker_ok(
            "claude",
            "\u{1b}[38;5;250m✳ claude-2.1.163\u{1b}[0m | 󰉋 /w | 󰂠 lane/x"
        ));
        assert!(statusline_marker_ok("claude", "✳ claude | 󰉋 /w"));
        assert!(statusline_marker_ok("kimi", "kimi-2.1.1 | hst_rs | lane/x"));
        assert!(
            statusline_marker_ok("claude", "claude:working"),
            "旧冒号形兼容"
        );
        assert!(
            statusline_marker_ok("claude", "claude-2.1.163:working"),
            "旧版本冒号形兼容"
        );
        assert!(
            !statusline_marker_ok("claude", "󰉋 /x/claude | 󰂠 y"),
            "路径尾段不认"
        );
        assert!(
            !statusline_marker_ok("claude", "󰉋 /x/claude-code | y"),
            "同前缀名不认"
        );
        assert!(
            !statusline_marker_ok("claude", "grok-1.0.50 | /w"),
            "他家名不认"
        );
    }
}
