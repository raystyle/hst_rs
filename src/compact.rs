//! claude 压缩触发（auto-compact）配置面（总台功能单 2026-09-23，ledger
//! issue n6）。配置面实证（本机 claude 2.1.270 二进制反混淆，2026-09-23）：
//! 百分比阈值唯一面 = 环境变量 `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`
//! （parseFloat，0 < 值 <= 100，百分数刻度，"70" 即 70%；二进制内名
//! testPctOverride，官方文档无此键，未文档面，升级可能变）；窗口面 =
//! 环境变量 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`（裸 token 数，界
//! [100000, 1000000]，优先于 settings 顶层键 `autoCompactWindow`、
//! /autocompact 命令与 --autocompact 旗标）；`autoCompactEnabled` 顶层
//! 键缺省 true。阈值算式：有效窗口 = 窗口 - min(模型输出上限, 20000)；
//! 阈值 = min(floor(有效窗口 × pct / 100), 有效窗口 - 13000)；无 pct 时
//! 阈值 = 有效窗口 - 13000（缺省）。`CLAUDE_CODE_AUTO_COMPACT_THRESHOLD`
//! 不存在；`autoCompactThreshold` 是计算态遥测字段非配置键。hst 写面 =
//! `hst init --compact-pct <1-100|off> [--yes]`（预览不加 --yes），
//! 读面 = `hst doctor` 的 claude compact 检查项。

use std::path::Path;

use serde_json::{json, Value as Json};

/// env 键：百分比阈值覆盖（未文档面，0 < 值 <= 100 百分数刻度）。
pub const ENV_PCT: &str = "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE";
/// env 键：窗口覆盖（裸 token 数，优先于 settings 键与命令面）。
pub const ENV_WINDOW: &str = "CLAUDE_CODE_AUTO_COMPACT_WINDOW";

/// 输出预留上限（阈值算式常量，随二进制实证）。
pub const OUTPUT_RESERVE_CAP: u64 = 20_000;
/// 缺省阈值预留（无 pct 时阈值 = 有效窗口 - 此值）。
pub const DEFAULT_RESERVE: u64 = 13_000;

/// 压缩触发配置快照（原文保留，解析宽容：坏值不报错按缺省口径报告）。
pub struct CompactState {
    /// pct 覆盖 env 原文（parse 后 1 至 100 才算生效）。
    pub pct_raw: Option<String>,
    /// 窗口覆盖 env 原文。
    pub window_env_raw: Option<String>,
    /// settings 顶层 autoCompactWindow（env 缺席才生效）。
    pub window_setting: Option<u64>,
    /// settings 顶层 autoCompactEnabled（None = 缺省 true）。
    pub enabled: Option<bool>,
}

/// 读用户级 `~/.claude/settings.json` 压缩触发快照（BOM 容忍，文件
/// 缺席 = 全空快照）。
///
/// # Errors
///
/// 文件在而 JSON 坏损时返回 `String` 错误（不吞真数据问题）。
pub fn read_state(user_home: &Path) -> Result<CompactState, String> {
    let v = crate::yolo::read_json(&user_home.join(".claude").join("settings.json"))?;
    let env = v.get("env").and_then(|e| e.as_object());
    let grab = |k: &str| -> Option<String> {
        env.and_then(|o| o.get(k))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
    };
    Ok(CompactState {
        pct_raw: grab(ENV_PCT),
        window_env_raw: grab(ENV_WINDOW),
        window_setting: v.get("autoCompactWindow").and_then(|x| x.as_u64()),
        enabled: v.get("autoCompactEnabled").and_then(|x| x.as_bool()),
    })
}

