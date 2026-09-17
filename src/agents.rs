use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::pathutil::abs_display;

/// Default agents this orchestrator knows how to spawn.
pub const DEFAULT_AGENTS: &[&str] = &["claude", "codex", "grok", "kimi"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// agent 探测的Source面（细则见模块文档与集成测试）。
pub enum Source {
    /// 该字段承载agent 探测的Env数据。
    Env,
    /// 该字段承载agent 探测的Path数据。
    Path,
    /// 该字段承载agent 探测的Hst数据。
    Hst,
    /// 该字段承载agent 探测的Default数据。
    Default,
}

impl Source {
    /// agent 探测的as_str面（细则见模块文档与集成测试）。
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Env => "env",
            Source::Path => "path",
            Source::Hst => "hst",
            Source::Default => "default",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Source::Env => 0,
            Source::Path => 1,
            Source::Hst => 2,
            Source::Default => 3,
        }
    }
}

#[derive(Clone, Debug)]
/// agent 探测的Hit面（细则见模块文档与集成测试）。
pub struct Hit {
    /// 该字段承载agent 探测的agent数据。
    pub agent: &'static str,
    /// 该字段承载agent 探测的command数据。
    pub command: String,
    /// 该字段承载agent 探测的path数据。
    pub path: PathBuf,
    /// 该字段承载agent 探测的source数据。
    pub source: Source,
    /// 该字段承载agent 探测的version数据。
    pub version: Option<String>,
    /// 该字段承载agent 探测的extras数据。
    pub extras: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
/// agent 探测的Report面（细则见模块文档与集成测试）。
pub struct Report {
    /// 该字段承载agent 探测的agent数据。
    pub agent: &'static str,
    /// 该字段承载agent 探测的hit数据。
    pub hit: Option<Hit>,
}

struct Spec {
    name: &'static str,
    commands: &'static [&'static str],
    env_keys: &'static [&'static str],
}

const SPECS: &[Spec] = &[
    Spec {
        name: "claude",
        commands: &["claude"],
        env_keys: &["HST_CLAUDE_BIN", "CLAUDE_BIN"],
    },
    Spec {
        name: "codex",
        commands: &["codex"],
        env_keys: &["HST_CODEX_BIN", "CODEX_BIN"],
    },
    Spec {
        name: "grok",
        commands: &["grok"],
        env_keys: &["HST_GROK_BIN", "GROK_BIN"],
    },
    Spec {
        name: "kimi",
        commands: &["kimi", "kimi-code"],
        env_keys: &["HST_KIMI_BIN", "KIMI_BIN", "KIMI_CODE_BIN"],
    },
];

/// Search roots used by `detect`. Tests inject dirs instead of reading the process env.
pub struct Probe {
    /// 该字段承载agent 探测的env_bins数据。
    pub env_bins: BTreeMap<String, PathBuf>,
    /// 该字段承载agent 探测的path_dirs数据。
    pub path_dirs: Vec<PathBuf>,
    /// 该字段承载agent 探测的extra_dirs数据。
    pub extra_dirs: Vec<PathBuf>,
    /// hst 自管安装（oma 纪元存量布局 `<根>/agents/<name>/<ver>/`，manifest 指路）的精确二进制表。
    pub hst_files: Vec<(String, PathBuf)>,
    /// 该字段承载agent 探测的default_files数据。
    pub default_files: Vec<(String, PathBuf)>,
    /// 该字段承载agent 探测的probe_version数据。
    pub probe_version: bool,
}

