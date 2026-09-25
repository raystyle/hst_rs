# release.ps1 - 本地发布脚本（ADR-0007 批 A：三段式第一二段，本地编译打包 + gh 直发）
# 载体 pwsh 7（wsl 主开发机跑；中文注释，输出面 ASCII）。
# 前置：wsl 有 mingw（win-gnu 交叉）与 gh 已登录；lan-mac 有 cargo 与 git（ssh 免密）；
# macbook（intelMBP 实机，x86_64 冒烟宿主，ssh 免密）。
# 用法：git tag v2.x.y && git push origin v2.x.y 之后：
#   pwsh -NoProfile -File .tools/release.ps1            # 全链（闸 + 构建 + 打包 + 冒烟 + gh 直发）
#   pwsh -NoProfile -File .tools/release.ps1 -DryRun    # 到冒烟为止，不建 Release
param(
    [string]$MacHost = "lan-mac",
    [string]$IntelMacHost = "macbook",
    [string]$WinHost = "ray@127.0.0.1",
    [switch]$DryRun,
    [switch]$SkipWinSmoke,
    [switch]$SkipIntelMacSmoke
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $root

# ---- 1 版本一致性闸：HEAD 必须钉在 v* tag 上，tag 与 Cargo.toml version 逐字对 ----
$tag = git describe --tags --exact-match HEAD 2>$null | Out-String
$tag = $tag.Trim()
if ($tag -notmatch '^v\d+\.\d+\.\d+$') {
    throw "gate: HEAD is not on a vX.Y.Z tag (got '$tag'); tag and push first"
}
$ver = ($tag -replace '^v', '')
$cargoVer = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
if ($ver -ne $cargoVer) {
    throw "gate: tag $ver != Cargo.toml version $cargoVer"
}
Write-Output "gate.version=ok tag=$ver"

# ---- 1b 发布预检（codex 批 A G2/G3）：gh 已登、远端 tag 已推且指本 sha、tag 无既有 Release ----
gh auth status *> $null
if ($LASTEXITCODE -ne 0) { throw "precheck: gh not logged in (run gh auth login)" }
$remoteTag = git ls-remote --tags origin "refs/tags/$tag" 2>$null | Out-String
if ($remoteTag.Trim() -eq "") {
    throw "precheck: tag $tag not on origin yet; git push origin $tag first (gh would auto-tag a different HEAD)"
}
if ($remoteTag -notmatch [regex]::Escape((git rev-parse HEAD).Trim())) {
    throw "precheck: remote tag $tag points elsewhere; re-push the tag from this HEAD"
}
gh release view $tag *> $null
if ($LASTEXITCODE -eq 0) {
    throw "precheck: release $tag already exists; recover via gh release delete $tag --yes then rerun, or gh release upload $tag <assets> --clobber"
}
Write-Output "gate.precheck=ok gh-auth remote-tag no-existing-release"

# ---- 2 测试闸：cargo test --locked 加 md 四门禁加 aidoc 漂移（G4）先行 ----
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw "gate: cargo test failed" }
# pwsh 原生命令失败不触发 ErrorActionPreference，逐门禁查退出码（codex 二
# 轮新 G：否则 md 门禁等于没跑）。
foreach ($g in @(
    @{ c = "uv run --script .tools/md-ref-scan.py"; n = "md-ref-scan" },
    @{ c = "uv run --script .tools/md-heading-scan.py"; n = "md-heading-scan" },
    @{ c = "uv run --script .tools/mdcharlint.py"; n = "mdcharlint" },
    @{ c = "uvx rumdl check ."; n = "rumdl" },
    @{ c = "cargo aidoc --check --strict"; n = "aidoc-strict" }
)) {
    Invoke-Expression $g.c
    if ($LASTEXITCODE -ne 0) { throw "gate: $($g.n) failed (rc=$LASTEXITCODE)" }
}
Write-Output "gate.test=ok cargo+md4+aidoc"