impl CompactState {
    /// 生效 pct（env 原文 parse 为 1 至 100 的整数或小数；坏值 None 走
    /// 缺省口径）。
    pub fn effective_pct(&self) -> Option<f64> {
        self.pct_raw
            .as_deref()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|p| *p > 0.0 && *p <= 100.0)
    }

    /// 生效窗口：env 覆盖优先（parse 宽容），回落 settings 键；两者皆无
    /// None（模型缺省窗口，本工具不可知）。
    pub fn effective_window(&self) -> Option<u64> {
        if let Some(w) = self
            .window_env_raw
            .as_deref()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|w| *w >= 100_000 && *w <= 1_000_000)
        {
            return Some(w);
        }
        self.window_setting
            .filter(|w| *w >= 100_000 && *w <= 1_000_000)
    }

    /// 阈值 tokens 近似口径（输出预留按 20000 上限取，模型输出上限小于
    /// 此值时实际阈值更高）：返回 (阈值, 是否 pct 生效)；窗口未知 None。
    pub fn threshold_approx(&self) -> Option<(u64, bool)> {
        let window = self.effective_window()?;
        let effective = window.saturating_sub(OUTPUT_RESERVE_CAP);
        let default_thr = effective.saturating_sub(DEFAULT_RESERVE);
        match self.effective_pct() {
            Some(p) => Some((
                ((effective as f64 * p / 100.0).floor() as u64).min(default_thr),
                true,
            )),
            None => Some((default_thr, false)),
        }
    }

    /// doctor detail 单行（人读事实面：pct、窗口与来源、开关、近似阈值）。
    pub fn detail_line(&self) -> String {
        let pct = match (&self.pct_raw, self.effective_pct()) {
            (Some(raw), Some(_pct)) => format!("pct={raw}"),
            (Some(raw), None) => format!("pct={raw}(坏值,走缺省)"),
            (None, _) => "pct=unset(缺省=有效窗口-13000)".to_string(),
        };
        let window = match self.effective_window() {
            Some(w) => format!(
                "window={w}(src={})",
                if self.window_env_raw.is_some() {
                    "env"
                } else {
                    "settings"
                }
            ),
            None => "window=unset(模型缺省,阈值不可算)".to_string(),
        };
        let enabled = match self.enabled {
            Some(b) => format!("enabled={b}"),
            None => "enabled=true(缺省)".to_string(),
        };
        let thr = match self.threshold_approx() {
            Some((t, by_pct)) => format!(" threshold~{t}(by_pct={by_pct})"),
            None => String::new(),
        };
        format!("{pct} {window} {enabled}{thr} (pct键未文档,2.1.270实证)")
    }
}