impl Probe {
    /// agent 探测的from_env面（细则见模块文档与集成测试）。
    pub fn from_env() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let local = dirs::data_local_dir();
        let roaming = dirs::data_dir();
        let mut env_bins = BTreeMap::new();
        for spec in SPECS {
            for key in spec.env_keys {
                if let Some(p) = env_path(key) {
                    env_bins.insert((*key).to_string(), p);
                    break;
                }
            }
        }
        let mut extra_dirs = split_path_var("HST_AGENT_PATH");
        extra_dirs.extend(codex_home_bins());
        let hst_files = crate::install::hst_home()
            .map(|h| crate::install::managed_binaries(&h))
            .unwrap_or_default();
        Probe {
            env_bins,
            path_dirs: split_path_var("PATH"),
            extra_dirs,
            hst_files,
            default_files: default_binaries(&home, local.as_deref(), roaming.as_deref()),
            probe_version: true,
        }
    }

    /// agent 探测的detect面（细则见模块文档与集成测试）。
    pub fn detect(&self) -> Vec<Report> {
        SPECS.iter().map(|spec| self.detect_one(spec)).collect()
    }

    /// 按名探测一家 agent：PATH、`HST_AGENT_PATH`、`HST_<AGENT>_BIN`、hst 自管根与默认目录五源；命中即返回安装位与版本。
    pub fn find(&self, name: &str) -> Option<Hit> {
        SPECS
            .iter()
            .find(|s| s.name == name)
            .and_then(|s| self.detect_one(s).hit)
    }

    fn detect_one(&self, spec: &Spec) -> Report {
        let mut found: Vec<(Source, String, PathBuf)> = Vec::new();

        for key in spec.env_keys {
            if let Some(p) = self.env_bins.get(*key) {
                if let Some(resolved) = existing_bin(p) {
                    found.push((Source::Env, spec.commands[0].to_string(), resolved));
                    break;
                }
            }
        }

        for dir in self.path_dirs.iter().chain(self.extra_dirs.iter()) {
            for cmd in spec.commands {
                for cand in command_candidates(dir, cmd) {
                    if let Some(resolved) = existing_bin(&cand) {
                        if !already(&found, &resolved) {
                            found.push((Source::Path, (*cmd).to_string(), resolved));
                        }
                    }
                }
            }
        }

        for (name, p) in &self.hst_files {
            if name != spec.name {
                continue;
            }
            if let Some(resolved) = existing_bin(p) {
                if !already(&found, &resolved) {
                    found.push((Source::Hst, spec.commands[0].to_string(), resolved));
                }
            }
        }

        for (name, p) in &self.default_files {
            if name != spec.name {
                continue;
            }
            if let Some(resolved) = existing_bin(p) {
                if !already(&found, &resolved) {
                    found.push((Source::Default, spec.commands[0].to_string(), resolved));
                }
            }
        }

        found.sort_by_key(|(src, _, _)| src.rank());
        let Some((source, command, path)) = found.first().cloned() else {
            return Report {
                agent: spec.name,
                hit: None,
            };
        };
        let extras = found
            .into_iter()
            .skip(1)
            .map(|(_, _, p)| p)
            .collect::<Vec<_>>();
        let version = if self.probe_version {
            read_version(&path)
        } else {
            None
        };
        Report {
            agent: spec.name,
            hit: Some(Hit {
                agent: spec.name,
                command,
                path,
                source,
                version,
                extras,
            }),
        }
    }
}

/// agent 探测的detect面（细则见模块文档与集成测试）。
pub fn detect() -> Vec<Report> {
    Probe::from_env().detect()
}

/// 按名探测一家 agent：PATH、`HST_AGENT_PATH`、`HST_<AGENT>_BIN`、hst 自管根与默认目录五源；命中即返回安装位与版本。
pub fn find(name: &str) -> Option<Hit> {
    Probe::from_env().find(name)
}

/// agent 探测的print_reports面（细则见模块文档与集成测试）。
pub fn print_reports(reports: &[Report]) {
    let mut installed = 0u32;
    let mut missing = 0u32;
    for r in reports {
        match &r.hit {
            Some(h) => {
                installed += 1;
                print!(
                    "agent={} status=installed source={} path={}",
                    h.agent,
                    h.source.as_str(),
                    h.path.display()
                );
                if let Some(v) = &h.version {
                    print!(" version={v}");
                } else {
                    // 探针失败才补跑一次分类（S021）：illegal-instruction 即
                    // 指令集不匹配命中，与「装了但说不出版本」区分开。
                    if let Some(kind) = probe_failure_kind(&h.path) {
                        print!(" probe={kind}");
                        if kind == "illegal-instruction" {
                            print!(" hint=cpu lacks instructions this binary needs; try npm variant or older build (S021)");
                        }
                    }
                }
                println!();
                for extra in &h.extras {
                    println!("agent={} extra={}", h.agent, extra.display());
                }
            }
            None => {
                missing += 1;
                println!(
                    "agent={} status=missing detail=not on PATH, HST_AGENT_PATH, HST_*_BIN, hst root, or default locations hint=ark install {}",
                    r.agent, r.agent
                );
            }
        }
    }
    println!("agents.installed={installed}");
    println!("agents.missing={missing}");
}

fn already(found: &[(Source, String, PathBuf)], path: &Path) -> bool {
    found.iter().any(|(_, _, p)| same_file(p, path))
}

fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    let v = std::env::var_os(key)?;
    if v.is_empty() {
        return None;
    }
    Some(PathBuf::from(v))
}

fn split_path_var(key: &str) -> Vec<PathBuf> {
    std::env::var_os(key)
        .map(|v| {
            std::env::split_paths(&v)
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn codex_home_bins() -> Vec<PathBuf> {
    let Some(home) = env_path("CODEX_HOME") else {
        return Vec::new();
    };
    vec![
        home.join("packages").join("standalone").join("current"),
        home.join("packages")
            .join("standalone")
            .join("current")
            .join("bin"),
        home.clone(),
    ]
}

fn existing_bin(path: &Path) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    Some(abs_display(path))
}

fn command_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = vec![dir.join(name)];
    if cfg!(windows) {
        let exts = pathext();
        for ext in &exts {
            let mut file = name.to_string();
            file.push_str(ext);
            out.push(dir.join(file));
        }
    }
    out
}

fn pathext() -> Vec<String> {
    let raw = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
    raw.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.starts_with('.') {
                s.to_string()
            } else {
                format!(".{s}")
            }
        })
        .collect()
}

fn exe(dir: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        dir.join(format!("{name}.exe"))
    } else {
        dir.join(name)
    }
}

fn default_binaries(
    home: &Path,
    local: Option<&Path>,
    roaming: Option<&Path>,
) -> Vec<(String, PathBuf)> {
    let local_bin = home.join(".local").join("bin");
    let mut out = Vec::new();

    // Claude: native installer, legacy local npm, Homebrew, WinGet-ish, npm global.
    out.push(("claude".into(), exe(&local_bin, "claude")));
    out.push((
        "claude".into(),
        exe(&home.join(".claude").join("bin"), "claude"),
    ));
    out.push((
        "claude".into(),
        exe(&home.join(".claude").join("local"), "claude"),
    ));
    out.push(("claude".into(), PathBuf::from("/opt/homebrew/bin/claude")));
    out.push(("claude".into(), PathBuf::from("/usr/local/bin/claude")));
    if let Some(local) = local {
        out.push((
            "claude".into(),
            exe(&local.join("Programs").join("ClaudeCode"), "claude"),
        ));
    }
    if let Some(roaming) = roaming {
        out.push(("claude".into(), roaming.join("npm").join("claude.cmd")));
        out.push(("claude".into(), exe(&roaming.join("npm"), "claude")));
    }

    // Codex: Windows junction, POSIX ~/.local/bin symlink, cargo, Homebrew.
    out.push(("codex".into(), exe(&local_bin, "codex")));
    out.push((
        "codex".into(),
        exe(
            &home
                .join(".codex")
                .join("packages")
                .join("standalone")
                .join("current"),
            "codex",
        ),
    ));
    out.push((
        "codex".into(),
        exe(
            &home
                .join(".codex")
                .join("packages")
                .join("standalone")
                .join("current")
                .join("bin"),
            "codex",
        ),
    ));
    out.push((
        "codex".into(),
        exe(&home.join(".cargo").join("bin"), "codex"),
    ));
    out.push(("codex".into(), PathBuf::from("/opt/homebrew/bin/codex")));
    out.push(("codex".into(), PathBuf::from("/usr/local/bin/codex")));
    if let Some(local) = local {
        out.push((
            "codex".into(),
            exe(
                &local
                    .join("Programs")
                    .join("OpenAI")
                    .join("Codex")
                    .join("bin"),
                "codex",
            ),
        ));
    }

    // Grok: ~/.grok/bin plus ~/.local/bin symlink.
    out.push(("grok".into(), exe(&home.join(".grok").join("bin"), "grok")));
    out.push(("grok".into(), exe(&local_bin, "grok")));
    out.push(("grok".into(), PathBuf::from("/opt/homebrew/bin/grok")));
    out.push(("grok".into(), PathBuf::from("/usr/local/bin/grok")));

    // Kimi: ~/.kimi-code (on PATH after installer) and ~/.local/bin.
    let kimi_home = home.join(".kimi-code");
    out.push(("kimi".into(), exe(&kimi_home, "kimi")));
    out.push(("kimi".into(), exe(&kimi_home, "kimi-code")));
    out.push(("kimi".into(), exe(&kimi_home.join("bin"), "kimi")));
    out.push(("kimi".into(), exe(&local_bin, "kimi")));
    out.push(("kimi".into(), exe(&local_bin, "kimi-code")));
    out.push(("kimi".into(), PathBuf::from("/opt/homebrew/bin/kimi")));
    out.push(("kimi".into(), PathBuf::from("/usr/local/bin/kimi")));

    out
}

