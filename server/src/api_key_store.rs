//! API key 的存取：**校验走哈希，回显走密文，明文只在过渡期存在**。
//!
//! 原来这张表是 `api_key TEXT NOT NULL UNIQUE` + 一条建在明文上的索引，八处查询一律
//! `WHERE api_key = $1`。任何一次只读的库暴露（备份、只读副本、pg_dump 误配、一条 SQL
//! 注入）都直接产出一批**可立即使用**的网关凭据：持有者以受害者身份跑
//! `/v1/chat/completions`，费用记受害者钱包，而且允许记成负债。
//! 同一份代码对上游供应商的 key 一直是加密存的（field_crypto），管理端还做了 mask ——
//! 标准不一致本身就是这条的证据。
//!
//! 为什么是两列而不是"存哈希就完了"
//! --------------------------------
//! `GET /api/ide-key` 要把**同一把 key 原样还给登录用户**（IDE 自动配置，跨设备跨会话
//! 必须稳定）。哈希单向，只存哈希这个接口直接废掉。所以：
//!
//!   `api_key_sha256`  确定性 → 可建唯一索引 → 鉴权仍是一次索引命中，不用全表解密。
//!                     单向，库泄漏拿不到能用的凭据。
//!   `api_key_enc`     field_crypto 加密（随机 nonce，不可索引）。只有 ide-key 解它。
//!
//! 过渡期怎么保证用户无感
//! ----------------------
//! 1. 迁移只**加列**、列可空、明文列改为可空并去掉唯一约束——回滚到旧二进制照常工作。
//! 2. 查询**先哈希后明文**：回填还没跑完、或滚动发布期间旧二进制刚写入的行，仍能命中。
//! 3. 命中明文回退时**顺手补上哈希和密文**（self-healing），不必等后台回填。
//! 4. 明文的清除是**单独一次部署**的事（`API_KEY_PURGE_PLAINTEXT=1`），确认无恙再做。

use sha2::{Digest, Sha256};

/// field_crypto 的 AAD 上下文。换了它旧密文就解不开，别改。
pub const API_KEY_CTX: &str = "api_keys.api_key";

