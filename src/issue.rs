//! hst issue 面（REQ-057 对齐，总台统一入口 issues.ohmygh.com）：agent
//! 使用过程中遇缺陷一键反馈，自动带上下文（tool=hst 加版本加平台加主机名）。
//! 契约真源 = ohmycloud docs/requirements/REQ-057（POST /api/issues 体与
//! 校验、GET 列表与详情、每 IP 10 条每时限速）；本模块是客户端投影，
//! env `HST_ISSUES_API` 覆盖基址（测与灰度）。

use serde_json::{json, Value};

/// 统一 issue 入口的提交回执面（细则见模块文档）。
#[derive(Debug)]
pub struct Filed {
    /// 服务端分配的 issue id。
    pub id: u64,
    /// 详情页 URL（/i/<id>）。
    pub url: String,
}

/// API 基址（env `HST_ISSUES_API` 覆盖，缺省总台统一入口）。
pub fn base_url() -> String {
    std::env::var("HST_ISSUES_API").unwrap_or_else(|_| "https://issues.ohmygh.com".into())
}

/// 契约长度单位 = UTF-16 code units（真源 JS .length；codex 三面评审 G2：
/// chars().count() 对 emoji 类会计少，服务端拒 400 而客户端自以为过）。
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// 按 UTF-16 上限截断（取不切裂代理对的最大前缀）。
fn truncate_utf16(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in s.chars() {
        let u = c.len_utf16();
        if n + u > max {
            break;
        }
        n += u;
        out.push(c);
    }
    out
}

/// 平台串（os-arch 形，契约至多 64 字符，客户端先截断）。
fn platform_string() -> String {
    truncate_utf16(
        &format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        64,
    )
}

/// 主机名（HOSTNAME 或 COMPUTERNAME 兜底 unknown，至多 64）。
fn host_string() -> String {
    let h = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    truncate_utf16(&h, 64)
}

