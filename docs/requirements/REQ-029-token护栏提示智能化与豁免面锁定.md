---
id: REQ-029
title: token护栏提示智能化与豁免面锁定
status: implemented
priority: should
trace: secretguard 单测三件新增（位置词分类、provider 前缀去重、env 引用豁免语料七形加明文对照组）加实弹（假 key 经 hst-token.sh 阻断单条中文位置词句式 exit 2）；全测 256 加 49 绿
---

# REQ-029:token护栏提示智能化与豁免面锁定

## Scenario

用户令（2026-09-27）：「hst token护栏 提示智能一些 比如拦截提示命令参数中检测到token，文本读取中检测到token；另外token写入到文件的场景，如果是不接触token明文是环境变量或命令功能代码读取token后透传，不是agent打开文件明文读取的情况都不应该拦截」。现状：拦截原因句式是英文工具名定位（`Anthropic API Key in Bash command`），provider 前缀与通用 sk- 前缀双报同一密钥；env 引用形态（`$ANTHROPIC_API_KEY`、`Bearer ${KIMI_API_KEY}`、`os.environ['KEY']`）按正则字符类天然不中，但无测试锁定该豁免面。

## Criteria

验收判据,可检验、可勾选:

- [x] 位置词智能化：scan_text 按工具与事件分类（shell 类取 command 字段为「命令参数」、read 类与 PostToolUse 回读为「文本读取」、write/edit/notebook 类取内容字段为「文件写入」（content 加 new_string 加 edits 数组串）、其余「工具输入」、UserPromptSubmit 为「提示词文本」）；原因句式改「在{位置}中检测到{标签}（掩码）」
- [x] 双报去重：provider 专属前缀（sk-ant- 加 sk-proj- 加 sk-svcacct- 加 sk_live_ 加 sk_test_）命中时通用 sk- 形不再重复报（防线 1 收口）
- [x] 豁免面锁定：env 引用与透传七形语料（Bearer $VAR、export API_KEY=$VAR、printenv、echo "$VAR"、aws configure set、python os.environ、Bearer ${VAR}）零 findings 零 block；明文对照组仍拦
- [x] M060a 白名单兼容：stderr 前缀 `hst secretguard:` 不变（shim 透传判据不受影响）
- [x] 已知边界（评审 G2 加 G3）：write/edit 扫描面有意收窄（只扫 content 加 new_string 加 edits，old_string 与其余字段不扫；notebook 无内容字段回落整体序列化）；工具名子串匹配可能给第三方工具贴错位置词（只影响提示文案不影响扫描面）
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
