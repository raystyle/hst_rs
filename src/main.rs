//! hst CLI 入口：子命令分发（init/doctor/agents/hook/self/skill/completions/
//! trace/diagnose/statusline/verify）与 `--format`/`--json` 信封出口（R011）。

use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;

use hst::agents;
use hst::doctor;
use hst::hook;
use hst::install;
use hst::trace;
use hst::yolo;

#[derive(Parser)]
#[command(name = "hst", version)]
struct Cli {
    /// JSON 信封输出（--format json 简写）
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
    /// 输出格式：kv（marker 行，缺省）| json（信封）| jsonl（列表逐行对象）
    #[arg(long, global = true)]
    format: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 用户级 yolo 与非阻塞键落盘（默认全套：yolo 键加 hook/skill 注册）
    Init {
        /// 写用户级无阻塞键（仅 yolo，不落 hook/skill；D28 第 3 轮起两级显式；
        /// D33 起取值式分级 full|partial|off，缺省 full，裸旗标兼容）
        #[arg(
            long,
            value_enum,
            num_args = 0..=1,
            default_missing_value = "full",
            conflicts_with = "project_yolo"
        )]
        yolo: Option<yolo::YoloLevel>,
        /// 写项目级无阻塞键（仅 yolo，项目覆盖用户级；与 --yolo 互斥；D33 分级同款）
        #[arg(
            long = "project-yolo",
            value_enum,
            num_args = 0..=1,
            default_missing_value = "full"
        )]
        project_yolo: Option<yolo::YoloLevel>,
        /// 预写用户家目录信任库（claude/codex/kimi/grok）
        #[arg(long = "pre-trust")]
        pretrust: bool,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 只读诊断 yolo / 信任 / 二进制 / state / 登录态 / hook 形态 / 状态栏
    Doctor {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 检测本机已装哪些 agent（PATH、HST_AGENT_PATH、HST_*_BIN、hst 自管根、默认目录）
    Agents {
        #[command(subcommand)]
        cmd: Option<AgentsCmd>,
    },
    /// hook 面：状态写入入口、注册部署与无头验收（D29 三支）
    Hook {
        #[command(subcommand)]
        cmd: HookCmd,
    },
    /// 配置四家状态栏（幂等：claude/codex/kimi/grok 各自配置面，脚本随 hst 释放）
    Statusline {
        /// 指定 agent（claude/codex/kimi/grok）；缺省四家都配
        #[arg(value_name = "名")]
        names: Vec<String>,
        /// 打印 ~/.hst/statusline.toml 定制示例模板后退出（D18）
        #[arg(long)]
        example: bool,
        /// 部署自备状态栏脚本（D18 整脚本替换；调用契约：首参 agent 名、stdin 喂 agent JSON、stdout 单行）
        #[arg(long, conflicts_with_all = ["example", "builtin"])]
        script: Option<PathBuf>,
        /// 还原内嵌脚本（撤销 --script 的自备替换）
        #[arg(long, conflicts_with = "example")]
        builtin: bool,
    },
    /// hst 自身管理（self update 自更新）
    #[command(name = "self")]
    SelfGroup {
        #[command(subcommand)]
        cmd: SelfSub,
    },
    /// 生成 shell 补全脚本到 stdout（S016 吸收）
    Completions {
        /// 目标 shell
        shell: clap_complete::Shell,
    },
    /// 项目内四家 agent 对话历史检索（六视图联邦读，P0013/P0014；D19 恢复）
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
    /// 活性诊断（D21，ohmycloud D45 配套）：打真网关烧最小 token，与 doctor 的零网络体检分家
    Diagnose {
        #[command(subcommand)]
        cmd: DiagnoseCmd,
    },
    /// 生成 hst 自身 SKILL.md（从活命令树自适应渲染；--write 落用户级 ~/.claude/skills/hst/，旧牌 ohmyagents 目录幂等退役）
    Skill {
        /// 写入用户级技能目录后退出（缺省打印到 stdout）
        #[arg(long)]
        write: bool,
    },
}

#[derive(Subcommand)]
enum DiagnoseCmd {
    /// 网关缓存探测：逐别名双连同 payload 判前缀缓存命中矩阵（claude 线 /v1/messages 加 codex 线 /v1/responses；ds 线自动前缀不可见特判）
    Cache {
        /// 只测这些别名；缺省 = 网关 /v1/models 全量
        #[arg(value_name = "别名")]
        aliases: Vec<String>,
    },
    /// agent 配置活性检测：claude/codex 配置指向、别名在册核对、key 活性、thinking 上限对照
    Agents,
}

