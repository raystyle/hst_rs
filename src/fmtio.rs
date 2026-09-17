//! 全局输出格式（issue #1 总台集成契约，与 ome S003 同构）：
//! `--format kv|json|jsonl`，`--json` 为 json 简写（互斥）。kv 是人读
//! marker 行（缺省）；json 出 `{ok,data|error,meta}` 信封（P0015 三传输
//! 同形——oma 与 ome 裸数据裁决的分道点，三传输复用优先，契约文档记档）；
//! jsonl 是列表型数据的逐行对象（数据即数据，无信封）。
//!
//! 结构化模式（json/jsonl）下的失败回执分两形：经信封出口的失败（业务
//! 失败转发与 `--filter-output` 响错）双道，stdout 吐 ok:false 信封加 stderr
//! 单行 JSON `{"code":"error","message":...}`；不经信封的失败只走 stderr
//! 单行，stdout 无输出。两者退出码 1；kv 模式错误 `hst: <e>`。
//! serde_json 开 preserve_order：JSON 字段序与 kv 行序一致（ome S003 实证
//! 教训——默认 BTreeMap 字母序会打乱）。

use serde_json::Value;

/// 响应信封（S016 吸收，原 api.rs；P0011 删除后归位本模块）：CLI `--json`
/// 吐它。形：`{ok, data|error, meta:{command, project, duration_ms}}`；
/// duration_ms 自 fmtio::init 起计（cli-docs 采纳轮补，注入非手写）。
pub fn envelope(command: &str, root: &std::path::Path, outcome: Result<Value, String>) -> Value {
    let duration_ms = START
        .get()
        .map(|t| t.elapsed().as_millis() as u64)
        .unwrap_or(0);
    // --filter-output 在此单点作用于 data；未命中路径折成信封错误
    // （ok:false 加 error），上层照常 stderr 单行与退出 1。
    let outcome = match outcome {
        Ok(d) => match FILTER.get() {
            Some(f) => filter_output(&d, f),
            None => Ok(d),
        },
        Err(e) => Err(e),
    };
    let mut v = serde_json::json!({
        "ok": outcome.is_ok(),
        "meta": {
            "command": command,
            "project": root.display().to_string(),
            "duration_ms": duration_ms,
        },
    });
    match outcome {
        Ok(d) => v["data"] = d,
        Err(e) => v["error"] = Value::String(e),
    }
    v
}

/// `--filter-output <keys>`：json 信封 data 的键路径过滤（cli-docs 采纳轮）。
/// keys 逗号分隔多路径（括号感知，下标集内逗号不切分），路径点号嵌套，段可
/// 带数组下标（`items[0]` 单下标取元素、`items[0,2]` 多下标集成数组且为
/// 路径终点）。返回对象以原路径串为键、命中值为值。响错即 Err（不静默截
/// 断、不静默跳过；错误文案按类分道：键不存在、不是数组、下标越界带长度、
/// 多下标不可续导航），经信封错误出口承接。仅对出 json 信封的命令生效
/// （无信封命令面忽略）；kv 与 jsonl 模式在 init 期即拒。
///
/// # Errors
///
/// 路径为空、格式非法（如 `a[`）、键不存在或下标越界时返回 `String` 错误。
pub fn filter_output(data: &Value, keys: &str) -> Result<Value, String> {
    let mut out = serde_json::Map::new();
    for path in split_paths(keys) {
        let mut segs: Vec<Seg> = Vec::new();
        for raw in path.split('.') {
            segs.push(parse_seg(raw)?);
        }
        let v = pick(data, &segs, &path)?;
        out.insert(path.to_string(), v);
    }
    if out.is_empty() {
        return Err("--filter-output 未给出任何键路径".into());
    }
    Ok(Value::Object(out))
}

