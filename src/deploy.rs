//! `hst init` hook deployment（D28：hook 注册与 shim 常驻用户级）。
//! 用户级注册面（用户裁 2026-09-11「hook 应用户全局」，对齐 codex 用户层）：
//! claude `~/.claude/settings.json`（settings 家族用户层生效，S015）、codex
//! `~/.codex/hooks.json` 加 `~/.codex/config.toml` features 与 trusted_hash
//! 预种、grok `~/.grok/hooks/ohmyagents-state.json`（global 层）、kimi
//! `~/.kimi-code/config.toml [[hooks]]`（kimi 仅用户级，S015）。shim 常驻
//! `~/.hst/hooks/`，状态按 session 分键写 `~/.hst/state/`（D28）。项目级
//! 旧注册与 `.oma/hooks/` 由 init 迁移退役（未 init 项目零数据根因消除）。
//! skill 面已退役（ADR-0005/D54）：两级生成面删，在位 ours 技能目录由
//! init 幂等清扫；AGENTS/CLAUDE 说明仍是项目级（项目内语义）。Schemas are
//! first-hand verified in S015 (official docs + openai/codex, xai-org/grok-build,
//! MoonshotAI/kimi-code sources).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use crate::yolo::{read_json, read_toml, toml_write, write_json, write_text};
#[cfg(test)]
use crate::yolo::ensure_parent;

#[derive(Default)]
/// 部署报告：写入、跳过、形态与警告清单。
pub struct DeployReport {
    /// 该字段承载部署的wrote数据。
    pub wrote: Vec<String>,
    /// 该字段承载部署的skipped数据。
    pub skipped: Vec<String>,
    /// Hook command form chosen this run: "user" (D28: registrations live in
    /// the four agents' user-level configs and point at the self-contained
    /// state writer in `~/.hst/hooks/`, zero hst dependency).
    pub form: Option<&'static str>,
    /// Advisory warnings (non-fatal), e.g. jq missing from PATH at deploy time
    /// (state shim falls back to findstr parsing).
    pub warns: Vec<String>,
}

/// oma-owned handler marker: the current exe, or a stale oma binary whose
/// entry should be replaced (path moved between builds). Matches the bare
/// name `oma`, `oma.exe`, and test-harness binaries like `oma-<hash>.exe`.
fn oma_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hst"))
}

/// token 的文件名 stem（剥路径、剥引号、剥 .exe 后缀）。
fn token_stem(token: &str) -> &str {
    token
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(token)
        .trim_matches('"')
        .trim_end_matches(".exe")
}

/// hst 现行形态加 oma 历史形态（heal 与重部署收敛旧注册都靠旧名可识）。
fn family_stem(stem: &str) -> bool {
    stem == "hst" || stem.starts_with("hst-") || stem == "oma" || stem.starts_with("oma-")
}

/// 解释器包裹形态（D39）的程序 token：cmd 系取 `/c` 后一 token、
/// powershell 系取 `-File` 后一 token；非包裹形返回 None（程序就是首
/// token）。入参须已小写（is_ours 消费侧）。
fn wrapped_program(lowered: &str) -> Option<&str> {
    let mut tokens = lowered.trim_start_matches('&').split_whitespace();
    let head = tokens.next()?;
    let head_stem = token_stem(head);
    if !matches!(head_stem, "cmd" | "powershell" | "pwsh") {
        return None;
    }
    let rest: Vec<&str> = tokens.collect();
    if head_stem == "cmd" {
        let start = usize::from(rest.first() == Some(&"/c"));
        return Some(rest.get(start).copied().unwrap_or(""));
    }
    rest.iter()
        .position(|t| *t == "-file")
        .and_then(|i| rest.get(i + 1).copied())
}

/// 命令串的程序 token（保持原大小写，doctor 判形共用，codex review F2）：
/// 包裹形态取解释器后的程序位（`powershell.exe ... -File <shim>` 的程序位
/// 是 shim 脚本，不是解释器路径），非包裹形取首 token。
pub(crate) fn program_token(command: &str) -> &str {
    let lowered = command.to_ascii_lowercase();
    if wrapped_program(&lowered).is_none() {
        return command
            .trim_start()
            .trim_start_matches('&')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches('"');
    }
    let mut tokens = command
        .trim_start()
        .trim_start_matches('&')
        .split_whitespace();
    let head = tokens.next().unwrap_or("");
    let head_lower = head.to_ascii_lowercase();
    let head_stem = token_stem(&head_lower);
    let rest: Vec<&str> = tokens.collect();
    if head_stem == "cmd" {
        let start = usize::from(
            rest.first()
                .map(|t| t.eq_ignore_ascii_case("/c"))
                .unwrap_or(false),
        );
        rest.get(start).copied().unwrap_or("").trim_matches('"')
    } else {
        rest.iter()
            .position(|t| t.eq_ignore_ascii_case("-file"))
            .and_then(|i| rest.get(i + 1).copied())
            .unwrap_or("")
            .trim_matches('"')
    }
}

pub(crate) fn is_ours(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if lower.contains(&oma_exe().display().to_string().to_ascii_lowercase()) {
        return true;
    }
    // First whitespace token covers both the exec form (bare path) and the
    // Grok shell form (`"C:\...\oma.exe" hook`). The codex Windows form puts
    // the PowerShell call operator first (`& "exe" hook`), so strip a leading
    // `&` before taking the token — without this the Windows-side field of a
    // shared project reads as foreign (doctor misses it; redeploy appends a
    // duplicate once the embedded exe path goes stale). D39：Windows 注册是
    // `powershell.exe ... -File <shim> agent` 解释器前缀形（claude 的 hook
    // 执行 shell 是 POSIX sh 系，直路径 .cmd 在 WSL 形不认盘符、cmd.exe /c
    // 在 Git Bash 被 MSYS 吃掉，宿主实弹 2026-09-13）；解释器头按 stem 认
    // （含全路径头）。包裹分支只认自家 shim 名与确切二进制名（codex review
    // F6：防 `cmd.exe /c C:\tools\hst-logger.exe` 类外来前缀族在清扫面被
    // 误伤），家族前缀只留给裸形态。
    if let Some(prog) = wrapped_program(&lower) {
        let stem = token_stem(prog);
        return stem.starts_with("hst-state")
            || stem.starts_with("oma-state")
            || stem == "hst"
            || stem == "oma";
    }
    family_stem(token_stem(
        lower
            .trim_start_matches('&')
            .split_whitespace()
            .next()
            .unwrap_or(""),
    ))
}

/// JSON arrays of handler groups under settings["hooks"][event], append-only:
/// drop stale oma entries, keep foreign ones, add ours exactly once.
fn merge_hook_event(settings: &mut Json, event: &str, our_handler: Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let groups = obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !groups.is_object() {
        *groups = json!({});
    }
    let groups = groups
        .as_object_mut()
        .ok_or_else(|| "hooks is not an object".to_string())?;
    let entry = groups.entry(event.to_string()).or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    let arr = entry
        .as_array_mut()
        .ok_or_else(|| "event groups is not an array".to_string())?;
    let mut changed = false;
    // ours 条目的陈旧判据是「不等于本次要写的 shim 命令」——老形态（bare
    // oma、旧 exe 绝对路径、D27 项目级 shim 路径）、重复条目都覆盖：弃后
    // 统一补一条现行注册，重部署恒单条（D28 后现行命令指向 ~/.hst/hooks/）。
    let ours_cmd = our_handler
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stale = |c: &str| -> bool { is_ours(c) && !c.to_ascii_lowercase().eq(&ours_cmd) };
    for group in arr.iter_mut() {
        if let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        {
            let before = hooks.len();
            hooks.retain(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| !stale(c))
                    .unwrap_or(true)
            });
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    arr.retain(|g| {
        !g.as_object().is_some_and(|g| {
            g.get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|a| a.is_empty())
        })
    });
    // Update an existing oma handler in place (byte-equal survivors are
    // no-ops), or append when none survived. Same-shape duplicates collapse:
    // after the update pass every ours-handler equals our_handler, so keep
    // the first and drop the rest (review-verified: identical duplicates
    // previously survived redeploy).
    let mut replaced = false;
    let mut seen_current = false;
    for group in arr.iter_mut() {
        let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        else {
            continue;
        };
        let before = hooks.len();
        hooks.retain(|handler| {
            let ours = handler
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| is_ours(c))
                .unwrap_or(false);
            if !ours {
                return true;
            }
            if *handler == our_handler && !seen_current {
                seen_current = true;
                return true;
            }
            false
        });
        if hooks.len() != before {
            changed = true;
        }
        for handler in hooks.iter_mut() {
            let ours = handler
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| is_ours(c))
                .unwrap_or(false);
            if ours && *handler != our_handler {
                *handler = our_handler.clone();
                changed = true;
            }
            if ours {
                replaced = true;
            }
        }
    }
    if !replaced {
        arr.push(json!({ "matcher": "*", "hooks": [our_handler] }));
        changed = true;
    }
    Ok(changed)
}

/// 全文件清扫异形态 ours 残留（D39 第 2 轮，宿主终验回执 2026-09-13）：
/// 只在自管事件集内做陈旧替换会漏——非管理事件里的 ours 旧形态行（历史
/// 版本事件集变更或手工救济行残留）加新不清旧即每事件双注册并存、坏行
/// 持续报错（宿主手清 8 条实证）。清扫只动 `managed` 之外的**非管理事
/// 件**：ours 行整弃（该事件已不归本面管）、空组与空事件键收尾；管理事
/// 件与外来条目永不碰（管理事件由 merge 的陈旧替换与字段保留语义照旧，
/// 异侧字段按字节保留语义不动）。
fn sweep_unmanaged_ours(settings: &mut Json, managed: &[&str]) -> bool {
    let Some(obj) = settings.as_object_mut() else {
        return false;
    };
    let keys: Vec<String> = obj
        .get("hooks")
        .and_then(|h| h.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let mut changed = false;
    for event in keys {
        if managed.contains(&event.as_str()) {
            continue;
        }
        let Some(list) = obj
            .get_mut("hooks")
            .and_then(|h| h.as_object_mut())
            .and_then(|m| m.get_mut(&event))
            .and_then(|v| v.as_array_mut())
        else {
            continue;
        };
        for group in list.iter_mut() {
            if let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            {
                let before = hooks.len();
                hooks.retain(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .map(|c| !is_ours(c))
                        .unwrap_or(true)
                });
                if hooks.len() != before {
                    changed = true;
                }
            }
        }
        let before = list.len();
        list.retain(|g| {
            !g.as_object().is_some_and(|g| {
                g.get("hooks")
                    .and_then(|h| h.as_array())
                    .is_some_and(|a| a.is_empty())
            })
        });
        if list.len() != before || list.is_empty() {
            changed = true;
        }
        if list.is_empty() {
            obj.get_mut("hooks")
                .and_then(|h| h.as_object_mut())
                .map(|m| m.remove(&event));
        }
    }
    let hooks_now_empty = obj
        .get("hooks")
        .and_then(|h| h.as_object())
        .map(|m| m.is_empty())
        .unwrap_or(false);
    if hooks_now_empty {
        obj.remove("hooks");
    }
    changed
}

