# hst-cli::caps

CPU 指令集能力与探针退出形态分类（S021/P0018）。
CPU 指令集能力与探针退出形态（S021 落地的 Windows 可测部分）。
检测用 std 的 `is_x86_feature_detected!`：内部先 CPUID 再验 OS 使能面
（OSXSAVE 与 XCR0），比 flags 筛查可靠；非 x86_64 目标返回 None（unknown）。
退出形态分类覆盖 S021 问题类的两个崩溃面：Windows
STATUS_ILLEGAL_INSTRUCTION（0xC000001D）与 Unix SIGILL（signal 4，
cfg(unix) 分支待 P0012 Linux 实机编译验证）。

## Functions

- `caps_line` — marker 行形态：`x86_64 avx=true avx2=true avx512f=false`（unknown 时同形）。
- `classify_probe_exit` — 探针退出形态分类。
- `detect` — 指令集能力的detect面（细则见模块文档与集成测试）。

## Types

- `CpuCaps` — 指令集能力的CpuCaps面（细则见模块文档与集成测试）。

