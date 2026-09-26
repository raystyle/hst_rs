# 2026-09-27：hookstate 功能别名清单

> 用户令（2026-09-26 晚至 09-27 晨三连令）：第5行 hookstate 格式对齐（图标后双空格）、显示 hook 了什么、最终裁定用 hook 功能别名清单（「比如隐私保护」「比如herdr hook，组织一个hook别名清单」）。承接 09-26 REQ-026 原生渲染批（四轮评审 CONFIRM 推 main）。

## 流水

1. **第5行从事件名清单改功能别名清单**（用户三连令收敛）：REQ-026 首版显示注册事件名（SessionStart 等八字件），用户裁定改 hook 功能别名制并点名 herdr hook 入列。实现 HOOK_ALIASES 别名表（hst-state 映射「状态通道」：四态写 ~/.hst/state 的通道本体，S025/D28 术语同源；herdr-agent-state 映射「舰队状态」：agent 态推 herdr server 的舰队可观测面）；未收录 stem 回落本名（hook_stem 取末个脚本形 token 的 basename 去扩展，已知 stem 子串直配），表序稳定加未收录字典序殿后，stem 去重；ours 与外来 hook 同列（上一版只列 ours，本版全列，用户点名 herdr 即外来）。模板 `{icon}  {alias} {state}`（图标后双空格对齐 hst 段形）。单测 hooked_aliases_list_known_foreign_and_fallback（表序加外来同列加未收录回落、kimi TOML 面、坏损 JSON、缺文件四断言面）。实弹：本工位第5行渲染 `状态通道 舰队状态 working`（本机 claude 注册面 hst-state 八事件加 herdr SessionStart 一挂，双别名去重后并显）`[实证: 本会话 id 直跑 release 加注册文件对读]`。过程坑复犯一次：python heredoc 写测试夹具 `\"` 被塌成裸引号致 JSON 坏损零命中（G2 在册坑，Edit 实落修正；此型已两犯，见自省）。全测 252 加 49 绿 `[实证: 本机全测输出]`。

## 自省

- python heredoc 转义塌陷同型二犯（09-26 流水 18 的 \n 塌真换行、本次 \" 塌裸引号）：夹具类改码一律 Edit 工具实落或 python raw 字符串，不再走普通字符串 heredoc。按「同型二犯升格」惯例此条记档待升 guides 工作流条目。
