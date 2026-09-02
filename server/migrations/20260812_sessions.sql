-- Signed-in devices, so a person can see where their account is logged in and cut one off.
--
-- The gateway's tokens are stateless JWTs with a 30-day life, which means that until now
-- there was no list to show and no way to end a session short of rotating the signing
-- secret and logging everybody out at once. This table is the missing record: one row per
-- sign-in, and `revoked_at` is what makes a single one stop working.
--
-- The check happens in the Claims extractor, which already reads the user's row on every
-- authenticated request, so revocation costs no extra round trip.
CREATE TABLE IF NOT EXISTS sessions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- 'web' | 'desktop' | 'mobile'. Taken from a hint the client sends, falling back to
    -- the User-Agent. Display only — it grants nothing, so a client lying about it gains
    -- nothing either.
    kind         TEXT NOT NULL DEFAULT 'web',
    -- Truncated before it is stored. Shown back to the account holder, so it is treated
    -- as untrusted text everywhere it is rendered.
    user_agent   TEXT NOT NULL DEFAULT '',
    -- Coarse origin for "was this me?". Held as text because it arrives from a proxy
    -- header and may be either IPv4 or IPv6.
    ip           TEXT NOT NULL DEFAULT '',

    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Updated at most every 5 minutes, not per request: a write on every call would turn
    -- a read-only auth check into a write on the hottest path in the service.
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at   TIMESTAMPTZ
);

-- The list query: this user's sessions, newest first.
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions (user_id, created_at DESC);
-- Expired rows are dead weight; a session cannot outlive the token that points at it.
CREATE INDEX IF NOT EXISTS idx_sessions_last_seen ON sessions (last_seen_at);
