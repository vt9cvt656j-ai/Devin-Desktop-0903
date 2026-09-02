---
name: deploying-server
description: How to deploy the server/ gateway — the separate test and production environments, what isolates them, and the two-phase API key storage migration. Read this before touching docker-compose, deploy.sh, .env, or anything under server/migrations.
---

# 部署 `server/` 网关

完整说明在 [`docs/OPERATIONS.md`](../../../docs/OPERATIONS.md)。这里是动手前必须知道的部分。

## 先看这里：当前实际状态（2026-08-19）

- **测试环境当前是拆掉的**（容器/卷/目录全删，拆前确认过库是空的）。`TARGET=test` 那条路
  代码没动，随时可以重新拉起来一套——下面写的隔离规则在重建时仍然全部适用。
- **API key 存储迁移第一步已上生产**：13 个 key 全部完成哈希+密文回填。**第二步
  （`API_KEY_PURGE_PLAINTEXT=1` 清除明文）还没做**，什么时候做要有人明确决定。
- **远端判断一律用 `ssh_true`，不要用 `ssh_run`。** 这台机器的 SSH 握手会随机掉
  （`banner exchange ... invalid format`），而 ssh 的 255 是它自己失败、不是远端命令的
  返回值。混用会把"连不上"报成"远端没有这个文件"——已经真实发生过一次，差点照着提示
  把生产的 `.env` 覆盖掉。

## 有两套环境，别搞混

```bash
cd server

# 生产 —— 真实用户在用
SERVER_HOST=154.44.13.133 SERVER_KEY=~/.ssh/michael_server bash deploy.sh

# 测试 —— 随便折腾
TARGET=test SERVER_HOST=154.44.13.133 SERVER_KEY=~/.ssh/michael_server bash deploy.sh
```

`SERVER_KEY` 必须显式给。默认的 `id_rsa` 会被拒，报 `Permission denied (publickey)` ——
看着像密钥失效，其实是用错了那把。

| | 生产 | 测试 |
|---|---|---|
| 项目名 | `server` | `server-test` |
| 端口 | 8080（nginx 代理到 code.mrday.one） | 8081（**不对公网**，走 SSH 转发） |
| 配置 | `.env` | `.env.test` |
| 数据库卷 | `server_pgdata` | `server-test_pgdata` |

## 三条硬规矩

1. **改了 `server/` 的代码就要重新部署**，否则线上跑的还是旧镜像。
   验证：`curl -s https://code.mrday.one/health`，以及比对容器里的源码 md5。

2. **危险的改动先上测试环境。** 尤其是：数据库迁移、计费逻辑、鉴权逻辑。
   本地 `cargo test` 不连真实 Postgres，迁移能不能跑通只有真环境说了算。

3. **端口和静态站目录走环境变量，不要在 `docker-compose.test.yml` 里覆盖。**
   Compose 合并 `ports` 这类列表字段时是**拼接不是替换** —— 在覆盖文件里写一行
   `- "127.0.0.1:8081:8080"`，最终配置会同时发布 8080 和 8081，测试环境直接和生产抢端口。
   `docker-compose.test.yml` 是**覆盖层**不是第二份完整配置：两份完整配置一定会漂，
   而漂了之后"在测试上验过了"就成了假话，比没有测试环境更危险。

## API key 存储正在迁移中（分两次部署）

`api_keys` 表现在有三列并存：

- `api_key_sha256` —— 鉴权用（唯一索引，单向）
- `api_key_enc` —— 回显用（`field_crypto` 加密；`GET /api/ide-key` 要把同一把 key 原样还给
  用户，所以不能只存哈希）
- `api_key` —— **过渡期遗留的明文**，第二次部署时清除

所有读写都走 `server/src/api_key_store.rs`，**不要**再写 `WHERE api_key = $1`。

清除明文需要显式设 `API_KEY_PURGE_PLAINTEXT=1` 再部署一次，而且**必须先配好
`FIELD_ENC_KEY`**：`field_crypto` 没密钥时是 passthrough（原样存明文），这时候清掉明文列
只是把同一份明文换了个列名，一点安全收益都没有还给人"修好了"的错觉。代码里已经拦住了
这种情况，但你该在跑之前就知道为什么。

## 部署完要确认它真的成功了

`deploy.sh` 历史上出过"失败报成功"：`if ssh …; then return 0; fi` 之后取 `$?` 拿到的是
**if 语句**的退出码，条件失败且没有 else 时它是 0。已经修了，但别再引入同类写法。

还有两个会让部署"假成功"的坑：
- **磁盘满** → `up --build` 失败，旧容器继续健康跑着，迁移没跑却像成功。
  定期 `docker builder prune -a`。
- **只 `docker cp` 不写源码树** → 下次重建镜像悄悄回滚，不报错。改什么都要落到仓库里。
