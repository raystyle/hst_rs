//! hst CLI 入口：子命令分发（init/doctor/agents/hook/self/completions/
//! trace/diagnose/statusline/verify）与 `--format`/`--json` 信封出口（fmtio 三态契约）。

use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;

use hst::agents;
use hst::doctor;
use hst::hook;
use hst::install;
use hst::loopmgmt;
use hst::trace;
use hst::yolo;

#[derive(Parser)]
#[command(
    name = "hst",
    version,
    about = "HST（Hooks, Statusline, Trace）：agent 全平台部署配置与诊断 CLI",
    // 帮助面头行（cli-docs 采纳轮）：name@version 连接一句定位，版本由
    // {version} 从载体 manifest 注入，禁手写第二份。
    help_template = "{name}@{version} {about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}",
    after_help = "agent 手册面：hst --llms（markdown 手册）；hst --llms --json（机器形）"
)]
struct Cli {
    /// json 信封 data 的键路径过滤（仅对出信封的命令生效；逗号分隔，点号嵌套，数组下标如 items[0]；响错不静默截断）
    #[arg(
        long = "filter-output",
        value_name = "keys",
        global = true,
        help_heading = "Global Options"
    )]
    filter_output: Option<String>,
    /// 输出格式（kv|json|jsonl；kv 为缺省 marker 行，json 出信封，jsonl 逐行对象）
    #[arg(
        long,
        value_name = "kv|json|jsonl",
        global = true,
        help_heading = "Global Options"
    )]
    format: Option<String>,
    /// JSON 信封输出（--format json 简写）
    #[arg(
        long,
        global = true,
        conflicts_with = "format",
        help_heading = "Global Options"
    )]
    json: bool,
    /// 打印 agent 说明书（markdown 手册；配 --json 出机器形）后退出
    #[arg(long)]
    llms: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 用户级 yolo 与非阻塞键落盘（默认全套：yolo 键加 hook 注册加状态栏面；skill 面已退役，ours 技能目录幂等清扫，ADR-0005）
    Init {
        /// 写用户级无阻塞键（仅 yolo，不落 hook；D28 第 3 轮起两级显式；
        /// D33 起取值式分级 full|partial|off，缺省 full，裸旗标兼容）
        #[arg(
            long,
            value_enum,
            num_args = 0..=1,
            default_missing_value = "full",
            conflicts_with_all = ["project_yolo", "clear_project_yolo"]
        )]
        yolo: Option<yolo::YoloLevel>,
        /// 写项目级无阻塞键（仅 yolo，项目覆盖用户级；与 --yolo 互斥；D33 分级同款）
        #[arg(
            long = "project-yolo",
            value_enum,
            num_args = 0..=1,
            default_missing_value = "full",
            conflicts_with_all = ["yolo", "clear_project_yolo"]
        )]
        project_yolo: Option<yolo::YoloLevel>,
        /// 一键清除项目级对用户级 yolo 的干扰键（ours 与外来都摘，让用户级生效；REQ-009/D55；与 --yolo/--project-yolo 互斥）
        #[arg(long, conflicts_with_all = ["yolo", "project_yolo"])]
        clear_project_yolo: bool,
        /// 写 claude 压缩触发阈值（仅 compact 面：env CLAUDE_AUTOCOMPACT_PCT_OVERRIDE，1-100 百分数，off 摘键；与 yolo 族互斥；缺省预览零写，加 --yes 落盘）
        #[arg(
            long = "compact-pct",
            value_name = "1-100|off",
            conflicts_with_all = ["yolo", "project_yolo", "clear_project_yolo", "pretrust"]
        )]
        compact_pct: Option<String>,
        /// 确认落盘（配 --compact-pct；不带则预览）
        #[arg(long, requires = "compact_pct")]
        yes: bool,
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
    /// issue 入口（账本 issue 流，真源 ledger.ohmygh.com；只增面）：开单、列表、详情（关单与状态推进归 omc 工作台）
    Issue {
        #[command(subcommand)]
        cmd: IssueCmd,
    },
    /// 产物共享库面（ledger artifact 流：publish 加 attest（三验证型）加 list；promote/demote/supersede 归 omc 工作台；真源 ledger.ohmygh.com）
    Artifact {
        #[command(subcommand)]
        cmd: ArtifactCmd,
    },
    /// 账本面（密钥管理）
    Ledger {
        #[command(subcommand)]
        cmd: LedgerCmd,
    },
    /// yolo 面：项目级干扰只读检测（REQ-017；写 marker 驱动状态栏 proj-yolo! 升格，零改动纯可见化）
    Yolo {
        #[command(subcommand)]
        cmd: YoloCmd,
    },
    /// 活性诊断（D21，ohmycloud D45 配套）：打真网关烧最小 token，与 doctor 的零网络体检分家
    Diagnose {
        #[command(subcommand)]
        cmd: DiagnoseCmd,
    },
    /// loop 与 goal 管理面（REQ-019）：当前会话 durable 定时任务的设置、列表、删除；唯一真相 = 项目 .claude/scheduled_tasks.json，状态栏 loop/goal 段同源显示
    Loop {
        #[command(subcommand)]
        cmd: LoopCmd,
    },
    /// goal 面（REQ-019）：当前会话最新 loop 的 goal 文本设置、查看与清空（改目标不动节奏）
    Goal {
        #[command(subcommand)]
        cmd: GoalCmd,
    },
}