/// codex 版清扫（D39 第 2 轮）：同 `sweep_unmanaged_ours` 但只看本侧字段
/// 的 ours（异侧字段是对方 OS 的活注册，M042/M044 字段所有权语义）。
/// codex review F5：本侧 ours 且异侧携带外来活串时**只摘本侧键留对象**
///（异侧逐字节保留契约）；两侧皆 ours 或异侧空才整弃。
fn sweep_unmanaged_ours_codex(settings: &mut Json, managed: &[&str], side: OsSide) -> bool {
    let key = if side == OsSide::Windows {
        "commandWindows"
    } else {
        "command"
    };
    let other_key = if side == OsSide::Windows {
        "command"
    } else {
        "commandWindows"
    };
    let Some(obj) = settings.as_object_mut() else {
        return false;
    };
    let keys: Vec<String> = obj
        .get("hooks")
        .and_then(|h| h.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let mut changed = false;
    for event in keys {
        if managed.contains(&event.as_str()) {
            continue;
        }
        let Some(list) = obj
            .get_mut("hooks")
            .and_then(|h| h.as_object_mut())
            .and_then(|m| m.get_mut(&event))
            .and_then(|v| v.as_array_mut())
        else {
            continue;
        };
        for group in list.iter_mut() {
            if let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            {
                // 先外科摘键（F5）：本侧 ours 加异侧外来活串 → 只摘本侧。
                for h in hooks.iter_mut() {
                    let side_ours = h
                        .get(key)
                        .and_then(|c| c.as_str())
                        .map(is_ours)
                        .unwrap_or(false);
                    if !side_ours {
                        continue;
                    }
                    let other = h.get(other_key).and_then(|c| c.as_str()).unwrap_or("");
                    if !other.is_empty() && !is_ours(other) {
                        if let Some(o) = h.as_object_mut() {
                            o.remove(key);
                            changed = true;
                        }
                    }
                }
                let before = hooks.len();
                hooks.retain(|h| {
                    h.get(key)
                        .and_then(|c| c.as_str())
                        .map(|c| !is_ours(c))
                        .unwrap_or(true)
                });
                if hooks.len() != before {
                    changed = true;
                }
            }
        }
        let before = list.len();
        list.retain(|g| {
            !g.as_object().is_some_and(|g| {
                g.get("hooks")
                    .and_then(|h| h.as_array())
                    .is_some_and(|a| a.is_empty())
            })
        });
        if list.len() != before || list.is_empty() {
            changed = true;
        }
        if list.is_empty() {
            obj.get_mut("hooks")
                .and_then(|h| h.as_object_mut())
                .map(|m| m.remove(&event));
        }
    }
    let hooks_now_empty = obj
        .get("hooks")
        .and_then(|h| h.as_object())
        .map(|m| m.is_empty())
        .unwrap_or(false);
    if hooks_now_empty {
        obj.remove("hooks");
    }
    changed
}

/// codex 本侧字段的现行命令串（D39 第 2 轮抽出单一来源：sweep 与
/// codex_handler_value 共用）。
fn codex_side_command(oma: &Path, side: OsSide) -> String {
    match side {
        OsSide::Unix => format!(
            "\"{}\" codex",
            oma.join("hooks").join("hst-state.sh").display()
        ),
        OsSide::Windows => format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {} codex",
            crate::pathutil::forward_slash(&oma.join("hooks").join("hst-state.ps1"))
        ),
    }
}

/// oma-owned if either per-OS field names us (codex shape).
fn handler_is_ours(handler: &Json) -> bool {
    ["command", "commandWindows"].iter().any(|k| {
        handler
            .get(*k)
            .and_then(|c| c.as_str())
            .is_some_and(is_ours)
    })
}

/// 从 JSON 形 hook 注册（claude / grok）里剥除全部 ours 处理器；空组、空
/// 事件与空 hooks 对象一并清掉。返回是否变更（D28 项目面退役）。
fn strip_ours_handlers(settings: &mut Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let Some(groups) = obj.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(false);
    };
    let mut changed = false;
    for (_event, arr) in groups.iter_mut() {
        let Some(groups_arr) = arr.as_array_mut() else {
            continue;
        };
        for group in groups_arr.iter_mut() {
            let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            else {
                continue;
            };
            let before = hooks.len();
            hooks.retain(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| !is_ours(c))
                    .unwrap_or(true)
            });
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    // 空组与空事件键摘除；hooks 对象空了连键一起摘。
    for (_event, arr) in groups.iter_mut() {
        if let Some(groups_arr) = arr.as_array_mut() {
            let before = groups_arr.len();
            groups_arr.retain(|g| {
                !g.as_object().is_some_and(|g| {
                    g.get("hooks")
                        .and_then(|h| h.as_array())
                        .is_some_and(|a| a.is_empty())
                })
            });
            if groups_arr.len() != before {
                changed = true;
            }
        }
    }
    let before = groups.len();
    groups.retain(|_, arr| !arr.as_array().is_some_and(|a| a.is_empty()));
    if groups.len() != before {
        changed = true;
    }
    if groups.is_empty() {
        obj.remove("hooks");
    }
    Ok(changed)
}

/// codex 形（command/commandWindows 双字段）的 ours 剥除（D28 项目面退役）。
fn strip_ours_codex_handlers(settings: &mut Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let Some(groups) = obj.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(false);
    };
    let mut changed = false;
    for (_event, arr) in groups.iter_mut() {
        let Some(groups_arr) = arr.as_array_mut() else {
            continue;
        };
        for group in groups_arr.iter_mut() {
            let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            else {
                continue;
            };
            let before = hooks.len();
            hooks.retain(|h| !handler_is_ours(h));
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    for (_event, arr) in groups.iter_mut() {
        if let Some(groups_arr) = arr.as_array_mut() {
            let before = groups_arr.len();
            groups_arr.retain(|g| {
                !g.as_object().is_some_and(|g| {
                    g.get("hooks")
                        .and_then(|h| h.as_array())
                        .is_some_and(|a| a.is_empty())
                })
            });
            if groups_arr.len() != before {
                changed = true;
            }
        }
    }
    let before = groups.len();
    groups.retain(|_, arr| !arr.as_array().is_some_and(|a| a.is_empty()));
    if groups.len() != before {
        changed = true;
    }
    if groups.is_empty() {
        obj.remove("hooks");
    }
    Ok(changed)
}

/// 部署的退役写入面（细则见 R002 与模块文档）。
fn retire_json_file(
    path: &Path,
    strip: fn(&mut Json) -> Result<bool, String>,
    report: &mut DeployReport,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut settings = read_json(path)?;
    if !settings.is_object() {
        return Ok(());
    }
    if strip(&mut settings)? {
        if settings.as_object().is_some_and(|o| o.is_empty()) {
            fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
            report.wrote.push(format!("{} (retired)", path.display()));
        } else {
            write_json(path, &settings)?;
            report
                .wrote
                .push(format!("{} (retired-ours)", path.display()));
        }
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

fn claude_handler(oma: &Path, side: OsSide) -> Json {
    json!({
        "type": "command",
        "command": shim_command_ps_or_sh("claude", oma, side),
        "timeout": 10,
    })
}

/// 部署的D27面（细则见 R002 与模块文档）。
/// 根，跨项目共享、hst 轮换无痛）。Windows 用 **`powershell.exe -NoProfile
/// -ExecutionPolicy Bypass -File` 前缀加无引号正斜杠 ps1 路径**加参数
/// （D39，2026-09-13 宿主实弹）：settings.json 是双消费者（claude 本体 hook
/// 执行 shell 是 POSIX sh 系、grok 经 PowerShell，M047/M059），sh 系对直路
/// 径 `.cmd` 在 WSL 形不认盘符、`cmd.exe /c` 在 Git Bash 被 MSYS 参数转换
/// 吃掉 `/c`、`//c` 在 PS 进交互态——唯「按名解析的解释器加参数位 Windows
/// 路径」三态成立（powershell.exe 恒在、无前导斜杠参数不触发转换，与状态
/// 栏 pwsh -File 同构）；M047 双消费者约束仍满足：单字符串、无 `&`、无引
/// 号。边界：路径含空格时该形态在各 shell 都裂，部署侧 warn（doctor 提
/// 示）。POSIX 一律 sh 路径直引（引号在 sh 合法且必要）。
fn shim_command_ps_or_sh(agent: &str, oma: &Path, side: OsSide) -> String {
    match side {
        OsSide::Windows => format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {} {}",
            crate::pathutil::forward_slash(&oma.join("hooks").join("hst-state.ps1")),
            agent
        ),
        OsSide::Unix => format!(
            "\"{}\" {}",
            oma.join("hooks").join("hst-state.sh").display(),
            agent
        ),
    }
}

/// Which OS consumes a codex registration field: `command` on Unix,
/// `commandWindows` on Windows (S015). Injected so tests exercise both
/// sides from one host.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OsSide {
    /// 该字段承载部署的Windows数据。
    Windows,
    /// 该字段承载部署的Unix数据。
    Unix,
}

/// 部署的host_side面（细则见 R002 与模块文档）。
pub fn host_side() -> OsSide {
    if cfg!(windows) {
        OsSide::Windows
    } else {
        OsSide::Unix
    }
}

/// Build the field-ownership form of a codex handler: the deploying side's
/// field is rewritten to the shim path; the foreign side's field survives
/// from `base` byte-verbatim (absent stays absent). Keys keep a fixed order
/// so reruns converge byte-identically on both sides. D28：shim 在 hst 自管
/// 根（用户级注册只写本侧家目录文件，异侧字段保留语义沿用不动）。
fn codex_handler_value(base: &Json, oma: &Path, session_end: bool, side: OsSide) -> Json {
    // 注册指向自包含 shim（零 oma 依赖）。Windows 侧用 powershell -File 前
    // 缀加无引号正斜杠 ps1 路径（D39 sh 兼容形态，与 claude/kimi 面同款；
    // M059 直路径形态在 PS/cmd 成立但 sh 系不认盘符路径）。Unix 用 sh 路径
    // 直引。`command` 为 schema 必填（M055）：无异侧保留值时落 bare 兜底。
    // 本侧命令串单一来源 codex_side_command（sweep 共用）。
    let foreign = |key: &str| base.get(key).filter(|v| v.is_string()).cloned();
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!("command"));
    if side == OsSide::Unix {
        obj.insert("command".into(), json!(codex_side_command(oma, side)));
    } else if let Some(v) = foreign("command") {
        obj.insert("command".into(), v);
    } else {
        obj.insert("command".into(), json!("hst hook status --agent codex"));
    }
    if side == OsSide::Unix {
        if let Some(v) = foreign("commandWindows") {
            obj.insert("commandWindows".into(), v);
        }
    } else {
        obj.insert(
            "commandWindows".into(),
            json!(codex_side_command(oma, side)),
        );
    }
    obj.insert("timeout".into(), json!(if session_end { 3 } else { 10 }));
    Json::Object(obj)
}

/// codex merge with per-OS field ownership. Never stale-drops oma entries:
/// the foreign OS's absolute path inside the foreign field is live on that
/// OS（D28 用户级注册后此语义主要保护既有异侧残留与人为混写）。
fn merge_codex_hook_event(
    settings: &mut Json,
    event: &str,
    session_end: bool,
    side: OsSide,
    oma: &Path,
) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let groups = obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !groups.is_object() {
        *groups = json!({});
    }
    let groups = groups
        .as_object_mut()
        .ok_or_else(|| "hooks is not an object".to_string())?;
    let entry = groups.entry(event.to_string()).or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    let arr = entry
        .as_array_mut()
        .ok_or_else(|| "event groups is not an array".to_string())?;
    let mut changed = false;
    let mut replaced = false;
    // 全量已见集合而非单值（review 复核残余：A,B,A 序列单值判定会漏掉尾
    // A；集合语义按形态全等去重，异形各自保留）。
    let mut seen: HashSet<Json> = HashSet::new();
    for group in arr.iter_mut() {
        let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        else {
            continue;
        };
        let before = hooks.len();
        hooks.retain(|handler| {
            if !handler_is_ours(handler) {
                return true;
            }
            let next = codex_handler_value(handler, oma, session_end, side);
            seen.insert(next) // 同形重复：保首条弃余
        });
        if hooks.len() != before {
            changed = true;
        }
        for handler in hooks.iter_mut() {
            if !handler_is_ours(handler) {
                continue;
            }
            let next = codex_handler_value(handler, oma, session_end, side);
            if handler != &next {
                *handler = next;
                changed = true;
            }
            replaced = true;
        }
    }
    if !replaced {
        arr.push(json!({
            "matcher": "*",
            "hooks": [codex_handler_value(&Json::Null, oma, session_end, side)]
        }));
        changed = true;
    }
    Ok(changed)
}

