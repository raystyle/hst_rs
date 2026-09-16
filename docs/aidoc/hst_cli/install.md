# hst-cli::install

hst 根解析与共享下载件（self update 复用）。
hst 自管根与共享下载件。agent 二进制安装 / 升级机器已随 D20 移除（归
ome，D07 迁册）；本模块只留三件消费面共享的能力：hst 数据根解析、自管
根存量安装的只读探测（`hst agents` 的 source=oma 面）、单文件下载
（`hst self update` 复用）。

## Functions

- `agents_root` — hst 自管 agent 安装根（oma 纪元存量布局 `<根>/agents/<name>/<version>/`）。
- `hst_home` — hst 应用数据根：`HST_ROOT` 环境变量 > `~/.hst`（D29；旧 `~/.oma` 与
- `managed_binaries` — hst 自管安装的二进制清单（agents.rs 的 Probe 以 source=oma 消费）。
- `managed_version` — hst 自管已装版本（manifest 的 version 字段；多个版本目录取最大）。

