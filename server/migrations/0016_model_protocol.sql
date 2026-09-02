-- Per-connection upstream wire protocol: "anthropic" (native /v1/messages) or "openai"
-- (/chat/completions compat). The gateway translates the OpenAI request/response ⇄ Anthropic
-- when a connection is "anthropic". New connections default to 'anthropic' (schema default).
ALTER TABLE models ADD COLUMN IF NOT EXISTS protocol TEXT NOT NULL DEFAULT 'anthropic';

-- One-time backfill for EXISTING rows: everything currently works over OpenAI-compat, so keep
-- non-Claude connections on 'openai' (don't break gpt / deepseek / gemini / minimax). Claude-
-- family connections (Claude, Claude Code MAX, …) flip to native Anthropic — the whole point.
UPDATE models SET protocol = 'openai'
  WHERE NOT (
    lower(coalesce(provider, '')) LIKE '%claude%'
    OR lower(coalesce(label, '')) LIKE '%claude%'
    OR lower(coalesce(model_id, '')) LIKE 'claude%'
    OR EXISTS (SELECT 1 FROM unnest(enabled_models) e WHERE lower(e) LIKE 'claude%')
  );
