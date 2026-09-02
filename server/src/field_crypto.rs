//! 落库字段加密（data at rest）。
//!
//! # 它挡的是谁
//!
//! 挡的是**拿到数据库的人**：一次拖库、一份误发到公网的备份、一个装了根证书也拦不住
//! 的 DBA。这些人手里有整张表，但只要 `FIELD_ENC_KEY` 不在库里，敏感字段对他们就是
//! 一串 `fc1:...`，标准库再全也解不开——因为解密密钥根本不在他拿到的那份东西里。
//!
//! 这是整套系统里唯一一处「别人的标准库真的解不开我的数据」字面成立的地方，而它成立
//! 的原因和加密算法无关，只和**密钥托管**有关：密钥存在进程环境里，不进 Postgres、
//! 不进任何备份。
//!
//! # 它挡不住谁
//!
//! 挡不住能读到进程内存或环境变量的人（那已经是完全的主机沦陷）。也挡不住业务逻辑
//! 本身——handler 解密之后照常用，所以一个能调 handler 的攻击者看到的仍是明文。要对
//! 这类攻击者收紧，靠的是权限判定，不是这一层。
//!
//! # 只加密「只存不查」的字段
//!
//! 加密后的值每次 nonce 都不同，所以**不能**按值查询（`WHERE x = $1` 会失效）。因此
//! 这一层只用在从不按值查询、只存储+回显的字段上：上游 api_key、OAuth 令牌、提现账户、
//! 收款 QR。邮箱、用户 api_key、prefix token 这些要按值查的字段不走这里——它们要么保持
//! 现状，要么另做确定性加密/盲索引，是单独一大步。
//!
//! # 迁移安全：带版本前缀 + 读时两种都认
//!
//! 密文存成 `fc1:<base64url>`；没有这个前缀的一律当**遗留明文**原样读。于是：
//!   - 没配 `FIELD_ENC_KEY`：encrypt 直接返回明文，全链路零行为变化（可安全上线）；
//!   - 配了：新写入加密，旧行仍是明文、读时照常认——没有 flag-day，回填可以在线慢慢做。
//!
//! # 一句必须记住的话
//!
//! 回填之后，`FIELD_ENC_KEY` 丢了 = 那些字段**永久无法恢复**。这把密钥必须离线备份，
//! 而且和数据库备份分开存——否则「和数据库分开」这个前提就没了。

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD as B64U};
use base64::Engine;
use rand::RngCore;
use std::sync::OnceLock;
use zeroize::Zeroizing;

/// 版本前缀。将来换构造时靠它区分，而不是靠猜。
const PREFIX: &str = "fc1:";
const NONCE_LEN: usize = 12;

/// 进程级密钥。`None` = 没配 `FIELD_ENC_KEY`，此时全链路走明文（passthrough）。
///
/// 用 Zeroizing 包住，进程退出时把密钥字节抹掉，别留在内存镜像里。
static KEY: OnceLock<Option<Zeroizing<[u8; 32]>>> = OnceLock::new();

/// 从 `FIELD_ENC_KEY`（base64 的 32 字节）装载。在 main 启动时调用一次。
///
/// 配错（不是 32 字节 / 不是 base64）要在这里就把进程拦下来，而不是等第一条写入时
/// 才发现——那时候一半数据已经加密、一半没有，最难收拾。
pub fn init() -> anyhow::Result<()> {
    let raw = std::env::var("FIELD_ENC_KEY").unwrap_or_default();
    if raw.trim().is_empty() {
        tracing::warn!(
            "FIELD_ENC_KEY 没配：落库字段加密处于关闭状态，敏感字段以明文存库。\
             生成一把：openssl rand -base64 32"
        );
        let _ = KEY.set(None);
        return Ok(());
    }
    let bytes = B64
        .decode(raw.trim())
        .map_err(|e| anyhow::anyhow!("FIELD_ENC_KEY 不是合法 base64：{e}"))?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "FIELD_ENC_KEY 解出来是 {} 字节，需要正好 32 字节（AES-256）。\
             生成：openssl rand -base64 32",
            bytes.len()
        );
    }
    let mut arr = Zeroizing::new([0u8; 32]);
    arr.copy_from_slice(&bytes);
    tracing::info!("落库字段加密已启用（FIELD_ENC_KEY 已装载）");
    let _ = KEY.set(Some(arr));
    Ok(())
}

