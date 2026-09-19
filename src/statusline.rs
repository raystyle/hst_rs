//! agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
//! - claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
//!   读改写，保留 env/permissions 等，只覆盖 statusLine 键）
//! - codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
//!   `status_line` 为内置项 ID 数组（Codex 无外部命令面，S016）
//! 状态栏脚本本体（pwsh）随 hst 释放到 `~/.hst/statusline/`。
//! 用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
//! 包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；hst 段 = 当前
//! agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025），另探测
//! agent 宿主 shell（macOS 走 ps 兜底）。

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::yolo::{read_toml, toml_write};

/// 状态栏脚本 HEAD：param、首尾强制 UTF-8（输出侧 CP936 控制台下 emoji 会被
/// 替换成字面 `??`，S024；输入侧重定向 stdin 默认按 OEM 码页解码，中文
/// cwd 会进来即花成「缁跨洘」形 GBK 误解码，D28 补钉）、stdin JSON 解析
/// （claude code 供给 model 等；codex 无 stdin 数据时退化）、Seg 与 FmtTok、
/// FmtDur 助手、`$parts` 收集器。
const PS1_HEAD: &str = r#"
param([string]$AgentName = 'agent')
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
# stdin 字节级读取显式 UTF-8 解码：[Console]::In 随控制台码页（中文端
# CP936）解码重定向输入，agent 喂的 UTF-8 JSON 里中文路径进来即花
# （「团队」成「缁跨洘」形 GBK 误解码，D28 补钉；S024 只钉过输出侧）。
$raw = $null
try {
    $ms = [System.IO.MemoryStream]::new()
    [Console]::OpenStandardInput().CopyTo($ms)
    if ($ms.Length -gt 0) { $raw = [System.Text.Encoding]::UTF8.GetString($ms.ToArray()) }
} catch {}
$d = $null
if (-not [string]::IsNullOrWhiteSpace($raw)) { try { $d = $raw | ConvertFrom-Json } catch {} }
# Grok TUI 字体没有 Nerd 私用区字形，PUA 图标显示成替换符（M046）。
$nerd = $AgentName -ne 'grok'

function Seg([string]$text, [string]$code = '') {
    if ([string]::IsNullOrWhiteSpace($text)) { return $null }
    if ($code) { return "$([char]27)[${code}m$text$([char]27)[0m" }
    return $text
}
function FmtTok([double]$n) {
    if ($n -ge 1MB) { return '{0:N0}M' -f ($n / 1MB) }
    if ($n -ge 1KB) { return '{0:N0}k' -f ($n / 1KB) }
    return [string][long]$n
}
# 会话时长人性化：3d4h / 4h12m / 12m30s / 30s
function FmtDur([double]$ms) {
    $s = [math]::Floor($ms / 1000)
    if ($s -ge 86400) { return '{0}d{1}h' -f [math]::Floor($s / 86400), [math]::Floor(($s % 86400) / 3600) }
    if ($s -ge 3600) { return '{0}h{1}m' -f [math]::Floor($s / 3600), [math]::Floor(($s % 3600) / 60) }
    if ($s -ge 60) { return '{0}m{1}s' -f [math]::Floor($s / 60), ($s % 60) }
    return "${s}s"
}

$parts = [System.Collections.Generic.List[string]]::new()
"#;

/// 状态栏的COMMON面（细则见模块文档与集成测试）。
/// 状态栏的才拼入面（细则见模块文档与集成测试）。
const PS1_COMMON: &str = r#"
# ── 工作目录与仓库根（hst 段与目录段共用）──
$dir = $null
if ($d.workspace) { $dir = "$($d.workspace.current_dir)" }
if (-not $dir -or $dir -eq '.') { $dir = "$($d.cwd)" }
if (-not $dir -or $dir -eq '.') { $dir = "$(Get-Location)" }
$root = (& git -C $dir rev-parse --show-toplevel 2>$null | Out-String).Trim()
"#;

/// shell 段：agent 宿主 shell（祖先链跳过 agent 本体，向上找最近 shell）。
const SEG_SHELL: &str = r#"
# ── Shell 段：agent 宿主 shell（祖先链跳过 agent 本体，向上找最近 shell）──
$shellName = $null
$shells = '^(pwsh|powershell|bash|zsh|sh|fish|cmd|nu|elvish|xonsh)'
$agentStems = '^(node|claude|codex|grok|kimi|hst)'
$chain = @()
if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    $p = Get-Process -Id $PID -ErrorAction SilentlyContinue
    for ($i = 0; $i -lt 8 -and $p; $i++) {
        try { $p = $p.Parent } catch { $p = $null }
        if (-not $p) { break }
        $chain += $p.ProcessName.ToLowerInvariant()
    }
} else {
    # Unix：Linux/WSL 走 /proc；macOS 无 /proc，回退 BSD ps（-o ppid=/comm=）。
    $cur = $PID
    for ($i = 0; $i -lt 8; $i++) {
        $ppid = $null
        $procStat = Get-Content "/proc/$cur/status" -ErrorAction SilentlyContinue
        if ($procStat) {
            $ppidLine = $procStat | Where-Object { $_ -match '^PPid:\s+(\d+)' } | Select-Object -First 1
            if ($ppidLine -and $ppidLine -match '^PPid:\s+(\d+)') { $ppid = [int]$Matches[1] }
        } else {
            $psOut = (& ps -o ppid= -p $cur 2>$null | Out-String).Trim()
            if ($psOut -match '^\d+$') { $ppid = [int]$psOut }
        }
        if (-not $ppid -or $ppid -le 1) { break }
        $cur = $ppid
        $comm = (Get-Content "/proc/$cur/comm" -ErrorAction SilentlyContinue | Select-Object -First 1)
        if (-not $comm) { $comm = (& ps -o comm= -p $cur 2>$null | Out-String).Trim() }
        if ($comm) { $chain += $comm.Trim().ToLowerInvariant() }
    }
}
if ($chain.Count -gt 0) {
    $agentIdx = -1
    for ($i = 0; $i -lt $chain.Count; $i++) {
        if ($chain[$i] -match $agentStems) { $agentIdx = $i; break }
    }
    $search = if ($agentIdx -ge 0 -and $agentIdx + 1 -lt $chain.Count) { $chain[($agentIdx + 1)..($chain.Count - 1)] } else { $chain }
    foreach ($n in $search) {
        if ($n -match $shells) { $shellName = $n -replace '\.exe$', ''; break }
    }
}
if (-not $shellName -and $env:SHELL) { $shellName = (Split-Path -Leaf $env:SHELL) }
if ($shellName) {
    $shKey = if ($shellName -match '^(pwsh|powershell)') { 'shell-pwsh' } else { 'shell' }
    $shLabel = ApplyFmt (Tmpl 'shell') @{ icon = (Ico $shKey); name = $shellName }
    $sh = Seg $shLabel '38;5;245'
    if ($sh) { $parts.Add($sh) }
}
"#;

/// dir 段：完整路径（用户定调 2026-09-02）。
const SEG_DIR: &str = r#"
# ── 目录：完整路径（用户定调 2026-09-02）──
if ($dir) {
    $p = Seg (ApplyFmt (Tmpl 'dir') @{ path = $dir }) '38;5;39'
    if ($p) { $parts.Add($p) }
}
"#;

/// clock 段（D51）：第一行年月日与当前时间。Get-Date 本机取时零子进程；
/// 刷新时机随状态栏渲染事件（不独立走秒）；用户级 `{datetime}` 占位与
/// `clock` 图标键可定制。
const SEG_CLOCK: &str = r#"
# ── clock 段：年月日 + 当前时间（D51；Get-Date 零子进程）──
$dt = (Get-Date -Format 'yyyy-MM-dd HH\:mm')
if ($dt) {
    $ck = Seg (ApplyFmt (Tmpl 'clock') @{ icon = (Ico 'clock'); datetime = $dt }) '38;5;245'
    if ($ck) { $parts.Add($ck) }
}
"#;

/// hst 段（D45 更名，原 oma 段）：当前 agent 名 + 实时四态（hook 状态通
/// 道与会话闸，机读标记 S025；D28 用户级 session 分键读序）。
const SEG_OMA: &str = r#"
# ── hst 段：当前 agent 名 + 版本（D46）+ 实时状态（hook 状态通道；机读
# 标记见 S025，D46 起可带连字符版本形 `claude-2.1.270:working`）──
# agent 名：hst 会话 env 优先，部署参数次之（每家配置注入自家名字）。
$agent = if ($env:HST_AGENT) { $env:HST_AGENT } else { $AgentName }
# 版本两级（D46，codex 设计轮 F1/F2/F4/F5/F6 收口）：payload version 字
# 段优先（claude/kimi/grok stdin 契约都带自家版本，kimi/grok 二进制实
# 证见 S034 D46 追记），取值与 probe 输出走同一条归一化（`数字.数字`
# 起头 token；非数字如 nightly 不显示，回落旧形——显示面与 verify 判据
# 同源，S025）。否则本地探：定位序对齐 agents 探测（HST_<AGENT>_BIN 显
# 式钉优先，其次 HST_AGENT_PATH 目录，最后 PATH；hst 自管根与默认位不
# 参与，差异记档于 git 历史与集成测试）；探针只对四家白名单放行（spawn 面与缓存文件名
# 受控）；键 = (终目标路径, mtime, size) 三元组（软链解析一层 LinkTarget；
# 壳 shim 与保留 mtime 的升级由 7 天兜底重探覆盖）；缓存按 agent 单文
# 件、临时件加改名原子落盘，读失败按 miss；探测失败记空值加 5 分钟静
# 默窗（防每帧重试 spawn）。HST_VER_CACHE_DIR 覆盖缓存目录（verify 与
# 测试隔离通道，对齐 HST_STATE_FILE 先例）。$agent 本体保持纯名（下方
# state 文件定位吃它），版本只进 $agentDisp 渲染值。
$ver = $null
if ($d.version) {
    $pv = "$($d.version)"
    if ($pv -match '([0-9]+(\.[0-9]+)+[A-Za-z0-9.+-]*)') { $ver = $Matches[1] }
}
if (-not $ver -and ($agent -match '^(claude|codex|grok|kimi)$')) {
    $binPath = $null
    $envBin = [Environment]::GetEnvironmentVariable("HST_$($agent.ToUpper())_BIN")
    if ($envBin -and (Test-Path -LiteralPath $envBin)) { $binPath = $envBin }
    if (-not $binPath -and $env:HST_AGENT_PATH) {
        $sep = if ($IsWindows) { ';' } else { ':' }
        foreach ($ad in ($env:HST_AGENT_PATH -split [regex]::Escape($sep))) {
            if (-not $ad) { continue }
            foreach ($c in @((Join-Path $ad $agent), (Join-Path $ad "$agent.exe"), (Join-Path $ad "$agent.cmd"))) {
                if (Test-Path -LiteralPath $c) { $binPath = $c; break }
            }
            if ($binPath) { break }
        }
    }
    if (-not $binPath) {
        $gcmd = @(Get-Command $agent -ErrorAction SilentlyContinue)[0]
        if ($gcmd -and $gcmd.Source) { $binPath = "$($gcmd.Source)" }
    }
    if ($binPath) {
        $gi = $null
        try {
            $gi = Get-Item -LiteralPath $binPath -ErrorAction Stop
            # 软链解析一层：升级动目标不动链本体时键仍变。LinkTarget 存的
            # 是链接内原样串，相对路径须按链接所在目录拼（按 CWD 解析会取
            # 到无关件或空转，npm/nvm 的 ~/.local/bin 相对软链即此形态，
            # codex diff 轮 F2）。
            if ($gi.LinkTarget) {
                $lt = "$($gi.LinkTarget)"
                if (-not [IO.Path]::IsPathRooted($lt)) { $lt = Join-Path (Split-Path -Parent $gi.FullName) $lt }
                try { $g2 = Get-Item -LiteralPath $lt -ErrorAction Stop; if ($g2) { $gi = $g2 } } catch {}
            }
        } catch { $gi = $null }
        if ($gi) {
            $binPath = "$($gi.FullName)"
            # mtime / probed_at 记 ticks 整数串：ConvertFrom-Json 会把 ISO 日
            # 期串自动转 [DateTime]（文化格式化后与 'o' 串永不相等，本机实
            # 弹踩坑），整数键免转换、比较精确且文化无关。
            $binMtime = "$($gi.LastWriteTimeUtc.Ticks)"
            $binSize = "$($gi.Length)"
            $verDir = if ($env:HST_VER_CACHE_DIR) { $env:HST_VER_CACHE_DIR } elseif ($HOME) { Join-Path (Join-Path $HOME '.hst') 'cache' }
            $verFile = if ($verDir) { Join-Path $verDir "agent-version-$agent.json" } else { $null }
            $ent = $null
            if ($verFile -and (Test-Path -LiteralPath $verFile)) {
                try { $ent = Get-Content -Raw -LiteralPath $verFile | ConvertFrom-Json } catch { $ent = $null }
            }
            $needProbe = $true
            $nowTicks = [DateTime]::UtcNow.Ticks
            $win5 = [TimeSpan]::FromMinutes(5).Ticks
            $d7 = [TimeSpan]::FromDays(7).Ticks
            if ($ent -and ("$($ent.bin)" -eq $binPath) -and ("$($ent.mtime)" -eq $binMtime) -and ("$($ent.size)" -eq $binSize)) {
                $probedTicks = $null
                try { $probedTicks = [Int64]"$($ent.probed_at)" } catch { $probedTicks = $null }
                if ($ent.version) {
                    $ver = "$($ent.version)"
                    $needProbe = $false
                    # 7 天兜底：壳与保留 mtime 的升级键不变，到期强制重探；
                    # probed_at 坏值按到期处理。
                    if ($null -eq $probedTicks -or ($nowTicks - $probedTicks) -ge $d7) { $ver = $null; $needProbe = $true }
                } elseif ($null -ne $probedTicks -and ($nowTicks - $probedTicks) -lt $win5) {
                    # 上次探测失败：静默窗内不重试（保持无版本），窗外重探。
                    $needProbe = $false
                }
            }
            # 无缓存可写（HST_VER_CACHE_DIR 与 $HOME 双缺）就不探：否则每
            # 帧裸 spawn，静默窗与三元组键全失效（codex diff 轮 F3），天然
            # 回落旧形。
            if ($needProbe -and $verFile) {
                # 两步取值（codex G1/G2）：stdout 优先（2>$null 静默 stderr），
                # 正则未中再合流取 stderr（此步只在 stdout 无版本样串时走，
                # 不存在噪声抢跑；注意不能写 2>&1 1>$null——重定向按出现次
                # 序处理，1>$null 会把并流后的整条丢空、恒得空串，codex 实
                # 弹证伪过）——与 agents.rs read_version「stdout 空取 stderr」
                # 同口径（多一次 spawn 只在缓存 miss 路径，可接受）。
                $vout = ''
                try { $vout = (& $binPath --version 2>$null | Out-String).Trim() } catch { $vout = '' }
                if ($vout -notmatch '([0-9]+(\.[0-9]+)+[A-Za-z0-9.+-]*)') {
                    try { $vout = (& $binPath --version 2>&1 | Out-String).Trim() } catch { $vout = '' }
                }
                if ($vout -match '([0-9]+(\.[0-9]+)+[A-Za-z0-9.+-]*)') { $ver = $Matches[1] } else { $ver = $null }
                try {
                    $vd = Split-Path -Parent $verFile
                    if (-not (Test-Path -LiteralPath $vd)) { New-Item -ItemType Directory -Path $vd -Force | Out-Null }
                    $tmp = "$verFile.tmp"
                    @{ version = "$ver"; bin = $binPath; mtime = $binMtime; size = $binSize; probed_at = "$nowTicks" } |
                        ConvertTo-Json -Compress | Set-Content -LiteralPath $tmp -NoNewline -Encoding UTF8
                    # [IO.File]::Move 三平台覆盖语义确定（Move-Item -Force
                    # 在 Unix 覆盖分支有历史反复，codex diff 轮 F5）。
                    [IO.File]::Move($tmp, $verFile, $true)
                } catch {}
            }
        }
    }
}
$agentDisp = if ($ver) { "$agent-$ver" } else { $agent }
# 状态读序（D28，D45 去 oma 纪元旧名）：1) HST_STATE_FILE 覆盖；2) 用户级
# session 键 ~/.hst/state/<agent>-<session>.json（session 取 payload
# session_id / sessionId）；3) 用户级 <agent>.json（agent 最新）；4) 项目级
# 旧协议 .hst/state/<agent>.json。候选按序试，会话闸不符续找。
$sid = $null
if ($d) {
    if ($d.session_id) { $sid = "$($d.session_id)" }
    elseif ($d.sessionId) { $sid = "$($d.sessionId)" }
}
$state = $null
$stateFile = $null
if ($env:HST_STATE_FILE) {
    $stateFile = $env:HST_STATE_FILE
    if (Test-Path $stateFile) {
        try {
            $st = Get-Content -Raw $stateFile | ConvertFrom-Json
            if ($st.state) { $state = "$($st.state)" }
        } catch {}
    }
} else {
    $hstStateDir = $null
    if ($HOME) { $hstStateDir = Join-Path (Join-Path $HOME '.hst') 'state' }
    $cands = @()
    if ($hstStateDir -and $sid) { $cands += (Join-Path $hstStateDir "$agent-$sid.json") }
    if ($hstStateDir) { $cands += (Join-Path $hstStateDir "$agent.json") }
    $base = if ($root) { $root } else { $dir }
    if ($base) {
        $cands += (Join-Path (Join-Path (Join-Path $base '.hst') 'state') "$agent.json")
    }
    foreach ($c in $cands) {
        if (-not (Test-Path $c)) { continue }
        try {
            $st = Get-Content -Raw $c | ConvertFrom-Json
            if (-not $st.state) { continue }
            # 会话闸：记录带 session 且与当前会话不符 → 是别的（可能已死）
            # 会话遗留，续找下一候选（session 键候选按构造恒匹配）。
            if ($sid -and $st.session -and ("$($st.session)" -ne $sid)) { continue }
            $stateFile = $c
            $state = "$($st.state)"
            break
        } catch { continue }
    }
}
if (-not $state) { $state = 'unknown' }
# 注册哨兵（REQ-014，issue #31）：unknown 时探注册面（按 agent 定位配置
# 文件，标记串 hst-state.sh），缺失即升格 no-hook! 可见告警；节流窗（每
# agent 1 小时）到期才 best-effort 自愈调 hst hook init（幂等重注册，仅
# hook 面不动 yolo 与状态栏配置；hst 不在 PATH 或失败静默，不阻塞渲染）。
if ($state -eq 'unknown' -and $HOME) {
    $regFile = switch ($agent) {
        'claude' { Join-Path $HOME '.claude/settings.json' }
        'codex' { Join-Path $HOME '.codex/hooks.json' }
        'grok' { Join-Path $HOME '.grok/hooks/ohmyagents-state.json' }
        'kimi' { Join-Path $HOME '.kimi-code/config.toml' }
        default { $null }
    }
    $regOk = $false
    if ($regFile -and (Test-Path -LiteralPath $regFile)) {
        try { $regOk = [bool](Select-String -Path $regFile -Pattern 'hst-state.sh' -SimpleMatch -Quiet) } catch { $regOk = $false }
    }
    if ($regFile -and -not $regOk) {
        $state = 'no-hook!'
        $stampDir = Join-Path $HOME '.hst/state'
        $stamp = Join-Path $stampDir ('.hookcheck-' + $agent)
        $due = $true
        try {
            if (Test-Path -LiteralPath $stamp) {
                $due = (([DateTime]::UtcNow - (Get-Item -LiteralPath $stamp).LastWriteTimeUtc).TotalSeconds -ge 3600)
            }
        } catch {}
        if ($due) {
            try {
                if (-not (Test-Path -LiteralPath $stampDir)) { New-Item -ItemType Directory -Path $stampDir -Force | Out-Null }
                [IO.File]::WriteAllText($stamp, 'x')
            } catch {}
            try { & hst hook init *> $null } catch {}
        }
    }
}
$stateColor = switch ($state) {
    'idle' { '38;5;108' }
    'working' { '38;5;179' }
    'blocked' { '38;5;203' }
    'no-hook!' { '38;5;203' }
    default { '38;5;245' }
}
$hstTxt = ApplyFmt (Tmpl 'hst') @{ icon = (Ico 'hst'); agent = $agentDisp; state = $state }
$parts.Add((Seg $hstTxt $stateColor))
"#;