fn grok_handler(oma: &Path, side: OsSide) -> Json {
    // Windows：grok 只认可整串 spawn 的单路径（M048），指向 baked 包装；
    // Unix：sh 直带参。
    let command = match side {
        OsSide::Windows => oma
            .join("hooks")
            .join("hst-state-grok.cmd")
            .display()
            .to_string(),
        OsSide::Unix => shim_command_ps_or_sh("grok", oma, side),
    };
    json!({
        "type": "command",
        "command": command,
        "timeout": 10,
    })
}

/// 部署的Claude面（细则见 R002 与模块文档）。
/// statusline 各占一键，互不干扰）。事件集含 PermissionRequest。
fn deploy_claude_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let path = user_home.join(".claude").join("settings.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    report.form = Some("user");
    let mut changed = false;
    // D39 第 2 轮：先清非管理事件里的异形态 ours 残留，再并现行（管理事件
    // 由 merge 的陈旧替换照旧，is_ours 已认全路径解释器头）。
    changed |= sweep_unmanaged_ours(&mut settings, &events);
    for event in events {
        changed |= merge_hook_event(&mut settings, event, claude_handler(oma, side))?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// 部署的Codex面（细则见 R002 与模块文档）。
/// 两者都非空触发警告，故只用 hooks.json 一层）+ `~/.codex/config.toml`
/// `[features] hooks` 与 `[hooks.state]` trusted_hash 预种。Notification
/// does not exist in Codex (S015)。
///
/// Trust is pre-seeded by replicating codex's own identity scheme (S015
/// source): key `<config.toml abs>:<event_label>:<group>:<handler>`, hash
/// over the normalized handler identity (canonical key-sorted JSON, sha256)。
/// 用户层的 key source 即用户 config.toml 路径。
fn deploy_codex_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        ("SessionStart", false),
        ("UserPromptSubmit", false),
        ("PreToolUse", false),
        ("PermissionRequest", false),
        ("PostToolUse", false),
        ("Stop", false),
        ("SessionEnd", true),
    ];
    let path = user_home.join(".codex").join("hooks.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    // D39 第 2 轮：先清非管理事件里的异形态 ours 残留（只看本侧字段），
    // 再并现行（管理事件由 merge 照旧，异侧字段字节保留语义不动）。
    let managed: Vec<&str> = events.iter().map(|(e, _)| *e).collect();
    changed |= sweep_unmanaged_ours_codex(&mut settings, &managed, side);
    for (event, session_end) in events {
        changed |= merge_codex_hook_event(&mut settings, event, session_end, side, oma)?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }

    // [features] hooks = true in user config.toml.
    let cfg = user_home.join(".codex").join("config.toml");
    let mut toml = read_toml(&cfg)?;
    let table = match &mut toml {
        toml::Value::Table(t) => t,
        _ => return Err("codex config.toml is not a table".into()),
    };
    let features = table
        .entry("features".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let feats = match features {
        toml::Value::Table(t) => t,
        _ => return Err("codex [features] is not a table".into()),
    };
    let feature_missing = feats.get("hooks").and_then(|v| v.as_bool()) != Some(true);
    if feature_missing {
        feats.insert("hooks".into(), toml::Value::Boolean(true));
    }

    // Pre-seed [hooks.state]."<key>" trusted_hash for every oma handler in
    // the final hooks.json (real indices, not assumption zero)。键源 =
    // **hooks.json 路径**（F1 根修，codex review 2026-09-11：codex 的键源是
    // 定义该 hook 的文件路径——discovery.rs 的 load_hooks_json 返回
    // `<config_folder>/hooks.json` 作 source_path；本机 config.toml 里 codex
    // 自写键即 `hooks.json:` 前缀实证。P0010 期「config.toml 路径」假设至此
    // 证伪，见 M061）。
    let final_hooks = read_json(&path)?;
    let entries = codex_trust_entries(&final_hooks, &path)?;
    let mut trust_changed = false;
    // 单一来源卫生（M056 顺带）：hook 定义只在 hooks.json；config.toml 里
    // [hooks] 下除 state 外的**ours**定义键（旧 oma 部署残留）清掉。外来定
    // 义保留（F4 收窄，codex review：用户级 config 是用户主配置，静默删外
    // 来定义是数据损失；项目级旧口径见 P0044）。
    if let Some(toml::Value::Table(hooks_tbl)) = table.get_mut("hooks") {
        let stale_defs: Vec<String> = hooks_tbl
            .iter()
            .filter(|(k, v)| k.as_str() != "state" && def_is_all_ours(v))
            .map(|(k, _)| k.clone())
            .collect();
        if !stale_defs.is_empty() {
            for k in stale_defs {
                hooks_tbl.remove(&k);
            }
            trust_changed = true;
        }
    }
    let states = table.entry("hooks".to_string()).or_insert_with(|| {
        let mut hooks_tbl = toml::map::Map::new();
        hooks_tbl.insert(
            "state".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
        toml::Value::Table(hooks_tbl)
    });
    let state_table = match states {
        toml::Value::Table(t) => t
            .entry("state".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new())),
        _ => return Err("codex [hooks] is not a table".into()),
    };
    let state_map = match state_table {
        toml::Value::Table(t) => t,
        _ => return Err("codex [hooks.state] is not a table".into()),
    };
    for (key, hash) in entries {
        let current = state_map
            .get(&key)
            .and_then(|v| v.get("trusted_hash"))
            .and_then(|v| v.as_str());
        if current != Some(hash.as_str()) {
            let mut m = toml::map::Map::new();
            m.insert("trusted_hash".into(), toml::Value::String(hash));
            state_map.insert(key, toml::Value::Table(m));
            trust_changed = true;
        }
    }
    if trust_changed || feature_missing {
        toml_write(&cfg, &toml)?;
        report.wrote.push(cfg.display().to_string());
    } else {
        report.skipped.push(cfg.display().to_string());
    }
    Ok(())
}

/// 部署的Grok面（细则见 R002 与模块文档）。
/// Claude 同构 JSON）。No PermissionRequest event exists (S015)。
fn deploy_grok_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let path = user_home
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    // D39 第 2 轮：先清非管理事件里的异形态 ours 残留，再并现行。
    changed |= sweep_unmanaged_ours(&mut settings, &events);
    for event in events {
        changed |= merge_hook_event(&mut settings, event, grok_handler(oma, side))?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// kimi `[[hooks]]` 表项（S015 schema `.strict()` 只收 event/matcher/
/// command/timeout 四字段；matcher 不填匹配全部）。
fn kimi_hook_entry(event: &str, command: &str) -> toml::Value {
    let mut m = toml::map::Map::new();
    m.insert("event".into(), toml::Value::String(event.into()));
    m.insert("command".into(), toml::Value::String(command.into()));
    m.insert("timeout".into(), toml::Value::Integer(10));
    toml::Value::Table(m)
}

/// kimi hook 命令串：Windows 用 powershell -File 前缀加无引号正斜杠 ps1
/// 路径（D39 sh 兼容形态，与 claude/codex 面同款）；POSIX 是正斜杠裸命令
/// 行（M058 实证：kimi 对整串命令朴素消费，引号形态静默不执行；TOML
/// basic string 反斜杠是转义符，一律正斜杠）。
fn kimi_hook_command(oma: &Path, side: OsSide) -> String {
    match side {
        OsSide::Windows => format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {} kimi",
            crate::pathutil::forward_slash(&oma.join("hooks").join("hst-state.ps1"))
        ),
        OsSide::Unix => format!("{} kimi", oma.join("hooks").join("hst-state.sh").display()),
    }
}

/// 部署的Kimi面（细则见 R002 与模块文档）。
/// 仅用户级，项目级 hook 注册不存在）。陈旧 ours（bare oma、旧路径、异形）
/// 弃后按事件补现行单条，同形去重；外来条目保留。
fn apply_kimi_hooks(
    toml: &mut toml::Value,
    command: &str,
    events: &[&str],
) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi config.toml is not a table".into()),
    };
    let arr = table
        .entry("hooks".to_string())
        .or_insert_with(|| toml::Value::Array(Vec::new()));
    let items = match arr {
        toml::Value::Array(a) => a,
        _ => return Err("kimi [hooks] is not an array of tables".into()),
    };
    let mut changed = false;
    let before = items.len();
    // 现形保留（事件对应的现行条目），陈旧/异形弃。
    items.retain(|h| {
        let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
            return true;
        };
        if !is_ours(c) {
            return true;
        }
        let ev = h.get("event").and_then(|e| e.as_str()).unwrap_or("");
        *h == kimi_hook_entry(ev, command)
    });
    if items.len() != before {
        changed = true;
    }
    // 同形重复去重（保首条；toml::Value 不可哈希，键取 (event, command)）。
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let before = items.len();
    items.retain(|h| {
        let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
            return true;
        };
        if !is_ours(c) {
            return true;
        }
        let ev = h
            .get("event")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .to_string();
        seen.insert((ev, c.to_string()))
    });
    if items.len() != before {
        changed = true;
    }
    for event in events {
        let want = kimi_hook_entry(event, command);
        if !items.contains(&want) {
            items.push(want);
            changed = true;
        }
    }
    Ok(changed)
}

/// Kimi 用户级部署（hook 注册；kimi 项目侧本就无注册面，说明层与技能
/// 目录清扫归 init 全套面）。
fn deploy_kimi_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let config = user_home.join(".kimi-code").join("config.toml");
    let mut toml = read_toml(&config)?;
    if apply_kimi_hooks(&mut toml, &kimi_hook_command(oma, side), &events)? {
        toml_write(&config, &toml)?;
        report.wrote.push(config.display().to_string());
    } else {
        report.skipped.push(config.display().to_string());
    }
    Ok(())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 用户级部署总入口（可注入：测试传临时 user_home 与 oma 根；生产传真实