#[derive(Subcommand)]
enum SelfSub {
    /// hst 自更新：dev 滚动源或 latest 正式版自替换；封版前用 --git 源码安装
    Update {
        /// 仓库（owner/name）；缺省 raystyle/hst_rs
        #[arg(long)]
        repo: Option<String>,
        /// 走正式稳定通道（releases/latest，封版 tag 触发）；缺省 dev 滚动源
        #[arg(long)]
        stable: bool,
        /// 走 cargo install --git 源码安装（封版前主路径）
        #[arg(long)]
        git: bool,
        /// 同版本也重装
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum TraceCmd {
    /// 列项目内各 agent 的会话
    Sessions {
        /// 条数上限（1-1000）；缺省全列
        #[arg(long)]
        limit: Option<usize>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 列编辑事件（按 operation_id 归组的意图操作块）
    Timeline {
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 文件过滤（glob；解析失败退子串）
        #[arg(long)]
        file: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移：跳过最新 offset 条再取窗口（往更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 按正则检索 patch、file、双意图四域（非法正则退字面子串）
    Search {
        #[arg(value_name = "关键词")]
        query: String,
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移：跳过最新 offset 条再取窗口（往更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 单文件的 agent 修改轨迹：谁、何时、基于什么意图改了这个文件
    File {
        /// 项目内相对路径（可用 glob）
        #[arg(value_name = "文件")]
        file: String,
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移：跳过最新 offset 条再取窗口（往更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 意图操作块视图：一个 operation_id 一块（一次工具调用，可能多文件）
    Blocks {
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000，取最新 N 块）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移：跳过最新 offset 块再取窗口（往更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// agent 轨迹：某家 agent 在项目内的操作块时间线
    Agent {
        /// agent 名（claude/codex/grok/kimi）
        name: String,
        /// 条数上限（1-1000，取最新 N 块）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移：跳过最新 offset 块再取窗口（往更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum HookCmd {
    /// hook 注册部署与 shim 落位（init 的 hook 面，含 ~/.oma 迁 ~/.hst 的 heal 改写）
    Init {
        /// 项目根（skill 面部署用；hook 注册本身用户级）
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 状态写入入口：读事件写用户级 `~/.hst/state/`（session 分键），加 secretguard 拦截
    Status {
        /// 事件名或四态（idle/working/blocked/unknown）；省略则读 stdin JSON
        #[arg(value_name = "事件")]
        event: Option<String>,
        /// agent 名（注册参数注入）
        #[arg(long)]
        agent: Option<String>,
    },
    /// hook 层无头验收（agents verify 的 hook 子面）
    Verify {
        /// 指定 agent（claude/codex/grok/kimi）；缺省四家全验
        #[arg(value_name = "名")]
        names: Vec<String>,
        /// 单家无头会话最长秒数
        #[arg(long)]
        timeout: Option<u64>,
    },
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// 四家 hook 与状态栏全平台无头验收（D17）：状态栏脚本直跑加 hook 无头落盘，任一非跳过项失败退出 1
    Verify {
        /// 指定 agent（claude/codex/grok/kimi）；缺省四家全验
        #[arg(value_name = "名")]
        names: Vec<String>,
        /// 单家无头会话最长秒数（超时杀进程不算失败，判据只看 state 落盘）
        #[arg(long)]
        timeout: Option<u64>,
    },
}

fn main() {
    if let Err(e) = run() {
        hst::fmtio::error_exit(e);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    hst::fmtio::init(cli.json, cli.format.as_deref())?;
    let Some(command) = cli.command else {
        // 裸 hst：无命令时打印帮助，打印帮助退出。
        let mut cmd = Cli::command();
        cmd.print_help().map_err(|e| e.to_string())?;
        println!();
        return Ok(());
    };
    match command {
        Commands::Init {
            yolo,
            project_yolo,
            pretrust,
            project,
        } => cmd_init(yolo, project_yolo, pretrust, project),
        Commands::Doctor { project } => cmd_doctor(project),
        Commands::Agents { cmd } => match cmd {
            None => {
                // 结构化三态（issue #1 契约）：json 信封；jsonl 逐 agent 行。
                match hst::fmtio::mode() {
                    hst::fmtio::Format::Json => {
                        let rows: Vec<Value> =
                            agents::detect().iter().map(agent_report_row).collect();
                        let v = serde_json::json!({
                            "installed": rows.iter().filter(|r| r["status"] == "installed").count(),
                            "missing": rows.iter().filter(|r| r["status"] == "missing").count(),
                            "agents": rows,
                        });
                        let cwd = std::env::current_dir().unwrap_or_default();
                        print_json("agents", &cwd, Ok(v))?;
                    }
                    hst::fmtio::Format::Jsonl => {
                        let rows: Vec<Value> =
                            agents::detect().iter().map(agent_report_row).collect();
                        hst::fmtio::print_jsonl(&rows);
                    }
                    hst::fmtio::Format::Kv => agents::print_reports(&agents::detect()),
                }
                Ok(())
            }
            Some(AgentsCmd::Verify { names, timeout }) => cmd_agents_verify(names, timeout, false),
        },
        Commands::Hook { cmd } => match cmd {
            HookCmd::Init { project } => cmd_hook_init(project),
            HookCmd::Status { event, agent } => cmd_hook(event, agent),
            HookCmd::Verify { names, timeout } => cmd_agents_verify(names, timeout, true),
        },
        Commands::Statusline {
            names,
            example,
            script,
            builtin,
        } => cmd_agents_statusline(names, example, script, builtin),
        Commands::SelfGroup { cmd } => match cmd {
            SelfSub::Update {
                repo,
                stable,
                git,
                force,
            } => hst::update::run(
                &repo.unwrap_or_else(|| hst::update::DEFAULT_REPO.into()),
                if stable {
                    hst::update::Channel::Latest
                } else {
                    hst::update::Channel::Dev
                },
                git,
                force,
            ),
        },
        Commands::Completions { shell } => cmd_completions(shell),
        Commands::Trace { cmd } => cmd_trace(cmd),
        Commands::Diagnose { cmd } => cmd_diagnose(cmd),
        Commands::Skill { write } => cmd_skill(write),
    }
}

/// `hst skill [--write]`：从 clap 活命令树自适应渲染 SKILL.md（D22）。
/// D49：技能名翻 hst 唯一名；旧牌 ohmyagents 目录幂等退役（第 2 轮裁定
/// 直接删除，用户手改跳过）。
fn cmd_skill(write: bool) -> Result<(), String> {
    let body = hst::skillgen::render_skill(&Cli::command());
    if write {
        let skills = hst::pathutil::user_home()?.join(".claude").join("skills");
        let dir = skills.join(hst::skillgen::SKILL_NAME);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join("SKILL.md");
        std::fs::write(&path, &body).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("skill.wrote={}", path.display());
        // D49 第 2 轮：旧牌用户级目录退役（内容带我们生成签名才删）。
        let legacy = skills.join("ohmyagents");
        if let Some(p) = retire_user_skill(&legacy) {
            println!("skill.retired={}", p.display());
        }
        Ok(())
    } else {
        println!("{body}");
        Ok(())
    }
}

/// 旧牌用户级技能目录退役：SKILL.md 带我们生成签名（活命令树自适应
/// 生成行，oma 与 hst 两代都含）才退役；用户手改或他源不动。外科式
///（codex F4）：先删 SKILL.md，目录仅在空时收（伴生资源不连带删）。
fn retire_user_skill(dir: &std::path::Path) -> Option<&std::path::Path> {
    let md = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
    if !md.contains("活命令树自适应生成") {
        return None;
    }
    std::fs::remove_file(dir.join("SKILL.md")).ok()?;
    let _ = std::fs::remove_dir(dir);
    Some(dir)
}

/// `hst diagnose cache|agents`：活性诊断族（D21）。打真 API、烧最小 token。
fn cmd_diagnose(cmd: DiagnoseCmd) -> Result<(), String> {
    match cmd {
        DiagnoseCmd::Cache { aliases } => {
            let out = hst::diagnose::run_cache(&aliases)?;
            for line in hst::diagnose::render_cache_rows(&out) {
                println!("{line}");
            }
            let failed = out
                .iter()
                .any(|(_, _, v)| matches!(v, hst::diagnose::CacheVerdict::Error(_)));
            if failed {
                std::process::exit(1);
            }
            Ok(())
        }
        DiagnoseCmd::Agents => {
            let rows = hst::diagnose::run_agents()?;
            for (k, v) in rows {
                println!("{k}={v}");
            }
            println!("diagnose.agents.ok=true");
            Ok(())
        }
    }
}

/// --json 出口：信封进 stdout（机器面），业务失败先吐信封再向上传播退出非 0。
fn print_json(command: &str, root: &Path, outcome: Result<Value, String>) -> Result<(), String> {
    let env = hst::fmtio::envelope(command, root, outcome);
    let text = serde_json::to_string_pretty(&env).map_err(|e| e.to_string())?;
    println!("{text}");
    match env.get("ok").and_then(|v| v.as_bool()) {
        Some(true) => Ok(()),
        _ => Err(env
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("command failed")
            .to_string()),
    }
}

/// completions：clap_complete 生成，stdout 直接吐脚本。
fn cmd_completions(shell: clap_complete::Shell) -> Result<(), String> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "hst", &mut std::io::stdout());
    Ok(())
}

/// `hst statusline [名] [--example] [--script 路径] [--builtin]`：
/// 配置四家状态栏（幂等）。--script 部署自备脚本（D18 整脚本替换），
/// --builtin 还原内嵌。
fn cmd_agents_statusline(
    names: Vec<String>,
    example: bool,
    script: Option<PathBuf>,
    builtin: bool,
) -> Result<(), String> {
    if example {
        println!("{}", hst::statusline::EXAMPLE_TOML.trim_end());
        return Ok(());
    }
    let home = install::hst_home()?;
    let user_home = hst::pathutil::user_home()?;
    let supported = ["claude", "codex", "kimi", "grok"];
    let do_all = names.is_empty();
    let unknown: Vec<String> = names
        .iter()
        .filter(|n| !supported.contains(&n.as_str()))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "statusline supports claude/codex/kimi/grok only: {}",
            unknown.join(",")
        ));
    }
    // 整脚本替换先行（同一部署文件名，后续 merge 指向不变）。
    if let Some(src) = &script {
        hst::statusline::deploy_custom_script(&home, src)?;
    } else if builtin {
        hst::statusline::restore_builtin_script(&home)?;
    }
    if do_all || names.iter().any(|n| n == "claude") {
        let p = hst::statusline::merge_claude(&home, &user_home)?;
        println!("statusline.claude={p}");
    }
    if do_all || names.iter().any(|n| n == "codex") {
        let p = hst::statusline::merge_codex(&home, &user_home)?;
        println!("statusline.codex={p}");
    }
    if do_all || names.iter().any(|n| n == "kimi") {
        let p = hst::statusline::merge_kimi(&home, &user_home)?;
        println!("statusline.kimi={p}");
    }
    if do_all || names.iter().any(|n| n == "grok") {
        let p = hst::statusline::merge_grok(&home, &user_home)?;
        println!("statusline.grok={p}");
    }
    // The bar renders through pwsh on every platform; without it the merged
    // config is inert. Advisory, never fatal (P0027).
    if hst::statusline::pwsh_on_path() {
        println!("statusline.pwsh=found");
    } else {
        println!("statusline.pwsh=missing");
        println!("statusline.warn=pwsh-not-on-path-statusline-will-not-run");
    }
    if let Some(src) = &script {
        println!("statusline.custom=true");
        println!("statusline.script={}", src.display());
    } else if builtin {
        println!("statusline.custom=false");
    } else if hst::statusline::custom_active(&home) {
        println!("statusline.custom=true");
    }
    println!("statusline.ok=true");
    Ok(())
}

/// `hst agents verify [名...] [--timeout N]`：四家 hook 与状态栏无头验收（D17）。
/// 缺省四家全验；skip（未装）不算失败，任一非跳过项失败退出 1。
fn cmd_agents_verify(
    names: Vec<String>,
    timeout: Option<u64>,
    hook_only: bool,
) -> Result<(), String> {
    let mut outcomes =
        hst::verify::run(&names, timeout.unwrap_or(hst::verify::DEFAULT_TIMEOUT_SECS))?;
    if hook_only {
        // `hst hook verify`（D29）：只验 hook 层，状态栏层剔除（skip 不计败）。
        for o in outcomes.iter_mut() {
            if !matches!(o.statusline, hst::verify::LayerVerdict::Skip(_)) {
                o.statusline = hst::verify::LayerVerdict::Skip("hook-only".into());
            }
        }
    }
    for line in hst::verify::render(&outcomes) {
        println!("{line}");
    }
    if hst::verify::any_fail(&outcomes) {
        std::process::exit(1);
    }
    Ok(())
}

/// `hst hook init`：hook 面部署（注册加 shim 加 heal 迁移改写）；skill 与
/// 说明面不在此（那是 `hst init` 全套的事）。
fn cmd_hook_init(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let user_home = hst::pathutil::user_home()?;
    let oma = hst::install::hst_home()?;
    let mut report = hst::deploy::DeployReport::default();
    hst::deploy::deploy_user_hooks_with(&user_home, &oma, hst::deploy::host_side(), &mut report)?;
    for p in &report.wrote {
        println!("hook.init.wrote={p}");
    }
    println!("hook.init.wrote.count={}", report.wrote.len());
    if let Some(form) = report.form {
        println!("hook.init.form={form}");
    }
    for w in &report.warns {
        println!("hook.init.warn={w}");
    }
    Ok(())
}

fn cmd_hook(event: Option<String>, agent: Option<String>) -> Result<(), String> {
    match hook::run(event.as_deref(), agent.as_deref()) {
        Ok(outcome) => {
            if let Some(path) = outcome.state_file {
                if std::env::var_os("HST_HOOK_VERBOSE").is_some() {
                    eprintln!("hst.hook.wrote={}", path.display());
                }
            }
            if let Some(g) = outcome.guard {
                if g.block {
                    // exit 2 = agent 侧拒工具调用，stderr 原因回给模型（S030）。
                    eprintln!("hst secretguard: {}", g.reasons.join("; "));
                    std::process::exit(2);
                }
                if std::env::var_os("HST_HOOK_VERBOSE").is_some() && !g.findings.is_empty() {
                    eprintln!("hst.secretguard.findings={}", g.findings.len());
                }
            }
        }
        Err(e) => {
            // Never fail the agent session over a state-file write.
            if std::env::var_os("HST_HOOK_VERBOSE").is_some() {
                eprintln!("hst hook: {e}");
            }
        }
    }
    Ok(())
}

fn project_root(project: Option<PathBuf>) -> Result<PathBuf, String> {
    let raw = match project {
        Some(p) => p,
        None => std::env::current_dir().map_err(|e| format!("cwd: {e}"))?,
    };
    // 相对路径必须在此展开为绝对（M031）：下游按收到路径原样落盘与注册，
    // 相对路径经工作目录漂移会落错位置。
    if raw.is_relative() {
        let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
        return Ok(cwd.join(raw));
    }
    Ok(raw)
}

fn cmd_init(
    yolo: Option<yolo::YoloLevel>,
    project_yolo: Option<yolo::YoloLevel>,
    pretrust: bool,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    // Default init is the full deployment: user-level yolo keys plus user-level
    // hook registration and project skills (D28 round 2: yolo keys are
    // user-level too). Round 3: yolo scope is explicit — `--yolo` = user-level
    // keys only, `--project-yolo` = project-level keys only (project overrides
    // user where the agent supports layering). D33: both yolo flags are
    // level-taking (full|partial|off, bare flag = full); off retires ours keys
    // at the chosen scope instead of writing.
    let keys_only = yolo.is_some();
    if let Some(level) = project_yolo {
        println!("init.flag.project_yolo=true");
        println!("init.yolo.level={}", level.as_str());
        // D52 铁证修复（codex F1）：家目录不是项目——--project-yolo 于家
        // 目录会把用户级文件当项目层写摘（与裸 init 同型，宿主实弹类），
        // 整支跳过并打点。
        let home = hst::pathutil::user_home()?;
        if hst::pathutil::same_location(&root, &home) {
            println!(
                "init.warn=project-yolo skipped: root is the user home \
                 (home is not a project; user-level keys stay)"
            );
            println!("init.hooks=skipped");
        } else {
            match level {
                yolo::YoloLevel::Off => {
                    for p in yolo::retire_project_yolo(&root)? {
                        println!("init.retired={p}");
                    }
                }
                _ => {
                    let report = yolo::apply_project_yolo_level(&root, level)?;
                    for p in &report.wrote {
                        println!("init.wrote={p}");
                    }
                }
            }
            println!("init.hooks=skipped");
        }
    } else {
        // 裸 init（无旗标）= 全套部署，用户级 yolo 固定 full 级；
        // --yolo[=<级>] = 仅键模式，级别缺省 full（clap default_missing_value）。
        let level = yolo.unwrap_or(yolo::YoloLevel::Full);
        println!("init.flag.yolo={keys_only}");
        println!("init.yolo.level={}", level.as_str());
        match level {
            yolo::YoloLevel::Off => {
                for p in yolo::retire_user_yolo()? {
                    println!("init.retired={p}");
                }
            }
            _ => {
                let report = yolo::apply_user_yolo_level(level)?;
                for p in &report.wrote {
                    println!("init.wrote={p}");
                }
            }
        }
        if !keys_only {
            let deployed = hst::deploy::deploy_all(&root)?;
            for p in &deployed.wrote {
                println!("init.hooks.wrote={p}");
            }
            println!("init.hooks.wrote.count={}", deployed.wrote.len());
            println!("init.hooks.skipped.count={}", deployed.skipped.len());
            // Registration-form marker (D28): hooks live in the four agents'
            // user-level configs and point at the self-contained state shim in
            // ~/.hst/hooks/, zero hst-binary dependency.
            if let Some(form) = deployed.form {
                println!("init.hooks.form={form}");
            }
            for w in &deployed.warns {
                println!("init.hooks.warn={w}");
            }
        } else {
            println!("init.hooks=skipped");
        }
    }
    if pretrust {
        let trust = yolo::apply_pretrust(&root)?;
        for p in &trust.wrote {
            println!("init.pretrust.wrote={p}");
        }
        println!("init.pretrust=wrote");
    } else {
        println!("init.pretrust=skipped");
    }
    println!("init.project={}", root.display());
    println!(
        "init.scope={}",
        if project_yolo.is_some() {
            "yolo-project"
        } else if keys_only {
            "yolo"
        } else {
            "full"
        }
    );
    Ok(())
}

/// agents 检测行 → 结构化对象（字段序与 kv 行序一致，preserve_order）。
fn agent_report_row(r: &agents::Report) -> Value {
    match &r.hit {
        Some(h) => serde_json::json!({
            "agent": r.agent,
            "status": "installed",
            "source": h.source.as_str(),
            "path": h.path.display().to_string(),
            "version": h.version.as_deref().unwrap_or("-"),
            "extras": h.extras.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }),
        None => serde_json::json!({
            "agent": r.agent,
            "status": "missing",
            "hint": format!("ark install {}", r.agent),
        }),
    }
}

fn cmd_doctor(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let d = doctor::diagnose(&root)?;
    let findings: Vec<Value> = d
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "agent": f.agent,
                "check": f.check,
                "status": f.status.as_str(),
                "path": f.path,
                "detail": f.detail,
            })
        })
        .collect();
    // 结构化三态（issue #1 契约）：json 信封；jsonl 逐 finding 行对象；
    // blocked 退出码 1 在三种模式下一致。
    match hst::fmtio::mode() {
        hst::fmtio::Format::Json => {
            let v = serde_json::json!({ "blocked": d.blocked(), "findings": findings });
            print_json("doctor", &root, Ok(v))?;
        }
        hst::fmtio::Format::Jsonl => {
            hst::fmtio::print_jsonl(&findings);
        }
        hst::fmtio::Format::Kv => doctor::print_diagnosis(&d),
    }
    if d.blocked() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_trace(cmd: TraceCmd) -> Result<(), String> {
    let clip = |s: &str| -> String { s.chars().take(80).collect() };
    let resolve = |p: Option<PathBuf>| {
        p.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };
    match cmd {
        TraceCmd::Sessions { limit, project } => {
            let project = resolve(project);
            let mut sessions = trace::list_sessions(&project);
            let total = sessions.len();
            let shown = limit.map(|n| n.clamp(1, trace::MAX_LIMIT)).unwrap_or(total);
            sessions.truncate(shown);
            let rows: Vec<TraceRow> = sessions
                .iter()
                .map(|s| TraceRow {
                    kv: format!(
                        "trace.session agent={} id={} started={} file={}",
                        s.agent,
                        s.id,
                        s.started_at.as_deref().unwrap_or("-"),
                        s.file.display()
                    ),
                    json: serde_json::json!({
                        "agent": s.agent, "id": s.id,
                        "started": s.started_at,
                        "file": s.file.display().to_string(),
                    }),
                })
                .collect();
            emit_trace("trace.sessions.count", rows, total, 0, &project);
            Ok(())
        }
        TraceCmd::Timeline {
            agent,
            file,
            limit,
            offset,
            project,
        } => {
            let project = resolve(project);
            let filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: file.as_deref(),
                limit,
                offset,
            };
            let (events, total) = trace::apply_filter_counted(trace::timeline(&project), &filter);
            let rows: Vec<TraceRow> = events
                .iter()
                .map(|e| TraceRow {
                    kv: format!(
                        "trace.edit agent={} session={} op={} file={} kind={} tool={} ts={} intent={} op_intent={}",
                        e.agent,
                        e.session_id,
                        e.operation_id(),
                        e.file.as_deref().unwrap_or("-"),
                        e.kind.as_str(),
                        e.tool.as_deref().unwrap_or("-"),
                        e.ts.as_deref().unwrap_or("-"),
                        clip(e.user_intent.as_deref().unwrap_or("-")),
                        clip(e.op_intent.as_deref().unwrap_or("-")),
                    ),
                    json: serde_json::json!({
                        "agent": e.agent, "session": e.session_id,
                        "op": e.operation_id(),
                        "file": e.file, "kind": e.kind.as_str(),
                        "tool": e.tool, "ts": e.ts,
                        "intent": e.user_intent, "op_intent": e.op_intent,
                    }),
                })
                .collect();
            emit_trace("trace.edits.count", rows, total, offset, &project);
            Ok(())
        }
        TraceCmd::Search {
            query,
            agent,
            limit,
            offset,
            project,
        } => {
            let project = resolve(project);
            // 先全量匹配再开窗：limit 若在匹配前生效会把候选池截没。
            let pool_filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: None,
                limit: trace::MAX_LIMIT,
                offset: 0,
            };
            let (mut all, _) = trace::apply_filter_counted(trace::timeline(&project), &pool_filter);
            all.retain(|e| trace::search_matches(e, &query));
            let total = all.len();
            let n = limit.clamp(1, trace::MAX_LIMIT);
            let end = total.saturating_sub(offset);
            let start = end.saturating_sub(n);
            let events: Vec<_> = all.into_iter().skip(start).take(end - start).collect();
            let rows: Vec<TraceRow> = events
                .iter()
                .map(|e| TraceRow {
                    kv: format!(
                        "trace.hit agent={} session={} op={} file={} kind={} intent={} op_intent={}",
                        e.agent,
                        e.session_id,
                        e.operation_id(),
                        e.file.as_deref().unwrap_or("-"),
                        e.kind.as_str(),
                        clip(e.user_intent.as_deref().unwrap_or("-")),
                        clip(e.op_intent.as_deref().unwrap_or("-")),
                    ),
                    json: serde_json::json!({
                        "agent": e.agent, "session": e.session_id,
                        "op": e.operation_id(),
                        "file": e.file, "kind": e.kind.as_str(),
                        "intent": e.user_intent, "op_intent": e.op_intent,
                    }),
                })
                .collect();
            emit_trace("trace.hits.count", rows, total, offset, &project);
            Ok(())
        }
        TraceCmd::File {
            file,
            agent,
            limit,
            offset,
            project,
        } => {
            let project = resolve(project);
            // 文件维度轨迹：按传入路径或 glob 过滤，窗口从最新端往早翻页。
            let filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: Some(&file),
                limit,
                offset,
            };
            let (events, total) = trace::apply_filter_counted(trace::timeline(&project), &filter);
            let rows: Vec<TraceRow> = events
                .iter()
                .map(|e| TraceRow {
                    kv: format!(
                        "trace.file agent={} session={} op={} kind={} tool={} ts={} intent={} op_intent={}",
                        e.agent,
                        e.session_id,
                        e.operation_id(),
                        e.kind.as_str(),
                        e.tool.as_deref().unwrap_or("-"),
                        e.ts.as_deref().unwrap_or("-"),
                        clip(e.user_intent.as_deref().unwrap_or("-")),
                        clip(e.op_intent.as_deref().unwrap_or("-")),
                    ),
                    json: serde_json::json!({
                        "agent": e.agent, "session": e.session_id,
                        "op": e.operation_id(),
                        "file": e.file, "kind": e.kind.as_str(),
                        "tool": e.tool, "ts": e.ts,
                        "intent": e.user_intent, "op_intent": e.op_intent,
                    }),
                })
                .collect();
            emit_trace("trace.file.edits", rows, total, offset, &project);
            Ok(())
        }
        TraceCmd::Blocks {
            agent,
            limit,
            offset,
            project,
        } => {
            let project = resolve(project);
            print_block_timeline(&project, agent.as_deref(), limit, offset);
            Ok(())
        }
        TraceCmd::Agent {
            name,
            limit,
            offset,
            project,
        } => {
            let project = resolve(project);
            let known = ["claude", "codex", "grok", "kimi"];
            if !known.contains(&name.as_str()) {
                return Err(format!("unknown agent {name}; known: {}", known.join(", ")));
            }
            print_block_timeline(&project, Some(&name), limit, offset);
            Ok(())
        }
    }
}

