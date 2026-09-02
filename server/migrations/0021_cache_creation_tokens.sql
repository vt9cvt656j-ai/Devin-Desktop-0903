-- Preserve Anthropic cache-write usage so exact settlements expose every billed token class.
ALTER TABLE model_usage
    ADD COLUMN IF NOT EXISTS cache_creation_tokens BIGINT NOT NULL DEFAULT 0;
