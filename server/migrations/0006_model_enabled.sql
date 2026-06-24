-- A model entry is a provider *connection* (one api_key + base_url) that can
-- expose many of the upstream models. `enabled_models` is the admin-chosen
-- subset to surface to the IDE. Empty array = fall back to the single model_id.
ALTER TABLE models ADD COLUMN IF NOT EXISTS enabled_models TEXT[] NOT NULL DEFAULT '{}';
