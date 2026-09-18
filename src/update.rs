//! hst 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
//! releases 为空时走 `--git` 源码安装路径）。
//! 机制见 S028：releases/latest API、资产命名约定 `hst-<triple>.(zip|tar.gz)`、
//! Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。
//! D16 起 `HST_MIRROR=<基址>` 走镜像通道；ADR-0008（2026-09-18 家族统一
//! 标准，承接 D48）：未设 = 镜像段优先（默认基址 env.ohmygh.com，任一步
//! 网络类失败整对回落 GitHub，不回环）；设值 = 基址覆盖同读序；空串 =
//! 镜像全关。镜像判新走 `<基址>/hst/<seg>/<资产名>.sha256` 边车对安装
//! 记录（段随通道，dev 禁落 stable）。镜像侧仅网络类失败回落；哈希不符
//! 是安全问题，报错不回落（GitHub 官方腿同判：下载后对 API digest 或同
//! Release 边车硬校验，锚不可得同样拒装）。latest 判新走 release tag
//! 三态（本地领先报 localNewer 不动），镜像 stable 腿降级守卫由暂存件
//! `--version` 预检承载（命中按 localNewer 收束，不装不回落）；自替换带
//! 更新锁、陈旧收割、入位后自证五次重试与回滚复核；ark 管理布局（同
//! 目录落痕或带落痕的符号链接入口）让位走 ark。GH_TOKEN 在位附 Bearer。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Default repo; `--repo owner/name` overrides. 更名史（2026-09-02）：
/// 仓库名 OhMyAgents → hst_rs（`hst-rs` 等旧名 URL 自动重定向，
/// canonical 与公开发布面一律 hst_rs，D35 评审 D 裁）。
pub const DEFAULT_REPO: &str = "raystyle/hst_rs";

const UA: &str = concat!("hst/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize)]
/// GitHub release 元数据（tag 与资产清单），self update 的查询面。
pub struct Release {
    /// 该字段承载自更新的tag_name数据。
    pub tag_name: String,
    #[serde(default)]
    /// 该字段承载自更新的assets数据。
    pub assets: Vec<Asset>,
    /// false for prereleases/drafts (latest already excludes them).
    #[serde(default = "default_true")]
    pub draft: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
/// release 资产条目：名、下载直链与可选 digest（判新锚）。
pub struct Asset {
    /// 该字段承载自更新的name数据。
    pub name: String,
    /// 该字段承载自更新的browser_download_url数据。
    pub browser_download_url: String,
    /// GitHub 资产摘要（新 API 形如 "sha256:<hex>"；旧响应可能缺省）。
    #[serde(default)]
    pub digest: Option<String>,
}

/// 自更新的更新通道面（细则见模块文档与集成测试）。
/// latest = 正式封版后的 releases/latest。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// 该字段承载自更新的Dev数据。
    Dev,
    /// 该字段承载自更新的Latest数据。
    Latest,
}

impl Channel {
    /// 自更新的as_str面（细则见模块文档与集成测试）。
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Dev => "dev",
            Channel::Latest => "latest",
        }
    }

    /// 镜像段名（D48）：段随通道、禁跨段回落（dev 不落 stable，防正式版
    /// 装进滚动源；ark 同款裁定）。
    pub fn mirror_seg(self) -> &'static str {
        match self {
            Channel::Dev => "dev",
            Channel::Latest => "stable",
        }
    }
}

/// GH_TOKEN 鉴权头（D48，ark resolve.rs 同款语义）：在位附 Bearer，匿名
/// 限流 60 次每时升 5000 次。纯函数入参可单测。
fn auth_header(token: Option<&str>) -> Option<String> {
    let t = token?.trim();
    if t.is_empty() {
        None
    } else {
        Some(format!("Bearer {t}"))
    }
}

fn github_token() -> Option<String> {
    std::env::var("GH_TOKEN").ok()
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 向 GitHub API 拉取指定通道的 release 元数据；未发布返回 Err 提示 `--git` 路径。
pub fn fetch_release(repo: &str, channel: Channel) -> Result<Release, String> {
    let url = match channel {
        Channel::Latest => format!("https://api.github.com/repos/{repo}/releases/latest"),
        Channel::Dev => format!("https://api.github.com/repos/{repo}/releases/tags/dev"),
    };
    let mut req = ureq::get(&url)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json");
    if let Some(h) = auth_header(github_token().as_deref()) {
        req = req.set("Authorization", &h);
    }
    let resp = req.call();
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Err(format!(
                "release '{ch}' not published for {repo} yet (pre-release phase); use `hst self update --git`",
                ch = channel.as_str()
            ))
        }
        Err(e) => return Err(format!("github api: {e}")),
    };
    resp.into_string()
        .map_err(|e| format!("read body: {e}"))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("parse release: {e}")))
}

/// Host target triple keywords for asset matching (keep in sync with the
/// release asset naming convention: hst-<triple>.zip / .tar.gz).
fn asset_keywords(os: &str) -> &'static [&'static str] {
    match os {
        // D47（2026-09-14 用户裁）：构建切 gnu 交叉编译摆脱 VC。新源首选
        // gnu 资产；旧 release 仅 msvc 资产时按 msvc 词命中，通用 windows
        // 词保底旧 msvc 二进制升级。
        "windows" => &["windows-gnu", "windows-msvc", "windows"],
        "macos" => &["apple-darwin", "darwin"],
        _ => &["linux-gnu", "linux"],
    }
}

/// 架构取参归一：`std::env::consts::ARCH` 形原样保 aarch64，其余归
/// x86_64（与发布矩阵的资产名架构词一致）。
fn asset_arch(arch: &str) -> &'static str {
    if arch == "aarch64" {
        "aarch64"
    } else {
        "x86_64"
    }
}

/// Pick the hst asset for this host from a release's asset list.
pub fn pick_asset(assets: &[Asset]) -> Option<&Asset> {
    pick_asset_for(assets, std::env::consts::OS, std::env::consts::ARCH)
}

/// pick_asset 的 (os, arch) 取参形（REQ-002）：D47 后 windows 测试岗已
/// 裁，cfg! 形分支留 gnu 字面量笔误盲区（linux/mac 岗只编译不出本机臂
/// 判据）；取参形让全平台选资产行为在任一 CI 岗可断言。
fn pick_asset_for<'a>(assets: &'a [Asset], os: &str, arch: &str) -> Option<&'a Asset> {
    let kws = asset_keywords(os);
    let arch = asset_arch(arch);
    assets
        .iter()
        .find(|a| {
            let n = a.name.to_ascii_lowercase();
            n.starts_with("hst")
                && (n.ends_with(".zip") || n.ends_with(".tar.gz"))
                && n.contains(arch)
                && kws.iter().any(|k| n.contains(k))
        })
        .or_else(|| {
            assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().starts_with("hst"))
        })
}

/// Dotted-numeric compare: is `tag` (v-prefix tolerated) strictly newer than
/// `current` (CARGO_PKG_VERSION)?
pub fn version_newer(tag: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parse(tag), parse(current));
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// 自更新的上次安装记录面（细则见模块文档与集成测试）。
/// digest 是压缩包哈希，与 exe 哈希不可比）。
fn record_path() -> Result<PathBuf, String> {
    Ok(crate::install::hst_home()?.join("selfupdate.json"))
}

