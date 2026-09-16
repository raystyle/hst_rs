//! hst 自管根与共享下载件。agent 二进制安装 / 升级机器已随 D20 移除（归
//! ome，D07 迁册）；本模块只留三件消费面共享的能力：hst 数据根解析、自管
//! 根存量安装的只读探测（`hst agents` 的 source=oma 面）、单文件下载
//! （`hst self update` 复用）。

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

const UA: &str = concat!("hst/", env!("CARGO_PKG_VERSION"));
/// 磁盘上的存量 manifest 名（oma 纪元落的名字）：**活的磁盘协议**，agents
/// 探测据此认定 hst 自管根里的存量安装（source=hst 面），改名会断存量识
/// 别；D45 oma 遗产清扫显式保留（与技能名 ohmyagents、doctor 的 oma-state
/// 历史识别同列不做清单）。
const MANIFEST_NAME: &str = ".oma-agent-manifest.toml";

// ---- 根解析 ----

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// hst 应用数据根：`HST_ROOT` 环境变量 > `~/.hst`（D29；旧 `~/.oma` 与
/// `~/.ohmyagents` 仅旧在且新根未初始化时迁过去，见 pathutil::data_dir）。
pub fn hst_home() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os("HST_ROOT") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    let home = crate::pathutil::user_home()?;
    Ok(crate::pathutil::data_dir(&home))
}

/// hst 自管 agent 安装根（oma 纪元存量布局 `<根>/agents/<name>/<version>/`）。
pub fn agents_root(home: &Path) -> PathBuf {
    home.join("agents")
}

// ---- 探测集成 ----

/// hst 自管安装的二进制清单（agents.rs 的 Probe 以 source=oma 消费）。
/// D20 后 oma 不再安装 agent，这里只读存量（多为 ome 接管前或 ome 复用
/// 同布局时的在位件）。
pub fn managed_binaries(home: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let root = agents_root(home);
    let Ok(names) = fs::read_dir(&root) else {
        return out;
    };
    for ent in names.flatten() {
        let name_dir = ent.path();
        if !name_dir.is_dir() {
            continue;
        }
        let agent = ent.file_name().to_string_lossy().into_owned();
        if let Ok(vers) = fs::read_dir(&name_dir) {
            for v in vers.flatten() {
                let manifest = v.path().join(MANIFEST_NAME);
                if let Some(rel) = manifest_field(&manifest, "binary_rel") {
                    let bin = v.path().join(&rel);
                    if bin.is_file() {
                        out.push((agent.clone(), bin));
                    }
                }
            }
        }
    }
    out
}

/// hst 自管已装版本（manifest 的 version 字段；多个版本目录取最大）。
pub fn managed_version(home: &Path, name: &str) -> Option<String> {
    let dir = agents_root(home).join(name);
    let mut best: Option<String> = None;
    let Ok(vers) = fs::read_dir(&dir) else {
        return None;
    };
    for v in vers.flatten() {
        let manifest = v.path().join(MANIFEST_NAME);
        if let Some(ver) = manifest_field(&manifest, "version") {
            let take = match &best {
                Some(b) => version_ge(&ver, b),
                None => true,
            };
            if take {
                best = Some(ver);
            }
        }
    }
    best
}

fn manifest_field(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// 点分数值比较（a >= b）；非数值段按 0 处理。
fn version_ge(a: &str, b: &str) -> bool {
    let key = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap_or(0))
            .collect()
    };
    let (mut ka, mut kb) = (key(a), key(b));
    let n = ka.len().max(kb.len());
    ka.resize(n, 0);
    kb.resize(n, 0);
    ka >= kb
}

// ---- 下载 ----

/// 单文件下载（github 域自动带 GH_TOKEN）；update.rs 复用。
pub(crate) fn download_asset(url: &str, dest: &Path) -> Result<(), String> {
    let mut req = ureq::get(url).set("User-Agent", UA);
    if url.starts_with("https://github.com/") || url.starts_with("https://api.github.com/") {
        if let Ok(token) = std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
            if !token.is_empty() {
                req = req.set("Authorization", &format!("Bearer {token}"));
            }
        }
    }
    let resp = req.call().map_err(|e| format!("GET {url}: {e}"))?;
    if resp.status() != 200 {
        return Err(format!("GET {url} -> HTTP {}", resp.status()));
    }
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    io::copy(&mut reader, &mut file).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-install-{}-{}-{}",
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

    #[test]
    fn version_ge_orders() {
        assert!(version_ge("0.151.0", "0.149.1"));
        assert!(!version_ge("0.149.1", "0.151.0"));
        assert!(version_ge("2.1.251", "2.1.246"));
        assert!(version_ge("1.0.13", "1.0.9"));
        assert!(version_ge("1.0.13", "1.0.13"));
    }

    #[test]
    fn managed_binaries_reads_manifest() {
        let home = fresh();
        let dest = agents_root(&home).join("claude").join("2.1.251");
        let bin = dest.join("claude.exe");
        fs::create_dir_all(&dest).unwrap();
        fs::write(&bin, b"fake").unwrap();
        // manifest_field 是行式解析：key = "value" 两行即够探测面。
        fs::write(
            dest.join(MANIFEST_NAME),
            "version = \"2.1.251\"\nbinary_rel = \"claude.exe\"\n",
        )
        .unwrap();
        let found = managed_binaries(&home);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "claude");
        assert!(found[0].1.ends_with(&bin));
        assert_eq!(managed_version(&home, "claude").as_deref(), Some("2.1.251"));
        let _ = fs::remove_dir_all(&home);
    }
}
