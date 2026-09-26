//! herdr 本地 NDJSON RPC 最小客户端（ADR-0009、REQ-023）：仅 `ping` 探
//! 活与 `agent.prompt`（带 wait）派发两方法，一连接一请求短连接，不做
//! 事件流长连接。传输零新依赖：Unix 走 std UnixStream，Windows 命名管
//! 道按文件开（`\\.\pipe\` 前缀映射，对齐 herdr 二进制 strings 实证）；
//! serde_json 组帧；sha256 请求 id（tokio/thiserror/uuid/dirs 不进
//! Cargo.toml）。socket 路径解析序：`HERDR_SOCKET_PATH` 到
//! `HERDR_SESSION`（`~/.config/herdr/sessions/<名>/herdr.sock`）到缺省
//! `~/.config/herdr/herdr.sock`。本通道不注入任何 token：本地 socket
//! 权限（srw-------）即边界（评审 F2 回填；用户工件情报
//! HERDR_SOCKET_PASSWORD 经 schema 加文档加二进制加全站检索四路核实
//! 非 herdr 机制）。

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};

use crate::pathutil::user_home;

/// herdr RPC 面错误（REQ-023）：`Display` 面向人，`code` 给 agent 短码
/// （api 变体透传 herdr 错误码如 agent_blocked、agent_not_found）。
#[derive(Debug)]
pub enum HerdrError {
    /// socket 路径不在场（herdr 未跑或路径配错；携带路径）。
    NoSocket(String),
    /// 连接失败（携带路径与原因）。
    Connect {
        /// 连接目标路径。
        path: String,
        /// 连接失败原因。
        cause: String,
    },
    /// 读写或序列化失败（携带原因）。
    Io(String),
    /// 响应不合约：id 不配对、非对象、无 result 无 error（携带原因）。
    Protocol(String),
    /// herdr API 错误（code 透传 herdr 侧错误码）。
    Api {
        /// herdr 错误码（如 agent_not_found、timeout）。
        code: String,
        /// herdr 错误消息。
        message: String,
    },
    /// 目标 agent 回执 blocked 态（权限或提问框在屏；指令不落 Enter）。
    AgentBlocked(String),
}

impl HerdrError {
    /// agent 面稳定短码（api 变体透传 herdr 错误码）。
    pub fn code(&self) -> String {
        match self {
            HerdrError::NoSocket(_) => "no_socket".into(),
            HerdrError::Connect { .. } => "connect".into(),
            HerdrError::Io(_) => "io".into(),
            HerdrError::Protocol(_) => "protocol".into(),
            HerdrError::Api { code, .. } => code.clone(),
            HerdrError::AgentBlocked(_) => "agent_blocked".into(),
        }
    }
}

impl std::fmt::Display for HerdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HerdrError::NoSocket(path) => {
                write!(f, "herdr socket not found at {path} (is herdr running?)")
            }
            HerdrError::Connect { path, cause } => write!(f, "connect {path}: {cause}"),
            HerdrError::Io(cause) => write!(f, "herdr rpc io: {cause}"),
            HerdrError::Protocol(cause) => write!(f, "herdr rpc protocol violation: {cause}"),
            HerdrError::Api { code, message } => write!(f, "herdr api {code}: {message}"),
            HerdrError::AgentBlocked(detail) => {
                write!(
                    f,
                    "target agent blocked (dialog on screen), prompt not executed: {detail}"
                )
            }
        }
    }
}

/// socket 路径解析：`HERDR_SOCKET_PATH` 到 `HERDR_SESSION`
/// （`~/.config/herdr/sessions/<名>/herdr.sock`）到缺省
/// `~/.config/herdr/herdr.sock`（序同 herdr 官方文档）。
///
/// # Errors
///
/// 用户家目录不可解析时返回 [`HerdrError::Io`]。
pub fn socket_path() -> Result<PathBuf, HerdrError> {
    if let Some(p) = std::env::var("HERDR_SOCKET_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return Ok(PathBuf::from(p.trim()));
    }
    let cfg = user_home()
        .map_err(HerdrError::Io)?
        .join(".config")
        .join("herdr");
    if let Some(name) = std::env::var("HERDR_SESSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return Ok(cfg.join("sessions").join(name.trim()).join("herdr.sock"));
    }
    Ok(cfg.join("herdr.sock"))
}

/// Windows 管道名：路径串前缀 `\\.\pipe\`（对齐 herdr 二进制
/// `\\.\pipe\${socketPath}` 映射）；已带前缀的原样透传。
#[cfg(windows)]
fn windows_pipe(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    if s.starts_with(r"\\.\pipe\") {
        s
    } else {
        format!(r"\\.\pipe\{s}")
    }
}

/// 一连接一请求的传输载体：Unix socket 流或 Windows 管道文件。
enum Conn {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    #[cfg(windows)]
    Pipe(std::fs::File),
}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.read(buf),
            #[cfg(windows)]
            Conn::Pipe(f) => f.read(buf),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.write(buf),
            #[cfg(windows)]
            Conn::Pipe(f) => f.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.flush(),
            #[cfg(windows)]
            Conn::Pipe(f) => f.flush(),
        }
    }
}

