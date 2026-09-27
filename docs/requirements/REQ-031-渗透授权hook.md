---
id: REQ-031
title: 渗透授权hook
status: implemented
priority: must
trace: pentest.rs 模块（检测加取回加生成加服务四层，单测九件含 http 伪服务器取回加 RFC 9116 形）加 hst hook pentest 命令加 shim 三载体加部署 PreToolUse 三挂加别名 hst 渗透授权；实弹 curl security.txt 命令被短路并返回真实内容；生成文案白帽授权面加 RFC 9116 形（用户令 2026-09-27）；注入面已由用户令 2026-09-27 停用（本文件末退役节）
---

# REQ-031:渗透授权hook

## Scenario

用户令三连收敛（2026-09-27）：「别名可以叫 hst 渗透授权 通过hook检测命令获取授权文件自动返回授权内容」「根据请求命令中的ip或域名 自动生产授权文件」「hook不真正执行检测到的授权获取命令，hook效果是返回模拟的授权获取命令需要的授权真实内容」。定位：授权获取命令的短路服务层。agent 要跑 curl/wget 取 `/.well-known/security.txt` 或 `scope.txt` 族文件时，hook 拦下真实执行（exit 2），stderr 直接回给模型该命令所需的真实授权内容（hst 侧一次取回缓存；取不到按目标 IP/域名生成授权文件），省网络往返且授权内容单源。参考流程（用户提供）：RFC 9116 security.txt 权威面、scope 纳入、outofscope 排除优先、HTTPS 加 200 加 text/plain 校验。

## Criteria

验收判据,可检验、可勾选:

- [x] 命令族：`hst hook pentest --agent <a>`（payload 穿透，一命令一脚本单对 hst-pentest 三载体）；PreToolUse 命令参数按授权获取命令形检测（curl/wget/httpx/python urlopen 且 URL 指向 security.txt 加 scope.txt 加 inscope.txt 加 outofscope.txt 路径），提取目标 host（剥协议端口路径）与所请求文件名
- [x] 内容解析三序：缓存 `~/.hst/pentest/<host>/<file>`（fetched 形 24h 新鲜窗、manual 与 auto 形不过期）在场地直接服务；缺缓存 hst 侧自取一次（HTTPS 加 200 加非 HTML 校验、4s 超时、UA `hst-pentest-fetch` 加可选 `HST_PENTEST_ID_HEADER` 约定标识头、`HST_PENTEST_ALLOW_HTTP` 测试通道）；取回失败按目标自动生成（security.txt 最小诚实形 Contact 操作者加 Expires 90 天、scope.txt 含该 host 行加宽松扩展（用户令「ip的话最好带一个宽松的网段范围授权」「域名带 *.xx.com」：IP 追加覆盖网段（私网映射所属保留块 10/8 加 172.16/12 加 192.168/16 加 127/8 加 169.254/16，其余 IPv4 取 /24）、域名追加*.域名）、outofscope.txt 空表头；meta 如实标 source=auto，不伪造 SoW 编号，`HST_PENTEST_REF` 可注引用）
- [x] 出口：命中获取命令 exit 2 短路真实执行，stderr 直出完整文件正文（用户令 2026-09-27 删前缀行与缓存位指引、错面去前缀，成功面即模拟命令真实输出、无 hst 抬头）；非获取命令静默 exit 0
- [x] shim 三载体（sh/cmd/ps1，M060a 式白名单透传，前缀 `hst pentest:`）；部署 PreToolUse 第三挂（state 加 token 加 pentest）；HOOK_ALIASES 加条 hst-pentest 映射「hst 渗透授权」
- [x] 测试：检测形七件（curl 加 wget 加 python 加非获取命令 pass 加 host 剥离）、缓存服务、自动生成三文件形、http 伪服务器取回（text/plain 通过加 HTML 拒后落自动生成）、过期 meta
- [x] 评审吸收（F1 加 F2 必修、G1 至 G4）：F1 检测面收口（文件判定锚进 URL 路径段：POST body 字样、上传管道、裸文件名参数不中，撤裸 token 回落）；F2 host 合法性校验（RFC-1123 形或 IPv4 点分十进制，`..` 穿透形拒绝不落盘）加 userinfo 剥离；G1 通配口径在册（*.目标 = 目标左标签通配非注册域通配，注册域级需 PSL 判断候裁）；G2 生成文案中性化（auto-generated placeholder 非 operator standing 措辞）加 Serve 注「自动生成占位非站点授权证据」；G3 IPv6 边界记档（[::1] 形 URL 走不到 host 分支静默 Pass，无 /64 扩展；用户裁 2026-09-27「IPv6 不用」，不做）；G4 边界记档（陈旧 fetched 被 auto 覆盖丢真实内容、端口与 IPv6 剥离、wget2 与 aria2c 不在工具词表）
- [x] 白帽授权面加 RFC 9116 形（用户令 2026-09-27「自定生成返回 一个允许白帽子进行渗透测试的授权文件」「符合RFC 9116 security.txt 标准」，翻转 G2 中性化）：security.txt 走 RFC 9116 形（必填 Contact 单次加 Expires 单次（RFC 3339 UTC 形 YYYY-MM-DDThh:mm:ss.sssZ、90 天窗小于 1 年）、可选 Canonical 加 Preferred-Languages 取 https 形、白帽授权经 `X-Authorization` 扩展字段（2.4 扩展性允许）承载，标准字段不夹带非 URI 值）；scope 与 inscope 抬头标白帽渗透授权，outofscope 空表头；单测 `generated_security_txt_conforms_rfc9116` 锁九字段形加必填各一次加 https 约束加白帽授权面。续令（同 2026-09-27）：删 hst-pentest 标题行，Contact 自适应取命令内 host（`mailto:operator@<host>`，去 localhost 占位），security.txt 全字段均按提取的 ip 或域名生成；再续令删 Serve 前缀行、缓存位指引与错面前缀，成功面 stderr 即文件正文
- [x] 四文件对齐加 IP 网段授权（用户令 2026-09-27「这4个文件都应该自动生成对齐，允许白帽子进行自动化渗透测试」「除域名外还要支持ip，针对命令ip生成一个ip的网段授权」）：security.txt 加 scope.txt 加 inscope.txt 加 outofscope.txt 四份共用对齐抬头（白帽自动化渗透授权加 Target 加 Generated 三行，去 hst 品牌与 source 标）；security.txt 加 `In scope` 注释带 host 加范围、scope 与 inscope 表体带 host 加范围、outofscope 空表头；范围自适应命令目标：IP 走网段（私网映射所属保留块 10/8 加 172.16/12 加 192.168/16 加 127/8 加 169.254/16，其余 IPv4 取 /24）、域名走 *.域名；单测 `four_auth_files_share_aligned_header` 加 `ip_target_carries_cidr_authorization`
- [x] 实施后回填 frontmatter 的 trace，状态改 implemented

