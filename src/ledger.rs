//! hst ledger 面（REQ-063 Phase 3；总台修正令 2026-09-20 权限收口）：
//! 客户端实现统一走 `ledger-client` crate（ledger-rs v0.1.0，舰队唯一实
//! 现，签名道加只增面全在其中），本模块只剩仓身份加密档管理加本地快检
//! 加家族读序三件。**权限只增**：issue new/list/show 加 artifact
//! publish/attest（attest_dev 加 attest_prod 加 verification_failed 三验
//! 证型）/list；close/delete 与 status 推进加 promote/demote/supersede 归
//! omc 工作台（开发工作台经 herdr 委托执行）。私钥密档
//! `~/.hst/ledger/ed25519.key`（hex 32 字节，0600）或 env
//! `HST_LEDGER_PRIVATE_KEY`（hex），不进仓不进 argv。

use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// 本仓账本身份（REQ-063：repo_id = 规范化 remote）。
pub const REPO_ID: &str = "github.com/raystyle/hst_rs";

/// 本仓公钥 JWK（字母键序紧凑形；与总台在册 kid 对应）。配对自检用，
/// 协议实现在 ledger-client crate。
pub const PUBKEY_JWK: &str =
    "{\"crv\":\"Ed25519\",\"kty\":\"OKP\",\"x\":\"QlyooL1oXMP7d3krEGnur7cpCLo6EsnsGGvhnnmrNo0\"}";

/// issue kind 集（服务端 ISSUE_KINDS 同源，本地快检面）：bug（BUG 错误
/// 任务）与 improvement（改进优化任务），缺省 bug。
pub const ISSUE_KINDS: &[&str] = &["bug", "improvement"];

/// kid = sha256hex(规范化 JWK)，与 ledger-client 派生约定一致；常量本身
/// 已是字母键序紧凑形，直接哈希（单一真相）。
pub fn key_id() -> String {
    let mut h = Sha256::new();
    h.update(PUBKEY_JWK.as_bytes());
    format!("{:x}", h.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// 私钥密档路径（`~/.hst/ledger/ed25519.key`，内容 = hex 32 字节 seed；
/// 0600 由 keygen 落）。
pub fn private_key_path() -> Result<PathBuf, String> {
    Ok(crate::install::hst_home()?
        .join("ledger")
        .join("ed25519.key"))
}

fn read_seed_hex() -> Result<String, String> {
    if let Some(s) = std::env::var("HST_LEDGER_PRIVATE_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return Ok(s.trim().to_string());
    }
    let p = private_key_path()?;
    let text = std::fs::read_to_string(&p).map_err(|e| {
        format!(
            "私钥密档不可读（{}；或设 env HST_LEDGER_PRIVATE_KEY）：{e}",
            p.display()
        )
    })?;
    Ok(text.trim().to_string())
}

/// 旧形密档识别与就地迁移（评审 G2）：v2.6.0 期密档是 base64url（43 字
/// 符），crate 只吃 hex（64 字符）。识别旧形即转 hex 原子落回（kid 不
/// 变，同一把钥），舰队存量机升级即得迁移路径。返回迁移与否。
fn migrate_legacy_vault() -> Result<bool, String> {
    let p = private_key_path()?;
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Ok(false);
    };
    let t = text.trim();
    if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(false);
    }
    // base64url 32 字节（无填充 43 字符）形。
    if t.len() == 43
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        use base64::Engine;
        let seed = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(t.as_bytes())
            .map_err(|e| format!("旧形密档解码失败（{}）：{e}", p.display()))?;
        let hex_seed: String = seed.iter().map(|b| format!("{b:02x}")).collect();
        // 原子换入（评审 G5）：同目录 temp（0600）加 rename，迁移中途中
        // 断不丢钥（截断重写无此保证，该钥在册丢了只能重注册）。
        let tmp = p.with_extension("key.tmp");
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true).mode(0o600);
            let mut f = opts
                .open(&tmp)
                .map_err(|e| format!("{}: {e}", tmp.display()))?;
            use std::io::Write;
            f.write_all(hex_seed.as_bytes())
                .map_err(|e| format!("{}: {e}", tmp.display()))?;
        }
        #[cfg(not(unix))]
        std::fs::write(&tmp, &hex_seed).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &p)
            .map_err(|e| format!("{} -> {}: {e}", tmp.display(), p.display()))?;
        return Ok(true);
    }
    Ok(false)
}

