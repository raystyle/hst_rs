# hst-cli::doctor

`hst doctor` 只读体检：yolo、信任、二进制、登录态、hook 形态、状态栏与状态面。

## Functions

- `diagnose` — Read-only. Does not attach, send-keys, or wait on TUI.
- `print_diagnosis` — print_diagnosis：只读体检 doctor的公开入口（行为细则与 marker 见 R002）。

## Types

- `Diagnosis` — Diagnosis：只读体检 doctor的数据面。
- `Finding` — Finding：只读体检 doctor的数据面。
- `Status` — Status：只读体检 doctor的取值集。