/// 家目录与 `install::hst_home()`）。shim 先落（三平台全侧），四家注册
/// 幂等合并。
pub fn deploy_user_hooks_with(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let (shim_wrote, shim_warns) = crate::shim::deploy_shims(oma)?;
    for p in shim_wrote {
        report.wrote.push(p.display().to_string());
    }
    report.warns.extend(shim_warns);
    // M059 边界：Windows 注册是无引号正斜杠形态，oma 根路径含空格三 shell
    // 全裂（用户级注册路径固定在家目录，此坑随家目录出现）。
    if side == OsSide::Windows && oma.to_string_lossy().contains(' ') {
        report.warns.push(
            "oma home path contains spaces: Windows hook registrations are \
             unquoted forward-slash forms that break in every shell; relocate \
             ~/.hst (or the user profile) to a space-free path"
                .to_string(),
        );
    }
    deploy_claude_user(user_home, oma, side, report)?;
    deploy_codex_user(user_home, oma, side, report)?;
    deploy_grok_user(user_home, oma, side, report)?;
    deploy_kimi_user(user_home, oma, side, report)?;
    Ok(())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 部署的生产入口面（细则见 R002 与模块文档）。
pub fn deploy_user_hooks(report: &mut DeployReport) -> Result<(), String> {
    let user_home = crate::pathutil::user_home()?;
    let oma = crate::install::hst_home()?;
    deploy_user_hooks_with(&user_home, &oma, host_side(), report)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 项目面退役（D28）：摘除项目级 ours hook 注册（claude/codex/grok）、
/// 删除 oma 部署的项目 shim（`.oma/hooks/` 三件，带生成标记才删）。
/// kimi 项目侧无注册面；项目 `.oma/state/` 旧状态文件保留（状态栏旧协议
/// 兼容读，不属注册面）。
pub fn retire_project_hooks_with(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let root = crate::pathutil::abs_display(root);
    retire_json_file(
        &root.join(".claude").join("settings.json"),
        strip_ours_handlers,
        report,
    )?;
    retire_json_file(
        &root.join(".codex").join("hooks.json"),
        strip_ours_codex_handlers,
        report,
    )?;
    retire_json_file(
        &root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json"),
        strip_ours_handlers,
        report,
    )?;
    // 项目 shim 退役：oma 生成的三件删（内容带标记才动，用户自置同名文件
    // 不碰）；hooks 目录空了连目录摘。旧名 .ohmyagents 同查（D14 前部署）。
    // F3 守卫（codex review）：项目根即 hst 根（`hst init --project $HOME`
    // 或 cwd 在家）时，base 就是用户级 shim 落点，跳过防自删。
    let oma_root = crate::install::hst_home().unwrap_or_default();
    // 两侧都过 abs_display（CI runner 的 temp 路径是 8.3 短名，canonicalize
    // 后与 env 原串不等会让守卫失效，v0.5.4 CI 红根因）。
    let oma_root = crate::pathutil::abs_display(&oma_root);
    for base in [root.join(".oma"), root.join(".ohmyagents")] {
        if !oma_root.as_os_str().is_empty() && crate::pathutil::abs_display(&base) == oma_root {
            continue;
        }
        let hooks_dir = base.join("hooks");
        for name in [
            "hst-state.cmd",
            "hst-state-grok.cmd",
            "hst-state.sh",
            // D28 前项目级旧名（v0.6.0 sweep 把环误换成新名成死码，D30 修：
            // 项目级只会有旧名，用户级新名落的是别的根不会被本环碰到）。
            "oma-state.cmd",
            "oma-state-grok.cmd",
            "oma-state.sh",
        ] {
            let p = hooks_dir.join(name);
            if let Ok(text) = fs::read_to_string(&p) {
                if text.contains("generated by oma init") || text.contains("generated by hst init")
                {
                    fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                    report.wrote.push(format!("{} (retired)", p.display()));
                }
            }
        }
        if hooks_dir.is_dir()
            && fs::read_dir(&hooks_dir)
                .map(|d| d.flatten().next().is_none())
                .unwrap_or(false)
        {
            let _ = fs::remove_dir(&hooks_dir);
        }
    }
    Ok(())
}

/// Strip the Windows canonicalization prefix codex never sees (`\\?\`),
/// because the trust key must match the path form codex derives from its own
/// project discovery.
fn plain_absolute(path: &Path) -> String {
    let s = path.display().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn event_label(event: &str) -> String {
    event
        .chars()
        .flat_map(|c| {
            if c.is_ascii_uppercase() {
                vec!['_', c.to_ascii_lowercase()]
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .trim_start_matches('_')
        .to_string()
}

/// codex matcher semantics (S015 source): these events ignore matchers, so
/// the hashed identity drops the key entirely (TOML drops nulls).
fn hashed_matcher(event: &str, matcher: Option<&str>) -> Option<String> {
    match event {
        "UserPromptSubmit" | "Stop" | "Interrupt" => None,
        _ => matcher.filter(|m| !m.is_empty()).map(String::from),
    }
}

fn canonical_json(value: &Json) -> Json {
    match value {
        Json::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonical_json(&map[key]));
            }
            Json::Object(sorted)
        }
        Json::Array(items) => Json::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

/// Command string Codex on this OS actually runs: Windows prefers
/// `commandWindows`, Unix prefers `command`, each falling back to the
/// other so a single-field handler still hashes.
fn effective_codex_command(handler: &Json) -> Result<&str, String> {
    let unix = handler.get("command").and_then(|c| c.as_str());
    let windows = handler.get("commandWindows").and_then(|c| c.as_str());
    let picked = if cfg!(windows) {
        windows.or(unix)
    } else {
        unix.or(windows)
    };
    picked.ok_or_else(|| "handler missing command and commandWindows".into())
}

/// Replicate codex `hook_hash` (S015 source): identity over the normalized
/// handler (commandWindows dropped, timeout clamped per event), serialized
/// as canonical key-sorted JSON, sha256, `sha256:<hex>`.
fn codex_hook_hash(event: &str, matcher: Option<&str>, handler: &Json) -> Result<String, String> {
    let effective = effective_codex_command(handler)?;
    let timeout = handler.get("timeout").and_then(|t| t.as_u64());
    let timeout = match event {
        "SessionEnd" | "Interrupt" => timeout.unwrap_or(1).clamp(1, 3),
        _ => timeout.unwrap_or(600).max(1),
    };
    let r#async = handler
        .get("async")
        .and_then(|a| a.as_bool())
        .unwrap_or(false);
    let mut entry = serde_json::Map::new();
    entry.insert("type".into(), json!("command"));
    entry.insert("command".into(), json!(effective));
    entry.insert("timeout".into(), json!(timeout));
    entry.insert("async".into(), json!(r#async));
    if let Some(sm) = handler.get("statusMessage").and_then(|s| s.as_str()) {
        entry.insert("statusMessage".into(), json!(sm));
    }
    if let Some(limit) = handler
        .get("additionalContextLimit")
        .and_then(|s| s.as_u64())
    {
        entry.insert("additionalContextLimit".into(), json!(limit));
    }

    let mut identity = serde_json::Map::new();
    identity.insert("event_name".into(), json!(event_label(event)));
    if let Some(m) = hashed_matcher(event, matcher) {
        identity.insert("matcher".into(), json!(m));
    }
    identity.insert("hooks".into(), Json::Array(vec![Json::Object(entry)]));

    let canonical = canonical_json(&Json::Object(identity));
    let bytes = serde_json::to_vec(&canonical).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{digest:x}"))
}

/// config.toml `[hooks]` 下一个定义键的值（事件 group 数组，group =
/// `{matcher?, hooks:[handler]}`）是否全部由 ours 处理器构成（F4 判据：
/// 只清 ours 残留定义，外来与混合保留保守不动）。
fn def_is_all_ours(def: &toml::Value) -> bool {
    /// 单个 handler 形（claude 同构 command 字段或 codex 双字段）判 ours。
    fn handler_ours(h: &toml::Value) -> bool {
        ["command", "commandWindows"]
            .iter()
            .any(|k| h.get(*k).and_then(|c| c.as_str()).is_some_and(is_ours))
    }
    let Some(groups) = def.as_array() else {
        return false;
    };
    let mut any = false;
    for g in groups {
        let Some(hs) = g.get("hooks").and_then(|h| h.as_array()) else {
            return false;
        };
        if hs.is_empty() || hs.iter().any(|h| !handler_ours(h)) {
            return false;
        }
        any = true;
    }
    any
}

/// Walk the final hooks.json and produce (key, trusted_hash) pairs for every
/// oma-owned handler at its real group/handler indices。键源 = 定义该 hook
/// 的 hooks.json 路径（F1 根修；doctor 的信任判据同源复用）。
pub(crate) fn codex_trust_entries(
    hooks_json: &Json,
    hooks_json_path: &Path,
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let Some(events) = hooks_json.get("hooks").and_then(|h| h.as_object()) else {
        return Ok(out);
    };
    let key_source = plain_absolute(hooks_json_path);
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (gi, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(|m| m.as_str());
            let Some(handlers) = group.get("hooks").and_then(|h| h.as_array()) else {
                continue;
            };
            for (hi, handler) in handlers.iter().enumerate() {
                if !handler_is_ours(handler) {
                    continue;
                }
                let hash = codex_hook_hash(event, matcher, handler)?;
                let key = format!("{}:{}:{}:{}", key_source, event_label(event), gi, hi);
                out.push((key, hash));
            }
        }
    }
    Ok(out)
}

/// 部署历史生成标记面（ADR-0005 起 skill 面退役，标记仅作 ours 退役判据）。
const SKILL_MARKER: &str = "<!-- generated by hst init; rerun hst init to sync the command map -->";

/// v0.5.x 与 v0.6.0 历史标记的公共前缀（v0.5.x 尾巴 rerun oma init、v0.6.0
/// 尾巴 rerun hst init）：前缀双收才能把旧项目生成物认作 ours（整串匹配
/// 会漏旧尾巴，D30 修）。
const LEGACY_SKILL_MARKER_PREFIX: &str = "<!-- generated by oma init; rerun";

/// 旧版静态 skill 全文：识别后退役（ADR-0005 前的升级语义已删）。
const LEGACY_SKILL_MD: &str = "---\nname: ohmyagents\ndescription: Oh My Agents 项目编排说明与状态通道\n---\n\n# Oh My Agents\n\n本项目会话由 oma 编排。agent 状态在 `.ohmyagents/state/`；委派与诊断经 oma CLI。\n";

/// `hst skill --write` 生成物的签名行（oma 与 hst 两代都含）。
const USER_SKILL_SIGNATURE: &str = "活命令树自适应生成";

const AGENTS_MD: &str = "# AGENTS\n\n本项目会话由 HST（Hooks, Statusline, Trace，原 Oh My Agents）治理：agent 状态写用户级 `~/.hst/state/`，诊断与部署经 hst CLI。\n";

/// ours 判据（项目面与用户级共用，codex 评审 F1 收口）：init 生成标记
/// 家族（hst 与 oma 两代）、旧静态版全文、或 `hst skill --write` 生成
/// 签名，三者其一即 ours。
fn skill_md_is_ours(md: &str) -> bool {
    md.contains(SKILL_MARKER)
        || md.contains(LEGACY_SKILL_MARKER_PREFIX)
        || md == LEGACY_SKILL_MD
        || md.contains(USER_SKILL_SIGNATURE)
}

/// ADR-0005（D54）：skill 面退役，项目级 fan-out 不再写，在位 ours 件由
/// init 清扫。四目录（.agents/.claude/.grok/.kimi-code）乘两名（hst 与旧牌
/// ohmyagents）逐一退役。
fn retire_skills(root: &Path, report: &mut DeployReport) {
    for target in [".agents", ".claude", ".grok", ".kimi-code"] {
        let base = root.join(target).join("skills");
        for name in ["hst", "ohmyagents"] {
            retire_ours_skill_dir(&base.join(name), report);
        }
    }
}

/// SKILL.md 属 ours 才退役；用户手改或他源内容不动。外科式（codex F4）：
/// 先删 SKILL.md，目录仅在空时收（伴生资源不连带删）。
fn retire_ours_skill_dir(dir: &Path, report: &mut DeployReport) {
    let Ok(md) = std::fs::read_to_string(dir.join("SKILL.md")) else {
        return;
    };
    if !skill_md_is_ours(&md) {
        return;
    }
    if std::fs::remove_file(dir.join("SKILL.md")).is_ok() {
        let _ = std::fs::remove_dir(dir);
        report.wrote.push(format!("{} (retired)", dir.display()));
    }
}

/// ADR-0005（D54）：用户级技能目录退役（`hst skill` 面已删）。四家用户级
/// skills 根（.claude 加 .agents 加 .grok 加 .kimi-code）乘 hst 与旧牌
/// ohmyagents 两名，历史两代产物都在清扫面（codex 评审 F1：home-root
/// init fan-out 落的 marker 件加 `skill --write` 落的签名件）；判据同
/// 项目面共用；用户手改或他源不动；外科式先删 SKILL.md，目录仅在空时
/// 收。grok 自带 `~/.grok/bundled/skills/` 是另一父目录，不在扫描面。
fn retire_user_skills(user_home: &Path, report: &mut DeployReport) {
    for target in [".claude", ".agents", ".grok", ".kimi-code"] {
        let skills = user_home.join(target).join("skills");
        for name in ["hst", "ohmyagents"] {
            retire_ours_skill_dir(&skills.join(name), report);
        }
    }
}

/// AGENTS.md only when absent (never overwrite user content); CLAUDE.md is a
/// one-line @AGENTS.md include.
fn deploy_instructions(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let agents = root.join("AGENTS.md");
    if !agents.exists() {
        write_text(&agents, AGENTS_MD)?;
        report.wrote.push(agents.display().to_string());
    } else {
        report.skipped.push(agents.display().to_string());
    }
    let claude = root.join("CLAUDE.md");
    if !claude.exists() {
        write_text(&claude, "@AGENTS.md\n")?;
        report.wrote.push(claude.display().to_string());
    } else {
        report.skipped.push(claude.display().to_string());
    }
    Ok(())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Deploy the full init surface (D28)：用户级 hook 注册加 shim（真实家目录
/// 与 oma 根）、本项目旧注册与 shim 退役、说明层部署与 ours 技能目录
/// 清扫（ADR-0005）。
pub fn deploy_all(root: &Path) -> Result<DeployReport, String> {
    let user_home = crate::pathutil::user_home()?;
    let oma = crate::install::hst_home()?;
    deploy_all_with(root, &user_home, &oma, host_side())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Test seam：user_home 与 oma 根注入（不碰真实家目录），side 注入双测。
pub fn deploy_all_with(
    root: &Path,
    user_home: &Path,
    oma: &Path,
    side: OsSide,
) -> Result<DeployReport, String> {
    // abs_display（非裸 canonicalize）：剥掉 Windows `\\?\` 前缀，路径要进
    // 注册命令与 codex 信任键，带前缀 cmd 侧不可执行。
    let root = crate::pathutil::abs_display(root);
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut report = DeployReport::default();
    deploy_user_hooks_with(user_home, oma, side, &mut report)?;
    // D52 铁证修复：root == 用户家目录时，项目级退役趟（D28 面向「项目内
    // 旧 ours 注册与项目 yolo 键」）会打到用户级文件本体——把本批用户级
    // 刚落的 yolo 键当项目键整批摘掉（宿主裸 init 于 C:\Users\ray 实弹：
    // settings.json 与 codex 与 kimi 三文件同批 (retired-yolo) 洗键）。
    // 家目录不是项目：退役趟整组跳过并打点。
    if crate::pathutil::same_location(&root, user_home) {
        report.warns.push(
            "project-level retirement skipped: project root is the user home \
             (home is not a project; user-level keys stay)"
                .to_string(),
        );
    } else {
        retire_project_hooks_with(&root, &mut report)?;
        for c in crate::yolo::retire_project_yolo(&root)? {
            report.wrote.push(c);
        }
    }
    retire_skills(&root, &mut report);
    retire_user_skills(user_home, &mut report);
    deploy_instructions(&root, &mut report)?;
    // D53：状态栏面并入全套部署（与 `hst statusline` 同路径）：脚本重生
    //（自备脚本 marker 保护跳过）加四家 statusLine 幂等合并。fleet 实弹：
    // init 从不触碰状态栏导致在位脚本停旧版（D51 clock 段永不到位）。
    // 注意 statusline 函数族的 home 形参是 **hst 根**（oma，脚本落
    // `<根>/statusline/`），不是用户家目录；merge_* 内部自取 user_home。
    // 内容判等幂等：脚本与配置无变化时零写入零输出，init 重跑安静。
    crate::statusline::deploy_script(oma)?;
    crate::statusline::merge_claude(oma, user_home)?;
    crate::statusline::merge_codex(oma, user_home)?;
    crate::statusline::merge_kimi(oma, user_home)?;
    crate::statusline::merge_grok(oma, user_home)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_refreshes_stale_statusline_and_respects_custom_marker() {
        // D53（fleet 实弹）：init 全套并入状态栏面——在位旧版脚本被刷新
        //（D51 clock 段到位）、四家 statusLine 配置幂等合并；自备脚本
        // marker 保护不动；重跑内容判等零写入。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let base = std::env::temp_dir().join(format!("hst-d53-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let user = base.join("user");
        let root = base.join("proj");
        let oma = base.join("hst");
        fs::create_dir_all(user.join(".claude")).unwrap();
        fs::create_dir_all(&root).unwrap();
        // 在位旧版脚本（无 marker）。
        fs::create_dir_all(oma.join("statusline")).unwrap();
        let script = oma.join("statusline").join("hst-statusline.ps1");
        fs::write(&script, "# old 09-13 era script, no clock segment\n").unwrap();
        std::env::set_var("HST_USER_HOME", &user);
        std::env::set_var("HST_ROOT", &oma);
        deploy_all_with(&root, &user, &oma, host_side()).unwrap();
        let body = fs::read_to_string(&script).unwrap();
        assert!(
            body.contains("clock"),
            "stale script must be refreshed with the clock segment"
        );
        let settings: Json = serde_json::from_str(
            &fs::read_to_string(user.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert!(settings.get("statusLine").is_some(), "bar config merged");
        // 幂等：再跑脚本与四家配置 mtime 全不动（codex F2：claude 与 codex
        // 写入也须内容判等）。
        let cfgs = [
            user.join(".claude").join("settings.json"),
            user.join(".codex").join("config.toml"),
            user.join(".kimi-code").join("tui.toml"),
            user.join(".grok").join("config.toml"),
        ];
        let before: Vec<_> = std::iter::once(fs::metadata(&script).unwrap().modified().unwrap())
            .chain(
                cfgs.iter()
                    .map(|c| fs::metadata(c).unwrap().modified().unwrap()),
            )
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(20));
        deploy_all_with(&root, &user, &oma, host_side()).unwrap();
        let after: Vec<_> = std::iter::once(fs::metadata(&script).unwrap().modified().unwrap())
            .chain(
                cfgs.iter()
                    .map(|c| fs::metadata(c).unwrap().modified().unwrap()),
            )
            .collect();
        assert_eq!(
            before, after,
            "content-equal rerun must not touch any mtime"
        );
        // 自备脚本 marker：init 不覆盖用户定制。
        fs::write(&script, "# user custom bar\n").unwrap();
        fs::write(
            oma.join("statusline").join("hst-statusline.ps1.custom"),
            "/somewhere/custom.ps1",
        )
        .unwrap();
        deploy_all_with(&root, &user, &oma, host_side()).unwrap();
        assert_eq!(
            fs::read_to_string(&script).unwrap(),
            "# user custom bar\n",
            "custom script marker must protect user content"
        );
        std::env::remove_var("HST_USER_HOME");
        std::env::remove_var("HST_ROOT");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn home_root_skips_project_retirement_and_keeps_user_keys() {
        // D52 铁证回归：宿主裸 init 于家目录（cwd = C:\Users\ray），项目级
        // 退役趟把用户级刚落的 yolo 键当项目键摘掉（settings 与 codex 与
        // kimi 三文件同批 (retired-yolo) 洗键）。家目录不是项目：退役趟
        // 跳过、键全保、打 warn。
        let home = std::env::temp_dir().join(format!("hst-home-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::write(
            home.join(".claude").join("settings.json"),
            r#"{"permissions": {"defaultMode": "bypassPermissions", "blockReadsOutsideWorkingDirectories": false, "skipDangerousModePermissionPrompt": true}}"#,
        )
        .unwrap();
        let report = deploy_all_with(&home, &home, &home.join(".hst"), host_side()).unwrap();
        let v: Json = serde_json::from_str(
            &fs::read_to_string(home.join(".claude").join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            v["permissions"]["defaultMode"], "bypassPermissions",
            "user keys must survive when project root is the home"
        );
        assert_eq!(
            v["permissions"]["blockReadsOutsideWorkingDirectories"], false,
            "D52 readblock key must survive"
        );
        assert!(
            report
                .warns
                .iter()
                .any(|w| w.contains("project-level retirement skipped")),
            "skip must be loud: {:?}",
            report.warns
        );
        assert!(
            !report.wrote.iter().any(|w| w.contains("retired-yolo")),
            "no project retirement against user files: {:?}",
            report.wrote
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn is_ours_handles_windows_call_operator_form() {
        // codex commandWindows 是 `& "exe" hook`：调用操作符在首 token 前，
        // 不剥掉会把 Windows 侧字段误判成外来者（跨环境共享目录必踩）。
        assert!(is_ours(r#"& "D:\cargo\bin\oma.exe" hook --agent codex"#));
        assert!(is_ours(r#""C:\somewhere\oma.exe" hook"#));
        assert!(is_ours("oma hook --agent claude"));
        assert!(is_ours(r#""/home/ray/.cargo/bin/oma" hook --agent codex"#));
        assert!(!is_ours(r#"& "D:\tools\echo.exe" args"#));
        assert!(!is_ours("echo hi"));
    }

    #[test]
    fn is_ours_recognizes_interpreter_wrapped_form_and_foreign_wraps() {
        // D39：Windows 注册是解释器前缀形，跳过前缀（powershell 系无 /c、
        // cmd 系带可选 /c）后取程序 token；外来解释器包裹（非 hst/oma 家
        // 族）不得误判 ours。
        assert!(is_ours(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:/Users/ray/.hst/hooks/hst-state.ps1 claude"
        ));
        assert!(is_ours("pwsh -File C:/x/.oma/hooks/hst-state.ps1 codex"));
        assert!(is_ours(
            "cmd.exe /c C:/Users/ray/.hst/hooks/hst-state.cmd claude"
        ));
        assert!(is_ours("cmd /c C:/x/.oma/hooks/hst-state.cmd codex"));
        assert!(is_ours(
            "C:\\Windows\\System32\\cmd.exe /c C:/Users/ray/.hst/hooks/hst-state.cmd claude"
        ));
        assert!(is_ours(
            "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe -File C:/x/.hst/hooks/hst-state.ps1 kimi"
        ));
        assert!(!is_ours("cmd.exe /c C:/tools/foreign.exe run"));
        assert!(!is_ours("powershell -File C:/tools/foreign.ps1 run"));
        assert!(!is_ours("cmd.exe /c echo hi"));
        // F6：外来前缀族（hst- 开头但非 shim）在包裹分支不得误判。
        assert!(!is_ours("cmd.exe /c C:\\tools\\hst-logger.exe run"));
        assert!(!is_ours("powershell.exe -File C:/tools/oma-tool.ps1 x"));
    }

    #[test]
    fn sweep_clears_stale_ours_in_unmanaged_events_and_keeps_foreign() {
        // D39 第 2 轮（宿主终验回执）：手包 cmd.exe /c 行与更早裸 cmd 行在
        // 非管理事件里残留时，init 全文件清扫弃之（管理事件补回现行单条），
        // 外来条目永不碰。
        let user = fresh_dir("sweep");
        let oma = fresh_dir("sweep-oma");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        let cmd_path = format!(
            "{}/hooks/hst-state.cmd",
            crate::pathutil::forward_slash(&oma)
        );
        let seed = json!({
            "hooks": {
                "SessionStart": [{"matcher": "*", "hooks": [
                    {"type": "command", "command": format!("cmd.exe /c {cmd_path} claude")},
                    {"type": "command", "command": "C:\\tools\\keep.sh"},
                ]}],
                "PreCompact": [{"matcher": "*", "hooks": [
                    {"type": "command", "command": format!("{cmd_path} claude")},
                    {"type": "command", "command": "C:\\tools\\fmt.js"},
                ]}],
            }
        })
        .to_string();
        write_text(&claude, &seed).unwrap();

        let mut report = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut report).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        // 管理事件：现行单条加外来保留。
        let ss = ours_in_event(&claude, "SessionStart");
        assert_eq!(
            ss.len(),
            1,
            "managed event collapses to one current: {ss:?}"
        );
        assert!(ss[0].ends_with("/hooks/hst-state.ps1 claude"), "{}", ss[0]);
        assert!(v["hooks"]["SessionStart"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\keep.sh")));
        // 非管理事件：ours 行清光、外来保留。
        let pre = v["hooks"].get("PreCompact").cloned();
        let foreign_kept = pre.as_ref().and_then(|p| p.as_array()).is_some_and(|a| {
            a.iter()
                .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\fmt.js"))
        });
        let ours_gone = pre
            .as_ref()
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .all(|g| g["hooks"][0]["command"].as_str() != Some(&cmd_path))
            })
            .unwrap_or(true);
        assert!(foreign_kept, "foreign entry in unmanaged event survives");
        assert!(ours_gone, "stale ours line in unmanaged event swept");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn codex_sweep_clears_stale_ours_in_unmanaged_events() {
        // codex 面：非管理事件里旧形态 commandWindows 行外科摘键（codex
        // review F5：异侧携带外来活串时只摘本侧键留对象，异侧逐字节保留
        // 契约）；两侧皆 ours 整弃；纯外来 handler 保留。
        let user = fresh_dir("sweepcx");
        let oma = fresh_dir("sweepcx-oma");
        let path = user.join(".codex").join("hooks.json");
        ensure_parent(&path).unwrap();
        let cmd = format!(
            "{}/hooks/hst-state.cmd",
            crate::pathutil::forward_slash(&oma)
        );
        let sh = format!(
            "\"{}/hooks/hst-state.sh\" codex",
            crate::pathutil::forward_slash(&oma)
        );
        let seed = json!({
            "hooks": {
                "PreCompact": [{"matcher": "*", "hooks": [
                    {"type": "command", "command": "/foreign/tool.sh",
                     "commandWindows": format!("cmd.exe /c {cmd} codex")},
                    {"type": "command", "command": "C:\\tools\\keep.js"},
                    {"type": "command", "command": sh.clone(),
                     "commandWindows": format!("cmd.exe /c {cmd} codex")},
                ]}],
            }
        })
        .to_string();
        write_text(&path, &seed).unwrap();

        let mut report = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut report).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let handlers: Vec<Json> = v["hooks"]["PreCompact"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .flat_map(|g| g["hooks"].as_array().cloned().unwrap_or_default())
            .collect();
        // 纯外来 handler 原样保留。
        assert!(
            handlers
                .iter()
                .any(|h| h.get("command").and_then(|c| c.as_str()) == Some("C:\\tools\\keep.js")),
            "pure-foreign handler survives"
        );
        // F5：本侧 ours 加异侧外来活串 = 只摘 commandWindows，对象存活且
        // 异侧字段逐字节保留。
        let stripped = handlers
            .iter()
            .find(|h| h.get("command").and_then(|c| c.as_str()) == Some("/foreign/tool.sh"));
        let Some(h) = stripped else {
            panic!("surgically stripped handler must survive: {handlers:?}");
        };
        assert!(
            h.get("commandWindows").is_none(),
            "ours side key removed: {h}"
        );
        // 两侧皆 ours 的 handler 整弃。
        assert!(
            !handlers.iter().any(|h| h
                .get("command")
                .and_then(|c| c.as_str())
                .is_some_and(|c| c.contains("hst-state.sh"))),
            "both-sides-ours handler fully swept"
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn windows_direct_path_form_upgrades_to_ps1_wrap() {
        // D39 存量救济回归：M059 直路径形态（WSL 形 sh 不认盘符，宿主实弹
        // not found）重部署后被幂等覆盖为 powershell -File 包裹单条；外来
        // 条目保留。
        let user = fresh_dir("wrap");
        let oma = fresh_dir("wrap-oma");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        let legacy = format!(
            r#"{{"hooks": {{"SessionStart": [{{"matcher": "*", "hooks": [
                {{"type": "command", "command": "{}/hooks/hst-state.cmd claude"}},
                {{"type": "command", "command": "C:\\tools\\keep.sh"}}]}}]}}}}"#,
            crate::pathutil::forward_slash(&oma)
        );
        write_text(&claude, &legacy).unwrap();

        let mut report = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut report).unwrap();
        let ours = ours_in_event(&claude, "SessionStart");
        assert_eq!(
            ours.len(),
            1,
            "legacy direct form collapses to one current entry: {ours:?}"
        );
        assert!(
            ours[0].starts_with("powershell.exe -NoProfile -ExecutionPolicy Bypass -File "),
            "wrapped form: {}",
            ours[0]
        );
        assert!(
            ours[0].ends_with("/hooks/hst-state.ps1 claude"),
            "{}",
            ours[0]
        );
        // 再跑一次幂等：包裹形态被 is_ours 认领且不陈旧，不追加重复。
        let mut second = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut second).unwrap();
        assert_eq!(
            ours_in_event(&claude, "SessionStart").len(),
            1,
            "wrapped form is not stale on rerun"
        );
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        assert!(
            v["hooks"]["SessionStart"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\keep.sh")),
            "foreign entry survives the upgrade"
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-deploy-test-{tag}-{}-{}-{}",
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

    /// 收集 JSON 形注册的全部 command 串（跨事件全量）。
    fn collect_commands(p: &Path) -> Vec<String> {
        let v: Json = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
        v["hooks"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(|g| g.as_array())
            .flatten()
            .filter_map(|grp| grp.get("hooks").and_then(|h| h.as_array()))
            .flatten()
            .filter_map(|h| h.get("command").and_then(|c| c.as_str()).map(String::from))
            .collect()
    }

    /// 单事件内的 ours command 数（收敛判据按事件论：八事件各一条）。
    fn ours_in_event(p: &Path, event: &str) -> Vec<String> {
        let v: Json = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
        v["hooks"][event]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|grp| grp.get("hooks").and_then(|h| h.as_array()))
            .flatten()
            .filter_map(|h| h.get("command").and_then(|c| c.as_str()))
            .filter(|c| is_ours(c))
            .map(String::from)
            .collect()
    }

    #[test]
    fn user_deploys_merge_and_are_idempotent() {
        let user = fresh_dir("user");
        let oma = fresh_dir("oma");
        // Foreign hook must survive every deploy.
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"statusLine": {"type": "command", "command": "x"},
                "hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "C:\\tools\\fmt.sh"}]}]}}"#,
        )
        .unwrap();
        let kimi = user.join(".kimi-code").join("config.toml");
        ensure_parent(&kimi).unwrap();
        write_text(
            &kimi,
            "theme = \"dark\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"my-tool\"\ntimeout = 5\n",
        )
        .unwrap();

        let mut first = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut first).unwrap();
        assert_eq!(first.form.as_deref(), Some("user"));
        assert!(
            first.wrote.iter().any(|p| p.contains(".claude")),
            "claude user settings written: {:?}",
            first.wrote
        );
        assert!(oma.join("hooks").join("hst-state.cmd").exists());
        assert!(oma.join("hooks").join("hst-state.sh").exists());

        // statusLine 键与外来 hook 保留，ours 注册指向用户级 shim。
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        assert!(v.get("statusLine").is_some(), "statusLine key survives");
        let stop = v["hooks"]["Stop"].as_array().unwrap();
        assert!(stop
            .iter()
            .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\fmt.sh")));
        let ours = stop
            .iter()
            .find(|g| {
                g["hooks"][0]["command"]
                    .as_str()
                    .unwrap()
                    .contains("hst-state")
            })
            .unwrap();
        let claude_cmd = ours["hooks"][0]["command"].as_str().unwrap();
        if cfg!(windows) {
            assert!(!claude_cmd.contains('"'), "{claude_cmd}");
            assert!(!claude_cmd.starts_with('&'), "{claude_cmd}");
            // D39：powershell -File 前缀（claude 的 hook 执行 shell 是
            // POSIX sh 系，直路径 .cmd 在 WSL 形不认盘符）；无引号正斜杠
            // ps1 路径与 agent 参数随尾。
            assert!(
                claude_cmd.starts_with("powershell.exe -NoProfile -ExecutionPolicy Bypass -File "),
                "{claude_cmd}"
            );
            assert!(
                claude_cmd.ends_with("/hooks/hst-state.ps1 claude"),
                "{claude_cmd}"
            );
        } else {
            assert!(
                claude_cmd.ends_with("hst-state.sh\" claude"),
                "{claude_cmd}"
            );
        }
        assert!(v["hooks"]["PermissionRequest"].is_array());

        // codex 用户层：hooks.json + config.toml features/trust。
        let codex: Json = serde_json::from_str(
            &fs::read_to_string(user.join(".codex").join("hooks.json")).unwrap(),
        )
        .unwrap();
        let handler = &codex["hooks"]["SessionEnd"][0]["hooks"][0];
        if cfg!(windows) {
            let cw = handler["commandWindows"].as_str().unwrap();
            assert!(!cw.contains('"') && !cw.starts_with('&'), "{cw}");
            assert!(cw.ends_with("/hooks/hst-state.ps1 codex"), "{cw}");
            assert_eq!(
                handler["command"].as_str(),
                Some("hst hook status --agent codex"),
                "schema-required fallback must be present on Windows"
            );
        } else {
            assert!(handler["command"]
                .as_str()
                .unwrap()
                .ends_with("hst-state.sh\" codex"));
            assert!(
                handler.get("commandWindows").is_none(),
                "Unix fresh deploy must not invent the foreign-OS field"
            );
        }
        assert_eq!(handler["timeout"], 3);
        let codex_toml = fs::read_to_string(user.join(".codex").join("config.toml")).unwrap();
        assert!(codex_toml.contains("hooks = true"));
        assert!(
            codex_toml.contains("trusted_hash"),
            "user-layer trust seeded: {codex_toml}"
        );
        // F1：信任键源 = hooks.json 路径（codex 以定义文件为键源；本机
        // config.toml 里 codex 自写键即此前缀，一手实证 2026-09-11）。
        assert!(
            codex_toml.contains("hooks.json:"),
            "trust keys must carry the hooks.json path prefix: {codex_toml}"
        );
        assert!(
            !codex_toml.contains("config.toml:session_start"),
            "stale config.toml-prefixed keys must not be (re)written: {codex_toml}"
        );

        // grok 用户层 global hooks 文件。
        let grok_cmds = collect_commands(
            &user
                .join(".grok")
                .join("hooks")
                .join("ohmyagents-state.json"),
        );
        assert!(grok_cmds.iter().any(|c| c.contains("hst-state")));
        if cfg!(windows) {
            let g = grok_cmds.iter().find(|c| c.contains("hst-state")).unwrap();
            assert!(g.ends_with("hst-state-grok.cmd"), "M048 single path: {g}");
        }

        // kimi [[hooks]]：外来条目存活、ours 八事件补齐、strict 四字段。
        let kimi_toml = fs::read_to_string(&kimi).unwrap();
        assert!(kimi_toml.contains("theme"), "foreign key survives");
        assert!(kimi_toml.contains("\"my-tool\""), "foreign hook survives");
        assert!(kimi_toml.contains("hst-state"));
        let kv: toml::Value = toml::from_str(&kimi_toml).unwrap();
        let hooks = kv.get("hooks").and_then(|h| h.as_array()).unwrap();
        let ours: Vec<&toml::Value> = hooks
            .iter()
            .filter(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c.contains("hst-state"))
            })
            .collect();
        assert_eq!(ours.len(), 8, "eight events registered: {ours:?}");
        for h in &ours {
            assert!(h.get("event").is_some());
            assert_eq!(h.get("matcher"), None, "matcher omitted = match all");
            assert_eq!(
                h.get("timeout").and_then(|t| t.as_integer()),
                Some(10),
                "strict schema fields only"
            );
        }

        // 幂等：重部署零写入。
        let mut second = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut second).unwrap();
        assert!(
            second.wrote.is_empty(),
            "redeploy must write nothing: {:?}",
            second.wrote
        );

        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn legacy_and_project_forms_heal_to_user_shim() {
        // 用户级文件里的老形态（bare oma、旧 exe、D27 项目级 shim 路径）与
        // 重复条目：重部署一律收敛到用户级 shim 单条。
        let user = fresh_dir("heal");
        let oma = fresh_dir("oma-heal");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "oma hook --agent claude"},
                {"type": "command", "command": "D:\\old\\oma.exe hook --agent claude"},
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\hst-state.cmd claude"}]}]}}"#,
        )
        .unwrap();

        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r).unwrap();
        // 注入的三条旧形态都在 Stop 事件：收敛为现行单条；全文件无旧路径残留。
        let ours = ours_in_event(&claude, "Stop");
        assert_eq!(ours.len(), 1, "legacy forms collapse to one: {ours:?}");
        assert!(ours[0].contains("hst-state"), "{}", ours[0]);
        let cmds = collect_commands(&claude);
        assert!(!cmds.iter().any(|c| c.contains("D:\\old")), "old exe gone");
        assert!(
            !cmds.iter().any(|c| c.contains(".oma\\hooks")),
            "project shim path healed to user-level"
        );
        // 其余事件各一条现行注册。
        for event in ["SessionStart", "PreToolUse"] {
            assert_eq!(ours_in_event(&claude, event).len(), 1, "{event}");
        }
        // 幂等。
        let mut second = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut second).unwrap();
        assert!(second.wrote.is_empty(), "idempotent after healing");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn codex_user_duplicates_collapse_and_sides_preserve() {
        let user = fresh_dir("codexuser");
        let oma = fresh_dir("oma-cu");
        let path = user.join(".codex").join("hooks.json");
        ensure_parent(&path).unwrap();
        // 异侧字段保留语义沿用：Unix 侧写 command，Windows 侧只重写
        // commandWindows 且 command 字节保留。
        write_text(
            &path,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "\"/mnt/c/old/oma\" hook --agent codex",
                 "commandWindows": "& \"D:\\old2\\oma.exe\" hook --agent codex", "timeout": 10}]}]}}"#,
        )
        .unwrap();
        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut r).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let h = &v["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(
            h["command"].as_str(),
            Some("\"/mnt/c/old/oma\" hook --agent codex"),
            "foreign-OS field preserved verbatim"
        );
        let cw = h["commandWindows"].as_str().unwrap();
        assert!(!cw.contains('"') && !cw.contains('&'), "{cw}");
        assert!(cw.ends_with("/hooks/hst-state.ps1 codex"), "{cw}");
        assert!(!cw.contains("old2"), "owned field rewritten: {cw}");

        // 同形重复植入后收敛。
        let mut v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let canonical = v["hooks"]["Stop"][0]["hooks"][0].clone();
        v["hooks"]["Stop"][0]["hooks"] = Json::Array(vec![canonical.clone(), canonical]);
        fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
        let mut r2 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut r2).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let n = v["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter(|h| handler_is_ours(h))
            .count();
        assert_eq!(n, 1, "identical duplicates collapse: {v}");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn retire_strips_project_registrations_and_shims() {
        // v0.5.3 形项目（项目注册 + 项目 shim + 外来 hook）一次 init 后退役。
        let user = fresh_dir("retire-user");
        let oma = fresh_dir("retire-oma");
        let root = fresh_dir("retire-proj");
        // 项目级注册（v0.5.3 部署形态）。
        let claude = root.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "C:\\tools\\fmt.sh"},
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\hst-state.cmd claude"}]}]}}"#,
        )
        .unwrap();
        let codex = root.join(".codex").join("hooks.json");
        ensure_parent(&codex).unwrap();
        write_text(
            &codex,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "oma hook --agent codex"}]}]}}"#,
        )
        .unwrap();
        let grok = root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json");
        ensure_parent(&grok).unwrap();
        write_text(
            &grok,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\hst-state-grok.cmd"}]}]}}"#,
        )
        .unwrap();
        // 项目 shim 三件（带生成标记）+ D28 前旧名三件（v0.6.0 退役环误换单查
        // 新名成死码，D30 修）+ 用户自置同名文件保护判据。
        let shims = root.join(".oma").join("hooks");
        fs::create_dir_all(&shims).unwrap();
        fs::write(shims.join("hst-state.cmd"), "rem generated by hst init\r\n").unwrap();
        fs::write(shims.join("hst-state.sh"), "# generated by hst init\n").unwrap();
        fs::write(
            shims.join("hst-state-grok.cmd"),
            "@echo off\r\nrem generated by hst init\r\n",
        )
        .unwrap();
        fs::write(shims.join("oma-state.cmd"), "rem generated by oma init\r\n").unwrap();
        fs::write(shims.join("oma-state.sh"), "# generated by oma init\n").unwrap();
        fs::write(
            shims.join("oma-state-grok.cmd"),
            "@echo off\r\nrem generated by oma init\r\n",
        )
        .unwrap();
        fs::write(shims.join("my-own.cmd"), "@echo off\r\n").unwrap();

        // skill 面（ADR-0005）：预置 ours 两名与外来内容，一次 init 清扫。
        let ours_skill = root.join(".agents").join("skills").join("hst");
        ensure_parent(&ours_skill.join("SKILL.md")).unwrap();
        write_text(&ours_skill.join("SKILL.md"), &format!("{SKILL_MARKER}\n")).unwrap();
        let fanout = root.join(".claude").join("skills").join("hst");
        ensure_parent(&fanout.join("SKILL.md")).unwrap();
        write_text(&fanout.join("SKILL.md"), &format!("{SKILL_MARKER}\n")).unwrap();
        let owned = root.join(".grok").join("skills").join("ohmyagents");
        ensure_parent(&owned.join("SKILL.md")).unwrap();
        write_text(&owned.join("SKILL.md"), "user-owned skill\n").unwrap();
        // ours 目录带伴生资源（codex F4 外科语义）：SKILL.md 删、伴生件保留。
        let extra = root.join(".kimi-code").join("skills").join("hst");
        ensure_parent(&extra.join("SKILL.md")).unwrap();
        write_text(&extra.join("SKILL.md"), &format!("{SKILL_MARKER}\n")).unwrap();
        write_text(&extra.join("notes.md"), "supporting resource\n").unwrap();

        let report = deploy_all_with(&root, &user, &oma, host_side()).unwrap();

        // 外来 hook 保留、ours 全摘。
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        let cmds = collect_commands(&claude);
        assert_eq!(
            cmds,
            vec!["C:\\tools\\fmt.sh".to_string()],
            "ours stripped, foreign kept"
        );
        assert!(
            v.get("hooks").is_some(),
            "hooks key stays while foreign hooks live"
        );
        // codex hooks.json 只剩 ours → 整文件删除。
        assert!(
            !codex.exists(),
            "ours-only hooks.json removed: {:?}",
            report.wrote
        );
        // grok 同理。
        assert!(!grok.exists(), "ours-only grok hooks removed");
        // shim：ours 生成的（新旧名与新旧标记）删、用户自置的留。
        assert!(!shims.join("hst-state.cmd").exists());
        assert!(!shims.join("hst-state.sh").exists());
        assert!(!shims.join("hst-state-grok.cmd").exists());
        assert!(
            !shims.join("oma-state.cmd").exists(),
            "D28 legacy name retired"
        );
        assert!(
            !shims.join("oma-state.sh").exists(),
            "D28 legacy name retired"
        );
        assert!(
            !shims.join("oma-state-grok.cmd").exists(),
            "D28 legacy name retired"
        );
        assert!(shims.join("my-own.cmd").exists(), "user file untouched");
        // hooks 目录非空（my-own.cmd）不删。
        assert!(shims.is_dir());
        // skill 面已退役（ADR-0005）：ours 技能目录清扫、外来不动、外科式。
        assert!(!ours_skill.exists(), "ours skill dir removed");
        assert!(!fanout.exists(), "fan-out ours skill dir removed");
        assert!(owned.join("SKILL.md").exists(), "user-owned untouched");
        assert!(!extra.join("SKILL.md").exists());
        assert!(extra.join("notes.md").exists());
        assert!(extra.is_dir());
        assert!(
            report
                .wrote
                .iter()
                .any(|w| w.contains("(retired)") && w.contains("skills")),
            "skill retirement recorded: {:?}",
            report.wrote
        );

        // 幂等：再跑零退役写入（用户级注册也零写入）。
        let second = deploy_all_with(&root, &user, &oma, host_side()).unwrap();
        assert!(
            second.wrote.is_empty(),
            "redeploy writes nothing: {:?}",
            second.wrote
        );

        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retire_keeps_user_owned_files_deletes_ours_only() {
        // 用户自己的 settings.json（无 ours）退役不动；只有 ours 的删文件。
        let root = fresh_dir("retire2");
        let claude = root.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        let user_body = r#"{"permissions": {"defaultMode": "acceptEdits"}}"#;
        write_text(&claude, user_body).unwrap();
        let ours_only = root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json");
        ensure_parent(&ours_only).unwrap();
        write_text(
            &ours_only,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\hst-state-grok.cmd"}]}]}}"#,
        )
        .unwrap();
        let mut report = DeployReport::default();
        retire_project_hooks_with(&root, &mut report).unwrap();
        // 无 ours 的用户文件逐字节不动。
        assert_eq!(fs::read_to_string(&claude).unwrap(), user_body);
        // 只有 ours 的整文件删除。
        assert!(!ours_only.exists(), "ours-only file removed");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retire_skips_shim_removal_when_project_root_is_oma_root() {
        // F3 回归钉：项目根即 hst 根（`hst init --project $HOME`）时不得
        // 把用户级 shim 当项目残留自删。HST_ROOT 注入（共享 env 锁）。
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let root = fresh_dir("homeproj");
        let shims = root.join(".oma").join("hooks");
        fs::create_dir_all(&shims).unwrap();
        fs::write(
            shims.join("hst-state.cmd"),
            "rem generated by oma init
",
        )
        .unwrap();
        fs::write(
            shims.join("hst-state.sh"),
            "# generated by oma init
",
        )
        .unwrap();
        // 缝值取 <root>/.oma（与 base 同路）：oma_home() 即项目 .oma。
        std::env::set_var("HST_ROOT", root.join(".oma"));
        let mut report = DeployReport::default();
        retire_project_hooks_with(&root, &mut report).unwrap();
        std::env::remove_var("HST_ROOT");
        assert!(
            shims.join("hst-state.cmd").exists(),
            "user shim must survive"
        );
        assert!(
            shims.join("hst-state.sh").exists(),
            "user shim must survive"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn def_is_all_ours_three_states() {
        // F4 回归钉：全 ours 定义键判 true、全外来 false、混合 false（保守
        // 保留）。期望来自清理语义（只清 oma 自己的残留定义）。
        let ours = r#"[[hooks]]
matcher = "*"
hooks = [{ command = "C:/x/.oma/hooks/hst-state.cmd codex", timeout = 10 }]
"#;
        let groups = |text: &str| -> toml::Value {
            // 根层 [[hooks]] 解析为 Table 包 hooks 键（= 事件 group 数组）。
            let v = toml::from_str::<toml::Value>(text).unwrap();
            v.get("hooks").unwrap().clone()
        };
        assert!(def_is_all_ours(&groups(ours)));
        let foreign = r#"[[hooks]]
matcher = "*"
hooks = [{ command = "my-tool", timeout = 5 }]
"#;
        assert!(!def_is_all_ours(&groups(foreign)));
        let mixed = r#"[[hooks]]
matcher = "*"
hooks = [
  { command = "C:/x/.oma/hooks/hst-state.cmd codex", timeout = 10 },
  { command = "my-tool", timeout = 5 },
]
"#;
        assert!(!def_is_all_ours(&groups(mixed)));
    }

    #[test]
    fn codex_trust_identity_shape_and_determinism() {
        assert_eq!(event_label("SessionStart"), "session_start");
        assert_eq!(event_label("UserPromptSubmit"), "user_prompt_submit");
        // These events drop the matcher from the hashed identity.
        assert_eq!(hashed_matcher("UserPromptSubmit", Some("*")), None);
        assert_eq!(hashed_matcher("PreToolUse", Some("*")), Some("*".into()));
        assert_eq!(hashed_matcher("Stop", Some("*")), None);

        let handler = serde_json::json!({
            "type": "command",
            "command": "oma",
            "commandWindows": "D:\\bin\\oma.exe",
            "timeout": 10,
            "async": false
        });
        let h1 = codex_hook_hash("PreToolUse", Some("*"), &handler).unwrap();
        let h2 = codex_hook_hash("PreToolUse", Some("*"), &handler).unwrap();
        assert_eq!(h1, h2, "hash must be deterministic");
        assert!(h1.starts_with("sha256:"), "{h1}");
        // Matcher participates for matcher-respecting events.
        let h3 = codex_hook_hash("PreToolUse", Some("Bash"), &handler).unwrap();
        assert_ne!(h1, h3);
        // Timeout clamps for SessionEnd. Feed a command without
        // commandWindows so the identity is platform-independent: on
        // Windows the windows field would leak into the hash and the
        // convergence below would hold only there.
        let se = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":10,"async":false}),
        )
        .unwrap();
        let se_clamped = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":99}),
        )
        .unwrap();
        let se_again = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":3,"async":false}),
        )
        .unwrap();
        assert_eq!(se_clamped, se_again, "clamped 99 and explicit 3 converge");
        assert_eq!(se, se_clamped, "10 and 99 both clamp to 3 and converge");
    }

    #[test]
    fn codex_trust_entries_use_real_indices_and_skip_foreign() {
        let hooks = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Bash", "hooks": [
                        {"type": "command", "command": "C:\\tools\\fmt.sh"}
                    ]},
                    {"matcher": "*", "hooks": [
                        {"type": "command", "command": "oma",
                         "commandWindows": "D:\\oma.exe", "timeout": 10}
                    ]}
                ]
            }
        });
        let cfg = Path::new(r"D:\\proj\\.codex\\config.toml");
        let entries = codex_trust_entries(&hooks, cfg).unwrap();
        assert_eq!(entries.len(), 1, "foreign handlers are not trusted for");
        let (key, hash) = &entries[0];
        assert!(
            key.ends_with(":pre_tool_use:1:0"),
            "ours sits at group 1 handler 0, got {key}"
        );
        assert!(key.starts_with("D:"), "key_source is the plain config path");
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn codex_trust_entries_seed_windows_only_commandwindows() {
        let hooks = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "*", "hooks": [
                        {"type": "command",
                         "commandWindows": "& \"D:\\ohmyenv\\cargo\\bin\\oma.exe\" hook --agent codex",
                         "timeout": 10}
                    ]}
                ]
            }
        });
        let cfg = Path::new(r"D:\proj\.codex\config.toml");
        let entries = codex_trust_entries(&hooks, cfg).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "commandWindows-only oma handler must be seeded: {entries:?}"
        );
        assert!(
            entries[0].0.ends_with(":pre_tool_use:0:0"),
            "got {}",
            entries[0].0
        );
        assert!(entries[0].1.starts_with("sha256:"));
    }

    #[test]
    fn identical_duplicate_ours_handlers_collapse_to_one() {
        // 同形重复条目收敛（claude 用户面）。
        let user = fresh_dir("dedup");
        let oma = fresh_dir("oma-dd");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(&claude, r#"{"hooks": {}}"#).unwrap();
        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r).unwrap();
        let mut v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        let canonical = v["hooks"]["Stop"][0]["hooks"][0].clone();
        let dup = json!({ "matcher": "*", "hooks": [canonical.clone(), canonical.clone()] });
        let single = json!({ "matcher": "*", "hooks": [canonical.clone()] });
        v["hooks"]["Stop"]
            .as_array_mut()
            .unwrap()
            .extend([single, dup.clone(), dup]);
        fs::write(&claude, serde_json::to_string(&v).unwrap()).unwrap();
        let mut r2 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r2).unwrap();
        let ours = ours_in_event(&claude, "Stop");
        assert_eq!(ours.len(), 1, "duplicates must collapse: {ours:?}");
        let mut r3 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r3).unwrap();
        assert!(
            r3.wrote.is_empty(),
            "idempotent after dedup: {:?}",
            r3.wrote
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn kimi_merge_dedupes_and_upgrades_bare_forms() {
        // bare oma 老条目升级为用户级 shim 形态，外来保留，幂等。
        let user = fresh_dir("kimi");
        let oma = fresh_dir("oma-kimi");
        let cfg = user.join(".kimi-code").join("config.toml");
        ensure_parent(&cfg).unwrap();
        write_text(
            &cfg,
            "[[hooks]]\nevent = \"Stop\"\ncommand = \"oma hook --agent kimi\"\ntimeout = 30\n\
             [[hooks]]\nevent = \"Stop\"\ncommand = \"my-tool\"\n",
        )
        .unwrap();
        let mut toml = read_toml(&cfg).unwrap();
        let cmd = kimi_hook_command(&oma, host_side());
        assert!(apply_kimi_hooks(&mut toml, &cmd, &["SessionStart", "Stop"]).unwrap());
        // 再跑不变（幂等判据）。
        assert!(!apply_kimi_hooks(&mut toml, &cmd, &["SessionStart", "Stop"]).unwrap());
        let arr = toml.get("hooks").and_then(|h| h.as_array()).unwrap();
        assert!(arr
            .iter()
            .any(|h| h.get("command").and_then(|c| c.as_str()) == Some("my-tool")));
        assert!(!arr
            .iter()
            .any(|h| h.get("command").and_then(|c| c.as_str()) == Some("oma hook --agent kimi")),
            "bare form upgraded away");
        assert_eq!(arr.len(), 3, "sessionstart + stop ours + foreign my-tool");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }
}