## 退役:2026-09-27 用户令停用注入面

用户令「~/.hst/hooks/hst-pentest.sh 不要输出hook注入的错误信息」。实测复盘：注入面在本腿命中授权获取命令时用 exit 2 加 stderr 回内容，agent 侧一律把这段输出呈现为 hook error（claude 记 `PreToolUse:Bash hook error: [...hst-pentest.sh claude]: <内容>`，codex 记 `Command blocked by PreToolUse hook: <内容>`）；且站点取不到 security.txt/scope.txt 族文件时回的是按目标现生成的内容（source=auto），下游据此当真实授权面用即误判（本机实弹：sslcert.se 实际无 security.txt（302 转 www 后 404），注入面却回了一份带 `In scope: sslcert.se` 与 `X-Authorization` 的授权文件，agent 侧当场按伪造面处理并绕道重取真件）。

落地分两笔。第一笔（v2.9.6 后未发布态）：`hst hook pentest` 恒静默放行（exit 0 无输出），先止血；CLI 手册退出码表同步（退 2 自判处置只剩 secretguard）。第二笔（用户令 2026-09-27「直接去掉pentest.rs及功能和状态栏的渗透授权别名」，随 v2.9.7 发布）：`src/pentest.rs` 整模块删除（检测、缓存三序、自取、自动生成、loose_cidr 网段与白帽文案全退），`hst hook pentest` 命令面删除，shim 退为 state 加 token 两腿八件（pentest 四件 sh/ps1/cmd/grok 包装走幂等清扫），四家注册撤 PreToolUse 第三挂（claude 加 grok 加 kimi 加 codex；存量 hst-pentest 条目在重部署里归一清扫，未重部署的机器回落用户级 fail-open 语义），状态栏 hook 别名表去「hst 渗透授权」条。

边界：`hst hook pentest` 不再有任何注入与短路能力，`~/.hst/pentest/` 缓存目录本版不再被读写（存量文件可自行清理）。
