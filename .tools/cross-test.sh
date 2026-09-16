#!/usr/bin/env bash
# 全平台实弹测试矩阵（REQ-007）：本地构建各平台产物，scp 到 SSH 端，跑
# --version 与 doctor 冒烟，汇总矩阵。linux 端就地跑（本机即 linux）；
# Windows 宿主恒走 127.0.0.1 回环（口径见 dev-evo env-platform 第十节）。
# 用法：.tools/cross-test.sh [版本标签]；版本标签缺省取 Cargo.toml。
# 依赖：ssh BatchMode 已配好 lan-mac / lan-ubuntu；宿主 sshd 监听回环。
set -euo pipefail
cd "$(dirname "$0")/.."

VER="${1:-$(rg -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
OUT=/tmp/hst-cross-$VER
mkdir -p "$OUT"

build() { # target out-name dest
  echo "== build $1"
  cargo build --release --locked --target "$1" --bins
  cp "target/$1/release/${2}" "$OUT/${3}"
}

# mac 产物：Linux 无法交叉（ring 的 C 面需 macos SDK），从 CI dev release 取
#（digest 与 GitHub 三方对账面，S028）；失败则跳过 mac 端。
fetch_mac() {
  echo "== fetch mac asset from dev release"
  gh release download dev -p 'hst-aarch64-apple-darwin.tar.gz' --clobber -O /tmp/hst-mac.tgz -R raystyle/hst_rs \
    && tar xzf /tmp/hst-mac.tgz -C /tmp && cp /tmp/hst-aarch64-apple-darwin/hst "$OUT/hst-mac"
}

run_remote() { # host bin
  local h="$1" b="$2" tmpd
  tmpd=$(ssh -o BatchMode=yes "$h" 'mktemp -d')
  scp -q "$OUT/$b" "$h:$tmpd/hst"
  echo "-- $h"
  ssh -o BatchMode=yes "$h" "chmod +x $tmpd/hst && $tmpd/hst --version && { $tmpd/hst doctor | tail -1 || true; }; rm -rf $tmpd"
}

run_remote_win() {
  # Windows 宿主：WSL 恒走 127.0.0.1 回环（不走宿主 mesh IP，dev-evo
  # env-platform 第十节）；临时目录走 %TEMP%，powershell 冒烟后清理。
  local tmpd="hst-cross-test-$RANDOM"
  local winpath="C:/Users/ray/AppData/Local/Temp/$tmpd"
  echo "-- lan-win (loopback 127.0.0.1)"
  ssh -o BatchMode=yes ray@127.0.0.1 "powershell -NoProfile -Command \"New-Item -ItemType Directory -Force $winpath | Out-Null\"" >/dev/null
  scp -q "$OUT/hst.exe" "ray@127.0.0.1:$winpath/hst.exe"
  ssh -o BatchMode=yes ray@127.0.0.1 "powershell -NoProfile -Command \"& $winpath\\hst.exe --version; & $winpath\\hst.exe doctor | Select-Object -Last 1; Remove-Item -Recurse -Force $winpath\"" || true
}

echo "hst $VER cross matrix"
build x86_64-unknown-linux-gnu hst hst-linux
build x86_64-pc-windows-gnu hst.exe hst.exe
fetch_mac

echo "-- local-linux"
"$OUT/hst-linux" --version
{ "$OUT/hst-linux" doctor | tail -1 || true; }
run_remote lan-ubuntu hst-linux
run_remote lan-mac hst-mac
run_remote_win
echo "matrix done"
