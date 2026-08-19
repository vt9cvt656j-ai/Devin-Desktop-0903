//! 把**存量**明文敏感字段加密回去。
//!
//! `field_crypto` 让新写入变成密文、读时两种都认，但已经躺在库里的旧行仍是明文。
//! 拖库的人要的正是那些旧行。这个任务在启动时跑一次，把它们逐行加密。
//!
//! 安全性质：
//!   - **只在配了 `FIELD_ENC_KEY` 时跑**（没密钥没法加密，也没意义）；
//!   - **幂等**：只处理没有 `fc1:` 前缀的行，跑第二次什么都不做；
//!   - **条件更新**：`UPDATE ... WHERE col = <旧值>`，绝不覆盖一条并发写进来的新值；
//!   - **逐行独立**，不开大事务，跑到一半挂了也只是下次接着跑剩下的。
//!
//! 它在后台 spawn，不挡启动。量很小（模型十几行、连接和提现最多几百行）。

use crate::field_crypto;
use crate::AppState;

/// 在后台跑一次存量加密。配了密钥才跑。
pub fn spawn(state: AppState) {
    if !field_crypto::enabled() {
        return;
    }
    tokio::spawn(async move {
        // 稍等，让迁移和主要初始化先过去。它不急。
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if let Err(e) = run(&state).await {
            tracing::error!(error = %e, "字段加密回填出错（下次启动会接着跑剩下的）");
        }
    });
}

async fn run(state: &AppState) -> anyhow::Result<()> {
    let mut total = 0u64;
    total += backfill_by_uuid(state, "models", "api_key", "models.api_key").await?;
    total += backfill_connected_accounts(state).await?;
    total += backfill_by_uuid(state, "withdrawals", "account", "withdrawals.account").await?;
    total += backfill_by_uuid(state, "withdrawals", "qr", "withdrawals.qr").await?;
    total += backfill_api_keys(state).await?;
    if total > 0 {
        tracing::info!(rows = total, "字段加密回填完成");
    } else {
        tracing::info!("字段加密回填：没有遗留明文需要处理");
    }
    Ok(())
}

/// 通用回填：一张以 uuid `id` 为主键、单个文本列的表。
///
/// 列可空（withdrawals.qr）也能处理：`col <> ''` 顺带排除了 NULL 和空串，两者都不需要加密。
async fn backfill_by_uuid(
    state: &AppState,
    table: &str,
    col: &str,
    ctx: &str,
) -> anyhow::Result<u64> {
    // 表名和列名是本文件里写死的字面量，不来自任何外部输入，拼进 SQL 是安全的。
    let select = format!(
        "SELECT id, {col} FROM {table} WHERE {col} IS NOT NULL AND {col} <> '' \
         AND {col} NOT LIKE 'fc1:%' LIMIT 5000"
    );
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(&select).fetch_all(&state.db).await?;
    let update = format!("UPDATE {table} SET {col} = $1 WHERE id = $2 AND {col} = $3");
    let mut n = 0u64;
    for (id, plain) in rows {
        let enc = field_crypto::encrypt(&plain, ctx);
        // encrypt 幂等：万一 plain 已是 fc1:（并发写抢先了），enc == plain，
        // 下面的条件更新写回同一个值，无害。
        let res = sqlx::query(&update)
            .bind(&enc)
            .bind(id)
            .bind(&plain) // 条件：只在这一行还是我读到的那个旧值时才写
            .execute(&state.db)
            .await?;
        n += res.rows_affected();
    }
    Ok(n)
}

/// connected_accounts 主键是 (user_id, provider)，且有两列要处理（access + refresh，
/// refresh 可空），所以单独写。
async fn backfill_connected_accounts(state: &AppState) -> anyhow::Result<u64> {
    let rows: Vec<(uuid::Uuid, String, String, Option<String>)> = sqlx::query_as(
        "SELECT user_id, provider, access_token, refresh_token FROM connected_accounts \
         WHERE (access_token NOT LIKE 'fc1:%' AND access_token <> '') \
            OR (refresh_token IS NOT NULL AND refresh_token <> '' AND refresh_token NOT LIKE 'fc1:%') \
         LIMIT 5000",
    )
    .fetch_all(&state.db)
    .await?;

    let mut n = 0u64;
    for (uid, provider, access, refresh) in rows {
        if !access.is_empty() && !field_crypto::is_encrypted(&access) {
            let enc = field_crypto::encrypt(&access, "connected_accounts.access_token");
            let res = sqlx::query(
                "UPDATE connected_accounts SET access_token = $1 \
                 WHERE user_id = $2 AND provider = $3 AND access_token = $4",
            )
            .bind(&enc)
            .bind(uid)
            .bind(&provider)
            .bind(&access)
            .execute(&state.db)
            .await?;
            n += res.rows_affected();
        }
        if let Some(r) = refresh {
            if !r.is_empty() && !field_crypto::is_encrypted(&r) {
                let enc = field_crypto::encrypt(&r, "connected_accounts.refresh_token");
                let res = sqlx::query(
                    "UPDATE connected_accounts SET refresh_token = $1 \
                     WHERE user_id = $2 AND provider = $3 AND refresh_token = $4",
                )
                .bind(&enc)
                .bind(uid)
                .bind(&provider)
                .bind(&r)
                .execute(&state.db)
                .await?;
                n += res.rows_affected();
            }
        }
    }
    Ok(n)
}

/// API key 的存量迁移：给每行补上 `api_key_sha256`（鉴权用）和 `api_key_enc`（回显用）。
///
/// 和上面几张表不同，这里**两列一起补**，因为两个用途都要满足：
///   - 鉴权要能按 key 反查用户 → 确定性哈希 + 唯一索引，一次索引命中；
///   - `GET /api/ide-key` 要把同一把 key 原样还给登录用户 → 可解密的密文。
///     只存哈希的话这个接口直接废掉（哈希是单向的），而它是 IDE 自动配置的来源。
///
/// 明文列**不在这里清**：清除是单独一次部署的事（API_KEY_PURGE_PLAINTEXT=1），
/// 这样回滚到旧二进制时旧代码照常读 api_key，线上用户无感。见 docs/OPERATIONS.md。
async fn backfill_api_keys(state: &AppState) -> anyhow::Result<u64> {
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, api_key FROM api_keys \
         WHERE api_key IS NOT NULL AND api_key <> '' AND api_key_sha256 IS NULL LIMIT 5000",
    )
    .fetch_all(&state.db)
    .await?;
    let mut done = 0u64;
    for (id, plain) in rows {
        let digest = crate::api_key_store::sha256_hex(&plain);
        let enc = field_crypto::encrypt(&plain, crate::api_key_store::API_KEY_CTX);
        // 条件更新：只在这一行仍是我们读到的那个明文、且还没被别人补过时才写，
        // 绝不覆盖并发写进来的新值。
        let n = sqlx::query(
            "UPDATE api_keys SET api_key_sha256 = $1, api_key_enc = $2 \
             WHERE id = $3 AND api_key = $4 AND api_key_sha256 IS NULL",
        )
        .bind(&digest)
        .bind(&enc)
        .bind(id)
        .bind(&plain)
        .execute(&state.db)
        .await?
        .rows_affected();
        done += n;
    }
    if done > 0 {
        tracing::info!(rows = done, "api_keys 哈希/密文回填完成");
    }
    Ok(done)
}