/// 构造标准客户端（ledger-client crate）：env 优先次密档。
/// # Errors
///
/// 失败返回 `String` 错误（密钥读取与解析类）。
pub fn client() -> Result<ledger_client::Ledger, String> {
    let _ = migrate_legacy_vault()?;
    let seed = read_seed_hex()?;
    let key = ledger_client::KeyPair::load_secret_hex(&seed).map_err(|e| e.to_string())?;
    Ok(ledger_client::Ledger::new(REPO_ID, key))
}

/// 只读客户端（GET 面免钥，评审 F2 加 F4）：crate `read_only` 构造
/// （v0.1.2 起）；密钥在位用真钥形（语义同，仅省一次生成），旧形密档
/// 先迁移，不可解析不拦读（stderr 提示写入会失败）。写入面守钥报
/// `LedgerError::Key` 由 crate 承担。
/// # Errors
///
/// 失败返回 `String` 错误（密钥在位但不可解析类）。
pub fn client_readonly() -> Result<ledger_client::Ledger, String> {
    // crate `read_only` 构造（v0.1.2）：GET 免签免钥；旧形密档先迁移，
    // 不可解析不拦读（stderr 提示写入会失败，评审 F4）。
    let _ = migrate_legacy_vault();
    if let Ok(seed) = read_seed_hex() {
        if let Ok(key) = ledger_client::KeyPair::load_secret_hex(&seed) {
            return Ok(ledger_client::Ledger::new(REPO_ID, key));
        }
        eprintln!(
            "ledger.warn=密档在位但不可解析（读面继续，写入会失败）；hst ledger verify 自检或 hst ledger keygen --force 重建"
        );
    }
    Ok(ledger_client::Ledger::read_only(REPO_ID))
}

/// 密钥对生成（一次性或轮换）：自采 32 字节 seed（getrandom）写私钥密档
/// （hex，0600 原子落）再经 crate 回读公面，返回 (kid, JWK, 旧 kid)；
/// 私钥不打印不进 argv，公钥 JWK 供总台在册（在册后方可写入）。在位密
/// 档非 `--force` 即拒（防静默销毁，旧钥不可恢复）。
/// # Errors
///
/// 失败返回 `String` 错误（路径与写入类）。
pub fn keygen_write(force: bool) -> Result<(String, String, Option<String>), String> {
    let path = private_key_path()?;
    let old_kid = if path.exists() {
        let old = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| ledger_client::KeyPair::load_secret_hex(t.trim()).ok())
            .map(|k| sha256_hex(k.public_jwk.as_bytes()));
        if !force {
            return Err(format!(
                "密档已在位（{}）；重复生成会销毁在册私钥，确认轮换加 --force",
                path.display()
            ));
        }
        old
    } else {
        None
    };
    // crate 不导出私钥面（KeyPair.signing 私有），seed 本地采、hex 落档、
    // 回读取公面（kid 与 JWK 出 crate，派生约定单源）。
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| format!("getrandom: {e}"))?;
    let hex_seed: String = seed.iter().map(|b| format!("{b:02x}")).collect();
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    // 原子 0600：OpenOptions 带模式一次落，消两步间的 0644 窗。
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true).mode(0o600);
        let mut f = opts
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        use std::io::Write;
        f.write_all(hex_seed.as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    std::fs::write(&path, &hex_seed).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = ledger_client::KeyPair::load_secret_hex(&hex_seed).map_err(|e| e.to_string())?;
    Ok((key.key_id.clone(), key.public_jwk.clone(), old_kid))
}

/// 本地私钥与内置公钥 JWK 的配对自检：密档/env 缺位回 None（无法判），
/// 在位回配对与否——不配对则写入会全体 401 且难归因。
/// # Errors
///
/// 失败返回 `String` 错误（密钥读取类）。
pub fn pairing_ok() -> Result<Option<bool>, String> {
    let _ = migrate_legacy_vault()?;
    let seed = match read_seed_hex() {
        Ok(s) => s,
        Err(_) => return Ok(None), // absent：密档与 env 双缺位
    };
    let key = ledger_client::KeyPair::load_secret_hex(&seed).map_err(|e| {
        format!("密档在位但不可解析：{e}；v2.6.0 期 base64url 形由迁移器自动处理，若手动改坏可 hst ledger keygen --force 重建")
    })?;
    Ok(Some(key.public_jwk == PUBKEY_JWK))
}

