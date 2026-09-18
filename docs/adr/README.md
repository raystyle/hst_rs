# ADR 索引

> 架构决策记录:不可逆技术选择先立 ADR 再动手。新建拷 0000-template.md,编号接当前最大号,退役不复用。状态流转 proposed 到 accepted 到 superseded;supersede 须两篇互指。历史方案全文与 P 编号归档已随 ADR-0006 清退出仓（git 历史可考），ADR 承接仍约束现状的决策。

| id | 状态 | 标题 | 替代 |
|---|---|---|---|
| ADR-0001 | accepted | 产品定位Agent全平台部署配置与诊断工具 | 替代 P0004 旧编排定位 |
| ADR-0002 | accepted | hook注册与状态通道常驻用户级 | |
| ADR-0003 | accepted | Windows构建切gnu交叉编译摆脱VC | |
| ADR-0004 | superseded | self-update镜像腿与缺省回退 | 被 ADR-0008 承接（缺省读序翻镜像优先） |
| ADR-0005 | accepted | skill面退役发现通道收敛llms | 替代 D22/D49 两级 skill 面 |
| ADR-0006 | accepted | 文档体系完整重构契约回归代码与投影 | 取代 09-15 guide/guides 并存裁定，清退 references/G/M/proven 老文档层 |
| ADR-0007 | accepted | 发布流水线对齐build-release标准 | 三段式产地迁移（本地编译打包加 gh 直发），dev 轻岗豁免与读序豁免入册（总台核准单 2026-09-17） |
| ADR-0008 | accepted | selfupdate家族统一标准对齐 | 承接 ADR-0004；镜像优先缺省、官方腿锚校验、自替换锁与自证回滚、管理方让位（REQ-013，总台家族标准轮 2026-09-18） |
