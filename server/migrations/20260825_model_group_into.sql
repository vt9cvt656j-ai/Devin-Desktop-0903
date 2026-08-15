-- Show one route's models under another route's name.
--
-- This is a DISPLAY grouping and nothing else. It feeds exactly one field of `/api/models`:
-- `group`, the heading the IDE files a model under in its picker. Nothing else reads it.
--
-- In particular it is NOT how a request finds a route. chat_completions picks the route by
-- model id — every active connection whose enabled set contains the requested id, in
-- (sort, created_at) order, first one serving. That query neither filters nor orders on this
-- column, so grouping cannot move a request.
--
-- So after grouping 「免费模型2」 into 「免费模型」:
--   * the picker shows both routes' models under 免费模型;
--   * a request for one of 免费模型2's models still goes to 免费模型2 — its own base_url,
--     its own api_key, its own billing_mode and per-model prices, all untouched;
--   * `model_usage.model_id` still records 免费模型2, so usage and cost stay attributed to
--     the route that actually served the call.
--
-- Nothing is copied, moved or merged in the database. Ungrouping is setting this back to
-- NULL, and it restores the previous display immediately because nothing else changed.
--
-- ON DELETE SET NULL: deleting the target route must not delete the routes filed under it.
-- They simply go back to showing under their own names.
ALTER TABLE models
  ADD COLUMN IF NOT EXISTS group_into UUID REFERENCES models (id) ON DELETE SET NULL;

-- The picker resolves every grouped route's label on each load.
CREATE INDEX IF NOT EXISTS idx_models_group_into
  ON models (group_into) WHERE group_into IS NOT NULL;