fn read_record_digest() -> Option<String> {
    let text = std::fs::read_to_string(record_path().ok()?).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("digest")
        .and_then(|d| d.as_str())
        .and_then(normalize_digest)
}

fn write_record(digest: &str, tag: &str) {
    if let Ok(p) = record_path() {
        let body = format!(
            "{{\"digest\": \"{digest}\", \"tag\": \"{tag}\", \"ts\": {}}}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        let _ = std::fs::write(p, body);
    }
}

/// 自更新的判据纯函数面（细则见模块文档与集成测试）。
fn digest_matches(record: Option<&str>, asset: Option<&str>) -> bool {
    match (record, asset) {
        (Some(r), Some(a)) => r.eq_ignore_ascii_case(a),
        _ => false,
    }
}

/// digest 归一（批 C 钉死，ADR-0007）：GitHub API 的 digest 新形 `sha256:<hex>`
/// 与旧响应裸 hex 统一为 `sha256:<hex>`，与镜像腿 parse_sidecar 同形态；非法
/// 串回 None 走保守面。
fn normalize_digest(raw: &str) -> Option<String> {
    let lower = raw.trim().to_ascii_lowercase();
    let hex = lower.strip_prefix("sha256:").unwrap_or(&lower);
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("sha256:{hex}"))
    } else {
        None
    }
}

/// GitHub 腿判新 digest 取值（批 C 钉死）：API digest 归一优先；缺省或非法时
/// 回落取同 Release 的 `<资产名>.sha256` 边车资产内容（发布器与升级器同锚，
/// 旧 API 响应不再降级成每跑必重装）；边车也取不到回 None（安装臂对 None
/// fail-closed 拒装不回落，ADR-0008 评审收口）。回落臂无离线测试 seam，由
/// 下版发布首跑实测覆盖（codex 批 C O4 口径）。
fn github_asset_digest(release: &Release, asset: &Asset) -> Option<String> {
    if let Some(d) = asset.digest.as_deref().and_then(normalize_digest) {
        return Some(d);
    }
    let sidecar = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", asset.name))?;
    let text = http_get_string(&sidecar.browser_download_url).ok()?;
    parse_sidecar(&text).ok()
}

/// 文件 sha256 摘要（批 C 钉死：API digest 缺省时安装后按下载件实算写记录；
/// 复用镜像腿同款流式 archive::sha256_file，锚形态统一 sha256:hex）。
fn file_sha256(path: &std::path::Path) -> Result<String, String> {
    Ok(format!("sha256:{}", crate::archive::sha256_file(path)?))
}

/// dev 通道判新：滚动源资产 digest 与上次安装记录一致即已最新。
fn dev_is_current(release: &Release) -> bool {
    let Some(asset) = pick_asset(&release.assets) else {
        return false;
    };
    let Some(d) = github_asset_digest(release, asset) else {
        return false;
    };
    digest_matches(read_record_digest().as_deref(), Some(&d))
}

// ===== 镜像通道（D16 起 HST_MIRROR；D48 双通道与缺省回退） =====

/// 镜像缺省基址（D48）：GitHub 腿失败时的自动回退腿用它；`HST_MIRROR`
/// 设值覆盖（mirror-first 语义沿用 D16）。
const DEFAULT_MIRROR_BASE: &str = "https://env.ohmygh.com";

/// 镜像计划三态（ADR-0008 家族标准对齐，纯函数可测）：`HST_MIRROR` 未设 =
/// 镜像段优先（默认基址，stable 通道落 stable 滚动段，任一步失败整对回落
/// GitHub）；设值 = 基址覆盖同读序；空串 = 镜像全关。D48 的
/// DefaultFallback（GitHub 优先镜像回退）缺省形随本批退役。
#[derive(Debug, PartialEq, Eq)]
enum MirrorPlan {
    Off,
    DefaultFirst,
    First(String),
}

fn resolve_mirror_plan(env_raw: Option<String>) -> MirrorPlan {
    match env_raw {
        None => MirrorPlan::DefaultFirst,
        Some(v) => {
            let v = v.trim().trim_end_matches('/').to_string();
            if v.is_empty() {
                MirrorPlan::Off
            } else {
                MirrorPlan::First(v)
            }
        }
    }
}

impl MirrorPlan {
    /// 镜像腿基址：DefaultFirst 用默认基址、First 用覆盖值、Off 无镜像腿。
    fn base(&self) -> Option<&str> {
        match self {
            MirrorPlan::Off => None,
            MirrorPlan::DefaultFirst => Some(DEFAULT_MIRROR_BASE),
            MirrorPlan::First(b) => Some(b),
        }
    }

    /// kv 标记值：First 沿用基址原值；DefaultFirst 标 default-first 形
    /// （区别于 off 与显式基址）；Off 沿用 off。
    fn kv(&self) -> String {
        match self {
            MirrorPlan::Off => "off".to_string(),
            MirrorPlan::DefaultFirst => format!("default-first:{DEFAULT_MIRROR_BASE}"),
            MirrorPlan::First(b) => b.clone(),
        }
    }
}

/// 本机 host 三元组的确定性资产名（与 dev-release.yml 命名约定一致：
/// 资产名即编译目标三元组，windows 用 zip、其余 tar.gz）。
fn host_asset_name() -> String {
    asset_name(std::env::consts::OS, std::env::consts::ARCH)
}

/// 资产名取参纯函数（REQ-002）：os 取 `std::env::consts::OS` 常量形
/// （"windows" / "macos" / 其余按 linux），arch 经 `asset_arch` 归一；
/// 全平台字面量在任一 CI 岗可断言（cfg! 形盲区见 pick_asset_for 注）。
fn asset_name(os: &str, arch: &str) -> String {
    let arch = asset_arch(arch);
    match os {
        // D47（2026-09-14 用户裁）：构建切 gnu 交叉编译摆脱 VC，资产名随
        // CI 交叉岗改 pc-windows-gnu；stable 未封版前旧 release 仍为 msvc
        // 名，取侧由 asset_keywords 回落兜住。
        "windows" => format!("hst-{arch}-pc-windows-gnu.zip"),
        "macos" => format!("hst-{arch}-apple-darwin.tar.gz"),
        _ => format!("hst-{arch}-unknown-linux-gnu.tar.gz"),
    }
}

/// 镜像边车 URL（判新与校验）。段随通道（D48：dev 与 stable 各回各段）。
/// `?t=<unix>` 缓存击穿取 origin 现值（ohmycloud 2026-09-08 建议，CF 缓存
/// 键含 query；D16 尾巴修）。
fn mirror_sidecar_url(base: &str, seg: &str, name: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!(
        "{}/hst/{seg}/{name}.sha256?t={ts}",
        base.trim_end_matches('/')
    )
}

/// 镜像资产 URL。段随通道（D48）。`?v=<边车锚>` 以边车哈希为缓存键：每滚
/// 天然新键，永久免疫「边车新、资产旧」的陈旧缓存窗口（R2 取对象只看 path）。
fn mirror_asset_url(base: &str, seg: &str, name: &str, anchor: &str) -> String {
    format!("{}/hst/{seg}/{name}?v={anchor}", base.trim_end_matches('/'))
}

