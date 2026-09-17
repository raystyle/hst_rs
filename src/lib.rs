//! hst-cli：HST（Hooks, Statusline, Trace）库面——四家 agent（claude、
//! codex、grok、kimi）的部署配置与诊断内核。承载五功能：hook 状态落盘
//! （用户级 session 分键）、状态栏部署与拆段拼装、只读 trace 六视图联邦
//! 检索、doctor 只读体检与 yolo 无阻塞键分级管理。自更新走 dev 滚动与
//! stable 双通道（镜像腿带缺省回退）。命令契约见 clap 帮助与集成测试；行为动机见
//! docs/research 与 docs/adr；输出信封契约见 fmtio 模块文档。

/// 四家 agent 探测（PATH、env、hst 自管根、默认目录四源）。
pub mod agents;
/// 的通用归档工具面（细则见模块文档与集成测试）。
pub mod archive;
/// CPU 指令集能力与探针退出形态分类（S021/P0018）。
pub mod caps;
/// `hst init` 部署层：hook 注册四家用户级、shim 落位、状态栏面与 ours 技能目录清扫（D53/ADR-0005）。
pub mod deploy;
/// `hst diagnose` 活性诊断族：网关缓存命中矩阵与配置活性（D21）。
pub mod diagnose;
/// `hst doctor` 只读体检：yolo、信任、二进制、登录态、hook 形态、状态栏与状态面。
pub mod doctor;
/// 全局输出三态（kv/json/jsonl）与结构化错误出口契约。
pub mod fmtio;
/// `hst hook`：事件到四态映射、用户级 session 分键 state 落盘与密钥拦截分流（D28/S030）。
pub mod hook;
/// hst 根解析与共享下载件（self update 复用）。
pub mod install;
/// 的路径工具面（细则见模块文档与集成测试）。
pub mod pathutil;
/// 的密钥拦截闸面（细则见模块文档与集成测试）。
pub mod secretguard;
/// shim 三形态自包含状态写入器：cmd/ps1/sh 加 grok 包装（D27/D28/D39）。
pub mod shim;
/// `hst statusline`：四家状态栏写入面幂等合并与拆段拼装（S025/D18/D42 至 D51）。
pub mod statusline;
/// trace 六视图：联邦读四家原生会话库归一检索（P0013/P0014，D19）。
pub mod trace;
/// `hst self update`：dev 滚动与 stable 正式版自更新、镜像腿与缺省回退（S028/D16/D48）。
pub mod update;
/// `hst agents verify`：四家无头验收两层判据（D17/S033）。
pub mod verify;
/// `hst init --yolo`：四家分级落盘、ours 退役与 pretrust（D33/D52）。
pub mod yolo;

/// 测试共享 env 互斥：动 HST_ROOT / HST_USER_HOME / SOPS_AGE_KEY_FILE 等
/// 进程级环境变量的测试跨模块也要互斥（各自局部锁挡不住并发互踩）。
#[cfg(test)]
pub(crate) mod testenv {
    pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