#[derive(Subcommand)]
enum LoopCmd {
    /// 设置当前会话 loop（goal 文本落 prompt 字段；--every 周期形与 --at 一次性形二选一）
    Set {
        /// goal 文本（任务 prompt；状态栏 goal 段显示源）
        goal: String,
        /// 周期间隔（Nm 分钟 1 至 59、Nh 小时 1 至 23；小时形分钟位取落盘时刻并避开 0 与 30）
        #[arg(long, conflicts_with = "at")]
        every: Option<String>,
        /// 一次性时刻（HH:MM 24 小时制；到点触发后任务自动删除）
        #[arg(long = "at", conflicts_with = "every")]
        at: Option<String>,
        /// 会话 id；缺省解析序：CLAUDE_CODE_SESSION_ID 环境变量回落 ~/.claude.json 项目表 lastSessionId
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 列出项目定时任务（全量并标 ours 归属，含非本会话）
    List {
        /// 会话 id（ours 归属判据；解析序同 set，失败不影响列表）
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 删除定时任务（id 精确删任意任务；latest 本会话最新；all 本会话全部）
    Del {
        /// 目标：任务 id 或 latest 或 all
        target: String,
        /// 会话 id（latest 与 all 的归属判据；解析序同 set）
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum GoalCmd {
    /// 设置当前会话最新 loop 的 goal 文本（prompt 就地改写；无本会话任务报错）
    Set {
        /// goal 文本（trim 后非空；状态栏 goal 段显示源）
        text: String,
        /// 会话 id；解析序同 loop set
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 查看当前会话最新 loop 的 goal（无任务或会话不可解析给 goal.present=false）
    Show {
        /// 会话 id；解析序同 loop set
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 清空当前会话最新 loop 的 goal（prompt 置空、任务与节奏保留）
    Clear {
        /// 会话 id；解析序同 loop set
        #[arg(long)]
        session: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum IssueCmd {
    /// 开 issue（账本 issue 流；kind=bug BUG 错误任务或 improvement 改进优化任务，缺省 bug；真源 ledger.ohmygh.com，旧 issues.ohmygh.com 过渡保役）
    New {
        /// 标题（trim 后 1 至 200 字符）
        title: String,
        /// 任务性质（bug=BUG 错误任务；improvement=改进优化任务）
        #[arg(long, default_value = "bug")]
        kind: String,
        /// 验收条件（关单 result 引 digest 即完成判据）
        #[arg(long)]
        acceptance: String,
        /// 补充说明（随开单事件）
        #[arg(long)]
        body: Option<String>,
    },
    /// 列 issue（新到旧；count 为返回条数非在册总数，打满 limit 即 stderr 出截断提示）
    List {
        /// 条数（1 至 100，缺省 100）
        #[arg(long)]
        limit: Option<u32>,
        /// 翻页游标：取该 issue 号之前更旧一页（末行号作下一页游标，has_more=false 即止）
        #[arg(long)]
        before: Option<String>,
    },
    /// 看单条 issue 详情（projection 加 timeline）
    Show {
        /// issue 号（数字）
        n: u64,
    },
}

#[derive(Subcommand)]
enum ArtifactCmd {
    /// 发布产物（共享库本体；digest=正文或记录哈希，库不收二进制实体）
    Publish {
        /// 名称（trim 后 1 至 200 字符）
        name: String,
        /// 产物性质（experience 加 lesson 加 research 加 prototype 加 binary 等十五种）
        #[arg(long)]
        kind: String,
        /// 内容哈希（sha256:<64hex 小写>；一律正文或记录哈希为身份）
        #[arg(long)]
        digest: String,
        /// 版本（tag 或版本号；实现记录类适用）
        #[arg(long)]
        version: Option<String>,
        /// 开发记录区间（如 v2.5.0..v2.6.0；实现记录类适用）
        #[arg(long = "git-range")]
        git_range: Option<String>,
        /// 依赖出处（可多次；回溯链即证据链）
        #[arg(long = "dep")]
        deps: Vec<String>,
        /// 摘要
        #[arg(long)]
        summary: Option<String>,
        /// 成败面（experience 类：success 或 failure）
        #[arg(long)]
        outcome: Option<String>,
        /// 关联 git sha（入 artifacts 表）
        #[arg(long = "git-sha")]
        git_sha: Option<String>,
        /// 说明（随 publish 事件 body）
        #[arg(long)]
        note: Option<String>,
    },
    /// 产物验证事件（只增面：attest_dev 加 attest_prod 加 verification_failed；promote/demote/supersede 与删改归 omc 工作台）
    Attest {
        /// artifact id
        id: String,
        /// 验证类型（attest_dev|attest_prod|verification_failed）
        #[arg(long)]
        r#type: String,
        /// 附加注记（进事件 body）
        #[arg(long)]
        note: Option<String>,
    },
    /// 列产物（current 投影当前有效集；count 为返回条数）
    List {
        /// 只看当前有效集
        #[arg(long)]
        current: bool,
        /// 按 env 过滤（dev 或 prod）
        #[arg(long)]
        env: Option<String>,
    },
}

#[derive(Subcommand)]
enum LedgerCmd {
    /// 生成 Ed25519 密钥对（私钥写 ~/.hst/ledger/ed25519.key 0600，不打印不进 argv；公钥 JWK 与 kid 打印供总台在册）
    Keygen {
        /// 确认覆盖在位密档（销毁旧私钥不可恢复）
        #[arg(long)]
        force: bool,
    },
    /// 本地私钥与内置公钥的配对自检（不配对则写入全体 401 难归因；密档缺位报 absent）
    Verify,
}

#[derive(Subcommand)]
enum YoloCmd {
    /// 检测项目级 yolo 干扰键（claude 两层加 codex 加 kimi，只读零改动；写 ~/.hst/state/projyolo/ marker，命中时状态栏升格 proj-yolo!；清除走 hst init --clear-project-yolo。本命令只出 kv marker 行，不走 fmtio 三态）
    Check {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum DiagnoseCmd {
    /// 网关缓存探测（打真 API、烧最小 token；探测错误退出 1）
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
        /// 翻页偏移（向更早翻页）
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
        /// 翻页偏移（向更早翻页）
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
        /// 翻页偏移（向更早翻页）
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 意图操作块视图（按 operation_id 归组的操作块清单）
    Blocks {
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000，取最新 N 块）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 翻页偏移（向更早翻页）
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
        /// 翻页偏移（向更早翻页）
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
        /// 项目根（项目面退役与说明层部署用；hook 注册本身用户级）
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 状态写入入口（事件参数或 stdin JSON，落 ~/.hst/state/）
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
    // `hst --llms`（REQ-060 更正后族标准名）：裸出 markdown 手册（帮助面
    // 同款直打 stdout）；配 --json 出机器形态 {name,version,description,
    // commands[]}。活命令树渲染，禁手维护双份（ADR-0005/D54）。filter 与
    // llms 的互斥不能走 clap conflicts_with（global 参数在子命令面无对端
    // 可指，debug_asserts 必炸），在此单点守卫。
    if cli.llms {
        if cli.filter_output.is_some() {
            return Err("--filter-output 只作用于 json 信封，不配 --llms".into());
        }
        if cli.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&llm_machine_form(&Cli::command()))
                    .map_err(|e| e.to_string())?
            );
        } else {
            println!("{}", render_llms(&Cli::command()));
        }
        return Ok(());
    }
    hst::fmtio::init(
        cli.json,
        cli.format.as_deref(),
        cli.filter_output.as_deref(),
    )?;
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
            clear_project_yolo,
            pretrust,
            compact_pct,
            yes,
            project,
        } => cmd_init(
            yolo,
            project_yolo,
            clear_project_yolo,
            pretrust,
            compact_pct,
            yes,
            project,
        ),
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
        Commands::Artifact { cmd } => cmd_artifact(cmd),
        Commands::Ledger { cmd } => match cmd {
            LedgerCmd::Keygen { force } => cmd_ledger_keygen(force),
            LedgerCmd::Verify => cmd_ledger_verify(),
        },
        Commands::Yolo { cmd } => match cmd {
            YoloCmd::Check { project } => cmd_yolo_check(project),
        },
        Commands::Diagnose { cmd } => cmd_diagnose(cmd),
        Commands::Loop { cmd } => match cmd {
            LoopCmd::Set {
                goal,
                every,
                at,
                session,
                project,
            } => cmd_loop_set(
                &goal,
                every.as_deref(),
                at.as_deref(),
                session.as_deref(),
                project,
            ),
            LoopCmd::List { session, project } => cmd_loop_list(session.as_deref(), project),
            LoopCmd::Del {
                target,
                session,
                project,
            } => cmd_loop_del(&target, session.as_deref(), project),
        },
        Commands::Goal { cmd } => match cmd {
            GoalCmd::Set {
                text,
                session,
                project,
            } => cmd_goal_set(&text, session.as_deref(), project),
            GoalCmd::Show { session, project } => cmd_goal_show(session.as_deref(), project),
            GoalCmd::Clear { session, project } => cmd_goal_clear(session.as_deref(), project),
        },
        Commands::Issue { cmd } => cmd_issue(cmd),
    }
}

/// `hst --llms`：紧凑版 agent 说明书（REQ-060 族标准面，总长至多 120 行）。
/// 名加版本加一句定位、读序、子命令表（组递归到叶）、通用旗标、退出码、
/// 输出契约、常用例；命令表从 clap 活命令树自适应渲染（新命令自动出现，
/// 禁手维护双份）；不落盘、不装技能（ADR-0005 后唯一机读手册面）。本面
/// 是速查投影，契约在 clap 帮助与集成测试。
fn render_llms(root: &clap::Command) -> String {
    let mut rows: Vec<(String, String)> = Vec::new();
    walk(root, String::new(), &mut rows);
    let mut table = String::new();
    for (usage, about) in &rows {
        let about = if about.is_empty() {
            "（见子命令）".into()
        } else {
            about.clone()
        };
        table.push_str(&format!("| `{usage}` | {about} |\n"));
    }
    format!(
        "# hst {ver}\n\n> HST（Hooks, Statusline, Trace）：agent 全平台部署配置与诊断 CLI（四家 hook 落盘、状态栏、只读对话 trace、可用性诊断、yolo 分级）。手册由活命令树渲染；契约以 clap 帮助与集成测试为准。\n\n## 读序\n\n常见任务直达：部署 `hst init`、体检 `hst doctor`、查文件谁改的 `hst trace file <文件>`。本手册机器形：`hst --llms --json`。契约权威：`hst --help` 与集成测试。\n\n## 子命令表\n\n| 命令 | 说明 |\n| --- | --- |\n{table}\n## 通用旗标\n\n| 旗标 | 说明 |\n| --- | --- |\n| `--format kv\\|json\\|jsonl` | 输出三态（kv 是缺省 marker 行，json 出信封，jsonl 逐行对象） |\n| `--json` | `--format json` 简写（信封形） |\n| `--filter-output <keys>` | json 信封 data 键路径过滤（仅出信封命令生效；点号嵌套、数组下标如 items[0,2]，响错不静默截断） |\n| `--llms` | 本手册；配 `--json` 出机器形态（REQ-060 族标准） |\n| `--help` / `--version` | 帮助与版本 |\n\n## 退出码\n\n| 码 | 义 |\n| --- | --- |\n| 0 | 成功（裸 hst 打印帮助亦退 0） |\n| 1 | 业务失败与启动期旗标校验错（doctor blocked、verify 失败、运行错误、--filter-output 配对与响错） |\n| 2 | 用法错误（clap 解析级）；secretguard 拦截（hook 面） |\n\n## 输出契约\n\n结构化错误 stderr 单行 JSON；json 信封 meta 带 duration_ms。\n\n## 常用例\n\n```bash\nhst init                     # 全套部署（幂等）：yolo 键加 hook 加状态栏\nhst doctor                   # 零网络只读体检（block 才退 1）\nhst trace file src/main.rs   # 单文件谁改的、为什么\nhst --json --filter-output blocked doctor   # 信封只留 blocked 键\nhst --llms --json            # 机器形手册（agent 面）\nhst issue new \"发现缺陷\" --body \"复现步骤\"   # 一键反馈（issues.ohmygh.com）\n```\n",
        ver = env!("CARGO_PKG_VERSION"),
        table = table,
    )
}

/// `hst --llms --json` 的机器形态（REQ-060）：{name, version, description,
/// commands:[{name, description, subcommands?}]}，活命令树递归，组到叶。
fn llm_machine_form(root: &clap::Command) -> Value {
    fn cmd_node(cmd: &clap::Command) -> Value {
        let name = cmd.get_name().to_string();
        let description = cmd.get_about().map(|s| s.to_string()).unwrap_or_default();
        let subs: Vec<Value> = cmd
            .get_subcommands()
            .filter(|s| s.get_name() != "help")
            .map(cmd_node)
            .collect();
        let mut v = serde_json::json!({ "name": name, "description": description });
        if !subs.is_empty() {
            v["subcommands"] = Value::Array(subs);
        }
        v
    }
    let subs: Vec<Value> = root
        .get_subcommands()
        .filter(|s| s.get_name() != "help")
        .map(cmd_node)
        .collect();
    serde_json::json!({
        "name": "hst",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "HST（Hooks, Statusline, Trace）：agent 全平台部署配置与诊断 CLI",
        "commands": subs,
    })
}

/// 递归收集 (usage, about)。父命令可裸调（如 `hst agents`）时也记一行。
fn walk(cmd: &clap::Command, prefix: String, rows: &mut Vec<(String, String)>) {
    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        let path = if prefix.is_empty() {
            format!("hst {}", sub.get_name())
        } else {
            format!("{prefix} {}", sub.get_name())
        };
        let about = sub.get_about().map(|s| s.to_string()).unwrap_or_default();
        rows.push((synopsis(sub, &path), about));
        if sub.has_subcommands() {
            walk(sub, path, rows);
        }
    }
}

/// 命令表用法串拼装：路径加位置参数加可选旗标（全局与隐藏旗标不入串）。
fn synopsis(cmd: &clap::Command, path: &str) -> String {
    let mut s = String::from(path);
    let mut opts: Vec<String> = Vec::new();
    for a in cmd.get_arguments() {
        if a.is_global_set() || a.is_hide_set() {
            continue;
        }
        let id = a.get_id().as_str();
        if id == "help" || id == "version" {
            continue;
        }
        if a.is_positional() {
            let name = a
                .get_value_names()
                .and_then(|v| v.first())
                .map(|n| n.to_string())
                .unwrap_or_else(|| id.to_string());
            if matches!(a.get_action(), clap::ArgAction::Append) {
                s.push_str(&format!(" [{name}]..."));
            } else {
                s.push_str(&format!(" <{name}>"));
            }
        } else if let Some(long) = a.get_long() {
            let name = a
                .get_value_names()
                .and_then(|v| v.first())
                .map(|n| n.to_string());
            if matches!(
                a.get_action(),
                clap::ArgAction::Set | clap::ArgAction::Append
            ) {
                // 裸旗标合法的取值旗标（num_args 下界 0，如 --yolo）出
                // `[--yolo[=full|partial|off]]`（有枚举值列值集，与裸旗标
                // 取值语义对齐，codex 评审 G2）；必值旗标仍 `[--script <路径>]`。
                let optional_value = a
                    .get_num_args()
                    .map(|r| r.min_values() == 0)
                    .unwrap_or(false);
                if optional_value {
                    let possible = a.get_possible_values();
                    let inner = if !possible.is_empty() {
                        possible
                            .iter()
                            .map(|v| v.get_name().to_string())
                            .collect::<Vec<_>>()
                            .join("|")
                    } else {
                        name.clone().unwrap_or_else(|| long.to_string())
                    };
                    opts.push(format!("[--{long}[={inner}]]"));
                } else {
                    let val = name.map(|n| format!(" <{n}>")).unwrap_or_default();
                    opts.push(format!("[--{long}{val}]"));
                }
            } else {
                opts.push(format!("[--{long}]"));
            }
        }
    }
    for o in opts {
        s.push(' ');
        s.push_str(&o);
    }
    s
}

/// `hst issue new|list|show`：账本 issue 流（REQ-063，真源
/// ledger.ohmygh.com；只增面，close/status 推进归 omc 工作台）。kv 出
/// marker 行，json 加 jsonl 走 fmtio 三态。
fn cmd_issue(cmd: IssueCmd) -> Result<(), String> {
    match cmd {
        IssueCmd::New {
            title,
            kind,
            acceptance,
            body,
        } => {
            let title_trim = title.trim();
            if title_trim.is_empty() || title_trim.chars().count() > 200 {
                return Err(format!(
                    "title 必填且至多 200 字符（trim 后），得 {}",
                    title_trim.chars().count()
                ));
            }
            if !hst::ledger::ISSUE_KINDS.contains(&kind.as_str()) {
                return Err(format!("kind 仅 bug|improvement，得 {kind}"));
            }
            let n = hst::ledger::client()?
                .issue_new(title_trim, &kind, &acceptance, body.as_deref())
                .map_err(|e| e.to_string())?;
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    print_json(
                        "issue-new",
                        &cwd,
                        Ok(serde_json::json!({ "filed": true, "n": n })),
                    )?;
                }
                hst::fmtio::Format::Jsonl => {
                    hst::fmtio::print_jsonl(&[serde_json::json!({ "filed": true, "n": n })]);
                }
                hst::fmtio::Format::Kv => {
                    println!("ledger.issue.opened=n{n}");
                }
            }
            Ok(())
        }
        IssueCmd::List { limit, before } => {
            // 家族标准（#52/#53）：默认 100、打满即 stderr 截断提示、count
            // 为返回条数；账本 has_more 由 more=1 恒在（权威信号）。
            let eff = hst::ledger::clamp_issue_limit(limit.unwrap_or(100));
            let before_n = match before.as_deref() {
                Some(b) if !b.trim().is_empty() => Some(
                    b.trim()
                        .parse::<u64>()
                        .map_err(|_| format!("before 须数字 issue 号，得 {b}"))?,
                ),
                _ => None,
            };
            let v = hst::ledger::client_readonly()?
                .issue_list(eff, before_n)
                .map_err(|e| e.to_string())?;
            let rows = v["issues"].as_array().cloned().unwrap_or_default();
            let saturated = v["has_more"]
                .as_bool()
                .unwrap_or_else(|| hst::ledger::issue_list_saturated(rows.len(), eff));
            if saturated {
                eprintln!("{}", hst::ledger::issue_list_truncation_hint(eff));
            }
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    let mut payload = serde_json::json!({ "count": rows.len(), "issues": rows });
                    if let Some(hm) = v["has_more"].as_bool() {
                        payload["has_more"] = serde_json::json!(hm);
                    }
                    print_json("issue-list", &cwd, Ok(payload))?;
                }
                hst::fmtio::Format::Jsonl => hst::fmtio::print_jsonl(&rows),
                hst::fmtio::Format::Kv => {
                    println!("issue.list.count={}", rows.len());
                    if let Some(hm) = v["has_more"].as_bool() {
                        println!("issue.list.has_more={hm}");
                    }
                    for r in &rows {
                        println!(
                            "issue.row n={} kind={} status={} assignee={} has_result={} title={}",
                            r["issue_n"],
                            r["kind"].as_str().unwrap_or("-"),
                            r["status"].as_str().unwrap_or("-"),
                            r["assignee"].as_str().unwrap_or("-"),
                            r["hasResult"],
                            r["title"].as_str().unwrap_or("-"),
                        );
                    }
                }
            }
            Ok(())
        }
        IssueCmd::Show { n } => {
            let v = hst::ledger::client_readonly()?
                .issue_show(n)
                .map_err(|e| e.to_string())?;
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    print_json("issue-show", &cwd, Ok(v.clone()))?;
                }
                hst::fmtio::Format::Jsonl => {
                    hst::fmtio::print_jsonl(&[v.clone()]);
                }
                hst::fmtio::Format::Kv => {
                    for line in render_issue_show_kv(&v) {
                        println!("{line}");
                    }
                }
            }
            Ok(())
        }
    }
}