/// sha256sum 边车解析：首字段即哈希（标准双空格、单空格均可），容错
/// `sha256:` 前缀；归一为 `sha256:<hex>` 小写。脏输入报错（数据面失败，
/// 不归网络回落）。
fn parse_sidecar(text: &str) -> Result<String, String> {
    let first = text
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty sidecar".to_string())?;
    normalize_digest(first)
        .ok_or_else(|| format!("sidecar first field is not a sha256 hex: {first}"))
}

fn http_get_string(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|e| format!("{e}"))?
        .into_string()
        .map_err(|e| format!("read body: {e}"))
}

enum MirrorStep {
    /// 镜像路径走完（已最新或已替换），记录已写。
    Done,
    /// 网络类失败（边车/下载连不上、404、超时）：回落 GitHub dev 路径。
    Fallback(String),
}

/// 镜像腿（D48 双通道）：deterministic 名取段内边车 digest、对安装记录判新
/// （一致即 current、不同保守更新、无记录保守更新）→ 下载 → sha256 强校验
/// （不符即报错，安全问题不回落）→ 解包安装复用现状。段随通道传入。
fn via_mirror(base: &str, seg: &str, force: bool) -> Result<MirrorStep, String> {
    let name = host_asset_name();
    let sidecar_url = mirror_sidecar_url(base, seg, &name);
    let sidecar = match http_get_string(&sidecar_url) {
        Ok(t) => t,
        Err(e) => return Ok(MirrorStep::Fallback(format!("sidecar {sidecar_url}: {e}"))),
    };
    let digest =
        parse_sidecar(&sidecar).map_err(|e| format!("mirror sidecar {sidecar_url}: {e}"))?;
    if !force && digest_matches(read_record_digest().as_deref(), Some(&digest)) {
        println!("update.source=mirror");
        println!("update.ok=already-latest");
        return Ok(MirrorStep::Done);
    }
    // 资产缓存键锚定边车哈希（裸 hex），消边车与资产的缓存不同步窗口。
    let anchor = digest.strip_prefix("sha256:").unwrap_or(&digest);
    let asset_url = mirror_asset_url(base, seg, &name, anchor);
    println!("update.asset={name}");
    let tmp = std::env::temp_dir().join(format!(
        "hst-update-{}-{}",
        std::process::id(),
        name.replace('/', "_")
    ));
    if let Err(e) = crate::install::download_asset(&asset_url, &tmp) {
        return Ok(MirrorStep::Fallback(format!("download {asset_url}: {e}")));
    }
    // 安全闸：下载件必须与边车一致，不符拒绝安装且不回落（回落等于绕过校验）。
    let got = format!("sha256:{}", crate::archive::sha256_file(&tmp)?);
    if !got.eq_ignore_ascii_case(&digest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "mirror asset sha256 mismatch: sidecar {digest} got {got}; refusing install"
        ));
    }
    let out = tmp.with_extension("unpacked");
    let extracted = if name.ends_with(".zip") {
        crate::archive::extract_zip(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    } else {
        crate::archive::extract_tar_gz(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    };
    match self_replace(&extracted, None)? {
        ReplaceOutcome::Replaced(final_path) => {
            println!("update.replaced={}", final_path.display());
            write_record(&digest, &format!("{seg}-mirror"));
            println!("update.source=mirror");
            println!("update.ok=true");
        }
        // 降级守卫命中即收束态：不装、不写记录、不回落（镜像滞后窗口与
        // 本地预发布构建同收一支，semver 只升不降）。
        ReplaceOutcome::LocalNewer { reported } => {
            println!("update.ok=localNewer");
            println!(
                "update.note=镜像腿资产 {reported} 低于现版 {}（本地领先），不降级不回落；如确要回退走 GitHub Releases 手动装",
                env!("CARGO_PKG_VERSION")
            );
        }
    }
    // 证毕清理下载件与解包目录（失败臂保留残件供诊断）。
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_dir_all(&out);
    Ok(MirrorStep::Done)
}

/// semver 判新三态（家族标准）：同版、本地领先、远端更新。
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// 同版：already-latest。
    UpToDate,
    /// 本地领先（测试构建等）：报 localNewer 不动，semver 只升不降。
    LocalNewer,
    /// 远端更新：放行下载安装。
    Newer,
}

fn semver_verdict(tag: &str, current: &str) -> Verdict {
    if version_newer(tag, current) {
        Verdict::Newer
    } else if version_newer(current, tag) {
        Verdict::LocalNewer
    } else {
        Verdict::UpToDate
    }
}

/// 锁内 pid 是否仍活（linux 走 /proc；他端保守判活不收割，browse 同形）。
fn pid_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

/// exe 旁的更新锁路径（create_new 语义，占用即另一 update 在跑）。
fn lock_path(exe: &Path) -> PathBuf {
    exe.with_file_name(".hst-update.lock")
}

/// 锁件陈旧 mtime 窗（评审 G5）：正常持锁窗口（staging 加探针加 rename
/// 加自证重试）在秒级，超此窗即按陈旧收割，兜住空锁与他端保守判活面。
const STALE_LOCK_AFTER: std::time::Duration = std::time::Duration::from_secs(600);

/// 锁件是否按 mtime 判陈旧（读不到元数据按不陈旧处理，保守面）。
fn lock_is_stale(lock: &Path) -> bool {
    std::fs::metadata(lock)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age >= STALE_LOCK_AFTER)
}

/// 更新锁守卫：drop 时清锁件（盖 panic 面；SIGKILL 面靠陈旧 pid 收割）。
#[derive(Debug)]
struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 取更新锁：create_new 语义。AlreadyExists 先做陈旧判据：持有者 pid 已死，
/// 或锁件 mtime 超窗（SIGKILL 落在 create_new 与 writeln 之间留下的空锁、
/// 他端保守判活面，评审 G5），即收割重取一次；其余 io 错误报真因（安装位
/// 不可写等），不误报「在跑」。
fn acquire_lock(exe: &Path) -> Result<LockGuard, String> {
    let lock = lock_path(exe);
    let take = || -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create_new(&lock)?;
        let _ = writeln!(f, "{}", std::process::id());
        Ok(())
    };
    match take() {
        Ok(()) => {
            // 同抢竞态：两进程同抢时后到者回读锁 pid 让位。
            let owner = std::fs::read_to_string(&lock).unwrap_or_default();
            if owner.trim() == std::process::id().to_string() {
                Ok(LockGuard { path: lock })
            } else {
                Err(format!(
                    "另一 hst self update 正在跑（{}）；若确无 update 在跑，删 {} 后重试",
                    lock.display(),
                    lock.display()
                ))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let owner_dead = std::fs::read_to_string(&lock)
                .ok()
                .and_then(|t| t.trim().parse::<u32>().ok())
                .is_some_and(|pid| !pid_alive(pid));
            if (owner_dead || lock_is_stale(&lock))
                && std::fs::remove_file(&lock).is_ok()
                && take().is_ok()
            {
                println!("update.lock=reaped {}", lock.display());
                return Ok(LockGuard { path: lock });
            }
            Err(format!(
                "另一 hst self update 正在跑（{}）；若确无 update 在跑，删 {} 后重试",
                lock.display(),
                lock.display()
            ))
        }
        Err(e) => Err(format!(
            "建更新锁失败（{}）：{e}；确认安装位可写（系统目录需提权）",
            lock.display()
        )),
    }
}

