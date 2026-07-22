-- Per-connection friendly display names for enabled models. Maps the raw upstream
-- model id (which can be ugly, e.g. "[甲厚]claude-opus-4-8特价") to a clean name shown
-- in the IDE picker. The IDE still sends the RAW id upstream; only the label changes.
-- Empty object = no overrides (every model shows its raw id, unchanged behavior).
ALTER TABLE models ADD COLUMN IF NOT EXISTS model_names JSONB NOT NULL DEFAULT '{}';
