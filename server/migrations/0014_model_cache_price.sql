-- Per-token CACHE pricing: separate unit prices for cache-READ (cheap, served from a
-- prompt cache) and cache-CREATE/write (premium) tokens, in the SAME unit as input_price
-- / output_price (per 1,000,000 tokens). 0 means "not configured" → the gateway keeps the
-- old fixed factors (cache read = 0.1× input, cache create = 1.25× input), so NOTHING
-- changes for a model until you set its real cache prices in the admin.
ALTER TABLE models ADD COLUMN IF NOT EXISTS cache_read_price   DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN IF NOT EXISTS cache_create_price DOUBLE PRECISION NOT NULL DEFAULT 0;
