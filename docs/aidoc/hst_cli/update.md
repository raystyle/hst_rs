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

- `fetch_release` — # Errors
- `git_install` — # Errors
- `pick_asset` — Pick the hst asset for this host from a release's asset list.
- `run` — # Errors
- `self_replace` — # Errors
- `version_newer` — Dotted-numeric compare: is `tag` (v-prefix tolerated) strictly newer than

## Types

- `Asset` — release 资产条目：名、下载直链与可选 digest（判新锚）。
- `Channel` — 自更新的更新通道面（细则见 R002 与模块文档）。
- `Release` — GitHub release 元数据（tag 与资产清单），self update 的查询面。

## Constants

- `DEFAULT_REPO` — Default repo; `--repo owner/name` overrides. 更名史（2026-09-02）：

