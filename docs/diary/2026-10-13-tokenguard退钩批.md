# 2026-10-13 token-guard 全面退钩批 REQ-052

## 事

总台三令收敛(「默认关掉 hst-token-guard」「不 hook hst-token-guard」「只摘除 hst-token-guard 相关」)加追加令「不能有个开关直接关掉这个 hook」。2.14 批同步单(AI-LAB w13:p1 转)四件中本单只做 token 退钩(goal alias 与 hook goal 摘除收窄出单,插件纯名 237ad50 已在 main 随版携带)。lane/token-unhook,评审 hst-rev。

## 设计终形

- 默认退钩:缺省 init 不注册 token 腿(claude 只挂 task 四事件;codex 与 grok 与 kimi 零 hst 钩常态,deploy 转纯清扫道);存量经新 init 一趟清出(claude 非管理事件整弃加 codex 分侧外科摘键加 grok 空档删除加 kimi 表项清扫);shim 腿表 OFF 位只剩 task,hst-token 四载体入退役清扫;CLI hst hook token 保留手动面。
- 开关面:--token-guard 开(标记档 ~/.hst/token-guard.on,四家注册机件自 HEAD 恢复收在开关后)加 --no-token-guard 关并清出加缺省不动按现态幂等;运行时逃生闸 HST_TOKEN_GUARD=off 直通;doctor 认标记档(ON 机注册在场 Ok 不误报存量)。
- 判据随动:doctor hooks.form 他家零钩 Ok 与存量 Warn 引导重 init;statusline no-hook 哨兵仅 claude(他家零钩常态不哨,判据保 token 分量认未重 init 存量机);verify 部署道随开关读档。

## 证

- 单测 242 集成 63 绿(NO_COLOR=1 直跑验);主矩阵件与 shim 件与 codex 三件与 kimi 件随形重写;开关全链件 off 加 on 加 off 三态与标记档读写撤在测;集成 init_token_guard_switch_roundtrip_and_runtime_hatch 全链(缺省退钩加开位回归加关位清出加逃生闸)。
- 门禁:fmt 加 clippy(新码零告警)加 aidoc 加 md 四门禁绿。
- 途中:ledger 与 secretguard 三件曾随失败测试级联红(单跑绿,修完真失败件后全绿),测试并行干扰一笔记档。

## 次

goal alias 与 hook goal 与 HST_GOAL_* 摘除移后续批(收窄令);2.14.0 发布链与滚版矩阵归总台同步单执行序。

## 同车三件 总台连令

- L4 task 摘要:宽度感知预算(独占行,原 16 字档太抠)加换行连续空白并单空格加词边界截断(总台令「省略显式得太多了也没考虑换行内容合并成一行」)。
- task 按 agent 会话隔离(REQ-053):键 `<slug>--<agent>.json` 同项目互不可见,legacy 无尾档 claude 兼容读加首写迁移,CLI --agent 三态,状态栏按渲染 agent 取键。装位落差窗坑:shim 恒烘焙装位 exe,旧版 hook 不识新键,verify 判据档落 legacy 键位两态都认(活弹 rounds=2 复绿实证)。
- 收窄令「只摘除 hst-token-guard 相关」:goal alias 与 hook goal 与 HST_GOAL_* 摘除全保移后续批(曾动手即回滚)。
