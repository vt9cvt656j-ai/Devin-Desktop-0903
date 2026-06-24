use anyhow::Context;

/// All runtime configuration comes from the environment (a `.env` file in dev,
/// real env vars / Docker secrets in prod). Nothing is hardcoded.
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    pub db_max_connections: u32,
    pub code_ttl_secs: u64,
    pub jwt_ttl_secs: i64,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_host: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: req("DATABASE_URL")?,
            redis_url: opt("REDIS_URL", "redis://127.0.0.1:6379"),
            jwt_secret: req("JWT_SECRET")?,
            bind_addr: opt("BIND_ADDR", "0.0.0.0:8080"),
            db_max_connections: opt("DB_MAX_CONNECTIONS", "20").parse().unwrap_or(20),
            code_ttl_secs: opt("CODE_TTL_SECS", "600").parse().unwrap_or(600),
            jwt_ttl_secs: opt("JWT_TTL_SECS", "2592000").parse().unwrap_or(2_592_000), // 30d
            smtp_user: std::env::var("QQ_SMTP_USER").unwrap_or_default(),
            smtp_pass: std::env::var("QQ_SMTP_PASS").unwrap_or_default(),
            smtp_host: opt("SMTP_HOST", "smtp.qq.com"),
        })
    }

    pub fn smtp_enabled(&self) -> bool {
        !self.smtp_user.is_empty() && !self.smtp_pass.is_empty()
    }
}

fn req(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("missing required env var {key}"))
}

fn opt(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
