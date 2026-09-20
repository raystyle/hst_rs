# hst-cli::ledger

统一 issue 入口客户端面（issues.ohmygh.com，REQ-057 对齐）。
hst ledger 面（REQ-063 Phase 3；总台修正令 2026-09-20 权限收口）：
客户端实现统一走 `ledger-client` crate（ledger-rs v0.1.0，舰队唯一实
现，签名道加只增面全在其中），本模块只剩仓身份加密档管理加本地快检
加家族读序三件。**权限只增**：issue new/list/show 加 artifact
publish/attest（attest_dev 加 attest_prod 加 verification_failed 三验
证型）/list；close/delete 与 status 推进加 promote/demote/supersede 归
omc 工作台（开发工作台经 herdr 委托执行）。私钥密档
`~/.hst/ledger/ed25519.key`（hex 32 字节，0600）或 env
`HST_LEDGER_PRIVATE_KEY`（hex），不进仓不进 argv。

## Functions

- `clamp_issue_limit` — issue list 的 limit 钳制（1 至 100，服务端上限；#52 家族标准）：钳制
- `client` — 构造标准客户端（ledger-client crate）：env 优先次密档。
- `client_readonly` — 只读客户端（GET 面免钥，评审 F2 加 F4）：crate `read_only` 构造
- `issue_list_saturated` — issue list 饱和判定（#52 家族标准）：返回条数不少于钳制后 limit 即示
- `issue_list_truncation_hint` — issue list 饱和提示行（#52/#53 家族标准）：返回条数打满钳制后 limit
- `key_id` — kid = sha256hex(规范化 JWK)，与 ledger-client 派生约定一致；常量本身
- `keygen_write` — 密钥对生成（一次性或轮换）：自采 32 字节 seed（getrandom）写私钥密档
- `pairing_ok` — 本地私钥与内置公钥 JWK 的配对自检：密档/env 缺位回 None（无法判），
- `private_key_path` — 私钥密档路径（`~/.hst/ledger/ed25519.key`，内容 = hex 32 字节 seed；
- `validate_digest` — digest 本地快检（服务端 DIGEST_RE 同源，不触网秒红）：`sha256:<64hex

## Constants

- `ISSUE_KINDS` — issue kind 集（服务端 ISSUE_KINDS 同源，本地快检面）：bug（BUG 错误
- `PUBKEY_JWK` — 本仓公钥 JWK（字母键序紧凑形；与总台在册 kid 对应）。配对自检用，
- `REPO_ID` — 本仓账本身份（REQ-063：repo_id = 规范化 remote）。

