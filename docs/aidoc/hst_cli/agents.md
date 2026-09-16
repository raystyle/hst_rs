# hst-cli::agents

四家 agent 探测（PATH、env、hst 自管根、默认目录四源）。

## Functions

- `detect` — detect：四家 agent 探测的公开入口（行为细则与 marker 见 R002）。
- `find` — find：四家 agent 探测的公开入口（行为细则与 marker 见 R002）。
- `print_reports` — print_reports：四家 agent 探测的公开入口（行为细则与 marker 见 R002）。

## Types

- `Hit` — Hit：四家 agent 探测的数据面。
- `Probe` — Search roots used by `detect`. Tests inject dirs instead of reading the process env.
- `Report` — Report：四家 agent 探测的数据面。
- `Source` — Source：四家 agent 探测的取值集。

## Constants

- `DEFAULT_AGENTS` — Default agents this orchestrator knows how to spawn.

