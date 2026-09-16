---
id: ADR-0005
title: skill面退役发现通道收敛llms
status: accepted
date: 2026-09-16
deciders:
  - raystyle
supersedes: []
superseded_by: None
tags: ['skill', 'D54', 'llms.txt']
---

# ADR-0005:skill面退役发现通道收敛llms

## Context

两级 skill 面与 aidoc llms.txt 并存：D22/D49 的 `hst skill`（clap 活命令树渲染，用户级 `~/.claude/skills/hst/`）加 init 项目级 COMMAND_MAP fan-out（`.agents` 等四目录）；cargo aidoc 出的 `docs/aidoc/llms.txt`（dev-evo 第五十九批 ADR-0006 强制投影）已承载仓内发现通道（dev-evo 终态对照表在册）。仓内会话里说明书三层重复（AGENTS 到 docs README 到 R002 已全覆盖）。2026-09-16 用户裁定（D54）：有 llms.txt 即不需要 skill 面，全删。事实边界已核实：llms.txt 是库面 API 投影且只在本仓，skill 是仓外装机运行时发现面，删 skill 即放弃仓外发现，用户知悉并接受。

## Decision

删 `hst skill` 子命令与 init 项目级 skill fan-out（COMMAND_MAP、skill_md、write_skill、deploy_skills、kimi skill 布局整层）；不再有任何 hst 技能安装面。agent 紧凑说明书以 `hst --llms` 一条命令出口：从 clap 活命令树自适应渲染 llms 风格命令速查直打 stdout（帮助面同款裸输出，不落盘不装技能）。init 幂等清扫 ours 技能目录（项目四目录乘 hst 与 ohmyagents 两名，加用户级 `~/.claude/skills/` 两名；marker 家族或生成签名判 ours，外来内容不动）；仓内细则唯一权威 R002，库面投影 aidoc llms.txt；AGENTS.md/CLAUDE.md 说明层（只增不覆写）保留。契约破裂升 major。

## Consequences

- 好：命令面与四处同步链收缩（SKILL 重生与 COMMAND_MAP 两步摘除）；说明书单一口径（R002 细则加 llms 投影），无双份生成面、零安装件维护。
- 坏：仓外装机侧不再经技能清单被动发现 hst（agent 需主动跑 `hst --llms` 或经项目 AGENTS 说明）；升 2.0.0，旧装机重跑 init 清扫在位技能件。
