-- Correlate each IDE model turn with the exact server-side billing settlement.
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS request_id TEXT;

CREATE INDEX IF NOT EXISTS idx_model_usage_user_request
    ON model_usage (user_id, request_id)
    WHERE request_id IS NOT NULL;