/// issue 详情的 kv 行（projection 加时间线；title 与 acceptance 在
/// timeline 的 issue_open payload 里，服务端 projection 只回状态面——评审
/// F1）。payload 是嵌套 JSON 字符串，顺手解成对象入行。
fn render_issue_show_kv(v: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let p = &v["projection"];
    let open = v["timeline"]
        .as_array()
        .and_then(|tl| tl.iter().find(|ev| ev["type"] == "issue_open"))
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let payload: serde_json::Value = open["payload"]
        .as_str()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    out.push(format!("issue.n={}", v["issue"]));
    out.push(format!(
        "issue.title={}",
        payload["title"].as_str().unwrap_or("-")
    ));
    out.push(format!(
        "issue.kind={}",
        payload["kind"].as_str().unwrap_or("-")
    ));
    out.push(format!(
        "issue.status={}",
        p["status"].as_str().unwrap_or("-")
    ));
    out.push(format!(
        "issue.acceptance={}",
        payload["acceptance"].as_str().unwrap_or("-")
    ));
    if let Some(a) = p["assignee"].as_str() {
        out.push(format!("issue.assignee={a}"));
    }
    if let Some(tl) = v["timeline"].as_array() {
        out.push(format!("issue.timeline.count={}", tl.len()));
        for ev in tl {
            out.push(format!(
                "issue.event seq={} type={} at={}",
                ev["seq"],
                ev["type"].as_str().unwrap_or("-"),
                ev["created_at"],
            ));
        }
    }
    out
}

