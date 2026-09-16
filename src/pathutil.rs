use std::path::{Path, PathBuf};

/// 数据目录名（D29，2026-09-12 用户裁更名 HST）：`.hst`。
pub const DIR: &str = ".hst";
/// D14 至 D29 的旧名（oma 纪元）。仅旧在、新不在时改名迁过去。
pub const LEGACY_OMA_DIR: &str = ".oma";
/// 2026-08-31 至 D14 的更旧名。仅旧在、新不在时改名迁过去。
pub const LEGACY_DIR: &str = ".ohmyagents";

/// `<parent>/.hst`。旧名（`.oma` 加 `.ohmyagents`）仅旧在、新不在时**同卷
/// rename** 迁过去（D29 启动探测迁移；rename 在同一父目录内恒同卷，跨卷
/// copy 校验删的分支由 HST_ROOT 显式覆盖场景规避——覆盖时不自动迁）；两
/// 者都在用新名、旧目录不动，**唯一例外**：新根只有 `bin/`（README 装法
/// 把 hst.exe 先落 `~/.hst/bin` 建了根）或空根视为未初始化，旧根子项逐个
/// move 迁入（撞名保新侧，bin 不动，迁空的旧根摘除；D30 堵 bin 先建根挡
/// rename 的缝）。
pub fn data_dir(parent: &Path) -> PathBuf {
    let neu = parent.join(DIR);
    if neu.exists() {
        if is_pristine_root(&neu) {
            for legacy in [LEGACY_OMA_DIR, LEGACY_DIR] {
                let old = parent.join(legacy);
                if old.exists() && merge_legacy_children(&old, &neu) {
                    break;
                }
            }
        }
        return neu;
    }
    for legacy in [LEGACY_OMA_DIR, LEGACY_DIR] {
        let old = parent.join(legacy);
        if old.exists() && std::fs::rename(&old, &neu).is_ok() {
            return neu;
        }
    }
    neu
}

/// 新根是否「未初始化」：只有 `bin/`（或空）。有 hooks/state 等任何实内容
/// 即视为已初始化（迁移窗口活会话重建 .oma/state 的并存实况，不动旧）。
fn is_pristine_root(neu: &Path) -> bool {
    match std::fs::read_dir(neu) {
        Ok(entries) => entries
            .flatten()
            .all(|e| e.file_name().to_str().is_some_and(|n| n == "bin")),
        // 读不了的根当已初始化，走不动旧语义（保守）。
        Err(_) => false,
    }
}

/// 旧根子项逐个 rename 进新根：撞名保新侧跳过；全部迁走则连旧根目录摘除，
/// 有迁不动的残留则旧根保留（下次启动重试）。返回是否迁了至少一项。
fn merge_legacy_children(old: &Path, neu: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(old) else {
        return false;
    };
    let mut moved = false;
    for e in entries.flatten() {
        let to = neu.join(e.file_name());
        if to.exists() {
            continue;
        }
        if std::fs::rename(e.path(), &to).is_ok() {
            moved = true;
        }
    }
    if moved {
        // 迁空才摘得掉；非空（有撞名保留或失败残留）保留旧根。
        let _ = std::fs::remove_dir(old);
    }
    moved
}

/// 用户家目录解析（D28）：`HST_USER_HOME` 覆盖优先（集成测试与 verify 的
/// 隔离缝：init/doctor/verify/statusline 的用户级读写全部经此），缺省
/// `dirs::home_dir()`。
pub fn user_home() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os("HST_USER_HOME") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    dirs::home_dir().ok_or_else(|| "cannot resolve home dir".to_string())
}

/// 跨模块共享的 env 互斥锁（测试专用）：HST_ROOT / HST_USER_HOME 等进程
/// 级环境变量的读写测试必须串行（各模块各自的锁锁不住彼此）。单一权威
/// 在 `testenv::ENV_LOCK`，此处只重导出（防两把锁并存，codex review F5）。
#[cfg(test)]
pub(crate) use crate::testenv::ENV_LOCK;

/// 项目侧数据根：`<project>/.oma`（session / state / tasks）。
pub fn project_dir(root: &Path) -> PathBuf {
    data_dir(root)
}

/// Absolute path without Windows `\\?\` prefix. Agent trust stores use this shape.
pub fn abs_display(path: &Path) -> PathBuf {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    });
    let raw = canon.to_string_lossy();
    let stripped = raw
        .strip_prefix(r"\\?\")
        .or_else(|| raw.strip_prefix("//?/"))
        .unwrap_or(&raw);
    PathBuf::from(stripped)
}

/// native_slash：路径工具的公开入口（行为细则与 marker 见 R002）。
pub fn native_slash(path: &Path) -> String {
    abs_display(path).to_string_lossy().into_owned()
}

/// forward_slash：路径工具的公开入口（行为细则与 marker 见 R002）。
pub fn forward_slash(path: &Path) -> String {
    native_slash(path).replace('\\', "/")
}

/// norm_key：路径工具的公开入口（Windows 侧配置键归一：反斜杠加小写；行为细则见 R002）。
#[cfg(windows)]
pub fn norm_key(s: &str) -> String {
    s.replace('/', "\\").to_ascii_lowercase()
}

#[cfg(not(windows))]
/// norm_key：路径工具的公开入口（行为细则与 marker 见 R002）。
pub fn norm_key(s: &str) -> String {
    s.replace('\\', "/").to_string()
}

/// keys_match：路径工具的公开入口（行为细则与 marker 见 R002）。
pub fn keys_match(a: &str, b: &str) -> bool {
    norm_key(a) == norm_key(b)
}

