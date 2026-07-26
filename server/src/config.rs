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
    /// michael-compression 总开关，**默认关闭**。
    ///
    /// 关着的时候：不做任何压缩，`/api/me` 也把 michael_compression 报成 null。
    /// 两件事必须由同一个开关控制 —— 客户端一旦从 /api/me 看到档位，就会**关掉
    /// 自己的本地压缩**（认为网关接管了）。只报档位却不真压，等于两边都不压，
    /// 长对话直接撞穿模型原生窗口。
    pub compression_enabled: bool,
    pub code_ttl_secs: u64,
    pub jwt_ttl_secs: i64,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_host: String,
    pub brevo_api_key: String,
    pub mail_from: String,
    pub mail_from_name: String,
    // Speech-to-text (voice input) upstream — OpenAI-compatible /audio/transcriptions.
    // Defaults to Groq's free Whisper-large-v3. Key from GROQ_API_KEY (or TRANSCRIBE_API_KEY).
    pub transcribe_api_key: String,
    pub transcribe_url: String,
    pub transcribe_model: String,
    pub ide_update_manifest_url: String,
    pub ide_release_github_token: String,
    pub ide_release_github_repo: String,
    pub ide_release_github_workflow: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: req("DATABASE_URL")?,
            redis_url: opt("REDIS_URL", "redis://127.0.0.1:6379"),
            jwt_secret: req("JWT_SECRET")?,
            bind_addr: opt("BIND_ADDR", "0.0.0.0:8080"),
            db_max_connections: opt("DB_MAX_CONNECTIONS", "20").parse().unwrap_or(20),
            // 显式写 1/true 才开。缺省缺失都按关处理（fail-closed）。
            compression_enabled: matches!(
                opt("MICHAEL_COMPRESSION_ENABLED", "0").trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            code_ttl_secs: opt("CODE_TTL_SECS", "600").parse().unwrap_or(600),
            jwt_ttl_secs: opt("JWT_TTL_SECS", "2592000").parse().unwrap_or(2_592_000), // 30d
            smtp_user: std::env::var("QQ_SMTP_USER").unwrap_or_default(),
            smtp_pass: std::env::var("QQ_SMTP_PASS").unwrap_or_default(),
            smtp_host: opt("SMTP_HOST", "smtp.qq.com"),
            brevo_api_key: std::env::var("BREVO_API_KEY").unwrap_or_default(),
            mail_from: std::env::var("MAIL_FROM")
                .unwrap_or_else(|_| std::env::var("QQ_SMTP_USER").unwrap_or_default()),
            mail_from_name: opt("MAIL_FROM_NAME", "Michael"),
            transcribe_api_key: std::env::var("GROQ_API_KEY")
                .or_else(|_| std::env::var("TRANSCRIBE_API_KEY"))
                .unwrap_or_default(),
            transcribe_url: opt(
                "TRANSCRIBE_URL",
                "https://api.groq.com/openai/v1/audio/transcriptions",
            ),
            transcribe_model: opt("TRANSCRIBE_MODEL", "whisper-large-v3"),
            ide_update_manifest_url: opt(
                "IDE_UPDATE_MANIFEST_URL",
                "https://github.com/fendoushaonian/Devin-Desktop/releases/latest/download/latest.json",
            ),
            ide_release_github_token: std::env::var("IDE_RELEASE_GITHUB_TOKEN")
                .unwrap_or_default(),
            ide_release_github_repo: opt(
                "IDE_RELEASE_GITHUB_REPO",
                "fendoushaonian/Devin-Desktop",
            ),
            ide_release_github_workflow: opt(
                "IDE_RELEASE_GITHUB_WORKFLOW",
                "ide-package.yml",
            ),
        })
    }

    pub fn smtp_enabled(&self) -> bool {
        !self.smtp_user.is_empty() && !self.smtp_pass.is_empty()
    }

    /// Whether outbound mail can actually be sent (via the Brevo HTTP API over 443).
    pub fn mail_enabled(&self) -> bool {
        !self.brevo_api_key.is_empty() && !self.mail_from.is_empty()
    }
}

fn req(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("missing required env var {key}"))
}

fn opt(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