/// trace 输出行：kv 形态（稳定现契约）加结构化对象（json / jsonl 用）。
struct TraceRow {
    kv: String,
    json: Value,
}

/// 六视图共享三态输出器（D26）：kv 打 marker 行加 count，窗口截断时补
/// has_more 加 total；jsonl 逐行对象；json 出信封（items 全意图不截断）。
fn emit_trace(count_key: &str, rows: Vec<TraceRow>, total: usize, offset: usize, project: &Path) {
    let shown = rows.len();
    let has_more = offset + shown < total;
    match hst::fmtio::mode() {
        hst::fmtio::Format::Kv => {
            for r in &rows {
                println!("{}", r.kv);
            }
            println!("{count_key}={shown}");
            if has_more {
                println!(
                    "trace.has_more=true total={total} offset_next={}",
                    offset + shown
                );
            }
        }
        hst::fmtio::Format::Jsonl => {
            for r in &rows {
                println!("{}", r.json);
            }
        }
        hst::fmtio::Format::Json => {
            let data = serde_json::json!({
                "count": shown,
                "total": total,
                "has_more": has_more,
                "items": rows.iter().map(|r| r.json.clone()).collect::<Vec<_>>(),
            });
            let env = hst::fmtio::envelope(count_key, project, Ok(data));
            let text = serde_json::to_string_pretty(&env).unwrap_or_default();
            println!("{text}");
        }
    }
}

