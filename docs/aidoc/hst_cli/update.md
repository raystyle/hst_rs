# hst-cli::update

`hst self update`：dev 滚动与 stable 正式版自更新、镜像腿与缺省回退（S028/D16/D48）。
hst 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
releases 为空时走 `--git` 源码安装路径）。
机制见 S028：releases/latest API、资产命名约定 `hst-<triple>.(zip|tar.gz)`、
Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。
D16 起 `HST_MIRROR=<基址>` 走镜像通道；ADR-0008（2026-09-18 家族统一
标准，承接 D48）：未设 = 镜像段优先（默认基址 env.ohmygh.com，任一步
网络类失败整对回落 GitHub，不回环）；设值 = 基址覆盖同读序；空串 =
镜像全关。镜像判新走 `<基址>/hst/<seg>/<资产名>.sha256` 边车对安装
记录（段随通道，dev 禁落 stable）。镜像侧仅网络类失败回落；哈希不符
是安全问题，报错不回落（GitHub 官方腿同判：下载后对 API digest 或同
Release 边车硬校验，锚不可得同样拒装）。latest 判新走 release tag
三态（本地领先报 localNewer 不动），镜像 stable 腿降级守卫由暂存件
`--version` 预检承载（命中按 localNewer 收束，不装不回落）；自替换带
更新锁、陈旧收割、入位后自证五次重试与回滚复核；ark 管理布局（同
目录落痕或带落痕的符号链接入口）让位走 ark。GH_TOKEN 在位附 Bearer。

## Functions

- `fetch_release` — # Errors
- `git_install` — # Errors
- `pick_asset` — Pick the hst asset for this host from a release's asset list.
- `run` — # Errors
- `self_replace` — # Errors
- `version_newer` — Dotted-numeric compare: is `tag` (v-prefix tolerated) strictly newer than

## Types

- `Asset` — release 资产条目：名、下载直链与可选 digest（判新锚）。
- `Channel` — 自更新的更新通道面（细则见模块文档与集成测试）。
- `Release` — GitHub release 元数据（tag 与资产清单），self update 的查询面。
- `ReplaceOutcome` — 自替换收束态（ADR-0008 家族标准）。

## Constants

- `DEFAULT_REPO` — Default repo; `--repo owner/name` overrides. 更名史（2026-09-02）：