/// 括号感知的路径切分：`a[0,3],b` 切成 `a[0,3]` 与 `b`（下标集内的逗号
/// 不是路径分隔，cli-docs 标准例 `foo,bar.baz,a[0,3]` 口径）。切出的路径
/// 两侧去空格，空段丢弃。
fn split_paths(keys: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for c in keys.chars() {
        match c {
            '[' => {
                depth += 1;
                cur.push(c);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            ',' if depth == 0 => {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    paths.push(t);
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        paths.push(t);
    }
    paths
}

/// 递归取路径值：单下标取元素可续导航，多下标集合成数组且为终点；响错
/// （键缺、非数组、越界、多下标续导航）返回带因的错误（codex 评审 F2/G5：
/// 不静默截断不静默跳过，错误按类分道）。
fn pick(v: &Value, segs: &[Seg], path: &str) -> Result<Value, String> {
    let Some((seg, rest)) = segs.split_first() else {
        return Ok(v.clone());
    };
    let name = &seg.name;
    let Some(base) = v.get(name) else {
        return Err(format!("--filter-output 键不存在：{name}（路径 {path}）"));
    };
    let val: Value = match &seg.idx {
        None => base.clone(),
        Some(idx) => {
            let Some(arr) = base.as_array() else {
                return Err(format!(
                    "--filter-output 段 {name} 不是数组，不能带下标（路径 {path}）"
                ));
            };
            for i in idx {
                if *i >= arr.len() {
                    return Err(format!(
                        "--filter-output 下标 {i} 越界（段 {name} 长度 {}，路径 {path}）",
                        arr.len()
                    ));
                }
            }
            if idx.len() == 1 {
                arr[idx[0]].clone()
            } else {
                if !rest.is_empty() {
                    return Err(format!(
                        "--filter-output 多下标段 {name} 是终点，不可续导航（路径 {path}）"
                    ));
                }
                Value::Array(idx.iter().map(|i| arr[*i].clone()).collect())
            }
        }
    };
    if rest.is_empty() {
        Ok(val)
    } else {
        pick(&val, rest, path)
    }
}

/// 键路径一段：字段名加可选数组下标集。
struct Seg {
    name: String,
    idx: Option<Vec<usize>>,
}

fn parse_seg(raw: &str) -> Result<Seg, String> {
    let Some(open) = raw.find('[') else {
        if raw.is_empty() {
            return Err(format!("--filter-output 路径段为空：{raw}"));
        }
        return Ok(Seg {
            name: raw.to_string(),
            idx: None,
        });
    };
    if !raw.ends_with(']') {
        return Err(format!("--filter-output 路径段缺右括号：{raw}"));
    }
    let name = &raw[..open];
    if name.is_empty() {
        return Err(format!("--filter-output 路径段名为空：{raw}"));
    }
    let inner = &raw[open + 1..raw.len() - 1];
    if inner.trim().is_empty() {
        return Err(format!("--filter-output 下标集为空：{raw}"));
    }
    let mut idx = Vec::new();
    for i in inner.split(',').map(str::trim) {
        let n: usize = i
            .parse()
            .map_err(|_| format!("--filter-output 下标非数字：{i}"))?;
        idx.push(n);
    }
    Ok(Seg {
        name: name.to_string(),
        idx: Some(idx),
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// 输出格式三态：kv 人读 marker 行、json 信封、jsonl 逐行对象。
pub enum Format {
    /// kv：人读 marker 行（缺省）。
    Kv,
    /// json：`{ok,data|error,meta}` 信封。
    Json,
    /// jsonl：列表型数据逐行对象（无信封）。
    Jsonl,
}

static MODE: std::sync::OnceLock<Format> = std::sync::OnceLock::new();
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static FILTER: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 启动期设置一次（main 解析后、分派前）；`--json` 与 `--format` 的互斥
/// 由 clap `conflicts_with` 保证；`--filter-output` 只配 json 信封面，
/// 配 kv 或 jsonl 直接报错（静默忽略是 agent 陷阱）。
pub fn init(
    json_shorthand: bool,
    format: Option<&str>,
    filter_output: Option<&str>,
) -> Result<Format, String> {
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
    if let Some(f) = filter_output {
        if mode != Format::Json {
            return Err("--filter-output 只作用于 json 信封（--json 或 --format json）".into());
        }
        let _ = FILTER.set(f.to_string());
    }
    let _ = MODE.set(mode);
    let _ = START.set(std::time::Instant::now());
    Ok(mode)
}

/// 当前输出格式（kv 缺省）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(data: &Value, keys: &str) -> Result<Value, String> {
        filter_output(data, keys)
    }

    #[test]
    fn filter_output_top_and_nested_keys() {
        let data = json!({ "blocked": true, "meta": { "n": 2 } });
        let v = run(&data, "blocked,meta.n").unwrap();
        assert_eq!(v["blocked"], json!(true));
        assert_eq!(v["meta.n"], json!(2));
        // 键序保插入序（与请求序一致）。
        let keys: Vec<_> = v.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["blocked", "meta.n"]);
    }

    #[test]
    fn filter_output_array_indices() {
        let data = json!({ "items": [10, 20, 30] });
        // 单下标取元素。
        assert_eq!(run(&data, "items[0]").unwrap()["items[0]"], json!(10));
        // 多下标集合成数组。
        assert_eq!(
            run(&data, "items[0,2]").unwrap()["items[0,2]"],
            json!([10, 30])
        );
        // 嵌套后再下标。
        let nested = json!({ "a": { "b": [{ "c": 1 }, { "c": 2 }] } });
        assert_eq!(run(&nested, "a.b[1].c").unwrap()["a.b[1].c"], json!(2));
    }

    #[test]
    fn filter_output_misses_and_bad_paths_error() {
        let data = json!({ "ok": true });
        // 错误按类分道（codex 评审 F2/G5）：不静默截断，文案指因。
        let e = run(&data, "nope").unwrap_err();
        assert!(e.contains("键不存在"), "{e}");
        let e = run(&data, "ok[0]").unwrap_err();
        assert!(e.contains("不是数组"), "{e}");
        let e = run(&data, "a[b").unwrap_err();
        assert!(e.contains("缺右括号"), "{e}");
        let e = run(&data, "a[]").unwrap_err();
        assert!(e.contains("下标集为空"), "{e}");
        let e = run(&data, "a[-1]").unwrap_err();
        assert!(e.contains("下标非数字"), "{e}");
        assert!(run(&data, " , ").is_err());
        // 空格容错：路径两侧空格被 trim。
        assert!(run(&data, " ok ").unwrap()["ok"] == json!(true));
    }

    #[test]
    fn filter_output_out_of_bounds_is_loud() {
        // 多下标部分越界不静默截短（codex 评审 F2 实弹反例：findings[0,99]）。
        let data = json!({ "items": [10, 20, 30] });
        let e = run(&data, "items[0,99]").unwrap_err();
        assert!(e.contains("越界") && e.contains("99"), "{e}");
        let e = run(&data, "items[9]").unwrap_err();
        assert!(e.contains("越界"), "{e}");
        // 多下标是终点，续导航响错不折「未命中」。
        let nested = json!({ "a": { "b": [{ "c": 1 }, { "c": 2 }] } });
        let e = run(&nested, "a.b[0,1].c").unwrap_err();
        assert!(e.contains("不可续导航"), "{e}");
    }
}
