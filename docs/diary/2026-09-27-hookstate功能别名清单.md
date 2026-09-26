# 2026-09-27：hookstate 功能别名清单

> 用户令（2026-09-26 晚至 09-27 晨三连令）：第5行 hookstate 格式对齐（图标后双空格）、显示 hook 了什么、最终裁定用 hook 功能别名清单（「比如隐私保护」「比如herdr hook，组织一个hook别名清单」）。承接 09-26 REQ-026 原生渲染批（四轮评审 CONFIRM 推 main）。

## 流水

1. **第5行从事件名清单改功能别名清单**（用户三连令收敛）：REQ-026 首版显示注册事件名（SessionStart 等八字件），用户裁定改 hook 功能别名制并点名 herdr hook 入列。实现 HOOK_ALIASES 别名表（hst-state 映射「状态通道」：四态写 ~/.hst/state 的通道本体，S025/D28 术语同源；herdr-agent-state 映射「舰队状态」：agent 态推 herdr server 的舰队可观测面）；未收录 stem 回落本名（hook_stem 取末个脚本形 token 的 basename 去扩展，已知 stem 子串直配），表序稳定加未收录字典序殿后，stem 去重；ours 与外来 hook 同列（上一版只列 ours，本版全列，用户点名 herdr 即外来）。模板 `{icon}  {alias} {state}`（图标后双空格对齐 hst 段形）。单测 hooked_aliases_list_known_foreign_and_fallback（表序加外来同列加未收录回落、kimi TOML 面、坏损 JSON、缺文件四断言面）。实弹：本工位第5行渲染 `状态通道 舰队状态 working`（本机 claude 注册面 hst-state 八事件加 herdr SessionStart 一挂，双别名去重后并显）`[实证: 本会话 id 直跑 release 加注册文件对读]`。过程坑复犯一次：python heredoc 写测试夹具 `\"` 被塌成裸引号致 JSON 坏损零命中（G2 在册坑，Edit 实落修正；此型已两犯，见自省）。全测 252 加 49 绿 `[实证: 本机全测输出]`。

2. **四令终裁与评审 F/G 一并吸收**（用户令「不需要working这些状态 显示hook功能的别名 参考 agent状态 token护栏 这些功能别名」）：模板改 `{icon}  {alias}`（态文本退出缺省显示仅以行色暗示，{state} 占位符保留自配）；别名定名 hst-state 映射「agent状态」（用户点名，事件映射四态写 ~/.hst/state 的功能本体）、herdr-agent-state 映射「会话上报」（读 hook 本体实证：SessionStart 把会话登记推 herdr server 的 pane.report_agent_session，非 token 检测面；本机无任何 token 检测类 hook，第2行 token 百分比来自状态栏 payload 非 hook）。评审快核回执吸收：F（grok 多文件注册面：本机 herdr 的 grok 挂载在 ~/.grok/hooks/herdr.json 而 hst 在 ohmyagents-state.json，单文件映射漏列外来 hook）即修：grok 臂全目录 *.json 排序合并收集；G1 加 G2（stem 提取扩展白名单漏 .bat/.exe/.py/.js 载体、末 token 启发式在参数位脚本路径时误取）即修：解释器跳过后取首个「分隔符在首字符之后或含点」token 的 basename 去末扩展（`/c` 与 `-File` 旗标形不误判，自查实弹修正一次）；G3 采纳（HOOK_ALIASES 旁注新增 hook 收录指引与 grok 多文件提醒）。单测两件九断言面（别名清单四态加 grok 双文件、stem 启发式七形含 bat 反斜杠与旗标形）。实弹：本工位 claude 面第5行 `agent状态 会话上报`、grok 面双文件合并同清单 `[实证: 双面直跑 release 加注册文件对读]`。全测 253 加 49 绿 `[实证: 本机全测输出]`。

## 自省

- python heredoc 转义塌陷同型二犯（09-26 流水 18 的 \n 塌真换行、本次 \" 塌裸引号）：夹具类改码一律 Edit 工具实落或 python raw 字符串，不再走普通字符串 heredoc。按「同型二犯升格」惯例此条记档待升 guides 工作流条目。