fn connect(path: &Path) -> Result<Conn, HerdrError> {
    #[cfg(unix)]
    {
        if !path.exists() {
            return Err(HerdrError::NoSocket(path.display().to_string()));
        }
        std::os::unix::net::UnixStream::connect(path).map_or_else(
            |e| {
                Err(HerdrError::Connect {
                    path: path.display().to_string(),
                    cause: e.to_string(),
                })
            },
            |s| {
                s.set_read_timeout(Some(std::time::Duration::from_secs(180)))
                    .map_err(|e| HerdrError::Io(e.to_string()))?;
                Ok(Conn::Unix(s))
            },
        )
    }
    #[cfg(windows)]
    {
        // 管道路径不做在场预判（fs 元数据对管道名不可靠），直开由错误分流。
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(windows_pipe(path))
            .map_or_else(
                |e| {
                    Err(HerdrError::Connect {
                        path: windows_pipe(path),
                        cause: e.to_string(),
                    })
                },
                |f| Ok(Conn::Pipe(f)),
            )
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(HerdrError::Io("unsupported platform".into()))
    }
}

/// 发一次 NDJSON RPC（一连接一请求）：写请求行，读一行响应，id 配对
/// 校验，error 分流为 [`HerdrError::Api`]，result 原样返回。
///
/// # Errors
///
/// socket 缺席、连接、读写、协议不合约或 herdr API 错误时返回
/// [`HerdrError`]。
pub fn call(method: &str, params: Json) -> Result<Json, HerdrError> {
    let path = socket_path()?;
    let mut conn = connect(&path)?;
    let id = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
                std::process::id()
            )
            .as_bytes()
        )
    )[..16]
        .to_string();
    // 评审 F2 回填：herdr 本地 socket 无口令机制（权限即边界），不注入
    // 任何 token；params 原样发。
    let req = json!({ "id": id, "method": method, "params": params });
    let line = serde_json::to_string(&req).map_err(|e| HerdrError::Io(e.to_string()))?;
    conn.write_all((line + "\n").as_bytes())
        .and_then(|_| conn.flush())
        .map_err(|e| HerdrError::Io(e.to_string()))?;
    let mut reader = BufReader::new(conn);
    let mut buf = String::new();
    reader
        .read_line(&mut buf)
        .map_err(|e| HerdrError::Io(e.to_string()))?;
    let resp: Json = serde_json::from_str(buf.trim())
        .map_err(|e| HerdrError::Protocol(format!("response not json: {e}")))?;
    match resp.get("id").and_then(|v| v.as_str()) {
        Some(rid) if rid == id => {}
        _ => {
            return Err(HerdrError::Protocol(format!(
                "response id mismatch: expected {id}, got {}",
                resp.get("id").cloned().unwrap_or(Json::Null)
            )))
        }
    }
    if let Some(e) = resp.get("error") {
        return Err(HerdrError::Api {
            code: e
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            message: e
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    Ok(resp.get("result").cloned().unwrap_or(Json::Null))
}

/// 探活：`ping` 一发（返回 pong result 原样）。
///
/// # Errors
///
/// 同 [`call`]。
pub fn ping() -> Result<Json, HerdrError> {
    call("ping", json!({}))
}

/// 派发即回执：`agent.prompt` 带 wait（until 三态含 blocked，blocked 单
/// 独分流为 [`HerdrError::AgentBlocked`]，绝不把指令打进权限框）。
///
/// # Errors
///
/// 同 [`call`]，另加目标 blocked 态。
pub fn agent_prompt(target: &str, text: &str, timeout_ms: u64) -> Result<Json, HerdrError> {
    let resp = call(
        "agent.prompt",
        json!({
            "target": target,
            "text": text,
            "wait": { "until": ["idle", "done", "blocked"], "timeout_ms": timeout_ms }
        }),
    )?;
    // 评审 F1 回填：真响应形是 {type: agent_prompted, agent: AgentInfo}，
    // agent_status 与 pane_id 嵌在 result.agent 下（schema 加真 socket
    // agent.list 同约定），不在顶层。
    let agent = resp.get("agent").cloned().unwrap_or(Json::Null);
    if agent.get("agent_status").and_then(|v| v.as_str()) == Some("blocked") {
        return Err(HerdrError::AgentBlocked(
            agent
                .get("pane_id")
                .and_then(|v| v.as_str())
                .unwrap_or(target)
                .to_string(),
        ));
    }
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_resolution_env_over_session_over_default() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = std::env::temp_dir().join(format!(
            "hst-herdr-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HST_USER_HOME", &home);
        std::env::remove_var("HERDR_SOCKET_PATH");
        // 缺省：~/.config/herdr/herdr.sock。
        std::env::remove_var("HERDR_SESSION");
        assert_eq!(
            socket_path().unwrap(),
            home.join(".config").join("herdr").join("herdr.sock")
        );
        // HERDR_SESSION：sessions/<名>/herdr.sock。
        std::env::set_var("HERDR_SESSION", "work");
        assert_eq!(
            socket_path().unwrap(),
            home.join(".config")
                .join("herdr")
                .join("sessions")
                .join("work")
                .join("herdr.sock")
        );
        // HERDR_SOCKET_PATH 最优先。
        std::env::set_var("HERDR_SOCKET_PATH", "/tmp/override.sock");
        assert_eq!(socket_path().unwrap(), PathBuf::from("/tmp/override.sock"));
        std::env::remove_var("HERDR_SOCKET_PATH");
        std::env::remove_var("HERDR_SESSION");
        std::env::remove_var("HST_USER_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(windows)]
    #[test]
    fn windows_pipe_prefix_mapping() {
        assert_eq!(
            windows_pipe(Path::new(r"C:\Users\me\.config\herdr\herdr.sock")),
            r"\\.\pipe\C:\Users\me\.config\herdr\herdr.sock"
        );
        assert_eq!(
            windows_pipe(Path::new(r"\\.\pipe\already")),
            r"\\.\pipe\already"
        );
    }
}