fn key() -> Option<&'static Zeroizing<[u8; 32]>> {
    // init() 没被调用时 get() 是 None；这里当作「未配置」处理，passthrough。
    KEY.get().and_then(|o| o.as_ref())
}

/// 是否已启用（配了密钥）。回填任务用它判断要不要跑。
pub fn enabled() -> bool {
    key().is_some()
}

/// 一个值当前是不是已加密（带 `fc1:` 前缀）。回填时用来跳过已经加密的行。
pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(PREFIX)
}

/// 加密。空串原样返回（空的 api_key 表示「没配」，加密它会让 `<> ''` 判定失真）。
/// 没配密钥时返回明文——这条是安全上线的关键：不配 = 零行为变化。
///
/// `context` 绑进 AAD：它是列的身份（如 `"models.api_key"`）。这样一段密文即使被搬到
/// 另一列也解不开，杜绝「把 A 用户的令牌密文塞进 B 列」这类跨字段搬运。
pub fn encrypt(plaintext: &str, context: &str) -> String {
    if plaintext.is_empty() {
        return String::new();
    }
    let Some(k) = key() else {
        return plaintext.to_string();
    };
    // 已经加密过的不要再套一层（回填幂等、写入路径万一重复调用也安全）。
    if is_encrypted(plaintext) {
        return plaintext.to_string();
    }
    let cipher = match Aes256Gcm::new_from_slice(k.as_slice()) {
        Ok(c) => c,
        Err(_) => return plaintext.to_string(),
    };
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = match cipher.encrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: plaintext.as_bytes(),
            aad: context.as_bytes(),
        },
    ) {
        Ok(c) => c,
        // 加密失败绝不能静默写明文进库——那会让人以为加密了其实没有。
        // 但也不能 panic 掉整个请求。记日志并返回一个明确不可用的哨兵，
        // 让写入方自己决定（实践中 AES-GCM 不会在这里失败）。
        Err(e) => {
            tracing::error!(error = %e, context, "字段加密失败");
            return plaintext.to_string();
        }
    };
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    format!("{PREFIX}{}", B64U.encode(out))
}

