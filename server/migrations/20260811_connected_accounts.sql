-- Code hosts a person has linked to their account, so the IDE can offer their own
-- repositories behind `@github:` / `@gitee:`.
--
-- One row per (person, provider): linking GitHub twice is not a thing, and the primary
-- key says so rather than leaving the second link to create a duplicate nobody notices.
-- ON DELETE CASCADE because a deleted account must not leave a live token behind.
CREATE TABLE IF NOT EXISTS connected_accounts (
    user_id          UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider         TEXT        NOT NULL,               -- 'github' | 'gitee'

    -- Snapshot of who was linked, so the page can say "connected as @someone" without
    -- spending an API call — and can still say it after the token stops working.
    account_login    TEXT        NOT NULL DEFAULT '',
    account_name     TEXT        NOT NULL DEFAULT '',
    avatar_url       TEXT        NOT NULL DEFAULT '',

    -- The tokens. These are the whole point of the table and the reason nothing here is
    -- ever returned by an API: a leaked GitHub token with `repo` scope is read/write
    -- access to every private repository the person owns. They are written by the OAuth
    -- callback and read only by this server when it calls the provider on their behalf.
    access_token     TEXT        NOT NULL,
    -- GitHub's tokens do not expire; Gitee's last a day and must be refreshed. Nullable
    -- rather than empty-string so "no refresh token" cannot be confused with "expired".
    refresh_token    TEXT,
    token_expires_at TIMESTAMPTZ,
    scopes           TEXT        NOT NULL DEFAULT '',

    connected_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_connected_accounts_provider
    ON connected_accounts (provider);