#[cfg(test)]
mod skill_tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-skill-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn retire_skills_ours_only_and_surgical() {
        // ADR-0005：ours（marker 家族或旧静态版）退役、用户内容不动、
        // 外科式（伴生资源保留，codex F4）。
        let root = tmp("retire");
        let ours = root.join(".agents").join("skills").join("hst");
        std::fs::create_dir_all(&ours).unwrap();
        std::fs::write(ours.join("SKILL.md"), format!("{SKILL_MARKER}\n")).unwrap();
        let legacy = root.join(".claude").join("skills").join("ohmyagents");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(&legacy.join("SKILL.md"), LEGACY_SKILL_MD).unwrap();
        // D30 前缀双收回归钉：v0.5.x 旧尾巴 marker（rerun oma init）也要认。
        let v05x = root.join(".grok").join("skills").join("ohmyagents");
        std::fs::create_dir_all(&v05x).unwrap();
        std::fs::write(
            &v05x.join("SKILL.md"),
            "<!-- generated by oma init; rerun oma init to sync the command map -->\n\n旧命令图正文\n",
        )
        .unwrap();
        let owned = root.join(".kimi-code").join("skills").join("hst");
        std::fs::create_dir_all(&owned).unwrap();
        std::fs::write(&owned.join("SKILL.md"), "我的私货 skill\n").unwrap();
        // 外科式：ours 目录带伴生资源，SKILL.md 删、伴生件与目录保留。
        let extra = root.join(".claude").join("skills").join("hst");
        std::fs::create_dir_all(&extra).unwrap();
        std::fs::write(&extra.join("SKILL.md"), format!("{SKILL_MARKER}\n")).unwrap();
        std::fs::write(&extra.join("notes.md"), "supporting resource\n").unwrap();