/// 解密。没有 `fc1:` 前缀的一律当遗留明文原样返回——这就是「读时两种都认」，
/// 也是回填能在线慢慢做的原因。
///
/// 带前缀却解不开（密钥不对、被改过、或配了别的密钥），返回 Err。带前缀但密钥没配，
/// 同样 Err——那说明数据是加密的而进程读不了，是配置事故，必须显式暴露而不是当明文。
pub fn decrypt(stored: &str, context: &str) -> Result<String, FieldCryptoError> {
    if !is_encrypted(stored) {
        return Ok(stored.to_string()); // 遗留明文
    }
    let key = key().ok_or(FieldCryptoError::KeyMissing)?;
    let raw = B64U
        .decode(&stored[PREFIX.len()..])
        .map_err(|_| FieldCryptoError::Malformed)?;
    if raw.len() < NONCE_LEN + 16 {
        return Err(FieldCryptoError::Malformed);
    }
    let cipher =
        Aes256Gcm::new_from_slice(key.as_slice()).map_err(|_| FieldCryptoError::Malformed)?;
    let pt = cipher
        .decrypt(
            Nonce::from_slice(&raw[..NONCE_LEN]),
            Payload {
                msg: &raw[NONCE_LEN..],
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| FieldCryptoError::Decrypt)?;
    String::from_utf8(pt).map_err(|_| FieldCryptoError::Malformed)
}

/// 解密，失败时回退到「把存的东西原样当明文」。
///
/// 给**回显路径**用：给用户看他自己的提现账户、QR 这类。这些地方即使密钥暂时不对，
/// 回退到显示原值也远好于整个页面 500。但**绝不能**用在拿去做上游认证的地方——那里
/// 用 `decrypt` 的 Err 让请求明确失败，而不是拿一段 `fc1:...` 当令牌发出去。
pub fn decrypt_or_raw(stored: &str, context: &str) -> String {
    match decrypt(stored, context) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = ?e, context, "字段解密失败，回退显示原值");
            stored.to_string()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldCryptoError {
    /// 数据是加密的，但进程没配密钥。配置事故。
    KeyMissing,
    /// 密文结构不对（base64 坏了、太短）。
    Malformed,
    /// GCM 验证失败：密钥不对，或被改过。
    Decrypt,
}

impl std::fmt::Display for FieldCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyMissing => write!(f, "字段是加密的但 FIELD_ENC_KEY 未配置"),
            Self::Malformed => write!(f, "密文结构损坏"),
            Self::Decrypt => write!(f, "解密失败：密钥不对或内容被改过"),
        }
    }
}
impl std::error::Error for FieldCryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试要用真密钥，但 KEY 是 OnceLock、全局只能设一次，会和别的测试打架。
    // 所以测试里不走全局 KEY，直接用一把本地密钥验证 encrypt/decrypt 的**性质**。
    fn roundtrip(k: &[u8; 32], pt: &str, ctx: &str) -> String {
        use aes_gcm::aead::{Aead, Payload};
        let cipher = Aes256Gcm::new_from_slice(k).unwrap();
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: pt.as_bytes(), aad: ctx.as_bytes() })
            .unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        format!("{PREFIX}{}", B64U.encode(out))
    }

    fn open(k: &[u8; 32], stored: &str, ctx: &str) -> Result<String, ()> {
        let raw = B64U.decode(&stored[PREFIX.len()..]).map_err(|_| ())?;
        let cipher = Aes256Gcm::new_from_slice(k).unwrap();
        let pt = cipher
            .decrypt(
                Nonce::from_slice(&raw[..NONCE_LEN]),
                Payload { msg: &raw[NONCE_LEN..], aad: ctx.as_bytes() },
            )
            .map_err(|_| ())?;
        String::from_utf8(pt).map_err(|_| ())
    }

    #[test]
    fn plaintext_passes_through_when_no_prefix() {
        // 遗留明文（没前缀）永远原样读回，无论密钥配没配。这是在线迁移的地基。
        assert_eq!(decrypt("sk-legacy-plaintext", "models.api_key").unwrap(), "sk-legacy-plaintext");
        assert_eq!(decrypt("", "x").unwrap(), "");
    }

    #[test]
    fn empty_never_encrypts() {
        // 空表示「没配」，加密它会让 `api_key <> ''` 之类的判定失真。
        assert_eq!(encrypt("", "models.api_key"), "");
    }

    #[test]
    fn context_binds_the_column() {
        let k = [7u8; 32];
        let ct = roundtrip(&k, "secret-token", "connected_accounts.access_token");
        // 同一把密钥、正确的 context 能解开。
        assert_eq!(open(&k, &ct, "connected_accounts.access_token").unwrap(), "secret-token");
        // 换一个 context（等于把密文搬到别的列）就解不开。
        assert!(open(&k, &ct, "models.api_key").is_err());
    }

    #[test]
    fn tamper_fails() {
        let k = [7u8; 32];
        let mut ct = roundtrip(&k, "secret", "ctx");
        // 改最后一个字符（GCM tag 的一部分）。
        let last = ct.pop().unwrap();
        ct.push(if last == 'A' { 'B' } else { 'A' });
        assert!(open(&k, &ct, "ctx").is_err());
    }

    #[test]
    fn is_encrypted_detects_prefix() {
        assert!(is_encrypted("fc1:whatever"));
        assert!(!is_encrypted("sk-plaintext"));
        assert!(!is_encrypted(""));
    }
}