/// model 段：display_name 优先，回退 id。
const SEG_MODEL: &str = r#"
# ── 模型（display_name 优先，回退 id）──
$model = $null
if ($d.model) {
    $model = if ($d.model.display_name) { "$($d.model.display_name)" } else { "$($d.model.id)" }
}
if ($model) {
    $modelTxt = ApplyFmt (Tmpl 'model') @{ icon = (Ico 'model'); model = $model }
    $m = Seg $modelTxt '38;5;147'
    if ($m) { $parts.Add($m) }
}
"#;

/// context 段：已用百分比加构成占比（{mix} = transcript 估算的
/// system/tools/messages 三分段，无 transcript 时空串）。
const SEG_CONTEXT: &str = r#"
# ── 上下文：󰍛 N%{mix}（构成 = transcript 估算 s/t/m 三分段占比）──
if ($d.context_window) {
    $cw = $d.context_window
    $win = [double]$cw.context_window_size
    $usedPct = $null
    if ($null -ne $cw.used_percentage) {
        $usedPct = [math]::Floor([double]$cw.used_percentage)
    } elseif ($null -ne $cw.remaining_percentage) {
        $usedPct = 100 - [math]::Floor([double]$cw.remaining_percentage)
    }
    if ($null -ne $usedPct -and $win -gt 0) {
        $usedTok = [math]::Round($win * $usedPct / 100)
        $ctxTxt = ApplyFmt (Tmpl 'context') @{ icon = (Ico 'context'); pct = [string]$usedPct; used = (FmtTok $usedTok); window = (FmtTok $win); mix = "$mixTxt" }
        $c = Seg $ctxTxt '38;5;116'
        if ($c) { $parts.Add($c) }
    }
}
"#;

/// CTXPROBE（D40）：transcript 尾段解析，一次供 context 构成（{mix}）与
/// tools 计数两段消费（拼入门控见 assemble_statusline_ps1；D43 起 {mix}
/// 退出缺省模板，纯「百分比加绝对值」配置不再拼入本探针，只有 tools 段
/// 或显式 {mix} 模板才是消费者；kimi 无 transcript_path 时秒过，300ms
/// 预算不伤）。构成按行字符量三分
/// （system+summary / tool_use+tool_result / 其余 user+assistant 文本）估
/// 算占比——transcript 不含真实 token 计量，占比是近似口径（wsl 总台
/// D40 需求「JSON 不含构成时 transcript 解析」路径）。
const PS1_CTXPROBE: &str = r#"
# ── CTXPROBE：transcript 尾段解析（构成占比与工具计数共用）──
$toolCalls = $null
$mixTxt = ''
$tp = if ($d.transcript_path) { "$($d.transcript_path)" } else { $null }
if ($tp -and (Test-Path $tp)) {
    $toolCalls = 0
    [double]$cs = 0; [double]$ct = 0; [double]$cm = 0
    foreach ($ln in (Get-Content $tp -Tail 500)) {
        if ([string]::IsNullOrWhiteSpace($ln)) { continue }
        try { $e = $ln | ConvertFrom-Json } catch { continue }
        if ($e.type -eq 'system' -or $e.type -eq 'summary') { $cs += $ln.Length; continue }
        $content = $null
        if ($e.message -and $e.message.content) { $content = $e.message.content }
        if ($content -is [string]) { $cm += $ln.Length; continue }
        if ($content) {
            $sawTool = $false
            foreach ($b in @($content)) {
                if (-not $b -or -not $b.type) { continue }
                if ($b.type -eq 'tool_use') { $toolCalls++; $sawTool = $true }
                elseif ($b.type -eq 'tool_result') { $sawTool = $true }
            }
            if ($sawTool) { $ct += $ln.Length } else { $cm += $ln.Length }
        } else { $cm += $ln.Length }
    }
    $tot = $cs + $ct + $cm
    if ($tot -gt 0) {
        $psx = [int][math]::Round(100 * $cs / $tot)
        $ptx = [int][math]::Round(100 * $ct / $tot)
        $mixTxt = ' [s{0} t{1} m{2}]' -f $psx, $ptx, (100 - $psx - $ptx)
    }
}
"#;

/// tools 段：工具调用计数（transcript 尾 500 行 tool_use 出现次数；近似
/// 口径，长会话低估，与 agents 探测同口径）。
const SEG_TOOLS: &str = r#"
# ── 工具计数： N（transcript 尾 500 行 tool_use 次数，近似口径）──
if ($null -ne $toolCalls) {
    $tTxt = ApplyFmt (Tmpl 'tools') @{ icon = (Ico 'tools'); count = [string]$toolCalls }
    $t = Seg $tTxt '38;5;215'
    if ($t) { $parts.Add($t) }
}
"#;

/// mcp 段：MCP server 计数。取值序：stdin JSON `mcp_servers`（claude 供）>
/// `~/.claude.json` `mcpServers` 键数 + 项目 `.mcp.json` 键数（用户级加项
/// 目级合计）；全缺省省略。
const SEG_MCP: &str = r#"
# ── MCP 计数：⋕ N（stdin mcp_servers > ~/.claude.json + .mcp.json 键数）──
$mcpCount = $null
if ($d.mcp_servers) { $mcpCount = @($d.mcp_servers).Count }
if (-not $mcpCount) {
    $slHome = if ($HOME) { $HOME } else { $env:USERPROFILE }
    if ($slHome) {
        $cj = Join-Path $slHome '.claude.json'
        if (Test-Path $cj) {
            try {
                $cjv = Get-Content -Raw $cj | ConvertFrom-Json
                if ($cjv.mcpServers) { $mcpCount = @($cjv.mcpServers.PSObject.Properties).Count }
            } catch {}
        }
    }
    $mcpJson = if ($dir) { Join-Path $dir '.mcp.json' } else { $null }
    if ($mcpJson -and (Test-Path $mcpJson)) {
        try {
            $mj = Get-Content -Raw $mcpJson | ConvertFrom-Json
            $n = 0
            if ($mj.mcpServers) { $n = @($mj.mcpServers.PSObject.Properties).Count }
            if ($n -gt 0) { if ($mcpCount) { $mcpCount += $n } else { $mcpCount = $n } }
        } catch {}
    }
}
if ($mcpCount) {
    $mTxt = ApplyFmt (Tmpl 'mcp') @{ icon = (Ico 'mcp'); count = [string]$mcpCount }
    $m = Seg $mTxt '38;5;140'
    if ($m) { $parts.Add($m) }
}
"#;

/// tokens 段：token 用量绝对值（已用/窗口；D40 入默认行、D43 起退出默认
/// 行改为显式选用，token 绝对值缺省并入 context 段括号）。
const SEG_TOKENS: &str = r#"
# ── Token 用量：N/M（已用/窗口绝对值）──
if ($d.context_window) {
    $cw = $d.context_window
    $win2 = [double]$cw.context_window_size
    $usedPct2 = $null
    if ($null -ne $cw.used_percentage) {
        $usedPct2 = [math]::Floor([double]$cw.used_percentage)
    } elseif ($null -ne $cw.remaining_percentage) {
        $usedPct2 = 100 - [math]::Floor([double]$cw.remaining_percentage)
    }
    if ($null -ne $usedPct2 -and $win2 -gt 0) {
        $usedTok2 = [math]::Round($win2 * $usedPct2 / 100)
        $kTxt = ApplyFmt (Tmpl 'tokens') @{ icon = (Ico 'tokens'); used = (FmtTok $usedTok2); window = (FmtTok $win2) }
        $k = Seg $kTxt '38;5;117'
        if ($k) { $parts.Add($k) }
    }
}
"#;

/// duration 段：会话累计时长（claude cost 段；无则省略）。
const SEG_DURATION: &str = r#"
# ── 会话累计：󰅐 时长（claude cost 段；无则省略。成本数字对网关计价不准，不展示）──
if ($d.cost) {
    if ($null -ne $d.cost.total_duration_ms -and [double]$d.cost.total_duration_ms -ge 1000) {
        $durTxt = ApplyFmt (Tmpl 'duration') @{ icon = (Ico 'duration'); duration = (FmtDur ([double]$d.cost.total_duration_ms)) }
        $dur = Seg $durTxt '38;5;245'
        if ($dur) { $parts.Add($dur) }
    }
}
"#;

/// git 段：分支与状态旗标 [!?]（starship 符号语义，porcelain 单次调用）。
const SEG_GIT: &str = r#"
# ── Git：分支  + 状态旗标 [!?]（starship 符号语义，porcelain 单次调用）──
$branch = $null
if ($d.worktree -and $d.worktree.branch) { $branch = "$($d.worktree.branch)" }
if (-not $branch -and $d.workspace -and $d.workspace.branch) { $branch = "$($d.workspace.branch)" }
if (-not $branch -and $d.workspace -and $d.workspace.git_worktree -and $d.workspace.git_worktree.name) {
    $branch = "$($d.workspace.git_worktree.name)"
}
$flags = ''
$aheadBehind = ''
$gs = & git status -b --porcelain=v1 2>$null
if (-not $branch -and $gs) {
    $hdr = ($gs | Where-Object { $_ -like '## *' } | Select-Object -First 1)
    if ($hdr -and $hdr -match '^##\s+([^\s.^]+)') { $branch = $Matches[1] }
}
if ($gs) {
    $conflicted = $staged = $modified = $untracked = $deleted = $renamed = $false
    foreach ($l in $gs) {
        if ($l -like '## *') {
            if ($l -match 'ahead (\d+)') {
                $n = [int]$Matches[1]
                $aheadBehind += if ($nerd) { [string][char]0x21E1 * $n } else { "+$n" }
            }
            if ($l -match 'behind (\d+)') {
                $n = [int]$Matches[1]
                $aheadBehind += if ($nerd) { [string][char]0x21E3 * $n } else { "-$n" }
            }
            continue
        }
        if ($l.Length -lt 2) { continue }
        $x = $l[0]; $y = $l[1]
        if ($x -eq '?') { $untracked = $true; continue }
        if ($x -eq 'U' -or $y -eq 'U' -or ($x -eq 'A' -and $y -eq 'A') -or ($x -eq 'D' -and $y -eq 'D')) { $conflicted = $true; continue }
        if ($x -ne ' ' -and $x -ne '?') { $staged = $true }
        if ($y -eq 'M' -or $x -eq 'M') { $modified = $true }
        if ($y -eq 'D') { $deleted = $true }
        if ($x -eq 'R' -or $y -eq 'R') { $renamed = $true }
    }
    $f = ''
    if ($conflicted) { $f += '=' }
    if ($deleted) { $f += if ($nerd) { [string][char]0x2718 } else { 'x' } }
    if ($renamed) { $f += if ($nerd) { [string][char]0x00BB } else { '>' } }
    if ($modified) { $f += '!' }
    if ($staged) { $f += '+' }
    if ($untracked) { $f += '?' }
    $flags = $f + $aheadBehind
}
if ($branch -or $flags) {
    $branchTxt = if ($branch) { " $branch" } else { '' }
    $flagTxt = if ($flags) { " [$flags]" } else { '' }
    $g = Seg (ApplyFmt (Tmpl 'git') @{ branch = $branchTxt; flags = $flagTxt }) '38;5;176'
    if ($g) { $parts.Add($g) }
}
"#;

