-- Durable, user-scoped source of truth for michael-compression.
-- Redis remains the hot cache; these rows make exact history recoverable after cache eviction
-- or a backend restart. Payloads are gzip-compressed RawSegmentArchive JSON.
CREATE TABLE IF NOT EXISTS michael_context_archives (
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_key    TEXT NOT NULL,
    payload         BYTEA NOT NULL,
    search_index    JSONB NOT NULL,
    summary         TEXT NOT NULL,
    raw_tokens      BIGINT NOT NULL CHECK (raw_tokens >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, archive_key)
);

CREATE INDEX IF NOT EXISTS michael_context_archives_last_accessed_idx
    ON michael_context_archives (last_accessed_at);

-- Prefix handles are also durable. Without this table a Redis restart invalidates the client
-- handle even though all archived source segments still exist in PostgreSQL.
CREATE TABLE IF NOT EXISTS michael_context_prefixes (
    token       TEXT PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    record      JSONB NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '90 days'),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS michael_context_prefixes_expiry_idx
    ON michael_context_prefixes (expires_at);
