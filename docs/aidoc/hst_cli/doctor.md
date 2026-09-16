# hst-cli::doctor

`hst doctor` 只读体检：yolo、信任、二进制、登录态、hook 形态、状态栏与状态面。

## Functions

- `diagnose` — # Errors
- `print_diagnosis` — 体检的print_diagnosis面（细则见 R002 与模块文档）。

## Types

- `Diagnosis` — 体检结果集：findings 列表与阻断判定。
- `Finding` — 单条体检发现：agent、检查名、状态、路径与明细。
- `Status` — 体检结论三态：ok、warn（部署缺口不计败）与 block（退出 1）。