/// 陈旧件收割：exe 旁 `.{file}.new-{pid}` 与 `.{file}.old-{pid}` 中 pid
/// 已死者清除（崩溃 run 残件）；活 pid 的不动（在跑 update 的暂存）。
/// `.old` 是回滚救援件：exe 缺位时全数保留（唯一好件可能在其中，评审
/// G4），仅 exe 在位才收。返回收割件路径清单（打点用）。
fn sweep_stale(dir: &Path, file: &str, exe: &Path) -> Vec<PathBuf> {
    let mut reaped = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return reaped;
    };
    let exe_present = exe.exists();
    for e in rd.flatten() {
        let Some(name) = e.file_name().into_string().ok() else {
            continue;
        };
        for (prefix, rescue_class) in [
            (format!(".{file}.new-"), false),
            (format!(".{file}.old-"), true),
        ] {
            let Some(rest) = name.strip_prefix(&prefix) else {
                continue;
            };
            if let Ok(pid) = rest.parse::<u32>() {
                if pid_alive(pid) {
                    continue;
                }
                if rescue_class && !exe_present {
                    continue;
                }
                if std::fs::remove_file(e.path()).is_ok() {
                    reaped.push(e.path());
                }
            }
        }
    }
    reaped
}

/// `--version` 输出的版号解析（`hst 2.3.0` 取 `2.3.0`；无数字 token 回
/// None）。
fn parse_reported_version(out: &str) -> Option<&str> {
    out.split_whitespace()
        .rev()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|t| t.trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.'))
}

/// 探针：跑 `bin --version` 取版号；跑不动或解析不出回 None（预检臂容
/// 忍缺省，自证臂另有强判）。
fn probe_version(bin: &Path) -> Option<String> {
    let out = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_reported_version(&String::from_utf8_lossy(&out.stdout)).map(String::from)
}

/// 降级守卫判据（纯函数）：预检探得版低于现版即拒装（semver 只升不降，
/// 镜像 stable 腿无版本段时的统一守卫）；探不到版（None）放行走自证臂。
fn downgrade_refused(probed: Option<&str>, current: &str) -> bool {
    probed.is_some_and(|v| version_newer(current, v))
}

/// 自证期望版推导（纯函数，评审 F2 收口）：tag 去 `v` 前缀后首字符为
/// 数字才取（semver 形，预发布后缀整体保留，与 `--version` 自报同形）；
/// `dev` 等非 semver tag 回 None（自证退化为可跑判，不强求等值）。
fn expect_version_from_tag(tag: &str) -> Option<&str> {
    let v = tag.strip_prefix('v').unwrap_or(tag);
    v.chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
        .then_some(v)
}

/// 管理方布局判据（家族标准，落痕生产者契约派 ark 侧）：exe 同目录
/// `ark-managed` 落痕，或用户面 bin 目录存在指向本 exe 的符号链接入口
/// 且链接目标目录有同落痕（ark 布局：真身进 EnvRoot 带落痕，用户面
/// symlink）。链接臂要求落痕，避免用户自建便利链接误拦（评审 G1）；
/// 相对链接目标按链接所在目录解析（canonicalize 按进程 CWD 会漂移）。
/// 返回命中物描述。
fn ark_managed_signal(exe: &Path) -> Option<String> {
    if let Some(d) = exe.parent() {
        let mark = d.join("ark-managed");
        if mark.exists() {
            return Some(format!("落痕 {}", mark.display()));
        }
    }
    let home_bin = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("bin"));
    let faces = [exe.parent().map(Path::to_path_buf), home_bin];
    faces
        .into_iter()
        .flatten()
        .filter_map(|d| std::fs::read_dir(&d).ok())
        .flat_map(|rd| rd.flatten().map(|e| e.path()))
        .filter_map(|p| {
            let t = std::fs::read_link(&p).ok()?;
            let target = if t.is_relative() {
                p.parent()?.join(&t)
            } else {
                t
            };
            let canon = target.canonicalize().ok()?;
            (canon == exe).then_some(canon)
        })
        .find_map(|canon| {
            let mark = canon.parent()?.join("ark-managed");
            mark.exists()
                .then(|| format!("链接目录落痕 {}", mark.display()))
        })
}

/// 回滚并复核终态：坏新件挪离原位、旧件回位、确认 exe 在位；任何一步
/// 失败报精确自救路径。入位失败臂与自证失败臂共用（browse 同形）。
fn rollback_and_verify(exe: &Path, bak: &Path, bad_new: &Path) -> Result<(), String> {
    let _ = std::fs::rename(exe, bad_new);
    match std::fs::rename(bak, exe) {
        Ok(()) if exe.exists() => Ok(()),
        Ok(()) => Err(format!(
            "回滚后复核 exe 缺位（旧件 {} 与新件 {} 已挪离原位）；下一步：按在位件手动复原到 {}",
            bak.display(),
            bad_new.display(),
            exe.display()
        )),
        Err(e) => Err(format!(
            "回滚受阻（{e}）：旧件在 {}，新件在 {}；下一步：手动复原 mv {} {} 后 hst issue new 反馈",
            bak.display(),
            bad_new.display(),
            bak.display(),
            exe.display()
        )),
    }
}

