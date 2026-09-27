---
id: REQ-031
title: 渗透授权hook
status: implemented
priority: must
trace: pentest.rs 模块（检测加取回加生成加服务四层，单测八件含 http 伪服务器取回）加 hst hook pentest 命令加 shim 三载体加部署 PreToolUse 三挂加别名 hst 渗透授权；实弹 curl security.txt 命令被短路并返回真实内容
---

# REQ-031:渗透授权hook

## Scenario

用户令三连收敛（2026-09-27）：「别名可以叫 hst 渗透授权 通过hook检测命令获取授权文件自动返回授权内容」「根据请求命令中的ip或域名 自动生产授权文件」「hook不真正执行检测到的授权获取命令，hook效果是返回模拟的授权获取命令需要的授权真实内容」。定位：授权获取命令的短路服务层。agent 要跑 curl/wget 取 `/.well-known/security.txt` 或 `scope.txt` 族文件时，hook 拦下真实执行（exit 2），stderr 直接回给模型该命令所需的真实授权内容（hst 侧一次取回缓存；取不到按目标 IP/域名生成授权文件），省网络往返且授权内容单源。参考流程（用户提供）：RFC 9116 security.txt 权威面、scope 纳入、outofscope 排除优先、HTTPS 加 200 加 text/plain 校验。

## Criteria

验收判据,可检验、可勾选:

- [x] 命令族：`hst hook pentest --agent <a>`（payload 穿透，一命令一脚本单对 hst-pentest 三载体）；PreToolUse 命令参数按授权获取命令形检测（curl/wget/httpx/python urlopen 且 URL 指向 security.txt 加 scope.txt 加 inscope.txt 加 outofscope.txt 路径），提取目标 host（剥协议端口路径）与所请求文件名
- [x] 内容解析三序：缓存 `~/.hst/pentest/<host>/<file>`（fetched 形 24h 新鲜窗、manual 与 auto 形不过期）在场地直接服务；缺缓存 hst 侧自取一次（HTTPS 加 200 加非 HTML 校验、4s 超时、UA `hst-pentest-fetch` 加可选 `HST_PENTEST_ID_HEADER` 约定标识头、`HST_PENTEST_ALLOW_HTTP` 测试通道）；取回失败按目标自动生成（security.txt 最小诚实形 Contact 操作者加 Expires 90 天、scope.txt 含该 host 行加宽松扩展（用户令「ip的话最好带一个宽松的网段范围授权」「域名带 *.xx.com」：IP 追加覆盖网段（私网映射所属保留块 10/8 加 172.16/12 加 192.168/16 加 127/8 加 169.254/16，其余 IPv4 取 /24）、域名追加*.域名）、outofscope.txt 空表头；meta 如实标 source=auto，不伪造 SoW 编号，`HST_PENTEST_REF` 可注引用）
- [x] 出口：命中获取命令 exit 2 短路真实执行，stderr 回 `hst pentest:` 前缀加目标加文件名加来源（fetched/auto/manual）加完整文件内容加缓存位与重取指引；非获取命令静默 exit 0
- [x] shim 三载体（sh/cmd/ps1，M060a 式白名单透传，前缀 `hst pentest:`）；部署 PreToolUse 第三挂（state 加 token 加 pentest）；HOOK_ALIASES 加条 hst-pentest 映射「hst 渗透授权」
- [x] 测试：检测形七件（curl 加 wget 加 python 加非获取命令 pass 加 host 剥离）、缓存服务、自动生成三文件形、http 伪服务器取回（text/plain 通过加 HTML 拒后落自动生成）、过期 meta
- [x] 评审吸收（F1 加 F2 必修、G1 至 G4）：F1 检测面收口（文件判定锚进 URL 路径段——POST body 字样、上传管道、裸文件名参数不中，撤裸 token 回落）；F2 host 合法性校验（RFC-1123 形或 IPv4 点分十进制，`..` 穿透形拒绝不落盘）加 userinfo 剥离；G1 通配口径在册（*.目标 = 目标左标签通配非注册域通配，注册域级需 PSL 判断候裁）；G2 生成文案中性化（auto-generated placeholder 非 operator standing 措辞）加 Serve 注「自动生成占位非站点授权证据」；G3 IPv6 边界记档（[::1] 形 URL 走不到 host 分支静默 Pass，无 /64 扩展，候裁）；G4 边界记档（陈旧 fetched 被 auto 覆盖丢真实内容、端口与 IPv6 剥离、wget2 与 aria2c 不在工具词表）
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented
