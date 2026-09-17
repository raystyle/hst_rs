# hst-cli::agents

四家 agent 探测（PATH、env、hst 自管根、默认目录四源）。

## Functions

- `detect` — agent 探测的detect面（细则见模块文档与集成测试）。
- `find` — 按名探测一家 agent：PATH、`HST_AGENT_PATH`、`HST_<AGENT>_BIN`、hst 自管根与默认目录五源；命中即返回安装位与版本。
- `print_reports` — agent 探测的print_reports面（细则见模块文档与集成测试）。

## Types

- `Hit` — agent 探测的Hit面（细则见模块文档与集成测试）。
- `Probe` — Search roots used by `detect`. Tests inject dirs instead of reading the process env.
- `Report` — agent 探测的Report面（细则见模块文档与集成测试）。
- `Source` — agent 探测的Source面（细则见模块文档与集成测试）。

## Constants

- `DEFAULT_AGENTS` — Default agents this orchestrator knows how to spawn.