fn read_version(bin: &Path) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    let text = if out.stdout.is_empty() {
        String::from_utf8_lossy(&out.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let line = text.lines().find(|l| !l.trim().is_empty())?;
    let s = line.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.chars().take(120).collect())
    }
}

/// 探针失败路径的退出形态分类（S021）：跑一次 `--version` 只为拿退出码。
/// 有版本输出的正常面不进这里（print_reports 只在 version 为 None 时调用）。
fn probe_failure_kind(bin: &Path) -> Option<&'static str> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    // Windows 崩溃：exit code 0xC000001D；Unix 信号退出：code() 为 None
    // （SIGILL=4 的 signal 细分留给 P0012 的 unix 分支）。
    let _ = out;
    Some(crate::caps::classify_probe_exit(out.status.code()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-agents-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn touch(path: &Path) {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    fn claude_name() -> &'static str {
        if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        }
    }

    #[test]
    fn path_beats_default() {
        let root = fresh();
        let path_dir = root.join("path");
        let def_dir = root.join("default");
        let path_bin = path_dir.join(claude_name());
        let def_bin = def_dir.join(claude_name());
        touch(&path_bin);
        touch(&def_bin);
        let probe = Probe {
            env_bins: BTreeMap::new(),
            path_dirs: vec![path_dir],
            extra_dirs: Vec::new(),
            hst_files: Vec::new(),
            default_files: vec![("claude".into(), def_bin)],
            probe_version: false,
        };
        let hit = probe.find("claude").expect("found");
        assert_eq!(hit.source, Source::Path);
        assert!(hit.path.ends_with(claude_name()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn oma_beats_default_but_not_path() {
        let root = fresh();
        let path_dir = root.join("path");
        let hst_bin = root.join("hst").join("claude.exe");
        let def_bin = root.join("default").join("claude.exe");
        touch(&path_dir.join(claude_name()));
        touch(&hst_bin);
        touch(&def_bin);
        let probe = Probe {
            env_bins: BTreeMap::new(),
            path_dirs: vec![path_dir],
            extra_dirs: Vec::new(),
            hst_files: vec![("claude".into(), hst_bin.clone())],
            default_files: vec![("claude".into(), def_bin.clone())],
            probe_version: false,
        };
        assert_eq!(probe.find("claude").unwrap().source, Source::Path);
        let probe = Probe {
            env_bins: BTreeMap::new(),
            path_dirs: Vec::new(),
            extra_dirs: Vec::new(),
            hst_files: vec![("claude".into(), hst_bin)],
            default_files: vec![("claude".into(), def_bin)],
            probe_version: false,
        };
        assert_eq!(probe.find("claude").unwrap().source, Source::Hst);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn env_beats_path() {
        let root = fresh();
        let path_dir = root.join("path");
        let env_bin = root.join("custom").join(claude_name());
        touch(&path_dir.join(claude_name()));
        touch(&env_bin);
        let mut env_bins = BTreeMap::new();
        env_bins.insert("HST_CLAUDE_BIN".into(), env_bin.clone());
        let probe = Probe {
            env_bins,
            path_dirs: vec![path_dir],
            extra_dirs: Vec::new(),
            hst_files: Vec::new(),
            default_files: Vec::new(),
            probe_version: false,
        };
        let hit = probe.find("claude").expect("found");
        assert_eq!(hit.source, Source::Env);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extra_path_is_custom_location() {
        let root = fresh();
        let extra = root.join("opt").join("agents");
        let bin = extra.join(claude_name());
        touch(&bin);
        let probe = Probe {
            env_bins: BTreeMap::new(),
            path_dirs: Vec::new(),
            extra_dirs: vec![extra],
            hst_files: Vec::new(),
            default_files: Vec::new(),
            probe_version: false,
        };
        let hit = probe.find("claude").expect("found");
        assert_eq!(hit.source, Source::Path);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_when_nowhere() {
        let probe = Probe {
            env_bins: BTreeMap::new(),
            path_dirs: Vec::new(),
            extra_dirs: Vec::new(),
            hst_files: Vec::new(),
            default_files: Vec::new(),
            probe_version: false,
        };
        assert!(probe.find("kimi").is_none());
        let reports = probe.detect();
        assert_eq!(reports.len(), 4);
        assert!(reports.iter().all(|r| r.hit.is_none()));
    }
}
