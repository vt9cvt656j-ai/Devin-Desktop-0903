-- AI models the platform offers to the IDE. The backend proxies calls so the
-- provider api_key never leaves the server, and each call bills the user's credits.
CREATE TABLE IF NOT EXISTS models (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    label       TEXT NOT NULL,                 -- display name, e.g. "DeepSeek V4"
    provider    TEXT NOT NULL DEFAULT '',      -- e.g. deepseek / openai / anthropic
    base_url    TEXT NOT NULL,                 -- OpenAI-compatible base, e.g. https://api.deepseek.com/v1
    model_id    TEXT NOT NULL,                 -- upstream model name, e.g. deepseek-chat
    api_key     TEXT NOT NULL DEFAULT '',      -- secret; never returned to clients
    price_cents BIGINT NOT NULL DEFAULT 0,     -- credits deducted per call, USD cents
    active      BOOLEAN NOT NULL DEFAULT true,
    sort        INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_models_active ON models (active, sort);

CREATE TABLE IF NOT EXISTS model_usage (
    id         BIGSERIAL PRIMARY KEY,
    user_id    UUID REFERENCES users (id) ON DELETE SET NULL,
    model_id   UUID REFERENCES models (id) ON DELETE SET NULL,
    cost_cents BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_model_usage_created ON model_usage (created_at DESC);
