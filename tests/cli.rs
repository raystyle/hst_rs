//! CLI smoke tests for the read-only / deploy commands (integration layer).
//! Assertions stick to stable surfaces only: exit codes and marker lines.
//! D15 后本仓是纯部署配置工具：编排面（spawn/status/send/serve/mcp）
//! 与其 rmux 闸门测试已随删除面一并移除；trace（只读检索，与 rmux 无耦合）
//! 经 D19 全量恢复。

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

/// Unique per-call suffix: same-millisecond parallel tests must not share a
/// temp dir.
static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn hst() -> Command {
    Command::cargo_bin("hst").unwrap()
}

#[test]
fn help_lists_the_deploy_surface() {
    // 命令面契约（D15 收窄、D19 恢复 trace）：帮助里是部署配置面加只读检索面。
    let out = hst()
        .args(["--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    for cmd in [
        "init",
        "doctor",
        "agents",
        "hook",
        "self",
        "completions",
        "trace",
    ] {
        assert!(s.contains(cmd), "help must list {cmd}");
    }
}

#[test]
fn trace_sessions_on_empty_project_is_zero() {
    // A fresh temp project has no agent sessions: trace must exit 0 with a
    // zero count (read-only federation over the native session stores).
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-trace-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    hst()
        .args(["trace", "sessions", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("trace.sessions.count=0"));
    hst()
        .args(["trace", "timeline", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("trace.edits.count=0"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn trace_formats_and_pagination_markers() {
    // D26：trace 全视图吃 --format 三态；截断时 kv 补 has_more；sessions 吃
    // --limit；--offset 翻页可用。数据自种金档（期望不依赖宿主机的
    // 真实会话历史——CI 检出无任何 agent 数据，靠本仓历史只会本机绿）：
    // HST_TRACE_HOME 重定向会话库根到夹具，.claude/projects/<slug>/ 下三
    // 会话各两轮 Edit 工具调用（timeline 6 事件、blocks 6 块）。
    let cwd = std::env::temp_dir().join(format!(
        "hst-cli-trace-fmt-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let home = cwd.join("fake-home");
    // slug 规则来自 claude 会话库约定（trace.rs 文首）：路径非字母数字一律换 -。
    let slug: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let proj = home.join(".claude").join("projects").join(&slug);
    std::fs::create_dir_all(&proj).unwrap();
    for i in 0..3 {
        let mut body = String::new();
        for t in 0..2 {
            let user = serde_json::json!({
                "type": "user",
                "timestamp": format!("2026-09-01T10:{i}{t}:00Z"),
                "message": {"role": "user", "content": format!("seed ask {i}-{t}")},
            });
            let assistant = serde_json::json!({
                "type": "assistant",
                "timestamp": format!("2026-09-01T10:{i}{t}:05Z"),
                "message": {"role": "assistant", "content": [
                    {"type": "text", "text": format!("seed op {i}-{t}")},
                    {"type": "tool_use", "id": format!("call-{i}-{t}"), "name": "Edit",
                     "input": {
                        "file_path": cwd.join(format!("f-{i}-{t}.txt")).to_string_lossy(),
                        "new_string": "x",
                     }},
                ]},
            });
            body.push_str(&user.to_string());
            body.push('\n');
            body.push_str(&assistant.to_string());
            body.push('\n');
        }
        std::fs::write(proj.join(format!("seed-sess-{i}.jsonl")), body).unwrap();
    }
    let run = |args: &[&str]| -> String {
        // --project 钉测试侧拼写：macOS 的 TMPDIR 是符号链接（/var 到
        // /private/var），子进程 getcwd 会解析成真实路径，slug 随之漂移；
        // 显式传参与夹具同串，三平台同形。
        let out = hst()
            .current_dir(&cwd)
            .env("HST_TRACE_HOME", &home)
            .args(args)
            .arg("--project")
            .arg(&cwd)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8_lossy(&out).into_owned()
    };
    // json：信封可解析、data.items 是数组（三会话取二）。
    let json = run(&["--format", "json", "trace", "sessions", "--limit", "2"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("json envelope parses");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["count"], serde_json::json!(2));
    assert!(v["data"]["items"].as_array().unwrap().len() == 2);
    // jsonl：逐行对象可解析（六事件取二）。
    let jsonl = run(&["--format", "jsonl", "trace", "timeline", "--limit", "2"]);
    let lines: Vec<serde_json::Value> = jsonl
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("jsonl line parses"))
        .collect();
    assert_eq!(lines.len(), 2, "jsonl rows = limit");
    // kv：截断时 has_more 与 offset_next 显式标记（不再静默 clamp）。
    let s = run(&["trace", "blocks", "--limit", "2"]);
    assert!(s.contains("trace.blocks.count=2"));
    assert!(
        s.contains("trace.has_more=true"),
        "truncation must be loud: {s}"
    );
    assert!(s.contains("offset_next=2"));
    // offset 翻页：第二窗与第一窗不重叠。
    let page2 = run(&["trace", "blocks", "--limit", "2", "--offset", "2"]);
    let ops = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| l.starts_with("trace.block "))
            .map(|l| {
                l.split("op=")
                    .nth(1)
                    .unwrap()
                    .split(' ')
                    .next()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    let p1 = ops(&s);
    let p2 = ops(&page2);
    assert!(!p1.is_empty() && !p2.is_empty());
    assert!(p1.iter().all(|o| !p2.contains(o)), "pages must not overlap");
    let _ = std::fs::remove_dir_all(&cwd);
}

#[test]
fn agents_lists_detection_lines() {
    hst()
        .args(["agents"])
        .assert()
        .success()
        .stdout(contains("agent=claude"))
        .stdout(contains("agent=codex"))
        .stdout(contains("agent=grok"))
        .stdout(contains("agent=kimi"));
}

#[test]
fn hook_is_silent_without_state_env() {
    // Outside an hst session there is no HST_STATE_FILE: the hook
    // entry must stay silent and exit 0 (never fail the agent session).
    hst()
        .args(["hook", "status", "blocked"])
        .env_remove("HST_STATE_FILE")
        .assert()
        .success();
}

#[test]
fn hook_secret_guard_blocks_with_exit_2() {
    // S030：PreToolUse 命中 block 级密钥 → exit 2（agent 侧拒工具调用）。
    // token 运行时拼接构造，测试源码不落字面密钥（防线 5）。HST_ROOT 钉
    // 临时根：D28 用户级状态写不落真实家。
    let tok = format!("{}{}", "ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
    let payload = format!(
        "{{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"curl -H bearauth:{tok} https://x\"}}}}"
    );
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-hook-guard-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    hst()
        .args(["hook", "status", "--agent", "claude"])
        .env_remove("HST_STATE_FILE")
        .env_remove("HST_AGENT")
        .env("HST_ROOT", &tmp)
        .write_stdin(payload)
        .assert()
        .code(2)
        .stderr(predicates::str::contains("secretguard"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn doctor_blocks_on_a_fresh_project_and_says_so() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-doctor-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    // A fresh project has no yolo keys: doctor exits 1 by contract.
    // CPU 能力段恒在（S021）：agent=cpu check=caps。
    hst()
        .args(["doctor", "--project"])
        .arg(&tmp)
        .assert()
        .failure()
        .stdout(contains("doctor."))
        .stdout(contains("check=caps"))
        .stdout(contains("avx2="));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn issue_new_invalid_title_fails_locally() {
    // REQ-010（codex 三面评审 G4）：空标题在本地拒绝，不触网（秒红，stderr
    // 结构化错误含 1-200 提示）。
    hst()
        .args(["issue", "new", "   "])
        .assert()
        .failure()
        .stderr(contains("1-200"));
}

#[test]
fn llms_flag_prints_compact_manual() {
    // REQ-060 更正后：--llms 是族标准名，裸出 markdown 手册（名加版本加
    // 定位加子命令表加通用旗标加常用例，活命令树自适应，至多 120 行）。
    let out = hst()
        .arg("--llms")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("# hst "), "manual header with version");
    assert!(s.contains("hst init"), "command table lists init");
    assert!(s.contains("通用旗标"), "common flags section");
    assert!(s.lines().count() <= 120, "compact manual within 120 lines");
}

#[test]
fn llms_json_flag_prints_machine_form() {
    // REQ-060 更正后：--llms --json 出机器形态 {name,version,description,commands[]}。
    let out = hst()
        .args(["--llms", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out)).expect("machine form parses");
    assert_eq!(v["name"], serde_json::json!("hst"));
    assert!(v["version"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(v["commands"].as_array().is_some_and(|a| !a.is_empty()));
    let names: Vec<&str> = v["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(names.contains(&"issue") && names.contains(&"init") && names.contains(&"trace"));
}

#[test]
fn init_full_deploys_hooks_yolo_and_sweeps_skills() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    // D28 隔离缝：用户级注册面落临时家目录与 hst 根，不碰真实家。
    let user = tmp.join("fake-user-home");
    let hst_root = tmp.join("fake-hst-home");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&hst_root).unwrap();
    // ADR-0005：预置 ours 技能目录（项目面带 marker、用户级带生成签名），
    // init 全套面应清扫；外来内容不动。
    let proj = tmp.join("proj");
    std::fs::create_dir_all(proj.join(".agents").join("skills").join("hst")).unwrap();
    std::fs::write(
        proj.join(".agents")
            .join("skills")
            .join("hst")
            .join("SKILL.md"),
        "<!-- generated by hst init; rerun hst init to sync the command map -->\n",
    )
    .unwrap();
    std::fs::create_dir_all(proj.join(".grok").join("skills").join("ohmyagents")).unwrap();
    std::fs::write(
        proj.join(".grok")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md"),
        "user-owned skill\n",
    )
    .unwrap();
    std::fs::create_dir_all(user.join(".claude").join("skills").join("hst")).unwrap();
    std::fs::write(
        user.join(".claude")
            .join("skills")
            .join("hst")
            .join("SKILL.md"),
        "> 本文件由 `hst skill` 从 hst 活命令树自适应生成。\n",
    )
    .unwrap();
    hst()
        .args(["init", "--project"])
        .arg(&tmp.join("proj"))
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.scope=full"))
        .stdout(contains("init.hooks.wrote.count="))
        .stdout(contains("init.hooks.form=user"));
    // claude USER registration shape: exactly one hst handler per event, a
    // single command-line string (Grok imports this file; exec form plus
    // args is ParserError on Windows PowerShell, M047), pointing at the
    // user-level shim (D28).
    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(user.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    for (_event, groups) in settings["hooks"].as_object().unwrap() {
        let ours: Vec<&serde_json::Value> = groups
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter(|h| h["command"].as_str().is_some_and(|c| c.contains("hst")))
            .collect();
        assert_eq!(ours.len(), 1, "one hst handler per event");
        assert!(
            ours[0].get("args").is_none(),
            "Grok PowerShell ParserError if command is the exe and args follow"
        );
        assert_eq!(ours[0]["timeout"], 10);
        let cmd = ours[0]["command"].as_str().unwrap();
        assert!(
            cmd.contains("hst-state"),
            "registration points at the self-contained state shim: {cmd}"
        );
        assert!(
            !cmd.contains("/mnt/"),
            "foreign-OS path must not survive: {cmd}"
        );
    }
    // shims 常驻 hst 根 hooks/（D28）。
    assert!(hst_root.join("hooks").join("hst-state.cmd").exists());
    assert!(hst_root.join("hooks").join("hst-state.sh").exists());
    // 四家用户级注册面落齐（kimi 是 [[hooks]] 数组、codex 带信任预种）。
    for rel in [
        ".claude/settings.json",
        ".codex/hooks.json",
        ".codex/config.toml",
        ".grok/hooks/ohmyagents-state.json",
        ".kimi-code/config.toml",
    ] {
        assert!(user.join(rel).exists(), "missing user-level {rel}");
    }
    let kimi_user = std::fs::read_to_string(user.join(".kimi-code").join("config.toml")).unwrap();
    assert!(
        kimi_user.contains("hst-state"),
        "kimi user-level [[hooks]] registered (D28): {kimi_user}"
    );
    // 项目面（ADR-0005）：说明层仍在项目；skill 面退役，ours 技能目录被
    // 清扫、外来内容不动、kimi 布局目录不再预建。
    for rel in ["CLAUDE.md", "AGENTS.md"] {
        assert!(proj.join(rel).exists(), "missing project {rel}");
    }
    assert!(
        !proj.join(".agents").join("skills").join("hst").exists(),
        "ours project skill dir must be swept (ADR-0005)"
    );
    assert!(
        proj.join(".grok")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists(),
        "user-owned skill untouched"
    );
    assert!(
        !proj.join(".kimi-code").join("skills").join("hst").exists(),
        "kimi skill layout no longer pre-created"
    );
    // 用户级：`hst skill --write` 在位件（生成签名）同批清扫。
    assert!(
        !user.join(".claude").join("skills").join("hst").exists(),
        "user-level ours skill dir swept (ADR-0005)"
    );
    // D28 第 2 轮：hook 与 yolo 面全量用户级，项目 .claude/settings.json
    // 与 .codex/hooks.json 都不再创建；yolo 键落四家用户配置。
    assert!(
        !proj.join(".claude").join("settings.json").exists(),
        "project-level claude settings must not be created (D28 r2)"
    );
    assert!(
        !proj.join(".codex").join("hooks.json").exists(),
        "project-level codex hooks must not be created (D28)"
    );
    let user_claude: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(user.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        user_claude["permissions"]["defaultMode"].as_str(),
        Some("bypassPermissions"),
        "user-level yolo key written (D28 r2)"
    );
    let user_codex = std::fs::read_to_string(user.join(".codex").join("config.toml")).unwrap();
    assert!(
        user_codex.contains("approval_policy"),
        "codex user yolo keys"
    );
    // kimi 项目 config 不再被任何面写入（hook 与 yolo 都在用户级）。
    // --yolo narrows to keys only: no hook files.
    let tmp2 = std::env::temp_dir().join(format!(
        "hst-cli-init-yolo-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp2).unwrap();
    let user2 = tmp2.join("fake-user-home");
    std::fs::create_dir_all(&user2).unwrap();
    hst()
        .args(["init", "--yolo", "--project"])
        .arg(&tmp2.join("proj"))
        .env("HST_USER_HOME", &user2)
        .env("HST_ROOT", &tmp2.join("fake-hst-home"))
        .assert()
        .success()
        .stdout(contains("init.scope=yolo"))
        .stdout(contains("init.hooks=skipped"));
    assert!(
        !tmp2
            .join("proj")
            .join(".claude")
            .join("settings.json")
            .exists(),
        "--yolo writes no project files (D28 r2)"
    );
    assert!(!tmp2.join("proj").join(".codex").join("hooks.json").exists());
    let user2_claude: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(user2.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    assert!(
        user2_claude["permissions"]["defaultMode"].as_str() == Some("bypassPermissions"),
        "--yolo writes user-level yolo keys"
    );
    assert!(
        user2_claude.get("hooks").is_none(),
        "--yolo must not register hooks even at user level"
    );
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&tmp2);
}

#[test]
fn init_retires_v053_project_registrations() {
    // D28 迁移：v0.5.3 形项目（项目注册 + 项目 shim）经一次 init 退役，
    // 外来 hook 保留。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-retire-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let proj = tmp.join("proj");
    let claude = proj.join(".claude").join("settings.json");
    std::fs::create_dir_all(claude.parent().unwrap()).unwrap();
    std::fs::write(
        &claude,
        r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
            {"type": "command", "command": "C:\\tools\\fmt.sh"},
            {"type": "command", "command": "D:\\p\\.oma\\hooks\\hst-state.cmd claude"}]}]}}"#,
    )
    .unwrap();
    let shims = proj.join(".oma").join("hooks");
    std::fs::create_dir_all(&shims).unwrap();
    std::fs::write(shims.join("hst-state.cmd"), "rem generated by oma init\r\n").unwrap();
    std::fs::write(shims.join("hst-state.sh"), "# generated by oma init\n").unwrap();
    hst()
        .args(["init", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &tmp.join("user"))
        .env("HST_ROOT", &tmp.join("hst"))
        .assert()
        .success();
    // 外来 hook 存活、ours 摘除、项目 shim 删除。
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&claude).unwrap()).unwrap();
    let cmds: Vec<&str> = v["hooks"]["Stop"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["hooks"].as_array().unwrap().iter())
        .filter_map(|h| h["command"].as_str())
        .collect();
    assert_eq!(
        cmds,
        vec!["C:\\tools\\fmt.sh"],
        "ours retired, foreign kept"
    );
    assert!(!shims.join("hst-state.cmd").exists());
    assert!(!shims.join("hst-state.sh").exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_project_yolo_writes_project_scope_only() {
    // D28 第 3 轮：yolo 两级显式。--project-yolo 写项目面（claude/codex/
    // kimi 项目配置），不碰用户级；与 --yolo 互斥（退出 2）。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-pyolo-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let user = tmp.join("fake-user-home");
    let proj = tmp.join("proj");
    std::fs::create_dir_all(&user).unwrap();
    hst()
        .args(["init", "--project-yolo", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &tmp.join("fake-hst-home"))
        .assert()
        .success()
        .stdout(contains("init.scope=yolo-project"));
    let shared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(proj.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        shared["permissions"]["defaultMode"].as_str(),
        Some("bypassPermissions"),
        "project-level claude yolo written"
    );
    let codex = std::fs::read_to_string(proj.join(".codex").join("config.toml")).unwrap();
    assert!(codex.contains("approval_policy"), "project codex yolo keys");
    let kimi = std::fs::read_to_string(proj.join(".kimi-code").join("config.toml")).unwrap();
    assert!(kimi.contains("yolo"), "project kimi yolo key");
    assert!(
        !user.join(".claude").join("settings.json").exists(),
        "--project-yolo must not touch user level"
    );
    // 与 --yolo 互斥：clap 退出 2。
    hst()
        .args(["init", "--yolo", "--project-yolo", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .assert()
        .failure()
        .code(2);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_pretrust_canonical_parses_and_legacy_alias_is_removed() {
    // D32 canonical 拼写 --pre-trust；D45 oma 遗产清扫：旧 --pretrust 隐藏
    // 别名已删（1.1.0 窗口已过），现在应被 clap 拒。
    // kv 标记 init.pretrust.* 不随拼写变（机器面冻结，ohmycloud 消费）。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-pretrust-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let user = tmp.join("fake-user-home");
    let proj = tmp.join("proj");
    std::fs::create_dir_all(&user).unwrap();
    hst()
        .args(["init", "--pre-trust", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &tmp.join("fake-hst-home"))
        .assert()
        .success()
        .stdout(contains("init.pretrust=wrote"));
    // 信任库真落用户家（claude.json 双 hasTrust* 键）。
    let cj: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(user.join(".claude.json")).unwrap()).unwrap();
    assert!(
        cj["projects"]
            .as_object()
            .is_some_and(|p| p.values().any(|e| {
                e["hasTrustDialogAccepted"].as_bool() == Some(true)
                    && e["hasTrustDialogHooksAccepted"].as_bool() == Some(true)
            })),
        "pretrust wrote both claude trust keys: {cj}"
    );
    // 旧拼写已被删（D45）：clap 拒之。
    hst()
        .args(["init", "--pretrust", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &tmp.join("fake-hst-home"))
        .assert()
        .failure()
        .code(2);
    // help 面只露 canonical 拼写。
    hst()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(contains("--pre-trust"))
        .stdout(predicates::str::contains("--pretrust").not());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_clear_project_yolo_strips_interference() {
    // REQ-009/D55：--clear-project-yolo 一键清项目级干扰键（ours 与外来
    // 都收），keys-only 不部署 hook；与 --yolo 互斥。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-clear-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let user = tmp.join("fake-user-home");
    let hst_root = tmp.join("fake-hst-home");
    let proj = tmp.join("proj");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&hst_root).unwrap();
    std::fs::create_dir_all(proj.join(".claude")).unwrap();
    std::fs::write(
        proj.join(".claude").join("settings.json"),
        r#"{"permissions": {"defaultMode": "acceptEdits", "ask": ["Bash*"], "allow": ["Read*"]}, "env": {"K": "v"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(proj.join(".codex")).unwrap();
    std::fs::write(
        proj.join(".codex").join("config.toml"),
        "sandbox_mode = \"read-only\"
",
    )
    .unwrap();
    // 互斥：与 --yolo 组合 clap 退出 2。
    hst()
        .args(["init", "--yolo", "--clear-project-yolo", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .failure()
        .code(2);
    hst()
        .args(["init", "--clear-project-yolo", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.flag.clear_project_yolo=true"))
        .stdout(contains("(cleared-yolo)"))
        .stdout(contains("init.hooks=skipped"));
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(proj.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    assert!(v["permissions"].get("defaultMode").is_none());
    assert!(v["permissions"].get("ask").is_none());
    assert_eq!(v["permissions"]["allow"][0], "Read*", "allow kept");
    assert_eq!(v["env"]["K"], "v");
    assert!(
        !proj.join(".codex").join("config.toml").exists(),
        "empty codex config removed"
    );
    // 家目录守卫（D52 同型）加输出面（codex 评审 O-2/O-3）：root 即用户家
    // 时整支跳过打 warn；scope/pretrust/project 三行不缺。
    hst()
        .args(["init", "--clear-project-yolo", "--project"])
        .arg(&user)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.warn=clear-project-yolo skipped"))
        .stdout(contains("init.scope=clear-project-yolo"))
        .stdout(contains("init.pretrust=skipped"))
        .stdout(contains("init.project="));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_yolo_partial_and_off_level_markers() {
    // D33：--yolo=<full|partial|off> 取值式分级；off 摘 hst 落键。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-lvl-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let user = tmp.join("fake-user-home");
    let hst_root = tmp.join("fake-hst-home");
    let proj = tmp.join("proj");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&hst_root).unwrap();
    // 非法级别：clap 退出 2。
    hst()
        .args(["init", "--yolo=bogus", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .failure()
        .code(2);
    // partial：标记与四家分级落键。
    hst()
        .args(["init", "--yolo=partial", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.scope=yolo"))
        .stdout(contains("init.yolo.level=partial"))
        .stdout(contains("init.hooks=skipped"));
    let uc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(user.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        uc["permissions"]["defaultMode"].as_str(),
        Some("acceptEdits")
    );
    let codex = std::fs::read_to_string(user.join(".codex").join("config.toml")).unwrap();
    assert!(
        codex.contains("workspace-write"),
        "codex partial sandbox: {codex}"
    );
    assert!(
        codex.contains("on-request"),
        "codex partial approval: {codex}"
    );
    let kimi = std::fs::read_to_string(user.join(".kimi-code").join("config.toml")).unwrap();
    assert!(kimi.contains("auto"), "kimi partial mode: {kimi}");
    let grok = std::fs::read_to_string(user.join(".grok").join("config.toml")).unwrap();
    assert!(grok.contains("auto"), "grok partial mode: {grok}");
    // off：retired 行加键摘除（kimi ours-only 文件删除）。
    hst()
        .args(["init", "--yolo=off", "--project"])
        .arg(&proj)
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.yolo.level=off"))
        .stdout(contains("init.retired="));
    assert!(
        !user.join(".claude").join("settings.json").exists(),
        "ours-only user claude settings deleted on off (no foreign keys to keep)"
    );
    assert!(
        !user.join(".kimi-code").join("config.toml").exists(),
        "ours-only kimi config deleted on off"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_project_yolo_off_retires_project_keys() {
    // D33：项目级 off 走 retire_project_yolo（ours 等值摘除）。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-pyoff-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let user = tmp.join("fake-user-home");
    let proj = tmp.join("proj");
    std::fs::create_dir_all(&user).unwrap();
    for args in [
        vec!["init", "--project-yolo", "--project"],
        vec!["init", "--project-yolo=off", "--project"],
    ] {
        let mut cmd = hst();
        cmd.args(&args).arg(&proj).env("HST_USER_HOME", &user);
        cmd.assert()
            .success()
            .stdout(contains("init.scope=yolo-project"));
    }
    assert!(
        !proj.join(".claude").join("settings.json").exists(),
        "ours-only project claude settings retired"
    );
    assert!(
        !proj.join(".kimi-code").join("config.toml").exists(),
        "ours-only project kimi config retired"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn init_rerun_is_byte_idempotent() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-init-idem-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let user = tmp.join("fake-user-home");
    let hst_root = tmp.join("fake-hst-home");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&hst_root).unwrap();
    // D28：幂等判据覆盖用户级注册五件。
    let rels = [
        ".claude/settings.json",
        ".codex/hooks.json",
        ".codex/config.toml",
        ".grok/hooks/ohmyagents-state.json",
        ".kimi-code/config.toml",
    ];
    let read_all = |base: &std::path::Path| -> Vec<String> {
        rels.iter()
            .map(|r| std::fs::read_to_string(base.join(r)).unwrap())
            .collect()
    };
    hst()
        .args(["init", "--project"])
        .arg(&tmp.join("proj"))
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success();
    let after_first = read_all(&user);
    // Second run rewrites nothing: the hook registrations converge.
    hst()
        .args(["init", "--project"])
        .arg(&tmp.join("proj"))
        .env("HST_USER_HOME", &user)
        .env("HST_ROOT", &hst_root)
        .assert()
        .success()
        .stdout(contains("init.hooks.wrote.count=0"));
    assert_eq!(read_all(&user), after_first, "rerun must be byte-identical");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dies_statusline_unknown_agent() {
    // Unknown names fail before any home config is touched.
    hst()
        .args(["statusline", "no-such-agent"])
        .assert()
        .failure()
        .stderr(contains("claude/codex/kimi/grok"));
}

#[test]
fn statusline_example_prints_customization_template() {
    // D18: --example 打印带注释模板后干净退出，不碰任何配置面。
    // D40：模板改双排面（segments / segments2 / single_line 三键）。
    // D43：三行精修（一行 dir/git、context 带 token 绝对值）。
    hst()
        .args(["statusline", "--example"])
        .assert()
        .success()
        .stdout(contains("~/.hst/statusline.toml"))
        .stdout(contains("segments = "))
        .stdout(contains("segments2 = "))
        .stdout(contains("segments3 = "))
        .stdout(contains("single_line"))
        .stdout(contains("tools / mcp / tokens"))
        .stdout(contains("rust / node / ts / zig / go / cpp"))
        // D51：--example 随默认段同步（codex F1：漂移会误导照抄丢时钟段）。
        .stdout(contains("\"clock\"]"))
        .stdout(contains("clock {icon}{datetime}"));
}

#[test]
fn dies_statusline_script_conflicts_with_builtin_and_example() {
    // clap 互斥：--script 与 --builtin / --example 不能同场。
    hst()
        .args(["statusline", "--script", "x.ps1", "--builtin"])
        .assert()
        .failure();
    hst()
        .args(["statusline", "--script", "x.ps1", "--example"])
        .assert()
        .failure();
}

#[test]
fn dies_statusline_script_unknown_agent_fails_before_deploy() {
    // 未知名在任何部署动作前快败（自备脚本不被触碰）。
    hst()
        .args(["statusline", "no-such-agent", "--script", "x.ps1"])
        .assert()
        .failure()
        .stderr(contains("claude/codex/kimi/grok"));
}

#[test]
fn completions_emit_shell_scripts() {
    for shell in ["bash", "powershell"] {
        let out = hst()
            .args(["completions", shell])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let s = String::from_utf8_lossy(&out);
        assert!(
            !s.is_empty() && s.contains("hst"),
            "{shell} script mentions hst"
        );
    }
    let out = hst()
        .args(["completions", "bash"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8_lossy(&out).contains("_hst"));
}

// ===== Agent 友好 IO 契约（issue #1，与 ome S003 同构）=====

#[test]
fn format_json_doctor_envelope_parses_and_blocked_exits_one() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-fmt-doctor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = hst()
        .args(["--format", "json", "doctor", "--project"])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("envelope parses");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["blocked"], serde_json::json!(true));
    assert!(v["data"]["findings"].as_array().unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn format_jsonl_agents_rows_each_parse() {
    let out = hst()
        .args(["--format", "jsonl", "agents"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 1, "至少一路 agent 行");
    for l in &lines {
        let v: serde_json::Value = serde_json::from_str(l).expect("jsonl 逐行可解析");
        assert!(v.get("agent").is_some(), "行带 agent 字段：{l}");
    }
    // 字段序契约（preserve_order）：installed 行首键 agent、次键 status。
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let keys: Vec<String> = first
        .as_object()
        .unwrap()
        .keys()
        .map(String::clone)
        .collect();
    assert_eq!(keys.first().map(String::as_str), Some("agent"));
}

#[test]
fn json_shorthand_works_after_subcommand() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-fmt-sh-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = hst()
        .args(["doctor", "--json", "--project"])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("--json 简写出信封");
    assert_eq!(v["data"]["blocked"], serde_json::json!(true));
    let _ = std::fs::remove_dir_all(&tmp);
}

// ===== `hst agents verify`（D17 无头验收）=====

#[test]
fn dies_verify_unknown_agent() {
    // 未知名在任何验收动作前快败，报支持面。
    hst()
        .args(["agents", "verify", "no-such-agent"])
        .assert()
        .failure()
        .stderr(contains("claude/codex/grok/kimi"));
}

/// 全 skip 环境的 env 罩子：PATH 空目录加 HST_ROOT 空目录，摘掉全部 agent 指引
/// 环境变量。默认目录源（~/.local/bin 等）无法罩住（dirs 走系统 API 不看
/// env），所以调用方要先自检仍检出 installed 就 skip。
fn verify_empty_env(cmd: &mut Command, sandbox: &std::path::Path) {
    cmd.env("PATH", sandbox.join("empty-path"))
        .env("HST_ROOT", sandbox.join("empty-hst-home"));
    for key in [
        "HST_AGENT_PATH",
        "CODEX_HOME",
        "HST_CLAUDE_BIN",
        "CLAUDE_BIN",
        "HST_CODEX_BIN",
        "CODEX_BIN",
        "HST_GROK_BIN",
        "GROK_BIN",
        "HST_KIMI_BIN",
        "KIMI_BIN",
        "KIMI_CODE_BIN",
    ] {
        cmd.env_remove(key);
    }
}

#[test]
fn verify_all_skip_exits_zero_when_no_agents_detected() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-verify-skip-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(tmp.join("empty-path")).unwrap();
    std::fs::create_dir_all(tmp.join("empty-hst-home")).unwrap();
    // 自检：罩子下仍有 agent 检出（默认目录源），本机造不出全缺，skip。
    let mut probe = hst();
    verify_empty_env(&mut probe, &tmp);
    let out = probe
        .args(["agents"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    if String::from_utf8_lossy(&out).contains("status=installed") {
        eprintln!("skip: default-dir agent installs survive the env sandbox on this host");
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }
    let mut cmd = hst();
    verify_empty_env(&mut cmd, &tmp);
    cmd.args(["agents", "verify"])
        .assert()
        .success()
        .stdout(contains("verify.claude=skip"))
        .stdout(contains("verify.codex=skip"))
        .stdout(contains("verify.grok=skip"))
        .stdout(contains("verify.kimi=skip"))
        .stdout(contains("verify.ok=true"));
    let _ = std::fs::remove_dir_all(&tmp);
}

/// 活体验收环境容错（D38 第 2 轮，2026-09-13 wsl 总台第三轮来函裁定）：
/// grok 与 kimi 的无头会话先过鉴权才触发 hook（S033 / P0038 / P0047 证据
/// 链），凭据不可用（缺文件、占位模板、过期、吊销均算）时 verify 的 hook
/// 层 fail 属环境事实非缺陷。**凭据文件形状探测无法证明可用性**（总台第
/// 三轮实证：auth.json 在场但不可用，形状闸门误放行后照红），唯一可靠的
/// 环境探测就是 verify 结果本身：hook 层 ok 才计入活体断言，hook 层不成
/// 打 skip 说明不计败。产品 verify 面不动（如实 fail，修环境归操作员，
/// 测试分层）。claude/codex 无鉴权前置的探测面，保持硬断言。
#[test]
fn verify_live_headless_acceptance_for_installed_agents() {
    // 闸门：依赖真 agent 二进制，消耗极少量真实 token；binary 不在
    // 则 eprintln skip。判据只押 hook state 落盘（SessionStart /
    // UserPromptSubmit 先于模型调用，S033）。
    let out = hst()
        .args(["agents"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&out).into_owned();
    let mut ran = 0;
    for name in ["claude", "codex", "grok", "kimi"] {
        let installed = text
            .lines()
            .any(|l| l.starts_with(&format!("agent={name} ")) && l.contains("status=installed"));
        if !installed {
            eprintln!("skip: {name} not installed");
            continue;
        }
        if !matches!(name, "grok" | "kimi") {
            ran += 1;
            hst()
                .args(["agents", "verify", name, "--timeout", "90"])
                .assert()
                .success()
                .stdout(contains(format!("verify.{name}.hook=ok")));
            continue;
        }
        // grok / kimi：跑一次不押断言，hook 层 ok 才计数；不成 = 无头鉴权
        // 不可用（环境事实），skip 带首条 reason 行供面板观察。
        let output = hst()
            .args(["agents", "verify", name, "--timeout", "90"])
            .assert()
            .get_output()
            .clone();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if stdout.contains(&format!("verify.{name}.hook=ok")) {
            ran += 1;
        } else {
            let reason = stdout
                .lines()
                .find(|l| l.starts_with(&format!("verify.{name}.")) && l.contains("fail"))
                .unwrap_or("no-fail-line");
            eprintln!(
                "skip: {name} headless auth unavailable (hook layer needs a live session; \
                 environment fact, not a defect; product verify stays fail): {reason}"
            );
        }
    }
    if ran == 0 {
        eprintln!("skip: no verifiable agents on this host (none installed or auth-gated)");
    }
}

#[test]
fn deployed_state_shim_roundtrips_stdin_payload() {
    // codex review G2 的最小集成钉：固定 payload 经管道喂给**落盘的**宿主
    // 载体 shim（Windows = ps1 面、Unix = sh 面），断言 state JSON 的
    // event / state / session 与输入一致——唯一能钉住 stdin 读法的一层
    // （G1 曾在读法回归下空解析假绿）。直管道不经中间 shell（claude 的
    // spawner 也是直管道形态）。session 用 ASCII（H1 对齐实测口径：非
    // ASCII 解码随宿主控制台代码页，不作跨宿主断言）。
    let tmp = std::env::temp_dir().join(format!(
        "hst-cli-shim-rt-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let root = tmp.join("hst-root");
    std::fs::create_dir_all(&root).unwrap();
    let state_file = tmp.join("state.json");
    // 部署 shim：走产品面（hook init 落全套 shim 到 HST_ROOT）。
    let dep = std::process::Command::new(env!("CARGO_BIN_EXE_hst"))
        .args(["hook", "init", "--project"])
        .arg(&tmp)
        .env("HST_ROOT", &root)
        .env("HST_USER_HOME", &tmp.join("user"))
        .output()
        .expect("hook init");
    assert!(dep.status.success(), "hook init failed: {:?}", dep.status);
    use std::io::Write;
    use std::process::{Command, Stdio};
    let payload = "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"rt-ascii-42\"}";
    let mut child = if cfg!(windows) {
        let ps1 = root
            .join("hooks")
            .join("hst-state.ps1")
            .display()
            .to_string()
            .replace('\\', "/");
        assert!(
            root.join("hooks").join("hst-state.ps1").is_file(),
            "shim deployed"
        );
        let mut c = Command::new("powershell.exe");
        c.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &ps1,
            "claude",
        ]);
        c
    } else {
        let sh = root.join("hooks").join("hst-state.sh");
        assert!(sh.is_file(), "shim deployed");
        let mut c = Command::new("sh");
        c.arg(&sh).arg("claude");
        c
    };
    let mut child = child
        .env("HST_STATE_FILE", &state_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn carrier shim");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .expect("feed payload");
    let out = child.wait_with_output().expect("wait carrier");
    assert!(out.status.success(), "carrier rc: {:?}", out.status.code());
    let text = std::fs::read_to_string(&state_file).expect("state written");
    let v: serde_json::Value = serde_json::from_str(&text).expect("state json");
    // event 大小写不敏感比对：PreToolUse / UserPromptSubmit 的 guard 委托
    // 把 payload 喂给 hst hook status，其自身也写 state 并规范化事件名
    // （小写，与 STATE_SH 的 tr 同向）；state 与 session 精确比对。
    let event = v["event"].as_str().unwrap_or("").to_ascii_lowercase();
    assert_eq!(event, "userpromptsubmit", "{text}");
    assert_eq!(v["state"].as_str(), Some("working"), "{text}");
    assert_eq!(v["session"].as_str(), Some("rt-ascii-42"), "{text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn json_and_format_are_mutually_exclusive() {
    hst()
        .args(["--json", "--format", "json", "agents"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn structured_error_goes_to_stderr_as_single_line_json() {
    // hst 契约（与 ome 裸数据的分道点）：结构化模式 stderr 单行 JSON 错误行
    // （人称与机器双通道），退出码非 0。
    let out = hst()
        .args(["--format", "json", "statusline", "no-such-agent"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "stderr 单行：{err}");
    let v: serde_json::Value = serde_json::from_str(lines[0]).expect("stderr 单行 JSON");
    assert_eq!(v["code"], serde_json::json!("error"));
    assert!(v["message"].as_str().is_some());
}

#[test]
fn self_update_mirror_first_does_not_loop_back() {
    // REQ-003（codex D48 评审 F3）假基址集成断言：mirror-first 两腿全失败
    // （假基址连接拒收加不存在的仓）时读序收束不回环——镜像腿只试一次、
    // GitHub 失败后不再回镜像（无 update.fallback=mirror 标记），按源码安
    // 装提示收尾退出 0。GitHub 腿任何失败形（404 加 403 加断网）都收在
    // 同一 Err 分支，断言只看标记不看 detail。
    let out = hst()
        .env("HST_MIRROR", "http://127.0.0.1:9/req003")
        .args([
            "self",
            "update",
            "--stable",
            "--repo",
            "req003/no-such-repo",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("update.mirror=http://127.0.0.1:9/req003"), "{s}");
    assert_eq!(
        s.matches("update.mirror=failed").count(),
        1,
        "镜像腿只试一次：{s}"
    );
    assert!(s.contains("update.fallback=github"), "{s}");
    assert!(s.contains("update.source=github"), "{s}");
    assert!(s.contains("update.release=unavailable"), "{s}");
    assert!(s.contains("update.hint="), "{s}");
    assert!(
        !s.contains("update.fallback=mirror"),
        "不回环：First 态 GitHub 失败后不得回退镜像：{s}"
    );
}

#[test]
fn self_update_mirror_off_runs_github_only() {
    // REQ-003 三态之 Off：HST_MIRROR 空串全关——GitHub 失败即收束（源码
    // 安装提示），全程无任何镜像腿标记（不尝试、不回退）。
    let out = hst()
        .env("HST_MIRROR", "")
        .args([
            "self",
            "update",
            "--stable",
            "--repo",
            "req003/no-such-repo",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("update.mirror=off"), "{s}");
    assert!(s.contains("update.source=github"), "{s}");
    assert!(s.contains("update.release=unavailable"), "{s}");
    assert!(s.contains("update.hint="), "{s}");
    assert!(!s.contains("update.mirror=failed"), "{s}");
    assert!(!s.contains("update.fallback=mirror"), "{s}");
    assert!(!s.contains("update.fallback=github"), "{s}");
}

// ===== cli-docs 采纳轮（2026-09-18）：帮助面节序、裸调用、--filter-output、漂移守卫 =====

#[test]
fn help_root_face_header_and_global_options_sections() {
    // 根帮助面：头行 name@version 连一句定位（版本从载体 manifest 注入）；
    // Options 与 Global Options 分节且 Global Options 在后；Global Options
    // 节内字典序且 --format 取值枚举全值（codex 评审 G1）；尾带 --llms 发现指引。
    let out = hst()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    let first = s.lines().next().unwrap_or_default();
    assert!(first.starts_with("hst@"), "头行 name@version：{first}");
    assert!(
        first.contains(env!("CARGO_PKG_VERSION")),
        "版本注入非手写：{first}"
    );
    let (Some(opts), Some(global)) = (s.find("Options:"), s.find("Global Options:")) else {
        panic!("Options 与 Global Options 分节齐备");
    };
    assert!(opts < global, "Options 先于 Global Options");
    let global_section = &s[global..];
    let order = [
        global_section.find("--filter-output"),
        global_section.find("--format"),
        global_section.find("--json"),
    ];
    assert!(
        order.iter().all(Option::is_some),
        "Global Options 三旗标在册"
    );
    let mut sorted = order;
    sorted.sort();
    assert_eq!(order, sorted, "Global Options 节内字典序");
    assert!(
        global_section.contains("--format <kv|json|jsonl>"),
        "--format 取值枚举全值"
    );
    assert!(s.contains("hst --llms"), "尾行 --llms 发现指引");
}

#[test]
fn bare_call_prints_help_and_exits_zero() {
    // 裸调用面（cli-docs 第五节）：无参 = 导航事件，全貌形（帮助体含命令
    // 表与 --llms 指引），exit 恒 0。
    let out = hst().assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Usage:"), "裸调用打印帮助体");
    assert!(s.contains("Commands:"), "帮助体含命令表");
    assert!(s.contains("--llms"), "帮助体含 --llms 发现指引");
}

#[test]
fn filter_output_shapes_envelope_data() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-filter-ok-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = hst()
        .args([
            "--json",
            "--filter-output",
            "blocked",
            "doctor",
            "--project",
        ])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("envelope parses");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["blocked"], serde_json::json!(true));
    assert_eq!(v["data"].as_object().unwrap().len(), 1, "data 只留过滤键");
    assert!(
        v["meta"]["duration_ms"].as_u64().is_some(),
        "meta 带 duration_ms"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn filter_output_missing_key_is_structured_error() {
    let tmp = std::env::temp_dir().join(format!(
        "hst-filter-miss-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let output = hst()
        .args(["--json", "--filter-output", "nope", "doctor", "--project"])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .clone();
    let err = String::from_utf8_lossy(&output.stderr);
    let line = err.lines().last().unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(line).expect("stderr 单行 JSON");
    assert_eq!(v["code"], serde_json::json!("error"));
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("键不存在")),
        "错误信息指未命中：{line}"
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout 信封仍可解析");
    assert_eq!(stdout["ok"], serde_json::json!(false));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn llms_manual_covers_command_tree_and_flags() {
    // 漂移守卫（cli-docs 第四节，codex 评审 G3/G4 收口）：以 --llms --json
    // 机器形为活命令树真源；命令路径只在「子命令表」节域断言（防读序与常
    // 用例的提法遮蔽表格缺行）；旗标按帮助面的节归属分域断言（Global
    // Options 节旗标对「通用旗标」节域、叶 Options 节旗标对「子命令表」节
    // 域、根专属旗标如 --llms 对「通用旗标」节域）；根帮助纳入遍历，叶形
    // 节序（头行、Options 先于 Global Options）随守卫。
    let manual =
        String::from_utf8_lossy(&hst().arg("--llms").assert().success().get_output().stdout)
            .into_owned();
    let section = |title: &str| -> String {
        let start = manual
            .find(&format!("## {title}"))
            .unwrap_or_else(|| panic!("手册缺节 {title}"));
        let rest = &manual[start..];
        let end = rest[3..]
            .find("\n## ")
            .map(|i| start + 3 + i)
            .unwrap_or(manual.len());
        manual[start..end].to_string()
    };
    for title in [
        "读序",
        "子命令表",
        "通用旗标",
        "退出码",
        "输出契约",
        "常用例",
    ] {
        let _ = section(title);
    }
    let cmd_table = section("子命令表");
    let common_flags = section("通用旗标");

    // 该命令自己的表行（组行首列恰为路径闭合反引号，叶行首列以路径加空格
    // 开头带 synopsis；防同名旗标跨命令遮蔽与组行前缀遮蔽，codex 二轮 G1）。
    let row_of = |table: &str, p: &str| -> String {
        let wanted = format!("| `{p}");
        table
            .lines()
            .find(|l| {
                l.starts_with(&wanted)
                    && l[wanted.len()..]
                        .chars()
                        .next()
                        .is_some_and(|c| c == '`' || c == ' ')
            })
            .unwrap_or_else(|| panic!("表 {table:?} 缺 {p} 自己的行"))
            .to_string()
    };
    // 通用旗标的首列在册判定（提及不算，codex 二轮 G2）。
    let first_cell_has = |table: &str, f: &str| -> bool {
        table.lines().any(|l| {
            l.strip_prefix("| `")
                .and_then(|rest| rest.split(" |").next())
                .is_some_and(|cell| cell.contains(f))
        })
    };

    let machine: serde_json::Value = serde_json::from_slice(
        &hst()
            .args(["--llms", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .expect("machine form parses");

    fn collect(node: &serde_json::Value, prefix: &str, paths: &mut Vec<String>) {
        // 根层键名 commands，嵌套层键名 subcommands（llm_machine_form 契约）。
        let empty = vec![];
        let cmds = node["commands"]
            .as_array()
            .or_else(|| node["subcommands"].as_array())
            .unwrap_or(&empty);
        for c in cmds {
            let name = c["name"].as_str().unwrap_or_default();
            let path = if prefix.is_empty() {
                format!("hst {name}")
            } else {
                format!("{prefix} {name}")
            };
            paths.push(path.clone());
            collect(c, &path, paths);
        }
    }
    let mut paths = Vec::new();
    collect(&machine, "", &mut paths);
    assert!(paths.len() >= 20, "命令树遍历到叶：{} 条", paths.len());
    for p in &paths {
        let _ = row_of(&cmd_table, p);
    }

    // flag 名提取：劈 [ = < , /（clap 冲突注记连写形如 --yolo/--project-yolo）。
    let flag_names = |text: &str| -> Vec<String> {
        let mut out = Vec::new();
        for token in text.split_whitespace() {
            let Some(name) = token.strip_prefix("--") else {
                continue;
            };
            let name: String = name
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect();
            if !name.is_empty() {
                out.push(format!("--{name}"));
            }
        }
        out
    };

    let mut checked = 0usize;
    // 根帮助：根专属旗标（Options 节，如 --llms）与全局旗标都登记在手册
    // 「通用旗标」节。
    let root_help =
        String::from_utf8_lossy(&hst().arg("--help").assert().success().get_output().stdout)
            .into_owned();
    let root_split = root_help
        .find("Global Options:")
        .expect("根帮助有 Global Options 节");
    for f in flag_names(&root_help[..root_split]) {
        assert!(
            first_cell_has(&common_flags, &f),
            "通用旗标节首列含根专属旗标 {f}（首列判定，提及不算）"
        );
        checked += 1;
    }
    for f in flag_names(&root_help[root_split..]) {
        assert!(
            first_cell_has(&common_flags, &f),
            "通用旗标节首列含全局旗标 {f}（首列判定，提及不算）"
        );
        checked += 1;
    }

    for p in &paths {
        let args: Vec<&str> = p.split_whitespace().skip(1).collect();
        let help = String::from_utf8_lossy(
            &hst()
                .args(&args)
                .arg("--help")
                .assert()
                .success()
                .get_output()
                .stdout,
        )
        .into_owned();
        // 叶/组形节序守卫（G4）：头行非空且不以 Usage 开头；Options 先于
        // Global Options。
        let first = help.lines().next().unwrap_or_default();
        assert!(
            !first.is_empty() && !first.starts_with("Usage:"),
            "{p} 头行在位"
        );
        let (Some(opts), Some(global)) = (help.find("Options:"), help.find("Global Options:"))
        else {
            panic!("{p} 帮助缺 Options 或 Global Options 节");
        };
        assert!(opts < global, "{p} Options 先于 Global Options");
        // 叶 Options 节旗标对该命令自己的表行（行级断言）；Global Options
        // 节旗标对通用旗标节首列；内建 --help/--version 恒对通用旗标节。
        let own_row = row_of(&cmd_table, p);
        for f in flag_names(&help[opts..global]) {
            if f == "--help" || f == "--version" {
                assert!(
                    first_cell_has(&common_flags, &f),
                    "通用旗标节首列含内建旗标 {f}（命令 {p}）"
                );
            } else {
                assert!(
                    own_row.contains(&f),
                    "{p} 自己的表行含旗标 {f}（行级断言，防跨命令遮蔽）"
                );
            }
            checked += 1;
        }
        for f in flag_names(&help[global..]) {
            assert!(
                first_cell_has(&common_flags, &f),
                "通用旗标节首列含全局旗标 {f}（命令 {p}）"
            );
            checked += 1;
        }
    }
    assert!(checked >= 40, "旗标核对量足够：{checked}");
}
