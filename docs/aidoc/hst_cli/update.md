# hst-cli::update

`hst self update`：dev 滚动与 stable 正式版自更新、镜像腿与缺省回退（S028/D16/D48）。
hst 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
releases 为空时走 `--git` 源码安装路径）。
机制见 S028：releases/latest API、资产命名约定 `hst-<triple>.(zip|tar.gz)`、
Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。
D16 起 `HST_MIRROR=<基址>` 走镜像通道；D48 扩到双通道与缺省回退：设值 =
mirror-first（失败回落 GitHub）；未设 = GitHub 优先、失败自动回退镜像腿
（默认基址 env.ohmygh.com）；空串 = 镜像全关。镜像判新走
`<基址>/hst/<seg>/<资产名>.sha256` 边车对安装记录（段随通道，dev 禁落
stable）。镜像侧仅网络类失败回落；哈希不符是安全问题，报错不回落。
GH_TOKEN 在位附 Bearer（D48，匿名 60 升 5000 次每时）。

## Functions

- `fetch_release` — fetch_release：self update 自更新的公开入口（行为细则与 marker 见 R002）。
- `git_install` — `cargo install --git` 源码安装（封版前的主路径）。
- `pick_asset` — Pick the hst asset for this host from a release's asset list.
- `run` — `hst self update` entry: release path with git fallback.
- `self_replace` — Atomic-ish self replace: write the new binary beside the current exe, then
- `version_newer` — Dotted-numeric compare: is `tag` (v-prefix tolerated) strictly newer than

## Types

- `Asset` — Asset：self update 自更新的数据面。
- `Channel` — 更新通道：dev = 滚动预发布 tag `dev`（CI 每推覆盖，部署位缺省）；
- `Release` — Release：self update 自更新的数据面。

## Constants

- `DEFAULT_REPO` — Default repo; `--repo owner/name` overrides. 更名史（2026-09-02）：

