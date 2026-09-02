-- Reachability samples for each configured model route.
--
-- Nothing in this database recorded whether a provider was up. `model_usage` has tens of
-- thousands of rows but only cost and tokens — no latency, no outcome — so "is this model
-- healthy" could not be answered from anything already stored. This is that missing
-- measurement, and it is deliberately the cheap half: an HTTP round trip to the route's
-- own base URL, which costs nothing per probe. Conversation latency would mean paying for
-- a completion against every model on every cycle, forever, and is not collected here.
--
-- One row per model per probe. Append-only; `prune` in health.rs drops what has aged out.
CREATE TABLE IF NOT EXISTS model_health (
    id         BIGSERIAL PRIMARY KEY,
    model_id   UUID NOT NULL REFERENCES models (id) ON DELETE CASCADE,

    checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Reachable, not "returned 200". A provider answering 401 or 404 on its base URL is
    -- up — it spoke. Only a refused connection, a TLS failure or a timeout is down, and
    -- conflating the two would have every route showing as broken.
    ok         BOOLEAN NOT NULL,
    -- Round trip in milliseconds, NULL when the probe never got an answer.
    latency_ms INTEGER,
    -- What the endpoint replied, for telling "answered 401" apart from "answered 500".
    status_code INTEGER,
    -- Bounded: this is upstream text and it is rendered in the console.
    error      TEXT NOT NULL DEFAULT ''
);

-- Every query is "this model, newest first" — the sparkline, the latest sample and the
-- availability window all read that way.
CREATE INDEX IF NOT EXISTS idx_model_health_model ON model_health (model_id, checked_at DESC);
-- Pruning scans by age across all models.
CREATE INDEX IF NOT EXISTS idx_model_health_age ON model_health (checked_at);
