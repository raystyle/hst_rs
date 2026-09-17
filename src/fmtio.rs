//! 全局输出格式（issue #1 总台集成契约，与 ome S003 同构）：
//! `--format kv|json|jsonl`，`--json` 为 json 简写（互斥）。kv 是人读
//! marker 行（缺省）；json 出 `{ok,data|error,meta}` 信封（P0015 三传输
//! 同形——oma 与 ome 裸数据裁决的分道点，三传输复用优先，契约文档记档）；
//! jsonl 是列表型数据的逐行对象（数据即数据，无信封）。
//!
//! 结构化模式（json/jsonl）下错误走 stderr 单行 JSON `{"code":"error",
//! "message":...}`，stdout 保持纯数据；kv 模式错误 `hst: <e>`。
//! serde_json 开 preserve_order：JSON 字段序与 kv 行序一致（ome S003 实证
//! 教训——默认 BTreeMap 字母序会打乱）。

use serde_json::Value;

/// 响应信封（S016 吸收，原 api.rs；P0011 删除后归位本模块）：CLI `--json`
/// 吐它。形：`{ok, data|error, meta:{command, project}}`。
pub fn envelope(command: &str, root: &std::path::Path, outcome: Result<Value, String>) -> Value {
    let mut v = serde_json::json!({
        "ok": outcome.is_ok(),
        "meta": { "command": command, "project": root.display().to_string() },
    });
    match outcome {
        Ok(d) => v["data"] = d,
        Err(e) => v["error"] = Value::String(e),
    }
    v
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// 输出与信封的Format面（细则见模块文档与集成测试）。
pub enum Format {
    /// 该字段承载输出与信封的Kv数据。
    Kv,
    /// 该字段承载输出与信封的Json数据。
    Json,
    /// 该字段承载输出与信封的Jsonl数据。
    Jsonl,
}

static MODE: std::sync::OnceLock<Format> = std::sync::OnceLock::new();

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 启动期设置一次（main 解析后、分派前）；`--json` 与 `--format` 的互斥
/// 由 clap `conflicts_with` 保证。
pub fn init(json_shorthand: bool, format: Option<&str>) -> Result<Format, String> {
    let mode = if json_shorthand {
        Format::Json
    } else {
        match format {
            None | Some("kv") => Format::Kv,
            Some("json") => Format::Json,
            Some("jsonl") => Format::Jsonl,
            Some(bad) => return Err(format!("未知 --format：{bad}（kv|json|jsonl）")),
        }
    };
    let _ = MODE.set(mode);
    Ok(mode)
}

/// 输出与信封的mode面（细则见模块文档与集成测试）。
pub fn mode() -> Format {
    MODE.get().copied().unwrap_or(Format::Kv)
}

/// 结构化模式（错误走单行 JSON、stdout 纯数据）。
pub fn structured() -> bool {
    matches!(mode(), Format::Json | Format::Jsonl)
}

/// main 错误出口：结构化模式 stderr 单行 JSON，kv 模式人称行；退出码 1。
pub fn error_exit(e: String) -> ! {
    if structured() {
        let obj = serde_json::json!({ "code": "error", "message": e });
        eprintln!("{obj}");
    } else {
        eprintln!("hst: {e}");
    }
    std::process::exit(1)
}

/// jsonl 模式：逐行对象（无信封）。列表型命令用；非列表命令 jsonl 视同
/// json（信封单对象），由调用方分支。
pub fn print_jsonl(rows: &[Value]) {
    for r in rows {
        println!("{r}");
    }
}
