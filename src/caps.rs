//! CPU 指令集能力与探针退出形态（S021 落地的 Windows 可测部分）。
//! 检测用 std 的 `is_x86_feature_detected!`：内部先 CPUID 再验 OS 使能面
//! （OSXSAVE 与 XCR0），比 flags 筛查可靠；非 x86_64 目标返回 None（unknown）。
//! 退出形态分类覆盖 S021 问题类的两个崩溃面：Windows
//! STATUS_ILLEGAL_INSTRUCTION（0xC000001D）与 Unix SIGILL（signal 4，
//! cfg(unix) 分支待 P0012 Linux 实机编译验证）。

/// Windows STATUS_ILLEGAL_INSTRUCTION 的 i32 形态
/// （0xC000001D = 3221225501，i32 回绕为负）。
#[cfg(windows)]
pub const ILLEGAL_INSTRUCTION_EXIT: i32 = -1_073_741_795;

/// 探针退出形态分类。
pub fn classify_probe_exit(code: Option<i32>) -> &'static str {
    match code {
        Some(0) => "ok",
        // Windows：STATUS_ILLEGAL_INSTRUCTION 的 i32 回绕形态。
        #[cfg(windows)]
        Some(c) if c == ILLEGAL_INSTRUCTION_EXIT => "illegal-instruction",
        Some(_) => "failed",
        // Unix：信号退出（code() 为 None），signal 4 = SIGILL。
        // Windows 上不会出现 None 形态。
        None => "signal-exit",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// CpuCaps：CPU 指令集能力的数据面。
pub struct CpuCaps {
    /// arch：CPU 指令集能力公开项。
    pub arch: &'static str,
    /// None = 非 x86_64 目标或检测不可用。
    pub avx: Option<bool>,
    /// avx2：CPU 指令集能力公开项。
    pub avx2: Option<bool>,
    /// avx512f：CPU 指令集能力公开项。
    pub avx512f: Option<bool>,
}

/// detect：CPU 指令集能力的公开入口（行为细则与 marker 见 R002）。
pub fn detect() -> CpuCaps {
    #[cfg(target_arch = "x86_64")]
    {
        CpuCaps {
            arch: std::env::consts::ARCH,
            avx: Some(std::arch::is_x86_feature_detected!("avx")),
            avx2: Some(std::arch::is_x86_feature_detected!("avx2")),
            avx512f: Some(std::arch::is_x86_feature_detected!("avx512f")),
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        CpuCaps {
            arch: std::env::consts::ARCH,
            avx: None,
            avx2: None,
            avx512f: None,
        }
    }
}

/// marker 行形态：`x86_64 avx=true avx2=true avx512f=false`（unknown 时同形）。
pub fn caps_line(c: &CpuCaps) -> String {
    let f = |v: Option<bool>| v.map(|b| b.to_string()).unwrap_or_else(|| "unknown".into());
    format!(
        "{} avx={} avx2={} avx512f={}",
        c.arch,
        f(c.avx),
        f(c.avx2),
        f(c.avx512f)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_reports_arch_and_fields_on_x86_64() {
        // 值随机器，只锁结构：x86_64 上三项必须是有值布尔。
        let c = detect();
        if cfg!(target_arch = "x86_64") {
            assert_eq!(c.arch, "x86_64");
            assert!(c.avx.is_some() && c.avx2.is_some() && c.avx512f.is_some());
        } else {
            assert!(c.avx.is_none() && c.avx2.is_none() && c.avx512f.is_none());
        }
        let line = caps_line(&c);
        assert!(line.starts_with(c.arch));
        assert!(line.contains("avx2="));
    }

    #[test]
    fn probe_exit_classification_covers_forms() {
        assert_eq!(classify_probe_exit(Some(0)), "ok");
        assert_eq!(classify_probe_exit(Some(1)), "failed");
        assert_eq!(classify_probe_exit(None), "signal-exit");
        #[cfg(windows)]
        assert_eq!(
            classify_probe_exit(Some(ILLEGAL_INSTRUCTION_EXIT)),
            "illegal-instruction"
        );
    }
}
