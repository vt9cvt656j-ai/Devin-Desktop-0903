# 落库字段加密（Data at rest）

`src/field_crypto.rs` + `src/field_backfill.rs`。2026-08-15 已上线生产。

## 挡的是谁

**拿到数据库的人**：一次拖库、一份误发的备份、一个越权的 DBA。他们手里有整张表，但只要
`FIELD_ENC_KEY` 不在库里，敏感字段对他们就是一串 `fc1:...`，标准库再全也解不开——解密密钥
根本不在他拿到的那份东西里。

这是整套系统里**唯一**一处「别人的标准库真的解不开我的数据」字面成立的地方，成立的原因和
算法无关，只和**密钥托管**有关：密钥在进程环境里，不进 Postgres、不进任何备份。

挡不住能读进程内存/环境变量的人（那是完全的主机沦陷），也挡不住业务逻辑本身（handler
解密后照常用）。要对能调 handler 的攻击者收紧，靠权限判定，不是这一层。

## 加了哪些字段

只加密**从不按值查询、只存储+回显**的敏感字段（加密后每次 nonce 不同，`WHERE x=$1` 会失效）：

| 列 | 泄露后果 | context（AAD） |
| --- | --- | --- |
| `models.api_key` | 别人烧你的上游模型钱 | `models.api_key` |
| `connected_accounts.access_token` | 读用户的私有仓库 | `connected_accounts.access_token` |
| `connected_accounts.refresh_token` | 同上，可续期 | `connected_accounts.refresh_token` |
| `withdrawals.account` | 支付路由信息（PII） | `withdrawals.account` |
| `withdrawals.qr` | 收款码（PII） | `withdrawals.qr` |

**没加密**的：`email` / `api_keys.api_key` / prefix token——它们要按值查（登录、鉴权），需要
确定性加密或盲索引，改动大、风险高，是单独一大步。`password_hash` 是 bcrypt 哈希，本就不可逆，
不碰。

## 密码学

AES-256-GCM，随机 96-bit nonce。存储格式 `fc1:<base64url(nonce||ct||tag)>`。context（列身份）
绑进 AAD，所以一段密文搬到别的列就解不开——杜绝「把 A 的令牌密文塞进 B 列」。复用 MSE 那套
`aes-gcm`，不引新依赖。

## 迁移安全：带版本前缀 + 读时两种都认

- 密文存 `fc1:...`；没这前缀的一律当**遗留明文**原样读。
- 没配 `FIELD_ENC_KEY`：`encrypt` 直接返回明文，全链路**零行为变化**（可安全先上线）。
- 配了：新写入加密，旧行仍明文、读时照常认——**没有 flag-day**。
- 启动时 `field_backfill` 后台跑一次，把存量明文逐行加密回去：**幂等**（跳过已 `fc1:` 的）、
  **条件更新**（`WHERE col=<旧值>`，不覆盖并发写）、逐行独立不开大事务。

上线用两阶段降风险：先只发代码不配密钥（passthrough，证明代码没弄坏东西），再配密钥重启
（回填加密存量）。生产验证：所有 9 条 model key 都解出合法 `sk-...`，真实聊天成功计费，
拖库视角看到的是 `fc1:...`。

## ⚠️ 密钥就是一切

**回填之后 `FIELD_ENC_KEY` 丢了 = 那些字段永久无法恢复。** 生产密钥已生成并离线备份在服务器
`/root/FIELD_ENC_KEY.backup` 和 `/root/.mrday-scratch/FIELD_ENC_KEY.backup`（两份）。**必须再
拷一份到服务器之外**，而且和数据库备份**分开**存——否则「密钥不在拖出来的那份东西里」这个
前提就没了。

## 回滚

回填是**提交点**：一旦加密，reverted 的旧代码会把 `fc1:...` 当明文令牌发出去。真要回滚，得写
一个反向回填把所有行解密回明文（`FIELD_ENC_KEY` 还在就能做）。所以先发代码后配密钥的两阶段
很重要——阶段 A 的代码问题在 passthrough 下零数据变更就能发现。

## 加新字段

1. 在读写点用 `field_crypto::encrypt(x, "table.col")` / `decrypt(...)`，context 用列的全名。
2. 只对**不按值查**的列做。要查的列不行。
3. 在 `field_backfill.rs` 加一条回填。
4. 回显路径（给用户/管理员看）用 `decrypt_or_raw`（解不开也让页面渲染）；拿去做认证的路径用
   `decrypt` 的 Err 让请求明确失败，别把 `fc1:...` 当凭据发出去。