/// digest 本地快检（服务端 DIGEST_RE 同源，不触网秒红）：`sha256:<64hex
/// 小写>`。
/// # Errors
///
/// 形不符返回 `String` 错误。
pub fn validate_digest(s: &str) -> Result<(), String> {
    let ok = s.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    });
    if ok {
        Ok(())
    } else {
        Err(format!("digest 须 sha256:<64hex 小写>，得 {s}"))
    }
}

/// issue list 的 limit 钳制（1 至 100，服务端上限；#52 家族标准）：钳制
/// 单源，命令面默认值与饱和提示判定共用。
pub fn clamp_issue_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

/// issue list 饱和判定（#52 家族标准）：返回条数不少于钳制后 limit 即示
/// 警；用 `>=` 不用 `==`，服务端若返回多于请求值，`==` 会静默漏报。
pub fn issue_list_saturated(returned: usize, eff: u32) -> bool {
    returned >= eff as usize
}

/// issue list 饱和提示行（#52/#53 家族标准）：返回条数打满钳制后 limit
///（或翻页面 has_more 为真）时出此行到 stderr，指向 `--limit` 提高加
/// `--before` 翻更旧一页与账本网页面；不饱和不出。
pub fn issue_list_truncation_hint(eff: u32) -> String {
    format!(
        "issue.list.truncated=limit-reached limit={eff} hint=返回条数打满 limit，可能仍有更多；提高 --limit（上限 100）、--before <id> 翻更旧一页，或账本网页面看全量 {}/repos/{REPO_ID}/issues",
        ledger_client::BASE_URL
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_id_is_sha256_of_canonical_jwk_const() {
        // kid = sha256hex(JWK 常量)；常量即字母键序紧凑形（单一真相）。
        let mut h = Sha256::new();
        h.update(PUBKEY_JWK.as_bytes());
        assert_eq!(key_id(), format!("{:x}", h.finalize()));
        assert!(PUBKEY_JWK.starts_with("{\"crv\":\"Ed25519\",\"kty\":\"OKP\",\"x\":\""));
        assert!(!PUBKEY_JWK.contains(' '));
    }

    #[test]
    fn digest_validation_accepts_lowercase_hex_only() {
        assert!(validate_digest(&format!("sha256:{}", "a".repeat(64))).is_ok());
        assert!(validate_digest("sha256:SHORT").is_err());
        assert!(
            validate_digest(&format!("sha256:{}", "A".repeat(64))).is_err(),
            "大写拒"
        );
        assert!(
            validate_digest(&format!("{}", "a".repeat(64))).is_err(),
            "缺前缀拒"
        );
    }

    #[test]
    fn crate_kid_convention_matches_local_derivation() {
        // crate 派生与本地常量派生同约定：同 JWK 同 kid。
        let key = ledger_client::KeyPair::generate();
        assert_eq!(key.key_id.len(), 64);
        assert_eq!(key.key_id, sha256_hex(key.public_jwk.as_bytes()));
    }

    #[test]
    fn truncation_hint_points_to_ledger_web_face() {
        let h = issue_list_truncation_hint(100);
        assert!(
            h.starts_with("issue.list.truncated=limit-reached limit=100"),
            "{h}"
        );
        assert!(h.contains("--before"), "{h}");
        assert!(h.contains(ledger_client::BASE_URL), "{h}");
        assert!(h.contains(REPO_ID), "{h}");
    }

    #[test]
    fn keygen_write_and_load_roundtrip_isolated() {
        let _env_guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "hst-ledger-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::env::set_var("HST_ROOT", &root);
        let (kid, jwk, old) = keygen_write(false).unwrap();
        assert_eq!(kid.len(), 64);
        assert!(old.is_none(), "首代无旧 kid");
        assert!(jwk.contains("Ed25519"));
        // 密档 hex 形可回读并构造公面（kid 与 JWK 同串）。
        let text = std::fs::read_to_string(private_key_path().unwrap()).unwrap();
        let key = ledger_client::KeyPair::load_secret_hex(text.trim()).unwrap();
        assert_eq!(key.public_jwk, jwk);
        assert_eq!(key.key_id, kid);
        // 覆盖守卫：在位密档非 force 拒、force 过且带出旧 kid。
        assert!(keygen_write(false).is_err());
        let (_, _, old2) = keygen_write(true).unwrap();
        assert_eq!(old2, Some(kid.clone()), "force 带出旧 kid");
        std::env::remove_var("HST_ROOT");
        let _ = std::fs::remove_dir_all(&root);
    }
}