/// `hst artifact publish|attest|list`：账本 artifact 流（REQ-063，只增面）。
fn cmd_artifact(cmd: ArtifactCmd) -> Result<(), String> {
    match cmd {
        ArtifactCmd::Publish {
            name,
            kind,
            digest,
            version,
            git_range,
            deps,
            summary,
            outcome,
            git_sha,
            note,
        } => {
            let name_trim = name.trim();
            if name_trim.is_empty() || name_trim.chars().count() > 200 {
                return Err(format!(
                    "name 必填且至多 200 字符（trim 后），得 {}",
                    name_trim.chars().count()
                ));
            }
            if !ledger_client::ARTIFACT_KINDS.contains(&kind.as_str()) {
                return Err(format!(
                    "kind 仅 {}，得 {kind}",
                    ledger_client::ARTIFACT_KINDS.join("|")
                ));
            }
            hst::ledger::validate_digest(&digest)?;
            let id = hst::ledger::client()?
                .artifact_publish_full(
                    name_trim,
                    &kind,
                    &digest,
                    version.as_deref(),
                    git_range.as_deref(),
                    &deps,
                    note.as_deref(),
                    summary.as_deref(),
                    outcome.as_deref(),
                    git_sha.as_deref(),
                )
                .map_err(|e| e.to_string())?;
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    print_json(
                        "artifact-publish",
                        &cwd,
                        Ok(serde_json::json!({ "published": true, "artifact_id": id })),
                    )?;
                }
                hst::fmtio::Format::Jsonl => {
                    hst::fmtio::print_jsonl(&[
                        serde_json::json!({ "published": true, "artifact_id": id }),
                    ]);
                }
                hst::fmtio::Format::Kv => {
                    println!("ledger.artifact.published={id}");
                }
            }
            Ok(())
        }
        ArtifactCmd::Attest { id, r#type, note } => {
            if !ledger_client::ATTEST_TYPES.contains(&r#type.as_str()) {
                return Err(format!(
                    "type 仅 {}（只增验证面；promote/demote/supersede 归 omc 工作台），得 {}",
                    ledger_client::ATTEST_TYPES.join("|"),
                    r#type
                ));
            }
            let v = hst::ledger::client()?
                .artifact_attest(&id, &r#type, serde_json::json!({}), note.as_deref())
                .map_err(|e| e.to_string())?;
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    print_json("artifact-attest", &cwd, Ok(v.clone()))?;
                }
                hst::fmtio::Format::Jsonl => hst::fmtio::print_jsonl(&[v.clone()]),
                hst::fmtio::Format::Kv => {
                    println!("ledger.event.seq={}", v["event"]["seq"].clone());
                    println!("ledger.event.type={}", v["event"]["type"].clone());
                }
            }
            Ok(())
        }
        ArtifactCmd::List { current, env } => {
            let v = hst::ledger::client_readonly()?
                .artifact_list(current, env.as_deref())
                .map_err(|e| e.to_string())?;
            let rows = v["artifacts"].as_array().cloned().unwrap_or_default();
            match hst::fmtio::mode() {
                hst::fmtio::Format::Json => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    print_json(
                        "artifact-list",
                        &cwd,
                        Ok(serde_json::json!({ "count": rows.len(), "artifacts": rows })),
                    )?;
                }
                hst::fmtio::Format::Jsonl => hst::fmtio::print_jsonl(&rows),
                hst::fmtio::Format::Kv => {
                    println!("artifact.list.count={}", rows.len());
                    for r in &rows {
                        println!(
                            "artifact.row id={} name={} kind={} current={} dev_verified={} prod_verified={}",
                            r["artifact_id"],
                            r["name"].as_str().unwrap_or("-"),
                            r["kind"].as_str().unwrap_or("-"),
                            r["current"],
                            r["dev_verified"],
                            r["prod_verified"],
                        );
                    }
                }
            }
            Ok(())
        }
    }
}

