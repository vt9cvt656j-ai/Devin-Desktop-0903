-- Track per-call token counts in model_usage for billing audit.
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS prompt_tokens    BIGINT  NOT NULL DEFAULT 0;
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS completion_tokens BIGINT NOT NULL DEFAULT 0;
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS cached_tokens    BIGINT  NOT NULL DEFAULT 0;
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS model_name       TEXT    NOT NULL DEFAULT '';
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS estimated        BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX IF NOT EXISTS idx_model_usage_user ON model_usage (user_id, created_at DESC);