/// key 的确定性摘要，用来查库。
///
/// 直接 SHA-256 而不是 bcrypt/argon2：这类 key 是我们自己生成的高熵随机串（见
/// `gen_api_key`），不存在字典攻击面，而鉴权在每个请求的热路径上——慢哈希会把
/// 网关的吞吐直接打下去。加盐也没有意义：要按 key 反查用户就必须是确定性的。
pub fn sha256_hex(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// 按 key 反查用户 id。
///
/// 先查哈希（正常路径，走唯一索引）；查不到再查明文（过渡期兜底），命中就顺手把这一行
/// 补齐——这样即使后台回填还没跑到，用户的第一次请求也会把自己的行修好。
pub async fn lookup_user(db: &sqlx::PgPool, key: &str) -> Option<uuid::Uuid> {
    let digest = sha256_hex(key);
    if let Ok(Some(uid)) =
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT user_id FROM api_keys WHERE api_key_sha256 = $1")
            .bind(&digest)
            .fetch_optional(db)
            .await
    {
        return Some(uid);
    }
    // 过渡期回退：这一行还没被回填，或者是滚动发布期间旧二进制刚写进来的。
    let hit = sqlx::query_scalar::<_, uuid::Uuid>("SELECT user_id FROM api_keys WHERE api_key = $1")
        .bind(key)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()?;
    // Self-healing：补齐这一行，下次就走索引。补不上也不影响本次鉴权。
    let enc = crate::field_crypto::encrypt(key, API_KEY_CTX);
    let _ = sqlx::query(
        "UPDATE api_keys SET api_key_sha256 = $1, api_key_enc = $2 \
         WHERE api_key = $3 AND api_key_sha256 IS NULL",
    )
    .bind(&digest)
    .bind(&enc)
    .bind(key)
    .execute(db)
    .await;
    Some(hit)
}

/// 记一次使用时间。和 lookup 一样两条路都认。
pub async fn touch_last_used(db: &sqlx::PgPool, key: &str) {
    let _ = sqlx::query(
        "UPDATE api_keys SET last_used_at = now() WHERE api_key_sha256 = $1 OR api_key = $2",
    )
    .bind(sha256_hex(key))
    .bind(key)
    .execute(db)
    .await;
}

/// 新签发一把 key 时要写的三列。
///
/// `plaintext` 只在 `API_KEY_KEEP_PLAINTEXT=1` 时才写——默认不写，也就是**新 key 从一开始
/// 就不落明文**。留这个开关是为了万一需要回滚到旧二进制时，新签发的 key 仍能被旧代码读到。
pub fn columns_for_new(key: &str) -> (String, String, Option<String>) {
    (
        sha256_hex(key),
        crate::field_crypto::encrypt(key, API_KEY_CTX),
        keep_plaintext().then(|| key.to_string()),
    )
}

pub fn keep_plaintext() -> bool {
    matches!(
        std::env::var("API_KEY_KEEP_PLAINTEXT").unwrap_or_default().trim(),
        "1" | "true" | "TRUE"
    )
}

/// 清除存量明文。**单独一次部署**才做，由 `API_KEY_PURGE_PLAINTEXT=1` 打开。
///
/// 只清已经补齐了哈希和密文的行——没补齐就清等于把这把 key 弄丢，用户再也登不上。
pub fn spawn_purge(state: crate::AppState) {
    let on = matches!(
        std::env::var("API_KEY_PURGE_PLAINTEXT").unwrap_or_default().trim(),
        "1" | "true" | "TRUE"
    );
    if !on {
        return;
    }
    // **没配 FIELD_ENC_KEY 就绝不能清**。field_crypto::encrypt 在没有密钥时是
    // passthrough（原样返回明文，见它自己的注释「不配 = 零行为变化」），于是
    // api_key_enc 里存的其实还是明文。这时候清掉 api_key 不但一点安全收益都没有，
    // 还把同一份明文从一列搬到另一列——纯粹的白忙，而且给人"已经处理好了"的错觉。
    if !crate::field_crypto::enabled() {
        tracing::error!(
            "API_KEY_PURGE_PLAINTEXT=1 但没有配置 FIELD_ENC_KEY —— 拒绝清除。\
             没有密钥时 api_key_enc 里存的仍是明文，清掉 api_key 只是把明文换了个列名。\
             先配好 FIELD_ENC_KEY，等回填把 api_key_enc 写成 fc1: 密文之后再来。"
        );
        return;
    }
    tokio::spawn(async move {
        // 排在回填后面：回填 5 秒后开始，这里等久一点，确保它先跑完。
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        match sqlx::query(
            // `api_key_enc LIKE 'fc1:%'` 是"这一行真的被加密过"的判据（fc1: 是
            // field_crypto 的密文前缀）。没有这一条的话，密钥配置事故期间写进去的
            // 明文行也会被当成"已加密"而清掉原列。
            "UPDATE api_keys SET api_key = NULL \
             WHERE api_key IS NOT NULL AND api_key_sha256 IS NOT NULL \
               AND api_key_enc LIKE 'fc1:%'",
        )
        .execute(&state.db)
        .await
        {
            Ok(r) => tracing::info!(rows = r.rows_affected(), "api_keys 明文已清除"),
            Err(e) => tracing::error!(error = %e, "清除 api_keys 明文失败"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_deterministic_and_not_the_key() {
        let k = "sk-abc123";
        assert_eq!(sha256_hex(k), sha256_hex(k), "同一把 key 必须算出同一个摘要，否则查不到");
        assert_ne!(sha256_hex(k), k, "摘要不能就是 key 本身");
        assert_ne!(sha256_hex(k), sha256_hex("sk-abc124"), "不同 key 不能撞同一个摘要");
        assert_eq!(sha256_hex(k).len(), 64);
        assert!(sha256_hex(k).chars().all(|c| c.is_ascii_hexdigit()));
        // 已知向量：确认就是标准 SHA-256，而不是某个自创构造
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn new_keys_do_not_store_plaintext_by_default() {
        std::env::remove_var("API_KEY_KEEP_PLAINTEXT");
        let (digest, enc, plain) = columns_for_new("sk-test-xyz");
        assert_eq!(digest.len(), 64);
        assert!(!enc.is_empty());
        assert!(plain.is_none(), "默认就不该把明文写进库——这正是这次要修的");
        // 注意：没配 FIELD_ENC_KEY 时 field_crypto::encrypt 是 **passthrough**
        //（它自己的注释写着「不配 = 零行为变化」），所以这里 enc 就等于明文。
        // 断言"密文≠明文"在测试环境里必然失败，而那不是 bug。真正要守的性质是：
        // **没有密钥就不许清除明文列**——否则等于把明文从一列搬到另一列。
        // 那条守在 purge 里，由下面这个测试钉住。
        assert!(
            !crate::field_crypto::enabled(),
            "测试环境不该配 FIELD_ENC_KEY；配了的话这条断言的前提要重写"
        );
    }

    #[test]
    fn purge_refuses_to_run_without_an_encryption_key() {
        // 这条守的是最坏的一种"修好了"的错觉：没配密钥时 api_key_enc 里其实是明文，
        // 这时候清掉 api_key 一点安全收益都没有，只是把同一份明文换了个列名。
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/api_key_store.rs"))
            .expect("read api_key_store.rs");
        let at = src.find("pub fn spawn_purge").expect("spawn_purge 改名了");
        let body: String = src[at..].chars().take(1_800).collect();
        let code: String = body
            .lines()
            .map(|l| match l.find("//") { Some(i) => &l[..i], None => l })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("if !crate::field_crypto::enabled()"),
            "没有拦住「没密钥也清明文」——那会把明文从一列搬到另一列还以为修好了",
        );
        assert!(
            code.contains("api_key_enc LIKE 'fc1:%'"),
            "清除条件必须要求这一行**真的**被加密过（fc1: 前缀），不能只看非空",
        );
    }
}