/// 非 2xx 统一透传：读体取服务端 error 文案（G1：ureq2 对非 2xx 直接返
/// Err，不读体会丢 429 限速与 400 校验的中文文案），体不可解析回落截断原文。
fn status_error(code: u16, resp: ureq::Response) -> String {
    let body = resp.into_string().unwrap_or_default();
    let err = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v["error"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    format!("status={code} error={err}")
}

/// 提交面（new）：title trim 后 1 至 200、body 至多 20000、version 至多 40
/// （客户端先截断，长度单位 UTF-16）；201 回 {ok,id,url}；429 每 IP 限速；
/// 400 校验不过（服务端 error 文案透传）。
/// # Errors
///
/// 失败返回 `String` 错误（校验不过、网络与解析类、服务端 error 透传）。
pub fn file_issue(title: &str, body: &str) -> Result<Filed, String> {
    let title = title.trim();
    let n = utf16_len(title);
    if n == 0 || n > 200 {
        return Err(format!(
            "title must be 1-200 UTF-16 units after trim, got {n}"
        ));
    }
    let payload = json!({
        "tool": "hst",
        "title": title,
        "body": truncate_utf16(body, 20_000),
        "version": truncate_utf16(env!("CARGO_PKG_VERSION"), 40),
        "platform": platform_string(),
        "host": host_string(),
    });
    let url = format!("{}/api/issues", base_url());
    let resp = match ureq::post(&url)
        .set("User-Agent", "hst-issue")
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(20))
        .send_string(&payload.to_string())
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => return Err(status_error(code, r)),
        Err(e) => return Err(format!("post {url}: {e}")),
    };
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("read receipt body: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("parse receipt: {e}"))?;
    if status == 201 && v["ok"] == json!(true) {
        Ok(Filed {
            id: v["id"].as_u64().ok_or("receipt id missing")?,
            url: v["url"].as_str().ok_or("receipt url missing")?.to_string(),
        })
    } else {
        Err(format!(
            "file failed status={status} error={}",
            v["error"].as_str().unwrap_or("-")
        ))
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（校验不过、网络与解析类、服务端 error 透传）。
/// 列表面（list）：GET /api/issues?tool=&status=&limit=（limit 1 至 100，
/// 新到旧）；回 {ok,count,issues[]}。
pub fn list_issues(tool: &str, status: Option<&str>, limit: u32) -> Result<Vec<Value>, String> {
    let limit = limit.clamp(1, 100);
    let mut url = format!("{}/api/issues?tool={}&limit={}", base_url(), tool, limit);
    if let Some(s) = status {
        url.push_str(&format!("&status={s}"));
    }
    let resp = match ureq::get(&url)
        .set("User-Agent", "hst-issue")
        .timeout(std::time::Duration::from_secs(20))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => return Err(status_error(code, r)),
        Err(e) => return Err(format!("get {url}: {e}")),
    };
    let status_code = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("read list body: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("parse list: {e}"))?;
    if status_code == 200 && v["ok"] == json!(true) {
        v["issues"]
            .as_array()
            .cloned()
            .ok_or_else(|| "issues field missing".to_string())
    } else {
        Err(format!(
            "list failed status={status_code} error={}",
            v["error"].as_str().unwrap_or("-")
        ))
    }
}

/// # Errors
///
/// 失败返回 `String` 错误（校验不过、网络与解析类、服务端 error 透传）。
/// 详情面（show）：GET /api/issues/<id>，回 {ok,issue{...body}}。
pub fn show_issue(id: &str) -> Result<Value, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("issue id must be numeric, got '{id}'"));
    }
    let url = format!("{}/api/issues/{id}", base_url());
    let resp = match ureq::get(&url)
        .set("User-Agent", "hst-issue")
        .timeout(std::time::Duration::from_secs(20))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => return Err(status_error(code, r)),
        Err(e) => return Err(format!("get {url}: {e}")),
    };
    let status_code = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("read issue body: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("parse issue: {e}"))?;
    if status_code == 200 && v["ok"] == json!(true) {
        Ok(v["issue"].clone())
    } else {
        Err(format!(
            "show failed status={status_code} error={}",
            v["error"].as_str().unwrap_or("-")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_and_host_are_truncated_shapes() {
        assert!(platform_string().chars().count() <= 64);
        assert!(platform_string().contains(std::env::consts::OS));
        assert!(host_string().chars().count() <= 64);
        assert!(!host_string().is_empty());
    }

    #[test]
    fn title_validation_is_local_before_network() {
        // 空标题与超长标题在本地拒绝（不触网）。
        assert!(file_issue("   ", "x").is_err());
        let long = "t".repeat(201);
        let e = file_issue(&long, "x").unwrap_err();
        assert!(e.contains("1-200"), "{e}");
        // 边界合法形（不实际发网：无效基址必网络错，验证校验先行通过）。
        std::env::set_var("HST_ISSUES_API", "https://invalid.invalid");
        let r = file_issue("ok-title", "b");
        std::env::remove_var("HST_ISSUES_API");
        assert!(
            r.is_err(),
            "invalid base must fail at network, not validation"
        );
    }

    #[test]
    fn utf16_units_count_and_truncate_keep_pairs() {
        // G2：契约单位 = UTF-16（emoji 星面字符计 2），截断不切裂代理对。
        assert_eq!(utf16_len("a"), 1);
        assert_eq!(utf16_len("\u{1F600}"), 2);
        let s = "ab\u{1F600}c";
        assert_eq!(truncate_utf16(s, 3), "ab");
        assert_eq!(truncate_utf16(s, 4), "ab\u{1F600}");
        assert_eq!(truncate_utf16(s, 99), s);
    }

    #[test]
    fn show_rejects_non_numeric_id_locally() {
        assert!(show_issue("abc").is_err());
        assert!(show_issue("").is_err());
    }
}