/// 状态栏的PROBE面（细则见模块文档与集成测试）。
/// D11 first-match 序 rust / node / python 先于 zig / go / cpp）。段序含
/// package 或任一工具链段才拼入（文件读加 git 子进程，无人消费时省掉）。
const PS1_PROBE: &str = r#"
# ── 包版本 󰏗 vN.N.N（Cargo.toml / package.json，就近向上找）──
$projDir = if ($d.workspace -and $d.workspace.current_dir) { "$($d.workspace.current_dir)" } else { "$(Get-Location)" }
$probe = $projDir
$pkgVer = $null
$projKind = $null
for ($i = 0; $i -lt 4 -and $probe; $i++) {
    if (Test-Path (Join-Path $probe 'Cargo.toml')) {
        $v = ((& git -C $probe config -f Cargo.toml --get package.version 2>$null) | Out-String).Trim()
        if (-not $v) {
            foreach ($ln in Get-Content (Join-Path $probe 'Cargo.toml')) {
                if ($ln -match '^\s*version\s*=\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
        }
        if ($v) { $pkgVer = "v$v" }
        $projKind = 'rust'
        break
    }
    if (Test-Path (Join-Path $probe 'package.json')) {
        try {
            $pj = Get-Content -Raw (Join-Path $probe 'package.json') | ConvertFrom-Json
            if ($pj.version) { $pkgVer = "v$($pj.version)" }
        } catch {}
        $projKind = 'node'
        break
    }
    if ((Test-Path (Join-Path $probe 'pyproject.toml')) -or
        (Test-Path (Join-Path $probe 'uv.lock')) -or
        (Test-Path (Join-Path $probe 'requirements.txt'))) {
        foreach ($ln in Get-Content (Join-Path $probe 'pyproject.toml') -ErrorAction SilentlyContinue) {
            if ($ln -match '^\s*version\s*=\s*"([^"]+)"') { $pkgVer = "v$($Matches[1])"; break }
        }
        $projKind = 'python'
        break
    }
    if (Test-Path (Join-Path $probe 'build.zig')) {
        $v = $null
        $zon = Join-Path $probe 'build.zig.zon'
        if (Test-Path $zon) {
            foreach ($ln in Get-Content $zon -ErrorAction SilentlyContinue) {
                if ($ln -match '\.version\s*=\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
            if ($v) { $pkgVer = "v$v" }
        }
        $projKind = 'zig'
        break
    }
    if (Test-Path (Join-Path $probe 'go.mod')) {
        $projKind = 'go'
        break
    }
    if ((Test-Path (Join-Path $probe 'CMakeLists.txt')) -or
        (Test-Path (Join-Path $probe 'meson.build'))) {
        $v = $null
        foreach ($ln in Get-Content (Join-Path $probe 'CMakeLists.txt') -ErrorAction SilentlyContinue) {
            if ($ln -match '(?i)project\s*\([^)]*VERSION\s+([\d.]+)') { $v = $Matches[1]; break }
        }
        if (-not $v) {
            foreach ($ln in Get-Content (Join-Path $probe 'meson.build') -ErrorAction SilentlyContinue) {
                if ($ln -match "version\s*:\s*'([^']+)'") { $v = $Matches[1]; break }
                if (-not $v -and $ln -match 'version\s*:\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
        }
        if ($v) { $pkgVer = "v$v" }
        $projKind = 'cpp'
        break
    }
    $parent = Split-Path -Parent $probe
    if ($parent -eq $probe) { break }
    $probe = $parent
}
"#;

/// package 段：包版本渲染（文本来自 PROBE）。
const SEG_PACKAGE: &str = r#"
if ($pkgVer) {
    $pk = Seg (ApplyFmt (Tmpl 'package') @{ icon = (Ico 'package'); version = $pkgVer }) '38;5;208'
    if ($pk) { $parts.Add($pk) }
}
"#;

/// python 段：Python 工具链（Grok ASCII 路径跳过工具链子进程，M046）。
const SEG_PYTHON: &str = r#"
# ── Python 工具链 󰌠 vN.N.N（pyproject/uv.lock/requirements 项目）──
# Grok TUI 跳过工具链子进程：慢且图标会豆腐（M046）。
if ($nerd -and $projKind -eq 'python') {
    $pv = (& python --version 2>$null | Out-String).Trim()
    if ($pv -match 'Python\s+([\d.]+)') {
        $py = Seg (ApplyFmt (Tmpl 'python') @{ icon = (Ico 'python'); version = "v$($Matches[1])" }) '38;5;143'
        if ($py) { $parts.Add($py) }
    }
}
"#;

/// rust 段：Rust 工具链（projKind 判型才探测）。
const SEG_RUST: &str = r#"
# ── Rust 工具链 󱘗 vN.N.N（Cargo.toml 项目才探测——projKind 判，不再
#    「有包版本就探测」：TS 项目曾因此误出 rust 段）──
if ($nerd -and $projKind -eq 'rust') {
    $rv = (& rustc --version 2>$null | Out-String).Trim()
    if ($rv -match 'rustc\s+([\d.]+)') {
        $r = Seg (ApplyFmt (Tmpl 'rust') @{ icon = (Ico 'rust'); version = "v$($Matches[1])" }) '38;5;180'
        if ($r) { $parts.Add($r) }
    }
}
"#;

/// node 段：Node/TS 工具链（TS 就绪再叠 ts 版本，不起 tsc 子进程）。
const SEG_NODE: &str = r#"
# ── Node/TS 工具链 󰎙 vN.N.N（package.json 项目；TS 就绪再叠 󰛦 vM.M.M，
#    typescript 版本就近读 node_modules 不起 tsc 子进程）──
if ($nerd -and $projKind -eq 'node') {
    $nv = (& node --version 2>$null | Out-String).Trim()
    if ($nv -match 'v?([\d.]+)') {
        $n = Seg (ApplyFmt (Tmpl 'node') @{ icon = (Ico 'node'); version = "v$($Matches[1])" }) '38;5;078'
        if ($n) { $parts.Add($n) }
    }
    $tsProbe = $projDir
    for ($j = 0; $j -lt 4 -and $tsProbe; $j++) {
        $tsPj = Join-Path $tsProbe 'node_modules\typescript\package.json'
        if (Test-Path $tsPj) {
            try {
                $tj = Get-Content -Raw $tsPj | ConvertFrom-Json
                if ($tj.version) {
                    $t = Seg (ApplyFmt (Tmpl 'ts') @{ icon = (Ico 'ts'); version = "v$($tj.version)" }) '38;5;067'
                    if ($t) { $parts.Add($t) }
                }
            } catch {}
            break
        }
        $p2 = Split-Path -Parent $tsProbe
        if ($p2 -eq $tsProbe) { break }
        $tsProbe = $p2
    }
}
"#;

/// zig 段：Zig 工具链 seti-zig U+E6A9（D11）。
const SEG_ZIG: &str = r#"
# ── Zig 工具链 seti-zig U+E6A9（cmap: CaskaydiaCove 与 0xProto 2026-09-07）──
if ($nerd -and $projKind -eq 'zig') {
    $zv = (& zig version 2>$null | Out-String).Trim()
    if ($zv -match '^([\d.]+)') {
        $z = Seg (ApplyFmt (Tmpl 'zig') @{ icon = (Ico 'zig'); version = "v$($Matches[1])" }) '38;5;178'
        if ($z) { $parts.Add($z) }
    }
}
"#;

/// go 段：Go 工具链 seti-go U+E627（D11）。
const SEG_GO: &str = r#"
# ── Go 工具链 seti-go U+E627 ──
if ($nerd -and $projKind -eq 'go') {
    $gv = (& go version 2>$null | Out-String).Trim()
    if ($gv -match 'go([\d.]+)') {
        $goSeg = Seg (ApplyFmt (Tmpl 'go') @{ icon = (Ico 'go'); version = "v$($Matches[1])" }) '38;5;080'
        if ($goSeg) { $parts.Add($goSeg) }
    }
}
"#;

/// cpp 段：C/C++ 工具链 seti-cpp U+E646（c++/g++/clang++，静默失败）。
const SEG_CPP: &str = r#"
# ── C/C++ 工具链 seti-cpp U+E646（c++/g++/clang++，静默失败）──
if ($nerd -and $projKind -eq 'cpp') {
    $cv = (& c++ --version 2>$null | Out-String).Trim()
    if (-not $cv) { $cv = (& g++ --version 2>$null | Out-String).Trim() }
    if (-not $cv) { $cv = (& clang++ --version 2>$null | Out-String).Trim() }
    if ($cv -match '(\d+\.\d+(?:\.\d+)?)') {
        $cx = Seg (ApplyFmt (Tmpl 'cpp') @{ icon = (Ico 'cpp'); version = "v$($Matches[1])" }) '38;5;110'
        if ($cx) { $parts.Add($cx) }
    }
}
"#;

/// 状态栏的TAIL面（细则见模块文档与集成测试）。
const PS1_TAIL: &str = "\nWrite-Output ($parts -join ' | ')\nexit 0\n";

/// 排间断点（D40 双排，D42 三行泛化）：第 n 排段块后收线并重置收集器。
fn ps1_rowsplit(n: usize) -> String {
    format!(
        "\n$slRow{n} = ($parts -join ' | ')\n$parts = [System.Collections.Generic.List[string]]::new()\n"
    )
}

/// 多行尾（D42 三行版）：末排收线后按 agent 出行——kimi 只取首行（S025
/// 源码实证）加 grok 多行未实证，运行时自动并单行（保包版本与工具链段
/// 可见性，codex review F2）；空排不出空行（F3）。
fn ps1_tail_multi(last_row: usize) -> String {
    let mut rows = String::new();
    for n in 1..=last_row {
        rows.push_str(&format!("$slRow{n}, "));
    }
    let rows = rows.trim_end_matches(", ");
    format!(
        "\n$slRow{last_row} = ($parts -join ' | ')\n$slRows = @({rows}) | Where-Object {{ $_ }}\nif ($AgentName -eq 'kimi' -or $AgentName -eq 'grok') {{\n    Write-Output ($slRows -join ' | ')\n}} else {{\n    $slRows | ForEach-Object {{ Write-Output $_ }}\n}}\nexit 0\n"
    )
}

/// 段 id 到脚本块查表。未知 id 报错：拼装无法命中段块；oma 纪元旧段名
/// （D45 更名 hst）带改名 CTA。
fn segment_block(id: &str) -> Result<&'static str, String> {
    if id == "oma" {
        return Err(
            "unknown statusline segment: oma（D45 起段名 oma 更名 hst，请把配置里的 \"oma\" 改为 \"hst\"）"
                .to_string(),
        );
    }
    Ok(match id {
        "shell" => SEG_SHELL,
        "dir" => SEG_DIR,
        "hst" => SEG_OMA,
        "model" => SEG_MODEL,
        "context" => SEG_CONTEXT,
        "tools" => SEG_TOOLS,
        "mcp" => SEG_MCP,
        "tokens" => SEG_TOKENS,
        "duration" => SEG_DURATION,
        "git" => SEG_GIT,
        "clock" => SEG_CLOCK,
        "package" => SEG_PACKAGE,
        "python" => SEG_PYTHON,
        "rust" => SEG_RUST,
        "node" => SEG_NODE,
        "zig" => SEG_ZIG,
        "go" => SEG_GO,
        "cpp" => SEG_CPP,
        _ => return Err(format!("unknown statusline segment: {id}")),
    })
}

/// 默认第一行「项目状态」（D44 用户四令排版）：shell / cwd / git 分支 /
/// 包版本与工具链尾巴（环境与项目同线，D18/D42 的 shell 领首惯例回归）。
/// D18 定制面 `segments` 键缺省回落此序。
pub(crate) const DEFAULT_SEGMENTS: &[&str] = &[
    "shell", "dir", "git", "package", "python", "rust", "node", "zig", "go", "cpp", "clock",
];

/// 默认第二行「agent 状态」（D43 精修、D45 段更名 hst）：agent 态 / 模型
/// / context 百分比加 token 绝对值（`46% [449k/977k]` 形，构成 mix 退位）
/// / 耗时。`segments2` 键缺省回落此序。
pub(crate) const DEFAULT_SEGMENTS2: &[&str] = &["hst", "model", "context", "duration"];

/// 默认第三行（D44 用户令「去掉第三行」）：默认空 = 两行布局；tools /
/// mcp 计数与 token 用量三段同退默认位，显式写 `segments3` 才有第三行。
pub(crate) const DEFAULT_SEGMENTS3: &[&str] = &[];

/// 内嵌默认模板（D18）。键 = 段 id；`context-ascii` 是 grok 的结构差异项
/// （nerd 版带 used/window 括号对，ascii 版只有百分比加 ctx 后缀）。
/// 各段可用占位符见 `hst statusline --example`；git 段默认前导空格在 branch / flags 变量里。
const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    ("shell", "{icon}{name}"),
    ("dir", "{path}"),
    ("hst", "{icon}{agent}:{state}"),
    ("model", "{icon}{model}"),
    ("context", "{icon}{pct}% [{used}/{window}]"),
    ("context-ascii", "{pct}% [{used}/{window}]"),
    ("tools", "{icon}{count}"),
    ("tools-ascii", "{count}"),
    ("mcp", "{icon}{count}"),
    ("mcp-ascii", "{count}"),
    ("tokens", "{icon}{used}/{window}"),
    ("tokens-ascii", "{used}/{window}"),
    ("duration", "{icon}{duration}"),
    ("git", "{branch}{flags}"),
    ("clock", "{icon}{datetime}"),
    ("package", "{icon}{version}"),
    ("python", "{icon}{version}"),
    ("rust", "{icon}{version}"),
    ("node", "{icon}{version}"),
    ("ts", "{icon}{version}"),
    ("zig", "{icon}{version}"),
    ("go", "{icon}{version}"),
    ("cpp", "{icon}{version}"),
];

/// 内嵌默认图标（码位与拆段前脚本逐字对齐；hst 机器人宽字形跟两空格，
/// D45 前键名 oma）。
/// Grok ASCII 路径图标恒空串（M046）。
const DEFAULT_ICONS: &[(&str, &str)] = &[
    ("shell-pwsh", "\u{ebc7} "),
    ("shell", "\u{ea85} "),
    ("hst", "\u{f06a9}  "),
    ("model", "\u{2726} "),
    ("context", "\u{f035b} "),
    ("tools", "\u{f0ad} "),
    ("mcp", "\u{f233} "),
    ("tokens", "\u{f080} "),
    ("duration", "\u{f0150} "),
    ("package", "\u{f03d7} "),
    ("python", "\u{f0320} "),
    ("rust", "\u{f1617} "),
    ("node", "\u{f0399} "),
    ("ts", "\u{f06e6} "),
    ("zig", "\u{e6a9} "),
    ("go", "\u{e627} "),
    ("cpp", "\u{e646} "),
    ("clock", "\u{f0954} "),
];

/// 配置块的取用与占位替换助手（随烘焙块注入，紧跟 HEAD）。
const PS1_CFG_HELPERS: &str = r#"
function Tmpl([string]$k) {
    if (-not $nerd -and $slTmpl.ContainsKey("$k-ascii")) { return "$($slTmpl["$k-ascii"])" }
    return "$($slTmpl[$k])"
}
function Ico([string]$k) {
    if ($nerd -and $slIcon.ContainsKey($k)) { return "$($slIcon[$k])" }
    return ''
}
function ApplyFmt([string]$fmt, [hashtable]$vars) {
    foreach ($k in @($vars.Keys)) { $fmt = $fmt.Replace(('{' + $k + '}'), [string]$vars[$k]) }
    return $fmt
}
"#;

/// ps1 单引号字面量（内嵌单引号加倍；用户值无法越出字面量）。
fn ps1_sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn lookup_override<'a>(user: &'a [(String, String)], key: &str) -> Option<&'a str> {
    user.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// 状态栏的烘焙定制块面（细则见模块文档与集成测试）。
/// 默认全键在场）。值经单引号转义，用户串无法越出字面量（模板注入不成立）。
fn render_cfg_block(cfg: &StatuslineConfig) -> String {
    let mut out = String::from(
        "\n# ── D18 定制烘焙：模板与图标（~/.hst/statusline.toml 键级回落内嵌默认）──\n$slTmpl = @{\n",
    );
    for (k, v) in DEFAULT_TEMPLATES {
        let merged = lookup_override(&cfg.template, k).unwrap_or(v);
        out.push_str(&format!("    {} = {}\n", ps1_sq(k), ps1_sq(merged)));
    }
    out.push_str("}\n$slIcon = @{\n");
    for (k, v) in DEFAULT_ICONS {
        let merged = lookup_override(&cfg.icons, k).unwrap_or(v);
        out.push_str(&format!("    {} = {}\n", ps1_sq(k), ps1_sq(merged)));
    }
    out.push_str("}\n");
    out.push_str(PS1_CFG_HELPERS);
    out
}

/// 按段序拼装状态栏脚本（D42 三行分组、D43 五点精修）：HEAD 加烘焙定制块
/// 加（按需）COMMON / CTXPROBE / PROBE 加逐行段块与排间断点加多行尾。
/// COMMON 在任一行含 dir / hst / mcp 时拼入（rev-parse 与目录消费）；
/// CTXPROBE 在含 tools 段、或 context 段且生效模板（nerd 与 ascii 任一）
/// 显式含 {mix} 时拼入（D43：{mix} 退出缺省模板，纯「百分比加绝对值」
/// 配置无消费者不白跑 transcript 尾段解析）；PROBE 在含 package 或任一
/// 工具链段时拼入。跨行重复段 id、未知段 id、未知模板
/// 或图标键报错；`single_line = true` 时全行并一（kimi / grok 运行时也
/// 自动并一）；空行不出空行；全空产出空栏（用户显式所为）。
pub(crate) fn assemble_statusline_ps1(
    rows: &[&[&str]],
    cfg: &StatuslineConfig,
) -> Result<String, String> {
    for (k, _) in &cfg.template {
        if !DEFAULT_TEMPLATES.iter().any(|(dk, _)| *dk == k) {
            return Err(format!("unknown statusline template key: {k}"));
        }
    }
    for (k, _) in &cfg.icons {
        if !DEFAULT_ICONS.iter().any(|(dk, _)| *dk == k) {
            return Err(format!("unknown statusline icon key: {k}"));
        }
    }
    // D44 空行剔除（默认第三行为空 = 干净两行脚本，不出残余收线）；
    // 单行 / 多行判定在剔除后做。
    let rows_owned: Vec<&[&str]> = rows.iter().copied().filter(|r| !r.is_empty()).collect();
    let rows: &[&[&str]] = &rows_owned;
    // 多行态 = 非 single_line 且至少两行非空；单行态全段并一行。
    let nonempty_rows = rows.len();
    let multi = !cfg.single_line && nonempty_rows >= 2;
    let all: Vec<&str> = rows.iter().flat_map(|r| r.iter().copied()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut out = String::with_capacity(16 * 1024);
    out.push_str(PS1_HEAD);
    out.push_str(&render_cfg_block(cfg));
    if all.iter().any(|id| matches!(*id, "dir" | "hst" | "mcp")) {
        out.push_str(PS1_COMMON);
    }
    // D43 CTXPROBE 门控（codex F1）：tools 计数恒是消费者；{mix} 只有在
    // context 段在场且生效模板显式含它时才消费（缺省模板已不含）。
    let tmpl_of = |key: &str| {
        cfg.template
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| {
                DEFAULT_TEMPLATES
                    .iter()
                    .find(|(dk, _)| *dk == key)
                    .map(|(_, v)| *v)
                    .unwrap_or("")
            })
    };
    let mix_used =
        tmpl_of("context").contains("{mix}") || tmpl_of("context-ascii").contains("{mix}");
    if all.iter().any(|id| *id == "tools") || (all.iter().any(|id| *id == "context") && mix_used) {
        out.push_str(PS1_CTXPROBE);
    }
    if all.iter().any(|id| {
        matches!(
            *id,
            "package" | "python" | "rust" | "node" | "zig" | "go" | "cpp"
        )
    }) {
        out.push_str(PS1_PROBE);
    }
    for (i, row) in rows.iter().enumerate() {
        for id in *row {
            if !seen.insert(id) {
                return Err(format!("duplicate statusline segment: {id}"));
            }
            out.push_str(segment_block(id)?);
        }
        // 多行态：每行段块后收线重置（末行由 tail 收）。
        if multi && i + 1 < rows.len() {
            out.push_str(&ps1_rowsplit(i + 1));
        }
    }
    if multi {
        out.push_str(&ps1_tail_multi(rows.len()));
    } else {
        out.push_str(PS1_TAIL);
    }
    Ok(out)
}

/// 状态栏的默认脚本面（细则见模块文档与集成测试）。
pub(crate) fn default_statusline_ps1() -> String {
    assemble_statusline_ps1(
        &[DEFAULT_SEGMENTS, DEFAULT_SEGMENTS2, DEFAULT_SEGMENTS3],
        &StatuslineConfig::default(),
    )
    .expect("default segment order is valid")
}

/// `~/.hst/statusline.toml` 用户级定制（D18）。键级缺省回落内嵌默认：
/// 没写的键用默认，写下的键生效；坏文件硬错退出 1。
#[derive(Debug, Default, PartialEq)]
pub struct StatuslineConfig {
    /// `segments`：第一排段 id 数组即全量序（显隐加顺序）；键缺省回落
    /// `DEFAULT_SEGMENTS`。
    pub segments: Option<Vec<String>>,
    /// `segments2`（D40）：第二行（agent 状态）段 id 数组；键缺省回落
    /// `DEFAULT_SEGMENTS2`，空数组 = 不出第二行。
    pub segments2: Option<Vec<String>>,
    /// `segments3`（D42）：第三行（运行时状态）段 id 数组；键缺省回落
    /// `DEFAULT_SEGMENTS3`，空数组 = 不出第三行。
    pub segments3: Option<Vec<String>>,
    /// `single_line`（D40）：退单排开关（两排段并一行；默认 false 双排）。
    /// kimi / grok 的多行渲染未实证时的逃生门。
    pub single_line: bool,
    /// `[template]`：段格式串（键 = 段 id；`<段>-ascii` 为 grok 结构差异项）；
    /// 键级回落 `DEFAULT_TEMPLATES`。
    pub template: Vec<(String, String)>,
    /// `[icons]`：图标映射（含 `ts`、`shell-pwsh` 子项键）；键级回落
    /// `DEFAULT_ICONS`。Grok ASCII 路径图标恒空（M046）。
    pub icons: Vec<(String, String)>,
    /// `[codex] items`：codex 内置项 ID 子集（原样透传，未知 id codex 侧
    /// 静默跳过）；键缺省回落 `CODEX_STATUS_LINE_ITEMS`。
    pub codex_items: Option<Vec<String>>,
}

pub(crate) fn config_path(home: &Path) -> PathBuf {
    home.join("statusline.toml")
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 读用户定制配置；文件不存在回落全默认（不是错误）。
pub fn read_config(home: &Path) -> Result<StatuslineConfig, String> {
    let p = config_path(home);
    if !p.exists() {
        return Ok(StatuslineConfig::default());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse_config(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// 状态栏的纯函数面（细则见模块文档与集成测试）。
fn parse_config(text: &str) -> Result<StatuslineConfig, String> {
    let v: toml::Value = toml::from_str(text).map_err(|e| format!("parse: {e}"))?;
    let mut cfg = StatuslineConfig::default();
    if let Some(segs) = v.get("segments") {
        let arr = segs.as_array().ok_or("segments 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments 元素必须是字符串")?.to_string());
        }
        cfg.segments = Some(out);
    }
    if let Some(segs) = v.get("segments2") {
        let arr = segs.as_array().ok_or("segments2 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments2 元素必须是字符串")?.to_string());
        }
        cfg.segments2 = Some(out);
    }
    if let Some(segs) = v.get("segments3") {
        let arr = segs.as_array().ok_or("segments3 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments3 元素必须是字符串")?.to_string());
        }
        cfg.segments3 = Some(out);
    }
    if let Some(sl) = v.get("single_line") {
        cfg.single_line = sl.as_bool().ok_or("single_line 必须是布尔值")?;
    }
    for (key, slot) in [("template", &mut cfg.template), ("icons", &mut cfg.icons)] {
        if let Some(t) = v.get(key) {
            let t = t.as_table().ok_or_else(|| format!("{key} 必须是表"))?;
            for (k, val) in t {
                let s = val
                    .as_str()
                    .ok_or_else(|| format!("{key}.{k} 必须是字符串"))?;
                slot.push((k.clone(), s.to_string()));
            }
        }
    }
    if let Some(items) = v.get("codex").and_then(|c| c.get("items")) {
        let arr = items.as_array().ok_or("[codex] items 必须是字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(
                s.as_str()
                    .ok_or("[codex] items 元素必须是字符串")?
                    .to_string(),
            );
        }
        cfg.codex_items = Some(out);
    }
    Ok(cfg)
}