/// 写 pct（1 至 100）或摘键（None）到用户级 settings env 块，读改写保留
/// 其余键；目标态与现态等值时零写（幂等）。返回变更文件路径清单。
///
/// # Panics
///
/// 正常路径不 panic；内部 unwrap 仅出现在构造不变量上（env 键刚建必为
/// 对象）。
/// # Errors
///
/// 文件在而 JSON 坏损或 IO 失败时返回 `String` 错误。
pub fn apply_pct(user_home: &Path, pct: Option<u8>) -> Result<Vec<String>, String> {
    let path = user_home.join(".claude").join("settings.json");
    let mut v = crate::yolo::read_json(&path)?;
    if !v.is_object() {
        v = json!({});
    }
    let cur = v
        .get("env")
        .and_then(|e| e.get(ENV_PCT))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let want = pct.map(|p| p.to_string());
    if cur == want {
        return Ok(Vec::new());
    }
    let obj = v.as_object_mut().unwrap();
    match pct {
        Some(p) => {
            let env = obj.entry("env".to_string()).or_insert_with(|| json!({}));
            if !env.is_object() {
                *env = json!({});
            }
            env.as_object_mut()
                .unwrap()
                .insert(ENV_PCT.into(), Json::String(p.to_string()));
        }
        None => {
            if let Some(env) = obj.get_mut("env").and_then(|e| e.as_object_mut()) {
                env.remove(ENV_PCT);
            }
            // env 空对象不摘键（保留结构最小侵入；hst 不own整个 env 块）。
        }
    }
    crate::yolo::write_json(&path, &v)?;
    Ok(vec![path.display().to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_home(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hst-compact-test-{}-{tag}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(p.join(".claude")).unwrap();
        p
    }

    #[test]
    fn apply_pct_roundtrip_preserves_siblings_and_idempotent() {
        let home = fresh_home("rt");
        let settings = home.join(".claude").join("settings.json");
        std::fs::write(
            &settings,
            format!(
                "{{\"env\": {{\"{ENV_WINDOW}\": \"1000000\", \"FOO\": \"bar\"}}, \"model\": \"x\"}}"
            ),
        )
        .unwrap();
        // set：三键并存,外来键保留。
        let changed = apply_pct(&home, Some(70)).unwrap();
        assert_eq!(changed.len(), 1);
        let v: Json = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(v["env"][ENV_PCT], "70");
        assert_eq!(v["env"]["FOO"], "bar");
        assert_eq!(v["env"][ENV_WINDOW], "1000000");
        assert_eq!(v["model"], "x");
        // 同值幂等零写。
        assert!(apply_pct(&home, Some(70)).unwrap().is_empty());
        // off：摘键,兄弟键幸存。
        let changed = apply_pct(&home, None).unwrap();
        assert_eq!(changed.len(), 1);
        let v: Json = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert!(v["env"].get(ENV_PCT).is_none());
        assert_eq!(v["env"]["FOO"], "bar");
        // 再 off 幂等。
        assert!(apply_pct(&home, None).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn threshold_math_matches_binary_semantics() {
        // 1M 窗口 70%:有效 980000,阈值 686000,by_pct。
        let cs = CompactState {
            pct_raw: Some("70".into()),
            window_env_raw: Some("1000000".into()),
            window_setting: None,
            enabled: None,
        };
        assert_eq!(cs.threshold_approx(), Some((686_000, true)));
        // 无 pct:缺省 = 有效 - 13000。
        let cs = CompactState {
            pct_raw: None,
            window_env_raw: Some("1000000".into()),
            window_setting: None,
            enabled: None,
        };
        assert_eq!(cs.threshold_approx(), Some((967_000, false)));
        // 钳制:99% 的 floor(970200) 钳到缺省 967000。
        let cs = CompactState {
            pct_raw: Some("99".into()),
            window_env_raw: Some("1000000".into()),
            window_setting: None,
            enabled: None,
        };
        assert_eq!(cs.threshold_approx(), Some((967_000, true)));
        // env 缺席回落 settings 键:200k 窗口缺省 167000。
        let cs = CompactState {
            pct_raw: None,
            window_env_raw: None,
            window_setting: Some(200_000),
            enabled: Some(true),
        };
        assert_eq!(cs.effective_window(), Some(200_000));
        assert_eq!(cs.threshold_approx(), Some((167_000, false)));
        // 坏值宽容:pct "abc" 与窗口 "99" 都按缺省口径。
        let cs = CompactState {
            pct_raw: Some("abc".into()),
            window_env_raw: Some("99".into()),
            window_setting: None,
            enabled: None,
        };
        assert!(cs.effective_pct().is_none());
        assert!(cs.effective_window().is_none());
        assert_eq!(cs.threshold_approx(), None);
        // 窗口未知但 pct 在场:detail_line 不 panic。
        assert!(cs.detail_line().contains("pct=abc"));
    }

    #[test]
    fn read_state_parses_user_settings() {
        let home = fresh_home("rd");
        let settings = home.join(".claude").join("settings.json");
        std::fs::write(
            &settings,
            format!(
                "{{\"env\": {{\"{ENV_PCT}\": \"70\", \"{ENV_WINDOW}\": \"1000000\"}}, \"autoCompactEnabled\": false}}"
            ),
        )
        .unwrap();
        let cs = read_state(&home).unwrap();
        assert_eq!(cs.pct_raw.as_deref(), Some("70"));
        assert_eq!(cs.window_env_raw.as_deref(), Some("1000000"));
        assert_eq!(cs.enabled, Some(false));
        assert_eq!(cs.threshold_approx(), Some((686_000, true)));
        assert!(cs.detail_line().contains("enabled=false"));
        // 文件缺席 = 全空快照。
        std::fs::remove_file(&settings).unwrap();
        let cs = read_state(&home).unwrap();
        assert!(cs.pct_raw.is_none());
        assert!(cs.effective_window().is_none());
        let _ = std::fs::remove_dir_all(&home);
    }
}