# ---- 3 本地构建：linux 本职 + win-gnu 交叉（wsl）；mac 实机双 darwin（ssh lan-mac，
# arm64 本职加 x86_64 交叉；2026-09-25 档扩，ADR-0007 后记）----
$targets = @("x86_64-unknown-linux-gnu", "x86_64-pc-windows-gnu")
foreach ($t in $targets) {
    cargo build --release --locked --target $t --bins
    if ($LASTEXITCODE -ne 0) { throw "build: $t failed" }
    Write-Output "build.local=ok $t"
}
$sha = git rev-parse HEAD
ssh -o BatchMode=yes $MacHost "set -e; export PATH=`"`$HOME/.cargo/bin:`$PATH`"; mkdir -p ~/repos; if [ -d ~/repos/hst_rs/.git ]; then git -C ~/repos/hst_rs fetch --tags --force -q; else git clone -q https://github.com/raystyle/hst_rs.git ~/repos/hst_rs; fi; git -C ~/repos/hst_rs checkout -q $sha; rustup target add x86_64-apple-darwin; cargo build --release --locked --target aarch64-apple-darwin --bins --manifest-path ~/repos/hst_rs/Cargo.toml; cargo build --release --locked --target x86_64-apple-darwin --bins --manifest-path ~/repos/hst_rs/Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "build: mac remote failed" }
New-Item -ItemType Directory -Force "dist/hst-aarch64-apple-darwin" | Out-Null
New-Item -ItemType Directory -Force "dist/hst-x86_64-apple-darwin" | Out-Null
scp -q "${MacHost}:~/repos/hst_rs/target/aarch64-apple-darwin/release/hst" "dist/hst-aarch64-apple-darwin/hst"
scp -q "${MacHost}:~/repos/hst_rs/target/x86_64-apple-darwin/release/hst" "dist/hst-x86_64-apple-darwin/hst"
Write-Output "build.mac=ok aarch64-apple-darwin x86_64-apple-darwin"

# ---- 4 打包：单顶层目录 = 二进制 + README + LICENSE；win 形 zip 他形 tar.gz；逐包 sha256 边车 ----
if (Test-Path dist/pkg) { Remove-Item -Recurse -Force dist/pkg }
New-Item -ItemType Directory -Force dist/pkg | Out-Null
foreach ($t in $targets + @("aarch64-apple-darwin", "x86_64-apple-darwin")) {
    $dir = "dist/pkg/hst-$t"
    New-Item -ItemType Directory -Force $dir | Out-Null
    $bin = if ($t -like "*-apple-darwin") { "dist/hst-$t/hst" }
    elseif ($t -eq "x86_64-pc-windows-gnu") { "target/$t/release/hst.exe" }
    else { "target/$t/release/hst" }
    Copy-Item $bin "$dir/" -Force
    Copy-Item README.md "$dir/" -Force
    Copy-Item LICENSE "$dir/" -Force
    Set-Location dist/pkg
    if ($t -eq "x86_64-pc-windows-gnu") {
        Compress-Archive -Path "hst-$t" -DestinationPath "hst-$t.zip" -Force
    }
    else {
        tar czf "hst-$t.tar.gz" "hst-$t"
    }
    $art = if ($t -eq "x86_64-pc-windows-gnu") { "hst-$t.zip" } else { "hst-$t.tar.gz" }
    $hash = (Get-FileHash $art -Algorithm SHA256).Hash.ToLower()
    "$hash  $art" | Out-File -Encoding ascii -NoNewline "$art.sha256"
    Write-Output "pkg=ok $art $hash"
    Set-Location $root
}

# F1（codex 批 A）：清打包暂存目录，publish 面只认八件并计数断言。
Get-ChildItem dist/pkg -Directory | Remove-Item -Recurse -Force
$plan = @(Get-ChildItem dist/pkg -File | Sort-Object Name)
if ($plan.Count -ne 8) {
    throw "gate: publish plan expects 8 files (4 archives + 4 sidecars), got $($plan.Count): $($plan.Name -join ', ')"
}
Write-Output "gate.publish-plan=ok $($plan.Name -join ', ')"

# ---- 5 解包冒烟：四端各解包跑 --version 与 tag 逐字对 ----
$smoke = "dist/smoke"
if (Test-Path $smoke) { Remove-Item -Recurse -Force $smoke }
New-Item -ItemType Directory -Force $smoke | Out-Null
Set-Location $smoke
tar xzf ../pkg/hst-x86_64-unknown-linux-gnu.tar.gz
$lv = ./hst-x86_64-unknown-linux-gnu/hst --version
if ($lv.Trim() -ne "hst $ver") { throw "smoke: linux version '$lv' != 'hst $ver'" }
Write-Output "smoke.linux=ok $lv"
Set-Location $root
scp -q dist/pkg/hst-aarch64-apple-darwin.tar.gz "${MacHost}:/tmp/hst-smoke.tar.gz"
$mv = ssh -o BatchMode=yes $MacHost "tar xzf /tmp/hst-smoke.tar.gz -C /tmp && /tmp/hst-aarch64-apple-darwin/hst --version; rm -rf /tmp/hst-smoke.tar.gz /tmp/hst-aarch64-apple-darwin"
if ("$mv".Trim() -ne "hst $ver") { throw "smoke: mac version '$mv' != 'hst $ver'" }
Write-Output "smoke.mac=ok $mv"
# intel mac 冒烟（2026-09-25 档扩）：宿主 macbook = intelMBP 实机（macOS 12.7.6），
# 不可达时与 win 腿同形：显式弃保旗标放行，否则红。
scp -q dist/pkg/hst-x86_64-apple-darwin.tar.gz "${IntelMacHost}:/tmp/hst-smoke-x64.tar.gz" 2>$null
if ($LASTEXITCODE -eq 0) {
    $iv = ssh -o BatchMode=yes $IntelMacHost "tar xzf /tmp/hst-smoke-x64.tar.gz -C /tmp && /tmp/hst-x86_64-apple-darwin/hst --version; rm -rf /tmp/hst-smoke-x64.tar.gz /tmp/hst-x86_64-apple-darwin"
    if ("$iv".Trim() -ne "hst $ver") { throw "smoke: intel-mac version '$iv' != 'hst $ver'" }
    Write-Output "smoke.intel-mac=ok $iv"
}
elseif ($SkipIntelMacSmoke) {
    Write-Output "smoke.intel-mac=skipped-by-flag (record in release notes)"
}
else {
    throw "smoke: intel mac host unreachable; pass -SkipIntelMacSmoke to explicitly waive"
}
scp -q dist/pkg/hst-x86_64-pc-windows-gnu.zip "${WinHost}:C:/Users/ray/hst-smoke.zip" 2>$null
if ($LASTEXITCODE -eq 0) {
    ssh -o BatchMode=yes $WinHost 'powershell -NoProfile -Command "Expand-Archive -Force C:\Users\ray\hst-smoke.zip C:\Users\ray\hst-smoke; & C:\Users\ray\hst-smoke\hst-x86_64-pc-windows-gnu\hst.exe --version; Remove-Item -Recurse -Force C:\Users\ray\hst-smoke, C:\Users\ray\hst-smoke.zip"' |
        ForEach-Object { $wv = $_ }
    if ("$wv".Trim() -ne "hst $ver") { throw "smoke: win version '$wv' != 'hst $ver'" }
    Write-Output "smoke.win=ok $wv"
}
else {
    if ($SkipWinSmoke) {
        Write-Output "smoke.win=skipped-by-flag (record in release notes)"
    }
    else {
        throw "smoke: win host unreachable and win PE has no other runtime check; pass -SkipWinSmoke to explicitly waive"
    }
}

# ---- 6 gh 直发（--latest 禁 draft；产物只进 Release 与镜像段，零 commit 回仓）----
if ($DryRun) {
    Write-Output "publish=dry-run skip (assets in dist/pkg)"
    exit 0
}
gh release create $tag dist/pkg/*.zip dist/pkg/*.tar.gz dist/pkg/*.sha256 --latest --title "hst $tag" --notes "正式版 $tag（本地编译打包直发，ADR-0007）。安装：hst self update --stable。"
if ($LASTEXITCODE -ne 0) { throw "publish: gh release create failed" }
Write-Output "publish=ok $tag --latest"
