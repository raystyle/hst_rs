//! hst 自适应 SKILL 渲染（D22）：从 clap 活命令树生成 SKILL.md（Agent
//! Skills 标准形态：frontmatter name 加 description 含何时用）。新命令/
//! 新旗标自动出现在生成物里，不需要手维护命令表；`hst skill` 打印、
//! `hst skill --write` 落用户级 `~/.claude/skills/hst/`（D49 起技能名翻
//! hst；旧牌 ohmyagents 不留兼容窗，第 2 轮用户裁定直接删除加幂等清扫）。
//! 项目级 init 生成物（deploy.rs COMMAND_MAP 加 marker 覆写语义）是另一
//! 层，不共用渲染器（lib 拿不到 bin 的 Cli 树，双层记档于 R002）。

use clap::Command as ClapCommand;

/// 技能生成的D49面（细则见 R002 与模块文档）。
pub const SKILL_NAME: &str = "hst";

/// 渲染完整 SKILL.md（canonical，name = hst）。
pub fn render_skill(root: &ClapCommand) -> String {
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
        "---\nname: hst\ndescription: hst 部署配置与诊断 CLI 的自适应命令速查：agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置与活性诊断。要在项目里查 agent 对话历史、体检部署形态、配置状态栏或探测网关缓存时使用。\n---\n\n# hst 命令速查\n\n> 本文件由 `hst skill` 从 hst 活命令树自适应生成（含子命令与旗标）；hst 升级后跑 `hst skill --write` 同步，勿手改。\n\n## 功能面\n\n- **可用性诊断**：`hst doctor`（零网络只读体检）、`hst agents`（四家检测）、`hst diagnose cache|agents`（活性诊断，打真网关烧最小 token）\n- **hook 设置**：`hst init`（部署，幂等）、`hst hook status`（状态落盘加密钥拦截）\n- **状态栏设置**：`hst statusline`（四家写入面加用户级定制）\n- **对话 trace**：`hst trace` 六视图只读检索四家原生会话库\n- **yolo 不阻塞设置**：`hst init --yolo`（项目级无阻塞键）\n\n## 命令表\n\n| 命令 | 说明 |\n| --- | --- |\n{table}\n## 输出契约\n\n全部命令支持 `--format kv|json|jsonl` 与 `--json` 信封（kv 是缺省 marker 行）；结构化错误 stderr 单行 JSON；doctor blocked 与 verify fail 退出 1，diagnose cache 探测错误退出 1。\n\n## 快速上手\n\n```powershell\nhst init                              # 进项目后一次性部署（幂等）\nhst doctor                            # 体检\nhst trace file <文件>                 # 这文件谁改的、为什么\nhst statusline --example       # 状态栏定制模板\nhst diagnose cache <别名>             # 网关缓存探测\nhst skill --write                     # 本技能自适应再生\n```\n"
    )
}

/// 递归收集 (usage, about)。父命令可裸调（如 `hst agents`）时也记一行。
fn walk(cmd: &ClapCommand, prefix: String, rows: &mut Vec<(String, String)>) {
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

/// 技能生成的用法串面（细则见 R002 与模块文档）。
fn synopsis(cmd: &ClapCommand, path: &str) -> String {
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
            let val = a
                .get_value_names()
                .and_then(|v| v.first())
                .map(|n| format!(" <{n}>"))
                .unwrap_or_default();
            if matches!(
                a.get_action(),
                clap::ArgAction::Set | clap::ArgAction::Append
            ) {
                opts.push(format!("[--{long}{val}]"));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 独立小命令树验证渲染形状（期望值来自 SKILL 标准与本渲染约定）。
    #[test]
    fn renders_frontmatter_and_adaptive_rows() {
        let tree = ClapCommand::new("hst")
            .subcommand(
                ClapCommand::new("agents")
                    .about("检测四家 agent")
                    .subcommand(
                        ClapCommand::new("statusline")
                            .about("配置状态栏")
                            .arg(
                                clap::Arg::new("names")
                                    .value_name("名")
                                    .num_args(1..)
                                    .action(clap::ArgAction::Append),
                            )
                            .arg(
                                clap::Arg::new("example")
                                    .long("example")
                                    .action(clap::ArgAction::SetTrue),
                            )
                            .arg(
                                clap::Arg::new("script")
                                    .long("script")
                                    .value_name("路径")
                                    .action(clap::ArgAction::Set),
                            ),
                    ),
            )
            .subcommand(ClapCommand::new("doctor").about("只读体检"));
        let md = render_skill(&tree);
        // D49：技能名翻 hst，唯一名无兼容窗（第 2 轮裁定直接删除旧牌）。
        assert!(md.starts_with("---\nname: hst\ndescription: hst "));
        assert!(md.contains(
            "| `hst agents statusline [名]... [--example] [--script <路径>]` | 配置状态栏 |"
        ));
        assert!(md.contains("| `hst doctor` | 只读体检 |"));
        assert!(md.contains("勿手改"));
        // help 不入表。
        assert!(!md.contains("`hst help`"));
    }

    #[test]
    fn new_subcommands_appear_without_hand_edits() {
        // 自适应判据：树加了子命令，命令表即出现该行。
        let tree = ClapCommand::new("hst").subcommand(ClapCommand::new("doctor").about("体检"));
        assert!(!render_skill(&tree).contains("| `oma verify`"));
        let tree = tree.subcommand(ClapCommand::new("verify").about("验收"));
        assert!(render_skill(&tree).contains("| `hst verify`"));
    }
}