/// 操作块时间线：窗口从最新端取 `[n-offset-limit, n-offset)`，窗内时间
/// 正序（与 timeline 的窗口语义一致，offset 向更早翻页）。
fn print_block_timeline(
    project: &std::path::Path,
    agent: Option<&str>,
    limit: usize,
    offset: usize,
) {
    let clip = |s: &str| -> String { s.chars().take(80).collect() };
    let filter = trace::TraceFilter {
        agent,
        file_glob: None,
        limit: trace::MAX_LIMIT,
        offset: 0,
    };
    let (events, _) = trace::apply_filter_counted(trace::timeline(project), &filter);
    let mut blocks = trace::group_blocks(&events);
    let total = blocks.len();
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(limit.clamp(1, trace::MAX_LIMIT));
    blocks.drain(..start);
    blocks.truncate(end - start);
    let rows: Vec<TraceRow> = blocks
        .iter()
        .map(|b| TraceRow {
            kv: format!(
                "trace.block op={} agent={} session={} edits={} files={} kinds={} ts={} intent={} op_intent={}",
                b.op,
                b.agent,
                b.session_id,
                b.edits,
                b.files.join(","),
                b.kinds.join("+"),
                b.first_ts.as_deref().unwrap_or("-"),
                clip(b.user_intent.as_deref().unwrap_or("-")),
                clip(b.op_intent.as_deref().unwrap_or("-")),
            ),
            json: serde_json::json!({
                "op": b.op, "agent": b.agent, "session": b.session_id,
                "edits": b.edits, "files": b.files, "kinds": b.kinds,
                "ts": b.first_ts,
                "intent": b.user_intent, "op_intent": b.op_intent,
            }),
        })
        .collect();
    emit_trace("trace.blocks.count", rows, total, offset, project);
}
