use std::time::{SystemTime, UNIX_EPOCH};

const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789abcdefghijkmnpqrstuvwxyz";

/// Generate a random, URL-safe access token.
///
/// This is not a cryptographic library: it derives entropy from the system
/// clock and address-space layout, which is sufficient for a local,
/// user-controlled bridge token that the user can rotate at any time.
pub fn generate_token(len: usize) -> String {
    let mut seed = seed();
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        // xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let idx = (seed % ALPHABET.len() as u64) as usize;
        out.push(ALPHABET[idx] as char);
    }
    out
}

fn seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let stack_marker = 0u8;
    let addr = &stack_marker as *const u8 as u64;
    let mixed = nanos ^ addr.rotate_left(32) ^ 0x9E3779B97F4A7C15;
    // ensure non-zero seed for xorshift
    if mixed == 0 {
        0x9E3779B97F4A7C15
    } else {
        mixed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_requested_length() {
        assert_eq!(generate_token(32).len(), 32);
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