/// 自替换收束态（ADR-0008 家族标准）。
#[derive(Debug)]
pub enum ReplaceOutcome {
    /// 新件已入位并过自证，值为最终 exe 路径。
    Replaced(PathBuf),
    /// 本地领先（降级守卫命中，semver 只升不降）：未安装、未动旧件，
    /// 调用方按 localNewer 报告收束。
    LocalNewer {
        /// 暂存件 `--version` 自报版本。
        reported: String,
    },
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 自替换三步舞（ADR-0008 家族标准）：陈旧收割加取锁 -> 暂存落 exe 同
/// 目录（pid 后缀防并发互踩，跨文件系统 rename 必炸故不用 temp）->
/// `--version` 预检（降级守卫命中按 `LocalNewer` 收束，不装不动旧件）
/// -> 旧件挪 pid 备份、新件入位 -> `--version` 自证五次重试（杀软瞬时
/// 锁面，期望版已知时必须命中）-> 证毕清备份；证败或入位败回滚并
/// 复核终态，回滚受阻报自救路径。
pub fn self_replace(
    new_bin: &Path,
    expect_version: Option<&str>,
) -> Result<ReplaceOutcome, String> {
    let cur = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let dir = cur
        .parent()
        .ok_or_else(|| "no parent dir for current exe".to_string())?;
    let file = cur
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "exe name not utf-8".to_string())?;
    let reaped = sweep_stale(dir, file, &cur);
    if !reaped.is_empty() {
        let names = reaped
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
            .collect::<Vec<_>>()
            .join(",");
        println!("update.sweep={} {names}", reaped.len());
    }
    let _guard = acquire_lock(&cur)?;
    let pid = std::process::id();
    let staged = dir.join(format!(".{file}.new-{pid}"));
    let bak = dir.join(format!(".{file}.old-{pid}"));
    std::fs::copy(new_bin, &staged).map_err(|e| format!("stage {}: {e}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&staged)
            .map_err(|e| format!("stage meta {}: {e}", staged.display()))?
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&staged, perm)
            .map_err(|e| format!("stage chmod {}: {e}", staged.display()))?;
    }
    // 预检：暂存件 --version 低于现版即按 LocalNewer 收束（降级守卫，
    // semver 只升不降；不装不回落）；探不到版放行，由入位后自证臂兜底。
    let probed = probe_version(&staged);
    if downgrade_refused(probed.as_deref(), env!("CARGO_PKG_VERSION")) {
        let _ = std::fs::remove_file(&staged);
        return Ok(ReplaceOutcome::LocalNewer {
            reported: probed.unwrap_or_default(),
        });
    }
    // 三步舞：旧件挪备份、新件入位（入位败即回滚复核）。
    std::fs::rename(&cur, &bak).map_err(|e| format!("rename current away: {e}"))?;
    if let Err(e) = std::fs::rename(&staged, &cur) {
        rollback_and_verify(&cur, &bak, &staged)
            .map_err(|r| format!("新件入位失败（{e}）且{r}"))?;
        let _ = std::fs::remove_file(&bak);
        return Err(format!("新件入位失败（已回滚并复核在位）：{e}"));
    }
    // 自证：杀软瞬时锁面重试五次；期望版已知时必须命中（browse 同形）。
    let mut probe_ok = false;
    for i in 0..5 {
        match probe_version(&cur) {
            Some(v) if expect_version.is_none_or(|e| v == e) => {
                println!("update.probe={v}");
                probe_ok = true;
                break;
            }
            _ => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(200 * (i + 1)));
    }
    if !probe_ok {
        rollback_and_verify(&cur, &bak, &staged).map_err(|r| format!("新件自证失败且{r}"))?;
        let _ = std::fs::remove_file(&bak);
        return Err(format!(
            "新件自证失败（--version 未达期望 {}，已回滚并复核在位）；下一步：重试一次，仍失败 hst issue new 反馈",
            expect_version.unwrap_or("可运行")
        ));
    }
    let _ = std::fs::remove_file(&bak);
    Ok(ReplaceOutcome::Replaced(cur))
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// `cargo install --git` 源码安装（封版前的主路径）。
pub fn git_install(repo: &str) -> Result<(), String> {
    let cargo = crate::pathutil::find_on_path("cargo")
        .ok_or_else(|| "cargo not on PATH; install Rust toolchain first".to_string())?;
    let url = format!("https://github.com/{repo}.git");
    println!("update.git.repo={url}");
    let status = std::process::Command::new(&cargo)
        .args(["install", "--git", &url, "--force"])
        .status()
        .map_err(|e| format!("cargo install: {e}"))?;
    if status.success() {
        println!("update.git.ok=true");
        Ok(())
    } else {
        Err("cargo install --git failed".into())
    }
}

// D48 读序回退守卫（REQ-003）随 ADR-0008 退役：缺省翻镜像优先后，
// 「不回环」性质结构化成立（镜像腿只在首位试一次，GitHub 只整对回落
// 一次，GitHub 再失败不回镜像），不再需要独立回退守卫函数。

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// `hst self update` entry: release path with git fallback.
///
/// 读序（ADR-0008 家族标准）：管理方布局让位（ark-managed 落痕或用户面
/// 符号链接入口即拦走 ark）；镜像段优先（未设 HST_MIRROR = 默认基址，
/// 任一步网络类失败整对回落 GitHub）；判新 dev 按 rolling digest、latest
/// 走 GitHub tag 三态（already-latest、localNewer 不动、更新），镜像
/// stable 腿的降级守卫由暂存件 --version 预检承载；GitHub 官方腿下载后
/// digest 锚硬校验（不符拒装不回落）。
pub fn run(repo: &str, channel: Channel, git_mode: bool, force: bool) -> Result<(), String> {
    println!("update.current={}", env!("CARGO_PKG_VERSION"));
    println!("update.channel={}", channel.as_str());
    let plan = resolve_mirror_plan(std::env::var("HST_MIRROR").ok());
    println!("update.mirror={}", plan.kv());
    if git_mode {
        return git_install(repo);
    }
    if let Some(signal) =
        ark_managed_signal(&std::env::current_exe().map_err(|e| format!("current exe: {e}"))?)
    {
        return Err(format!(
            "检测到管理方布局（{signal}）；升级走 ark（管理方滚 catalog pin）；确为自管安装则删该落痕或链接后再 hst self update"
        ));
    }
    if let Some(base) = plan.base() {
        match via_mirror(base, channel.mirror_seg(), force)? {
            MirrorStep::Done => return Ok(()),
            MirrorStep::Fallback(detail) => {
                println!("update.mirror=failed detail={detail}");
                println!("update.fallback=github");
            }
        }
    }
    println!("update.source=github");
    let release = match fetch_release(repo, channel) {
        Ok(r) => r,
        Err(e) => {
            // 镜像腿已试过（plan.base 有值）或显式全关：不回环重试，按源码
            // 安装提示收束。
            println!("update.release=unavailable detail={e}");
            println!("update.hint=hst self update --git 走源码安装（封版前主路径）");
            return Ok(());
        }
    };
    println!("update.latest={}", release.tag_name);
    if !force {
        match channel {
            Channel::Dev => {
                if dev_is_current(&release) {
                    println!("update.ok=already-latest");
                    return Ok(());
                }
            }
            // 资产名即编译目标（无版本段），版本判据走 release tag；本地
            // 领先（测试构建）报 localNewer 不动，semver 只升不降。
            Channel::Latest => match semver_verdict(&release.tag_name, env!("CARGO_PKG_VERSION")) {
                Verdict::UpToDate => {
                    println!("update.ok=already-latest");
                    return Ok(());
                }
                Verdict::LocalNewer => {
                    println!("update.ok=localNewer");
                    println!(
                        "update.note=本地版本领先（可能是测试构建），不降级；如确要回退走 GitHub Releases 手动装"
                    );
                    return Ok(());
                }
                Verdict::Newer => {}
            },
        }
    }
    let Some(asset) = pick_asset(&release.assets) else {
        println!("update.release=asset-missing names={}", {
            let mut v: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
            v.sort();
            v.join(",")
        });
        println!("update.hint=hst self update --git 走源码安装");
        return Ok(());
    };
    println!("update.asset={}", asset.name);
    let tmp = std::env::temp_dir().join(format!(
        "hst-update-{}-{}",
        std::process::id(),
        asset.name.replace('/', "_")
    ));
    crate::install::download_asset(&asset.browser_download_url, &tmp)?;
    // digest 锚硬校验（家族标准）：锚 = API digest 归一优先、缺省回落同
    // Release 边车资产内容；不符拒装且不回落，锚不可得同样拒装（安全
    // 面 fail-closed，非可用性，评审 G2；发布面恒有边车，缺边车属发布
    // 缺陷）。
    let expected = github_asset_digest(&release, asset);
    let got = file_sha256(&tmp)?;
    let Some(anchor) = expected.as_deref() else {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "github asset 锚不可得（无 API digest 且 {}.sha256 边车缺失）拒装不回落；Release 缺边车属发布面缺陷，hst issue new 反馈",
            asset.name
        ));
    };
    if !anchor.eq_ignore_ascii_case(&got) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "github asset sha256 mismatch: anchor {anchor} got {got}; refusing install (no fallback)"
        ));
    }
    // 压缩包解开找 hst 本体；裸二进制资产直接用。
    let packed = asset.name.ends_with(".zip") || asset.name.ends_with(".tar.gz");
    let extracted = if asset.name.ends_with(".zip") {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_zip(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    } else if asset.name.ends_with(".tar.gz") {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_tar_gz(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    } else {
        tmp.clone()
    };
    match self_replace(&extracted, expect_version_from_tag(&release.tag_name))? {
        ReplaceOutcome::Replaced(final_path) => {
            println!("update.replaced={}", final_path.display());
            write_record(anchor, &release.tag_name);
            println!("update.ok=true");
        }
        // --force 或 dev 滚动源与本地预发布构建的窗口：不降级不写记录。
        ReplaceOutcome::LocalNewer { reported } => {
            println!("update.ok=localNewer");
            println!(
                "update.note=官方腿资产 {reported} 低于现版 {}（本地领先），不降级；如确要回退走 GitHub Releases 手动装",
                env!("CARGO_PKG_VERSION")
            );
        }
    }
    // 证毕清理下载件与解包目录（失败臂保留残件供诊断，评审 G9）。
    let _ = std::fs::remove_file(&tmp);
    if packed {
        let _ = std::fs::remove_dir_all(tmp.with_extension("unpacked"));
    }
    Ok(())
}

fn find_hst_bin(root: &Path) -> Option<PathBuf> {
    if root.is_file() {
        return Some(root.to_path_buf());
    }
    let name = if cfg!(windows) { "hst.exe" } else { "hst" };
    let direct = root.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|s| s.to_str()) == Some(name) {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_dotted_numeric() {
        assert!(version_newer("v0.2.0", "0.1.0"));
        assert!(version_newer("0.1.10", "0.1.9"));
        assert!(!version_newer("v0.1.0", "0.1.0"));
        assert!(!version_newer("0.0.9", "0.1.0"));
        // 非数字段按 0 处理不炸。
        assert!(version_newer("v1.0.0-rc1", "0.9.0"));
    }

    #[test]
    fn digest_matches_is_the_rolling_freshness_rule() {
        assert!(digest_matches(Some("sha256:abc"), Some("sha256:ABC")));
        assert!(!digest_matches(Some("sha256:abc"), Some("sha256:def")));
        // 缺记录或缺摘要 = 需更新（保守）。
        assert!(!digest_matches(None, Some("sha256:abc")));
        assert!(!digest_matches(Some("sha256:abc"), None));
    }

    #[test]
    fn picks_host_asset_from_convention() {
        let mk = |names: &[&str]| -> Vec<Asset> {
            names
                .iter()
                .map(|n| Asset {
                    name: n.to_string(),
                    browser_download_url: format!("https://x/{n}"),
                    digest: None,
                })
                .collect()
        };
        // 期望来自命名约定（S028）：资产名即编译目标 hst-<triple>，
        // 本机平台与架构的 hst 包优先。D47 起 windows 新源为 gnu 资产
        //（同一 release 不与新 msvc 并存，旧 msvc-only 形态见下条回落）。
        // REQ-002 取参形：三平台判据在任一 CI 岗全跑，不再跟本机走。
        let assets = mk(&[
            "hst-x86_64-unknown-linux-gnu.tar.gz",
            "hst-aarch64-apple-darwin.tar.gz",
            "hst-x86_64-pc-windows-gnu.zip",
            "notes.txt",
        ]);
        assert_eq!(
            pick_asset_for(&assets, "windows", "x86_64").unwrap().name,
            "hst-x86_64-pc-windows-gnu.zip"
        );
        assert_eq!(
            pick_asset_for(&assets, "macos", "aarch64").unwrap().name,
            "hst-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(
            pick_asset_for(&assets, "linux", "x86_64").unwrap().name,
            "hst-x86_64-unknown-linux-gnu.tar.gz"
        );
        // host 形零漂移：转发 env 常量，与取参形同源选本机资产。
        assert_eq!(
            pick_asset(&assets).unwrap().name,
            pick_asset_for(&assets, std::env::consts::OS, std::env::consts::ARCH)
                .unwrap()
                .name
        );
        // 兜底：无平台匹配时拿任一 hst 资产（提示用户核对）。
        let fb_assets = mk(&["hst-any.bin", "x.txt"]);
        let fallback = pick_asset(&fb_assets).unwrap();
        assert_eq!(fallback.name, "hst-any.bin");
        // D47 回落：旧 release 仅 msvc 资产时 windows 命中 msvc 升级（stable
        // 封版前的存量 release 形态；取参形在 linux/mac 岗同样可断）。
        let msvc_only = mk(&["hst-x86_64-pc-windows-msvc.zip"]);
        assert_eq!(
            pick_asset_for(&msvc_only, "windows", "x86_64")
                .unwrap()
                .name,
            "hst-x86_64-pc-windows-msvc.zip"
        );
    }

    // ===== D16 镜像通道纯函数 =====

    #[test]
    fn asset_name_and_keywords_cover_all_release_platforms() {
        // REQ-002：取参纯函数全平台断言。D47 后 windows 测试岗已裁，cfg!
        // 形断言在 linux/mac 岗只出本机臂判据，gnu 字面量笔误可过 CI
        //（codex D47 评审 F3）；本表在任一岗全跑。资产名期望值来自
        // dev-release.yml 命名约定（资产名即编译目标三元组，windows 用
        // zip、其余 tar.gz）。
        let names = [
            ("windows", "x86_64", "hst-x86_64-pc-windows-gnu.zip"),
            ("windows", "aarch64", "hst-aarch64-pc-windows-gnu.zip"),
            ("macos", "x86_64", "hst-x86_64-apple-darwin.tar.gz"),
            ("macos", "aarch64", "hst-aarch64-apple-darwin.tar.gz"),
            ("linux", "x86_64", "hst-x86_64-unknown-linux-gnu.tar.gz"),
            ("linux", "aarch64", "hst-aarch64-unknown-linux-gnu.tar.gz"),
        ];
        for (os, arch, want) in names {
            assert_eq!(asset_name(os, arch), want, "asset_name({os}, {arch})");
        }
        // 关键词梯子（S028 / D47）：windows-gnu 优先、msvc 回落、通用词
        // 保底旧 msvc 二进制升级；mac 与 linux 各自双词。
        assert_eq!(
            asset_keywords("windows"),
            &["windows-gnu", "windows-msvc", "windows"]
        );
        assert_eq!(asset_keywords("macos"), &["apple-darwin", "darwin"]);
        assert_eq!(asset_keywords("linux"), &["linux-gnu", "linux"]);
        // host 形零漂移：转发 env 常量，与取参形同源；宿主 OS 必须在受测
        // 三域内（codex 评审 O2：恢复旧测试的未测宿主守卫，_ 臂无判据时
        // 至少本机域有卡）。
        assert!(matches!(
            std::env::consts::OS,
            "windows" | "macos" | "linux"
        ));
        assert_eq!(
            host_asset_name(),
            asset_name(std::env::consts::OS, std::env::consts::ARCH)
        );
    }

    #[test]
    fn mirror_urls_trim_trailing_slash() {
        // 缓存击穿约定（ohmycloud 2026-09-08）：边车带 ?t= 时间戳、资产带
        // ?v=<边车锚>；两者基址尾斜杠归一。段随通道（D48）：dev 与 stable 各回各段。
        let s = mirror_sidecar_url("https://env.ohmygh.com/", "dev", "hst-x.zip");
        assert!(
            s.starts_with("https://env.ohmygh.com/hst/dev/hst-x.zip.sha256?t="),
            "sidecar url: {s}"
        );
        assert!(
            s["https://env.ohmygh.com/hst/dev/hst-x.zip.sha256?t=".len()..]
                .chars()
                .all(|c| c.is_ascii_digit())
        );
        let a = mirror_asset_url("https://env.ohmygh.com/", "dev", "hst-x.zip", "abc123");
        assert_eq!(a, "https://env.ohmygh.com/hst/dev/hst-x.zip?v=abc123");
        let s2 = mirror_sidecar_url("https://env.ohmygh.com", "dev", "hst-x.zip");
        assert!(s2.starts_with("https://env.ohmygh.com/hst/dev/hst-x.zip.sha256?t="));
        // D48 stable 段双形（stable 通道镜像腿与回退腿的 URL 契约）。
        let s3 = mirror_sidecar_url("https://env.ohmygh.com", "stable", "hst-x.zip");
        assert!(s3.starts_with("https://env.ohmygh.com/hst/stable/hst-x.zip.sha256?t="));
        let a3 = mirror_asset_url("https://env.ohmygh.com", "stable", "hst-x.zip", "dead");
        assert_eq!(a3, "https://env.ohmygh.com/hst/stable/hst-x.zip?v=dead");
    }

    #[test]
    fn auth_header_attaches_bearer_only_for_nonempty_token() {
        // D48：GH_TOKEN 在位附 Bearer（匿名 60 升 5000 次每时）；空串与
        // 纯空白视同未设（ark resolve.rs 同款语义）。
        assert_eq!(auth_header(None), None);
        assert_eq!(auth_header(Some("")), None);
        assert_eq!(auth_header(Some("   ")), None);
        assert_eq!(auth_header(Some("tok")), Some("Bearer tok".to_string()));
        assert_eq!(auth_header(Some(" tok ")), Some("Bearer tok".to_string()));
    }

    #[test]
    fn mirror_plan_three_states_from_env_raw() {
        // ADR-0008 读序三态：未设 = 镜像段优先（默认基址）；设值 = 基址
        // 覆盖同读序；空串 = 镜像全关（显式退出通道）。D48 的 GitHub 优先
        // 缺省形（fallback-default 标记）随家族标准退役。
        assert!(matches!(
            resolve_mirror_plan(None),
            MirrorPlan::DefaultFirst
        ));
        assert!(matches!(
            resolve_mirror_plan(Some(String::new())),
            MirrorPlan::Off
        ));
        assert!(matches!(
            resolve_mirror_plan(Some("  ".to_string())),
            MirrorPlan::Off
        ));
        match resolve_mirror_plan(Some("https://m.example.com/".to_string())) {
            MirrorPlan::First(b) => assert_eq!(b, "https://m.example.com"),
            other => panic!("expected First, got {other:?}"),
        }
        // kv 标记值形：off 沿用、缺省镜像优先标 default-first、显式基址原值。
        assert_eq!(
            resolve_mirror_plan(None).kv(),
            "default-first:https://env.ohmygh.com"
        );
        assert_eq!(resolve_mirror_plan(Some(String::new())).kv(), "off");
        assert_eq!(
            resolve_mirror_plan(Some("https://m.example.com".to_string())).kv(),
            "https://m.example.com"
        );
        // 镜像腿基址：DefaultFirst 与 First 有腿、Off 无腿（run 据此单点
        // 定读序，镜像优先对两态同形）。
        assert_eq!(resolve_mirror_plan(None).base(), Some(DEFAULT_MIRROR_BASE));
        assert_eq!(
            resolve_mirror_plan(Some("https://m.example.com".to_string())).base(),
            Some("https://m.example.com")
        );
        assert_eq!(resolve_mirror_plan(Some(String::new())).base(), None);
    }

    #[test]
    fn mirror_seg_follows_channel_without_crossing() {
        // D48：段随通道（dev 与 stable 各回各段，dev 禁回落 stable）。
        assert_eq!(Channel::Dev.mirror_seg(), "dev");
        assert_eq!(Channel::Latest.mirror_seg(), "stable");
    }

    #[test]
    fn semver_verdict_three_states() {
        // 家族标准：同版 already-latest、本地领先 localNewer 不动、远端更新
        // 放行；v 前缀容错。
        assert_eq!(semver_verdict("v2.3.0", "2.3.0"), Verdict::UpToDate);
        assert_eq!(semver_verdict("2.3.0", "2.4.0"), Verdict::LocalNewer);
        assert_eq!(semver_verdict("v2.5.0", "2.4.0"), Verdict::Newer);
        assert_eq!(semver_verdict("0.1.10", "0.1.9"), Verdict::Newer);
    }

    #[test]
    fn parse_reported_version_takes_dotted_token() {
        assert_eq!(parse_reported_version("hst 2.3.0"), Some("2.3.0"));
        assert_eq!(parse_reported_version("hst 2.3.0\n"), Some("2.3.0"));
        assert_eq!(parse_reported_version("no digits here"), None);
        assert_eq!(parse_reported_version(""), None);
    }

    #[test]
    fn probe_version_parses_hst_binary_when_reachable() {
        // 探针实弹（可跳过形）：CARGO_BIN_EXE_hst 只在集成测试注入、lib
        // 单测壳（libtest）不识 --version，此处取得到才实弹；探针所依赖的
        // 输出契约（hst --version 出可解析点分版号）另由 cli 集成
        // update_version_output_parses 钉死。
        let Some(bin) = std::env::var_os("CARGO_BIN_EXE_hst") else {
            return;
        };
        assert_eq!(
            probe_version(Path::new(&bin)).as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn downgrade_guard_refuses_older_staged() {
        // 降级守卫判据：探得版低于现版拒装；同版与更新放行；探不到版放行
        //（由自证臂兜底）。
        assert!(downgrade_refused(Some("2.3.0"), "2.4.0"));
        assert!(!downgrade_refused(Some("2.4.0"), "2.4.0"));
        assert!(!downgrade_refused(Some("2.5.0"), "2.4.0"));
        assert!(!downgrade_refused(None, "2.4.0"));
    }

    #[test]
    fn acquire_lock_rejects_live_owner_and_reaps_stale() {
        // 家族标准更新锁：占用即拒（活 pid）；陈旧锁（死 pid）收割重取；
        // drop 清锁件。
        let dir = std::env::temp_dir().join(format!(
            "hst-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("hst");
        // 活锁：锁内写本进程 pid（活着）——第二个 acquire 必须拒。
        std::fs::write(lock_path(&exe), std::process::id().to_string()).unwrap();
        let err = acquire_lock(&exe).unwrap_err();
        assert!(err.contains("正在跑"), "{err}");
        // 陈旧锁：死 pid 收割重取成功，drop 后锁件消失。
        let dead = 4_000_000; // /proc 上限之上的死 pid（linux 实判，他端保
                              // 守判活由 pid_alive 的 cfg 分支承载。
        std::fs::write(lock_path(&exe), dead.to_string()).unwrap();
        if !pid_alive(dead) {
            let guard = acquire_lock(&exe).expect("stale lock reaped");
            drop(guard);
            assert!(!lock_path(&exe).exists(), "drop 清锁");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sweep_stale_harvests_dead_pid_files() {
        // 陈旧收割：死 pid 的 .new 残件清除、活 pid 暂存不动；.old 是救援
        // 件，exe 缺位时保留（唯一好件可能在其中），exe 在位才收（评审 G4）。
        let dir = std::env::temp_dir().join(format!(
            "hst-sweep-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dead = 4_000_000;
        if pid_alive(dead) {
            let _ = std::fs::remove_dir_all(&dir);
            return; // 非 linux 端保守判活，收割臂不适用
        }
        let exe = dir.join("hst");
        std::fs::write(dir.join(format!(".hst.new-{dead}")), b"x").unwrap();
        std::fs::write(dir.join(format!(".hst.old-{dead}")), b"x").unwrap();
        std::fs::write(dir.join(format!(".hst.new-{}", std::process::id())), b"x").unwrap();
        std::fs::write(dir.join("hst-unrelated"), b"x").unwrap();
        // exe 缺位：只收 .new，.old 留作救援。
        let reaped = sweep_stale(&dir, "hst", &exe);
        assert_eq!(reaped.len(), 1, "exe 缺位只收死 pid 的 .new 残件");
        assert!(
            dir.join(format!(".hst.old-{dead}")).exists(),
            "exe 缺位保救援件"
        );
        assert!(dir
            .join(format!(".hst.new-{}", std::process::id()))
            .exists());
        assert!(dir.join("hst-unrelated").exists());
        // exe 在位：.old 也收。
        std::fs::write(&exe, b"x").unwrap();
        let reaped = sweep_stale(&dir, "hst", &exe);
        assert_eq!(reaped.len(), 1, "exe 在位收 .old 残件");
        assert!(!dir.join(format!(".hst.old-{dead}")).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expect_version_from_tag_semver_shapes() {
        // 期望版推导（评审 F2）：semver 形去 v 整体取（预发布后缀保留，
        // 与 --version 自报同形）；dev 等非 semver tag 回 None（自证退化
        // 为可跑判，不强求等值）。
        assert_eq!(expect_version_from_tag("v2.5.0"), Some("2.5.0"));
        assert_eq!(expect_version_from_tag("v2.5.0-rc.1"), Some("2.5.0-rc.1"));
        assert_eq!(expect_version_from_tag("2.4.0"), Some("2.4.0"));
        assert_eq!(expect_version_from_tag("dev"), None);
        assert_eq!(expect_version_from_tag(""), None);
    }

    #[test]
    fn acquire_lock_reaps_unparseable_stale_lock_by_mtime() {
        // 空锁兜底（评审 G5）：SIGKILL 落在 create_new 与 writeln 之间留下
        // 不可解析锁，mtime 超窗即陈旧收割重取；新鲜空锁（并发在写窗口）
        // 不收。
        let dir = std::env::temp_dir().join(format!(
            "hst-lock2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("hst");
        let lock = lock_path(&exe);
        std::fs::write(&lock, "").unwrap();
        let f = std::fs::File::options().append(true).open(&lock).unwrap();
        f.set_modified(
            std::time::SystemTime::now()
                .checked_sub(STALE_LOCK_AFTER + std::time::Duration::from_secs(1))
                .unwrap(),
        )
        .unwrap();
        let guard = acquire_lock(&exe).expect("陈旧空锁收割重取");
        drop(guard);
        assert!(!lock.exists(), "drop 清锁");
        std::fs::write(&lock, "").unwrap();
        assert!(acquire_lock(&exe).is_err(), "新鲜空锁不收割");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sidecar_parses_sha256sum_formats() {
        // Oracle：sha256sum 输出格式——哈希、两个空格、文件名（POSIX 文本模式）。
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let want = format!("sha256:{hex}");
        // 标准双空格加文件名
        assert_eq!(
            parse_sidecar(&format!("{hex}  hst-x86_64-pc-windows-msvc.zip\n")).unwrap(),
            want
        );
        // 单空格容错
        assert_eq!(parse_sidecar(&format!("{hex} hst.zip")).unwrap(), want);
        // 大写哈希归一小写
        assert_eq!(parse_sidecar(&hex.to_ascii_uppercase()).unwrap(), want);
        // sha256: 前缀容错（大小写均收）
        assert_eq!(parse_sidecar(&format!("sha256:{hex}")).unwrap(), want);
        assert_eq!(parse_sidecar(&format!("SHA256:{hex}")).unwrap(), want);
    }

    #[test]
    fn dies_sidecar_rejects_dirty_input() {
        // 空、非 hex、长度不符都报错（数据面失败，不走网络回落）。
        assert!(parse_sidecar("").is_err());
        assert!(parse_sidecar("   \n").is_err());
        assert!(parse_sidecar("not-a-hash  hst.zip").is_err());
        assert!(parse_sidecar("abcd  hst.zip").is_err());
        assert!(parse_sidecar(
            "zz7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  hst.zip"
        )
        .is_err());
    }

    #[test]
    fn mirror_digest_interops_with_github_record() {
        // GitHub 记录形如 sha256:ABC（大写），边车小写 hex：digest_matches
        // 口径下互认为同一版本。
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let parsed = parse_sidecar(&format!("{hex}  hst.zip")).unwrap();
        assert!(digest_matches(
            Some(&format!("sha256:{}", hex.to_ascii_uppercase())),
            Some(&parsed)
        ));
    }

    #[test]
    fn normalize_digest_unifies_bare_and_prefixed_forms() {
        // 批 C 钉死：新旧 API 形归一为 sha256:hex，非法串回 None。
        let hex = "a".repeat(64);
        assert_eq!(
            normalize_digest(&format!("sha256:{hex}")),
            Some(format!("sha256:{hex}"))
        );
        assert_eq!(normalize_digest(&hex), Some(format!("sha256:{hex}")));
        assert_eq!(
            normalize_digest(&hex.to_uppercase()),
            Some(format!("sha256:{hex}"))
        );
        assert_eq!(normalize_digest("not-a-digest"), None);
        assert_eq!(normalize_digest(""), None);
    }

    #[test]
    fn github_asset_digest_prefers_api_over_sidecar_shape() {
        // API digest 在位（归一形）优先；缺省且边车不可达回 None（保守更新，
        // 不再降级成每跑必重装）。边车回落面由 http 层承载（集成面锚边车）。
        let hex = "b".repeat(64);
        let release = Release {
            tag_name: "dev".into(),
            draft: false,
            assets: vec![Asset {
                name: "hst-x.tar.gz".into(),
                browser_download_url: "https://example.invalid/hst-x.tar.gz".into(),
                digest: Some(format!("sha256:{hex}")),
            }],
        };
        assert_eq!(
            github_asset_digest(&release, &release.assets[0]),
            Some(format!("sha256:{hex}"))
        );
        let bare = Release {
            tag_name: "dev".into(),
            draft: false,
            assets: vec![Asset {
                name: "hst-x.tar.gz".into(),
                browser_download_url: "https://example.invalid/hst-x.tar.gz".into(),
                digest: None,
            }],
        };
        assert_eq!(github_asset_digest(&bare, &bare.assets[0]), None);
    }
}