        let mut r = DeployReport::default();
        retire_skills(&root, &mut r);
        assert!(!ours.exists(), "ours skill dir removed (empty after sweep)");
        assert!(!legacy.exists(), "legacy static ours removed");
        assert!(
            !v05x.join("SKILL.md").exists(),
            "v0.5.x marker prefix ours removed"
        );
        assert!(owned.join("SKILL.md").exists(), "user-owned untouched");
        assert!(!extra.join("SKILL.md").exists());
        assert!(extra.join("notes.md").exists(), "companion resource kept");
        assert!(extra.is_dir());
        assert_eq!(r.wrote.len(), 4, "four retirements recorded");
        // 幂等：再跑零退役。
        let mut r2 = DeployReport::default();
        retire_skills(&root, &mut r2);
        assert!(r2.wrote.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retire_user_skills_sweeps_four_roots_marker_and_signature() {
        // 用户级清扫面（codex 评审 F1）：四家 skills 根乘两名。`hst skill
        // --write` 落的签名件（.claude）与 home-root init fan-out 落的
        // marker 件（.agents/.grok 等）都要收；用户手改或他源不动；外科式
        // 伴生资源保留。
        let home = tmp("user");
        let claude = home.join(".claude").join("skills").join("hst");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(
            &claude.join("SKILL.md"),
            "> 本文件由 `hst skill` 从 hst 活命令树自适应生成。\n",
        )
        .unwrap();
        std::fs::write(claude.join("notes.md"), "supporting resource\n").unwrap();
        let agents = home.join(".agents").join("skills").join("hst");
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(&agents.join("SKILL.md"), format!("{SKILL_MARKER}\n")).unwrap();
        let grok = home.join(".grok").join("skills").join("hst");
        std::fs::create_dir_all(&grok).unwrap();
        std::fs::write(&grok.join("SKILL.md"), format!("{SKILL_MARKER}\n")).unwrap();
        let legacy = home.join(".kimi-code").join("skills").join("ohmyagents");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(
            &legacy.join("SKILL.md"),
            "> 本文件由 oma skill 从活命令树自适应生成。\n",
        )
        .unwrap();
        let mut r = DeployReport::default();
        retire_user_skills(&home, &mut r);
        assert!(!claude.join("SKILL.md").exists());
        assert!(claude.join("notes.md").exists(), "companion resource kept");
        assert!(!agents.exists(), "home-root init fan-out marker swept");
        assert!(!grok.exists(), "grok user-level ours swept");
        assert!(!legacy.exists(), "legacy ours dir removed (empty)");
        assert_eq!(r.wrote.len(), 4);
        // 用户手改（签名被覆写）不动：种一份无签名内容再跑。
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(&claude.join("SKILL.md"), "我的私货 skill\n").unwrap();
        let mut r2 = DeployReport::default();
        retire_user_skills(&home, &mut r2);
        assert!(claude.join("SKILL.md").exists(), "user-owned untouched");
        assert!(r2.wrote.is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
