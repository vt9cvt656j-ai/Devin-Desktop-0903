const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789abcdefghijkmnpqrstuvwxyz";

/// Generate a cryptographically random, URL-safe access token.
///
/// Entropy comes from the operating system CSPRNG (`getrandom`). Characters are
/// drawn from [`ALPHABET`] using rejection sampling so the distribution is
/// uniform and unbiased. The token is the sole authentication barrier when the
/// bridge is exposed through a tunnel, so it must be unpredictable.
pub fn generate_token(len: usize) -> String {
    let n = ALPHABET.len() as u16; // 55
                                   // Largest multiple of `n` that fits in a byte; values >= this are rejected
                                   // to avoid modulo bias.
    let limit = (256 / n) * n;

    let mut out = String::with_capacity(len);
    let mut buf = [0u8; 64];
    while out.len() < len {
        getrandom::getrandom(&mut buf).expect("OS CSPRNG unavailable");
        for &b in buf.iter() {
            if (b as u16) < limit {
                out.push(ALPHABET[(b as u16 % n) as usize] as char);
                if out.len() == len {
                    break;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_requested_length() {
        assert_eq!(generate_token(32).len(), 32);
        assert_eq!(generate_token(1).len(), 1);
        assert_eq!(generate_token(100).len(), 100);
    }

    #[test]
    fn token_uses_only_alphabet() {
        let t = generate_token(64);
        assert!(t.bytes().all(|b| ALPHABET.contains(&b)));
    }

    #[test]
    fn tokens_differ() {
        // Extremely unlikely to collide; guards against a constant token.
        let a = generate_token(32);
        let b = generate_token(32);
        assert_ne!(a, b);
    }
}