/// 段序生效值（D42 三行、D44 默认两行）：各行取用户清单或默认序（未知与
/// 跨行重复 id 由拼装器拒，空行由拼装器剔除）。**老配置原样升级语义**
/// （codex D42 评审 F1/H1 两轮收口）：用户显式写过 `segments` 而未写任何
/// 后续行键时，未写的行**不补默认**（D18 老单排与 v1.1.0 双排两种老形态
/// 原样升级，行为零漂移；新装用户键全缺省得两行默认）；用户写过任一后续
/// 行键（segments2 或 segments3）时，未写的行**补默认并去重**（显式段
/// id 从默认行剔除，防跨行重复硬错）。显式写的行不去重（跨行重复仍由拼
/// 装器拒）。
fn effective_orders(cfg: &StatuslineConfig) -> Result<Vec<Vec<&str>>, String> {
    let rows = [
        (&cfg.segments, DEFAULT_SEGMENTS),
        (&cfg.segments2, DEFAULT_SEGMENTS2),
        (&cfg.segments3, DEFAULT_SEGMENTS3),
    ];
    let any_later_row_written = cfg.segments2.is_some() || cfg.segments3.is_some();
    // 老配置形态 = 用户写过 segments 且没写任何后续行键 → 后续缺省行不补
    // （原样单行升级）。新装（segments 也缺省）与写过后续键的配置照常补
    // 默认（去重）。
    let legacy_single_row =
        cfg.segments.as_ref().is_some_and(|v| !v.is_empty()) && !any_later_row_written;
    // 用户显式写过的段 id 全集：补默认的行剔除这些 id。
    let user_ids: std::collections::HashSet<&str> = rows
        .iter()
        .filter_map(|(user, _)| user.as_ref())
        .flat_map(|segs| segs.iter().map(String::as_str))
        .collect();
    Ok(rows
        .iter()
        .enumerate()
        .map(|(i, (user, default))| match user {
            Some(segs) => segs.iter().map(String::as_str).collect(),
            None if i > 0 && legacy_single_row => Vec::new(),
            None => default
                .iter()
                .copied()
                .filter(|id| !user_ids.contains(id))
                .collect(),
        })
        .filter(|row| !row.is_empty())
        .collect())
}

pub(crate) fn script_path(home: &Path) -> PathBuf {
    home.join("statusline").join("hst-statusline.ps1")
}

pub(crate) fn grok_cmd_path(home: &Path) -> PathBuf {
    home.join("statusline").join("hst-statusline-grok.cmd")
}

/// Windows Grok `[ui.status_line].command` must be a single spawnable path.
/// grok-build `command.rs` does `Command::new(entire_string)` first and only
/// falls back to a shell on `NotFound` (or Unix ENOEXEC). A `pwsh -File "..."`
/// line contains quotes and slashes, so Windows returns ERROR_INVALID_NAME
/// 123 and paints `[status line: could not start the script: ...]` (M048).
const STATUSLINE_GROK_CMD: &str =
    "@echo off\r\npwsh -NoProfile -File \"%~dp0hst-statusline.ps1\" grok\r\n";

fn grok_command_line(script_str: &str) -> String {
    #[cfg(windows)]
    {
        match script_str.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/hst-statusline-grok.cmd"),
            None => "hst-statusline-grok.cmd".into(),
        }
    }
    #[cfg(not(windows))]
    {
        format!("pwsh -NoProfile -File \"{script_str}\" grok")
    }
}

/// pwsh is the statusline runtime on every platform. Advisory only: the
/// script is deployed regardless; without pwsh the bar simply won't render
/// in that environment.
pub fn pwsh_on_path() -> bool {
    crate::pathutil::find_on_path("pwsh").is_some()
}

