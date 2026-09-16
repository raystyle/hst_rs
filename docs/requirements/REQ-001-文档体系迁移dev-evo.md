---
id: REQ-001
title: 文档体系迁移dev-evo
status: implemented
priority: must
trace: PEVO_CHECK_ALLOW='^docs/aidoc/' uv run /mnt/wsl/repos/project-evo/plugins/evo-adr/skills/code-kit/scripts/check.py /mnt/wsl/repos/hst_rs（路径 2026-09-16 随二轮评审 G 修至现位，原录 /mnt/d/ProjectEvo 旧位与 /home/ray/hst_rs 旧根均已裁撤）
---

# REQ-001:文档体系迁移dev-evo

## Scenario

hst_rs 文档体系要从旧根原语（PRD/GOAL/PLAN/TODO/INDEX 加 docs 六目录）迁移到 dev-evo 文档即代码体系（AGENTS 五节合同加 ADR 加 REQ）。

## Criteria

- [x] dev-evo 骨架生成（init.py 幂等，存量件不覆盖）
- [x] AGENTS 重写为五节合同（旧四段留档 docs/guides）
- [x] 定位与架构级决策择要转 ADR 四件（指针承接不搬运）
- [x] 活跃队列转 REQ、历史归档注记（迁移不是重审的对立面：历史不回填）
- [x] docs/README 地图承接 INDEX 职责、四原语顶部迁移注记
- [x] 门禁接入（check.py PE 全项加 rumdl 加三件套加 md-ref-scan 断链回归）
