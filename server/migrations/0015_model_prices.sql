-- Per-MODEL billing price overrides for a connection's enabled models. Maps the raw
-- upstream model id to its own input/output price: { "<id>": {"in": usd_per_1M, "out": usd_per_1M} }.
-- When an entry is set (in>0 or out>0) it WINS over the built-in official price catalog for that
-- model; empty/absent → fall back to the catalog, then the connection-level input/output price.
-- The connection's 倍率 (rate) still multiplies on top. Empty object = no per-model overrides.
ALTER TABLE models ADD COLUMN IF NOT EXISTS model_prices JSONB NOT NULL DEFAULT '{}';