/// 自备脚本标记（D18 整脚本替换）：`<部署脚本>.custom`，内容为源路径。
/// 在场时 `deploy_script` 不覆写内嵌拼装产物。
pub(crate) fn marker_path(home: &Path) -> PathBuf {
    let mut s = script_path(home).into_os_string();
    s.push(".custom");
    PathBuf::from(s)
}

/// 自备脚本当前是否在场（kv 面 statusline.custom 用）。
pub fn custom_active(home: &Path) -> bool {
    marker_path(home).exists()
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 释放状态栏脚本（幂等覆写）。按 `~/.hst/statusline.toml` 生成时烘焙：
/// segments 键控段序与显隐，缺省回落内嵌默认（D18）。自备脚本标记在场时
/// 跳过覆写（只保 grok .cmd 壳，见 D18 整脚本替换）。
pub fn deploy_script(home: &Path) -> Result<PathBuf, String> {
    let p = script_path(home);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if !marker_path(home).exists() {
        let cfg = read_config(home)?;
        let orders = effective_orders(&cfg)?;
        let orders_ref: Vec<&[&str]> = orders.iter().map(|r| r.as_slice()).collect();
        let script = assemble_statusline_ps1(&orders_ref, &cfg).map_err(|e| {
            // 段清单来自用户配置时，错误带上文件出处才可操作。
            if cfg.segments.is_some() || cfg.segments2.is_some() || cfg.segments3.is_some() {
                format!("{}: {e}", config_path(home).display())
            } else {
                e
            }
        })?;
        // D53：内容判等幂等（init 全套并入后重跑不搅 mtime；只在内容
        // 变化时落盘）。
        if std::fs::read_to_string(&p).ok().as_deref() != Some(script.as_str()) {
            std::fs::write(&p, script).map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    let cmd = grok_cmd_path(home);
    if std::fs::read_to_string(&cmd).ok().as_deref() != Some(STATUSLINE_GROK_CMD) {
        std::fs::write(&cmd, STATUSLINE_GROK_CMD).map_err(|e| format!("{}: {e}", cmd.display()))?;
    }
    Ok(p)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 部署用户自备脚本（D18 整脚本替换）：拷到部署位，agent 配置命令行不动
/// （claude / kimi / grok 调用约定不变：首参 agent 名，stdin 喂 agent JSON，
/// stdout 单行状态栏）；marker 记源路径，此后无 `--script` 的重跑跳过内嵌
/// 覆盖。codex 无脚本面（M045），对 codex 只有 `[codex] items` 生效。
pub fn deploy_custom_script(home: &Path, src: &Path) -> Result<PathBuf, String> {
    let text =
        std::fs::read_to_string(src).map_err(|e| format!("--script {}: {e}", src.display()))?;
    if text.trim().is_empty() {
        return Err(format!("--script {}: empty script", src.display()));
    }
    let p = script_path(home);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    std::fs::write(marker_path(home), format!("{}\n", src.display()))
        .map_err(|e| format!("{}: {e}", marker_path(home).display()))?;
    Ok(p)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的还原内嵌脚本面（细则见模块文档与集成测试）。
pub fn restore_builtin_script(home: &Path) -> Result<PathBuf, String> {
    let m = marker_path(home);
    if m.exists() {
        std::fs::remove_file(&m).map_err(|e| format!("{}: {e}", m.display()))?;
    }
    deploy_script(home)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的claude面（细则见模块文档与集成测试）。
/// D53（codex F1）：user_home 显式透传（测试密闭，不读 env）。
pub fn merge_claude(home: &Path, user_home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let settings = user_home.join(".claude").join("settings.json");
    let mut v: serde_json::Value = if settings.exists() {
        let text = std::fs::read_to_string(&settings)
            .map_err(|e| format!("{}: {e}", settings.display()))?;
        // D52（codex F1）：BOM 容忍（与 yolo::read_json 同类，舰队 PS 脚本）。
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{}: corrupt: {e}", settings.display()))?
    } else {
        json!({})
    };
    let cmd = format!(
        "pwsh -NoProfile -File \"{}\" claude",
        script.display().to_string().replace('\\', "/")
    );
    v["statusLine"] = json!({ "type": "command", "command": cmd });
    let body = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n";
    // D53（codex F2）：内容判等幂等（init 重跑不搅 mtime）。
    if std::fs::read_to_string(&settings).ok().as_deref() != Some(body.as_str()) {
        std::fs::write(&settings, body).map_err(|e| format!("{}: {e}", settings.display()))?;
    }
    Ok(settings.display().to_string())
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的kimi面（细则见模块文档与集成测试）。
/// cmd/sh 执行，首行接管 footer；300ms 超时由 kimi 侧约束，超时自动回退
/// 内置布局——S025）。其它表保留。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_kimi(home: &Path, user_home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let config = user_home.join(".kimi-code").join("tui.toml");
    let script_str = script.display().to_string().replace('\\', "/");
    let mut toml = read_toml(&config)?;
    if apply_kimi_status_line(&mut toml, &script_str)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[status_line].command` 幂等落位；返回是否变更（可测纯函数）。
fn apply_kimi_status_line(toml: &mut toml::Value, script_str: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi tui.toml is not a table".into()),
    };
    let status_line = table
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("kimi [status_line] is not a table".into()),
    };
    let command = format!("pwsh -NoProfile -File \"{script_str}\" kimi");
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str());
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
    }
    Ok(changed)
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// 状态栏的grok面（细则见模块文档与集成测试）。
/// Windows 写 `.cmd` 单路径（M048）；Unix 仍写 `pwsh -File` 命令行（NotFound
/// 才回落 sh -c）。其它表保留。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_grok(home: &Path, user_home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let config = user_home.join(".grok").join("config.toml");
    let script_str = script.display().to_string().replace('\\', "/");
    let mut toml = read_toml(&config)?;
    if apply_grok_status_line(&mut toml, &script_str)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[ui.status_line]` 幂等落位（type=command + command 串）；返回是否变更
/// （可测纯函数）。
fn apply_grok_status_line(toml: &mut toml::Value, script_str: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("grok config.toml is not a table".into()),
    };
    let ui = table
        .entry("ui".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let ui = match ui {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui] is not a table".into()),
    };
    let status_line = ui
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui.status_line] is not a table".into()),
    };
    let command = grok_command_line(script_str);
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str())
        || sl.get("type").and_then(|v| v.as_str()) != Some("command");
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
        sl.insert("type".into(), toml::Value::String("command".into()));
    }
    Ok(changed)
}

/// `hst statusline --example` 打印的带注释全量示例（存到
/// `~/.hst/statusline.toml` 生效）。
pub const EXAMPLE_TOML: &str = r#"# ~/.hst/statusline.toml —— 状态栏用户级定制（D18）
# 生成时烘焙：hst statusline 每次运行读本文件重拼脚本后落盘，
# 改完本文件重跑一次 hst statusline 生效。
# 键级缺省回落：没写的键用内嵌默认；坏文件硬错退出 1。

# 段落清单（D44 用户四令后的默认两行）：
# segments = 第一行项目状态（shell / cwd / git 分支 / 包版本与工具链尾巴）、
# segments2 = 第二行 agent 状态（agent 态 / 模型 / context 百分比加 token
# 绝对值 / 耗时），段 id 数组即全量（显隐加顺序）。
# segments3 = 第三行（D44 起默认空 = 无第三行；可用段 id：tools / mcp /
# tokens 等显式选用才出现，如要看工具与 MCP 计数：
#   segments3 = ["tools", "mcp"]
# ）。
# 例（隐藏 shell 与时长段、git 提到目录前）：
#   segments = ["dir", "git"]
#   segments2 = ["hst", "model", "context"]
#   segments3 = []
# 退单行（kimi / grok 运行时自动并一行；显式退单行用）：
#   single_line = true
segments = ["shell", "dir", "git", "package", "python", "rust", "node", "zig", "go", "cpp", "clock"]
segments2 = ["hst", "model", "context", "duration"]
segments3 = []

# 段内模板（[template]）：每段一条格式串；`<段>-ascii` 是 grok 的 ASCII 形
#（缺省同用 nerd 模板、图标恒空）。可用占位符：
#   shell {icon}{name} / dir {path} / hst {icon}{agent}{state}
#   model {icon}{model} / context {icon}{pct}{used}{window}{mix}（mix = 构成
#   占比 [sN tN mN]，transcript 可解析时才有）
#   tools {icon}{count} / mcp {icon}{count} / tokens {icon}{used}{window}
#   duration {icon}{duration} / git {branch}{flags}
#   package 与七工具链段（含 ts）{icon}{version}
#   clock {icon}{datetime}（D51：年月日加当前时间，分钟精度）
# 例（hst 段去图标改方括号态）：
# [template]
# hst = "{agent}[{state}]"

# 图标映射（[icons]）：键级回落；hst 机器人宽字形默认跟两空格。
# 可用键：shell / shell-pwsh / hst / model / context / tools / mcp / tokens
#         / duration / package / python / rust / node / ts / zig / go / cpp
#         / clock（D51 加）
# 例：
# [icons]
# rust = "R "

# codex 内置项子集（[codex] items）：替换写入 ~/.codex/config.toml 的
# [tui].status_line 内置项 ID 清单；未知 id codex 侧静默跳过（S016）。
# codex 只有内置项面（无外部命令 statusline），D40 后缺省集已含 token
# 细分（used / total-input / total-output / window），D46 加 codex-version
# （版本段 codex 侧承载）；tools 与 MCP 计数、context 构成 codex 能力面
# 不可达。
# 例（只要分支与目录）：
# [codex]
# items = ["current-dir", "git-branch"]
"#;

/// Codex `[tui].status_line` is an ordered list of built-in item IDs
/// (ohmypwsh S016, openai/codex 0.148+). Unknown strings are silently
/// skipped, so a command argv (`"command", "pwsh", "-File", ...`) empties
/// the bar. hst cannot inject a custom script here.
/// codex 内置项默认集（D40 对齐增强：codex 只有内置项面，无外部命令
/// statusline（openai/codex#17827/#20244 未实现），可达上限就是富内置项
/// 清单——源码 status_line_setup.rs 全量约 30 项）。token 细分（used/
/// input/output/window）是 codex 侧对 D40「token 用量」要素的承载；
/// tools 计数、MCP 计数、context 构成三要素 codex 能力面不可达（差距
/// 说明见 S034 追记）。D46 加 `codex-version`（源码实证
/// `StatusLineItem::CodexVersion`，strum kebab_case ID），codex 侧版本
/// 显示走内置项，与 pwsh 面版本并入 agent 名同要素；同轮用户裁去
/// `context-remaining`（渲染成 left 百分比，与 `context-used` 的
/// `Context N% used` 重复占宽，只留 used 形，缺省集回十二项）。
const CODEX_STATUS_LINE_ITEMS: &[&str] = &[
    "run-state",
    "codex-version",
    "model-with-reasoning",
    "context-used",
    "used-tokens",
    "total-input-tokens",
    "total-output-tokens",
    "context-window-size",
    "permissions",
    "current-dir",
    "git-branch",
    "branch-changes",
];

fn render_codex_tui_section(items: &[&str]) -> String {
    let mut lines = vec!["[tui]".to_string(), "status_line = [".to_string()];
    let last = items.len().saturating_sub(1);
    for (i, id) in items.iter().enumerate() {
        let comma = if i == last { "" } else { "," };
        lines.push(format!("  \"{id}\"{comma}"));
    }
    lines.push("]".into());
    lines.push("status_line_use_colors = true".into());
    lines.join("\n")
}

fn strip_tui_section(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_tui = false;
    for ln in text.lines() {
        if ln.trim().starts_with('[') {
            in_tui = ln.trim() == "[tui]";
            if in_tui {
                continue;
            }
        }
        if !in_tui {
            lines.push(ln.to_string());
        }
    }
    lines.join("\n")
}

/// # Errors
///
/// 失败返回 `String` 错误（路径与原因；网络与解析类见模块文档）。
/// Codex: replace the `[tui]` table with built-in item IDs (ohmypwsh S016).
/// Does not deploy the pwsh script; Codex has no command-backed status line.
/// `[codex] items`（D18）用户清单原样透传：codex 对未知 id 静默跳过，hst
/// 不校验清单合法性；键缺省回落内嵌推荐八项。
/// D53（codex F1）：user_home 显式透传。
pub fn merge_codex(home: &Path, user_home: &Path) -> Result<String, String> {
    let cfg = read_config(home)?;
    let items: Vec<&str> = match &cfg.codex_items {
        Some(list) => list.iter().map(String::as_str).collect(),
        None => CODEX_STATUS_LINE_ITEMS.to_vec(),
    };
    let config = user_home.join(".codex").join("config.toml");
    let existing = if config.exists() {
        std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?
    } else {
        String::new()
    };
    let kept = strip_tui_section(&existing);
    let kept = kept.trim_end();
    let body = if kept.is_empty() {
        format!("{}\n", render_codex_tui_section(&items))
    } else {
        format!("{kept}\n\n{}\n", render_codex_tui_section(&items))
    };
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // D53（codex F2）：内容判等幂等（init 重跑不搅 mtime）。
    if std::fs::read_to_string(&config).ok().as_deref() != Some(body.as_str()) {
        std::fs::write(&config, body).map_err(|e| format!("{}: {e}", config.display()))?;
    }
    Ok(config.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独占临时目录（单测内 fs 落盘判据用；用完即删）。
    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hst-sl-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_config_reads_segments_and_falls_back_on_missing_key() {
        // 期望值来自 D18 裁定：segments 写下即全量序，键缺省回落默认。
        assert_eq!(parse_config("").unwrap().segments, None);
        assert_eq!(
            parse_config("segments = [\"hst\", \"git\"]\n")
                .unwrap()
                .segments,
            Some(vec!["hst".to_string(), "git".to_string()])
        );
        // segments = [] 是显式空清单（空栏），不回落默认。
        assert_eq!(
            parse_config("segments = []\n").unwrap().segments,
            Some(vec![])
        );
    }

    #[test]
    fn dies_parse_config_rejects_malformed() {
        assert!(parse_config("segments = ").is_err(), "truncated toml");
        assert!(
            parse_config("segments = \"git\"\n").is_err(),
            "not an array"
        );
        assert!(parse_config("segments = [1]\n").is_err(), "not strings");
    }

    #[test]
    fn clock_segment_default_line1_with_full_datetime() {
        // D51：clock 段入默认第一行行尾（工具链尾巴之后恒右置），格式
        // yyyy-MM-dd HH:mm（Get-Date 零子进程，分钟精度）；模板与图标键可定制。
        assert_eq!(DEFAULT_SEGMENTS.last(), Some(&"clock"));
        let ps1 = default_statusline_ps1();
        assert!(
            ps1.contains(r"Get-Date -Format 'yyyy-MM-dd HH\:mm'"),
            "clock uses local Get-Date with minute precision"
        );
        assert!(ps1.contains("{icon}{datetime}"), "clock template default");
        let ck = ps1.find("# ── clock 段").unwrap();
        let cp = ps1.find("# ── Python 工具链").unwrap();
        assert!(ck > cp, "clock block sits after the line-1 tail probes");
    }

    #[test]
    fn deploy_script_honors_segment_order_from_config() {
        let home = scratch("order");
        std::fs::write(
            home.join("statusline.toml"),
            "segments = [\"git\", \"hst\"]\nsegments2 = []\nsegments3 = []\n",
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let g = text.find("# ── Git：").unwrap();
        let o = text.find("# ── hst 段").unwrap();
        assert!(o > g, "git before hst per config");
        assert!(!text.contains("# ── Shell 段"), "shell hidden");
        assert!(text.contains("rev-parse"), "COMMON in for hst");
        assert!(!text.contains("Cargo.toml"), "PROBE gated off");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn deploy_script_without_config_is_default_order() {
        let home = scratch("default");
        let p = deploy_script(&home).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("# ── Shell 段"));
        assert_eq!(
            text,
            default_statusline_ps1(),
            "no config = default assembly"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn dies_deploy_script_rejects_unknown_segment_from_config() {
        let home = scratch("unknown");
        std::fs::write(home.join("statusline.toml"), "segments = [\"nope\"]\n").unwrap();
        let err = deploy_script(&home).unwrap_err();
        assert!(err.contains("unknown statusline segment"), "{err}");
        assert!(
            err.contains("statusline.toml"),
            "error names the config file: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn parse_config_reads_template_and_icons_tables() {
        let cfg = parse_config("[template]\nhst = '[{state}] {agent}'\n\n[icons]\nhst = '>'\n\n")
            .unwrap();
        assert_eq!(
            cfg.template,
            vec![("hst".to_string(), "[{state}] {agent}".to_string())]
        );
        assert_eq!(cfg.icons, vec![("hst".to_string(), ">".to_string())]);
    }

    #[test]
    fn dies_parse_config_rejects_non_string_template_or_icon_value() {
        assert!(parse_config("[template]\nhst = 1\n").is_err());
        assert!(parse_config("[icons]\nhst = true\n").is_err());
        assert!(parse_config("template = \"x\"\n").is_err(), "not a table");
    }

    #[test]
    fn cfg_block_merges_user_over_defaults() {
        let cfg = StatuslineConfig {
            template: vec![("hst".to_string(), "{agent}[{state}]".to_string())],
            icons: vec![("rust".to_string(), "R ".to_string())],
            ..Default::default()
        };
        let block = render_cfg_block(&cfg);
        assert!(
            block.contains("'hst' = '{agent}[{state}]'"),
            "user template wins:\n{block}"
        );
        assert!(block.contains("'rust' = 'R '"), "user icon wins:\n{block}");
        assert!(
            block.contains("'model' = '{icon}{model}'"),
            "untouched defaults survive"
        );
        assert!(
            block.contains("'hst' = '\u{f06a9}  '"),
            "default hst icon keeps the wide-glyph double space"
        );
    }

    #[test]
    fn cfg_values_cannot_escape_single_quote_literals() {
        // 注入判据：用户值内嵌单引号必须加倍，无法越出 ps1 字面量。
        let cfg = StatuslineConfig {
            template: vec![("hst".to_string(), "a'; Remove-Item x; '".to_string())],
            ..Default::default()
        };
        let block = render_cfg_block(&cfg);
        assert!(
            block.contains("'a''; Remove-Item x; '''"),
            "single quotes doubled: {block}"
        );
    }

    #[test]
    fn dies_assemble_rejects_unknown_template_or_icon_key() {
        let cfg = StatuslineConfig {
            template: vec![("nope".to_string(), "x".to_string())],
            ..Default::default()
        };
        let err = assemble_statusline_ps1(
            &[DEFAULT_SEGMENTS, DEFAULT_SEGMENTS2, DEFAULT_SEGMENTS3],
            &cfg,
        )
        .unwrap_err();
        assert!(err.contains("unknown statusline template key"), "{err}");
        let cfg = StatuslineConfig {
            icons: vec![("nope".to_string(), "x".to_string())],
            ..Default::default()
        };
        let err = assemble_statusline_ps1(
            &[DEFAULT_SEGMENTS, DEFAULT_SEGMENTS2, DEFAULT_SEGMENTS3],
            &cfg,
        )
        .unwrap_err();
        assert!(err.contains("unknown statusline icon key"), "{err}");
    }

    #[test]
    fn template_override_changes_rendered_output() {
        // pwsh 闸门 skip（测试分层形态）：无 pwsh 环境不跑行为判据。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("tmpl");
        std::fs::write(
            home.join("statusline.toml"),
            "[template]\nhst = '{agent}[{state}]'\n",
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        // 封闭性：HOME 钉 scratch（D28 用户级读序会看真实 ~/.hst/state，
        // 机器上的活会话状态会污染判据），cwd 同钉（无 git 无项目 state）。
        // payload 带 version（D46）：钉死版本来源，机器装没装真 claude 判据
        // 都确定，顺带钉「版本并入 {agent} 值、用户模板无需新占位符」。
        let stdout = run_statusline(&p, "claude", &home, br#"{"version":"9.9.9"}"#);
        assert!(
            stdout.contains("claude-9.9.9[unknown]"),
            "user template wins: {stdout}"
        );
        assert!(
            !stdout.contains('\u{f06a9}'),
            "custom template drops the icon: {stdout}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn hst_segment_reads_user_session_keyed_state() {
        // pwsh 闸门 skip（测试分层形态）：无 pwsh 环境不跑行为判据。
        if !pwsh_on_path() {
            return;
        }
        // D28 读序：session 键优先于 agent 最新键；最新键 session 不符（别
        // 会话遗留）续找而不是 unknown 断头。
        let home = scratch("sluser");
        let state = home.join(".hst").join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            state.join("claude-s1.json"),
            r#"{"state":"working","session":"s1"}"#,
        )
        .unwrap();
        std::fs::write(
            state.join("claude.json"),
            r#"{"state":"blocked","session":"other"}"#,
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(
            &p,
            "claude",
            &home,
            br#"{"session_id":"s1","version":"9.9.9"}"#,
        );
        assert!(
            out.contains("claude-9.9.9:working"),
            "session-keyed wins: {out}"
        );
        // 键文件缺位时回落 agent 最新键；session 不符的最新键被闸掉。
        let _ = std::fs::remove_file(state.join("claude-s1.json"));
        let out = run_statusline(
            &p,
            "claude",
            &home,
            br#"{"session_id":"s1","version":"9.9.9"}"#,
        );
        assert!(
            out.contains("claude-9.9.9:unknown"),
            "mismatched latest must be gated: {out}"
        );
        // kimi camelCase sessionId 同样命中键路径。
        std::fs::write(
            state.join("kimi-k1.json"),
            r#"{"state":"idle","session":"k1"}"#,
        )
        .unwrap();
        let out = run_statusline(
            &p,
            "kimi",
            &home,
            br#"{"sessionId":"k1","version":"9.9.9"}"#,
        );
        assert!(
            out.contains("kimi-9.9.9:idle"),
            "camelCase sessionId hits keyed path: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// 直跑状态栏脚本的公共小工具（行为测试用）：env 钉临时 HOME、清覆盖
    /// 变量（含 D46 版本探针的定位与缓存 env，保证机器上装没装真 agent
    /// 判据都确定），喂 stdin，收 stdout。
    fn run_statusline(script: &Path, agent: &str, home: &Path, stdin: &[u8]) -> String {
        run_statusline_with(script, agent, home, stdin, &[])
    }

    /// 播种注册面标记（REQ-014 哨兵探针判据）：行为测试默认种上，保住
    /// 「无状态但注册在册 → unknown」的既有断言语义；哨兵行为测试用
    /// run_statusline_opts 关掉播种验 no-hook! 面。
    fn seed_registration(home: &Path, agent: &str) {
        let rel = match agent {
            "claude" => ".claude/settings.json",
            "codex" => ".codex/hooks.json",
            "grok" => ".grok/hooks/ohmyagents-state.json",
            "kimi" => ".kimi-code/config.toml",
            _ => return,
        };
        let p = home.join(rel);
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(&p, "\"command\": \"/x/.hst/hooks/hst-state.sh\"\n");
    }

    /// run_statusline 带额外 env（D46 探针定位 / 缓存隔离通道用）。
    fn run_statusline_with(
        script: &Path,
        agent: &str,
        home: &Path,
        stdin: &[u8],
        extra_env: &[(&str, std::ffi::OsString)],
    ) -> String {
        run_statusline_opts(script, agent, home, stdin, extra_env, true)
    }

    /// run_statusline_with 的免播种变体（哨兵行为测试用）。
    fn run_statusline_opts(
        script: &Path,
        agent: &str,
        home: &Path,
        stdin: &[u8],
        extra_env: &[(&str, std::ffi::OsString)],
        seed_reg: bool,
    ) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if seed_reg {
            seed_registration(home, agent);
        }
        let mut cmd = Command::new("pwsh");
        cmd.arg("-NoProfile")
            .arg("-File")
            .arg(script)
            .arg(agent)
            .current_dir(home)
            .env_remove("HST_AGENT")
            .env_remove("HST_STATE_FILE")
            .env_remove("HST_VER_CACHE_DIR")
            .env_remove("HST_AGENT_PATH")
            .env_remove("HST_CLAUDE_BIN")
            .env_remove("HST_CODEX_BIN")
            .env_remove("HST_GROK_BIN")
            .env_remove("HST_KIMI_BIN");
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        // PowerShell $HOME：Windows 随 USERPROFILE、Unix 随 HOME（S025）。
        if cfg!(windows) {
            cmd.env("USERPROFILE", home);
        } else {
            cmd.env("HOME", home);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn chinese_cwd_survives_stdin_decode() {
        // D28 用户报修：中文目录乱码（D:\团队 显示「缁跨洘」= UTF-8 被 CP936
        // 解码）。判据：stdin 喂带中文 cwd 的 UTF-8 JSON，目录段原样输出。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("slcjk");
        let p = deploy_script(&home).unwrap();
        let bytes = r#"{"session_id":"s1","cwd":"D:\\团队"}"#.as_bytes().to_vec();
        let out = run_statusline(&p, "claude", &home, &bytes);
        assert!(out.contains("团队"), "chinese dir must survive: {out}");
        assert!(!out.contains('缁'), "no GBK mojibake: {out}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn d46_payload_version_normalizes_and_nonnumeric_falls_back() {
        // D46（codex F1/F2）：payload version 与 probe 输出走同一条归一化
        //（`数字.数字` 起头 token）；非数字版本（如 nightly）不显示，回落
        // 旧形 `claude:state`——显示面与 verify 判据同源（S025）。grok 文档
        // 级 payload 可能为带壳串（`1.0.30 (hash)` 形），归一化后只取版本。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("d46norm");
        let p = deploy_script(&home).unwrap();
        // 非数字：不显示版本，旧形保留。agent 用非白名单名（claudex）：探针
        // 白名单只放行四家，非四家不探，机器装没装真 claude 判据都确定。
        let out = run_statusline(&p, "claudex", &home, br#"{"version":"nightly"}"#);
        assert!(
            out.contains("claudex:unknown") && !out.contains("nightly"),
            "non-numeric version falls back to plain marker: {out}"
        );
        // 带壳串：归一化提取裸版本。
        let out = run_statusline(&p, "grok", &home, br#"{"version":"1.0.30 (hash)"}"#);
        assert!(
            out.contains("grok-1.0.30:unknown"),
            "shelled version normalizes to bare token: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn d46_probe_resolves_env_bin_and_caches_atomically() {
        // D46（codex F4/F6/F7）：probe 定位序 HST_<AGENT>_BIN 显式钉优先；
        // HST_VER_CACHE_DIR 覆盖目录；缓存原子落盘（无 .tmp 残留）；键记
        // size 字段（三元组）。假 claude 可执行件钉死判据（不依赖机器装没
        // 装真 agent）。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("d46probe");
        let fake_dir = home.join("fakebin");
        std::fs::create_dir_all(&fake_dir).unwrap();
        let fake = if cfg!(windows) {
            let f = fake_dir.join("claude.cmd");
            std::fs::write(&f, "@echo claude 3.2.1 (fake-build)\r\n").unwrap();
            f
        } else {
            let f = fake_dir.join("claude");
            std::fs::write(&f, "#!/bin/sh\necho 'claude 3.2.1 (fake-build)'\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            f
        };
        let cache = home.join("vcache");
        let envs: Vec<(&str, std::ffi::OsString)> = vec![
            ("HST_CLAUDE_BIN", fake.as_os_str().to_os_string()),
            ("HST_VER_CACHE_DIR", cache.as_os_str().to_os_string()),
        ];
        let p = deploy_script(&home).unwrap();
        let out = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(
            out.contains("claude-3.2.1:unknown"),
            "env-pinned binary is probed and versioned: {out}"
        );
        let ver_file = cache.join("agent-version-claude.json");
        let cached = std::fs::read_to_string(&ver_file).unwrap();
        assert!(
            cached.contains("\"version\":\"3.2.1\"") && cached.contains("\"size\""),
            "cache entry has version and size key: {cached}"
        );
        assert!(
            !cache.join("agent-version-claude.json.tmp").exists(),
            "atomic rename leaves no tmp file"
        );
        // 第二跑：缓存命中路径，同判据稳定（三元组未变不重探）。
        let out2 = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(
            out2.contains("claude-3.2.1:unknown"),
            "cache hit path renders identically: {out2}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn d46_probe_falls_back_to_stderr_for_version() {
        // codex G2：stdout 无版本样串时第二步合流取 stderr（read_version
        // 同口径）。假件只往 stderr 打版本（Windows cmd 1>&2、Unix >&2），
        // 判据 = 仍渲染带版本标记。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("d46stderr");
        let fake_dir = home.join("fakebin");
        std::fs::create_dir_all(&fake_dir).unwrap();
        let fake = if cfg!(windows) {
            let f = fake_dir.join("claude.cmd");
            std::fs::write(&f, "@echo claude 7.7.7 1>&2\r\n").unwrap();
            f
        } else {
            let f = fake_dir.join("claude");
            std::fs::write(&f, "#!/bin/sh\necho 'claude 7.7.7' >&2\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            f
        };
        let cache = home.join("vcache");
        let envs: Vec<(&str, std::ffi::OsString)> = vec![
            ("HST_CLAUDE_BIN", fake.as_os_str().to_os_string()),
            ("HST_VER_CACHE_DIR", cache.as_os_str().to_os_string()),
        ];
        let p = deploy_script(&home).unwrap();
        let out = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(
            out.contains("claude-7.7.7:unknown"),
            "stderr-only version is captured by the fallback step: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn d46_probe_failure_writes_silent_window_entry() {
        // D46（codex F6/F8 三态机）：探测失败记空值加 probed_at；静默窗内
        // 不重探（probed_at 不变）；窗外重探（probed_at 前移）。钉一个存在
        // 但不可执行的假件逼失败路径。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("d46fail");
        let fake_dir = home.join("fakebin");
        std::fs::create_dir_all(&fake_dir).unwrap();
        // .bin 扩展名：不可执行也无文件关联，& 调用必抛（防 Windows 关联
        // 打开真程序），Unix 无执行位同样拒。
        let fake = fake_dir.join("claude-fake.bin");
        std::fs::write(&fake, "not an executable").unwrap();
        let cache = home.join("vcache");
        let envs: Vec<(&str, std::ffi::OsString)> = vec![
            ("HST_CLAUDE_BIN", fake.as_os_str().to_os_string()),
            ("HST_VER_CACHE_DIR", cache.as_os_str().to_os_string()),
        ];
        let p = deploy_script(&home).unwrap();
        let out = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(
            out.contains("claude:unknown"),
            "failed probe falls back to plain marker: {out}"
        );
        let ver_file = cache.join("agent-version-claude.json");
        let first = std::fs::read_to_string(&ver_file).unwrap();
        assert!(
            first.contains("\"version\":\"\""),
            "failure caches empty version: {first}"
        );
        // 静默窗内第二跑：判据不变（无版本），probed_at 原样（未重探）。
        let out2 = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(out2.contains("claude:unknown"), "{out2}");
        let second = std::fs::read_to_string(&ver_file).unwrap();
        assert_eq!(
            first, second,
            "inside the 5-minute silence window no re-probe happens"
        );
        // 窗外（probed_at ticks 拨回 10 分钟前）：重探一次（仍失败，
        // probed_at 前移到当下，数值比较免 ConvertTo-Json 键序抖动）。
        let json: serde_json::Value = serde_json::from_str(&second).unwrap();
        let old_ticks: i64 = json
            .get("probed_at")
            .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
            .unwrap_or_default();
        assert!(old_ticks > 0, "probed_at present: {second}");
        let shifted = serde_json::Value::String((old_ticks - 600_000_000_000_i64).to_string());
        let mut json = json;
        json["probed_at"] = shifted;
        std::fs::write(&ver_file, serde_json::to_string(&json).unwrap()).unwrap();
        let out3 = run_statusline_with(&p, "claude", &home, br#"{"session_id":"s1"}"#, &envs);
        assert!(out3.contains("claude:unknown"), "{out3}");
        let third = std::fs::read_to_string(&ver_file).unwrap();
        let third_ticks: i64 = serde_json::from_str::<serde_json::Value>(&third)
            .unwrap()
            .get("probed_at")
            .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
            .unwrap_or_default();
        assert!(
            third_ticks > old_ticks,
            "outside the silence window the probe re-runs and re-stamps: {third}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn custom_script_marker_survives_plain_rerun_and_restores() {
        let home = scratch("custom");
        let src = home.join("my-statusline.ps1");
        std::fs::write(&src, "Write-Output 'my-bar'\n").unwrap();
        let p = deploy_custom_script(&home, &src).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "Write-Output 'my-bar'\n"
        );
        let marker = marker_path(&home);
        assert!(marker.exists(), "marker written");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            format!("{}\n", src.display()),
            "marker records the source path"
        );
        // 无 --script 的重跑不覆写自备脚本（只保 grok 壳）。
        deploy_script(&home).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "Write-Output 'my-bar'\n",
            "custom script must survive plain rerun"
        );
        assert!(home
            .join("statusline")
            .join("hst-statusline-grok.cmd")
            .exists());
        // --builtin 还原内嵌：marker 删除、内容回拼装产物。
        restore_builtin_script(&home).unwrap();
        assert!(!marker.exists());
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            default_statusline_ps1()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn dies_deploy_custom_script_rejects_missing_or_empty() {
        let home = scratch("custom-bad");
        let err = deploy_custom_script(&home, &home.join("nope.ps1")).unwrap_err();
        assert!(err.starts_with("--script"), "{err}");
        let empty = home.join("empty.ps1");
        std::fs::write(&empty, "   \n").unwrap();
        let err = deploy_custom_script(&home, &empty).unwrap_err();
        assert!(err.contains("empty script"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn codex_items_config_overrides_builtin_list() {
        // 期望值：用户清单原样透传（含未知 id——codex 侧静默跳过，hst 不拦）。
        let cfg =
            parse_config("[codex]\nitems = [\"current-dir\", \"git-branch\", \"nope\"]\n").unwrap();
        assert_eq!(
            cfg.codex_items,
            Some(vec![
                "current-dir".to_string(),
                "git-branch".to_string(),
                "nope".to_string()
            ])
        );
        let tui = render_codex_tui_section(&["current-dir", "git-branch", "nope"]);
        assert!(tui.contains("\"current-dir\""));
        assert!(tui.contains("\"nope\""));
        assert!(
            !tui.contains("run-state"),
            "default list replaced, not merged"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("nope"))
            .unwrap();
        assert!(!last.trim().ends_with(','), "no trailing comma: {last}");
        // 键缺省：无 [codex] items 时回落内嵌推荐八项。
        assert!(parse_config("").unwrap().codex_items.is_none());
    }

    #[test]
    fn dies_parse_config_rejects_malformed_codex_items() {
        assert!(parse_config("[codex]\nitems = \"x\"\n").is_err());
        assert!(parse_config("[codex]\nitems = [1]\n").is_err());
    }

    #[test]
    fn ps1_forces_utf8_before_any_output() {
        // Regression guard for the CP936 `??` corruption (P0027): the
        // encoding line must precede any output statement — the nerdfont
        // glyphs are literal UTF-8 in the script.
        let ps1 = default_statusline_ps1();
        let enc = ps1.find("[Console]::OutputEncoding").unwrap();
        let out = ps1.find("Write-Output").unwrap();
        assert!(enc < out);
        // D28 输入侧同钉：stdin 字节级读取加显式 UTF-8 解码（[Console]::In
        // 随控制台码页解码，中文路径 UTF-8 进来即「缁跨洘」形 GBK 乱码）。
        let stdin_utf8 = ps1.find("[System.Text.Encoding]::UTF8.GetString").unwrap();
        let parse = ps1.find("ConvertFrom-Json").unwrap();
        assert!(
            stdin_utf8 < parse,
            "stdin must be decoded as UTF-8 before JSON parsing"
        );
        assert!(
            ps1.contains("\u{f06a9}"),
            "hst segment robot glyph (md-robot, wide: two spaces survive one)"
        );
        assert!(
            ps1.contains("$nerd = $AgentName -ne 'grok'"),
            "Grok TUI has no Nerd PUA glyphs; script must take the ASCII path (M046)"
        );
        assert!(
            ps1.contains("Join-Path $base '.hst'"),
            "D28: statusline reads project state under .hst (D45 drops legacy dirs)"
        );
        assert!(
            !ps1.contains(".ohmyagents"),
            "D45 oma sweep: no legacy project state dir in the script"
        );
        assert!(
            ps1.contains("build.zig")
                && ps1.contains("go.mod")
                && ps1.contains("CMakeLists.txt")
                && ps1.contains("meson.build"),
            "D11 projKind must probe zig / go / cpp markers"
        );
        let cargo = ps1.find("Cargo.toml").expect("rust probe");
        let zig = ps1.find("build.zig").expect("zig probe");
        assert!(
            cargo < zig,
            "D11 first-match: rust/node/python stay ahead of zig/go/cpp"
        );
        assert!(
            ps1.contains("'zig' = '\u{e6a9} '")
                && ps1.contains("'go' = '\u{e627} '")
                && ps1.contains("'cpp' = '\u{e646} '"),
            "D11 icons baked into the default icon map: seti-zig E6A9, seti-go E627, seti-cpp E646 (CaskaydiaCove and 0xProto cmap 2026-09-07)"
        );
        assert!(
            ps1.contains("if ($nerd) { [string][char]0x2718 } else { 'x' }"),
            "deleted flag must not emit U+2718 on the Grok ASCII path"
        );
    }

    #[test]
    fn assemble_keeps_default_segment_order() {
        // 期望值来自段块的注释标记（源内容，独立于拼装逻辑）。D44 用户四
        // 令后的默认两行：一行项目状态（shell/dir/git/包版本与工具链尾
        // 巴），二行 agent 状态（hst/model/context/duration）；第三行去掉
        //（tokens、tools、mcp 三段都退出默认位，显式选用才出现）。
        let ps1 = default_statusline_ps1();
        let mut last = 0usize;
        for marker in [
            "# ── Shell 段",
            "# ── 目录：",
            "# ── Git：",
            "if ($pkgVer) {",
            "# ── Python 工具链",
            "# ── Rust 工具链",
            "# ── Node/TS 工具链",
            "# ── Zig 工具链",
            "# ── Go 工具链",
            "# ── C/C++ 工具链",
            "$slRow1 = ($parts -join ' | ')",
            "# ── hst 段",
            "# ── 模型（",
            "# ── 上下文：",
            "# ── 会话累计：",
        ] {
            let at = ps1
                .find(marker)
                .unwrap_or_else(|| panic!("missing {marker}"));
            assert!(at > last, "{marker} out of order at {at} (prev {last})");
            last = at;
        }
        assert!(
            !ps1.contains("# ── Token 用量："),
            "tokens segment out of default rows since D43"
        );
        assert!(
            !ps1.contains("# ── 工具计数："),
            "tools segment out of default rows since D44"
        );
        assert!(
            !ps1.contains("# ── MCP 计数："),
            "mcp segment out of default rows since D44"
        );
        assert!(
            !ps1.contains("$slRow3"),
            "empty default row 3 leaves no vestigial split since D44"
        );
        assert!(
            ps1.contains("'context' = '{icon}{pct}% [{used}/{window}]'"),
            "D43 context template bakes absolute tokens, not mix"
        );
        assert!(
            ps1.contains("$slRows | ForEach-Object { Write-Output $_ }"),
            "multi-row tail emits each non-empty row"
        );
    }

    #[test]
    fn assemble_reorders_and_drops_segments() {
        let ps1 =
            assemble_statusline_ps1(&[&["git", "hst"]], &StatuslineConfig::default()).unwrap();
        let g = ps1.find("# ── Git：").unwrap();
        let o = ps1.find("# ── hst 段").unwrap();
        assert!(o > g, "git must render before hst in this order");
        assert!(!ps1.contains("# ── Shell 段"), "shell dropped");
        assert!(!ps1.contains("# ── Python 工具链"), "python dropped");
        assert!(
            !ps1.contains("$slRow1"),
            "single non-empty row keeps single tail"
        );
    }

    #[test]
    fn assemble_gates_common_and_probe_on_consumers() {
        let bare = assemble_statusline_ps1(&[&["model"]], &StatuslineConfig::default()).unwrap();
        assert!(
            !bare.contains("rev-parse"),
            "COMMON skipped without dir/hst consumers"
        );
        assert!(
            !bare.contains("Cargo.toml"),
            "PROBE skipped without package/toolchain consumers"
        );
        assert!(
            !bare.contains("CTXPROBE"),
            "CTXPROBE skipped without context/tools consumers"
        );
        let probe_only =
            assemble_statusline_ps1(&[&["package"]], &StatuslineConfig::default()).unwrap();
        assert!(probe_only.contains("Cargo.toml"), "PROBE in for package");
        assert!(probe_only.contains("if ($pkgVer) {"));
        let common_only =
            assemble_statusline_ps1(&[&["hst"]], &StatuslineConfig::default()).unwrap();
        assert!(common_only.contains("rev-parse"), "COMMON in for hst");
        assert!(!common_only.contains("Cargo.toml"));
        let ctx_only =
            assemble_statusline_ps1(&[&["tools"]], &StatuslineConfig::default()).unwrap();
        assert!(
            ctx_only.contains("CTXPROBE"),
            "CTXPROBE in for tools consumer"
        );
        // D43（codex F1）：{mix} 退出缺省模板后，context 段单独在场且缺省
        // 模板 = 无消费者，不拼 CTXPROBE；显式 {mix} 模板才拼。
        let ctx_default =
            assemble_statusline_ps1(&[&["context"]], &StatuslineConfig::default()).unwrap();
        assert!(
            !ctx_default.contains("CTXPROBE"),
            "default context template has no mix consumer"
        );
        let ctx_mix = assemble_statusline_ps1(
            &[&["context"]],
            &StatuslineConfig {
                template: vec![("context".to_string(), "{icon}{pct}% {mix}".to_string())],
                ..StatuslineConfig::default()
            },
        )
        .unwrap();
        assert!(
            ctx_mix.contains("CTXPROBE"),
            "explicit {{mix}} template keeps CTXPROBE"
        );
    }

    #[test]
    fn dies_assemble_rejects_unknown_segment() {
        let err = assemble_statusline_ps1(&[&["model", "nope"]], &StatuslineConfig::default())
            .unwrap_err();
        assert!(err.contains("unknown statusline segment"), "{err}");
    }

    #[test]
    fn dies_assemble_rejects_duplicate_segment() {
        let err =
            assemble_statusline_ps1(&[&["git", "git"]], &StatuslineConfig::default()).unwrap_err();
        assert!(err.contains("duplicate statusline segment"), "{err}");
        // D40/D42：跨行重复同拒（同一渲染面出现两次）。
        let err = assemble_statusline_ps1(&[&["git"], &["git"]], &StatuslineConfig::default())
            .unwrap_err();
        assert!(
            err.contains("duplicate statusline segment"),
            "cross-row duplicate rejected: {err}"
        );
        let err = assemble_statusline_ps1(&[&["git"], &[], &["git"]], &StatuslineConfig::default())
            .unwrap_err();
        assert!(
            err.contains("duplicate statusline segment"),
            "cross-row-3 duplicate rejected: {err}"
        );
    }

    #[test]
    fn default_layout_renders_two_lines_and_explicit_third_row_opts_in() {
        // D44 行为判据（pwsh 闸门）：默认两行——一行项目状态（shell 与
        // cwd 与 git 与包版本工具链尾巴），二行 agent 状态（agent 态与
        // 模型与 context 百分比带 token 绝对值（构成 mix 退位）与耗时）；
        // 第三行去掉（tools 与 mcp 两计数、token 用量三段显式选用才出
        // 现）。codex D40 评审 G2 顺带钉 kimi 退化：同配置下 kimi 并一行。
        if !pwsh_on_path() {
            // pwsh 闸门 skip；eprintln 标痕防无 pwsh 环境静默空跑（codex
            // 评审 O3：CI 两岗带 pwsh 不空跑，本机缺位时四条判据全不跑
            // 且无提示）。
            eprintln!("skip: pwsh not on path (pwsh gate)");
            return;
        }
        let home = scratch("tworow");
        // Cargo.toml 夹具：一行尾巴的确定性判据（包版本段渲染 v9.9.9）。
        std::fs::write(
            home.join("Cargo.toml"),
            "[package]\nname = \"probe-fixture\"\nversion = \"9.9.9\"\n",
        )
        .unwrap();
        // cost 295200000 ms 经 FmtDur 恰为 3d10h（用户裁定示例形态）；payload
        // version 钉死版本来源（D46），机器装没装真 agent 判据都确定。
        let stdin = br#"{"session_id":"d1","version":"2.1.270","context_window":{"context_window_size":1000000,"used_percentage":20},"cost":{"total_duration_ms":295200000}}"#;
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(&p, "claude", &home, stdin);
        let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2, "default two-row layout: {out}");
        // 一行项目状态：包版本尾巴在场，agent 态不在一行。
        assert!(
            lines[0].contains("v9.9.9"),
            "package tail renders in row 1: {out}"
        );
        assert!(
            !lines[0].contains("claude:"),
            "agent state belongs to row 2: {out}"
        );
        assert!(
            !lines[0].contains("20% ["),
            "context belongs to row 2: {out}"
        );
        // 二行 agent 状态：agent 态（D46 带版本形）加耗时；context 段百分比
        // 直跟 token 绝对值括号（FmtTok 1024 进位：200000/1024=195k、
        // 1000000/1024=977k）。
        assert!(lines[1].contains("claude-2.1.270:unknown"), "{out}");
        assert!(
            lines[1].contains("20% [195k/977k]"),
            "D43 context shows absolute tokens, not mix: {out}"
        );
        assert!(lines[1].contains("3d10h"), "duration stays in row 2: {out}");
        assert!(
            !lines[1].contains("v9.9.9"),
            "package tail stays in row 1: {out}"
        );
        // kimi 退化（codex G2）：同配置并一行，agent 态与 token 绝对值仍可见。
        let out_kimi = run_statusline(&p, "kimi", &home, stdin);
        let kimi_lines = out_kimi
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(
            kimi_lines.len(),
            1,
            "kimi merges rows to one line: {out_kimi}"
        );
        assert!(
            out_kimi.contains("kimi-2.1.270:unknown") && out_kimi.contains("195k/977k"),
            "kimi single line keeps agent state and tokens: {out_kimi}"
        );
        // grok 退化（REQ-004，codex D40 G2 收尾）：同配置并一行，agent 态与
        // token 绝对值仍可见；且走 ASCII 面（$nerd 假路径，M046）：全行无
        // Nerd PUA 图标字形。
        let out_grok = run_statusline(&p, "grok", &home, stdin);
        let grok_lines = out_grok
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(
            grok_lines.len(),
            1,
            "grok merges rows to one line: {out_grok}"
        );
        assert!(
            out_grok.contains("grok-2.1.270:unknown") && out_grok.contains("195k/977k"),
            "grok single line keeps agent state and tokens: {out_grok}"
        );
        // ASCII 面判据取全行 is_ascii（codex 评审 O3 加宽）：不只 PUA 私用
        // 区，U+21E1 加 U+2718 加 U+00BB 类非 PUA nerd 符号漏进 grok 面
        // 同样要红（M046）。
        assert!(
            out_grok.is_ascii(),
            "grok ASCII path emits ASCII-only line (no Nerd glyphs of any block): {out_grok}"
        );
        let _ = std::fs::remove_dir_all(&home);
        // 显式选用面（D40 三要素段保留）：segments3 显式带 tools / mcp /
        // tokens 时第三行回来、三段照常渲染（transcript 夹具 3 次
        // tool_use；stdin mcp_servers 2 键）。
        let home = scratch("tworowx");
        std::fs::write(
            home.join("statusline.toml"),
            concat!(
                "segments = [\"dir\"]\n",
                "segments2 = [\"hst\", \"context\"]\n",
                "segments3 = [\"tools\", \"mcp\", \"tokens\"]\n"
            ),
        )
        .unwrap();
        let tp = home.join("session.jsonl");
        let mut tp_body = String::new();
        for _ in 0..3 {
            tp_body.push_str(
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#,
            );
            tp_body.push('\n');
        }
        std::fs::write(&tp, tp_body).unwrap();
        let stdin = format!(
            r#"{{"session_id":"d2","transcript_path":{},"mcp_servers":["a","b"],"context_window":{{"context_window_size":1000000,"used_percentage":20}}}}"#,
            serde_json::to_string(&tp.display().to_string()).unwrap()
        );
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(&p, "claude", &home, stdin.as_bytes());
        let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 3, "explicit third row renders: {out}");
        assert!(
            lines[2].contains(" 3") && lines[2].contains(" 2"),
            "explicit tools/mcp counts still render: {out}"
        );
        assert!(
            lines[2].contains("195k/977k"),
            "explicit tokens segment still renders: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn single_line_config_collapses_rows() {
        // D40/D42 逃生门：single_line = true 时三行并一行。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("sline");
        std::fs::write(
            home.join("statusline.toml"),
            "segments = [\"hst\"]\nsegments2 = [\"model\"]\nsegments3 = [\"tokens\"]\nsingle_line = true\n",
        )
        .unwrap();
        let stdin = br#"{"version":"9.9.9","context_window":{"context_window_size":1024,"used_percentage":50}}"#;
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(&p, "claude", &home, stdin);
        assert!(out.contains("claude-9.9.9:unknown"), "{out}");
        assert!(out.contains("512/1k"), "{out}");
        assert_eq!(
            out.lines().filter(|l| !l.trim().is_empty()).count(),
            1,
            "single line output: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn legacy_row_configs_upgrade_without_duplicate_error() {
        // codex D42 评审 F1/H1（升级破面与口径）：v1.1.0 双排配置（未写
        // segments3）补默认第三行并去重；D18 老单排（只写 segments）后续
        // 行不补默认 = 原样单行（真实形 payload 下也一行，区分「行不在」
        // 与「段自渲染空」）。
        if !pwsh_on_path() {
            return;
        }
        let real_payload =
            br#"{"version":"9.9.9","context_window":{"context_window_size":1000000,"used_percentage":20},"cost":{"total_duration_ms":51540000}}"#;
        // 形 1：v1.1.0 双排原样（EXAMPLE_TOML 上一版，D45 起段名 oma 改
        // hst，夹具同步）——segments3 补默认去重（tools 等已在 segments2，
        // 剔空；剩无重复段）。
        let home = scratch("upg1");
        std::fs::write(
            home.join("statusline.toml"),
            concat!(
                "segments = [\"shell\",\"dir\",\"hst\",\"model\",\"context\",\"git\"]\n",
                "segments2 = [\"tools\",\"mcp\",\"tokens\",\"duration\",\"package\",\"python\",\"rust\",\"node\",\"zig\",\"go\",\"cpp\"]\n"
            ),
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(&p, "claude", &home, real_payload);
        assert!(
            out.contains("claude-9.9.9:unknown") && out.contains("195k/977k"),
            "v1.1.0 dual-row config upgrades cleanly: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
        // 形 2：D18 老单排全量（14 段一行）——未写后续行键，不补默认 =
        // 真实形 payload 下仍是单行（老行为零漂移）。
        let home = scratch("upg2");
        std::fs::write(
            home.join("statusline.toml"),
            concat!(
                "segments = [\"shell\",\"dir\",\"hst\",\"model\",\"context\",\"duration\",\"git\",",
                "\"package\",\"python\",\"rust\",\"node\",\"zig\",\"go\",\"cpp\"]\n"
            ),
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        let out = run_statusline(&p, "claude", &home, real_payload);
        assert!(
            out.contains("claude-9.9.9:unknown") && out.contains("20% [195k/977k]"),
            "D18 single-row full config upgrades cleanly (D43 context form): {out}"
        );
        assert_eq!(
            out.lines().filter(|l| !l.trim().is_empty()).count(),
            1,
            "legacy single-row config stays one line under real payload: {out}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn dies_oma_segment_id_reports_rename_cta() {
        // D45 oma 遗产清扫：老段名 oma 不再是可用段 id，报错带改名 CTA。
        let err = assemble_statusline_ps1(&[&["oma"]], &StatuslineConfig::default()).unwrap_err();
        assert!(
            err.contains("unknown statusline segment: oma") && err.contains("改为 \"hst\""),
            "rename CTA present: {err}"
        );
    }

    #[test]
    fn grok_cmd_wrapper_invokes_ps1_with_grok_agent() {
        // Oracle: grok-build command.rs spawns the configured string as a
        // program path; the wrapper bakes the agent name so the command
        // value can stay a single path (M048).
        assert!(STATUSLINE_GROK_CMD.contains("@echo off"));
        assert!(STATUSLINE_GROK_CMD.contains("hst-statusline.ps1"));
        assert!(STATUSLINE_GROK_CMD.contains(" grok"));
        assert!(
            !STATUSLINE_GROK_CMD.contains("claude") && !STATUSLINE_GROK_CMD.contains("kimi"),
            "wrapper is Grok-only"
        );
    }

    #[test]
    fn grok_windows_command_is_bare_cmd_path() {
        // Oracle: grok-build `Command::new(entire_string)`; a shell line with
        // quotes is ERROR_INVALID_NAME 123, which is not NotFound, so the
        // shell fallback never runs (M048).
        let cmd = grok_command_line("C:/Users/ray/.ohmyagents/statusline/hst-statusline.ps1");
        #[cfg(windows)]
        {
            assert_eq!(
                cmd,
                "C:/Users/ray/.ohmyagents/statusline/hst-statusline-grok.cmd"
            );
            assert!(
                !cmd.contains('"'),
                "quotes in the program name are 123: {cmd}"
            );
            assert!(
                !cmd.contains("pwsh"),
                "args after the path are not passed: {cmd}"
            );
        }
        #[cfg(not(windows))]
        {
            assert_eq!(
                cmd,
                "pwsh -NoProfile -File \"C:/Users/ray/.ohmyagents/statusline/hst-statusline.ps1\" grok"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn dies_windows_pwsh_shell_line_is_invalid_filename() {
        // Independent oracle: Win32 ERROR_INVALID_NAME = 123. grok-build
        // command.rs only shells out on NotFound, so this error is painted.
        let cmd = r#"pwsh -NoProfile -File "C:/Users/ray/.ohmyagents/statusline/hst-statusline.ps1" grok"#;
        let err = std::process::Command::new(cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect_err("shell line must not be a valid program name");
        assert_eq!(err.raw_os_error(), Some(123), "{err}");
        assert_ne!(err.kind(), std::io::ErrorKind::NotFound, "{err:?}");
    }

    #[test]
    fn kimi_and_grok_merges_are_idempotent_and_keep_other_tables() {
        // 期望来自 kimi/grok 官方 schema（S025）：kimi [status_line].command、
        // grok [ui.status_line] type=command；其它表必须存活。
        let mut kimi: toml::Value =
            toml::from_str("theme = \"dark\"\n[status_line]\nitems = [\"model\"]\n").unwrap();
        assert!(apply_kimi_status_line(&mut kimi, "C:/x/hst-statusline.ps1").unwrap());
        assert!(!apply_kimi_status_line(&mut kimi, "C:/x/hst-statusline.ps1").unwrap());
        let kimi_t = kimi.as_table().unwrap();
        assert_eq!(kimi_t.get("theme").unwrap().as_str(), Some("dark"));
        let sl = kimi_t.get("status_line").unwrap().as_table().unwrap();
        assert_eq!(
            sl.get("command").unwrap().as_str(),
            Some("pwsh -NoProfile -File \"C:/x/hst-statusline.ps1\" kimi")
        );
        assert_eq!(
            sl.get("items").unwrap().as_array().unwrap().len(),
            1,
            "foreign [status_line] keys survive"
        );

        let mut grok: toml::Value =
            toml::from_str("model = \"x\"\n[ui]\npermission_mode = \"always-approve\"\n").unwrap();
        assert!(apply_grok_status_line(&mut grok, "C:/x/hst-statusline.ps1").unwrap());
        assert!(!apply_grok_status_line(&mut grok, "C:/x/hst-statusline.ps1").unwrap());
        let grok_t = grok.as_table().unwrap();
        assert_eq!(grok_t.get("model").unwrap().as_str(), Some("x"));
        let ui = grok_t.get("ui").unwrap().as_table().unwrap();
        assert_eq!(
            ui.get("permission_mode").unwrap().as_str(),
            Some("always-approve"),
            "yolo key in [ui] survives"
        );
        let sl = ui.get("status_line").unwrap().as_table().unwrap();
        assert_eq!(sl.get("type").unwrap().as_str(), Some("command"));
        let grok_cmd = sl.get("command").unwrap().as_str().unwrap();
        assert_eq!(grok_cmd, grok_command_line("C:/x/hst-statusline.ps1"));
        #[cfg(windows)]
        assert_eq!(grok_cmd, "C:/x/hst-statusline-grok.cmd");
        #[cfg(not(windows))]
        assert!(grok_cmd.ends_with("\" grok"));
    }

    #[test]
    fn codex_tui_section_is_builtin_ids_not_command_argv() {
        let tui = render_codex_tui_section(CODEX_STATUS_LINE_ITEMS);
        assert!(tui.contains("run-state"));
        // D46：codex 侧版本走内置项 codex-version（源码实证
        // StatusLineItem::CodexVersion，strum kebab_case）。
        assert!(tui.contains("codex-version"));
        assert!(tui.contains("git-branch"));
        assert!(tui.contains("status_line_use_colors = true"));
        assert!(
            !tui.contains("pwsh") && !tui.contains("oma-statusline"),
            "Codex silently skips unknown IDs; command argv empties the bar: {tui}"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("branch-changes"))
            .unwrap();
        assert!(
            !last.trim().ends_with(','),
            "TOML 1.0 rejects trailing commas: {last}"
        );
    }

    #[test]
    fn strip_tui_section_keeps_other_tables() {
        let text = "model = \"gpt\"\n[tui]\nstatus_line = [\"old\"]\n[sandbox]\nmode = \"rw\"\n";
        let out = strip_tui_section(text);
        assert!(out.contains("model = \"gpt\""));
        assert!(out.contains("[sandbox]"));
        assert!(!out.contains("\"old\""));
        assert!(!out.contains("[tui]"));
    }

    #[test]
    fn sentinel_selfheal_markers_present_in_script() {
        // REQ-014 形锁：hst 段含注册哨兵件（no-hook! 升格、hst-state.sh
        // 探针标记、hst hook init 自愈调用、节流 stamp 面）。
        let home = scratch("sentinel");
        let path = deploy_script(&home).unwrap();
        let script = std::fs::read_to_string(&path).unwrap();
        assert!(script.contains("no-hook!"), "升格态在册");
        assert!(script.contains("hst-state.sh"), "注册探针标记");
        assert!(script.contains("& hst hook init"), "自愈调用");
        assert!(script.contains(".hookcheck-"), "节流 stamp 面");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sentinel_probe_covers_four_agents() {
        // 四家注册面定位（claude/codex/grok/kimi 的配置文件路径行）都在
        // 生成脚本里。
        let home = scratch("sentinel4");
        let path = deploy_script(&home).unwrap();
        let script = std::fs::read_to_string(&path).unwrap();
        for marker in [
            ".claude/settings.json",
            ".codex/hooks.json",
            ".grok/hooks/ohmyagents-state.json",
            ".kimi-code/config.toml",
        ] {
            assert!(script.contains(marker), "探针面缺 {marker}");
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn sentinel_flags_missing_registration_and_stamps_throttle() {
        // REQ-014 行为面：注册面缺位时 hst 段升格 no-hook!，且节流 stamp
        // 落盘（自愈尝试的节流痕迹由脚本自写，不依赖 hst 在 PATH，CI 可
        // 移植；hst hook init 的真自愈回写由本机实弹验收在 diary 记档）。
        let home = scratch("sentinel-noreg");
        let path = deploy_script(&home).unwrap();
        let out = run_statusline_opts(&path, "claude", &home, b"{}", &[], false);
        assert!(out.contains("no-hook!"), "升格态渲染：{out}");
        assert!(
            home.join(".hst/state/.hookcheck-claude").exists(),
            "节流 stamp 落盘"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
