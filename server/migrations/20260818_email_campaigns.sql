-- Sending mail to everyone, and the right to stop receiving it.
--
-- The service could already send one message at a time — verification codes, and an admin
-- endpoint that looped over recipients inside the HTTP request. That loop is what this
-- replaces: a thousand addresses at a fifth of a second each is over three minutes with the
-- browser holding the connection open, and when it times out the operator learns nothing
-- about how far it got or whether sending is still happening somewhere.
--
-- A campaign is therefore a row that outlives the request. The handler writes it, hands the
-- work to a background task, and returns immediately; the console reads progress back off
-- this table. It is also the record of what was sent to whom, which matters more for a
-- broadcast than for a verification code — a code is a fact between two parties, a
-- broadcast is a thing you may have to answer for later.
CREATE TABLE IF NOT EXISTS email_campaigns (
    id          BIGSERIAL PRIMARY KEY,

    -- 'all' | 'members' | 'plan' | 'one'. Stored as written rather than normalised into
    -- a filter, because the question asked later is "who did this go to", and the answer
    -- "members, on the 18th" survives a change to what counts as a member.
    segment     TEXT        NOT NULL,
    plan        TEXT        NOT NULL DEFAULT '',

    subject     TEXT        NOT NULL,
    body        TEXT        NOT NULL,
    html        BOOLEAN     NOT NULL DEFAULT false,

    -- Counted once when the campaign is created, so progress has a denominator that does
    -- not move underneath it as people sign up mid-send.
    total       INTEGER     NOT NULL DEFAULT 0,
    sent        INTEGER     NOT NULL DEFAULT 0,
    failed      INTEGER     NOT NULL DEFAULT 0,

    -- 'running' | 'done' | 'dev'. 'dev' is a send attempted with no mail provider
    -- configured: nothing left the building, and the row says so rather than reporting a
    -- success that did not happen.
    status      TEXT        NOT NULL DEFAULT 'running',
    created_by  TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_email_campaigns_created ON email_campaigns (created_at DESC);

-- Opting out of broadcasts, never of transactional mail.
--
-- A verification code is not something to unsubscribe from: it is the reply to an action
-- the person just took, and suppressing it would lock them out of their own account. Only
-- the campaign sender consults this column; `send_mail` itself does not, so the code path
-- that mails a login code is unaffected by anyone's preference here.
--
-- NOT NULL DEFAULT false rather than nullable: "never said" and "said no" are the same
-- state to every reader, and a NULL would only add a second spelling of it.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_opt_out BOOLEAN NOT NULL DEFAULT false;