/// Resolve an executable name through PATH (`which` handles PATHEXT on
/// Windows, so "oma" finds oma.exe). None when absent — callers decide the
/// fallback. No oma-specific policy here.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    which::which(name).ok().map(|p| abs_display(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_parent() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-pathutil-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn d29_migration_three_cases() {
        // D29 三用例：有旧无新（rename 迁）、两者都有（用新不动旧）、只有
        // 新根（原样）。
        let parent = tmp_parent();
        // 1) 有 .oma 无 .hst：rename 迁过去，旧目录消失。
        std::fs::create_dir_all(parent.join(".oma").join("state")).unwrap();
        std::fs::write(parent.join(".oma").join("state").join("claude.json"), "{}").unwrap();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(
            got.join("state").join("claude.json").is_file(),
            "content survives rename"
        );
        assert!(!parent.join(".oma").exists(), "old root renamed away");
        let _ = std::fs::remove_dir_all(&parent);

        // 2) 两者都有：用新名，旧目录不动（活会话 shim 重建 .oma/state 的
        // 并存实况）。
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(".oma").join("state")).unwrap();
        std::fs::create_dir_all(parent.join(".hst").join("hooks")).unwrap();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(
            parent.join(".oma").exists(),
            "old root untouched when new present"
        );
        let _ = std::fs::remove_dir_all(&parent);

        // 3) 只有新根：原样返回。
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(".hst")).unwrap();
        assert_eq!(data_dir(&parent).file_name().unwrap(), DIR);
        assert!(!parent.join(".oma").exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn d30_bin_first_root_still_merges_legacy() {
        // README 装法先把 hst.exe 落 ~/.hst/bin 建根：rename 永不触发，旧根
        // 子项应逐个迁入（bin 保留、旧根摘除）；新根有实内容则仍不动旧。
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(".hst").join("bin")).unwrap();
        std::fs::write(parent.join(".hst").join("bin").join("hst.exe"), "bin").unwrap();
        std::fs::create_dir_all(parent.join(".oma").join("state")).unwrap();
        std::fs::write(parent.join(".oma").join("state").join("claude.json"), "{}").unwrap();
        std::fs::write(parent.join(".oma").join("statusline.toml"), "# cfg").unwrap();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(got.join("bin").join("hst.exe").is_file(), "bin survives");
        assert!(got.join("state").join("claude.json").is_file());
        assert!(
            got.join("statusline.toml").is_file(),
            "user customization migrates"
        );
        assert!(!parent.join(".oma").exists(), "legacy root drained");
        let _ = std::fs::remove_dir_all(&parent);

        // 撞名保新侧：pristine 语义下撞名只可能发生在 bin 本身（新根其余子项
        // 一出现即非 pristine）。旧根同名 bin 跳过、其余照迁，旧根因 bin 残
        // 留保留。
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(".hst").join("bin")).unwrap();
        std::fs::write(parent.join(".hst").join("bin").join("hst.exe"), "bin").unwrap();
        std::fs::create_dir_all(parent.join(".oma").join("bin")).unwrap();
        std::fs::write(parent.join(".oma").join("bin").join("junk.txt"), "old").unwrap();
        std::fs::create_dir_all(parent.join(".oma").join("state")).unwrap();
        std::fs::write(parent.join(".oma").join("state").join("claude.json"), "{}").unwrap();
        let got = data_dir(&parent);
        assert!(
            got.join("bin").join("hst.exe").is_file(),
            "collision keeps the new bin"
        );
        assert!(
            got.join("state").join("claude.json").is_file(),
            "non-colliding items migrate"
        );
        assert!(
            parent.join(".oma").join("bin").join("junk.txt").is_file(),
            "legacy bin residue kept in old root"
        );
        assert!(
            parent.join(".oma").exists(),
            "legacy root kept: bin collision residue"
        );
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn data_dir_renames_legacy_when_new_absent() {
        let parent = tmp_parent();
        let old = parent.join(LEGACY_DIR);
        std::fs::create_dir_all(old.join("state")).unwrap();
        std::fs::write(old.join("session.json"), "{}").unwrap();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(got.join("session.json").is_file());
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn data_dir_prefers_new_when_both_exist() {
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(DIR)).unwrap();
        std::fs::create_dir_all(parent.join(LEGACY_DIR)).unwrap();
        std::fs::write(parent.join(DIR).join("marker"), "new").unwrap();
        std::fs::write(parent.join(LEGACY_DIR).join("marker"), "old").unwrap();
        let got = data_dir(&parent);
        assert_eq!(std::fs::read_to_string(got.join("marker")).unwrap(), "new");
        assert!(parent.join(LEGACY_DIR).exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn data_dir_defaults_to_new_when_neither_exists() {
        let parent = tmp_parent();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(!got.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn find_on_path_missing_name_is_none() {
        assert!(find_on_path("oma-cargo-test-no-such-bin").is_none());
    }

    #[test]
    fn find_on_path_platform_shell_is_some() {
        // Expectation from the platform contract (cmd on Windows, sh
        // elsewhere), not from the implementation under test.
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(find_on_path(shell).is_some());
    }

    #[test]
    fn keys_match_slash_and_case() {
        assert!(keys_match(r"D:\foo\bar", "D:/foo/bar"));
        assert!(keys_match(r"d:\Foo", r"D:\foo") || !cfg!(windows));
        if cfg!(windows) {
            assert!(keys_match(r"D:\Foo", r"d:/foo"));
        }
    }
}

/// D52 铁证修复共享件：两路径是否同一位置（canonicalize 双侧归一符号
/// 链接，macOS /var 类；失败退 abs 字符串等值）。deploy_all 家目录守卫与
/// cmd_init 的 --project-yolo 守卫共用。
pub fn same_location(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => abs_display(a) == abs_display(b),
    }
}