/// `hst ledger keygen`：密钥对生成（私钥落密档，公钥 JWK 与 kid 打印）。
fn cmd_ledger_keygen(force: bool) -> Result<(), String> {
    let (kid, jwk, old_kid) = hst::ledger::keygen_write(force)?;
    if let Some(old) = old_kid {
        println!("ledger.keygen.old_kid={old}");
    }
    println!("ledger.keygen.kid={kid}");
    println!("ledger.keygen.jwk={jwk}");
    println!(
        "ledger.keygen.private_key={}",
        hst::ledger::private_key_path()?.display()
    );
    println!("ledger.keygen.hint=公钥 JWK 与 kid 供总台在册（在册后方可写入）");
    Ok(())
}

/// `hst ledger verify`：本地私钥与内置公钥配对自检。
fn cmd_ledger_verify() -> Result<(), String> {
    match hst::ledger::pairing_ok()? {
        Some(true) => {
            println!("ledger.pair=ok kid={}", hst::ledger::key_id());
            Ok(())
        }
        Some(false) => {
            println!("ledger.pair=mismatch kid={}", hst::ledger::key_id());
            println!("ledger.pair.hint=本地私钥与内置公钥 JWK 不配对（写入会全体 401）；轮换密钥对后需同步换 CLI 内置常量并在册新 kid");
            Ok(())
        }
        None => {
            println!("ledger.pair=absent kid={}", hst::ledger::key_id());
            println!("ledger.pair.hint=私钥密档或 env 缺位；hst ledger keygen 生成");
            Ok(())
        }
    }
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

/// completions 面：生成 shell 补全脚本到 stdout。
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

/// `hst hook init`：hook 面部署（注册加 shim 加 heal 迁移改写）；说明面
/// 与技能目录清扫不在此（那是 `hst init` 全套的事）。
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
    clear_project_yolo: bool,
    pretrust: bool,
    compact_pct: Option<String>,
    yes: bool,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    // 压缩触发面（总台功能单 2026-09-23，ledger n6）：仅 compact 面的
    // scoped 旗标（同 --yolo 先例，与 yolo 族互斥由 clap 保证）。预览
    // （缺省，零写）加 --yes 落盘加回读自证；键面实证见 compact 模块文档。
    if let Some(spec) = compact_pct {
        let home = hst::pathutil::user_home()?;
        let settings = home.join(".claude").join("settings.json");
        let spec_trim = spec.trim();
        let off = spec_trim.eq_ignore_ascii_case("off");
        let pct = if off {
            None
        } else {
            let p: u8 = spec_trim
                .parse()
                .map_err(|_| format!("invalid --compact-pct (expect 1-100 or off): {spec}"))?;
            if !(1..=100).contains(&p) {
                return Err(format!(
                    "invalid --compact-pct (expect 1-100 or off): {spec}"
                ));
            }
            Some(p)
        };
        if yes {
            let changed = hst::compact::apply_pct(&home, pct)?;
            if changed.is_empty() {
                println!("compact.write=none (already at target, idempotent)");
            } else {
                for c in &changed {
                    println!("compact.write file={c}");
                }
            }
            // 回读自证：落盘态必须与目标态等值。
            let st = hst::compact::read_state(&home)?;
            let want = pct.map(|p| p.to_string());
            if st.pct_raw != want {
                return Err(format!(
                    "compact readback mismatch: want {:?}, got {:?}",
                    want, st.pct_raw
                ));
            }
            match pct {
                Some(p) => println!("compact.readback pct={p}"),
                None => println!("compact.readback pct=unset"),
            }
            if let Some((thr, by_pct)) = st.threshold_approx() {
                println!(
                    "compact.threshold tokens={thr} by_pct={by_pct} (输出预留按 20000 上限近似,窗口 {})",
                    st.effective_window().unwrap_or(0)
                );
            }
        } else {
            match pct {
                Some(p) => println!(
                    "compact.plan key={} value={p} file={}",
                    hst::compact::ENV_PCT,
                    settings.display()
                ),
                None => println!(
                    "compact.plan key={} remove=true file={}",
                    hst::compact::ENV_PCT,
                    settings.display()
                ),
            }
            println!("compact.preview=true (zero-write; add --yes to apply)");
        }
        println!("init.hooks=skipped");
        println!("init.scope=compact");
        return Ok(());
    }
    // Default init is the full deployment: user-level yolo keys plus user-level
    // hook registration, statusline and ours skill-dir retirement (D28 round 2:
    // yolo keys are user-level too; ADR-0005: skill face retired). Round 3:
    // yolo scope is explicit — `--yolo` = user-level
    // keys only, `--project-yolo` = project-level keys only (project overrides
    // user where the agent supports layering). D33: both yolo flags are
    // level-taking (full|partial|off, bare flag = full); off retires ours keys
    // at the chosen scope instead of writing.
    let keys_only = yolo.is_some();
    // REQ-009/D55：--clear-project-yolo = 项目级干扰一键清除（keys-only，
    // 与两级 yolo 旗标互斥由 clap 保证）。家目录守卫同 --project-yolo
    //（D52）：root 是家目录时项目层即用户层文件本体，整支跳过打点。
    if clear_project_yolo {
        println!("init.flag.clear_project_yolo=true");
        let home = hst::pathutil::user_home()?;
        if hst::pathutil::same_location(&root, &home) {
            println!(
                "init.warn=clear-project-yolo skipped: root is the user home \
                 (home is not a project; user-level keys stay)"
            );
        } else {
            for p in hst::yolo::clear_project_yolo_interference(&root)? {
                println!("init.retired={p}");
            }
        }
        println!("init.hooks=skipped");
    } else if let Some(level) = project_yolo {
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
        if clear_project_yolo {
            "clear-project-yolo"
        } else if project_yolo.is_some() {
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

fn cmd_loop_set(
    goal: &str,
    every: Option<&str>,
    at: Option<&str>,
    session: Option<&str>,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    // REQ-021：互斥检查归 CLI 解析层（lib 面由 Cadence 枚举在类型层
    // 不可表示双缺/双给）。
    let cadence = match (every, at) {
        (Some(e), None) => loopmgmt::Cadence::Every(e),
        (None, Some(a)) => loopmgmt::Cadence::At(a),
        _ => {
            return Err(
                "exactly one of --every / --at is required (they are mutually exclusive)"
                    .to_string(),
            )
        }
    };
    let r = loopmgmt::set_loop(&root, goal, cadence, session)
        .map_err(|e| format!("loop error={}: {e}", e.code()))?;
    println!("loop.set id={}", r.id);
    println!("loop.set cron={}", r.cron);
    println!("loop.set recurring={}", r.recurring);
    println!("loop.set session={}", r.session);
    println!("loop.set file={}", r.file.display());
    println!(
        "loop.hint=durable 任务由 Claude Code 会话载入执行（会话启动时载入；已在跑会话不接管盘上外部写入，2026-09-26 实证阴性，写入后重开会话生效）"
    );
    Ok(())
}

fn cmd_loop_list(session: Option<&str>, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let (rows, resolved) = loopmgmt::list_tasks(&root, session)
        .map_err(|e| format!("loop error={}: {e}", e.code()))?;
    println!("loop.count={}", rows.len());
    if let Some(s) = resolved {
        println!("loop.session={s}");
    }
    for t in rows {
        println!(
            "loop.task id={} cron=\"{}\" recurring={} session={} ours={} goal={}",
            t.id, t.cron, t.recurring, t.session, t.ours, t.goal
        );
    }
    Ok(())
}

fn cmd_loop_del(
    target: &str,
    session: Option<&str>,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let removed = loopmgmt::del_loops(&root, target, session)
        .map_err(|e| format!("loop error={}: {e}", e.code()))?;
    println!("loop.del count={}", removed.len());
    for id in removed {
        println!("loop.del id={id}");
    }
    Ok(())
}

fn cmd_goal_set(text: &str, session: Option<&str>, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let r = loopmgmt::set_goal(&root, text, session)
        .map_err(|e| format!("goal error={}: {e}", e.code()))?;
    println!("goal.set id={}", r.id);
    println!("goal.set cron={}", r.cron);
    println!("goal.set text={}", text.trim());
    Ok(())
}

fn cmd_goal_show(session: Option<&str>, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    match loopmgmt::show_goal(&root, session)
        .map_err(|e| format!("goal error={}: {e}", e.code()))?
    {
        Some(t) => {
            println!("goal.present=true");
            println!("goal.id={}", t.id);
            println!("goal.cron={}", t.cron);
            println!("goal.text={}", t.goal);
        }
        None => println!("goal.present=false"),
    }
    Ok(())
}

fn cmd_goal_clear(session: Option<&str>, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    match loopmgmt::clear_goal(&root, session)
        .map_err(|e| format!("goal error={}: {e}", e.code()))?
    {
        Some(id) => println!("goal.clear id={id}"),
        None => println!("goal.clear count=0"),
    }
    Ok(())
}

fn cmd_yolo_check(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let hits = yolo::project_yolo_interferences(&root)?;
    println!("projyolo.hit={}", !hits.is_empty());
    println!("projyolo.count={}", hits.len());
    for h in &hits {
        println!("projyolo.key={h}");
    }
    if !hits.is_empty() {
        println!("projyolo.hint=hst init --clear-project-yolo 清除（本命令零改动）");
    }
    // marker：状态栏哨兵读（slug 规则同 trace——路径非字母数字一律 -；
    // project 原样透传不经 canonicalize，与状态栏侧字符串等值匹配）。
    let marker_dir = install::hst_home()?.join("state").join("projyolo");
    std::fs::create_dir_all(&marker_dir).map_err(|e| format!("{}: {e}", marker_dir.display()))?;
    let slug: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // real 字段加双 slug 落盘（评审 G1/G6）：canonicalize 形与给定形各落
    // 一份同容 marker，状态栏侧任一拼写（符号链接漂移、手动 CTA 的物理
    // 路径）都能命中。
    let real = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    let real_slug: String = real
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let marker = serde_json::json!({
        "hit": !hits.is_empty(),
        "project": root.display().to_string(),
        "real": real.display().to_string(),
        "count": hits.len(),
        "ts": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    let body = marker.to_string();
    std::fs::write(marker_dir.join(format!("{slug}.json")), &body)
        .map_err(|e| format!("write marker: {e}"))?;
    if real_slug != slug {
        std::fs::write(marker_dir.join(format!("{real_slug}.json")), &body)
            .map_err(|e| format!("write marker (real): {e}"))?;
    }
    Ok(())
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
            // --filter-output 未命中时信封折成错误：与 print_json 同道走
            // stderr 单行加退出 1（emit_trace 无返回值，就地收口）。
            if !env.get("ok").and_then(|v| v.as_bool()).unwrap_or(true) {
                let msg = env
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("command failed")
                    .to_string();
                hst::fmtio::error_exit(msg);
            }
        }
    }
}

/// 操作块时间线面：按 operation_id 归组渲染，正序翻页。
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
