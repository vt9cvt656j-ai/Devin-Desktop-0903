-- A stable name for the machine a session belongs to, so the list is devices and not
-- sign-ins.
--
-- Without it there is nothing that says two rows are the same computer. The console was
-- showing three "Chrome · macOS" rows at one IP because signing in three times inserted
-- three rows, and every one of them was a live token — accurate as a list of sign-ins,
-- wrong on a page titled "Devices".
--
-- The client generates this once and keeps it; it is an opaque random string, not
-- fingerprinting, and it identifies a browser profile or an app install rather than a
-- person. It is display grouping only: it grants nothing and is never trusted for
-- authorisation, so a client that makes one up gains nothing but a second row of its own.
--
-- Empty for every row written before this and for any client too old to send one. Those
-- fall back to grouping on User-Agent + IP, which is imperfect — two machines behind one
-- office NAT running the same browser look alike — but it only applies to rows that age
-- out with the token, and it beats listing the same laptop three times.
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS device_id TEXT NOT NULL DEFAULT '';

-- Signing in again on a device the account already has a live session on replaces that
-- session rather than adding one, so this lookup runs on every sign-in.
CREATE INDEX IF NOT EXISTS idx_sessions_device
    ON sessions (user_id, device_id)
    WHERE revoked_at IS NULL;
