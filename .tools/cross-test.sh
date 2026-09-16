#!/usr/bin/env bash
# 全平台实弹测试矩阵（REQ-007）：本地构建各平台产物，scp 到 SSH 端，跑
# --version 与 doctor 冒烟，汇总矩阵。linux 端就地跑（本机即 linux）。
# 用法：.tools/cross-test.sh [版本标签]；版本标签缺省取 Cargo.toml。
# 依赖：ssh BatchMode 已配好 lan-mac / lan-ubuntu（lan-win 待宿主 sshd）。
set -euo pipefail
cd "$(dirname "$0")/.."

VER="${1:-$(rg -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
OUT=/tmp/hst-cross-$VER
mkdir -p "$OUT"

build() { # target out-name
  echo "== build $1"
  cargo build --release --locked --target "$1" --bins
  cp "target/$1/release/${2}" "$OUT/${3}"
}

# mac 产物：Linux 无法交叉（ring 的 C 面需 macos SDK），从 CI dev release 取
#（digest 与 GitHub 三方对账面，S028）；失败则跳过 mac 端。
fetch_mac() {
  echo "== fetch mac asset from dev release"
  gh release download dev -p 'hst-aarch64-apple-darwin.tar.gz' --clobber -O /tmp/hst-mac.tgz -R raystyle/hst_rs     && tar xzf /tmp/hst-mac.tgz -C /tmp && cp /tmp/hst-aarch64-apple-darwin/hst "$OUT/hst-mac"
}

run_remote() { # host bin
  local h="$1" b="$2" tmpd
  tmpd=$(ssh -o BatchMode=yes "$h" 'mktemp -d')
  scp -q "$OUT/$b" "$h:$tmpd/hst"
  echo "-- $h"
  ssh -o BatchMode=yes "$h" "chmod +x $tmpd/hst && $tmpd/hst --version && { $tmpd/hst doctor | tail -1 || true; }; rm -rf $tmpd"
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
echo "windows channel: pending host sshd (lan-win port 22 refused)"
echo "matrix done"
