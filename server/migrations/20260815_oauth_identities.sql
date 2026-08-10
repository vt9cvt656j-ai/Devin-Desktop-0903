-- Signing in with GitHub or Google.
--
-- Separate from `connected_accounts`, which is a different thing wearing a similar name:
-- that table links a code host to an account that already exists, so the IDE can read
-- someone's repositories, and it stores provider access tokens to spend later. This one
-- answers "who is this person" at the door, holds no provider tokens at all, and the row
-- is created before there is necessarily a user.
--
-- Identity is the provider's immutable subject id, never the email address. Emails move
-- between accounts — someone renames their GitHub login, frees the address, and it is
-- reissued — so an email-keyed lookup hands the new owner the old account. The subject id
-- is stable for the life of the provider account and is what the unique index is on.
CREATE TABLE IF NOT EXISTS auth_identities (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- 'github' | 'google'.
    provider   TEXT NOT NULL,
    -- The provider's own user id. Opaque; compared for equality and nothing else.
    subject    TEXT NOT NULL,

    -- What the provider said the address was when this was linked. Kept for support
    -- ("which Google account is this?") and never used to find the account — see above.
    email      TEXT NOT NULL DEFAULT '',

    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ,

    -- One provider account signs in to exactly one account here. Without this, two rows
    -- could claim the same GitHub user and which account you land in becomes a race.
    UNIQUE (provider, subject)
);

CREATE INDEX IF NOT EXISTS idx_auth_identities_user ON auth_identities (user_id);

-- Accounts that only ever sign in through a provider have no password.
--
-- The column stays NOT NULL and gets '' for them, rather than becoming nullable: every
-- read path in the service already treats it as a String, and a NULL would turn a missing
-- password into an Option that each of those paths would have to start handling. '' is
-- not a valid bcrypt hash, so verification cannot accidentally succeed against it — and
-- the login handler rejects it explicitly rather than relying on that.
ALTER TABLE users ALTER COLUMN password_hash SET DEFAULT '';
