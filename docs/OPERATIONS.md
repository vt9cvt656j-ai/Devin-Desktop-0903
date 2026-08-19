# 运维手册：两套环境、部署、以及 API key 的存储迁移

这份文档解释**为什么这么设计**，而不只是罗列命令。照着敲之前先读对应那一节的理由——
这里每一条约束背后都有一次踩过的坑。

---

## 一、两套环境

| | 生产 | 测试 |
|---|---|---|
| compose 项目名 | `server` | `server-test` |
| 服务器目录 | `/opt/michael-ide-deploy/server` | `/opt/michael-ide-deploy/server-test` |
| 宿主端口 | `127.0.0.1:8080` | `127.0.0.1:8081` |
| 配置文件 | `.env` | `.env.test` |
| 数据库卷 | `server_pgdata` | `server-test_pgdata` |
| 静态站目录 | `/var/www/michael-sites` | `/var/www/michael-sites-test` |
| 对公网 | nginx 代理 → `code.mrday.one` | **不代理**，只能 SSH 端口转发 |

### 部署

```bash
cd server

# 生产
SERVER_HOST=154.44.13.133 SERVER_KEY=~/.ssh/michael_server bash deploy.sh

# 测试
TARGET=test SERVER_HOST=154.44.13.133 SERVER_KEY=~/.ssh/michael_server bash deploy.sh
```

`SERVER_KEY` **必须**指定。默认的 `id_rsa` 会被服务器拒绝，报的是 `Permission denied
(publickey)` —— 看起来像密钥失效，其实只是用错了那把。

访问测试环境（它不对公网开放）：

```bash
ssh -L 8081:127.0.0.1:8081 -i ~/.ssh/michael_server root@154.44.13.133
# 然后本机浏览器开 http://127.0.0.1:8081/health
```

### 隔离靠什么，为什么是这三样

1. **compose 项目名**决定容器名、网络名和**卷名**的前缀。这是数据隔离的根：
   `pgdata` 在两个项目下分别是 `server_pgdata` 和 `server-test_pgdata`，是两块盘。
2. **`REMOTE_DIR`** 是 rsync 的落点。两套用同一个目录的话，一次测试部署就把生产的代码
   覆盖掉了。
3. **`BACKEND_PORT` / `SITES_DIR`**（在各自的 env 文件里）决定宿主端口和静态站目录。

> ⚠️ 端口和静态站目录走的是**环境变量**，不是在 `docker-compose.test.yml` 里覆盖。
> 原因：Compose 对 `ports` 这类**列表**字段的合并语义是**拼接而不是替换**。在覆盖文件里
> 写一行 `- "127.0.0.1:8081:8080"`，最终配置会同时发布 8080 和 8081 —— 测试环境
> 直接和生产抢端口，两套一起崩。用变量完全绕开这个语义。

`docker-compose.test.yml` 是**覆盖层**，不是第二份完整配置。两份完整配置一定会漂，而这里
漂的后果特别坏：测试和生产不一致时，"在测试上验过了"就成了一句假话，比没有测试环境更危险。

### 在测试环境上做什么

- 验证迁移能跑通、能回滚
- 验证改动在**真实 Postgres**上的行为（本地 `cargo test` 不连库）
- 压测、调参、试危险操作

测试环境的数据可以随时删光：

```bash
ssh ... 'cd /opt/michael-ide-deploy/server-test && \
  docker compose -p server-test -f docker-compose.yml -f docker-compose.test.yml down -v'
```

`-v` 会**连卷一起删**。对 `server-test` 是安全的；对 `server` 就是删库。

---

## 二、API key 的存储迁移（分两次部署）

### 为什么要改

`api_keys.api_key` 原来是明文列 + 一条建在明文上的索引。任何一次只读的库暴露（备份、
只读副本、`pg_dump` 误配、一条 SQL 注入）都直接产出一批**可立即使用**的网关凭据：
持有者以受害者身份跑 `/v1/chat/completions`，费用记受害者钱包，而且允许记成负债。

同一份代码对上游供应商的 key 一直是加密存的（`field_crypto`），管理端还做了 mask ——
标准不一致本身就是这条的证据。

### 为什么是两列，不是"存哈希就完了"

`GET /api/ide-key` 要把**同一把 key 原样还给登录用户**（IDE 自动配置，跨设备跨会话必须
稳定）。哈希是单向的，只存哈希这个接口直接废掉。所以：

- `api_key_sha256` —— 校验用。确定性，可建唯一索引，鉴权仍是一次索引命中。单向。
- `api_key_enc` —— 回显用。`field_crypto` 加密（随机 nonce，不可索引）。只有 ide-key 解它。

### 第一次部署：加列 + 回填（用户完全无感）

代码合并后正常部署即可，**不需要任何额外开关**。会发生：

1. 迁移 `20260844_api_key_at_rest.sql` 只**加列**，新列可空，明文列改成可空并去掉唯一约束。
   → 回滚到旧二进制时旧代码照常读 `api_key`，一切照旧。
2. 启动后台回填给存量行补上 `api_key_sha256` 和 `api_key_enc`（幂等、逐行条件更新）。
3. 鉴权改成**先查哈希、查不到再查明文**，命中明文时顺手把那一行补齐（self-healing）。
   → 回填还没跑完、或滚动发布期间旧二进制刚写入的行，都不会认证失败。
4. 新签发的 key **从一开始就不落明文**。

先在测试环境跑一遍，确认：

```sql
SELECT count(*) FILTER (WHERE api_key_sha256 IS NULL) AS 待回填,
       count(*) FILTER (WHERE api_key_enc LIKE 'fc1:%') AS 已加密,
       count(*) AS 总数
FROM api_keys;
```

`待回填` 归零、`已加密` 等于 `总数`，才算这一步做完。

### 第二次部署：清除明文

**确认第一步无恙之后**，在 env 里加一行再部署一次：

```
API_KEY_PURGE_PLAINTEXT=1
```

启动 30 秒后会把已经补齐哈希和密文的行的 `api_key` 置 NULL。

> ⚠️ **必须先配好 `FIELD_ENC_KEY`。** `field_crypto` 在没有密钥时是 passthrough
> （原样返回明文，这是它有意的"不配 = 零行为变化"设计），于是 `api_key_enc` 里存的其实
> 还是明文。这时候清掉 `api_key` 一点安全收益都没有，只是把同一份明文从一列搬到另一列
> —— 而且给人"已经处理好了"的错觉。代码里已经拦住了这种情况（没密钥直接拒绝清除并
> 打 error 日志），但你应该在跑之前就知道为什么。

清完之后可以删掉明文列和它的旧索引（第三次部署，可选）。

### 回滚

- 第一步之后回滚：直接换回旧二进制。明文列还在，旧代码照常工作。
- 第二步之后回滚：**明文已经没了**，旧二进制读不到 key。只能靠 `api_key_enc` 解密回填，
  或让用户重新取一次 key。所以第二步之前一定要在测试环境验过。

---

## 三、几条踩过的坑

- **`deploy.sh` 曾经在失败时报成功**：`if ssh …; then return 0; fi` 之后取 `$?` 拿到的是
  **if 语句**的退出码，条件失败且没有 else 时它是 0。已修，但同类写法别再引入。
- **满盘会让部署假成功**：反复构建塞满磁盘后 `up --build` 失败，而旧容器还在健康跑着，
  迁移没跑却像成功。定期 `docker builder prune -a`。
- **只 `docker cp` 不写源码树**：下次重建镜像就悄悄回滚，而且不报错。改什么都要落到仓库里。
