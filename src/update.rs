//! hst 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
//! releases 为空时走 `--git` 源码安装路径）。
//! 机制见 S028：releases/latest API、资产命名约定 `hst-<triple>.(zip|tar.gz)`、
//! Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。
//! D16 起 `HST_MIRROR=<基址>` 走镜像通道；D48 扩到双通道与缺省回退：设值 =
//! mirror-first（失败回落 GitHub）；未设 = GitHub 优先、失败自动回退镜像腿
//! （默认基址 env.ohmygh.com）；空串 = 镜像全关。镜像判新走
//! `<基址>/hst/<seg>/<资产名>.sha256` 边车对安装记录（段随通道，dev 禁落
//! stable）。镜像侧仅网络类失败回落；哈希不符是安全问题，报错不回落。
//! GH_TOKEN 在位附 Bearer（D48，匿名 60 升 5000 次每时）。

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

/// 自更新的更新通道面（细则见 R002 与模块文档）。
/// latest = 正式封版后的 releases/latest。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// 该字段承载自更新的Dev数据。
    Dev,
    /// 该字段承载自更新的Latest数据。
    Latest,
}

impl Channel {
    /// 自更新的as_str面（细则见 R002 与模块文档）。
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

/// 自更新的上次安装记录面（细则见 R002 与模块文档）。
/// digest 是压缩包哈希，与 exe 哈希不可比）。
fn record_path() -> Result<PathBuf, String> {
    Ok(crate::install::hst_home()?.join("selfupdate.json"))
}

fn read_record_digest() -> Option<String> {
    let text = std::fs::read_to_string(record_path().ok()?).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("digest").and_then(|d| d.as_str()).map(String::from)
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

/// 自更新的判据纯函数面（细则见 R002 与模块文档）。
fn digest_matches(record: Option<&str>, asset: Option<&str>) -> bool {
    match (record, asset) {
        (Some(r), Some(a)) => r.eq_ignore_ascii_case(a),
        _ => false,
    }
}

/// dev 通道判新：滚动源资产 digest 与上次安装记录一致即已最新。
fn dev_is_current(release: &Release) -> bool {
    let Some(asset) = pick_asset(&release.assets) else {
        return false;
    };
    digest_matches(read_record_digest().as_deref(), asset.digest.as_deref())
}

// ===== 镜像通道（D16 起 HST_MIRROR；D48 双通道与缺省回退） =====

/// 镜像缺省基址（D48）：GitHub 腿失败时的自动回退腿用它；`HST_MIRROR`
/// 设值覆盖（mirror-first 语义沿用 D16）。
const DEFAULT_MIRROR_BASE: &str = "https://env.ohmygh.com";

/// 镜像计划三态（D48，纯函数可测）：`HST_MIRROR` 未设 = GitHub 优先、失败
/// 自动回退默认基址（不占缺省行为面，ark 先例）；设值 = 基址覆盖加
/// mirror-first（两通道，D16 语义扩 stable）；空串 = 镜像全关。
#[derive(Debug, PartialEq, Eq)]
enum MirrorPlan {
    Off,
    DefaultFallback,
    First(String),
}

fn resolve_mirror_plan(env_raw: Option<String>) -> MirrorPlan {
    match env_raw {
        None => MirrorPlan::DefaultFallback,
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
    /// kv 标记值：First 沿用基址原值；DefaultFallback 标 fallback-default
    /// 形（区别于 off 与显式基址）；Off 沿用 off。
    fn kv(&self) -> String {
        match self {
            MirrorPlan::Off => "off".to_string(),
            MirrorPlan::DefaultFallback => format!("fallback-default:{DEFAULT_MIRROR_BASE}"),
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
    let lower = first.to_ascii_lowercase();
    let hex = lower.strip_prefix("sha256:").unwrap_or(&lower);
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("sidecar first field is not a sha256 hex: {first}"));
    }
    Ok(format!("sha256:{hex}"))
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
    let extracted = if name.ends_with(".zip") {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_zip(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    } else {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_tar_gz(&tmp, &out)?;
        find_hst_bin(&out).ok_or("hst binary not found in archive")?
    };
    let final_path = self_replace(&extracted)?;
    println!("update.replaced={}", final_path.display());
    write_record(&digest, &format!("{seg}-mirror"));
    println!("update.source=mirror");
    println!("update.ok=true");
    Ok(MirrorStep::Done)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Atomic-ish self replace: write the new binary beside the current exe, then
/// swap. Windows cannot overwrite a running exe but CAN rename it away.
pub fn self_replace(new_bin: &Path) -> Result<PathBuf, String> {
    let cur = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let dir = cur
        .parent()
        .ok_or_else(|| "no parent dir for current exe".to_string())?;
    let file = cur
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "exe name not utf-8".to_string())?;
    let staged = dir.join(format!(".{file}.new-{}", std::process::id()));
    std::fs::copy(new_bin, &staged).map_err(|e| format!("stage {}: {e}", staged.display()))?;
    if cfg!(windows) {
        let old = dir.join(format!(".{file}.old-{}", std::process::id()));
        std::fs::rename(&cur, &old).map_err(|e| format!("rename current away: {e}"))?;
        if let Err(e) = std::fs::rename(&staged, &cur) {
            // Put the old binary back so the install stays bootable.
            let _ = std::fs::rename(&old, &cur);
            return Err(format!("swap in new binary: {e}"));
        }
        let _ = std::fs::remove_file(&old);
    } else {
        std::fs::rename(&staged, &cur).map_err(|e| format!("replace binary: {e}"))?;
    }
    Ok(cur)
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

/// D48 读序回退守卫（REQ-003）：GitHub 失败后是否回退镜像腿。仅
/// DefaultFallback（未设 HST_MIRROR）真；First 已试过镜像，再回退即回环
/// 重试故不回；Off 显式全关。钉「不回环」性质（codex D48 评审 F3）。
fn github_fail_falls_back_to_mirror(plan: &MirrorPlan) -> bool {
    matches!(plan, MirrorPlan::DefaultFallback)
}

/// GitHub 失败后的回退基址（codex 评审 O1 收口）：仅 DefaultFallback 返回
/// 默认镜像基址、其余态 None；run 据此单点决定回退腿，三态单测因此有
/// 独立 oracle（不与 matches! 定义同源）。
fn mirror_fallback_base(plan: &MirrorPlan) -> Option<&'static str> {
    github_fail_falls_back_to_mirror(plan).then_some(DEFAULT_MIRROR_BASE)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// `hst self update` entry: release path with git fallback.
///
/// 判新（D48 双通道）：dev 按 rolling digest（资产 sha256 对安装记录，滚动版
/// 版本号常不变）；latest 走 GitHub 时按版本 tag，走镜像腿时同 dev 按 digest。
/// 自更新的读序面（细则见 R002 与模块文档）。
/// 失败（403 限流与网络类）自动回退镜像腿（默认基址）；空串镜像全关。
pub fn run(repo: &str, channel: Channel, git_mode: bool, force: bool) -> Result<(), String> {
    println!("update.current={}", env!("CARGO_PKG_VERSION"));
    println!("update.channel={}", channel.as_str());
    let plan = resolve_mirror_plan(std::env::var("HST_MIRROR").ok());
    println!("update.mirror={}", plan.kv());
    if git_mode {
        return git_install(repo);
    }
    if let MirrorPlan::First(base) = &plan {
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
            // D48 回退腿：GitHub 失败自动落镜像段边车锚（救 api.github.com
            // 匿名 403 限流与断网）。mirror-first 已试过镜像的不回环重试。
            if let Some(base) = mirror_fallback_base(&plan) {
                println!("update.release=unavailable detail={e}");
                println!("update.fallback=mirror");
                return match via_mirror(base, channel.mirror_seg(), force)? {
                    MirrorStep::Done => Ok(()),
                    MirrorStep::Fallback(detail) => {
                        println!("update.mirror=failed detail={detail}");
                        println!("update.hint=hst self update --git 走源码安装（封版前主路径）");
                        Ok(())
                    }
                };
            }
            println!("update.release=unavailable detail={e}");
            println!("update.hint=hst self update --git 走源码安装（封版前主路径）");
            return Ok(());
        }
    };
    println!("update.latest={}", release.tag_name);
    if !force {
        let up_to_date = match channel {
            Channel::Dev => dev_is_current(&release),
            // 资产名即编译目标（无版本段），版本判据走 release tag。
            Channel::Latest => !version_newer(&release.tag_name, env!("CARGO_PKG_VERSION")),
        };
        if up_to_date {
            println!("update.ok=already-latest");
            return Ok(());
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
    // 压缩包解开找 hst 本体；裸二进制资产直接用。
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
    let final_path = self_replace(&extracted)?;
    println!("update.replaced={}", final_path.display());
    if let Some(d) = asset.digest.as_deref() {
        write_record(d, &release.tag_name);
    }
    println!("update.ok=true");
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
        // D48 读序三态：未设 = GitHub 优先失败自动回退默认基址；设值 = 基址
        // 覆盖加 mirror-first；空串 = 镜像全关（显式退出通道）。
        assert!(matches!(
            resolve_mirror_plan(None),
            MirrorPlan::DefaultFallback
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
        // kv 标记值形：off 沿用、默认回退标 fallback-default、显式基址原值。
        assert_eq!(
            resolve_mirror_plan(None).kv(),
            "fallback-default:https://env.ohmygh.com"
        );
        assert_eq!(resolve_mirror_plan(Some(String::new())).kv(), "off");
        assert_eq!(
            resolve_mirror_plan(Some("https://m.example.com".to_string())).kv(),
            "https://m.example.com"
        );
    }

    #[test]
    fn mirror_seg_follows_channel_without_crossing() {
        // D48：段随通道（dev 与 stable 各回各段，dev 禁回落 stable）。
        assert_eq!(Channel::Dev.mirror_seg(), "dev");
        assert_eq!(Channel::Latest.mirror_seg(), "stable");
    }

    #[test]
    fn github_fail_fallback_guard_covers_three_read_states() {
        // REQ-003 读序回退守卫三态（codex D48 评审 F3）：仅 DefaultFallback
        // 触发镜像回退；First 已试过镜像（再回退即回环重试）、Off 全关。
        // env 三态解析面另见 mirror_plan_three_states_from_env_raw；First
        // 与 Off 的实腿接线见 tests/cli.rs 假基址集成断言。回退基址面
        //（codex 评审 O1）与 matches! 定义不同源：DefaultFallback 返默认
        // 基址本体，另两态 None。
        assert!(github_fail_falls_back_to_mirror(
            &MirrorPlan::DefaultFallback
        ));
        assert!(!github_fail_falls_back_to_mirror(&MirrorPlan::First(
            "https://m.example.com".into()
        )));
        assert!(!github_fail_falls_back_to_mirror(&MirrorPlan::Off));
        assert_eq!(
            mirror_fallback_base(&MirrorPlan::DefaultFallback),
            Some(DEFAULT_MIRROR_BASE)
        );
        assert_eq!(
            mirror_fallback_base(&MirrorPlan::First("https://m.example.com".into())),
            None
        );
        assert_eq!(mirror_fallback_base(&MirrorPlan::Off), None);
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
}
